//! Verifying, writing and re-pinning `.vds/enforcement.lock`.
//!
//! VDS S-8(1): the lock is held OUTSIDE the gates it witnesses, so a weakening
//! edit bumps a digest and trips a loud blocking finding rather than passing
//! under its own possibly weakened logic.
//!
//! VDS S-8(5), stated plainly: the lock CANNOT bind an author with full write
//! access who edits a gate and re-locks it in the same act. The backstops for
//! that residue are non-machine. What the lock does is make the act visible in
//! a diff, which is why [`repin_lock`] refuses without a rationale and records
//! what each entry superseded.

use std::collections::BTreeSet;
use std::path::Path;

use vds_core::{
    Digest, DriftFinding, EnforcementLock, LOCK_SCHEMA_VERSION, LockEntry, LockNote, Project,
    Result, Timestamp, VdsError, actor, write_text_atomically,
};

use crate::Store;

/// The outcome of checking the lock against the tree.
#[derive(Debug, Default)]
pub struct LockVerdict {
    /// Fatal. VDS S-8(4): drift is fatal.
    pub findings: Vec<DriftFinding>,
    /// Not fatal, but a reader must see them.
    pub notes: Vec<LockNote>,
}

impl LockVerdict {
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }
}

/// Recompute every pinned digest and report what moved.
///
/// `gate_paths` is the set of paths the caller considers to be gates, used to
/// report an UNPINNED gate. It is passed in rather than discovered here because
/// what counts as a gate is a property of the build, and a hardcoded directory
/// walk would silently stop being true the moment the layout changed.
pub fn verify_lock(store: &Store, gate_paths: &[String]) -> Result<LockVerdict> {
    let mut verdict = LockVerdict::default();
    let Some(lock) = store.read_lock()? else {
        verdict.notes.push(LockNote::NoLock);
        return Ok(verdict);
    };

    let mut pinned: BTreeSet<String> = BTreeSet::new();

    for entry in &lock.entries {
        pinned.insert(entry.path.clone());
        let target = store.project.root.join(&entry.path);

        match current_digest(&target)? {
            None => verdict.findings.push(DriftFinding::Missing {
                path: entry.path.clone(),
                pinned: entry.digest.clone(),
            }),
            Some(actual) if actual != entry.digest => verdict.findings.push(DriftFinding::Drift {
                path: entry.path.clone(),
                pinned: entry.digest.clone(),
                actual,
                proves: entry.proves.clone(),
            }),
            Some(_) => {
                // The bytes match. Two further conditions still apply.
                if !store
                    .project
                    .root
                    .join(&entry.failing_direction_test.path)
                    .is_file()
                {
                    verdict
                        .findings
                        .push(DriftFinding::MissingFailingDirectionTest {
                            path: entry.path.clone(),
                            test_path: entry.failing_direction_test.path.clone(),
                            test_name: entry.failing_direction_test.test_name.clone(),
                        });
                }
                if !entry.has_blocking_ci() {
                    let mut surfaces: Vec<String> = entry
                        .invoked_by
                        .iter()
                        .map(|i| i.surface.as_str().to_owned())
                        .collect();
                    surfaces.sort();
                    surfaces.dedup();
                    verdict.notes.push(LockNote::InterimHookOnly {
                        path: entry.path.clone(),
                        surfaces,
                    });
                }
            }
        }
    }

    for gate in gate_paths {
        if !pinned.contains(gate) {
            verdict
                .notes
                .push(LockNote::Unpinned { path: gate.clone() });
        }
    }

    Ok(verdict)
}

pub fn current_digest(target: &Path) -> Result<Option<Digest>> {
    if !target.is_file() {
        return Ok(None);
    }
    Ok(Some(Digest::of_file(target)?))
}

