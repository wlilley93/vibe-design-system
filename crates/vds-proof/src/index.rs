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
//! A collision is therefore a fail-closed error naming both files (VDS S-4(4)),
//! with ONE exception: where a record DECLARES itself `supersededBy` the other,
//! the pair is a recorded succession rather than an accident. The successor owns
//! the lookup slot so a reference resolves to the component that is actually
//! there, and the superseded record stays reachable by id. A retired record with
//! no successor still owns its own slot, so S-9(8) fires against it as before.
//!
//! That exception is not a softening. Without it the error's advice could not be
//! followed: it said "Retire one", and retirement changed nothing, because a
//! retired record still claimed its coordinate. A project hitting this had no
//! move that worked, and on Opbox it left the contrast, states and parity proofs
//! stopped at a precondition, proving nothing, while the register looked tidy.
//! An unfollowable instruction is how a gate stops being enforcement and starts
//! being an obstacle people route around.

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
                if let Some(previous) = index.by_code.get(&coordinate).copied() {
                    let prior = &index.records[previous].value;
                    let current = &record.value;
                    // A DECLARED supersession is not a collision. The rule this
                    // guard enforces is that nothing is hidden by ACCIDENT; when one
                    // record names the other as its successor, the relationship is
                    // written down, checkable, and reachable by id, which is the
                    // opposite of the silent overwrite the guard exists to stop.
                    //
                    // Without this limb the error's own advice was unfollowable. It
                    // said "Retire one", and retiring changed nothing, because a
                    // retired record still entered this map and still collided. On
                    // Opbox that left contrast, states and parity switched off at a
                    // precondition for as long as both records existed, so three
                    // gates proved nothing while the register looked tidy.
                    let declared = current.superseded_by.as_ref() == Some(&prior.id)
                        || prior.superseded_by.as_ref() == Some(&current.id);
                    if !declared {
                        return Err(VdsError::Artefact {
                            path: store.project.rel(&record.path),
                            message: format!(
                                "claims the code coordinate {}::{}, which {} also claims. Two \
                                 records on one coordinate mean a lookup finds one of them and \
                                 never the other, so a rule that should fire against the hidden \
                                 record silently never does (VDS S-4(4)). Declare one \
                                 `supersededBy` the other, or give them distinct export names. \
                                 Retiring alone does NOT clear this: a retired record still \
                                 claims its coordinate, by design, so VDS S-9(8) can still fire \
                                 against a reference to it.",
                                code.import_path,
                                code.export_name,
                                store.project.rel(&index.records[previous].path)
                            ),
                        });
                    }
                    // The SUCCESSOR owns the slot, so a reference to this coordinate
                    // resolves to the component that is actually there. The
                    // superseded record stays reachable by id, and a retired record
                    // with NO successor still owns its own slot, so VDS S-9(8) fires
                    // against it exactly as before.
                    if current.superseded_by.is_none() {
                        index.by_code.insert(coordinate.clone(), position);
                    }
                } else {
                    index.by_code.insert(coordinate.clone(), position);
                }
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

    /// RETIRING ALONE MUST NOT CLEAR A COLLISION, and the error must say so.
    ///
    /// The test above already proves a retired duplicate is refused. This one
    /// pins the ADVICE, because the advice was the defect: it read "Retire one",
    /// a reader did exactly that, and nothing changed. An instruction a project
    /// cannot follow turns a gate into an obstacle to route around.
    #[test]
    fn the_collision_error_does_not_advise_retiring_alone() {
        let h = Harness::new();
        h.register("Button", Status::Registered);
        h.register_as("CMP-0002", "Button", "Button", Status::Retired);
        let store = h.store();
        let error = RegisterIndex::build(&store).unwrap_err().to_string();
        assert!(error.contains("supersededBy"), "{error}");
        assert!(error.contains("Retiring alone does NOT clear this"), "{error}");
    }

    /// A DECLARED succession is not a collision, and the SUCCESSOR owns the slot.
    #[test]
    fn a_declared_supersession_resolves_to_the_successor() {
        let h = Harness::new();
        let successor = h.register("Button", Status::Registered);
        let superseded = h.register_as("CMP-0002", "Button", "Button", Status::Retired);
        h.amend(&superseded, |r| r.superseded_by = Some(successor.clone()));

        let store = h.store();
        let index = RegisterIndex::build(&store).unwrap();
        let found = index
            .lookup("@/components/ui", "Button")
            .expect("the coordinate still resolves");
        // The live record wins, so a reference resolves to what is actually
        // there rather than to the record that was superseded.
        assert_eq!(found.id, successor);
        // And the superseded record is still in the register, reachable by id,
        // so nothing was hidden - which is the whole point of the guard.
        assert!(index.by_id(&superseded).is_some());
        assert_eq!(index.len(), 2);
    }

    /// The half that keeps VDS S-9(8) alive: a retired record with NO successor
    /// still owns its coordinate, so a reference to it is still found and the
    /// composition R3 rule still fires. If this ever goes green by accident the
    /// retirement rule has been silently switched off.
    #[test]
    fn a_retired_record_with_no_successor_still_owns_its_coordinate() {
        let h = Harness::new();
        let retired = h.register_as("CMP-0007", "Legacy", "Legacy", Status::Retired);
        let store = h.store();
        let index = RegisterIndex::build(&store).unwrap();
        let found = index
            .lookup("@/components/ui", "Legacy")
            .expect("a retired record must stay findable");
        assert_eq!(found.id, retired);
        assert_eq!(found.status, Status::Retired);
    }
}
