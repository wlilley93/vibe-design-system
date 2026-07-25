//! Reading and writing the `.vds/` record.
//!
//! One rule shapes this crate: **a collision is a fail-closed validation error,
//! never a silent overwrite** (VDS S-4(4)). Every create goes through
//! [`Store::create`], which refuses to write over an existing path, and every
//! amendment goes through [`Store::replace`], which refuses to write a path that
//! is not already there. There is deliberately no "write" that does either,
//! because the two intentions look identical at the call site and only one of
//! them can destroy a record.
//!
//! The second rule: **the record is committed, not scratch** (VDS S-3(9)). This
//! crate writes YAML that a human reads in a diff, and it writes it atomically,
//! so a reader never sees a half-written governance record.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

use vds_core::{
    ComponentId, ComponentRecord, Digest, EnforcementLock, LOCK_FILE_NAME, LOCK_SCHEMA_VERSION,
    PathRole, Pin, Project, ProofId, ProofKind, ProofResult, Result, Stage, Submission, Timestamp,
    VdsError, Warrant, WarrantId, WarrantStatus, write_text_atomically, yaml_files,
};

pub mod lock;

pub use lock::{repin_lock, verify_lock, write_lock};

/// A typed view over one project's record directories.
pub struct Store<'a> {
    pub project: &'a Project,
}

/// An artefact read from disk, with the path it came from.
///
/// The path travels with the value because every message VDS prints about a
/// record names the file, and a reader told "CMP-0004 is wrong" without being
/// told where it lives has been given half a finding.
#[derive(Debug, Clone)]
pub struct Located<T> {
    pub path: PathBuf,
    pub value: T,
}

impl<T> std::ops::Deref for Located<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}

impl<'a> Store<'a> {
    pub fn new(project: &'a Project) -> Self {
        Self { project }
    }

    // -- generic artefact IO -------------------------------------------------

    /// Read and parse one artefact.
    ///
    /// A parse failure names the file and the serde path inside it, because "the
    /// register is broken" is not a finding a reader can act on.
    pub fn read<T: DeserializeOwned>(&self, path: &Path) -> Result<T> {
        let text = std::fs::read_to_string(path).map_err(|e| VdsError::io(path.display(), e))?;
        serde_yaml::from_str(&text).map_err(|e| VdsError::Artefact {
            path: self.project.rel(path),
            message: format!("does not parse as the artefact it is filed as: {e}"),
        })
    }

    /// Write an artefact that must NOT already exist.
    pub fn create<T: Serialize>(&self, path: &Path, value: &T) -> Result<()> {
        if path.exists() {
            return Err(VdsError::Artefact {
                path: self.project.rel(path),
                message: "already exists. An identifier collision is a fail-closed validation \
                          error, never a silent overwrite (VDS S-4(4))."
                    .into(),
            });
        }
        self.emit(path, value)
    }

    /// Write an artefact that MUST already exist.
    pub fn replace<T: Serialize>(&self, path: &Path, value: &T) -> Result<()> {
        if !path.is_file() {
            return Err(VdsError::Artefact {
                path: self.project.rel(path),
                message: "does not exist, so there is nothing to amend. Creating a record \
                          through an amendment path would produce a record with no origin."
                    .into(),
            });
        }
        self.emit(path, value)
    }

    fn emit<T: Serialize>(&self, path: &Path, value: &T) -> Result<()> {
        let text = serde_yaml::to_string(value).map_err(|e| VdsError::Serialize {
            what: self.project.rel(path),
            message: e.to_string(),
        })?;
        write_text_atomically(path, &text)
    }

    /// Read every artefact of one kind, in path order.
    ///
    /// An unparseable file is an ERROR and not a skip. A reader of a partial
    /// register is reading a register that says something the directory does
    /// not, and every count taken from it is wrong by an unknown amount.
    pub fn read_all<T: DeserializeOwned>(&self, directory: &Path) -> Result<Vec<Located<T>>> {
        let mut out = Vec::new();
        for path in yaml_files(directory)? {
            out.push(Located {
                value: self.read(&path)?,
                path,
            });
        }
        Ok(out)
    }

    // -- the register --------------------------------------------------------

    pub fn register_dir(&self) -> PathBuf {
        self.project.path(PathRole::Register)
    }

