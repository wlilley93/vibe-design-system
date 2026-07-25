//! Fixtures for the Figma seam's tests.

use std::collections::BTreeMap;
use std::path::PathBuf;

use vds_core::{
    Accessibility, CodeCounterpart, ComponentId, ComponentRecord, ContrastFloor, Demand,
    FloorScope, NameSource, Project, State, StateContract, Status, Timestamp, default_config,
};
use vds_store::Store;

use crate::ledger::{FigmaLedger, FigmaNodeRow, GENERATOR_COMMAND, LEDGER_SCHEMA_VERSION};

pub struct Fixture {
    #[allow(dead_code)]
    tmp: tempfile::TempDir,
    project: Project,
}

impl Default for Fixture {
    fn default() -> Self {
        Self::new()
    }
}

impl Fixture {
    pub fn new() -> Fixture {
        let tmp = tempfile::tempdir().unwrap();
        for dir in ["register", "warrants", "proofs", "pins", "ledgers"] {
            std::fs::create_dir_all(tmp.path().join(".vds").join(dir)).unwrap();
        }
        std::fs::write(
            tmp.path().join(".vds/config.toml"),
            default_config("demo", "DEMO"),
        )
        .unwrap();
        let project = Project::discover(Some(tmp.path())).unwrap();
        Fixture { project, tmp }
    }

    pub fn root(&self) -> PathBuf {
        self.project.root.clone()
    }

    pub fn store(&self) -> Store<'_> {
        Store::new(&self.project)
    }

    pub fn register(&self, name: &str, status: Status) -> ComponentId {
        let store = self.store();
        let id = ComponentId::allocate(&store.register_dir()).unwrap();
        let record = ComponentRecord {
            id: id.clone(),
            name: name.into(),
            status,
            contract_version: 1,
            figma: None,
            code: Some(CodeCounterpart {
                import_path: "@/components/ui".into(),
                source_file: format!("src/components/ui/{}.tsx", name.to_lowercase()),
                export_name: name.into(),
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
                measured_by: "fixture".into(),
            },
            supersedes: vec![],
            superseded_by: None,
            amendments: vec![],
            basis: vec!["ACT-VDS-001:s5".into()],
            deprecated_at: None,
            retired_at: None,
            retirement_proof_id: None,
            notes: None,
        };
        store.create(&store.record_path(&id), &record).unwrap();
        id
    }

    pub fn amend(&self, id: &ComponentId, edit: impl FnOnce(&mut ComponentRecord)) {
        let store = self.store();
        let mut record = store.read_record(id).unwrap().value;
        edit(&mut record);
        store.replace(&store.record_path(id), &record).unwrap();
    }
}

/// A Figma ledger asserting that each component draws the given states.
pub fn figma_ledger(rows: &[(&ComponentId, Vec<State>)]) -> FigmaLedger {
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
            .map(|(id, states)| FigmaNodeRow {
                component_id: (*id).clone(),
                node_id: "12:34".into(),
                resolved: true,
                figma_name: Some("Node".into()),
                is_component_set: true,
                variant_properties: BTreeMap::new(),
                states_drawn: states.clone(),
                unresolved_because: None,
            })
            .collect(),
        unclaimed: vec![],
        notes: vec![],
    };
    ledger.content_digest = ledger.compute_content_digest().unwrap();
    ledger
}

/// Whether `text` contains something that is a design REALISATION rather than a
/// requirement (VDS S-2(4)).
///
/// Deliberately token-aware rather than a substring scan. The naive version of
/// this check fired on the word "requirement" because it contains "rem", which
/// is the exact false-positive class that gets a gate disabled, and a disabled
/// gate enforces nothing at all.
#[cfg(test)]
pub fn realisation_in(text: &str) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();

    // A CSS length or duration: digits, then a unit, then a boundary.
    const UNITS: [&str; 10] = ["px", "rem", "em", "vh", "vw", "pt", "ch", "ex", "ms", "s"];
    let mut i = 0;
    while i < chars.len() {
        if !chars[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        // A digit preceded by an identifier character is part of a name.
        if start > 0 && (chars[start - 1].is_ascii_alphanumeric() || chars[start - 1] == '-') {
            while i < chars.len() && chars[i].is_ascii_alphanumeric() {
                i += 1;
            }
            continue;
        }
        while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
            i += 1;
        }
        let number_end = i;
        for unit in UNITS {
            let unit_chars: Vec<char> = unit.chars().collect();
            if chars[number_end..].starts_with(&unit_chars[..]) {
                let after = number_end + unit_chars.len();
                let bounded = after >= chars.len() || !chars[after].is_ascii_alphanumeric();
                if bounded {
                    let matched: String = chars[start..after].iter().collect();
                    return Some(matched);
                }
            }
        }
    }

    // A hex colour: '#' then exactly 3, 6 or 8 hex digits, bounded.
    let bytes: Vec<char> = chars;
    for (index, ch) in bytes.iter().enumerate() {
        if *ch != '#' {
            continue;
        }
        let mut run = 0;
        while index + 1 + run < bytes.len() && bytes[index + 1 + run].is_ascii_hexdigit() {
            run += 1;
        }
        let after = index + 1 + run;
        let bounded = after >= bytes.len() || !bytes[after].is_ascii_alphanumeric();
        if bounded && matches!(run, 3 | 6 | 8) {
            let matched: String = bytes[index..after].iter().collect();
            return Some(matched);
        }
    }

    for function in [
        "rgb(",
        "rgba(",
        "hsl(",
        "hsla(",
        "oklch(",
        "lab(",
        "cubic-bezier(",
    ] {
        if text.contains(function) {
            return Some(function.to_owned());
        }
    }
    None
}
