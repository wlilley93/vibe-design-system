//! The screen record: one route's ARRANGEMENT requirement.
//!
//! # The gap this closes
//!
//! Every one of the ten proof kinds VDS shipped before this one reads a
//! COMPONENT. `register_completeness` and `composition` say the word "screen"
//! (VDS S-7(5)), but what they read is a screen's REFERENCES: the ledger at
//! `crates/vds-scan/src/lib.rs:88` records a route, a digest and a list of
//! `Reference` rows, and a reference is a component name, an import path and a
//! line number. So a page may render every registered component, each in a
//! registered state, in an arrangement its frame does not draw, and all ten
//! kinds stay green. Parity is a claim about SCREENS and there was no screen to
//! make it about.
//!
//! # Why this holds a COUNT and never a WIDTH
//!
//! VDS S-2(4) admits a REQUIREMENT and refuses a REALISATION, and a width is a
//! length. The prior art this is derived from records column widths (a Figma
//! frame's `[924, 420]`), which is exactly the form VDS may not store: a record
//! under `.vds/**` carrying `924px` is a design value recovered by reading it,
//! and `no_stored_values` R3 (`crates/vds-proof/src/no_stored_values.rs:87`)
//! would fail on the record forever. A COUNT of side-by-side content columns is
//! a structural fact about the arrangement, in the same shape VDS S-2(6) settles
//! for a contrast ratio: a numeral is not automatically a value. Deleting every
//! screen record loses no shipped pixel, and no reader can move one by editing
//! a column count.
//!
//! # The one register both sides must use
//!
//! A screen with no split still has ONE column. The prior art scored a route
//! `frame=0 code=1` and called agreement a deviation, because the ledger wrote
//! "no side-by-side columns" as an empty list and the code side counted panes.
//! [`ArrangementContract::columns`] is therefore a count of PANES and never a
//! count of SPLITS: the minimum a real screen can require is 1, and 0 is the
//! value this type refuses to let mean anything.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::Status;
use crate::ids::ScreenId;
use crate::timestamp::Timestamp;

/// One registered screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ScreenRecord {
    pub id: ScreenId,
    /// What the screen IS, in the project's own vocabulary: a route, a path, a
    /// name. Compared to nothing by VDS and printed in every finding, because a
    /// finding that names only `SCR-0007` makes the reader go looking.
    pub route: String,
    pub status: Status,
    /// Bumped on every amendment, so a contract change is a versioned event and
    /// never a silent edit (VDS S-9(2)).
    pub contract_version: u32,
    /// The frame in the decided-target Figma file that DRAWS this screen. Null
    /// while the screen is proposed and nothing has been drawn; a proof reports
    /// the absence rather than assuming it away.
    pub frame: Option<FigmaFrame>,
    pub arrangement: ArrangementContract,
    /// The authorities this registration rests on.
    pub basis: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// The frame a screen record is drawn by.
///
/// Deliberately NOT [`super::FigmaNode`], even though the fields coincide today.
/// A node is a component set and carries variants; a frame is a screen drawing
/// and carries an authority layer. Sharing the type would make a screen record
/// and a component record interchangeable at the seam where they are read, and
/// the first field either one gains that the other must not have would then be
/// an amendment to both.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct FigmaFrame {
    pub file_key: String,
    /// A Figma node id, `<digits>:<digits>` in a file URL and
    /// `<digits>-<digits>` in a deep link. Both spellings are accepted because
    /// both are what a designer actually copies.
    pub node_id: String,
    pub captured_at: Timestamp,
}

/// What arrangement this screen requires. A requirement, never a realisation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ArrangementContract {
    /// How many side-by-side content PANES the screen requires, left to right.
    ///
    /// A count, not a width. See the module note: a width is a realisation and
    /// has nowhere to live here.
    pub columns: u32,
    /// The named regions the screen requires, in the project's own vocabulary
    /// (`rail`, `cmdbar`, `body`, `statusbar` in the subject this was derived
    /// from).
    ///
    /// Free strings and not an enum, because the region vocabulary is the
    /// SUBJECT's and not VDS's. A closed set here would be VDS deciding what a
    /// screen is made of, which is the fourth authority [2026] VJS-CC-OPBOX 3
    /// forbids. The generator is told which names to look for through
    /// `[screens] region_names` in the project config, so the two halves read
    /// one list.
    #[serde(default)]
    pub regions: Vec<String>,
}

/// The largest column count this type will admit.
///
/// Not a taste judgement and not a bound on design. It is the point past which
/// a REQUIREMENT stops being a requirement about an arrangement: the derivation
/// that measures the other side clusters a frame's children by x-interval
/// (`crates/vds-figma/src/frames.rs`), so a count in the hundreds is not a
/// screen anybody drew, it is a record that no frame can ever satisfy. The twin
/// defect of a check that cannot fail is a check that cannot pass, and that one
/// is worse, because it is the one people route around.
pub const MAX_COLUMNS: u32 = 32;

