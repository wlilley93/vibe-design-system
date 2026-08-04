//! O10 of [2026] VJS-FI-VDS 2: the two `SignOff` shapes must become one type, and a
//! test must assert that the committed row files deserialise under it.
//!
//! WHY THIS FILE EXISTS. Until 2026-08-04 this estate carried two incompatible
//! `SignOff` types on two branches. Every one of the 19 rows in the live register
//! carries `evidence`, so under the seven-field shape - the one on `master` - the
//! whole register was a parse failure, and `vds signoff list` reported "19 rows" or
//! refused depending only on which branch the operator had built. A count over a
//! record is a statement about the reader, and the estate could not say which reader
//! it meant.
//!
//! The rows here are VENDORED, not read from the live register, because the live
//! register lives in a different repository (`opbox-frontend`, at
//! `.vds/signoffs/`) which a test in this crate cannot reach. A fixture that reaches
//! for a path outside the repo passes vacuously on any machine where the path is
//! absent, which is the failure mode this test is meant to prevent.

use std::collections::BTreeSet;
use std::path::PathBuf;

use vds_core::SignOff;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/live-register-2026-08-03")
}

/// The exact file set, named. A bare count would let a 20th row be added while the
/// assertion still read `== 19` against a table that never mentions it, and a
/// deletion plus an addition would cancel out entirely.
fn expected_ids() -> BTreeSet<String> {
    (1..=19).map(|n| format!("SGN-{n:04}.yaml")).collect()
}

#[test]
fn every_committed_row_of_the_live_register_deserialises() {
    let dir = fixture_dir();
    let found: BTreeSet<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("the vendored register is missing at {}: {e}", dir.display()))
        .map(|e| {
            e.expect("dir entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();

    assert_eq!(
        found,
        expected_ids(),
        "the vendored register drifted. This fixture is a snapshot of what the estate's \
         register actually held on 2026-08-03; if a row is added or removed here, say so \
         deliberately and update expected_ids(), because a silent change makes the test \
         assert something nobody chose."
    );

    for name in &found {
        let text = std::fs::read_to_string(dir.join(name)).expect("read");
        let row: SignOff = serde_yaml::from_str(&text).unwrap_or_else(|e| {
            panic!("{name} does not deserialise under the reconciled SignOff: {e}")
        });
        // Every one of these carries evidence and no basis. That is the whole point:
        // `evidence` is why the seven-field shape refused them, and the absent `basis`
        // is what O5 reports as coverage owed rather than rounding to zero.
        assert!(row.evidence.is_some(), "{name} lost its evidence binding");
        assert!(
            row.basis.is_none(),
            "{name} carries a basis, but these rows predate limb (b). If that changed \
             deliberately, this fixture needs re-snapshotting."
        );
    }
}

/// NEGATIVE CONTROL. The reconciled type must still be closed. Without this, widening
/// `SignOff` to make the rows parse would look identical to reconciling it, and the
/// test above would pass just as happily against a type that accepts anything.
#[test]
fn the_reconciled_shape_is_still_closed_to_unknown_fields() {
    let text = std::fs::read_to_string(fixture_dir().join("SGN-0001.yaml")).expect("read");
    let tampered = format!("{text}\nsmuggled: yes\n");
    let error = serde_yaml::from_str::<SignOff>(&tampered)
        .expect_err("deny_unknown_fields must still refuse an unknown key")
        .to_string();
    assert!(error.contains("smuggled"), "{error}");
}
