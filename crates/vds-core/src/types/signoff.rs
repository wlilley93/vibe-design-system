//! Frame sign-offs, authority states, and proposed redraws.
//!
//! Draft S-7D, ENACTMENT PENDING (SUBMISSION-VDS-017). The constitutional
//! direction this implements: THE SIGNED-OFF FIGMA DEFINES TASTE. Taste is
//! exercised once, at frame sign-off, and deviations are not adjudicated
//! downstream. Three consequences, each carried by a type here:
//!
//!   1. [`SignOff`]: authority for any frame-bound proof exists ONLY while the
//!      frame's current content hash matches a sign-off row. Staleness is by
//!      HASH, never by trust: a frame edited after sign-off reverts to
//!      UNSIGNED until re-signed, however senior the last signer.
//!   2. [`AuthorityVerdict`]: the verdict vocabulary of every frame-bound
//!      proof is `conform | deviate | no_authority`. `no_authority` is a
//!      DISTINCT state - never green, never red, reported as coverage owed -
//!      and a proof cannot claim conformance against an unsigned frame.
//!   3. There is NO ACCEPTANCE STATE in the engine. An addition the frame
//!      omits is a deviation exactly like a missing element. The resolution
//!      path is [`RedrawRecord`]: the band comes back through the design,
//!      never through an engine-side excusal. This deliberately REPEALS the
//!      direction-blind and direction-carrying acceptance concepts; the
//!      repeal is stated in the submission rather than smuggled.
//!
//! # Why a sign-off stores a hash and not an approval bit
//!
//! An approval bit is trust, and trust does not expire when the frame changes.
//! The hash is the frame's content at the moment taste was exercised, so the
//! authority claim is checkable forever: current hash matches, the taste
//! decision still covers what the frame now shows; it moved, the decision
//! covers a drawing that no longer exists.

use std::path::{Component, Path};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::digest::Digest;
use crate::error::{Result, VdsError};
use crate::ids::{DirectionId, RedrawId, ReviewId, SignoffId};
use crate::timestamp::Timestamp;
use crate::types::decision::DecisionReference;

/// One frame sign-off: the frame's content hash at the moment it was signed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SignOff {
    pub id: SignoffId,
    pub file_key: String,
    /// The frame's node id, in the `12:34` spelling.
    ///
    /// It is also the LOCUS on every row this register can hold. [2026]
    /// VJS-FI-VDS 2 refused to grow this type a locus field precisely because
    /// where the Principal adopts the frame's own node as the locus - which he
    /// did on all 127 signed frames in the subject estate - this field already
    /// names it. A decision adopting a different locus is refused and referred
    /// at the door; it does not arrive here in a weaker form.
    pub node_id: String,
    /// The frame's content digest AT SIGN-OFF, as the frames ledger computes
    /// it. The whole mechanism: authority holds while the current digest
    /// equals this one, and not a moment longer.
    pub frame_digest: Digest,
    pub signed_by: String,
    pub signed_at: Timestamp,
    /// The external act imported by this row, where `signed_at` records an
    /// event that happened before this command ran.
    ///
    /// Optional so every sign-off recorded through the existing live door
    /// keeps exactly its established shape. External time is different: it is
    /// accepted only with a durable, repository-local evidence binding whose
    /// bytes can be checked again whenever the register is read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<SignOffEvidence>,
    /// WHICH LIMB OF CC-OPBOX 6 ADMITTED THIS ROW ([2026] VJS-FI-VDS 2 order 5).
    ///
    /// OPTIONAL at the type and REQUIRED at the door, and the asymmetry is
    /// deliberate. Rows written before limb (b) existed carry no basis and must
    /// keep exactly their shape and continue to parse; making the field
    /// required would turn every one of them into a parse failure, which is a
    /// reader's problem misreported as a record's. But no NEW row may omit it:
    /// a register that cannot say why it admitted a subject is asserting
    /// authority it cannot account for, and the count of rows carrying no basis
    /// is reported as coverage owed on every run rather than rounded to zero.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub basis: Option<SignOffBasis>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl SignOff {
    /// The invariants a basis carries beyond its own shape.
    ///
    /// The door enforces this on the way IN (limb 4 of
    /// [`crate::admit_under_principal_act`]). This is the same invariant on the
    /// way OUT, on every read, so a row hand-edited after the fact to cite a
    /// locus that is not its own subject is a refusal at the front of every
    /// command rather than a discovery six months later. The redundancy between
    /// `decision.locus_id` and `node_id` is a CHECKED redundancy, not a second
    /// source of truth.
    ///
    /// Node ids go through [`crate::normalise_node_id`], the SAME one rule the
    /// door uses. Two normalisations that drifted apart would refuse a row for
    /// being itself.
    pub fn check_basis(&self) -> Result<()> {
        let Some(SignOffBasis::PrincipalAct { decision }) = &self.basis else {
            return Ok(());
        };
        if crate::normalise_node_id(&decision.locus_id) != crate::normalise_node_id(&self.node_id) {
            return Err(VdsError::precondition(format!(
                "{} registers {}/{} on Principal decision seq {}, whose adopted locus is {} \
                 and not the frame's own node. A sign-off row binds the FRAME's content \
                 digest and cannot express adoption of a sub-layer, so such a decision is \
                 REFERRED and not registered ([2026] VJS-FI-VDS 2, reserved question 4; it \
                 has happened once in 167 decisions, at seq 33, and that was a refusal).",
                self.id, self.file_key, self.node_id, decision.seq, decision.locus_id
            )));
        }
        if !decision.digest.is_well_formed() {
            return Err(VdsError::precondition(format!(
                "{} cites Principal decision seq {} at malformed digest {}",
                self.id, decision.seq, decision.digest
            )));
        }
        Ok(())
    }
}

