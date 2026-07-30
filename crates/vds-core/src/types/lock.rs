//! `.vds/enforcement.lock`: which checks are wired, by digest.
//!
//! VDS S-8(1). The lock is held OUTSIDE the gates it witnesses, so a weakening
//! edit bumps a digest and trips a loud blocking finding rather than passing
//! under its own possibly weakened logic.
//!
//! VDS S-8(5), stated plainly and not glossed: the lock CANNOT bind an author
//! with full write access who edits a gate and re-locks it in the same act. The
//! backstops for that residue are non-machine. The lock makes the act visible in
//! a diff; it does not prevent it. No VDS document may claim otherwise, and that
//! includes this one.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{InvokedBy, ProofKind};
use crate::digest::Digest;
use crate::timestamp::Timestamp;

pub const LOCK_SCHEMA_VERSION: u32 = 1;
pub const LOCK_FILE_NAME: &str = "enforcement.lock";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LockKind {
    ProofScript,
    LedgerGenerator,
    Hook,
    Schema,
    Config,
}

impl LockKind {
    pub const ALL: [LockKind; 5] = [
        LockKind::ProofScript,
        LockKind::LedgerGenerator,
        LockKind::Hook,
        LockKind::Schema,
        LockKind::Config,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            LockKind::ProofScript => "proof_script",
            LockKind::LedgerGenerator => "ledger_generator",
            LockKind::Hook => "hook",
            LockKind::Schema => "schema",
            LockKind::Config => "config",
        }
    }

    pub fn parse(raw: &str) -> Option<LockKind> {
        LockKind::ALL.into_iter().find(|k| k.as_str() == raw)
    }
}

/// Something that runs the gate which is not the author choosing to run it.
/// VDS S-7(2)(3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Invocation {
    pub surface: InvokedBy,
    /// Where it is wired: a workflow file and job, a hook path and line, a
    /// package script name.
    pub reference: String,
    /// False where the surface runs the gate but does not block on it.
    #[serde(default = "default_true")]
    pub blocking: bool,
}

fn default_true() -> bool {
    true
}

/// The test that proves the gate's FAILING direction.
///
/// VDS S-7(2)(2): a check whose failing direction is asserted nowhere has proven
/// only its happy path. This field is not optional, which is how that condition
/// is made structural rather than requested: an entry cannot be written without
/// naming the test.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FailingDirectionTest {
    pub path: String,
    pub test_name: String,
    /// What violation the test seeds, in one line, where a reviewer will see it.
    ///
    /// VDS docs/GOAL.md is explicit that the failing-direction test proves the
    /// check CAN fail and not that the seeded violation is the one that matters.
    /// That remains a review question, and this line is what a reviewer reads to
    /// answer it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seeds: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LockEntry {
    /// Repository-relative, no leading slash.
    pub path: String,
    pub digest: Digest,
    pub kind: LockKind,
    /// At least one. An empty list is not representable in a valid entry,
    /// because an uninvoked gate is not enforcement.
    pub invoked_by: Vec<Invocation>,
    pub proves: Vec<ProofKind>,
    pub failing_direction_test: FailingDirectionTest,
    pub pinned_at: Timestamp,
    pub pinned_by: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supersedes_digest: Option<Digest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relock_rationale: Option<String>,
}

impl LockEntry {
    /// Whether some blocking CI surface runs this gate.
    ///
    /// VDS S-7(3): a hook is not CI. `git commit --no-verify` bypasses a local
    /// hook, and an author with write access can edit a gate and re-pin it. A
    /// local hook alone satisfies the invocation limb only as an interim state,
    /// and the interim must be recorded.
    pub fn has_blocking_ci(&self) -> bool {
        self.invoked_by
            .iter()
            .any(|i| i.surface == InvokedBy::CiWorkflow && i.blocking)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EnforcementLock {
    pub schema_version: u32,
    pub generated_at: Timestamp,
    pub entries: Vec<LockEntry>,
}

impl EnforcementLock {
    pub fn entry(&self, path: &str) -> Option<&LockEntry> {
        self.entries.iter().find(|e| e.path == path)
    }
}

/// One finding from verifying the lock against the tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriftFinding {
    /// A pinned path whose bytes no longer match. Fatal (VDS S-8(4)).
    Drift {
        path: String,
        pinned: Digest,
        actual: Digest,
        proves: Vec<ProofKind>,
    },
    /// A pinned path that is gone. A pinned gate that is absent is a DELETED
    /// gate, not an absent finding.
    Missing { path: String, pinned: Digest },
    /// A named failing-direction test whose file does not exist. The entry
    /// claims VDS S-7(2)(2) is satisfied and the claim is unbacked.
    MissingFailingDirectionTest {
        path: String,
        test_path: String,
        test_name: String,
    },
}

impl std::fmt::Display for DriftFinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DriftFinding::Drift {
                path,
                pinned,
                actual,
                proves,
            } => {
                writeln!(f, "DRIFT    {path}")?;
                writeln!(f, "           pinned: {pinned}")?;
                writeln!(f, "           actual: {actual}")?;
                writeln!(
                    f,
                    "           proves: {}",
                    proves
                        .iter()
                        .map(|k| k.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )?;
                write!(
                    f,
                    "           re-pin only after a recorded gate change, and self-file the \
                     rationale (VDS S-8(4), S-12(3))."
                )
            }
            DriftFinding::Missing { path, pinned } => {
                writeln!(f, "MISSING  {path}")?;
                writeln!(f, "           pinned: {pinned}")?;
                write!(
                    f,
                    "           actual: the file does not exist. A pinned gate that is gone \
                     is a deleted gate, not an absent finding."
                )
            }
            DriftFinding::MissingFailingDirectionTest {
                path,
                test_path,
                test_name,
            } => {
                writeln!(f, "UNTESTED {path}")?;
                writeln!(f, "           names: {test_path}::{test_name}")?;
                write!(
                    f,
                    "           actual: no function of that name exists in that file, so the \
                     entry's claim that the gate's failing direction is asserted somewhere is \
                     unbacked (VDS S-7(2)(2)). This used to check only that the FILE existed, \
                     which for every entry in this lock was the same file whose digest had \
                     just been read - a condition that could not fail."
                )
            }
        }
    }
}

