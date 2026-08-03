//! The staged write: what an agent INTENDS to draw into a frame, reviewable
//! before anything reaches the canvas, and what an apply then did.
//!
//! Draft S-7E, ENACTMENT PENDING (SUBMISSION-VDS-020). The eleventh artefact
//! kind (S-4(1) was closed at ten) and the sixteenth proof kind (S-7(6) makes
//! that an amendment), both shipped carrying their DRAFTED marking on their own
//! faces.
//!
//! # WHAT THIS IS NOT, stated first because the brief that asked for it was
//! wrong about this
//!
//! This is NOT a mechanism that makes VDS the only writer to a Figma file, and
//! nothing here may be named, commented or documented as preventing a direct
//! write. Three measured facts settle it:
//!
//!   - the Figma REST API CANNOT WRITE DOCUMENT NODES. VDS holds no privileged
//!     write channel it could withhold from anyone; its own apply goes through
//!     the same plugin API every agent already has.
//!   - credential custody is the INVERSE of a control here. VDS reads
//!     `FIGMA_TOKEN` (`crates/vds-figma/src/pull.rs`); the plugin bridge writes
//!     through a desktop session and needs no token at all. Unsetting the token
//!     disables VDS's READS and leaves every agent's WRITE path untouched.
//!   - the estate's writer lock says on its own face that it is ADVISORY and
//!     "cannot stop a writer that does not ask for it".
//!
//! What this actually buys, in descending order of strength:
//!
//!   1. A CLOSED OPERATION VOCABULARY ([`StageOperation`]) that makes the
//!      2026-07-25 class of loss unrepeatable THROUGH THIS PATH. That incident
//!      came from a build step whose documented behaviour is to delete a page of
//!      a given name and recreate it, which discarded another writer's landed
//!      work. There is no page-level and no frame-level delete here, and
//!      [`StageOperation::DeleteBand`] reaches only a band whose name is in the
//!      closed review vocabulary AND which the intent no longer declares.
//!   2. REVIEWABILITY, which is the genuinely new thing: [`StagePlan`] is the
//!      operation list, on disk, BEFORE anything reaches the canvas. No such
//!      artefact existed.
//!   3. BYPASS DETECTION, which is the limb that changes behaviour without
//!      needing anyone's co-operation. See the `staged_write` proof's R5.
//!
//! # The record splits in two, on law rather than on tidiness
//!
//! [`StageIntent`] carries REALISATION - boxes and paints - so it lives in the
//! SUBSCRIBER TREE and never under `.vds/**`, exactly as
//! [`super::GeometryAuthority::capture`] states the rule for a saved REST
//! capture. Under `.vds/` it would fail `no_stored_values` R1 and R3 forever, on
//! a file VDS wrote itself, and [2026] VJS-FI-VDS 1 (orders 2 and 4) has refused
//! every narrowing that would rescue it. Its root is configured at
//! `[stage] intent_root`, and [`intent_root_defect`] REFUSES a root that
//! resolves under `.vds/`.
//!
//! [`StageRecord`] carries NO realisation: a route, a target frame, the intent's
//! path and digest, the input hashes, the three-valued gate verdicts and the
//! apply outcome. It sits BESIDE [`super::ScreenRecord`] and never on it:
//! that record's own test forbids a length or a colour in its serialised form,
//! and a stage is an EVENT rather than a contract.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::review::ReviewRegion;
use crate::digest::Digest;
use crate::error::{Result, VdsError};
use crate::ids::StageId;
use crate::project::Project;
use crate::timestamp::Timestamp;

// --------------------------------------------------------------- the target

/// The frame a staged write aims at.
///
/// Deliberately NOT [`super::FigmaFrame`], for the reason that type is
/// deliberately not [`super::FigmaNode`]: a screen's frame carries a capture
/// time because a screen record binds a contract to a drawing, and a stage
/// target binds nothing. It names where the write is going, and the moment it
/// was read is the PLAN's fact rather than the target's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StageTarget {
    pub file_key: String,
    /// A Figma node id, in either of the two spellings a designer copies.
    pub node_id: String,
}

// ---------------------------------------------------------------- the gates

/// The four gates a staged write is read against. CLOSED.
///
/// Closed for the reason [`ReviewRegion`] is: an open set lets one
/// undifferentiated "the checks passed" bucket back in, and a gate nobody can
/// name is a gate nobody can see did not run.
///
/// All four are limbs of ONE proof kind, `staged_write`, and never four kinds.
/// Four kinds would walk the same enumeration four times and let the four
/// disagree about which stages exist, which is the two-sources-of-truth failure
/// `visual_review` names when it folds band correspondence in rather than
/// filing a sixteenth kind for it.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum StageGate {
    /// G1. A staged control boundary clears the contrast floor in every theme
    /// the shipped stylesheet declares.
    ContrastFloor,
    /// G2. Every staged band is named from the CLOSED review vocabulary and is
    /// one the screen record declares. The limb that makes the diff key stable.
    BandNaming,
    /// G3. The boxes about to be written derive the column count the intent
    /// declares, under the same derivation `vds figma frames` runs afterwards.
    CanonicalGeometry,
    /// G4. No other artefact in the subscribing repository contradicts this
    /// route's binding to this frame.
    RouteBinding,
}

impl StageGate {
    pub const ALL: [StageGate; 4] = [
        StageGate::ContrastFloor,
        StageGate::BandNaming,
        StageGate::CanonicalGeometry,
        StageGate::RouteBinding,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            StageGate::ContrastFloor => "contrast_floor",
            StageGate::BandNaming => "band_naming",
            StageGate::CanonicalGeometry => "canonical_geometry",
            StageGate::RouteBinding => "route_binding",
        }
    }

    pub fn parse(raw: &str) -> Option<StageGate> {
        StageGate::ALL.into_iter().find(|g| g.as_str() == raw)
    }

    /// The limb label a finding cites, so a reader can find the rule.
    pub fn limb(self) -> &'static str {
        match self {
            StageGate::ContrastFloor => "draft S-7E(4) G1",
            StageGate::BandNaming => "draft S-7E(5) G2",
            StageGate::CanonicalGeometry => "draft S-7E(6) G3",
            StageGate::RouteBinding => "draft S-7E(7) G4",
        }
    }
}

