//! Proof kinds and proof results.
//!
//! VDS S-7(5) fixes eleven proof kinds as a CLOSED registry, and VDS S-7(6)
//! makes adding one an amendment to the specification and the invariant registry
//! rather than a script anyone may drop in. [`ProofKind`] is therefore an enum:
//! a kind outside the registry does not fail validation, it fails to compile.
//!
//! The eleventh, `screen_parity`, was added by amendment on 2026-07-30. The
//! first ten all read a COMPONENT, and the two that say "screen" read a screen's
//! REFERENCES rather than its arrangement, so a page could render every
//! registered component in an arrangement its frame does not draw and every kind
//! stayed green. S-7(6) is what makes that an amendment here rather than a
//! script somebody dropped in.
//!
//! VDS S-7(2)(5) requires a proof record to be written by the checker as a side
//! effect of running, and fixes `capture_mode` to the single value `automatic`.
//! [`CaptureMode`] is a one-variant enum for that reason. There is no
//! constructor on [`ProofResult`] that takes a status: the status is derived from
//! the run in [`crate::run::ProofRun`], so a passing record cannot be authored
//! for a run that failed.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::digest::Digest;
use crate::ids::{ProofId, WarrantId};
use crate::timestamp::Timestamp;

/// The closed registry at VDS S-7(5).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ProofKind {
    /// Every component referenced by any declared screen exists in the register.
    RegisterCompleteness,
    /// The register agrees with the codebase and with Figma, both directions.
    Reconciliation,
    /// No screen uses an unregistered component. The anti-drift proof.
    Composition,
    /// Every registered component's boundaries clear their floors in every theme.
    Contrast,
    /// Every required state of every registered component is drawn.
    States,
    /// Each registered component's code counterpart matches its contract.
    Parity,
    /// The two named records agree where the pin declares them aligned.
    TokenPin,
    /// A component proposed for retirement has zero remaining consumers.
    RetirementDrain,
    /// Each generated ledger is current with its source.
    LedgerStaleness,
    /// `.vds/**` holds no realisation.
    NoStoredValues,
    /// Each registered screen's required arrangement is the arrangement its
    /// authoritative frame draws. The only kind whose subject is a SCREEN.
    ScreenParity,
    /// Each registered surface's SHAPE is the one the design system specifies,
    /// and the count that does not comply is BOUNDED AND FALLING. The only kind
    /// that carries a DIRECTION rather than a threshold.
    Geometry,
    /// A registered pattern is ABSENT from its enumerated scope, and the scope
    /// cannot silently narrow. Draft S-7B, enactment pending.
    Prohibition,
    /// A pinned numeric reading whose only lawful direction is down: red on any
    /// increase AND on a decrease that was not re-pinned. Draft S-7C, enactment
    /// pending.
    Burndown,
    /// The recorded visual verdicts hold: shipped screenshot against signed
    /// frame, stale on either side moving, no conformance claim without
    /// authority. Draft S-7D, enactment pending.
    VisualReview,
}