    pub fn record_path(&self, id: &ComponentId) -> PathBuf {
        self.register_dir().join(format!("{id}.yaml"))
    }

    pub fn read_register(&self) -> Result<Vec<Located<ComponentRecord>>> {
        // A register record this reader cannot see is worse than one it cannot
        // parse. An adversarial reviewer put a `verified` record requiring four
        // states and drawing none into a subdirectory: the gate went green AND
        // produced an evidence digest byte-identical to the clean project, so a
        // warrant citing that digest could not distinguish the two states of
        // the world. The row set has to be the DIRECTORY, not a filtered view
        // of it.
        self.refuse_unreadable_entries(&self.register_dir(), "register record")?;
        let records: Vec<Located<ComponentRecord>> = self.read_all(&self.register_dir())?;
        // A record whose filename disagrees with its `id` is ambiguous: one of
        // the two is the identifier and nothing says which. Refuse rather than
        // pick, because picking silently decides which of two identifiers the
        // allocator will avoid next time.
        for record in &records {
            let expected = format!("{}.yaml", record.value.id);
            let actual = record
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if actual != expected {
                return Err(VdsError::Artefact {
                    path: self.project.rel(&record.path),
                    message: format!(
                        "is filed as {actual} and carries id {}. One of those is the \
                         identifier and nothing says which, so this is refused rather than \
                         guessed (VDS S-4(4)). Rename the file to {expected}.",
                        record.value.id
                    ),
                });
            }
        }
        Ok(records)
    }