impl std::fmt::Display for StageGate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What one gate READ. THREE-VALUED, and the third value is the whole point.
///
/// "Cleared" and "had no basis to run" are different facts, and one field
/// holding both reports the second as the first. On the estate this was written
/// for, the same class of collapse reported 143 unmeasured routes as clean, and
/// `screen_parity`'s coverage tally exists because a gate scored 32% of its
/// subject and would have printed zero deviations.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum GateReading {
    /// The gate ran, over something, and found nothing to refuse.
    Cleared,
    /// The gate ran and refuses this staged write.
    Refused,
    /// The gate had no basis to run: an input the estate has not supplied, or a
    /// declaration this intent does not make. NEVER a pass.
    CouldNotRun,
}

impl GateReading {
    pub const ALL: [GateReading; 3] = [
        GateReading::Cleared,
        GateReading::Refused,
        GateReading::CouldNotRun,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            GateReading::Cleared => "cleared",
            GateReading::Refused => "refused",
            GateReading::CouldNotRun => "could_not_run",
        }
    }

    pub fn parse(raw: &str) -> Option<GateReading> {
        GateReading::ALL.into_iter().find(|r| r.as_str() == raw)
    }

    /// Only a cleared reading admits a plan. A reading that could not run does
    /// not block the plan and does not licence it either: it is reported, and
    /// the coverage line says out loud how many rows it covers.
    pub fn admits_a_plan(self) -> bool {
        !matches!(self, GateReading::Refused)
    }
}

impl std::fmt::Display for GateReading {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One gate's reading on one staged write, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct GateVerdict {
    pub gate: StageGate,
    pub reading: GateReading,
    /// The reason, in the gate's own words, and required in EVERY reading.
    ///
    /// A cleared reading with no sentence cannot be told apart from a gate that
    /// returned early, and a `could_not_run` with no sentence hides which input
    /// was missing. Prose, and NOT exempt from VDS S-2(2): this record lands
    /// under `.vds/**`, which `no_stored_values` scans in full, so a reason
    /// names the CLASS of a value and never spells one.
    pub because: String,
}

impl GateVerdict {
    /// Why this verdict is invalid, or `None`.
    pub fn defect(&self) -> Option<String> {
        self.because.trim().is_empty().then(|| {
            format!(
                "gate {} reads {} and says nothing. A cleared reading with no sentence cannot be \
                 told apart from a gate that returned early, and a could_not_run with no \
                 sentence hides which input was missing.",
                self.gate, self.reading
            )
        })
    }
}

// -------------------------------------------------------------- the record

/// One input a gate read, and its digest at the moment it read it.
///
/// A NAME and a digest, never a path's contents. The stage record is the only
/// place that says what the verdicts above were computed over, and a verdict
/// whose inputs are not pinned is a verdict that silently survives them moving.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StageInput {
    /// Repository-relative, or a derived name for an input that is not a file.
    pub name: String,
    pub digest: Digest,
}

/// What one apply attempt did.
///
/// THERE IS NO ATOMICITY HERE AND THIS TYPE MUST NOT CLAIM ANY. The plugin
/// bridge caps a single call's code at a fixed character budget and offers no
/// transaction, so a large frame's plan goes over in ordered chunks and a
/// partial apply is reachable. That is why [`Self::verification`] is separate
/// and optional: applying is an attempt, and only a re-capture that recomputes
/// the delta and finds it EMPTY declares success.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ApplyOutcome {
    pub applied_at: Timestamp,
    pub applied_by: String,
    /// The ADVISORY writer lock this apply held, by holder name.
    ///
    /// Advisory, and the word is not decoration: the lock is a co-operating
    /// file protocol and cannot stop a writer that does not ask for it. What it
    /// converts is a silent collision into a loud refusal for every writer that
    /// does ask, and VDS taking it is VDS becoming one of those writers, which
    /// it was not before.
    pub lock_holder: String,
    /// How many ordered chunks the plan went over in.
    pub chunks: u32,
    /// How many operations those chunks carried.
    pub operations: u32,
    /// The plan's content digest at apply time, so an apply cannot claim a plan
    /// that has since been re-emitted.
    pub plan_digest: Digest,
    /// The measurement that declares success, or `None` while none has been
    /// taken. Never defaulted to a success.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification: Option<Verification>,
}

/// The re-capture that declares an apply finished. IDEMPOTENCE IS MEASURED AT
/// THE DESTINATION, never asserted in a comment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Verification {
    pub verified_at: Timestamp,
    /// The frame's content digest as the RE-CAPTURE read it. This is the value
    /// the bypass rule adds to the set of digests a frame may lawfully carry.
    pub frame_digest_after: Digest,
    /// How many operations the diff STILL emits against the re-captured frame.
    /// Zero, or the apply did not finish.
    pub residual_operations: u32,
}

impl Verification {
    pub fn succeeded(&self) -> bool {
        self.residual_operations == 0
    }
}