impl ProofKind {
    pub const ALL: [ProofKind; 15] = [
        ProofKind::RegisterCompleteness,
        ProofKind::Reconciliation,
        ProofKind::Composition,
        ProofKind::Contrast,
        ProofKind::States,
        ProofKind::Parity,
        ProofKind::TokenPin,
        ProofKind::RetirementDrain,
        ProofKind::LedgerStaleness,
        ProofKind::NoStoredValues,
        ProofKind::ScreenParity,
        ProofKind::Geometry,
        ProofKind::Prohibition,
        ProofKind::Burndown,
        ProofKind::VisualReview,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ProofKind::RegisterCompleteness => "register_completeness",
            ProofKind::Reconciliation => "reconciliation",
            ProofKind::Composition => "composition",
            ProofKind::Contrast => "contrast",
            ProofKind::States => "states",
            ProofKind::Parity => "parity",
            ProofKind::TokenPin => "token_pin",
            ProofKind::RetirementDrain => "retirement_drain",
            ProofKind::LedgerStaleness => "ledger_staleness",
            ProofKind::NoStoredValues => "no_stored_values",
            ProofKind::ScreenParity => "screen_parity",
            ProofKind::Geometry => "geometry",
            ProofKind::Prohibition => "prohibition",
            ProofKind::Burndown => "burndown",
            ProofKind::VisualReview => "visual_review",
        }
    }

    pub fn parse(raw: &str) -> Option<ProofKind> {
        ProofKind::ALL.into_iter().find(|k| k.as_str() == raw)
    }

    /// What the kind establishes, in one line, for `vds proof --list`.
    pub fn establishes(self) -> &'static str {
        match self {
            ProofKind::RegisterCompleteness => {
                "every component referenced by any declared screen exists in the register"
            }
            ProofKind::Reconciliation => {
                "the register agrees with Figma and with the codebase, both directions"
            }
            ProofKind::Composition => "no screen uses an unregistered component",
            ProofKind::Contrast => {
                "every registered component's boundaries clear their floors in every theme"
            }
            ProofKind::States => "every required state of every registered component is drawn",
            ProofKind::Parity => {
                "each registered component's code counterpart matches its props and states"
            }
            ProofKind::TokenPin => {
                "the two named records agree where the pin declares them aligned"
            }
            ProofKind::RetirementDrain => {
                "a component proposed for retirement has zero remaining consumers"
            }
            ProofKind::LedgerStaleness => "each generated ledger is current with its source",
            ProofKind::NoStoredValues => "`.vds/**` holds no realisation",
            ProofKind::ScreenParity => {
                "each registered screen's required arrangement is the one its authoritative \
                 frame draws"
            }
            ProofKind::Geometry => {
                "each registered surface's SHAPE is the one the design system specifies, and \
                 the count that does not comply is bounded AND falling"
            }
            ProofKind::Prohibition => {
                "each registered pattern is ABSENT from its enumerated scope, and the scope \
                 has not narrowed since it was recorded"
            }
            ProofKind::Burndown => {
                "each pinned metric reads exactly its pin: any increase is red, and a \
                 decrease not re-pinned is red too"
            }
            ProofKind::VisualReview => {
                "each recorded visual verdict still holds: shipped screenshot against SIGNED \
                 frame, stale the moment either side or the authority moves"
            }
        }
    }

    /// Why this kind is not implemented, or `None` where it is. Currently `None`
    /// for all twelve.
    ///
    /// KEPT after the last kind was built rather than deleted, and the emptiness
    /// is the point. The reason a kind is unbuilt has to be stated PER KIND
    /// (VDS S-14(2) requires the position to be honest, and the difference
    /// between "work" and "a dependency nobody owns" is what tells a reader which
    /// it is). A kind that later has to be withdrawn must therefore say why, and
    /// the shape for saying it already exists here rather than having to be
    /// reinvented under pressure.
    ///
    /// Six places read this: the dispatcher's refusal, `vds proof --list`,
    /// `vds proof --all`'s summary, `vds doctor` D2, `vds impl`'s "checked by
    /// nothing" paragraph, and the test that holds them together. All six
    /// currently print nothing, which is correct and is not the same as all six
    /// having been deleted.
    pub fn unimplemented_because(self) -> Option<&'static str> {
        None
    }

    pub fn is_implemented(self) -> bool {
        self.unimplemented_because().is_none()
    }

    pub fn implemented() -> Vec<ProofKind> {
        ProofKind::ALL
            .into_iter()
            .filter(|k| k.is_implemented())
            .collect()
    }
}

impl std::fmt::Display for ProofKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The three outcomes a proof record may carry.
///
/// There is no "skipped" and no "warning". VDS S-7(2)(4) requires a pass over
/// zero enforceable rows to be recorded as `vacuous` and never as `passed`, and
/// the absence of a fourth variant is what makes that structural.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProofStatus {
    Passed,
    Failed,
    Vacuous,
}

impl ProofStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            ProofStatus::Passed => "passed",
            ProofStatus::Failed => "failed",
            ProofStatus::Vacuous => "vacuous",
        }
    }

    /// Only a passed proof is evidence. VDS S-7(2)(4).
    pub fn is_evidence(self) -> bool {
        matches!(self, ProofStatus::Passed)
    }
}