/// The two limbs of CC-OPBOX 6's disjunctive registration test. CLOSED at two.
///
/// [2026] VJS-FI-VDS 2: registration is conditional on **(a)** a recognised
/// authority label **OR** **(b)** an express Principal act. The register
/// implemented (a) alone, and because (a) is satisfied by exactly the
/// machine-implanted labels (b) exists to displace, the door admitted the
/// implant and refused every frame the Principal had cleaned. There is no third
/// limb, and there is deliberately no variant for an attestation over the
/// frames ledger as a whole: that is not a label-resolution act.
///
/// Every field is renamed EXPLICITLY rather than by `rename_all_fields`.
/// schemars 0.8 does not implement that attribute, so serde renamed the field
/// and the published schema did not: `signoff.schema.json` declared
/// `authority_layer` while every row VDS writes says `authorityLayer`, and
/// `vds schema check` could not see it because it regenerates from the same
/// blind derive and compares the result with itself.
/// `the_published_schema_declares_the_keys_that_are_actually_written` is the
/// check that would have caught it, and does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SignOffBasis {
    /// LIMB (a). The frame's authoritative layer NAMED itself current, under
    /// `[screens] authority_markers`. The layer is recorded by name, because
    /// "it was labelled" is a claim about a specific string and a reader is
    /// entitled to see which one.
    RecognisedLabel {
        #[serde(rename = "authorityLayer")]
        authority_layer: String,
    },
    /// LIMB (b). An express, per-frame Principal decision admitted the frame,
    /// whatever its labels said. The row cites THAT decision and no other.
    PrincipalAct { decision: DecisionReference },
}

impl SignOffBasis {
    pub fn as_str(&self) -> &'static str {
        match self {
            SignOffBasis::RecognisedLabel { .. } => "recognised_label",
            SignOffBasis::PrincipalAct { .. } => "principal_act",
        }
    }

    /// The basis in one line, for a register listing. A `principal_act` names
    /// the decision it rests on: a citation nobody can follow is a citation.
    pub fn describe(&self) -> String {
        match self {
            SignOffBasis::RecognisedLabel { authority_layer } => {
                format!("recognised_label  {authority_layer:?}")
            }
            SignOffBasis::PrincipalAct { decision } => format!(
                "principal_act     decision seq {} at {} (locus {})",
                decision.seq, decision.digest, decision.locus_id
            ),
        }
    }
}

/// How many rows can say nothing about why they were admitted.
///
/// Reported on every run that reports register coverage. [2026] VJS-CA-VDS 1,
/// Estate J: an unreported reach is a pass over an unknown denominator, and a
/// register whose basis coverage is silent reads as a register whose basis
/// coverage is complete.
pub fn rows_without_a_basis(signoffs: &[SignOff]) -> usize {
    signoffs.iter().filter(|s| s.basis.is_none()).count()
}

/// A durable reference from one imported sign-off row to its external act.
///
/// `digest` identifies the exact evidence bytes. `frame_ledger_digest` records
/// the aggregate the act signed and lets a later reader validate the act
/// without needing the Figma/frames crate. The path is repository-relative so
/// the reference survives moving or cloning the project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SignOffEvidence {
    pub path: String,
    pub digest: Digest,
    pub frame_ledger_digest: Digest,
}