/// One staged write. The reviewable record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StageRecord {
    pub id: StageId,
    /// The route this write is about, as the screens ledger spells it.
    pub route: String,
    pub target: StageTarget,
    /// Where the INTENT lives, project-relative. Never under `.vds/`: it
    /// carries boxes and paints, and see the module note.
    pub intent_path: String,
    /// The intent's digest when the gates read it, so a plan cannot be emitted
    /// from one intent and applied from another.
    pub intent_digest: Digest,
    /// Every input the gates read, pinned.
    #[serde(default)]
    pub inputs: Vec<StageInput>,
    /// One verdict per gate that was asked. A gate missing from this list was
    /// never asked, which the proof reports rather than assuming cleared.
    #[serde(default)]
    pub gates: Vec<GateVerdict>,
    /// What an apply then did, or `None` while nothing has been applied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub apply: Option<ApplyOutcome>,
    pub staged_by: String,
    pub staged_at: Timestamp,
    #[serde(default)]
    pub basis: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl StageRecord {
    /// This stage's verdict for one gate, or `None` where it was never asked.
    pub fn verdict(&self, gate: StageGate) -> Option<&GateVerdict> {
        self.gates.iter().find(|v| v.gate == gate)
    }

    /// The gates that REFUSED this staged write.
    pub fn refusals(&self) -> Vec<&GateVerdict> {
        self.gates
            .iter()
            .filter(|v| v.reading == GateReading::Refused)
            .collect()
    }

    /// The gates that were never asked at all.
    ///
    /// Reported rather than treated as cleared. A gate absent from the record
    /// and a gate that cleared are the same green to anybody counting
    /// refusals, and that is how a gate stops running without anybody noticing.
    pub fn gates_not_asked(&self) -> Vec<StageGate> {
        StageGate::ALL
            .into_iter()
            .filter(|gate| self.verdict(*gate).is_none())
            .collect()
    }

    /// Whether an apply has been recorded AND verified empty at the frame.
    pub fn is_applied_and_verified(&self) -> bool {
        self.apply
            .as_ref()
            .and_then(|a| a.verification.as_ref())
            .is_some_and(Verification::succeeded)
    }

    /// Why this record is invalid, or an empty list.
    pub fn defects(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.route.trim().is_empty() {
            out.push(
                "names no route, so there is no subject: a staged write about nothing cannot be \
                 reviewed, applied or verified."
                    .to_owned(),
            );
        }
        if self.target.node_id.trim().is_empty() || self.target.file_key.trim().is_empty() {
            out.push(
                "names no target frame. A file key with no node id names a file and not a \
                 drawing, and a node id with no file key resolved against the wrong file returns \
                 \"not found\", which reads as a deleted frame and is really a wrong-file error."
                    .to_owned(),
            );
        }
        if let Some(why) = intent_path_defect(&self.intent_path) {
            out.push(why);
        }
        for verdict in &self.gates {
            if let Some(defect) = verdict.defect() {
                out.push(defect);
            }
        }
        let mut seen: Vec<StageGate> = self.gates.iter().map(|v| v.gate).collect();
        seen.sort();
        let before = seen.len();
        seen.dedup();
        if seen.len() != before {
            out.push(
                "records a gate twice. Two readings for one gate is two answers, and nothing \
                 says which governs."
                    .to_owned(),
            );
        }
        out
    }
}

/// Why an intent path may not be used, or `None`.
///
/// The one refusal that is a matter of law rather than of taste. An intent
/// carries boxes and paints; under `.vds/` those are the storing form
/// VDS S-2(2) prohibits, `no_stored_values` R1 and R3 would fail on them
/// forever, and a record is never deleted, so there would be no lawful way back.
pub fn intent_path_defect(path: &str) -> Option<String> {
    let normalised = path.replace('\\', "/");
    if normalised.trim().is_empty() {
        return Some(
            "names no intent file, so nothing says what was to be written. The stage record \
             holds no box and no paint by design; the intent is where they live."
                .to_owned(),
        );
    }
    let under_vds = normalised == ".vds"
        || normalised.starts_with(".vds/")
        || normalised.split('/').any(|segment| segment == ".vds");
    under_vds.then(|| {
        format!(
            "puts the intent at {path}, which resolves under `.vds/`. An intent carries boxes \
             and paints, which are REALISATIONS: under the record they are the storing form \
             VDS S-2(2) prohibits, `no_stored_values` R1 and R3 would fail on them forever on a \
             file VDS wrote itself, and [2026] VJS-FI-VDS 1 orders 2 and 4 refused every \
             narrowing that would rescue it. The intent lives in the subscriber tree, exactly \
             as a saved REST capture does. Move `[stage] intent_root` out of `.vds/`."
        )
    })
}

/// Why this project's configured intent root may not be used, or `None`.
pub fn intent_root_defect(project: &Project) -> Option<String> {
    intent_path_defect(&project.config.stage.intent_root.to_string_lossy())
}

// ------------------------------------------------------------- the intent

pub const STAGE_INTENT_SCHEMA_VERSION: u32 = 1;
pub const STAGE_PLAN_SCHEMA_VERSION: u32 = 1;
pub const ROUTE_BINDING_SCHEMA_VERSION: u32 = 1;

/// A rectangle, frame-relative.
///
/// A REALISATION, and it is lawful here for exactly one reason: this type is
/// only ever reached from [`StageIntent`] and [`StagePlan`], both of which live
/// in the SUBSCRIBER TREE. Nothing under `.vds/**` holds one, and the stage
/// record's own test holds that claim against its serialised bytes rather than
/// against this paragraph.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BandBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl BandBox {
    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }

    /// Whether this rectangle lies wholly inside `outer`, to the given slack.
    pub fn fits_inside(&self, outer: &BandBox, slack: f64) -> bool {
        self.x >= outer.x - slack
            && self.y >= outer.y - slack
            && self.right() <= outer.right() + slack
            && self.bottom() <= outer.bottom() + slack
    }
}

/// A paint a band is to carry: a NAME, a ROLE and the backdrop it is measured
/// against. Never a literal.
///
/// G1 refuses a literal, and the refusal is not pedantry. A literal cannot be
/// measured against the shipped stylesheet in every theme, so a staged boundary
/// spelled as a value is a boundary nothing can check, which is the founding
/// defect of this whole system wearing a Figma hat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PaintIntent {
    /// The custom property this paint takes its value from, with or without its
    /// leading dashes.
    pub property: String,
    /// What kind of thing the paint is. The SAME closed vocabulary the register
    /// uses for a contrast floor, reused rather than re-invented: two scope
    /// vocabularies for one question is one vocabulary and a disagreement.
    pub role: super::FloorScope,
    /// The custom property naming the surface behind it, without which no ratio
    /// exists.
    pub backdrop: String,
}

/// One band the intent declares, and ONLY the fields it declares.
///
/// PER BAND, ONLY DECLARED FIELDS ARE COMPARED. A field the intent does not
/// declare is neither compared nor written, so a designer's hand-added note
/// inside a band survives an apply untouched. This is
/// [`super::AgreementState::NotDrawn`] generalised: a frame binds only for what
/// it draws ([2026] VJS-SC-OPBOX 1 order 6), and an intent binds only for what
/// it declares.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BandIntent {
    /// WHICH BAND, from the CLOSED review vocabulary. This is the diff key.
    pub band: ReviewRegion,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub box_of: Option<BandBox>,
    /// The side-by-side content panes this band draws, where it draws any.
    ///
    /// Read by G3 and by nothing else: the column derivation clusters a
    /// container's children by x-interval, so the panes are what it needs and a
    /// band with none contributes one column exactly as a real frame does.
    #[serde(default)]
    pub panes: Vec<BandBox>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paint: Option<PaintIntent>,
    /// Where this band sits among the frame's bands, left-to-right then
    /// top-to-bottom, counting only bands the closed vocabulary names.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<u32>,
}