impl std::fmt::Display for ProofStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// VDS S-7(2)(5): a hand-written proof record is void, and the contract enforces
/// it by admitting exactly one value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CaptureMode {
    #[default]
    Automatic,
}

/// What invoked the run. VDS S-7(2)(3) requires something other than the author
/// choosing to run it, and VDS S-7(3) holds that a hook is not CI.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Default,
    Serialize,
    Deserialize,
    JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum InvokedBy {
    GithookPreCommit,
    GithookPrePush,
    CiWorkflow,
    PackageScript,
    Build,
    #[default]
    Manual,
}

impl InvokedBy {
    pub const ALL: [InvokedBy; 6] = [
        InvokedBy::GithookPreCommit,
        InvokedBy::GithookPrePush,
        InvokedBy::CiWorkflow,
        InvokedBy::PackageScript,
        InvokedBy::Build,
        InvokedBy::Manual,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            InvokedBy::GithookPreCommit => "githook_pre_commit",
            InvokedBy::GithookPrePush => "githook_pre_push",
            InvokedBy::CiWorkflow => "ci_workflow",
            InvokedBy::PackageScript => "package_script",
            InvokedBy::Build => "build",
            InvokedBy::Manual => "manual",
        }
    }

    pub fn parse(raw: &str) -> Option<InvokedBy> {
        InvokedBy::ALL.into_iter().find(|i| i.as_str() == raw)
    }

    /// VDS S-7(2)(3): "manual" is the author choosing to run it, and satisfies
    /// nothing.
    pub fn satisfies_invocation_limb(self) -> bool {
        !matches!(self, InvokedBy::Manual)
    }
}

impl std::fmt::Display for InvokedBy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    #[default]
    Fatal,
    Warning,
    Informational,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Fatal => "fatal",
            Severity::Warning => "warning",
            Severity::Informational => "informational",
        }
    }
}

/// WHICH SIDE MOVED, where the direction is knowable.
///
/// A finding that says only `expected` and `actual` names a disagreement and
/// not a JOB. "The contract requires a prop the code lacks" and "the code
/// accepts a prop no contract names" are the same disagreement pointing
/// opposite ways, and they are owed by different people: the first is usually an
/// implementation, the second is usually an AMENDMENT. Reading a wall of
/// findings and sorting them by hand is work the proof already did and threw
/// away.
///
/// Borrowed, with attribution, from `southleft/ds-contracts-poc`
/// (`parity/diff.ts`), where every finding carries
/// `classification: ahead | behind | mismatch` against a surface. Their model
/// is always contract-versus-surface and never surface-to-surface, which is the
/// half that makes the direction meaningful, and it is the same discipline VDS
/// already applies by making the register the record.
///
/// Derivable here WITHOUT history, which is why it can be added at all: the
/// direction follows from which side is missing the thing, not from what
/// changed. `Undetermined` is the honest default and is what every finding
/// carries until its emitter has an opinion - guessing a direction would be
/// worse than leaving it blank, because a wrong direction sends the work to the
/// wrong person with a proof's authority behind it.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    Serialize,
    Deserialize,
    JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Drift {
    /// The surface carries something the contract does not name. An amendment
    /// is usually owed, not a fix.
    Ahead,
    /// The contract requires something the surface does not have. An
    /// implementation is usually owed.
    Behind,
    /// Both sides name it and they disagree about it.
    Mismatch,
    /// This emitter has no opinion about the direction, and says so rather than
    /// choosing one.
    #[default]
    Undetermined,
}

impl Drift {
    pub fn as_str(self) -> &'static str {
        match self {
            Drift::Ahead => "ahead",
            Drift::Behind => "behind",
            Drift::Mismatch => "mismatch",
            Drift::Undetermined => "undetermined",
        }
    }

    /// Whether the direction says anything at all.
    pub fn is_determined(self) -> bool {
        !matches!(self, Drift::Undetermined)
    }
}