impl SignOffEvidence {
    /// Validate the stored reference itself before a store resolves it.
    pub fn validate(&self) -> Result<()> {
        let path = Path::new(&self.path);
        if self.path.is_empty()
            || self.path.contains('\\')
            || path.is_absolute()
            || path.components().any(|component| {
                matches!(
                    component,
                    Component::Prefix(_)
                        | Component::RootDir
                        | Component::ParentDir
                        | Component::CurDir
                )
            })
        {
            return Err(VdsError::precondition(format!(
                "external sign-off evidence path {:?} is not a canonical repository-relative \
                 path. Evidence must remain under the project root and use its durable relative \
                 spelling.",
                self.path
            )));
        }
        if !self.digest.is_well_formed() {
            return Err(VdsError::precondition(format!(
                "external sign-off evidence {} carries malformed evidence digest {}",
                self.path, self.digest
            )));
        }
        if !self.frame_ledger_digest.is_well_formed() {
            return Err(VdsError::precondition(format!(
                "external sign-off evidence {} carries malformed frame-ledger digest {}",
                self.path, self.frame_ledger_digest
            )));
        }
        Ok(())
    }
}

/// The narrow external act format accepted by `vds signoff record`.
///
/// Additional fields are deliberately allowed: the Principal's evidence may
/// preserve the reply, qualification, exclusions and capture identity in more
/// detail than VDS needs. These six fields are the irreducible binding VDS can
/// check locally without making a network call or inventing an approval.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalSignOffEvidence {
    schema_version: u32,
    kind: String,
    signed_by: String,
    signed_at: Timestamp,
    warrant: bool,
    scope: ExternalSignOffScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExternalSignOffScope {
    file_key: String,
    aggregate_digest: Digest,
}

pub const EXTERNAL_SIGNOFF_EVIDENCE_SCHEMA_VERSION: u32 = 1;
pub const EXTERNAL_SIGNOFF_EVIDENCE_KIND: &str = "external-principal-frame-signature";

impl ExternalSignOffEvidence {
    /// Parse the evidence bytes as one JSON object.
    pub fn parse(bytes: &[u8], path: &str) -> Result<Self> {
        serde_json::from_slice(bytes)
            .map_err(|error| VdsError::parse(path, "external sign-off evidence JSON", error))
    }

    /// Bind an external act to the exact row facts and local frame ledger.
    pub fn validate_for(
        &self,
        path: &str,
        file_key: &str,
        signed_by: &str,
        signed_at: &Timestamp,
        frame_ledger_digest: &Digest,
    ) -> Result<()> {
        if self.schema_version != EXTERNAL_SIGNOFF_EVIDENCE_SCHEMA_VERSION {
            return Err(VdsError::precondition(format!(
                "{path}: external sign-off evidence schemaVersion is {}, not the supported {}",
                self.schema_version, EXTERNAL_SIGNOFF_EVIDENCE_SCHEMA_VERSION
            )));
        }
        if self.kind != EXTERNAL_SIGNOFF_EVIDENCE_KIND {
            return Err(VdsError::precondition(format!(
                "{path}: external sign-off evidence kind is {:?}, not {:?}",
                self.kind, EXTERNAL_SIGNOFF_EVIDENCE_KIND
            )));
        }
        if self.signed_by != signed_by {
            return Err(VdsError::precondition(format!(
                "{path}: the evidence says {:?} signed, but --signed-by says {:?}",
                self.signed_by, signed_by
            )));
        }
        if &self.signed_at != signed_at {
            return Err(VdsError::precondition(format!(
                "{path}: the evidence says the act occurred at {}, but --signed-at says {}",
                self.signed_at, signed_at
            )));
        }
        if self.warrant {
            return Err(VdsError::precondition(format!(
                "{path}: the external sign-off evidence claims warrant=true. This door records \
                 a frame sign-off only; it grants no warrant and will not import evidence that \
                 conflates the two acts."
            )));
        }
        if self.scope.file_key != file_key {
            return Err(VdsError::precondition(format!(
                "{path}: the evidence signs Figma file {:?}, not {:?}",
                self.scope.file_key, file_key
            )));
        }
        if !self.scope.aggregate_digest.is_well_formed() {
            return Err(VdsError::precondition(format!(
                "{path}: scope.aggregateDigest {} is malformed",
                self.scope.aggregate_digest
            )));
        }
        if &self.scope.aggregate_digest != frame_ledger_digest {
            return Err(VdsError::precondition(format!(
                "{path}: the evidence signs frame-ledger aggregate {}, but the local CURRENT \
                 ledger is {}. A changed ledger is a different signing population and cannot \
                 be backfilled from this act.",
                self.scope.aggregate_digest, frame_ledger_digest
            )));
        }
        Ok(())
    }
}

