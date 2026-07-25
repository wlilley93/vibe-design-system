//! The designpack: the vendored normative corpus, and the lock that pins it.
//!
//! VDS S-11(1): adoption is vendored, read-only, digest-pinned and fail-closed,
//! and the runtime never fetches doctrine. A project subscribes to a designpack
//! by vendoring it and pinning its digest in `.vds/designpack.lock`, exactly as a
//! VJS subscriber pins a lawpack.
//!
//! The distinction this module keeps is between the digest **recorded** in the
//! lock and the digest **in force**, meaning the one recomputed from the
//! vendored tree right now. They are different claims and only one of them is a
//! measurement. A proof records the designpack digest in force when it ran, so
//! reading that from the lock would record the digest somebody last wrote down,
//! which stays convincing after the pack has been edited underneath it.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use vds_core::{Digest, Project, Result, Timestamp, VdsError, digest_rows, write_text_atomically};
use walkdir::WalkDir;

pub const LOCK_SCHEMA_VERSION: u32 = 1;
pub const LOCK_FILE: &str = "designpack.lock";
pub const VENDOR_DIR: &str = "designpack";

/// The digest a project pins when no pack is vendored.
///
/// A distinct, named constant rather than an empty digest, so "this project has
/// no doctrine" is a positive statement in the record and not an absence a
/// reader has to infer.
pub fn absent_digest() -> Digest {
    Digest::of_text("vds:designpack:absent")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesignpackLock {
    pub schema_version: u32,
    pub designpack_id: String,
    pub designpack_version: String,
    pub digest: Digest,
    pub generated_at: Timestamp,
    pub locked_by: String,
}

impl DesignpackLock {
    pub fn is_absent(&self) -> bool {
        self.designpack_id == "none"
    }
}

/// Recompute the digest of the vendored pack, or the absent digest.
///
/// Every file under `designpack/`, by relative path and content digest. The walk
/// is sorted and the digest is order-independent, so two checkouts of the same
/// pack produce the same digest.
pub fn digest_in_force(project: &Project) -> Result<Digest> {
    let root = project.root.join(VENDOR_DIR);
    if !root.is_dir() {
        return Ok(absent_digest());
    }
    let mut rows = Vec::new();
    for entry in WalkDir::new(&root).sort_by_file_name() {
        let entry = entry.map_err(|e| {
            VdsError::precondition(format!(
                "could not walk the vendored designpack at {}: {e}. A partial walk would \
                 produce a digest of less than the pack, which would then verify clean \
                 against a lock it does not match.",
                project.rel(&root)
            ))
        })?;
        if !entry.file_type().is_file() {
            continue;
        }
        rows.push((project.rel(entry.path()), Digest::of_file(entry.path())?));
    }
    digest_rows(&rows)
}

pub fn lock_path(project: &Project) -> PathBuf {
    project.designpack_lock_path()
}

/// Read the lock, refusing a schema version this build does not understand.
///
/// VDS S-11(2): a loader that skips clauses it cannot parse is silently lawless.
pub fn read_lock(project: &Project) -> Result<Option<DesignpackLock>> {
    let path = lock_path(project);
    if !path.is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path).map_err(|e| VdsError::io(path.display(), e))?;
    let raw: serde_yaml::Value = serde_yaml::from_str(&text).map_err(|e| VdsError::Artefact {
        path: project.rel(&path),
        message: format!("is not readable YAML: {e}"),
    })?;
    let found = raw
        .get("schema_version")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    if found > LOCK_SCHEMA_VERSION {
        return Err(VdsError::SchemaVersionAhead {
            path: project.rel(&path),
            kind: LOCK_FILE,
            found,
            understood: LOCK_SCHEMA_VERSION,
        });
    }
    let lock: DesignpackLock = serde_yaml::from_value(raw).map_err(|e| VdsError::Artefact {
        path: project.rel(&path),
        message: format!("is not a designpack lock: {e}"),
    })?;
    Ok(Some(lock))
}