impl std::fmt::Display for Drift {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One finding. `expected` and `actual` are both required and both non-empty,
/// because a finding that says only what is wrong makes the reader guess at what
/// right would have been.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Violation {
    /// `file:line`, or a route plus a component id.
    pub location: String,
    /// The invariant id or statute section the violation offends.
    pub rule: String,
    pub expected: String,
    pub actual: String,
    #[serde(default)]
    pub severity: Severity,
    /// Which side moved. `#[serde(default)]` so every proof record already on
    /// disk still parses, and reads as `undetermined` - which is true of them.
    #[serde(default)]
    pub drift: Drift,
}

impl Violation {
    pub fn fatal(
        location: impl Into<String>,
        rule: impl Into<String>,
        expected: impl Into<String>,
        actual: impl Into<String>,
    ) -> Self {
        Self {
            location: location.into(),
            rule: rule.into(),
            expected: expected.into(),
            actual: actual.into(),
            severity: Severity::Fatal,
            drift: Drift::Undetermined,
        }
    }

    /// The same finding, with the direction it points recorded.
    #[must_use]
    pub fn with_drift(mut self, drift: Drift) -> Self {
        self.drift = drift;
        self
    }

    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    pub fn is_fatal(&self) -> bool {
        self.severity == Severity::Fatal
    }
}

/// The machine output a warrant is granted against.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProofResult {
    pub id: ProofId,
    pub kind: ProofKind,
    pub status: ProofStatus,
    /// Set when a warrant cites this record. Null until then.
    #[serde(default)]
    pub warrant_id: Option<WarrantId>,
    /// The exact command that produced it, so VDS S-7(2)(1) is checkable.
    pub command: String,
    /// The gate that ran, repository-relative.
    pub script: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script_digest: Option<Digest>,
    pub exit_code: i32,
    pub rows_considered: u64,
    pub rows_enforced: u64,
    /// Count per reason a row was considered but not enforced, so a vacuous pass
    /// is diagnosable rather than merely flagged.
    #[serde(default)]
    pub rows_skipped_reasons: std::collections::BTreeMap<String, u64>,
    pub violations: Vec<Violation>,
    /// Lines the run printed that are not violations: reliance on a reserved
    /// clause, a deprecated consumer, a carve-out that was taken.
    #[serde(default)]
    pub notes: Vec<String>,
    pub inputs_digest: Digest,
    /// The digest a warrant cites. Covers the run's FINDINGS and inputs, and
    /// deliberately not its timing, so an unchanged input re-run cites the same
    /// evidence (VDS S-7(2)(1)).
    pub digest: Digest,
    pub designpack_digest: Digest,
    pub captured_at: Timestamp,
    pub capture_mode: CaptureMode,
    #[serde(default)]
    pub invoked_by: InvokedBy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

impl ProofResult {
    /// Whether this record may be cited as evidence for a warrant.
    ///
    /// Three conditions, all from VDS S-7(2): the status is `passed`, the
    /// capture was automatic, and at least one row was actually enforced. The
    /// third is not redundant: it is the D3 defect, a printed PASS over zero
    /// enforceable rows.
    pub fn is_citable_evidence(&self) -> bool {
        self.status.is_evidence()
            && self.capture_mode == CaptureMode::Automatic
            && self.rows_enforced > 0
    }

