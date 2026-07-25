//! Fixture machinery for the proof tests.
//!
//! Every proof test builds a throwaway project here. Nothing in this module may
//! touch a real project's `.vds/`: a test that mutates the record it is checking
//! has proven the mutation, not the record.
//!
//! The harness deliberately writes records through the STORE rather than by
//! hand-assembling YAML. A fixture that bypasses the writer can construct a
//! record the writer would refuse, and a proof tested only against impossible
//! records has been tested against nothing a user can produce.

use std::cell::RefCell;
use std::io::Write;
use std::path::{Path, PathBuf};

use vds_core::{
    Accessibility, CodeCounterpart, ComponentId, ComponentRecord, ContrastFloor, Demand,
    FloorScope, InvokedBy, NameSource, Project, ProofKind, ProofResult, State, StateContract,
    Status, Timestamp, VdsError, default_config,
};
use vds_store::Store;

use crate::{Outcome, ProofContext};

pub struct Harness {
    pub tmp: tempfile::TempDir,
    project: RefCell<Project>,
}

impl Default for Harness {
    fn default() -> Self {
        Self::new()
    }
}

impl Harness {
    pub fn new() -> Harness {
        Harness::with_config(&default_config("demo", "DEMO"))
    }

    pub fn with_config(config: &str) -> Harness {
        let tmp = tempfile::tempdir().expect("a temporary directory");
        for dir in [
            "register", "warrants", "proofs", "pins", "ledgers", "logs", "permits",
        ] {
            std::fs::create_dir_all(tmp.path().join(".vds").join(dir)).unwrap();
        }
        std::fs::write(tmp.path().join(".vds/config.toml"), config).unwrap();
        std::fs::create_dir_all(tmp.path().join("src/components/ui")).unwrap();
        let project = Project::discover(Some(tmp.path())).unwrap();
        Harness {
            project: RefCell::new(project),
            tmp,
        }
    }

    pub fn root(&self) -> PathBuf {
        self.project.borrow().root.clone()
    }

    /// Re-read `.vds/config.toml`, after a test has edited it.
    pub fn reload(&self) {
        let root = self.root();
        *self.project.borrow_mut() = Project::discover(Some(&root)).unwrap();
    }

    pub fn project(&self) -> Project {
        self.project.borrow().clone()
    }

