//! Pins: a derived one-way agreement assertion between two named records.
//!
//! # Why this type carries no per-value digest
//!
//! VDS S-2(7) as drafted says a pin row carries `source_value_digest` and
//! `target_value_digest` "and never the two strings", and calls that "what keeps
//! the pin a gate rather than a store".
//!
//! **That construction does not work, and it was measured.** An unsalted SHA-256
//! over a low-entropy domain is not one-way in any practical sense. The domain of
//! a design token value is tiny: a hex colour is 24 bits, roughly 16.7 million
//! candidates, and a spacing step or a duration is smaller still. An adversarial
//! agent recovered all 52 values from a pin of 26 rows in 27 seconds on one CPU.
//! A pin built as S-2(7) describes therefore STORES the decided and the shipped
//! values, in a form that is inconvenient to read and trivial to recover, which
//! is exactly the storing form [2026] VJS-CC-OPBOX 3 forbids.
//!
//! Salting does not rescue it. A salt recorded in the pin is a salt the reader
//! has, so the search is unchanged. A salt NOT recorded in the pin makes the pin
//! irreproducible, which fails the regeneration limb at VDS S-2(5)(4).
//!
//! So a row carries the NAME and the AGREEMENT, and nothing else. That is
//! sufficient for every limb of the four-limb test:
//!
//!   - **Deletion.** Delete the pin: no shipped or decided value is lost, because
//!     none was ever in it.
//!   - **Divergence.** Make the records disagree: `agrees` goes false and the
//!     gate fails closed.
//!   - **Authorship.** No reader can change a shipped pixel by editing this file;
//!     there is no pixel in it to edit.
//!   - **Regeneration.** The row is recomputable by the command in
//!     `generated_by` from the two named records.
//!
//! Whether the records as a whole moved is answered by
//! [`RecordOfTruth::digest`], which digests an entire file. A file is not a
//! low-entropy domain, so that digest is safe and is the right place for the
//! question.
//!
//! This departs from the drafted text of S-2(7), which is not commenced
//! (VDS S-15) and which mandates a construction that provably does the opposite
//! of what the clause says it does. The departure is referred as
//! `SUBMISSION-VDS-006` and the fail-closed interim is the design implemented
//! here: no per-value digest anywhere in `.vds/`.

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
    /// What this record is the system of record FOR, e.g. "what ships".
    pub authority_for: String,
    /// How to reach it: a repository-relative path, or a Figma file key.
    pub locator: String,
    /// The digest of the record AS A WHOLE, as read when the pin was generated.
    ///
    /// Safe where a per-value digest is not: a file is not a low-entropy domain,
    /// so this cannot be inverted to recover any value inside it. It answers
    /// "did this record move", which is a different question from "what does
    /// this record say".
    pub digest: Digest,
}

/// One compared row.
///
/// The name of the thing compared, and whether the two records agreed about it.
/// There is deliberately nowhere here to put a value or a digest of one; see the
/// module documentation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PinRow {
    /// The NAME of the token, boundary or property compared. A name, never a
    /// value: `control-border`, not `#ebebeb`.
    pub name: String,
    pub agrees: bool,
    /// Present only where the row was NOT enforced, saying why. A row silently
    /// excluded is a row nobody can audit, and a pin full of them looks
    /// identical to a pin that passed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_enforced_because: Option<String>,
}

impl PinRow {
    /// Compare two values and keep only the verdict.
    ///
    /// The values are borrowed, compared and dropped. Nothing derived from
    /// either survives this call except the boolean, which is the point.
    pub fn compare(name: impl Into<String>, source: &str, target: &str) -> Self {
        Self {
            name: name.into(),
            agrees: source.trim() == target.trim(),
            not_enforced_because: None,
        }
    }