/// Write the lock, refusing a duplicate path.
pub fn write_lock(project: &Project, entries: Vec<LockEntry>) -> Result<std::path::PathBuf> {
    let mut seen = BTreeSet::new();
    for entry in &entries {
        if entry.path.starts_with('/') {
            return Err(VdsError::precondition(format!(
                "lock entry path {:?} is absolute. Every pinned path is repository-relative, \
                 or the lock stops meaning the same thing on another machine.",
                entry.path
            )));
        }
        if entry.invoked_by.is_empty() {
            return Err(VdsError::precondition(format!(
                "lock entry {:?} names no invocation. An empty invocation list is not \
                 representable, because an uninvoked gate is not enforcement \
                 (VDS S-7(2)(3)).",
                entry.path
            )));
        }
        if !seen.insert(entry.path.clone()) {
            return Err(VdsError::precondition(format!(
                "{LOCK_FILE_NAME_LOCAL}: duplicate entry for path {:?}. Two pins on one gate \
                 mean one of them is not witnessing anything.",
                entry.path
            )));
        }
    }

    let mut entries = entries;
    entries.sort_by(|a, b| a.path.cmp(&b.path));

    let lock = EnforcementLock {
        schema_version: LOCK_SCHEMA_VERSION,
        generated_at: Timestamp::now(),
        entries,
    };
    let text = serde_yaml::to_string(&lock).map_err(|e| VdsError::Serialize {
        what: LOCK_FILE_NAME_LOCAL.into(),
        message: e.to_string(),
    })?;
    let path = project.enforcement_lock_path();
    write_text_atomically(&path, &text)?;
    Ok(path)
}

const LOCK_FILE_NAME_LOCAL: &str = vds_core::LOCK_FILE_NAME;

/// One re-pinned entry, for reporting.
#[derive(Debug)]
pub struct Repinned {
    pub path: String,
    pub was: Digest,
    pub now: Digest,
}