/// A PRINCIPAL DIRECTION that disposes of a surface's conformance: the second
/// row kind of the sign-off register.
///
/// [2026] VJS-CA-VDS 1 order 26, giving effect to [2026] VJS-SC-OPBOX 1 orders
/// 15, 30 and 31. The register as first built could not record one, and the
/// consequence was not cosmetic: a direction is not a frame - it has no file
/// key, no node id and no frame content hash - so the four founding directions
/// order 32 requires could not pass through the door at all, and a register
/// that cannot execute the order that founds it is not the condition precedent
/// order 23 requires.
///
/// A direction is taste exercised AT the register, hash-bound, by the only
/// person entitled to exercise it. It is not taste exercised downstream by an
/// engine, which is what the constitutional direction forbids, and the
/// distinction is the whole reason this row kind is lawful.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DirectionRecord {
    pub id: DirectionId,
    /// The decision-log entry the direction was given in.
    pub log_id: String,
    /// That entry's content digest when the direction was registered.
    ///
    /// The direction's authority holds while, and only while, this equals the
    /// log entry's current digest: staleness by hash, never by trust, on the
    /// same terms as a [`SignOff`]. A direction edited after registration is a
    /// different direction.
    pub decision_digest: Digest,
    /// The surface directed: a route, or a frame.
    pub surface: DirectedSurface,
    /// The [2026] VJS-CC-OPBOX 155 O2 form: what was directed, and how much.
    /// Preserved rather than summarised, because it becomes the redraw brief.
    pub direction: String,
    pub magnitude: String,
    pub directed_at: Timestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// What a direction touches: a route where it names no frame, a frame where it
/// does. Two variants rather than three optional fields, so "a direction that
/// names neither" is unrepresentable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum DirectedSurface {
    Route { route: String },
    Frame { file_key: String, node_id: String },
}

impl DirectedSurface {
    pub fn describe(&self) -> String {
        match self {
            DirectedSurface::Route { route } => route.clone(),
            DirectedSurface::Frame { file_key, node_id } => format!("{file_key}/{node_id}"),
        }
    }
}

/// The CURRENT digest of the decision-log entry a direction names, or `None`
/// where it cannot be resolved.
///
/// `log_id` is resolved two ways, in order: as a repository-relative path, and
/// as a decision record under `[paths] logs`. Two ways rather than one because
/// the log entry a Principal direction was given in lives wherever the estate
/// keeps its decisions, and a resolver that knew only VDS's own log would make
/// every direction from a consuming repo unresolvable - which resolves
/// UNSIGNED, which would read as "the direction lapsed" when it means "I did
/// not look there".
pub fn decision_log_digest(project: &crate::project::Project, log_id: &str) -> Option<Digest> {
    let direct = project.root.join(log_id);
    if direct.is_file() {
        return Digest::of_file(&direct).ok();
    }
    let in_logs = project
        .path(crate::config::PathRole::Logs)
        .join("decisions")
        .join(format!("{log_id}.yaml"));
    if in_logs.is_file() {
        return Digest::of_file(&in_logs).ok();
    }
    None
}

/// Whether a direction row still carries authority, given the CURRENT digest
/// of the decision log entry it names.
///
/// `None` for the current digest resolves UNSIGNED, fail-closed and for the
/// same reason [`frame_authority`] does: a direction whose log entry cannot be
/// read cannot be shown to be the direction that was given.
pub fn direction_authority(
    direction: &DirectionRecord,
    current_decision_digest: Option<&Digest>,
) -> FrameAuthority {
    match current_decision_digest {
        Some(current) if current == &direction.decision_digest => FrameAuthority::Signed {
            signoff: SignoffId::parse("SGN-0000").unwrap_or_else(|_| unreachable!()),
        },
        Some(current) => FrameAuthority::Unsigned {
            because: format!(
                "direction {} was registered against decision {} at digest {}, and that log \
                 entry now digests to {current}. The direction was edited after registration, \
                 so its authority lapsed: staleness by hash, never by trust.",
                direction.id, direction.log_id, direction.decision_digest
            ),
        },
        None => FrameAuthority::Unsigned {
            because: format!(
                "direction {} names decision log {}, which cannot be read, so the direction \
                 cannot be shown to be the one that was given.",
                direction.id, direction.log_id
            ),
        },
    }
}

