//! Proof kinds and proof results.
//!
//! VDS S-7(5) fixes ten proof kinds as a CLOSED registry, and VDS S-7(6) makes
//! adding one an amendment to the specification and the invariant registry
//! rather than a script anyone may drop in. [`ProofKind`] is therefore an enum:
//! a kind outside the registry does not fail validation, it fails to compile.
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
}

impl ProofKind {
    pub const ALL: [ProofKind; 10] = [
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
        }
    }

    /// Why this kind is not implemented, or `None` where it is.
    ///
    /// Stated per kind rather than as a blanket "unimplemented", because the
    /// reasons differ and the difference is what tells a reader whether the gap
    /// is work or a dependency. VDS S-14(2) requires the position to be honest.
    pub fn unimplemented_because(self) -> Option<&'static str> {
        match self {
            ProofKind::Contrast => Some(
                "needs the subject project's shipped CSS and its theme set, which are named \
                 records VDS reads and does not own ([2026] VJS-CC-OPBOX 3 D1)",
            ),
            ProofKind::Parity => Some(
                "needs to read the subject project's component source and compare its props \
                 and states against the record, which is a TypeScript analysis and not a \
                 digest comparison",
            ),
            ProofKind::TokenPin => Some(
                "needs both named records present: the shipped CSS and the decided-target \
                 Figma file. The Figma side is a network read, and VDS S-7(2)(1) forbids a \
                 network call inside a proof, so the pin must be generated out of band and \
                 then checked",
            ),
            _ => None,
        }
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
        }
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
    fn the_registry_is_closed_at_ten() {
        assert_eq!(ProofKind::ALL.len(), 10);
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

    #[test]
    fn an_unimplemented_kind_says_why() {
        for kind in ProofKind::ALL {
            if !kind.is_implemented() {
                let reason = kind.unimplemented_because().unwrap();
                assert!(reason.len() > 40, "{kind}: {reason:?} is not a reason");
            }
        }
        assert_eq!(
            ProofKind::implemented().len(),
            7,
            "seven of the ten kinds are implemented; the other three need a named record \
             VDS does not own"
        );
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
