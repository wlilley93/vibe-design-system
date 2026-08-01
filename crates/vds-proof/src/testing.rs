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
    Accessibility, ArrangementContract, BoundEntry, CodeCounterpart, ComponentId, ComponentRecord,
    ContrastFloor, Demand, FigmaFrame, FloorScope, GeometryBound, GeometryId, GeometryReading,
    InvokedBy, KindReading, NameSource, Project, ProofKind, ProofResult, ReadFrom, ScreenId,
    ScreenRecord, State, StateContract, Status, SurfaceKind, Timestamp, VdsError, default_config,
};
use vds_store::Store;

use crate::{Outcome, ProofContext};

/// One component's variant axes for `Harness::figma_variants`: the component,
/// then each axis name with its legal values.
pub type VariantRow<'a> = (&'a vds_core::ComponentId, &'a [(&'a str, &'a [&'a str])]);

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
            "register",
            "screens",
            "geometry",
            "prohibitions",
            "burndowns",
            "signoffs",
            "redraws",
            "reviews",
            "warrants",
            "proofs",
            "pins",
            "ledgers",
            "logs",
            "permits",
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

    // -- geometry -------------------------------------------------------------

    /// Write a geometry bound with the given history, as `(date, bound)` pairs
    /// OLDEST FIRST.
    ///
    /// The history is spelled out at every call site rather than defaulted,
    /// because the direction rule is the whole subject of this kind and a
    /// fixture that supplies a plausible history for you is a fixture that
    /// decides the answer.
    pub fn geometry_bound(
        &self,
        kind: SurfaceKind,
        window_days: u32,
        history: &[(&str, u32)],
    ) -> PathBuf {
        let store = self.store();
        let id = GeometryId::allocate(&store.geometry_dir()).expect("a geometry id");
        let record = GeometryBound {
            id: id.clone(),
            surface_kind: kind,
            status: Status::Registered,
            declared_window_days: window_days,
            history: history
                .iter()
                .map(|(at, bound)| BoundEntry {
                    at: Timestamp::parse(format!("{at}T00:00:00Z")).expect("a fixture date"),
                    bound: *bound,
                    because: None,
                })
                .collect(),
            basis: vec!["VDS S-7A(2)".into()],
            notes: None,
        };
        let path = store.geometry_path(&id);
        let text = serde_yaml::to_string(&record).expect("a serialisable bound");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        path
    }

    /// Write a geometry reading covering one surface kind.
    pub fn geometry_reading(
        &self,
        taken: &str,
        read_from: ReadFrom,
        kinds: &[(SurfaceKind, u32, u32, u32)],
    ) -> PathBuf {
        let mut reading = GeometryReading {
            schema_version: vds_core::READING_SCHEMA_VERSION,
            generated_by: "vds ledger geometry".into(),
            taken_at: Timestamp::parse(format!("{taken}T00:00:00Z")).expect("a fixture date"),
            read_from,
            sources: vec![".next/static/css/app.css".into()],
            kinds: kinds
                .iter()
                .map(|(kind, considered, non_compliant, undecided)| KindReading {
                    surface_kind: *kind,
                    considered: *considered,
                    non_compliant: *non_compliant,
                    undecided: *undecided,
                    sample: vec![],
                })
                .collect(),
            does_not_cover: vec!["inline style attributes".into()],
            content_digest: vds_core::Digest::of_text("placeholder"),
        };
        // Computed rather than asserted, so every fixture is a reading the
        // generator could really have produced. A fixture that carried a wrong
        // digest would trip R10 in every test and hide whatever each one is about.
        reading.content_digest = reading.compute_content_digest().expect("a digest");
        let project = self.project();
        vds_core::write_reading(&project, &reading).expect("a written reading")
    }

    /// Write a figma ledger giving one registered component a set of variant
    /// properties and their legal values.
    ///
    /// Spelled out per call rather than defaulted, because the whole subject of
    /// the R11/R12 limb is whether two value sets are the same set, and a fixture
    /// that supplies a plausible set for you decides the answer.
    pub fn figma_variants(&self, rows: &[VariantRow<'_>]) {
        use std::collections::BTreeMap;
        use vds_figma::ledger::{
            FigmaLedger, FigmaNodeRow, GENERATOR_COMMAND, LEDGER_SCHEMA_VERSION,
        };
        // Built here rather than borrowed from `vds_figma::testing`, which is
        // `#[cfg(test)]` and therefore does not exist outside its own crate.
        let mut ledger = FigmaLedger {
            schema_version: LEDGER_SCHEMA_VERSION,
            generated_at: Timestamp::fixed(2026, 7, 25, 10, 0, 0),
            generated_by: GENERATOR_COMMAND.into(),
            file_key: "KEY".into(),
            file_version: "1".into(),
            file_name: "Decided target".into(),
            content_digest: vds_core::Digest::of_text("placeholder"),
            nodes: rows
                .iter()
                .map(|(id, variants)| {
                    let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
                    for (name, values) in variants.iter() {
                        map.insert(
                            (*name).to_owned(),
                            values.iter().map(|v| (*v).to_owned()).collect(),
                        );
                    }
                    FigmaNodeRow {
                        component_id: (*id).clone(),
                        node_id: "12:34".into(),
                        resolved: true,
                        figma_name: Some("Node".into()),
                        is_component_set: true,
                        variant_properties: map,
                        states_drawn: vec![State::Default],
                        unresolved_because: None,
                    }
                })
                .collect(),
            unclaimed: vec![],
            notes: vec![],
        };
        ledger.content_digest = ledger.compute_content_digest().unwrap();
        vds_figma::pull::write(&self.store(), &ledger).expect("the figma ledger writer");
    }

    /// Give a registered record a prop with a `figmaProperty` correspondence.
    pub fn prop_with_variant(
        &self,
        id: &vds_core::ComponentId,
        name: &str,
        type_expr: &str,
        figma_property: &str,
    ) {
        let store = self.store();
        let mut record = store.read_record(id).unwrap().value;
        record.props.push(vds_core::PropContract {
            name: name.into(),
            type_expr: type_expr.into(),
            required: true,
            figma_property: Some(figma_property.into()),
        });
        store.replace(&store.record_path(id), &record).unwrap();
    }

    /// Regenerate the screens ledger.
    pub fn ledger(&self) {
        let project = self.project();
        vds_scan::write(&project).expect("the ledger generator");
    }

    // -- prohibitions ---------------------------------------------------------

    /// A prohibition whose expansion is recorded from the CURRENT tree, exactly
    /// as `vds prohibition add` records it.
    pub fn prohibition(&self, pattern: &str, scope: &[&str]) -> PathBuf {
        self.prohibition_with_status(pattern, scope, "registered")
    }

    pub fn prohibition_with_status(&self, pattern: &str, scope: &[&str], status: &str) -> PathBuf {
        let store = self.store();
        let project = self.project();
        let id = vds_core::ProhibitionId::allocate(&store.prohibitions_dir()).unwrap();
        let scope: Vec<String> = scope.iter().map(|s| (*s).to_owned()).collect();
        let mut expansion: Vec<String> = vds_scan::glob::match_globs(&project.root, &scope)
            .unwrap()
            .iter()
            .map(|p| project.rel(p))
            .collect();
        expansion.sort();
        let record = vds_core::ProhibitionRecord {
            id: id.clone(),
            status: Status::parse(status).unwrap(),
            pattern: pattern.into(),
            scope,
            expansion,
            directed_at: Some(Timestamp::fixed(2026, 8, 1, 10, 0, 0)),
            because: None,
            basis: vec!["draft S-7B".into()],
            notes: None,
        };
        let path = store.prohibition_path(&id);
        store.create(&path, &record).unwrap();
        path
    }

    // -- burndowns ------------------------------------------------------------

    /// A burndown with the given pin history, `(date, value)` OLDEST FIRST.
    pub fn burndown(
        &self,
        metric: &str,
        deadline: Option<&str>,
        history: &[(&str, u64)],
    ) -> PathBuf {
        self.burndown_aged(metric, deadline, deadline.map(|_| 7), history)
    }

    /// A burndown declaring its own maximum reading age, for the S-7C(5) seeds.
    pub fn burndown_aged(
        &self,
        metric: &str,
        deadline: Option<&str>,
        max_reading_age_days: Option<u32>,
        history: &[(&str, u64)],
    ) -> PathBuf {
        let store = self.store();
        let id = vds_core::BurndownId::allocate(&store.burndowns_dir()).unwrap();
        let record = vds_core::BurndownRecord {
            id: id.clone(),
            status: Status::Registered,
            metric: metric.into(),
            deadline: deadline
                .map(|d| Timestamp::parse(format!("{d}T00:00:00Z")).expect("a fixture date")),
            max_reading_age_days,
            history: history
                .iter()
                .map(|(at, value)| vds_core::PinnedValue {
                    at: Timestamp::parse(format!("{at}T00:00:00Z")).expect("a fixture date"),
                    value: *value,
                    because: None,
                })
                .collect(),
            basis: vec!["draft S-7C".into()],
            notes: None,
        };
        let path = store.burndown_path(&id);
        store.create(&path, &record).unwrap();
        path
    }

    /// A burndown reading covering the given metrics, `(metric, value)`.
    pub fn burndown_reading(&self, taken: &str, rows: &[(&str, u64)]) -> PathBuf {
        let mut reading = vds_core::BurndownReading {
            schema_version: vds_core::BURNDOWN_READING_SCHEMA_VERSION,
            generated_by: "vds ledger burndown --from -".into(),
            taken_at: Timestamp::parse(format!("{taken}T00:00:00Z")).expect("a fixture date"),
            rows: rows
                .iter()
                .map(|(metric, value)| vds_core::BurndownRow {
                    metric: (*metric).to_owned(),
                    value: *value,
                    measured_by: Some("a named counter".into()),
                })
                .collect(),
            does_not_cover: vec![],
            content_digest: vds_core::Digest::of_text("placeholder"),
        };
        reading.content_digest = reading.compute_content_digest().expect("a digest");
        let project = self.project();
        vds_core::write_burndown_reading(&project, &reading).expect("a written reading")
    }

    /// Write a geometry authority snapshot bound to the CURRENT reading and a
    /// capture file this helper writes, with the given agreement rows.
    pub fn geometry_authority(
        &self,
        file_key: &str,
        node_id: &str,
        rows: &[(SurfaceKind, vds_core::AgreementState)],
    ) -> PathBuf {
        let project = self.project();
        let capture_rel = "design/captures/geometry-authority.json";
        let capture = self.write(
            capture_rel,
            "{\"nodes\":{\"1:2\":{\"document\":{\"name\":\"decided\"}}}}\n",
        );
        // The comparator is a real file in the fixture tree, because the
        // proof digests it and W3 asks whether the lock pins it.
        let comparator_rel = "scripts/geometry-comparator.py";
        let comparator = self.write(
            comparator_rel,
            "# the out-of-band comparator that produced the agreement rows\n",
        );
        let reading = vds_core::read_reading(&project)
            .expect("a readable reading")
            .expect("a generated geometry reading to bind to");
        let mut snapshot = vds_core::GeometryAuthority {
            schema_version: vds_core::AUTHORITY_SNAPSHOT_SCHEMA_VERSION,
            generated_by: "vds ledger geometry-authority --from -".into(),
            fetched_at: Timestamp::fixed(2026, 8, 1, 11, 0, 0),
            file_key: file_key.into(),
            node_id: node_id.into(),
            capture: capture_rel.into(),
            capture_digest: vds_core::Digest::of_file(&capture).unwrap(),
            reading_digest: reading.content_digest.clone(),
            comparator: comparator_rel.into(),
            comparator_digest: vds_core::Digest::of_file(&comparator).unwrap(),
            rows: rows
                .iter()
                .map(|(kind, agrees)| vds_core::AuthorityAgreement {
                    surface_kind: *kind,
                    agrees: *agrees,
                    because: match agrees {
                        vds_core::AgreementState::Disagrees => {
                            Some("the shipped step sits off the decided scale".to_owned())
                        }
                        vds_core::AgreementState::NotDrawn => {
                            Some("the signed frame draws no radius at all".to_owned())
                        }
                        vds_core::AgreementState::Agrees => None,
                    },
                })
                .collect(),
            content_digest: vds_core::Digest::of_text("placeholder"),
        };
        snapshot.content_digest = snapshot.compute_content_digest().expect("a digest");
        vds_core::write_authority(&project, &snapshot).expect("a written snapshot")
    }

    // -- sign-offs, redraws, reviews ------------------------------------------

    /// Sign a frame off at its CURRENT content digest, as read from the frames
    /// ledger. Panics if the frame has no current digest: a fixture signing a
    /// hash nothing measured would decide the answer.
    pub fn signoff(&self, file_key: &str, node_id: &str) -> vds_core::SignoffId {
        let project = self.project();
        let ledger = vds_figma::frames::read(&project)
            .expect("the frames ledger")
            .expect("a generated frames ledger");
        let digest = ledger
            .row(node_id)
            .and_then(|r| r.content_digest.clone())
            .expect("a frame with a current content digest");
        self.signoff_at(file_key, node_id, digest)
    }

    /// Sign a frame off at an EXPLICIT digest, for the staleness seeds.
    pub fn signoff_at(
        &self,
        file_key: &str,
        node_id: &str,
        frame_digest: vds_core::Digest,
    ) -> vds_core::SignoffId {
        let store = self.store();
        let id = vds_core::SignoffId::allocate(&store.signoffs_dir()).unwrap();
        let record = vds_core::SignOff {
            id: id.clone(),
            file_key: file_key.into(),
            node_id: node_id.into(),
            frame_digest,
            signed_by: "the principal".into(),
            signed_at: Timestamp::fixed(2026, 8, 1, 10, 0, 0),
            notes: None,
        };
        let path = store.signoff_path(&id);
        store.create(&path, &record).unwrap();
        id
    }

    /// A registered Principal direction, hash-bound to a decision log entry
    /// this helper writes into the project tree.
    pub fn direction(&self, surface: vds_core::DirectedSurface) -> vds_core::DirectionId {
        let store = self.store();
        let id = vds_core::DirectionId::allocate(&store.directions_dir()).unwrap();
        let log_rel = format!("logs/{id}-decision.md");
        let log = self.write(
            &log_rel,
            "LOG-2026-08-01-104739: the band goes off-screen for now.\n",
        );
        let record = vds_core::DirectionRecord {
            id: id.clone(),
            log_id: log_rel,
            decision_digest: vds_core::Digest::of_file(&log).unwrap(),
            surface,
            direction: "band off-screen".into(),
            magnitude: "the whole band, until the frames carry it".into(),
            directed_at: Timestamp::fixed(2026, 8, 1, 10, 47, 39),
            notes: None,
        };
        let path = store.direction_path(&id);
        store.create(&path, &record).unwrap();
        id
    }

    /// Rewrite a direction's log entry, so its decisionDigest lapses.
    pub fn move_direction_log(&self, id: &vds_core::DirectionId) {
        let store = self.store();
        let record: vds_core::DirectionRecord = store.read(&store.direction_path(id)).unwrap();
        self.write(
            &record.log_id,
            "the direction was rewritten after registration\n",
        );
    }

    pub fn review(&self, record: vds_core::VisualReviewRecord) -> PathBuf {
        let store = self.store();
        let path = store.review_path(&record.id);
        store.create(&path, &record).unwrap();
        path
    }

    pub fn redraw(&self, record: vds_core::RedrawRecord) -> PathBuf {
        let store = self.store();
        let path = store.redraw_path(&record.id);
        store.create(&path, &record).unwrap();
        path
    }

    /// The estate's route manifest: the enumeration a stage-4 run reports
    /// against. Spelled out per call rather than derived from the screens
    /// ledger, because the whole subject of the coverage limb is that the
    /// estate says what is in the programme and the proof reports on THAT.
    pub fn route_manifest(&self, routes: &[&str]) -> PathBuf {
        let mut manifest = vds_core::RouteManifest {
            schema_version: vds_core::ROUTE_MANIFEST_SCHEMA_VERSION,
            generated_by: "vds ledger routes --from -".into(),
            taken_at: Timestamp::fixed(2026, 8, 1, 9, 0, 0),
            source: "the estate's route tracker".into(),
            routes: routes.iter().map(|r| (*r).to_owned()).collect(),
            does_not_cover: vec![],
            content_digest: vds_core::Digest::of_text("placeholder"),
        };
        manifest.content_digest = manifest.compute_content_digest().expect("a digest");
        let project = self.project();
        vds_core::write_route_manifest(&project, &manifest).expect("a written manifest")
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
            measured_by: vec![],
            directed_at: None,
            grace_days: None,
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

    // -- the screen register -------------------------------------------------

    /// A screen record, written THROUGH THE STORE.
    ///
    /// The same rule as `register_as` above and for the same reason: a fixture
    /// that hand-assembles YAML can construct a record the writer would refuse,
    /// and a proof tested only against impossible records has been tested
    /// against nothing a user can produce.
    pub fn screen_record(
        &self,
        id: &str,
        route: &str,
        columns: u32,
        regions: &[&str],
        node_id: Option<&str>,
    ) -> ScreenId {
        let id = ScreenId::parse(id).unwrap();
        let record = ScreenRecord {
            id: id.clone(),
            route: route.into(),
            status: Status::Registered,
            contract_version: 1,
            frame: node_id.map(|node_id| FigmaFrame {
                file_key: "KEY".into(),
                node_id: node_id.into(),
                captured_at: Timestamp::fixed(2026, 7, 30, 10, 0, 0),
            }),
            arrangement: ArrangementContract {
                columns,
                regions: regions.iter().map(|r| (*r).to_owned()).collect(),
            },
            basis: vec!["ACT-VDS-001:s5a".into()],
            notes: None,
        };
        let store = self.store();
        let path = store.screen_path(&id);
        if path.exists() {
            store.replace(&path, &record).unwrap();
        } else {
            store.create(&path, &record).unwrap();
        }
        id
    }

    pub fn amend_screen(&self, id: &str, edit: impl FnOnce(&mut ScreenRecord)) {
        let id = ScreenId::parse(id).unwrap();
        let store = self.store();
        let mut record = store.read_screen(&id).unwrap().value;
        edit(&mut record);
        store.replace(&store.screen_path(&id), &record).unwrap();
    }

    // -- the frame ledger ----------------------------------------------------

    /// One screen frame, drawn as `columns` disjoint panes inside a `body`.
    ///
    /// The geometry is written out rather than abstracted because the column
    /// derivation is geometric: a fixture that produced the count directly
    /// would test the proof against a number nothing measured.
    ///
    /// Each pane carries a child, and that is not decoration. The capture depth
    /// is DERIVED from the deepest chain present, so a pane drawn as a leaf
    /// would sit exactly on the boundary and every single-pane fixture would
    /// read as truncated. A real pane has content in it; a fixture whose shape
    /// no real capture has is a fixture that tests the wrong thing.
    pub fn frame(
        node_id: &str,
        name: &str,
        regions: &[&str],
        columns: u32,
    ) -> (String, serde_json::Value) {
        let mut children = Vec::new();
        for region in regions {
            let panes: Vec<serde_json::Value> = if *region == "body" {
                (0..columns)
                    .map(|i| {
                        Self::node(
                            "pane",
                            f64::from(i) * 400.0,
                            360.0,
                            700.0,
                            serde_json::json!([Self::node(
                                "content",
                                f64::from(i) * 400.0,
                                340.0,
                                600.0,
                                serde_json::json!([])
                            )]),
                        )
                    })
                    .collect()
            } else {
                vec![Self::node(
                    "chrome",
                    0.0,
                    200.0,
                    40.0,
                    serde_json::json!([]),
                )]
            };
            children.push(Self::node(
                region,
                0.0,
                1440.0,
                860.0,
                serde_json::json!(panes),
            ));
        }
        (
            node_id.to_owned(),
            Self::node(name, 0.0, 1440.0, 900.0, serde_json::json!(children)),
        )
    }

    /// A frame whose content band is CHILDLESS AT the capture boundary.
    ///
    /// The prior art's own case: a `children: []` that means "nothing fetched
    /// this", recorded by the ledger as "draws nothing here". The two are the
    /// same bytes and only the depth asked for knows the difference.
    pub fn boundary_frame(node_id: &str, name: &str) -> (String, serde_json::Value) {
        (
            node_id.to_owned(),
            Self::node(
                name,
                0.0,
                1440.0,
                900.0,
                serde_json::json!([Self::node(
                    "body",
                    0.0,
                    1440.0,
                    860.0,
                    serde_json::json!([Self::node(
                        "hub",
                        0.0,
                        1440.0,
                        800.0,
                        serde_json::json!([])
                    )])
                )]),
            ),
        )
    }

    fn node(
        name: &str,
        x: f64,
        width: f64,
        height: f64,
        children: serde_json::Value,
    ) -> serde_json::Value {
        serde_json::json!({
            "id": "0:0",
            "name": name,
            "type": "FRAME",
            "absoluteBoundingBox": {"x": x, "y": 0, "width": width, "height": height},
            "children": children,
        })
    }

    /// Generate the frame ledger from these frames.
    pub fn frames(&self, frames: &[(String, serde_json::Value)]) {
        let mut nodes = serde_json::Map::new();
        for (node_id, document) in frames {
            nodes.insert(
                node_id.clone(),
                serde_json::json!({"document": document.clone()}),
            );
        }
        self.write_frames_capture(&serde_json::Value::Object(nodes));
    }

    /// Generate the frame ledger from a raw `nodes` map, for the tests whose
    /// subject is the capture itself.
    pub fn write_frames_capture(&self, nodes: &serde_json::Value) {
        let project = self.project();
        let body = serde_json::json!({"nodes": nodes}).to_string();
        let ledger = vds_figma::frames::build_ledger(
            "KEY",
            &[body],
            &project.config.screens,
            "a test capture",
        )
        .expect("the frame ledger generator");
        vds_figma::frames::write(&project, &ledger).expect("the frame ledger writer");
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