/// Whether a frame currently carries authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameAuthority {
    /// The frame's current content hash matches a sign-off row.
    Signed { signoff: SignoffId },
    /// No sign-off covers the frame AS IT NOW IS. Either it was never signed,
    /// or it changed after sign-off; the two are distinguished in `because`
    /// and identical in effect (staleness by hash, never by trust).
    Unsigned { because: String },
}

impl FrameAuthority {
    pub fn is_signed(&self) -> bool {
        matches!(self, FrameAuthority::Signed { .. })
    }
}

/// Resolve a frame's authority from its CURRENT content digest and the
/// sign-off register.
///
/// `current` is `None` where the frame is not in the ledger or the ledger
/// predates per-frame digests. That resolves UNSIGNED, fail-closed: a frame
/// whose current content cannot be measured cannot be shown to match any
/// sign-off, and guessing "signed" there would be authority by trust.
pub fn frame_authority(
    file_key: &str,
    node_id: &str,
    current: Option<&Digest>,
    signoffs: &[SignOff],
) -> FrameAuthority {
    let rows: Vec<&SignOff> = signoffs
        .iter()
        .filter(|s| s.file_key == file_key && s.node_id == node_id)
        .collect();
    let Some(current) = current else {
        return FrameAuthority::Unsigned {
            because: format!(
                "the frame {file_key}/{node_id} has no current content digest: it is not in \
                 the frames ledger, or the ledger predates per-frame digests. A frame whose \
                 current content cannot be measured cannot be shown to match any sign-off."
            ),
        };
    };
    if let Some(matched) = rows.iter().find(|s| &s.frame_digest == current) {
        return FrameAuthority::Signed {
            signoff: matched.id.clone(),
        };
    }
    FrameAuthority::Unsigned {
        because: if rows.is_empty() {
            format!("no sign-off row exists for {file_key}/{node_id}: the frame was never signed")
        } else {
            format!(
                "{} sign-off row(s) exist for {file_key}/{node_id} and none matches the \
                 frame's current content digest {current}. The frame changed after sign-off, \
                 so authority reverted to UNSIGNED until re-signed (staleness by hash, never \
                 by trust).",
                rows.len()
            )
        },
    }
}

/// The verdict vocabulary of every frame-bound proof. CLOSED at three.
///
/// There is deliberately no fourth variant for an accepted deviation. An
/// acceptance state is taste exercised downstream of sign-off, which the
/// constitutional direction forbids: the resolution path for a deviation is a
/// new signed frame version, recorded as a [`RedrawRecord`].
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityVerdict {
    /// The shipped artefact matches the signed frame.
    Conform,
    /// The shipped artefact differs from the signed frame. An ADDITION the
    /// frame omits is a deviation exactly like a missing element; direction
    /// carries no excuse.
    Deviate,
    /// The frame is unsigned, parked, or a proposal: there is no authority to
    /// conform to. Never green, never red; reported as coverage owed.
    NoAuthority,
}

impl AuthorityVerdict {
    pub fn as_str(self) -> &'static str {
        match self {
            AuthorityVerdict::Conform => "conform",
            AuthorityVerdict::Deviate => "deviate",
            AuthorityVerdict::NoAuthority => "no_authority",
        }
    }
}

impl std::fmt::Display for AuthorityVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Where a proposed redraw stands. CLOSED at four; there is no "accepted".
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum RedrawStatus {
    /// The change is described and owed to the design.
    Proposed,
    /// The change has been drawn and awaits sign-off.
    Drawn,
    /// PARKED under a registered Principal direction ([2026] VJS-CA-VDS 1
    /// order 27, giving effect to [2026] VJS-SC-OPBOX 1 order 29).
    ///
    /// Lawful ONLY with a `directedBy` naming a [`DirectionRecord`] whose
    /// `decisionDigest` still matches. A parked subject retains its render
    /// rights and is reported and never fatal: while the registered direction
    /// stands no gate may count it a violation.
    ///
    /// This is NOT the acceptance state returning. An acceptance was a verdict
    /// the ENGINE reached about a difference; a park is a direction the
    /// Principal gave, recorded at the register and hash-bound to its own log
    /// entry, which is where the constitutional direction puts taste.
    Parked,
    /// A sign-off row covering the change exists. ONLY lawful with
    /// `resolved_by` naming it; the proof refuses the word without the row.
    Signed,
    /// The proposal is ABANDONED and the deviation stands.
    ///
    /// Documented as what it is, per [2026] VJS-CA-VDS 1 order 27: it was
    /// briefly the only status a direction could be squeezed into, and it says
    /// the opposite of what a direction means. A direction is recorded as
    /// `parked` with a covering direction row, never here.
    Withdrawn,
}