/// What a staged write intends the frame to be.
///
/// Lives in the SUBSCRIBER TREE under `[stage] intent_root`. See the module
/// note for why that is law and not filing preference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StageIntent {
    pub schema_version: u32,
    pub route: String,
    pub file_key: String,
    pub node_id: String,
    /// How many side-by-side content PANES the finished frame is to draw.
    ///
    /// Declared here and DERIVED by G3 from the boxes below, so the two can
    /// disagree and the disagreement is the finding. A count that was simply
    /// read off the boxes would agree with them by construction and check
    /// nothing.
    pub columns: u32,
    pub bands: Vec<BandIntent>,
    pub authored_by: String,
    pub authored_at: Timestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl StageIntent {
    /// The bands this intent declares, in the closed vocabulary.
    pub fn declared_bands(&self) -> Vec<ReviewRegion> {
        self.bands.iter().map(|b| b.band).collect()
    }

    pub fn band(&self, band: ReviewRegion) -> Option<&BandIntent> {
        self.bands.iter().find(|b| b.band == band)
    }

    /// Why this intent is unusable, or an empty list.
    pub fn defects(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.bands.is_empty() {
            out.push(
                "declares no band, so it intends nothing and the diff against it is empty by \
                 construction. An intent that cannot produce an operation is not a smaller \
                 intent, it is no intent."
                    .to_owned(),
            );
        }
        let mut seen: Vec<ReviewRegion> = self.bands.iter().map(|b| b.band).collect();
        seen.sort();
        let before = seen.len();
        seen.dedup();
        if seen.len() != before {
            out.push(
                "declares a band twice. THE DIFF IS KEYED ON BAND NAME, so two declarations for \
                 one key is two answers and nothing says which the apply would write."
                    .to_owned(),
            );
        }
        if self.columns == 0 {
            out.push(
                "declares 0 columns. Every arrangement draws at least one content pane, so \
                 nothing derived from any set of boxes can ever equal zero of them and G3 could \
                 not fail. A frame with no split declares 1."
                    .to_owned(),
            );
        }
        out
    }
}

// ----------------------------------------------------------- the operations

/// What a staged apply may do. CLOSED AT SIX, and the closure is the strongest
/// thing in this module.
///
/// THERE IS NO PAGE-LEVEL AND NO FRAME-LEVEL DELETE, AND THERE MUST NEVER BE.
/// The 2026-07-25 loss on the subscribing estate came from a build step whose
/// correct and documented behaviour is to delete a page of a given name and
/// recreate it; a second writer ran it and discarded work the first had landed.
/// Neither agent was at fault and the step was not wrong: the destructive verb
/// simply existed and two writers reached it. Widening this enum re-creates that
/// verb inside the sanctioned path, and the loss becomes repeatable through the
/// route that was built to prevent it. That is why the reason is written here in
/// the code and not only in a design note.
///
/// [`StageOperation::DeleteBand`] is admitted under two conditions checked
/// together and never separately: the band's name is in the closed review
/// vocabulary, AND the intent no longer declares it. A node VDS did not create
/// and the intent does not name is never touched at all.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum StageOperation {
    /// The intent declares a band the frame does not have.
    CreateBand { band: ReviewRegion, box_of: BandBox },
    /// The frame has the band and its rectangle is not the declared one.
    SetBox { band: ReviewRegion, box_of: BandBox },
    /// The band's layer name is a variant spelling of its own region.
    ///
    /// The band's IDENTITY is the region parsed from its layer name, so this
    /// operation moves a spelling onto the canonical one and is idempotent by
    /// construction: once the name is canonical it parses to the same region and
    /// the comparison is equal.
    SetName { band: ReviewRegion, to: String },
    /// The band's paint is not the one the named custom property resolves to.
    SetPaint {
        band: ReviewRegion,
        /// The custom property the paint takes its value from. The NAME travels
        /// with the operation so a reader of the plan can see which token was
        /// staged, not only what it happened to resolve to.
        property: String,
        /// What that property resolves to in the base scope of the shipped
        /// stylesheet. A realisation, lawful because a plan lives in the
        /// subscriber tree.
        resolved: String,
    },
    /// The band sits at a different index among the frame's bands.
    Reorder { band: ReviewRegion, to: u32 },
    /// The frame carries a band whose name IS in the closed vocabulary and which
    /// the intent no longer declares. The only destructive verb, and its reach
    /// is one band.
    DeleteBand { band: ReviewRegion },
}

impl StageOperation {
    /// The band this operation touches. Every variant has one, which is the
    /// enum's other guarantee: there is no operation whose subject is a page, a
    /// frame or a file.
    pub fn band(&self) -> ReviewRegion {
        match self {
            StageOperation::CreateBand { band, .. }
            | StageOperation::SetBox { band, .. }
            | StageOperation::SetName { band, .. }
            | StageOperation::SetPaint { band, .. }
            | StageOperation::Reorder { band, .. }
            | StageOperation::DeleteBand { band } => *band,
        }
    }

    pub fn verb(&self) -> &'static str {
        match self {
            StageOperation::CreateBand { .. } => "create-band",
            StageOperation::SetBox { .. } => "set-box",
            StageOperation::SetName { .. } => "set-name",
            StageOperation::SetPaint { .. } => "set-paint",
            StageOperation::Reorder { .. } => "reorder",
            StageOperation::DeleteBand { .. } => "delete-band",
        }
    }

    pub fn is_destructive(&self) -> bool {
        matches!(self, StageOperation::DeleteBand { .. })
    }
}

impl std::fmt::Display for StageOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.verb(), self.band())
    }
}

// ------------------------------------------------------------------ the plan

/// One chunk of a plan, digest-pinned.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PlanChunk {
    /// Position in the ordered sequence, from 1. The order is part of the plan:
    /// a create must precede the set-box that positions what it created.
    pub ordinal: u32,
    pub operations: Vec<StageOperation>,
    /// A digest over this chunk's operations, so a chunk cannot be reordered,
    /// dropped or edited between emission and apply without saying so.
    pub digest: Digest,
}