    pub fn fatal_violations(&self) -> impl Iterator<Item = &Violation> {
        self.violations.iter().filter(|v| v.is_fatal())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_registry_is_closed_at_fifteen() {
        // Twelve enacted kinds, plus three DRAFTED on 2026-08-01 (prohibition,
        // burndown, visual_review) whose amendments are filed and pending
        // enactment; the code ships when the ruling lands, and the drafts say
        // so on their faces. This number is asserted rather than derived on
        // purpose: VDS S-7(6) makes adding a kind an amendment to the
        // specification, so a variant appearing without one has to fail
        // somewhere, and this is where.
        assert_eq!(ProofKind::ALL.len(), 15);
        assert!(serde_json::from_str::<ProofKind>("\"vibes\"").is_err());
    }

    #[test]
    fn every_kind_round_trips_through_its_wire_form() {
        for kind in ProofKind::ALL {
            let text = serde_json::to_string(&kind).unwrap();
            assert_eq!(text, format!("\"{}\"", kind.as_str()));
            assert_eq!(serde_json::from_str::<ProofKind>(&text).unwrap(), kind);
            assert_eq!(ProofKind::parse(kind.as_str()), Some(kind));
        }
    }

    /// Every kind in the closed registry is implemented.
    ///
    /// `unimplemented_because` is deliberately KEPT after the last kind was
    /// built. VDS S-14A(3) requires the position to be honest per kind, and a
    /// kind that later has to be withdrawn must say WHY rather than quietly
    /// disappearing from a match arm; keeping the method means the honest form
    /// already exists when that happens. This test is what holds the two halves
    /// together: a reason present without a matching refusal in the dispatcher,
    /// or a refusal without a reason, fails here.
    #[test]
    fn every_kind_in_the_closed_registry_is_implemented_and_any_gap_says_why() {
        let unimplemented: Vec<ProofKind> = ProofKind::ALL
            .into_iter()
            .filter(|k| !k.is_implemented())
            .collect();
        assert!(
            unimplemented.is_empty(),
            "these kinds report themselves unimplemented: {unimplemented:?}. That is lawful, \
             but VDS.md S-14A(3), crates/vds-proof/src/lib.rs and docs/ADOPTING.md all say all \
             twelve are built, and one of the four is now wrong."
        );
        assert_eq!(ProofKind::implemented().len(), ProofKind::ALL.len());
        assert_eq!(
            ProofKind::ALL.len(),
            15,
            "the registry is closed at fifteen"
        );

        // The honest form, held in place for whenever a kind has to be
        // withdrawn: a reason is a sentence, not a shrug.
        for kind in ProofKind::ALL {
            if let Some(reason) = kind.unimplemented_because() {
                assert!(reason.len() > 40, "{kind}: {reason:?} is not a reason");
            }
        }
    }

    #[test]
    fn capture_mode_admits_exactly_one_value() {
        assert!(serde_json::from_str::<CaptureMode>("\"manual\"").is_err());
        assert_eq!(
            serde_json::from_str::<CaptureMode>("\"automatic\"").unwrap(),
            CaptureMode::Automatic
        );
    }

    #[test]
    fn manual_invocation_does_not_satisfy_the_invocation_limb() {
        assert!(!InvokedBy::Manual.satisfies_invocation_limb());
        assert!(InvokedBy::CiWorkflow.satisfies_invocation_limb());
        assert!(InvokedBy::GithookPrePush.satisfies_invocation_limb());
    }

    #[test]
    fn only_passed_is_evidence() {
        assert!(ProofStatus::Passed.is_evidence());
        assert!(!ProofStatus::Vacuous.is_evidence());
        assert!(!ProofStatus::Failed.is_evidence());
    }

    fn result(status: ProofStatus, rows_enforced: u64) -> ProofResult {
        ProofResult {
            id: ProofId::parse("PROOF-20260725-100000").unwrap(),
            kind: ProofKind::Composition,
            status,
            warrant_id: None,
            command: "vds proof composition".into(),
            script: "crates/vds-proof/src/composition.rs".into(),
            script_digest: None,
            exit_code: 0,
            rows_considered: 3,
            rows_enforced,
            rows_skipped_reasons: Default::default(),
            violations: vec![],
            notes: vec![],
            inputs_digest: Digest::of_text("i"),
            digest: Digest::of_text("d"),
            designpack_digest: Digest::of_text("p"),
            captured_at: Timestamp::fixed(2026, 7, 25, 10, 0, 0),
            capture_mode: CaptureMode::Automatic,
            invoked_by: InvokedBy::CiWorkflow,
            duration_ms: Some(1),
        }
    }

    #[test]
    fn a_vacuous_record_is_not_citable_evidence() {
        assert!(!result(ProofStatus::Vacuous, 0).is_citable_evidence());
    }

    #[test]
    fn a_passed_record_over_zero_enforced_rows_is_not_citable_evidence() {
        assert!(
            !result(ProofStatus::Passed, 0).is_citable_evidence(),
            "this is the [2026] VJS-CC-OPBOX 3 D3 defect: a printed PASS over nothing"
        );
    }

    #[test]
    fn a_passed_record_over_real_rows_is_citable_evidence() {
        assert!(result(ProofStatus::Passed, 3).is_citable_evidence());
    }
}