pub fn write_lock(project: &Project, lock: &DesignpackLock) -> Result<PathBuf> {
    let text = serde_yaml::to_string(lock).map_err(|e| VdsError::Serialize {
        what: LOCK_FILE.into(),
        message: e.to_string(),
    })?;
    let path = lock_path(project);
    write_text_atomically(&path, &text)?;
    Ok(path)
}

/// Pin whatever is vendored right now.
pub fn pin(project: &Project, locked_by: &str) -> Result<DesignpackLock> {
    let vendored = project.root.join(VENDOR_DIR);
    let (id, version) = if vendored.is_dir() {
        (
            "local".to_owned(),
            detect_version(&vendored).unwrap_or_else(|| "v1".to_owned()),
        )
    } else {
        ("none".to_owned(), "0".to_owned())
    };
    Ok(DesignpackLock {
        schema_version: LOCK_SCHEMA_VERSION,
        designpack_id: id,
        designpack_version: version,
        digest: digest_in_force(project)?,
        generated_at: Timestamp::now(),
        locked_by: locked_by.to_owned(),
    })
}

/// The highest `vN` directory directly under the vendor root.
fn detect_version(vendored: &Path) -> Option<String> {
    let mut best: Option<(u32, String)> = None;
    for entry in std::fs::read_dir(vendored).ok()?.flatten() {
        if !entry.file_type().ok()?.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if let Some(number) = name.strip_prefix('v').and_then(|n| n.parse::<u32>().ok())
            && best.as_ref().is_none_or(|(highest, _)| number > *highest)
        {
            best = Some((number, name));
        }
    }
    best.map(|(_, name)| name)
}

/// What is wrong between the lock and the tree.
#[derive(Debug, PartialEq, Eq)]
pub enum PackVerdict {
    /// No lock. VDS S-8(3)-style: quiet rather than broken, but a proof cannot
    /// record a digest in force it was never told to expect.
    NoLock,
    InForce,
    Drifted {
        pinned: Digest,
        actual: Digest,
    },
}

impl std::fmt::Display for PackVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackVerdict::NoLock => write!(
                f,
                "no {LOCK_FILE} is present, so no doctrine is pinned. Run: vds init"
            ),
            PackVerdict::InForce => {
                write!(f, "the vendored designpack matches its pin")
            }
            PackVerdict::Drifted { pinned, actual } => write!(
                f,
                "DESIGNPACK DRIFT: the vendored pack does not match its pin.\n  \
                 pinned: {pinned}\n  actual: {actual}\n  \
                 A digest bump is a deliberate recorded act, and no doctrine flows \
                 downstream by silence (VDS S-11(3)). Re-pin with: vds pack pin"
            ),
        }
    }
}