/// Re-pin every entry whose bytes moved, recording what each superseded.
///
/// VDS S-8(4): re-pinning is deliberate. A rationale is required, and a missing
/// pinned file is refused rather than re-pinned, because re-pinning a deleted
/// gate would erase the finding instead of answering it.
pub fn repin_lock(store: &Store, rationale: &str) -> Result<Vec<Repinned>> {
    if rationale.trim().is_empty() {
        return Err(VdsError::precondition(
            "re-pinning needs a rationale. Re-locking without recording why, and without \
             self-filing under VDS S-12(3), is itself the breach the lock exists to make \
             visible (VDS S-8(4)).",
        ));
    }
    let Some(lock) = store.read_lock()? else {
        return Err(VdsError::precondition(format!(
            "no {LOCK_FILE_NAME_LOCAL} to re-pin"
        )));
    };

    let mut changed = Vec::new();
    let mut entries = Vec::new();
    for entry in lock.entries {
        let target = store.project.root.join(&entry.path);
        let Some(actual) = current_digest(&target)? else {
            return Err(VdsError::precondition(format!(
                "{} is pinned and missing. Re-pinning a deleted gate would erase the finding \
                 rather than answer it.",
                entry.path
            )));
        };
        if actual == entry.digest {
            entries.push(entry);
            continue;
        }
        changed.push(Repinned {
            path: entry.path.clone(),
            was: entry.digest.clone(),
            now: actual.clone(),
        });
        entries.push(LockEntry {
            supersedes_digest: Some(entry.digest.clone()),
            relock_rationale: Some(rationale.to_owned()),
            digest: actual,
            pinned_at: Timestamp::now(),
            pinned_by: actor(),
            ..entry
        });
    }

    if !changed.is_empty() {
        write_lock(store.project, entries)?;
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vds_core::{
        FailingDirectionTest, Invocation, InvokedBy, LockKind, ProofKind, default_config,
    };

    struct Scaffold {
        _tmp: tempfile::TempDir,
        project: Project,
    }

    fn scaffold() -> Scaffold {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".vds")).unwrap();
        std::fs::write(
            tmp.path().join(".vds/config.toml"),
            default_config("demo", "DEMO"),
        )
        .unwrap();
        std::fs::create_dir_all(tmp.path().join("gates")).unwrap();
        std::fs::write(tmp.path().join("gates/a.rs"), "fn gate() {}\n").unwrap();
        std::fs::write(tmp.path().join("gates/a_test.rs"), "fn seeds() {}\n").unwrap();
        let project = Project::discover(Some(tmp.path())).unwrap();
        Scaffold { _tmp: tmp, project }
    }

    fn entry(project: &Project, blocking_ci: bool) -> LockEntry {
        LockEntry {
            path: "gates/a.rs".into(),
            digest: Digest::of_file(&project.root.join("gates/a.rs")).unwrap(),
            kind: LockKind::ProofScript,
            invoked_by: vec![if blocking_ci {
                Invocation {
                    surface: InvokedBy::CiWorkflow,
                    reference: ".github/workflows/vds.yml".into(),
                    blocking: true,
                }
            } else {
                Invocation {
                    surface: InvokedBy::GithookPrePush,
                    reference: ".githooks/pre-push:1".into(),
                    blocking: true,
                }
            }],
            proves: vec![ProofKind::Composition],
            failing_direction_test: FailingDirectionTest {
                path: "gates/a_test.rs".into(),
                test_name: "gate_fails_on_a_seeded_violation".into(),
                seeds: Some("an unregistered component".into()),
            },
            pinned_at: Timestamp::fixed(2026, 7, 25, 10, 0, 0),
            pinned_by: "tester".into(),
            supersedes_digest: None,
            relock_rationale: None,
        }
    }

    #[test]
    fn no_lock_is_quiet_rather_than_broken() {
        let s = scaffold();
        let store = Store::new(&s.project);
        let verdict = verify_lock(&store, &[]).unwrap();
        assert!(verdict.is_clean());
        assert!(matches!(verdict.notes.as_slice(), [LockNote::NoLock]));
    }

    #[test]
    fn a_clean_lock_produces_no_finding() {
        let s = scaffold();
        let store = Store::new(&s.project);
        write_lock(&s.project, vec![entry(&s.project, true)]).unwrap();
        let verdict = verify_lock(&store, &["gates/a.rs".into()]).unwrap();
        assert!(verdict.is_clean(), "{:?}", verdict.findings);
        assert!(verdict.notes.is_empty(), "{:?}", verdict.notes);
    }

    /// VDS S-8(6): the positive direction of the drift check is itself tested.
    /// A test that only ever sees unmodified files has proven that unmodified
    /// files are unmodified.
    #[test]
    fn editing_a_pinned_gate_trips_a_fatal_drift_finding() {
        let s = scaffold();
        let store = Store::new(&s.project);
        write_lock(&s.project, vec![entry(&s.project, true)]).unwrap();

        std::fs::write(
            s.project.root.join("gates/a.rs"),
            "fn gate() { /* weakened */ }\n",
        )
        .unwrap();

        let verdict = verify_lock(&store, &["gates/a.rs".into()]).unwrap();
        assert_eq!(verdict.findings.len(), 1, "{:?}", verdict.findings);
        assert!(matches!(verdict.findings[0], DriftFinding::Drift { .. }));
        assert!(verdict.findings[0].to_string().contains("DRIFT"));
    }

    #[test]
    fn deleting_a_pinned_gate_is_a_finding_and_not_a_silence() {
        let s = scaffold();
        let store = Store::new(&s.project);
        write_lock(&s.project, vec![entry(&s.project, true)]).unwrap();
        std::fs::remove_file(s.project.root.join("gates/a.rs")).unwrap();
        let verdict = verify_lock(&store, &[]).unwrap();
        assert!(matches!(
            verdict.findings.as_slice(),
            [DriftFinding::Missing { .. }]
        ));
    }

    #[test]
    fn a_named_failing_direction_test_that_does_not_exist_is_a_finding() {
        let s = scaffold();
        let store = Store::new(&s.project);
        std::fs::remove_file(s.project.root.join("gates/a_test.rs")).unwrap();
        write_lock(&s.project, vec![entry(&s.project, true)]).unwrap();
        let verdict = verify_lock(&store, &[]).unwrap();
        assert!(
            matches!(
                verdict.findings.as_slice(),
                [DriftFinding::MissingFailingDirectionTest { .. }]
            ),
            "{:?}",
            verdict.findings
        );
    }

    #[test]
    fn a_hook_only_gate_is_recorded_as_an_interim() {
        let s = scaffold();
        let store = Store::new(&s.project);
        write_lock(&s.project, vec![entry(&s.project, false)]).unwrap();
        let verdict = verify_lock(&store, &[]).unwrap();
        assert!(verdict.is_clean());
        assert!(matches!(
            verdict.notes.as_slice(),
            [LockNote::InterimHookOnly { .. }]
        ));
    }

    #[test]
    fn a_gate_the_lock_does_not_witness_is_reported() {
        let s = scaffold();
        let store = Store::new(&s.project);
        write_lock(&s.project, vec![entry(&s.project, true)]).unwrap();
        let verdict = verify_lock(&store, &["gates/a.rs".into(), "gates/b.rs".into()]).unwrap();
        assert!(matches!(
            verdict.notes.as_slice(),
            [LockNote::Unpinned { .. }]
        ));
    }

    #[test]
    fn writing_a_duplicate_path_is_refused() {
        let s = scaffold();
        let err = write_lock(
            &s.project,
            vec![entry(&s.project, true), entry(&s.project, true)],
        )
        .unwrap_err();
        assert!(err.to_string().contains("duplicate entry"), "{err}");
    }

    #[test]
    fn writing_an_entry_with_no_invocation_is_refused() {
        let s = scaffold();
        let mut e = entry(&s.project, true);
        e.invoked_by.clear();
        let err = write_lock(&s.project, vec![e]).unwrap_err();
        assert!(err.to_string().contains("uninvoked gate"), "{err}");
    }

    #[test]
    fn repinning_without_a_rationale_is_refused() {
        let s = scaffold();
        let store = Store::new(&s.project);
        write_lock(&s.project, vec![entry(&s.project, true)]).unwrap();
        let err = repin_lock(&store, "   ").unwrap_err();
        assert!(err.to_string().contains("VDS S-8(4)"), "{err}");
    }

    #[test]
    fn repinning_records_what_it_superseded() {
        let s = scaffold();
        let store = Store::new(&s.project);
        write_lock(&s.project, vec![entry(&s.project, true)]).unwrap();
        let before = store.read_lock().unwrap().unwrap().entries[0]
            .digest
            .clone();

        std::fs::write(
            s.project.root.join("gates/a.rs"),
            "fn gate() { /* fixed */ }\n",
        )
        .unwrap();
        let changed = repin_lock(&store, "gate corrected under DECISION-0001").unwrap();

        assert_eq!(changed.len(), 1);
        let after = store.read_lock().unwrap().unwrap().entries[0].clone();
        assert_eq!(after.supersedes_digest, Some(before));
        assert_eq!(
            after.relock_rationale.as_deref(),
            Some("gate corrected under DECISION-0001")
        );
        assert!(verify_lock(&store, &[]).unwrap().is_clean());
    }

    #[test]
    fn repinning_a_deleted_gate_is_refused() {
        let s = scaffold();
        let store = Store::new(&s.project);
        write_lock(&s.project, vec![entry(&s.project, true)]).unwrap();
        std::fs::remove_file(s.project.root.join("gates/a.rs")).unwrap();
        let err = repin_lock(&store, "because").unwrap_err();
        assert!(err.to_string().contains("erase the finding"), "{err}");
    }

    #[test]
    fn repinning_an_unchanged_lock_changes_nothing() {
        let s = scaffold();
        let store = Store::new(&s.project);
        write_lock(&s.project, vec![entry(&s.project, true)]).unwrap();
        assert!(repin_lock(&store, "no reason needed").unwrap().is_empty());
    }
}
