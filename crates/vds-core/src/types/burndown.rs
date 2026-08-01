//! A burndown: a pinned numeric reading whose only lawful direction is down.
//!
//! Draft S-7C, ENACTMENT PENDING (SUBMISSION-VDS-014). This consolidates the
//! pattern every consuming repo was carrying as a bespoke ratchet script, and it
//! is deliberately NOT the geometry bound wearing a second name. A geometry
//! bound admits a count of non-compliant surfaces and requires the ADMISSION to
//! fall on a declared window; a burndown pins the MEASURED reading itself, so:
//!
//!   - ANY increase over the pin is red. There is no slack between the pin and
//!     the measurement, because the pin IS the last measurement.
//!   - A DECREASE that was not re-pinned in the same change is ALSO red. A
//!     stale floor measures the next regression from the wrong place: a metric
//!     that fell from 100 to 60 under a pin of 100 has forty regressions of
//!     invisible headroom, and the instrument only works while the pin sits on
//!     the true number.
//!
//! # The reading is a ledger, not a field on this record
//!
//! The current value is generated out of band by the subject's own reader
//! (VDS S-7(2)(1) forbids a network call - or any measurement - inside a
//! proof), written through `vds ledger burndown`, and witnesses its own content
//! with a digest exactly as the geometry reading does. This record holds the
//! PINS: the history of values somebody stood behind, oldest first, the current
//! pin derived as the last entry and never stored beside it.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::Status;
use crate::digest::Digest;
use crate::error::{Result, VdsError};
use crate::ids::BurndownId;
use crate::project::Project;
use crate::timestamp::Timestamp;

/// One pin, as declared at one moment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PinnedValue {
    pub at: Timestamp,
    /// The measured value somebody stood behind at that moment.
    pub value: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub because: Option<String>,
}

/// One metric under burndown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BurndownRecord {
    pub id: BurndownId,
    pub status: Status,
    /// The machine key the reading reports this metric under. One enforceable
    /// record per metric, or nothing says which pin governs.
    pub metric: String,
    /// By when the metric must reach zero, where a deadline was directed.
    ///
    /// Measured against the READING's `taken_at` and never the wall clock, for
    /// the reason geometry R3 gives: a proof reading the system time produces
    /// different findings from identical inputs (VDS S-7(2)(1)).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline: Option<Timestamp>,
    /// Every pin ever declared, OLDEST FIRST. The pin in force is the last
    /// entry, derived and never stored beside it.
    pub history: Vec<PinnedValue>,
    #[serde(default)]
    pub basis: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl BurndownRecord {
    /// The pin in force, or `None` where none was ever declared.
    pub fn current(&self) -> Option<&PinnedValue> {
        self.history.last()
    }

    /// Whether the history is in chronological order. Checked, not assumed:
    /// every question this record answers reads the LAST entry.
    pub fn is_chronological(&self) -> bool {
        self.history
            .windows(2)
            .all(|w| w[0].at.as_str() <= w[1].at.as_str())
    }

    /// The first entry that RAISED the pin, if any. A pin that goes up is not a
    /// pin; the lawful route to a higher number is a new record with the reason
    /// on it, after deprecating this one.
    pub fn first_raise(&self) -> Option<&PinnedValue> {
        self.history
            .windows(2)
            .find(|w| w[1].value > w[0].value)
            .map(|w| &w[1])
    }
}

pub const BURNDOWN_READING_SCHEMA_VERSION: u32 = 1;

/// One metric's measured value, as the subject's reader produced it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BurndownRow {
    pub metric: String,
    pub value: u64,
    /// Where the count came from, so a row is auditable: a command, a glob, a
    /// query. Never a design value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measured_by: Option<String>,
}

/// The generated burndown reading: every metric's current value, one file.
///
/// A ledger under VDS S-4(2): generated, never hand-edited, byte-reproducible
/// by the named command, and witnessing its own content for the reason the
/// geometry reading does - this is the proof's ONLY measurement, and without
/// the digest a regression is cured by editing one integer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BurndownReading {
    pub schema_version: u32,
    pub generated_by: String,
    /// When the reading was taken. Inside the digest: the deadline rule
    /// measures from it, so moving it changes what the proof concludes.
    pub taken_at: Timestamp,
    pub rows: Vec<BurndownRow>,
    #[serde(default)]
    pub does_not_cover: Vec<String>,
    pub content_digest: Digest,
}

impl BurndownReading {
    pub fn row(&self, metric: &str) -> Option<&BurndownRow> {
        self.rows.iter().find(|r| r.metric == metric)
    }