pub fn verify(project: &Project) -> Result<PackVerdict> {
    let Some(lock) = read_lock(project)? else {
        return Ok(PackVerdict::NoLock);
    };
    let actual = digest_in_force(project)?;
    if actual == lock.digest {
        Ok(PackVerdict::InForce)
    } else {
        Ok(PackVerdict::Drifted {
            pinned: lock.digest,
            actual,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vds_core::default_config;

    fn scaffold() -> (tempfile::TempDir, Project) {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".vds")).unwrap();
        std::fs::write(
            tmp.path().join(".vds/config.toml"),
            default_config("demo", "DEMO"),
        )
        .unwrap();
        let project = Project::discover(Some(tmp.path())).unwrap();
        (tmp, project)
    }

    fn vendor(project: &Project, rel: &str, contents: &str) {
        let path = project.root.join(VENDOR_DIR).join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    #[test]
    fn no_vendored_pack_pins_the_absence_of_one() {
        let (_tmp, project) = scaffold();
        assert_eq!(digest_in_force(&project).unwrap(), absent_digest());
        let lock = pin(&project, "tester").unwrap();
        assert!(lock.is_absent());
        assert_eq!(lock.designpack_version, "0");
    }

    #[test]
    fn a_vendored_pack_digests_its_contents() {
        let (_tmp, project) = scaffold();
        vendor(
            &project,
            "v1/statutes/ACT-VDS-001.yaml",
            "id: ACT-VDS-001\n",
        );
        let digest = digest_in_force(&project).unwrap();
        assert_ne!(digest, absent_digest());
        assert_eq!(digest, digest_in_force(&project).unwrap(), "deterministic");
    }

    #[test]
    fn the_version_is_read_from_the_highest_vn_directory() {
        let (_tmp, project) = scaffold();
        vendor(&project, "v1/a.yaml", "a\n");
        vendor(&project, "v2/a.yaml", "a\n");
        assert_eq!(pin(&project, "t").unwrap().designpack_version, "v2");
    }

    #[test]
    fn a_pinned_pack_verifies_in_force() {
        let (_tmp, project) = scaffold();
        vendor(&project, "v1/a.yaml", "a\n");
        let lock = pin(&project, "tester").unwrap();
        write_lock(&project, &lock).unwrap();
        assert_eq!(verify(&project).unwrap(), PackVerdict::InForce);
    }

    /// VDS S-11(3): no doctrine flows downstream by silence.
    #[test]
    fn editing_the_vendored_pack_after_pinning_is_drift() {
        let (_tmp, project) = scaffold();
        vendor(&project, "v1/a.yaml", "a\n");
        let lock = pin(&project, "tester").unwrap();
        write_lock(&project, &lock).unwrap();

        vendor(&project, "v1/a.yaml", "a\nedited\n");
        let verdict = verify(&project).unwrap();
        assert!(matches!(verdict, PackVerdict::Drifted { .. }), "{verdict}");
        assert!(verdict.to_string().contains("VDS S-11(3)"));
    }

    #[test]
    fn adding_a_file_to_the_vendored_pack_is_drift() {
        let (_tmp, project) = scaffold();
        vendor(&project, "v1/a.yaml", "a\n");
        write_lock(&project, &pin(&project, "t").unwrap()).unwrap();
        vendor(&project, "v1/b.yaml", "b\n");
        assert!(matches!(
            verify(&project).unwrap(),
            PackVerdict::Drifted { .. }
        ));
    }

    /// The distinction the module exists to keep. A proof records the digest IN
    /// FORCE, and reading that from the lock records what someone last wrote
    /// down, which stays convincing after the pack is edited underneath it.
    #[test]
    fn the_digest_in_force_is_measured_and_not_read_from_the_lock() {
        let (_tmp, project) = scaffold();
        vendor(&project, "v1/a.yaml", "a\n");
        write_lock(&project, &pin(&project, "t").unwrap()).unwrap();
        let pinned = read_lock(&project).unwrap().unwrap().digest;

        vendor(&project, "v1/a.yaml", "edited\n");
        assert_ne!(
            digest_in_force(&project).unwrap(),
            pinned,
            "digest_in_force must reflect the tree, not the lock"
        );
    }

    #[test]
    fn a_lock_from_the_future_is_refused_rather_than_partially_read() {
        let (_tmp, project) = scaffold();
        std::fs::write(
            lock_path(&project),
            "schema_version: 99\ndesignpack_id: x\ndesignpack_version: v1\ndigest: \
             sha256:0000000000000000000000000000000000000000000000000000000000000000\n\
             generated_at: 2026-07-25T10:00:00Z\nlocked_by: t\n",
        )
        .unwrap();
        let error = read_lock(&project).unwrap_err();
        assert!(error.to_string().contains("VDS S-11(2)"), "{error}");
    }

    #[test]
    fn no_lock_reads_as_none_and_verifies_as_no_lock() {
        let (_tmp, project) = scaffold();
        assert!(read_lock(&project).unwrap().is_none());
        assert_eq!(verify(&project).unwrap(), PackVerdict::NoLock);
    }
}