    /// A row that was considered and not enforced.
    pub fn not_enforced(name: impl Into<String>, because: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            agrees: false,
            not_enforced_because: Some(because.into()),
        }
    }

    pub fn is_enforced(&self) -> bool {
        self.not_enforced_because.is_none()
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
    /// The digest of what this pin SAYS, which is what `digest` must equal.
    ///
    /// Over the subject, the direction, both records, the rows, the counts,
    /// `fails_closed` and `generated_by`. Deliberately NOT over `generated_at`,
    /// `id`, `digest` or `proof_id`.
    ///
    /// Excluding `generated_at` is the load-bearing part. A regeneration over an
    /// unchanged pair of records restamps it, and digesting the stamp would move
    /// this value on a pin whose verdict did not change, which makes every
    /// warrant citing it look spent and teaches a reader to ignore the field
    /// (VDS S-7(2)(1)).
    ///
    /// Why it exists at all: without it nothing could tell a generated agreement
    /// from a hand-edited one. Flipping `agrees: false` to `true` in a committed
    /// pin produced a PASSING `token_pin` run, because no code derived this field
    /// and there was nothing to compare it with. A pin is a generated artefact
    /// and never hand-edited, and this is what makes that a refusal at the door
    /// rather than a hope.
    ///
    /// It lives HERE and not in the proof so there is one definition. Two
    /// canonicalisations of one shape drift, and the drift shows up as a pin that
    /// passes the gate and fails its own generator.
    pub fn compute_content_digest(&self) -> crate::Result<Digest> {
        #[derive(Serialize)]
        struct Content<'a> {
            subject: &'a str,
            direction: &'a PinDirection,
            source: [&'a str; 3],
            target: [&'a str; 3],
            rows: &'a [PinRow],
            rows_considered: u64,
            rows_enforced: u64,
            fails_closed: bool,
            generated_by: &'a str,
        }
        Digest::of_value(&Content {
            subject: &self.subject,
            direction: &self.direction,
            source: [
                &self.source_of_record.authority_for,
                &self.source_of_record.locator,
                self.source_of_record.digest.as_str(),
            ],
            target: [
                &self.target_of_record.authority_for,
                &self.target_of_record.locator,
                self.target_of_record.digest.as_str(),
            ],
            rows: &self.rows,
            rows_considered: self.rows_considered,
            rows_enforced: self.rows_enforced,
            fails_closed: self.fails_closed,
            generated_by: &self.generated_by,
        })
    }

    /// Whether this pin still says what its digest says it says.
    pub fn digest_matches(&self) -> crate::Result<bool> {
        Ok(self.compute_content_digest()? == self.digest)
    }

    pub fn disagreements(&self) -> impl Iterator<Item = &PinRow> {
        self.rows.iter().filter(|r| r.is_enforced() && !r.agrees)
    }

    /// Why this pin is not sound, or an empty list.
    pub fn defects(&self) -> Vec<String> {
        let mut out = Vec::new();
        if !self.fails_closed {
            out.push(
                "fails_closed is false. A pin that does not fail closed keeps serving its own \
                 answer when the records diverge, which is a third opinion nobody asked for \
                 (VDS S-2(5)(2))."
                    .into(),
            );
        }
        let enforced = self.rows.iter().filter(|r| r.is_enforced()).count() as u64;
        if enforced != self.rows_enforced {
            out.push(format!(
                "rows_enforced says {} and {enforced} rows are actually enforced",
                self.rows_enforced
            ));
        }
        if self.rows.len() as u64 != self.rows_considered {
            out.push(format!(
                "rows_considered says {} and the pin holds {} rows",
                self.rows_considered,
                self.rows.len()
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_keeps_the_verdict_and_neither_value() {
        let row = PinRow::compare("control-border", "#ebebeb", "#ebebeb");
        assert!(row.agrees);
        let serialised = serde_json::to_string(&row).unwrap();
        assert!(
            !serialised.contains("ebebeb"),
            "a pin row must not carry the value it compared: {serialised}"
        );
    }

    /// The defect this type is shaped around. A per-value digest over a
    /// low-entropy domain is recoverable, so it stores the value.
    #[test]
    fn a_row_carries_nothing_a_brute_force_could_invert() {
        let row = PinRow::compare("control-border", "#ebebeb", "#ececec");
        let serialised = serde_json::to_string(&row).unwrap();

        // Stand in for the attack: enumerate a slice of the 24-bit colour domain
        // and confirm no field of the row can be matched back to a candidate.
        // The row holds one name and one boolean, so there is nothing to match.
        for r in 0xea..=0xee {
            for g in 0xea..=0xee {
                for b in 0xea..=0xee {
                    let candidate = format!("#{r:02x}{g:02x}{b:02x}");
                    let digest = Digest::of_text(&candidate);
                    assert!(
                        !serialised.contains(digest.as_str()),
                        "the row contains the digest of {candidate}, which is recoverable by \
                         exactly this loop in under a minute"
                    );
                    assert!(!serialised.contains(&candidate));
                }
            }
        }
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

    #[test]
    fn an_unenforced_row_is_not_counted_as_a_disagreement() {
        let pin = pin(vec![
            PinRow::compare("a", "1", "1"),
            PinRow::not_enforced("b", "the target record does not name this token"),
        ]);
        assert_eq!(pin.disagreements().count(), 0);
    }

    #[test]
    fn a_pin_whose_counts_disagree_with_its_rows_is_defective() {
        let mut p = pin(vec![PinRow::compare("a", "1", "2")]);
        p.rows_enforced = 99;
        assert!(!p.defects().is_empty());
    }

    #[test]
    fn a_pin_that_does_not_fail_closed_is_defective() {
        let mut p = pin(vec![PinRow::compare("a", "1", "1")]);
        p.fails_closed = false;
        assert!(p.defects().iter().any(|d| d.contains("third opinion")));
    }

    fn pin(rows: Vec<PinRow>) -> Pin {
        let enforced = rows.iter().filter(|r| r.is_enforced()).count() as u64;
        Pin {
            id: PinId::parse("PIN-20260725-100000").unwrap(),
            subject: "control boundary".into(),
            direction: PinDirection::OneWayDerived,
            source_of_record: RecordOfTruth {
                authority_for: "what ships".into(),
                locator: "app/globals.css".into(),
                digest: Digest::of_text("css"),
            },
            target_of_record: RecordOfTruth {
                authority_for: "what is decided".into(),
                locator: "FIGMAKEY".into(),
                digest: Digest::of_text("figma"),
            },
            rows_considered: rows.len() as u64,
            rows_enforced: enforced,
            rows,
            fails_closed: true,
            generated_at: Timestamp::fixed(2026, 7, 25, 10, 0, 0),
            generated_by: "vds pin token".into(),
            digest: Digest::of_text("pin"),
            proof_id: None,
        }
    }
}
