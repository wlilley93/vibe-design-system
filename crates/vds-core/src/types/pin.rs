//! Pins: a derived one-way agreement assertion between two named records.
//!
//! VDS S-2(7) is the whole design of this type. Where a proof must compare two
//! values, it compares DIGESTS of the normalised values, not the values. A pin
//! row therefore carries `source_value_digest` and `target_value_digest` and an
//! agreement flag, and never the two strings.
//!
//! There is no field on [`PinRow`] that can hold a colour, a length, a font, a
//! duration or an easing curve. That absence is what keeps a pin a gate rather
//! than a store, and it is why deleting `.vds/pins/` loses no design value:
//! every row is recomputable from the named records by the command in
//! `generated_by`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::digest::Digest;
use crate::ids::{PinId, ProofId};
use crate::timestamp::Timestamp;

/// A named system of record. VDS S-2(3) fixes the set, and it is not VDS's to
/// move: relocating a source of truth is a referral, not an implementation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecordOfTruth {
    /// What the record is the system of record FOR, e.g. "what ships".
    pub authority_for: String,
    /// How to reach it: a repository-relative path, or a Figma file key.
    pub locator: String,
    /// The digest of the record as read when the pin was generated.
    pub digest: Digest,
}

/// One compared row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PinRow {
    /// The name of the thing compared. A NAME, never a value.
    pub name: String,
    pub source_value_digest: Digest,
    pub target_value_digest: Digest,
    pub agrees: bool,
    /// Why this row was not enforced, where it was not. A row excluded with no
    /// reason is a row nobody can audit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_enforced_because: Option<String>,
}

impl PinRow {
    pub fn compare(name: impl Into<String>, source: &str, target: &str) -> Self {
        let source_digest = Digest::of_text(source.trim());
        let target_digest = Digest::of_text(target.trim());
        Self {
            name: name.into(),
            agrees: source_digest == target_digest,
            source_value_digest: source_digest,
            target_value_digest: target_digest,
            not_enforced_because: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PinDirection {
    /// The only lawful direction. A pin derives one way, from the named records,
    /// and never feeds a value back into either.
    #[default]
    OneWayDerived,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Pin {
    pub id: PinId,
    pub subject: String,
    pub direction: PinDirection,
    pub source_of_record: RecordOfTruth,
    pub target_of_record: RecordOfTruth,
    pub rows: Vec<PinRow>,
    pub rows_considered: u64,
    pub rows_enforced: u64,
    /// Always true. A pin that does not fail closed is a third opinion.
    pub fails_closed: bool,
    pub generated_at: Timestamp,
    /// The exact command that regenerates this pin byte for byte (VDS S-2(5)(4)).
    pub generated_by: String,
    pub digest: Digest,
    #[serde(default)]
    pub proof_id: Option<ProofId>,
}

impl Pin {
    pub fn disagreements(&self) -> impl Iterator<Item = &PinRow> {
        self.rows
            .iter()
            .filter(|r| !r.agrees && r.not_enforced_because.is_none())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_compares_digests_and_keeps_neither_value() {
        let row = PinRow::compare("control-border", "#ebebeb", "#ebebeb");
        assert!(row.agrees);
        let serialised = serde_json::to_string(&row).unwrap();
        assert!(
            !serialised.contains("ebebeb"),
            "a pin row must not carry the value it compared (VDS S-2(7)): {serialised}"
        );
    }

    #[test]
    fn a_row_disagrees_when_the_two_records_disagree() {
        assert!(!PinRow::compare("x", "#ebebeb", "#ececec").agrees);
    }

    #[test]
    fn comparison_normalises_surrounding_whitespace_and_nothing_else() {
        assert!(PinRow::compare("x", " #ebebeb ", "#ebebeb").agrees);
        assert!(
            !PinRow::compare("x", "#EBEBEB", "#ebebeb").agrees,
            "case folding is a judgement about the record's syntax, and VDS makes none"
        );
    }
}