impl RedrawStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            RedrawStatus::Proposed => "proposed",
            RedrawStatus::Drawn => "drawn",
            RedrawStatus::Parked => "parked",
            RedrawStatus::Signed => "signed",
            RedrawStatus::Withdrawn => "withdrawn",
        }
    }
}

impl std::fmt::Display for RedrawStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A deviation routed back through the design: the machine-readable form of
/// "add it neatly later".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RedrawRecord {
    pub id: RedrawId,
    /// The deviation this resolves: a review record and the delta it named.
    pub deviation: String,
    /// The review that recorded the deviation, where one exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_id: Option<ReviewId>,
    /// The proposed design change, described. A DESCRIPTION of intent, never a
    /// value: the change itself happens in the design file.
    pub proposed: String,
    pub status: RedrawStatus,
    pub file_key: String,
    pub node_id: String,
    /// The sign-off row that resolves this redraw. `signed` without it is
    /// refused by the proof: a deviation is resolvable ONLY by a sign-off row
    /// whose hash covers the change, never by the word "signed".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_by: Option<SignoffId>,
    /// The direction row that PARKS this redraw. `parked` without it is
    /// refused in the same terms `signed` without a sign-off row is
    /// ([2026] VJS-CA-VDS 1 order 27).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directed_by: Option<DirectionId>,
    pub opened_at: Timestamp,
    #[serde(default)]
    pub basis: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signoff(node: &str, digest: &str) -> SignOff {
        SignOff {
            id: SignoffId::parse("SGN-0001").unwrap(),
            file_key: "KEY".into(),
            node_id: node.into(),
            frame_digest: Digest::of_text(digest),
            signed_by: "the principal".into(),
            signed_at: Timestamp::fixed(2026, 8, 1, 10, 0, 0),
            evidence: None,
            basis: None,
            notes: None,
        }
    }

    fn principal_act() -> SignOffBasis {
        SignOffBasis::PrincipalAct {
            decision: DecisionReference {
                seq: 7,
                digest: Digest::of_text("the decision's bytes"),
                locus_id: "1:2".into(),
                locus_name: "Screen \u{b7} /x".into(),
            },
        }
    }

    /// [2026] VJS-FI-VDS 2 order 5: OPTIONAL at the type, so a row written
    /// before limb (b) existed keeps EXACTLY its shape. A field that appeared
    /// in the bytes of nineteen live rows would be a silent rewrite of the
    /// register, and `deny_unknown_fields` on the way back in would make it a
    /// parse failure under the other reader.
    #[test]
    fn a_row_with_no_basis_keeps_exactly_its_shape_and_parses() {
        let row = signoff("1:2", "frame-v1");
        let yaml = serde_yaml::to_string(&row).expect("serialises");
        assert!(!yaml.contains("basis"), "{yaml}");
        assert!(!yaml.contains("evidence"), "{yaml}");
        let back: SignOff = serde_yaml::from_str(&yaml).expect("parses");
        assert_eq!(back, row);
        assert_eq!(rows_without_a_basis(&[back]), 1);
    }

    #[test]
    fn a_principal_act_row_carries_the_decision_on_its_face() {
        let mut row = signoff("1:2", "frame-v1");
        row.basis = Some(principal_act());
        let yaml = serde_yaml::to_string(&row).expect("serialises");
        assert!(yaml.contains("kind: principal_act"), "{yaml}");
        assert!(yaml.contains("seq: 7"), "{yaml}");
        let back: SignOff = serde_yaml::from_str(&yaml).expect("parses");
        assert_eq!(back, row);
        assert_eq!(rows_without_a_basis(&[back]), 0);
    }

    #[test]
    fn a_recognised_label_row_names_the_layer_that_admitted_it() {
        let mut row = signoff("1:2", "frame-v1");
        row.basis = Some(SignOffBasis::RecognisedLabel {
            authority_layer: "CURRENT SOURCE \u{b7} /x".into(),
        });
        let yaml = serde_yaml::to_string(&row).expect("serialises");
        assert!(yaml.contains("kind: recognised_label"), "{yaml}");
        let back: SignOff = serde_yaml::from_str(&yaml).expect("parses");
        assert_eq!(
            back.basis.as_ref().map(SignOffBasis::as_str),
            Some("recognised_label")
        );
    }

    /// THE PUBLISHED SCHEMA MUST DECLARE THE KEYS THE TOOL ACTUALLY WRITES.
    ///
    /// This is not belt and braces, it caught a live defect. `SignOffBasis`
    /// was first written with `#[serde(rename_all_fields = "camelCase")]`;
    /// serde honours it and schemars 0.8 silently does not, so every row on
    /// disk said `authorityLayer` and `schema/signoff.schema.json` said
    /// `authority_layer`. `vds schema check` cannot see that, because it
    /// regenerates the schema from the same blind derive and compares it with
    /// itself - a check answering about the wrong artefact. This one asks the
    /// SERIALISED BYTES and the PUBLISHED SCHEMA the same question.
    #[test]
    fn the_published_schema_declares_the_keys_that_are_actually_written() {
        let mut generator = schemars::r#gen::SchemaSettings::draft2019_09().into_generator();
        let schema =
            serde_json::to_value(generator.root_schema_for::<SignOff>()).expect("a schema");
        let variants = schema["definitions"]["SignOffBasis"]["oneOf"]
            .as_array()
            .expect("two variants")
            .clone();

        for basis in [
            SignOffBasis::RecognisedLabel {
                authority_layer: "CURRENT SOURCE".into(),
            },
            principal_act(),
        ] {
            let written = serde_json::to_value(&basis).expect("serialises");
            let keys: Vec<&String> = written.as_object().expect("an object").keys().collect();
            let declared = variants
                .iter()
                .find(|v| v["properties"]["kind"]["enum"][0].as_str() == Some(basis.as_str()))
                .unwrap_or_else(|| panic!("the schema declares no {:?} variant", basis.as_str()));
            for key in keys {
                assert!(
                    !declared["properties"][key].is_null(),
                    "a {} row writes {key:?}, and the published schema does not declare it. \
                     A schema that describes a shape the tool never writes is a contract \
                     nobody is held to.\n  written: {written}\n  declared: {declared}",
                    basis.as_str()
                );
            }
        }
    }

    /// The basis vocabulary is CLOSED at two. There is no variant for an
    /// attestation over the frames ledger as a whole, and one cannot be written
    /// into a row file either.
    #[test]
    fn the_basis_vocabulary_is_closed_at_two_and_has_no_aggregate() {
        assert!(serde_yaml::from_str::<SignOffBasis>("kind: ledger_aggregate\n").is_err());
        assert!(serde_yaml::from_str::<SignOffBasis>("kind: external_signature\n").is_err());
        assert!(
            serde_yaml::from_str::<SignOffBasis>(
                "kind: recognised_label\nauthorityLayer: CURRENT SOURCE\n"
            )
            .is_ok()
        );
    }

    #[test]
    fn a_matching_hash_is_signed_authority() {
        let rows = vec![signoff("1:2", "frame-v1")];
        let current = Digest::of_text("frame-v1");
        assert!(frame_authority("KEY", "1:2", Some(&current), &rows).is_signed());
    }

    /// The seed the mandate names: a signed frame whose hash then drifts must
    /// revert to UNSIGNED. Staleness by hash, never by trust.
    #[test]
    fn a_frame_that_changed_after_signoff_reverts_to_unsigned() {
        let rows = vec![signoff("1:2", "frame-v1")];
        let moved = Digest::of_text("frame-v2");
        let authority = frame_authority("KEY", "1:2", Some(&moved), &rows);
        assert!(!authority.is_signed());
        let FrameAuthority::Unsigned { because } = authority else {
            unreachable!()
        };
        assert!(because.contains("changed after sign-off"), "{because}");
        assert!(because.contains("never by trust"), "{because}");
    }

    #[test]
    fn a_never_signed_frame_is_unsigned_and_says_so_differently() {
        let current = Digest::of_text("frame-v1");
        let authority = frame_authority("KEY", "9:9", Some(&current), &[]);
        let FrameAuthority::Unsigned { because } = authority else {
            panic!("must be unsigned")
        };
        assert!(because.contains("never signed"), "{because}");
    }

    #[test]
    fn a_frame_with_no_measurable_current_content_is_unsigned_fail_closed() {
        let rows = vec![signoff("1:2", "frame-v1")];
        assert!(!frame_authority("KEY", "1:2", None, &rows).is_signed());
    }

    #[test]
    fn the_verdict_vocabulary_is_closed_at_three_and_has_no_acceptance() {
        assert!(serde_json::from_str::<AuthorityVerdict>("\"accepted\"").is_err());
        assert!(serde_json::from_str::<AuthorityVerdict>("\"no_authority\"").is_ok());
        assert!(serde_json::from_str::<RedrawStatus>("\"accepted\"").is_err());
    }

    fn external_evidence(at: &str, aggregate: &Digest) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "schemaVersion": 1,
            "kind": "external-principal-frame-signature",
            "signedBy": "Principal",
            "signedAt": at,
            "warrant": false,
            "scope": {
                "fileKey": "KEY",
                "aggregateDigest": aggregate,
            },
            "actualReply": "Signed off. proceed all"
        }))
        .unwrap()
    }

    #[test]
    fn external_evidence_binds_the_act_to_the_row_and_ledger() {
        let aggregate = Digest::of_text("the exact frames ledger");
        let bytes = external_evidence("2026-08-03T16:49:22Z", &aggregate);
        let evidence = ExternalSignOffEvidence::parse(&bytes, "evidence.json").unwrap();
        evidence
            .validate_for(
                "evidence.json",
                "KEY",
                "Principal",
                &Timestamp::parse("2026-08-03T16:49:22Z").unwrap(),
                &aggregate,
            )
            .unwrap();
    }

    /// Negative control: move the local aggregate while leaving the act
    /// untouched. The binding must fire rather than treating any JSON as
    /// evidence for any later ledger.
    #[test]
    fn external_evidence_refuses_a_different_frame_ledger() {
        let signed = Digest::of_text("signed ledger");
        let current = Digest::of_text("mutated ledger");
        let bytes = external_evidence("2026-08-03T16:49:22Z", &signed);
        let evidence = ExternalSignOffEvidence::parse(&bytes, "evidence.json").unwrap();
        let error = evidence
            .validate_for(
                "evidence.json",
                "KEY",
                "Principal",
                &Timestamp::parse("2026-08-03T16:49:22Z").unwrap(),
                &current,
            )
            .unwrap_err();
        assert!(
            error.to_string().contains("different signing population"),
            "{error}"
        );
    }

    /// Negative control: an external timestamp has to survive Timestamp's one
    /// canonical form even when it arrives inside otherwise valid JSON.
    #[test]
    fn external_evidence_refuses_a_malformed_timestamp() {
        let aggregate = Digest::of_text("the exact frames ledger");
        let bytes = external_evidence("2026-08-03T16:49:22+00:00", &aggregate);
        let error = ExternalSignOffEvidence::parse(&bytes, "evidence.json").unwrap_err();
        assert!(error.to_string().contains("canonical form"), "{error}");
    }

    #[test]
    fn an_evidence_reference_refuses_an_escaping_path() {
        let reference = SignOffEvidence {
            path: "../elsewhere/act.json".into(),
            digest: Digest::of_text("act"),
            frame_ledger_digest: Digest::of_text("ledger"),
        };
        assert!(reference.validate().is_err());
    }

    /// NEGATIVE CONTROL. A row citing a decision whose adopted locus is NOT its
    /// own node is the one shape the register must refer rather than hold. The
    /// door refuses it on the way in; this is the same refusal on the way out,
    /// for a row somebody edited afterwards.
    #[test]
    fn a_basis_citing_a_foreign_locus_is_refused_on_read() {
        let mut row = signoff("675:74319", "frame-v1");
        row.basis = Some(SignOffBasis::PrincipalAct {
            decision: DecisionReference {
                seq: 33,
                digest: Digest::of_text("seq 33"),
                locus_id: "1007:89086".into(),
                locus_name: "SOURCE AUTHORITY - /documents/packs/[id] - clone".into(),
            },
        });
        let error = row.check_basis().unwrap_err();
        assert!(error.to_string().contains("REFERRED"), "{error}");
    }

    /// And the spelling of a node id must not be able to trigger that refusal.
    /// Figma writes `12:34` in a URL and `12-34` in a deep link; the door
    /// normalises both, and so must this.
    #[test]
    fn the_two_figma_spellings_of_one_node_are_not_a_foreign_locus() {
        let mut row = signoff("675:74319", "frame-v1");
        row.basis = Some(SignOffBasis::PrincipalAct {
            decision: DecisionReference {
                seq: 30,
                digest: Digest::of_text("seq 30"),
                locus_id: "675-74319".into(),
                locus_name: "Screen - /x".into(),
            },
        });
        row.check_basis().expect("one node, two spellings");
    }
}