impl ArrangementContract {
    /// Why no arrangement could ever fail this contract, or `None`.
    ///
    /// The shape is deliberately [`super::ProofKind::unimplemented_because`]'s:
    /// a reason is a sentence, and the caller decides what to do with it. The
    /// contrast proof's R7 settles the same question for a contrast floor
    /// (`crates/vds-proof/src/contrast.rs:155`): a floor at or below 1.0 is met
    /// by every pair of colours, so an enforced row that carries one cannot
    /// fail, and a record was once "fixed" by writing exactly that. A column
    /// count of 0 is the same move: every arrangement has at least one pane, so
    /// nothing can be below zero of them.
    pub fn unfalsifiable_because(&self) -> Option<String> {
        (self.columns == 0).then(|| {
            "columns is 0. Every arrangement draws at least one content pane, so no screen can \
             render fewer than zero of them and this requirement cannot be failed by anything. \
             A screen with no split still has ONE column; 0 is not the way to say \"single \
             pane\", 1 is (VDS S-7(2)(4))."
                .to_owned()
        })
    }

    /// Why no arrangement could ever satisfy this contract, or `None`.
    ///
    /// The twin of [`Self::unfalsifiable_because`], and the one that gets
    /// skipped. A gate that is permanently red teaches people to reach for the
    /// escape hatch, and then the twenty checks behind it go off too.
    pub fn unsatisfiable_because(&self) -> Option<String> {
        (self.columns > MAX_COLUMNS).then(|| {
            format!(
                "columns is {}, above the {MAX_COLUMNS} this contract admits. Nothing measured \
                 from a frame will ever reach it, so the row would be permanently red, which is \
                 how a gate teaches people to switch it off.",
                self.columns
            )
        })
    }

    /// Why this contract states nothing to check, or `None`.
    ///
    /// Both directions in one call, for the proof that has to decide whether a
    /// row is enforceable at all.
    pub fn unenforceable_because(&self) -> Option<String> {
        self.unfalsifiable_because()
            .or_else(|| self.unsatisfiable_because())
    }
}

impl ScreenRecord {
    /// The regions this screen requires that `drawn` does not contain, in the
    /// order the contract declares them.
    pub fn required_regions_missing_from(&self, drawn: &[String]) -> Vec<String> {
        self.arrangement
            .regions
            .iter()
            .filter(|required| !drawn.iter().any(|d| d == *required))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract(columns: u32) -> ArrangementContract {
        ArrangementContract {
            columns,
            regions: vec![],
        }
    }

    /// The R7 move, transplanted: a record "fixed" into something no
    /// measurement can fail.
    #[test]
    fn a_zero_column_contract_says_why_nothing_can_fail_it() {
        let reason = contract(0).unfalsifiable_because().expect("a reason");
        assert!(reason.contains("cannot be failed"), "{reason}");
        assert!(
            reason.contains("1 is"),
            "a refusal that does not say what to write instead is a wall: {reason}"
        );
        assert!(contract(1).unfalsifiable_because().is_none());
        assert!(contract(3).unfalsifiable_because().is_none());
    }

    #[test]
    fn a_contract_above_the_ceiling_says_why_nothing_can_satisfy_it() {
        assert!(contract(MAX_COLUMNS).unsatisfiable_because().is_none());
        let reason = contract(MAX_COLUMNS + 1)
            .unsatisfiable_because()
            .expect("a reason");
        assert!(reason.contains("permanently red"), "{reason}");
    }

    #[test]
    fn both_ends_are_reported_by_the_one_call_a_proof_makes() {
        assert!(contract(0).unenforceable_because().is_some());
        assert!(contract(9999).unenforceable_because().is_some());
        assert!(contract(2).unenforceable_because().is_none());
    }

    #[test]
    fn a_missing_region_is_reported_in_the_order_the_contract_declares() {
        let record = ScreenRecord {
            id: ScreenId::parse("SCR-0001").unwrap(),
            route: "/matters".into(),
            status: Status::Registered,
            contract_version: 1,
            frame: None,
            arrangement: ArrangementContract {
                columns: 2,
                regions: vec!["rail".into(), "cmdbar".into(), "body".into()],
            },
            basis: vec![],
            notes: None,
        };
        assert_eq!(
            record.required_regions_missing_from(&["body".to_owned()]),
            vec!["rail".to_string(), "cmdbar".to_string()]
        );
        assert!(
            record
                .required_regions_missing_from(&[
                    "body".to_owned(),
                    "cmdbar".to_owned(),
                    "rail".to_owned()
                ])
                .is_empty(),
            "the contract declares a SET of regions, not an order of them"
        );
    }

    /// VDS S-2(4). There is no field here a width could live in, and this test
    /// holds the serialised form to that rather than the paragraph above.
    #[test]
    fn a_serialised_screen_record_names_no_realisation() {
        let record = ScreenRecord {
            id: ScreenId::parse("SCR-0001").unwrap(),
            route: "/dashboards".into(),
            status: Status::Registered,
            contract_version: 1,
            frame: Some(FigmaFrame {
                file_key: "KEY".into(),
                node_id: "669:2".into(),
                captured_at: Timestamp::fixed(2026, 7, 30, 10, 0, 0),
            }),
            arrangement: ArrangementContract {
                columns: 2,
                regions: vec!["rail".into(), "body".into()],
            },
            basis: vec!["ACT-VDS-001:s5a".into()],
            notes: None,
        };
        let text = serde_yaml::to_string(&record).unwrap();
        for forbidden in ["px", "rem", "width", "height", "colour", "color", "#"] {
            assert!(
                !text.contains(forbidden),
                "the screen record serialises {forbidden:?}, which is a realisation and belongs \
                 in the Figma file, not in .vds/ (VDS S-2(4)): {text}"
            );
        }
    }
}