    pub fn compute_content_digest(&self) -> Result<Digest> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Content<'a> {
            schema_version: u32,
            generated_by: &'a str,
            taken_at: &'a Timestamp,
            rows: &'a [BurndownRow],
            does_not_cover: &'a [String],
        }
        Digest::of_value(&Content {
            schema_version: self.schema_version,
            generated_by: &self.generated_by,
            taken_at: &self.taken_at,
            rows: &self.rows,
            does_not_cover: &self.does_not_cover,
        })
    }

    /// Why this reading may not be relied on, or `None`.
    pub fn untrustworthy_because(&self) -> Result<Option<String>> {
        let recomputed = self.compute_content_digest()?;
        Ok((recomputed != self.content_digest).then(|| {
            format!(
                "the reading's contentDigest is {} and its content digests to {recomputed}. It \
                 was edited after it was generated, or generated by something that did not \
                 compute the digest. Regenerate it rather than correcting the digest by hand.",
                self.content_digest
            )
        }))
    }
}

/// Where the burndown reading lives, per `[burndown] reading_ledger`.
pub fn burndown_reading_path(project: &Project) -> std::path::PathBuf {
    project.root.join(&project.config.burndown.reading_ledger)
}

pub fn write_burndown_reading(
    project: &Project,
    reading: &BurndownReading,
) -> Result<std::path::PathBuf> {
    let path = burndown_reading_path(project);
    let text = serde_yaml::to_string(reading).map_err(|e| VdsError::Serialize {
        what: "the burndown reading".into(),
        message: e.to_string(),
    })?;
    crate::write_text_atomically(&path, &text)?;
    Ok(path)
}

/// Read the burndown reading, or `None` where none has been generated.
/// The schema version is read from the RAW value first (VDS S-11(2)).
pub fn read_burndown_reading(project: &Project) -> Result<Option<BurndownReading>> {
    let path = burndown_reading_path(project);
    if !path.is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path).map_err(|e| VdsError::io(path.display(), e))?;
    let raw: serde_yaml::Value = serde_yaml::from_str(&text).map_err(|e| VdsError::Artefact {
        path: project.rel(&path),
        message: format!("is not readable YAML: {e}"),
    })?;
    let found = raw
        .get("schemaVersion")
        .or_else(|| raw.get("schema_version"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    if found > BURNDOWN_READING_SCHEMA_VERSION {
        return Err(VdsError::SchemaVersionAhead {
            path: project.rel(&path),
            kind: "burndown reading",
            found,
            understood: BURNDOWN_READING_SCHEMA_VERSION,
        });
    }
    let reading: BurndownReading = serde_yaml::from_value(raw).map_err(|e| VdsError::Artefact {
        path: project.rel(&path),
        message: format!("is not a burndown reading: {e}"),
    })?;
    Ok(Some(reading))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pin(at: &str, value: u64) -> PinnedValue {
        PinnedValue {
            at: Timestamp::parse(at).unwrap(),
            value,
            because: None,
        }
    }

    fn record(history: Vec<PinnedValue>) -> BurndownRecord {
        BurndownRecord {
            id: BurndownId::parse("BRN-0001").unwrap(),
            status: Status::Registered,
            metric: "legacy_rule_blocks".into(),
            deadline: None,
            history,
            basis: vec!["draft S-7C".into()],
            notes: None,
        }
    }

    #[test]
    fn the_current_pin_is_the_last_entry() {
        let r = record(vec![
            pin("2026-07-01T00:00:00Z", 376),
            pin("2026-07-20T00:00:00Z", 200),
        ]);
        assert_eq!(r.current().map(|p| p.value), Some(200));
        assert!(r.is_chronological());
        assert!(r.first_raise().is_none());
    }

    #[test]
    fn a_raise_anywhere_in_the_history_is_detectable() {
        let r = record(vec![
            pin("2026-07-01T00:00:00Z", 200),
            pin("2026-07-10T00:00:00Z", 376),
            pin("2026-07-20T00:00:00Z", 100),
        ]);
        assert_eq!(r.first_raise().map(|p| p.value), Some(376));
    }

    #[test]
    fn an_edited_reading_is_untrustworthy_by_its_own_digest() {
        let mut reading = BurndownReading {
            schema_version: BURNDOWN_READING_SCHEMA_VERSION,
            generated_by: "vds ledger burndown --from -".into(),
            taken_at: Timestamp::fixed(2026, 8, 1, 10, 0, 0),
            rows: vec![BurndownRow {
                metric: "legacy_rule_blocks".into(),
                value: 200,
                measured_by: Some("grep -c over src/**".into()),
            }],
            does_not_cover: vec![],
            content_digest: Digest::of_text("placeholder"),
        };
        reading.content_digest = reading.compute_content_digest().unwrap();
        assert!(reading.untrustworthy_because().unwrap().is_none());
        reading.rows[0].value = 5;
        assert!(reading.untrustworthy_because().unwrap().is_some());
    }
}