    /// Refuse any entry in a record directory this reader would not pick up.
    ///
    /// `yaml_files` takes files ending `.yaml`, so a subdirectory, a `.yml`, a
    /// `.YAML` or a stray note is invisible to every count and every proof. That
    /// invisibility is the defect: a record nobody reads is not an absent record,
    /// it is a record whose absence nothing reports.
    fn refuse_unreadable_entries(&self, directory: &Path, what: &str) -> Result<()> {
        if !directory.is_dir() {
            return Ok(());
        }
        let mut unseen = Vec::new();
        for entry in
            std::fs::read_dir(directory).map_err(|e| VdsError::io(directory.display(), e))?
        {
            let entry = entry.map_err(|e| VdsError::io(directory.display(), e))?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                unseen.push(format!(
                    "{name}/ is a directory, and the reader does not recurse"
                ));
                continue;
            }
            let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if extension == "yaml" {
                continue;
            }
            // `.gitkeep` and the like are how an empty directory is committed,
            // and are not a record pretending to be absent.
            if name.starts_with('.') {
                continue;
            }
            if extension.eq_ignore_ascii_case("yaml") || extension.eq_ignore_ascii_case("yml") {
                unseen.push(format!(
                    "{name} is not read: the reader takes `.yaml` exactly, and this is `.{extension}`"
                ));
            } else {
                unseen.push(format!(
                    "{name} is not a `.yaml` file and is read by nothing"
                ));
            }
        }
        if unseen.is_empty() {
            return Ok(());
        }
        unseen.sort();
        Err(VdsError::Artefact {
            path: self.project.rel(directory),
            message: format!(
                "holds {} entries that no proof will ever see, so a {what} placed in one of \
                 them is invisible to every count and every digest:\n    {}\n  \
                 That invisibility is the defect. A record nobody reads is not an absent \
                 record, it is a record whose absence nothing reports, and a warrant citing a \
                 digest taken over the readable subset cannot distinguish the two states of \
                 the world. Rename or remove them.",
                unseen.len(),
                unseen.join("\n    ")
            ),
        })
    }

    pub fn read_record(&self, id: &ComponentId) -> Result<Located<ComponentRecord>> {
        let path = self.record_path(id);
        if !path.is_file() {
            return Err(VdsError::precondition(format!(
                "no register record at {}",
                self.project.rel(&path)
            )));
        }
        Ok(Located {
            value: self.read(&path)?,
            path,
        })
    }

    // -- warrants ------------------------------------------------------------

    pub fn warrants_dir(&self) -> PathBuf {
        self.project.path(PathRole::Warrants)
    }

    pub fn warrant_path(&self, id: &WarrantId) -> PathBuf {
        self.warrants_dir().join(format!("{id}.yaml"))
    }

    pub fn read_warrants(&self) -> Result<Vec<Located<Warrant>>> {
        self.read_all(&self.warrants_dir())
    }

    pub fn read_warrant(&self, id: &WarrantId) -> Result<Located<Warrant>> {
        let path = self.warrant_path(id);
        if !path.is_file() {
            return Err(VdsError::precondition(format!(
                "no warrant at {}",
                self.project.rel(&path)
            )));
        }
        Ok(Located {
            value: self.read(&path)?,
            path,
        })
    }

    /// The granted warrant for a stage, if there is one.
    pub fn granted_warrant(&self, stage: Stage) -> Result<Option<Located<Warrant>>> {
        Ok(self
            .read_warrants()?
            .into_iter()
            .find(|w| w.value.stage == stage && w.value.status == WarrantStatus::Granted))
    }

    // -- proofs --------------------------------------------------------------

    pub fn proofs_dir(&self) -> PathBuf {
        self.project.path(PathRole::Proofs)
    }

    pub fn proof_path(&self, id: &ProofId) -> PathBuf {
        self.proofs_dir().join(format!("{id}.yaml"))
    }

    pub fn read_proofs(&self) -> Result<Vec<Located<ProofResult>>> {
        self.read_all(&self.proofs_dir())
    }

    pub fn read_proof(&self, id: &ProofId) -> Result<Located<ProofResult>> {
        let path = self.proof_path(id);
        if !path.is_file() {
            return Err(VdsError::precondition(format!(
                "no proof record {id} on disk at {}",
                self.project.rel(&path)
            )));
        }
        Ok(Located {
            value: self.read(&path)?,
            path,
        })
    }

    fn sorted_by_capture(mut hits: Vec<Located<ProofResult>>) -> Vec<Located<ProofResult>> {
        // Capture time first, then id. Two records captured in the same second
        // would otherwise sort by whichever the filesystem listed first, and
        // "the latest proof" would depend on the directory's mood.
        hits.sort_by(|a, b| {
            a.value
                .captured_at
                .cmp(&b.value.captured_at)
                .then_with(|| a.value.id.cmp(&b.value.id))
        });
        hits
    }

    /// The most recent CITABLE proof of a kind.
    ///
    /// Citable, not merely passed: VDS S-7(2)(4) excludes a vacuous run, and a
    /// pass over zero enforceable rows is the [2026] VJS-CC-OPBOX 3 D3 defect
    /// rather than evidence.
    pub fn latest_citable_proof(&self, kind: ProofKind) -> Result<Option<Located<ProofResult>>> {
        let hits = self
            .read_proofs()?
            .into_iter()
            .filter(|p| p.value.kind == kind && p.value.is_citable_evidence())
            .collect();
        Ok(Self::sorted_by_capture(hits).pop())
    }

    /// The most recent proof of a kind whatever its status, for reporting what
    /// the last run actually said.
    pub fn latest_proof(&self, kind: ProofKind) -> Result<Option<Located<ProofResult>>> {
        let hits = self
            .read_proofs()?
            .into_iter()
            .filter(|p| p.value.kind == kind)
            .collect();
        Ok(Self::sorted_by_capture(hits).pop())
    }

    /// One row per proof kind: how many records exist, and what the last one
    /// said. The basis for the D2, D3 and D9 counts in docs/GOAL.md.
    pub fn proof_census(&self) -> Result<BTreeMap<ProofKind, (usize, Option<ProofResult>)>> {
        let proofs = self.read_proofs()?;
        let mut out = BTreeMap::new();
        for kind in ProofKind::ALL {
            let of_kind: Vec<Located<ProofResult>> = proofs
                .iter()
                .filter(|p| p.value.kind == kind)
                .cloned()
                .collect();
            let count = of_kind.len();
            let last = Self::sorted_by_capture(of_kind).pop().map(|p| p.value);
            out.insert(kind, (count, last));
        }
        Ok(out)
    }

    // -- pins ----------------------------------------------------------------

    pub fn pins_dir(&self) -> PathBuf {
        self.project.path(PathRole::Pins)
    }

    pub fn read_pins(&self) -> Result<Vec<Located<Pin>>> {
        self.read_all(&self.pins_dir())
    }

    // -- submissions ---------------------------------------------------------

    pub fn submissions_dir(&self) -> PathBuf {
        self.project.path(PathRole::Submissions)
    }

    /// Every submission across `draft/`, `filed/` and `docket/`.
    pub fn read_submissions(&self) -> Result<Vec<Located<Submission>>> {
        let mut out = Vec::new();
        for stage in ["draft", "filed", "docket"] {
            out.extend(self.read_all::<Submission>(&self.submissions_dir().join(stage))?);
        }
        Ok(out)
    }

    // -- the enforcement lock ------------------------------------------------

    pub fn lock_path(&self) -> PathBuf {
        self.project.enforcement_lock_path()
    }

    /// Read the lock, or `None`.
    ///
    /// VDS S-8(3): the lock is opt-in, so a repository with no lock file is
    /// quiet rather than broken. A lock file that is PRESENT and unreadable is
    /// not quiet, because a project that pinned its gates and then broke the pin
    /// file has lost its witness and does not know it.
    pub fn read_lock(&self) -> Result<Option<EnforcementLock>> {
        let path = self.lock_path();
        if !path.is_file() {
            return Ok(None);
        }
        let text = std::fs::read_to_string(&path).map_err(|e| VdsError::io(path.display(), e))?;
        let raw: serde_yaml::Value =
            serde_yaml::from_str(&text).map_err(|e| VdsError::Artefact {
                path: self.project.rel(&path),
                message: format!("is not readable YAML: {e}"),
            })?;
        let found = raw
            .get("schema_version")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        if found > LOCK_SCHEMA_VERSION {
            return Err(VdsError::SchemaVersionAhead {
                path: self.project.rel(&path),
                kind: LOCK_FILE_NAME,
                found,
                understood: LOCK_SCHEMA_VERSION,
            });
        }
        let lock: EnforcementLock =
            serde_yaml::from_value(raw).map_err(|e| VdsError::Artefact {
                path: self.project.rel(&path),
                message: format!("is not an enforcement lock: {e}"),
            })?;
        Ok(Some(lock))
    }

    // -- surface -------------------------------------------------------------

    /// The digest of the register directory. Half of the surface a warrant is
    /// granted over (VDS S-6(4)).
    pub fn register_digest(&self) -> Result<Digest> {
        self.project.register_digest()
    }

    pub fn now(&self) -> Timestamp {
        Timestamp::now()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vds_core::{
        Accessibility, CodeCounterpart, Demand, NameSource, State, StateContract, Status,
        default_config,
    };

    struct Scaffold {
        _tmp: tempfile::TempDir,
        project: Project,
    }

    fn scaffold() -> Scaffold {
        let tmp = tempfile::tempdir().unwrap();
        let vds = tmp.path().join(".vds");
        std::fs::create_dir_all(vds.join("register")).unwrap();
        std::fs::write(vds.join("config.toml"), default_config("demo", "DEMO")).unwrap();
        let project = Project::discover(Some(tmp.path())).unwrap();
        Scaffold { _tmp: tmp, project }
    }

    fn record(id: &str) -> ComponentRecord {
        ComponentRecord {
            id: ComponentId::parse(id).unwrap(),
            name: "Button".into(),
            status: Status::Registered,
            contract_version: 1,
            figma: None,
            code: Some(CodeCounterpart {
                import_path: "@/components/ui".into(),
                source_file: "src/components/ui/button.tsx".into(),
                export_name: "Button".into(),
            }),
            props: vec![],
            states: StateContract {
                required: vec![State::Default],
                drawn: vec![State::Default],
                built: vec![],
            },
            a11y: Accessibility {
                role: Some("button".into()),
                accessible_name_source: NameSource::Children,
                keyboard: vec![],
                contrast_floors: vec![],
            },
            demand: Demand {
                routes: 0,
                measured_at: Timestamp::fixed(2026, 7, 25, 10, 0, 0),
                measured_by: "test".into(),
            },
            supersedes: vec![],
            superseded_by: None,
            amendments: vec![],
            basis: vec!["ACT-VDS-001:s5".into()],
            deprecated_at: None,
            retired_at: None,
            retirement_proof_id: None,
            notes: None,
        }
    }

    #[test]
    fn create_then_read_round_trips() {
        let s = scaffold();
        let store = Store::new(&s.project);
        let value = record("CMP-0001");
        store.create(&store.record_path(&value.id), &value).unwrap();
        assert_eq!(store.read_record(&value.id).unwrap().value, value);
    }

    #[test]
    fn create_refuses_to_overwrite() {
        let s = scaffold();
        let store = Store::new(&s.project);
        let value = record("CMP-0001");
        let path = store.record_path(&value.id);
        store.create(&path, &value).unwrap();
        let err = store.create(&path, &value).unwrap_err();
        assert!(
            err.to_string().contains("never a silent overwrite"),
            "{err}"
        );
    }

    #[test]
    fn replace_refuses_to_create() {
        let s = scaffold();
        let store = Store::new(&s.project);
        let value = record("CMP-0001");
        let err = store
            .replace(&store.record_path(&value.id), &value)
            .unwrap_err();
        assert!(err.to_string().contains("nothing to amend"), "{err}");
    }

    #[test]
    fn a_record_filed_under_the_wrong_name_is_refused() {
        let s = scaffold();
        let store = Store::new(&s.project);
        store
            .create(
                &store.register_dir().join("button.yaml"),
                &record("CMP-0001"),
            )
            .unwrap();
        let err = store.read_register().unwrap_err();
        assert!(err.to_string().contains("nothing says which"), "{err}");
    }

    #[test]
    fn an_unparseable_record_is_an_error_and_not_a_skip() {
        let s = scaffold();
        let store = Store::new(&s.project);
        std::fs::write(
            store.register_dir().join("CMP-0002.yaml"),
            "not: a record\n",
        )
        .unwrap();
        let err = store.read_register().unwrap_err();
        assert!(
            err.to_string().contains("CMP-0002"),
            "the error must name the file: {err}"
        );
    }

    #[test]
    fn a_record_in_a_subdirectory_is_refused_rather_than_invisible() {
        let s = scaffold();
        let store = Store::new(&s.project);
        std::fs::create_dir_all(store.register_dir().join("nested")).unwrap();
        std::fs::write(
            store.register_dir().join("nested/CMP-0009.yaml"),
            "id: CMP-0009\n",
        )
        .unwrap();
        let err = store.read_register().unwrap_err();
        assert!(err.to_string().contains("no proof will ever see"), "{err}");
        assert!(err.to_string().contains("nested/"), "{err}");
    }

    #[test]
    fn a_record_with_a_yml_extension_is_refused_rather_than_invisible() {
        let s = scaffold();
        let store = Store::new(&s.project);
        std::fs::write(store.register_dir().join("CMP-0009.yml"), "id: CMP-0009\n").unwrap();
        let err = store.read_register().unwrap_err();
        assert!(err.to_string().contains("`.yaml` exactly"), "{err}");
    }

    #[test]
    fn a_dotfile_in_the_register_is_not_a_hidden_record() {
        let s = scaffold();
        let store = Store::new(&s.project);
        std::fs::write(store.register_dir().join(".gitkeep"), "").unwrap();
        assert!(store.read_register().unwrap().is_empty());
    }

    #[test]
    fn an_absent_directory_reads_as_empty() {
        let s = scaffold();
        let store = Store::new(&s.project);
        assert!(store.read_warrants().unwrap().is_empty());
        assert!(store.read_proofs().unwrap().is_empty());
        assert!(store.read_pins().unwrap().is_empty());
        assert!(store.read_submissions().unwrap().is_empty());
    }

    #[test]
    fn no_lock_reads_as_none_rather_than_an_error() {
        let s = scaffold();
        assert!(Store::new(&s.project).read_lock().unwrap().is_none());
    }

    #[test]
    fn a_lock_from_the_future_is_refused_rather_than_partially_read() {
        let s = scaffold();
        let store = Store::new(&s.project);
        std::fs::write(
            store.lock_path(),
            "schema_version: 99\ngenerated_at: 2026-07-25T10:00:00Z\nentries: []\n",
        )
        .unwrap();
        let err = store.read_lock().unwrap_err();
        assert!(err.to_string().contains("VDS S-11(2)"), "{err}");
    }
}