/// A note that is not a finding: something a reader must know that does not
/// block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockNote {
    NoLock,
    /// The gate is invoked, but by nothing blocking in CI.
    InterimHookOnly {
        path: String,
        surfaces: Vec<String>,
    },
    /// A gate the lock does not witness.
    Unpinned {
        path: String,
    },
}

impl std::fmt::Display for LockNote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LockNote::NoLock => write!(
                f,
                "no {LOCK_FILE_NAME} present. The lock is opt-in (VDS S-8(3)), so this is \
                 quiet rather than broken, and no warrant may cite a proof whose gate is \
                 absent from a present lock."
            ),
            LockNote::InterimHookOnly { path, surfaces } => write!(
                f,
                "INTERIM  {path} is invoked by {} and by no blocking ci_workflow. A hook is \
                 not CI: `git commit --no-verify` bypasses it, so this satisfies \
                 VDS S-7(2)(3) only as an interim state, and the interim is recorded here \
                 (VDS S-7(3)).",
                surfaces.join(", ")
            ),
            LockNote::Unpinned { path } => write!(
                f,
                "UNPINNED {path} is a gate that no lock entry witnesses. A warrant may not \
                 cite a proof whose gate is absent from a present lock (VDS S-8(3))."
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(surfaces: Vec<Invocation>) -> LockEntry {
        LockEntry {
            path: "crates/vds-proof/src/composition.rs".into(),
            digest: Digest::of_text("x"),
            kind: LockKind::ProofScript,
            invoked_by: surfaces,
            proves: vec![ProofKind::Composition],
            failing_direction_test: FailingDirectionTest {
                path: "crates/vds-proof/src/composition.rs".into(),
                test_name: "composition_fails_on_an_unregistered_component".into(),
                seeds: Some("a screen importing a component with no register record".into()),
            },
            pinned_at: Timestamp::fixed(2026, 7, 25, 10, 0, 0),
            pinned_by: "tester".into(),
            supersedes_digest: None,
            relock_rationale: None,
        }
    }

    #[test]
    fn a_hook_only_entry_is_not_blocking_ci() {
        let e = entry(vec![Invocation {
            surface: InvokedBy::GithookPrePush,
            reference: ".githooks/pre-push:106".into(),
            blocking: true,
        }]);
        assert!(!e.has_blocking_ci());
    }

    #[test]
    fn a_non_blocking_ci_entry_is_not_blocking_ci() {
        let e = entry(vec![Invocation {
            surface: InvokedBy::CiWorkflow,
            reference: ".github/workflows/vds.yml".into(),
            blocking: false,
        }]);
        assert!(
            !e.has_blocking_ci(),
            "a CI job that runs the gate and ignores its exit code is not enforcement"
        );
    }

    #[test]
    fn a_blocking_ci_entry_satisfies_the_limb() {
        let e = entry(vec![Invocation {
            surface: InvokedBy::CiWorkflow,
            reference: ".github/workflows/vds.yml".into(),
            blocking: true,
        }]);
        assert!(e.has_blocking_ci());
    }

    #[test]
    fn blocking_defaults_to_true_when_absent() {
        let parsed: Invocation =
            serde_yaml::from_str("surface: ci_workflow\nreference: .github/workflows/vds.yml\n")
                .unwrap();
        assert!(parsed.blocking);
    }

    #[test]
    fn an_entry_cannot_be_deserialised_without_a_failing_direction_test() {
        let text = "path: a.rs\ndigest: sha256:0000000000000000000000000000000000000000000000000000000000000000\nkind: proof_script\ninvoked_by: []\nproves: []\npinned_at: 2026-07-25T10:00:00Z\npinned_by: t\n";
        assert!(
            serde_yaml::from_str::<LockEntry>(text).is_err(),
            "VDS S-7(2)(2) is structural: no entry without a named failing-direction test"
        );
    }
}