    pub fn store(&self) -> Store<'static> {
        // The project is leaked deliberately: a `Store` borrows its project, and
        // handing a test one that outlives the borrow keeps every call site from
        // needing a `let project = ...` line. The leak is bounded by the test
        // process.
        let project: &'static Project = Box::leak(Box::new(self.project()));
        Store::new(project)
    }

    pub fn write(&self, rel: &str, contents: &str) -> PathBuf {
        let path = self.root().join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, contents).unwrap();
        path
    }

    pub fn read(&self, rel: &str) -> String {
        std::fs::read_to_string(self.root().join(rel)).unwrap()
    }

    /// A screen at `app/<route>/page.tsx` importing and rendering `uses` from
    /// the governed module.
    pub fn screen(&self, route: &str, uses: &[&str]) -> PathBuf {
        self.screen_from(route, uses, "@/components/ui")
    }

    pub fn screen_from(&self, route: &str, uses: &[&str], module: &str) -> PathBuf {
        let body: String = uses.iter().map(|n| format!("      <{n} />\n")).collect();
        self.write(
            &format!("app/{route}/page.tsx"),
            &format!(
                "import {{ {} }} from \"{module}\";\n\nexport default function Page() {{\n  \
                 return (\n    <div>\n{body}    </div>\n  );\n}}\n",
                uses.join(", ")
            ),
        )
    }

    /// A component source file in the governed library directory.
    pub fn component_file(&self, name: &str) -> PathBuf {
        self.write(
            &format!("src/components/ui/{}.tsx", name.to_lowercase()),
            &format!("export function {name}() {{ return <div />; }}\n"),
        )
    }

    /// Regenerate the screens ledger.
    pub fn ledger(&self) {
        let project = self.project();
        vds_scan::write(&project).expect("the ledger generator");
    }

    // -- register ------------------------------------------------------------

    pub fn register(&self, name: &str, status: Status) -> ComponentId {
        let id = ComponentId::allocate(&self.store().register_dir()).unwrap();
        self.register_as(id.as_str(), name, name, status)
    }

    pub fn register_as(
        &self,
        id: &str,
        name: &str,
        export_name: &str,
        status: Status,
    ) -> ComponentId {
        let id = ComponentId::parse(id).unwrap();
        let record = ComponentRecord {
            id: id.clone(),
            name: name.into(),
            status,
            contract_version: 1,
            figma: None,
            code: Some(CodeCounterpart {
                import_path: "@/components/ui".into(),
                source_file: format!("src/components/ui/{}.tsx", export_name.to_lowercase()),
                export_name: export_name.into(),
            }),
            props: vec![],
            states: StateContract {
                required: vec![State::Default],
                drawn: vec![State::Default],
                built: vec![State::Default],
            },
            a11y: Accessibility {
                role: Some("button".into()),
                accessible_name_source: NameSource::Children,
                keyboard: vec![],
                contrast_floors: vec![ContrastFloor {
                    boundary: "control-border".into(),
                    against: "surface".into(),
                    min_ratio: 3.0,
                    basis: "WCAG 2.2 SC 1.4.11".into(),
                    scope: Some(FloorScope::ControlBoundary),
                }],
            },
            demand: Demand {
                routes: 0,
                measured_at: Timestamp::fixed(2026, 7, 25, 10, 0, 0),
                measured_by: "harness".into(),
            },
            supersedes: vec![],
            superseded_by: None,
            amendments: vec![],
            basis: vec!["ACT-VDS-001:s5".into()],
            deprecated_at: matches!(status, Status::Deprecated | Status::Retired)
                .then(|| Timestamp::fixed(2026, 7, 25, 10, 0, 0)),
            retired_at: matches!(status, Status::Retired)
                .then(|| Timestamp::fixed(2026, 7, 25, 10, 0, 0)),
            retirement_proof_id: None,
            notes: None,
        };
        self.put(record);
        id
    }

    pub fn register_unbuilt(&self, name: &str, status: Status) -> ComponentId {
        let id = ComponentId::allocate(&self.store().register_dir()).unwrap();
        let mut record = self.record(&id);
        record.name = name.into();
        record.status = status;
        record.code = None;
        // A record created by `register_as` above; rewrite it without code.
        self.replace(record);
        id
    }

    fn record(&self, id: &ComponentId) -> ComponentRecord {
        // Allocate-then-read requires the record to exist, so create a default
        // one first. Written through the store, so a fixture cannot construct a
        // record the writer would refuse.
        let created = self.register_as(id.as_str(), "Placeholder", "Placeholder", Status::Proposed);
        self.store().read_record(&created).unwrap().value
    }

    pub fn put(&self, record: ComponentRecord) {
        let store = self.store();
        let path = store.record_path(&record.id);
        if path.exists() {
            store.replace(&path, &record).unwrap();
        } else {
            store.create(&path, &record).unwrap();
        }
    }

    pub fn replace(&self, record: ComponentRecord) {
        let store = self.store();
        let path = store.record_path(&record.id);
        store.replace(&path, &record).unwrap();
    }

    pub fn amend(&self, id: &ComponentId, edit: impl FnOnce(&mut ComponentRecord)) {
        let store = self.store();
        let mut record = store.read_record(id).unwrap().value;
        edit(&mut record);
        store.replace(&store.record_path(id), &record).unwrap();
    }

    // -- running proofs ------------------------------------------------------

    pub fn context(&self) -> ProofContext<'static> {
        let project: &'static Project = Box::leak(Box::new(self.project()));
        ProofContext {
            project,
            invoked_by: InvokedBy::CiWorkflow,
            allow_vacuous: false,
            capture: true,
        }
    }

    pub fn last_proof(&self, kind: ProofKind) -> ProofResult {
        self.store()
            .latest_proof(kind)
            .unwrap()
            .unwrap_or_else(|| panic!("no captured {kind} record"))
            .value
    }

    pub fn run_kind_err(&self, kind: ProofKind) -> VdsError {
        let ctx = self.context();
        crate::run(kind, &ctx, &mut Vec::new()).expect_err("expected a precondition failure")
    }
}

/// Run one proof kind against a harness, returning the outcome and everything
/// it printed.
pub fn run_kind(harness: &Harness, kind: ProofKind) -> (Outcome, String) {
    let ctx = harness.context();
    let mut out: Vec<u8> = Vec::new();
    let outcome = crate::run(kind, &ctx, &mut out)
        .unwrap_or_else(|e| panic!("{kind} failed its preconditions: {e}"));
    (outcome, String::from_utf8(out).expect("utf-8 output"))
}

/// Append to a file, for tests that need to move a digest without changing
/// meaning.
pub fn touch(path: &Path) {
    let mut file = std::fs::OpenOptions::new().append(true).open(path).unwrap();
    writeln!(file, "// touched").unwrap();
}