impl PlanChunk {
    pub fn compute_digest(ordinal: u32, operations: &[StageOperation]) -> Result<Digest> {
        #[derive(Serialize)]
        struct Content<'a> {
            ordinal: u32,
            operations: &'a [StageOperation],
        }
        Digest::of_value(&Content {
            ordinal,
            operations,
        })
    }
}

/// The operation list, on disk, BEFORE anything reaches the canvas.
///
/// The genuinely new thing this whole capability buys. It lives in the
/// SUBSCRIBER TREE beside the intent, because its operations carry boxes and
/// resolved paints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StagePlan {
    pub schema_version: u32,
    pub stage: StageId,
    pub route: String,
    pub file_key: String,
    pub node_id: String,
    pub emitted_by: String,
    pub emitted_at: Timestamp,
    /// The saved capture the diff was taken against, project-relative.
    pub reading: String,
    /// That capture's digest, so a plan cannot be applied against a frame
    /// reading it was never computed from.
    pub reading_digest: Digest,
    pub intent_digest: Digest,
    /// The bands present in the reading whose names the closed vocabulary does
    /// NOT name. Recorded and never touched: a node VDS did not create and the
    /// intent does not name is out of reach of every operation above.
    #[serde(default)]
    pub untouched: Vec<String>,
    pub chunks: Vec<PlanChunk>,
    pub content_digest: Digest,
}

impl StagePlan {
    pub fn operations(&self) -> impl Iterator<Item = &StageOperation> {
        self.chunks.iter().flat_map(|c| c.operations.iter())
    }

    pub fn operation_count(&self) -> usize {
        self.chunks.iter().map(|c| c.operations.len()).sum()
    }

    pub fn compute_content_digest(&self) -> Result<Digest> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Content<'a> {
            schema_version: u32,
            stage: &'a StageId,
            route: &'a str,
            file_key: &'a str,
            node_id: &'a str,
            reading: &'a str,
            reading_digest: &'a Digest,
            intent_digest: &'a Digest,
            untouched: &'a [String],
            chunks: &'a [PlanChunk],
        }
        Digest::of_value(&Content {
            schema_version: self.schema_version,
            stage: &self.stage,
            route: &self.route,
            file_key: &self.file_key,
            node_id: &self.node_id,
            reading: &self.reading,
            reading_digest: &self.reading_digest,
            intent_digest: &self.intent_digest,
            untouched: &self.untouched,
            chunks: &self.chunks,
        })
    }

    pub fn untrustworthy_because(&self) -> Result<Option<String>> {
        let recomputed = self.compute_content_digest()?;
        if recomputed != self.content_digest {
            return Ok(Some(format!(
                "the plan's contentDigest is {} and its content digests to {recomputed}. It was \
                 edited after it was emitted, and an edited plan is an operation list nobody \
                 reviewed. Re-emit it rather than correcting the digest by hand.",
                self.content_digest
            )));
        }
        for chunk in &self.chunks {
            let recomputed = PlanChunk::compute_digest(chunk.ordinal, &chunk.operations)?;
            if recomputed != chunk.digest {
                return Ok(Some(format!(
                    "chunk {} carries digest {} and its operations digest to {recomputed}. A \
                     chunk edited between emission and apply is the half of the plan nobody \
                     read.",
                    chunk.ordinal, chunk.digest
                )));
            }
        }
        Ok(None)
    }
}

// -------------------------------------------------- the route binding ledger

/// One claim, made by some artefact in the subscribing repository, that a route
/// is drawn by a node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RouteBinding {
    pub route: String,
    pub node_id: String,
    /// Where the claim is made, so a reader can open it and see for themselves.
    /// VDS does not decide which claim is true; it names both.
    pub claimed_at: String,
}

/// The subscribing repository's OWN claims about which frame draws which route.
///
/// A ledger under VDS S-4(2): generated, digest-witnessed, byte-reproducible by
/// the named command. Supplied by the subject and never derived here, for the
/// reason [`super::RouteManifest`] is supplied: which artefact in the estate
/// speaks for a route binding is the estate's question, and VDS deciding it
/// would make VDS the authority on the estate's own record.
///
/// It holds routes and node IDS and nothing else. Neither is a design value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RouteBindingLedger {
    pub schema_version: u32,
    pub generated_by: String,
    pub taken_at: Timestamp,
    /// The artefact the claims were read FROM, in the estate's own words.
    pub source: String,
    pub rows: Vec<RouteBinding>,
    #[serde(default)]
    pub does_not_cover: Vec<String>,
    pub content_digest: Digest,
}

impl RouteBindingLedger {
    /// Every claim about one route.
    pub fn claims_for(&self, route: &str) -> Vec<&RouteBinding> {
        self.rows.iter().filter(|r| r.route == route).collect()
    }

    pub fn compute_content_digest(&self) -> Result<Digest> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Content<'a> {
            schema_version: u32,
            generated_by: &'a str,
            taken_at: &'a Timestamp,
            source: &'a str,
            rows: &'a [RouteBinding],
            does_not_cover: &'a [String],
        }
        Digest::of_value(&Content {
            schema_version: self.schema_version,
            generated_by: &self.generated_by,
            taken_at: &self.taken_at,
            source: &self.source,
            rows: &self.rows,
            does_not_cover: &self.does_not_cover,
        })
    }

    pub fn untrustworthy_because(&self) -> Result<Option<String>> {
        let recomputed = self.compute_content_digest()?;
        Ok((recomputed != self.content_digest).then(|| {
            format!(
                "the route binding ledger's contentDigest is {} and its content digests to \
                 {recomputed}. It was edited after it was generated, and a contradicting claim \
                 quietly deleted from it is a contradiction that stops being reported. \
                 Regenerate it rather than correcting the digest by hand.",
                self.content_digest
            )
        }))
    }
}

// ------------------------------------------------------------------------ io

pub fn route_bindings_path(project: &Project) -> std::path::PathBuf {
    project.root.join(&project.config.stage.route_bindings)
}

