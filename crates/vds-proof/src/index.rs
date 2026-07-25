//! An index over the register, for the lookups every proof needs.
//!
//! One rule here is load-bearing: **two records may not claim the same code
//! coordinate**. The earlier implementation kept a map from
//! `(import path, export name)` to record and let a later insert overwrite an
//! earlier one. The consequence was not a wrong lookup but a MISSING check: two
//! records claiming `@/components/ui::Button`, one registered and one retired,
//! left only the last-inserted visible, so the retired record was never found
//! and the VDS S-9(8) rule never fired against it.
//!
//! A collision is therefore a fail-closed error naming both files (VDS S-4(4)).

use std::collections::BTreeMap;
use std::path::PathBuf;

use vds_core::{ComponentId, ComponentRecord, Result, VdsError};
use vds_store::{Located, Store};

#[derive(Debug)]
pub struct RegisterIndex {
    records: Vec<Located<ComponentRecord>>,
    by_id: BTreeMap<String, usize>,
    by_code: BTreeMap<(String, String), usize>,
    by_export: BTreeMap<String, Vec<usize>>,
    by_import: BTreeMap<String, Vec<usize>>,
}

impl RegisterIndex {
    pub fn build(store: &Store) -> Result<RegisterIndex> {
        let records = store.read_register()?;
        let mut index = RegisterIndex {
            by_id: BTreeMap::new(),
            by_code: BTreeMap::new(),
            by_export: BTreeMap::new(),
            by_import: BTreeMap::new(),
            records,
        };

        for (position, record) in index.records.iter().enumerate() {
            let id = record.value.id.to_string();
            if let Some(previous) = index.by_id.get(&id) {
                return Err(VdsError::Artefact {
                    path: store.project.rel(&record.path),
                    message: format!(
                        "duplicate register id {id}, also in {}. An identifier collision is a \
                         fail-closed error, never a silent overwrite (VDS S-4(4)).",
                        store.project.rel(&index.records[*previous].path)
                    ),
                });
            }
            index.by_id.insert(id, position);

            if let Some(code) = &record.value.code {
                let coordinate = (code.import_path.clone(), code.export_name.clone());
                if let Some(previous) = index.by_code.get(&coordinate) {
                    return Err(VdsError::Artefact {
                        path: store.project.rel(&record.path),
                        message: format!(
                            "claims the code coordinate {}::{}, which {} also claims. Two \
                             records on one coordinate mean a lookup finds one of them and \
                             never the other, so a rule that should fire against the hidden \
                             record silently never does (VDS S-4(4)). Retire one, or give \
                             them distinct export names.",
                            code.import_path,
                            code.export_name,
                            store.project.rel(&index.records[*previous].path)
                        ),
                    });
                }
                index.by_code.insert(coordinate, position);
                index
                    .by_export
                    .entry(code.export_name.clone())
                    .or_default()
                    .push(position);
                index
                    .by_import
                    .entry(code.import_path.clone())
                    .or_default()
                    .push(position);
            }
        }
        Ok(index)
    }

    pub fn records(&self) -> &[Located<ComponentRecord>] {
        &self.records
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn lookup(&self, import_path: &str, export_name: &str) -> Option<&ComponentRecord> {
        self.by_code
            .get(&(import_path.to_owned(), export_name.to_owned()))
            .map(|position| &self.records[*position].value)
    }

    pub fn by_id(&self, id: &ComponentId) -> Option<&ComponentRecord> {
        self.by_id
            .get(id.as_str())
            .map(|position| &self.records[*position].value)
    }

    pub fn path_of(&self, id: &ComponentId) -> Option<&PathBuf> {
        self.by_id
            .get(id.as_str())
            .map(|position| &self.records[*position].path)
    }

    /// Why a lookup missed, in terms a reader can act on.
    ///
    /// "No such record" is true and useless when the real cause is a typo in one
    /// half of the coordinate. These lines name the record that nearly matched
    /// and say which half is wrong.
    pub fn near_misses(&self, import_path: &str, export_name: &str) -> Vec<String> {
        let mut out = Vec::new();
        for position in self.by_export.get(export_name).into_iter().flatten() {
            let record = &self.records[*position].value;
            if let Some(code) = &record.code {
                out.push(format!(
                    "{} exports {export_name:?} but from {:?}",
                    record.id, code.import_path
                ));
            }
        }
        for position in self.by_import.get(import_path).into_iter().flatten() {
            let record = &self.records[*position].value;
            if let Some(code) = &record.code {
                out.push(format!(
                    "{} is at {import_path:?} but exports {:?}",
                    record.id, code.export_name
                ));
            }
        }
        out.sort();
        out.dedup();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Harness;
    use vds_core::Status;

    #[test]
    fn lookup_finds_a_record_by_its_code_coordinate() {
        let h = Harness::new();
        h.register("Button", Status::Registered);
        let store = h.store();
        let index = RegisterIndex::build(&store).unwrap();
        assert!(index.lookup("@/components/ui", "Button").is_some());
        assert!(index.lookup("@/components/ui", "Card").is_none());
    }

    /// The defect this type exists to prevent: an overwritten coordinate hides
    /// a record, so the rule that should fire against it never does.
    #[test]
    fn two_records_on_one_code_coordinate_are_refused() {
        let h = Harness::new();
        h.register("Button", Status::Registered);
        h.register_as("CMP-0002", "Button", "Button", Status::Retired);
        let store = h.store();
        let error = RegisterIndex::build(&store).unwrap_err();
        assert!(error.to_string().contains("also claims"), "{error}");
        assert!(error.to_string().contains("CMP-0001"), "{error}");
    }

    #[test]
    fn a_near_miss_says_which_half_of_the_coordinate_is_wrong() {
        let h = Harness::new();
        h.register("Button", Status::Registered);
        let store = h.store();
        let index = RegisterIndex::build(&store).unwrap();

        let wrong_module = index.near_misses("@/legacy/ui", "Button");
        assert_eq!(wrong_module.len(), 1);
        assert!(wrong_module[0].contains("but from"), "{wrong_module:?}");

        let wrong_export = index.near_misses("@/components/ui", "Buton");
        assert_eq!(wrong_export.len(), 1);
        assert!(wrong_export[0].contains("but exports"), "{wrong_export:?}");
    }

    #[test]
    fn an_unbuilt_record_claims_no_coordinate() {
        let h = Harness::new();
        h.register_unbuilt("Sketch", Status::Designed);
        let store = h.store();
        let index = RegisterIndex::build(&store).unwrap();
        assert_eq!(index.len(), 1);
        assert!(index.lookup("@/components/ui", "Sketch").is_none());
    }

    #[test]
    fn two_unbuilt_records_do_not_collide() {
        let h = Harness::new();
        h.register_unbuilt("SketchA", Status::Designed);
        h.register_unbuilt("SketchB", Status::Designed);
        let store = h.store();
        assert_eq!(RegisterIndex::build(&store).unwrap().len(), 2);
    }
}