pub fn write_route_bindings(
    project: &Project,
    ledger: &RouteBindingLedger,
) -> Result<std::path::PathBuf> {
    let path = route_bindings_path(project);
    let text = serde_yaml::to_string(ledger).map_err(|e| VdsError::Serialize {
        what: "the route binding ledger".into(),
        message: e.to_string(),
    })?;
    crate::write_text_atomically(&path, &text)?;
    Ok(path)
}

/// Read the route binding ledger, or `None` where the estate has supplied none.
///
/// `None` is NOT "nothing contradicts". G4 reports `could_not_run` on it, and
/// the coverage line says so out loud: a single unopposed self-claim must never
/// read as agreement.
pub fn read_route_bindings(project: &Project) -> Result<Option<RouteBindingLedger>> {
    let path = route_bindings_path(project);
    if !path.is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path).map_err(|e| VdsError::io(path.display(), e))?;
    let raw: serde_yaml::Value = serde_yaml::from_str(&text).map_err(|e| VdsError::Artefact {
        path: project.rel(&path),
        message: format!("is not readable YAML: {e}"),
    })?;
    let found = raw
        .get("schemaVersion")
        .or_else(|| raw.get("schema_version"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    if found > ROUTE_BINDING_SCHEMA_VERSION {
        return Err(VdsError::SchemaVersionAhead {
            path: project.rel(&path),
            kind: "route binding ledger",
            found,
            understood: ROUTE_BINDING_SCHEMA_VERSION,
        });
    }
    let ledger: RouteBindingLedger =
        serde_yaml::from_value(raw).map_err(|e| VdsError::Artefact {
            path: project.rel(&path),
            message: format!("is not a route binding ledger: {e}"),
        })?;
    Ok(Some(ledger))
}

/// Where a stage's intent lives, under the configured root.
pub fn intent_path(project: &Project, id: &StageId) -> std::path::PathBuf {
    project
        .root
        .join(&project.config.stage.intent_root)
        .join(format!("{id}.intent.yaml"))
}

/// Where a stage's plan lives: beside its intent, because it carries the same
/// realisations.
pub fn plan_path(project: &Project, id: &StageId) -> std::path::PathBuf {
    project
        .root
        .join(&project.config.stage.intent_root)
        .join(format!("{id}.plan.yaml"))
}

pub fn read_intent(project: &Project, path: &std::path::Path) -> Result<StageIntent> {
    if let Some(why) = intent_path_defect(&project.rel(path)) {
        return Err(VdsError::precondition(why));
    }
    let text = std::fs::read_to_string(path).map_err(|e| VdsError::io(path.display(), e))?;
    let raw: serde_yaml::Value = serde_yaml::from_str(&text).map_err(|e| VdsError::Artefact {
        path: project.rel(path),
        message: format!("is not readable YAML: {e}"),
    })?;
    let found = raw
        .get("schemaVersion")
        .or_else(|| raw.get("schema_version"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    if found > STAGE_INTENT_SCHEMA_VERSION {
        return Err(VdsError::SchemaVersionAhead {
            path: project.rel(path),
            kind: "stage intent",
            found,
            understood: STAGE_INTENT_SCHEMA_VERSION,
        });
    }
    serde_yaml::from_value(raw).map_err(|e| VdsError::Artefact {
        path: project.rel(path),
        message: format!("is not a stage intent: {e}"),
    })
}

pub fn write_intent(
    project: &Project,
    path: &std::path::Path,
    intent: &StageIntent,
) -> Result<std::path::PathBuf> {
    if let Some(why) = intent_path_defect(&project.rel(path)) {
        return Err(VdsError::precondition(why));
    }
    let text = serde_yaml::to_string(intent).map_err(|e| VdsError::Serialize {
        what: "the stage intent".into(),
        message: e.to_string(),
    })?;
    crate::write_text_atomically(path, &text)?;
    Ok(path.to_path_buf())
}

pub fn read_plan(project: &Project, path: &std::path::Path) -> Result<Option<StagePlan>> {
    if !path.is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(path).map_err(|e| VdsError::io(path.display(), e))?;
    let raw: serde_yaml::Value = serde_yaml::from_str(&text).map_err(|e| VdsError::Artefact {
        path: project.rel(path),
        message: format!("is not readable YAML: {e}"),
    })?;
    let found = raw
        .get("schemaVersion")
        .or_else(|| raw.get("schema_version"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    if found > STAGE_PLAN_SCHEMA_VERSION {
        return Err(VdsError::SchemaVersionAhead {
            path: project.rel(path),
            kind: "stage plan",
            found,
            understood: STAGE_PLAN_SCHEMA_VERSION,
        });
    }
    let plan: StagePlan = serde_yaml::from_value(raw).map_err(|e| VdsError::Artefact {
        path: project.rel(path),
        message: format!("is not a stage plan: {e}"),
    })?;
    Ok(Some(plan))
}

pub fn write_plan(
    project: &Project,
    path: &std::path::Path,
    plan: &StagePlan,
) -> Result<std::path::PathBuf> {
    if let Some(why) = intent_path_defect(&project.rel(path)) {
        return Err(VdsError::precondition(why));
    }
    let text = serde_yaml::to_string(plan).map_err(|e| VdsError::Serialize {
        what: "the stage plan".into(),
        message: e.to_string(),
    })?;
    crate::write_text_atomically(path, &text)?;
    Ok(path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::default_config;

    fn target() -> StageTarget {
        StageTarget {
            file_key: "KEY".into(),
            node_id: "669:171003".into(),
        }
    }

    fn record() -> StageRecord {
        StageRecord {
            id: StageId::parse("STG-0001").unwrap(),
            route: "/stakeholders/settings".into(),
            target: target(),
            intent_path: "design/stage/STG-0001.intent.yaml".into(),
            intent_digest: Digest::of_text("intent"),
            inputs: vec![StageInput {
                name: "app/globals.css".into(),
                digest: Digest::of_text("sheet"),
            }],
            gates: vec![GateVerdict {
                gate: StageGate::BandNaming,
                reading: GateReading::Cleared,
                because: "every staged band is in the closed vocabulary".into(),
            }],
            apply: None,
            staged_by: "an agent".into(),
            staged_at: Timestamp::fixed(2026, 8, 3, 10, 0, 0),
            basis: vec!["draft S-7E".into()],
            notes: None,
        }
    }

    #[test]
    fn a_stage_record_round_trips_and_refuses_an_unknown_field() {
        let r = record();
        let text = serde_yaml::to_string(&r).unwrap();
        assert_eq!(serde_yaml::from_str::<StageRecord>(&text).unwrap(), r);
        assert!(serde_yaml::from_str::<StageRecord>(&format!("{text}surprise: 1\n")).is_err());
    }

    /// VDS S-2(4), held against the BYTES rather than against the module note.
    /// The stage record is the half of this capability that lands under
    /// `.vds/**`, and the intent is the half that must not.
    #[test]
    fn a_serialised_stage_record_names_no_realisation() {
        let mut r = record();
        r.apply = Some(ApplyOutcome {
            applied_at: Timestamp::fixed(2026, 8, 3, 11, 0, 0),
            applied_by: "an agent".into(),
            lock_holder: "vds-stage-STG-0001".into(),
            chunks: 2,
            operations: 9,
            plan_digest: Digest::of_text("plan"),
            verification: Some(Verification {
                verified_at: Timestamp::fixed(2026, 8, 3, 11, 5, 0),
                frame_digest_after: Digest::of_text("after"),
                residual_operations: 0,
            }),
        });
        let text = serde_yaml::to_string(&r).unwrap();
        for forbidden in ["px", "rem", "width", "height", "colour", "color", "#"] {
            assert!(
                !text.contains(forbidden),
                "the stage record serialises {forbidden:?}, which is a realisation. The boxes \
                 and paints live in the INTENT, in the subscriber tree (VDS S-2(4)): {text}"
            );
        }
    }

    #[test]
    fn an_intent_under_the_record_is_refused_by_law_and_not_by_taste() {
        for path in [
            ".vds/stages/STG-0001.intent.yaml",
            ".vds",
            "a/b/.vds/intent.yaml",
        ] {
            let why = intent_path_defect(path).unwrap_or_else(|| panic!("{path} must be refused"));
            assert!(why.contains("no_stored_values"), "{why}");
            assert!(
                why.contains("VJS-FI-VDS 1"),
                "the refusal must cite the ruling that closed the escape route: {why}"
            );
        }
        assert!(intent_path_defect("design/stage/STG-0001.intent.yaml").is_none());
        // A path that merely MENTIONS the record directory is not under it.
        assert!(intent_path_defect("design/vds-stage/x.yaml").is_none());
    }

    #[test]
    fn an_intent_under_the_record_refuses_the_whole_stage_record() {
        let mut r = record();
        r.intent_path = ".vds/stages/STG-0001.intent.yaml".into();
        assert!(
            r.defects().iter().any(|d| d.contains("resolves under")),
            "{:?}",
            r.defects()
        );
    }

    #[test]
    fn the_default_intent_root_is_outside_the_record() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".vds")).unwrap();
        std::fs::write(
            tmp.path().join(".vds/config.toml"),
            default_config("demo", "DEMO"),
        )
        .unwrap();
        let project = Project::discover(Some(tmp.path())).unwrap();
        assert!(
            intent_root_defect(&project).is_none(),
            "the shipped default must not be the one shape the law forbids"
        );
    }

    #[test]
    fn a_gate_reading_is_three_valued_and_could_not_run_is_not_a_pass() {
        assert_eq!(GateReading::ALL.len(), 3);
        for reading in GateReading::ALL {
            assert_eq!(GateReading::parse(reading.as_str()), Some(reading));
        }
        assert!(GateReading::parse("passed").is_none());
        assert!(GateReading::Cleared.admits_a_plan());
        assert!(!GateReading::Refused.admits_a_plan());
        assert!(
            GateReading::CouldNotRun.admits_a_plan(),
            "a rule with no basis to run neither blocks nor licenses; it is reported, and the \
             coverage line says how many rows it covers"
        );
    }

    #[test]
    fn a_gate_that_says_nothing_is_refused_in_every_reading() {
        for reading in GateReading::ALL {
            let verdict = GateVerdict {
                gate: StageGate::ContrastFloor,
                reading,
                because: "  ".into(),
            };
            assert!(
                verdict.defect().unwrap().contains("says nothing"),
                "a {reading} reading with no sentence must be refused too"
            );
        }
    }

    #[test]
    fn a_gate_never_asked_is_reported_and_not_treated_as_cleared() {
        let r = record();
        let missing = r.gates_not_asked();
        assert_eq!(missing.len(), 3, "{missing:?}");
        assert!(!missing.contains(&StageGate::BandNaming));
        assert!(
            r.refusals().is_empty(),
            "a gate that was never asked must not be counted as a refusal either; it is its own \
             fact"
        );
    }

    /// The closure this whole module exists for.
    #[test]
    fn the_operation_vocabulary_is_closed_and_carries_no_page_or_frame_delete() {
        let text = serde_json::to_string(&StageOperation::DeleteBand {
            band: ReviewRegion::Rail,
        })
        .unwrap();
        assert!(text.contains("delete_band"), "{text}");
        // Each of these is a WELL-FORMED operation document naming a verb the
        // vocabulary does not carry, so the refusal comes from the closure and
        // not from malformed JSON. `delete_page` is the one that matters: the
        // 2026-07-25 loss came from a delete-page-and-recreate step, and
        // widening this enum makes that loss repeatable through the sanctioned
        // path.
        for absent in [
            r#"{"op":"delete_page","band":"rail"}"#,
            r#"{"op":"delete_frame","band":"rail"}"#,
            r#"{"op":"delete_node","band":"rail"}"#,
            r#"{"op":"replace_page","band":"rail"}"#,
        ] {
            assert!(
                serde_json::from_str::<StageOperation>(absent).is_err(),
                "{absent} must be unrepresentable, not merely discouraged"
            );
        }
        // And the control that proves the four above are refused for their VERB
        // and not for their shape: the same document with a lawful verb parses.
        assert!(
            serde_json::from_str::<StageOperation>(r#"{"op":"delete_band","band":"rail"}"#).is_ok()
        );
        assert!(
            StageOperation::DeleteBand {
                band: ReviewRegion::Rail
            }
            .is_destructive()
        );
        assert!(
            !StageOperation::SetBox {
                band: ReviewRegion::Rail,
                box_of: BandBox {
                    x: 0.0,
                    y: 0.0,
                    width: 1.0,
                    height: 1.0
                }
            }
            .is_destructive()
        );
    }

    #[test]
    fn every_operation_names_one_band_and_never_a_page() {
        let ops = [
            StageOperation::CreateBand {
                band: ReviewRegion::Header,
                box_of: BandBox {
                    x: 0.0,
                    y: 0.0,
                    width: 2.0,
                    height: 2.0,
                },
            },
            StageOperation::SetName {
                band: ReviewRegion::Footer,
                to: "footer".into(),
            },
            StageOperation::SetPaint {
                band: ReviewRegion::Rail,
                property: "--border-control".into(),
                resolved: "a value the sheet resolves".into(),
            },
            StageOperation::Reorder {
                band: ReviewRegion::Facets,
                to: 2,
            },
            StageOperation::DeleteBand {
                band: ReviewRegion::Keyboard,
            },
        ];
        for op in &ops {
            // The compiler already guarantees this; the test states the
            // guarantee so a future variant with a page-level subject fails
            // here as well as at the enum.
            let _: ReviewRegion = op.band();
        }
    }

    #[test]
    fn an_intent_declaring_one_band_twice_is_refused_because_the_diff_is_keyed_on_the_name() {
        let mut intent = intent();
        intent.bands.push(BandIntent {
            band: ReviewRegion::Header,
            box_of: None,
            panes: vec![],
            paint: None,
            order: None,
        });
        let defects = intent.defects();
        assert!(
            defects.iter().any(|d| d.contains("KEYED ON BAND NAME")),
            "{defects:?}"
        );
    }

    #[test]
    fn an_intent_declaring_zero_columns_could_not_fail_g3() {
        let mut intent = intent();
        intent.columns = 0;
        assert!(
            intent
                .defects()
                .iter()
                .any(|d| d.contains("could not fail")),
            "{:?}",
            intent.defects()
        );
    }

    fn intent() -> StageIntent {
        StageIntent {
            schema_version: STAGE_INTENT_SCHEMA_VERSION,
            route: "/stakeholders/settings".into(),
            file_key: "KEY".into(),
            node_id: "669:172814".into(),
            columns: 1,
            bands: vec![BandIntent {
                band: ReviewRegion::Header,
                box_of: Some(BandBox {
                    x: 0.0,
                    y: 0.0,
                    width: 100.0,
                    height: 10.0,
                }),
                panes: vec![],
                paint: None,
                order: Some(0),
            }],
            authored_by: "an agent".into(),
            authored_at: Timestamp::fixed(2026, 8, 3, 9, 0, 0),
            notes: None,
        }
    }

    #[test]
    fn an_edited_plan_is_untrustworthy_by_its_own_digest_and_by_its_chunks() {
        let operations = vec![StageOperation::CreateBand {
            band: ReviewRegion::Header,
            box_of: BandBox {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 10.0,
            },
        }];
        let mut plan = StagePlan {
            schema_version: STAGE_PLAN_SCHEMA_VERSION,
            stage: StageId::parse("STG-0001").unwrap(),
            route: "/stakeholders/settings".into(),
            file_key: "KEY".into(),
            node_id: "669:172814".into(),
            emitted_by: "vds stage plan".into(),
            emitted_at: Timestamp::fixed(2026, 8, 3, 10, 0, 0),
            reading: "design/captures/stakeholders.json".into(),
            reading_digest: Digest::of_text("capture"),
            intent_digest: Digest::of_text("intent"),
            untouched: vec!["a layer nobody named".into()],
            chunks: vec![PlanChunk {
                ordinal: 1,
                digest: PlanChunk::compute_digest(1, &operations).unwrap(),
                operations,
            }],
            content_digest: Digest::of_text("placeholder"),
        };
        plan.content_digest = plan.compute_content_digest().unwrap();
        assert!(plan.untrustworthy_because().unwrap().is_none());
        assert_eq!(plan.operation_count(), 1);

        let mut edited = plan.clone();
        edited.chunks[0]
            .operations
            .push(StageOperation::DeleteBand {
                band: ReviewRegion::Rail,
            });
        let why = edited.untrustworthy_because().unwrap().expect("a reason");
        assert!(
            why.contains("nobody reviewed") || why.contains("nobody read"),
            "{why}"
        );
    }

    #[test]
    fn an_edited_route_binding_ledger_is_untrustworthy_by_its_own_digest() {
        let mut ledger = RouteBindingLedger {
            schema_version: ROUTE_BINDING_SCHEMA_VERSION,
            generated_by: "vds ledger route-bindings --from -".into(),
            taken_at: Timestamp::fixed(2026, 8, 3, 9, 0, 0),
            source: "the estate's frame registry".into(),
            rows: vec![
                RouteBinding {
                    route: "/stakeholders/settings".into(),
                    node_id: "669:173031".into(),
                    claimed_at: "internal-docs/design/frame-registry.json".into(),
                },
                RouteBinding {
                    route: "/matters".into(),
                    node_id: "1:2".into(),
                    claimed_at: "internal-docs/design/frame-registry.json".into(),
                },
            ],
            does_not_cover: vec![],
            content_digest: Digest::of_text("placeholder"),
        };
        ledger.content_digest = ledger.compute_content_digest().unwrap();
        assert!(ledger.untrustworthy_because().unwrap().is_none());
        assert_eq!(ledger.claims_for("/stakeholders/settings").len(), 1);
        assert!(ledger.claims_for("/nowhere").is_empty());
        // The realistic edit: the contradicting claim is deleted.
        ledger.rows.remove(0);
        assert!(ledger.untrustworthy_because().unwrap().is_some());
    }

    #[test]
    fn a_verification_declares_success_only_on_an_empty_residual() {
        let mut v = Verification {
            verified_at: Timestamp::fixed(2026, 8, 3, 11, 0, 0),
            frame_digest_after: Digest::of_text("after"),
            residual_operations: 0,
        };
        assert!(v.succeeded());
        v.residual_operations = 1;
        assert!(
            !v.succeeded(),
            "a partial apply is reachable by construction: the bridge caps one call's code and \
             offers no transaction, so success is measured and never assumed"
        );
    }
}
