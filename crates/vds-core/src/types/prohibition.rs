//! A prohibition: a pattern asserted ABSENT from an enumerated scope.
//!
//! Draft S-7B, ENACTMENT PENDING (SUBMISSION-VDS-013). The instrument for the
//! "no container radius in body regions" / "no dotfield behind main areas" class
//! of directive, and the defect it closes was observed on the subscriber
//! project's migration: twenty-eight source-side gates and not one of them could
//! say "this pattern must not appear HERE", so every such directive lived as
//! prose, and prose is not enforcement.
//!
//! # Why the expansion is RECORDED
//!
//! The scope is an explicit file list or a glob, and the glob's expansion at
//! registration is written into the record. Without it the scope can narrow
//! silently: a file renamed out of the glob takes its violations with it, the
//! proof's population shrinks, and a pass over the smaller scope reads exactly
//! like a pass over the original one. With the expansion recorded, a file that
//! leaves the scope is a finding, not a disappearance.
//!
//! # What a prohibition may not hold
//!
//! A pattern is a STRING SOUGHT, not a value stored. `rounded-` names a class of
//! spellings the directive forbids; it realises nothing, decides nothing, and
//! deleting the record loses no shipped pixel (VDS S-2(5)). What compliance
//! means - which patterns a project forbids where - is the project's directive,
//! recorded with its basis, and never VDS's own taste.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::Status;
use crate::ids::ProhibitionId;
use crate::timestamp::Timestamp;

/// One prohibition: a pattern that must be ABSENT from every file in scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ProhibitionRecord {
    pub id: ProhibitionId,
    pub status: Status,
    /// The forbidden spelling, matched as a LITERAL SUBSTRING per line.
    ///
    /// A substring and not a regular expression, decided as a reversible call
    /// (DECISION-0007): a regex's failure modes - catastrophic backtracking, an
    /// unescaped dot silently widening the match - are exactly the silent-scope
    /// defects this kind exists to refuse, and every motivating directive is a
    /// literal class or import spelling.
    pub pattern: String,
    /// The scope, as declared: explicit paths or globs, repository-relative.
    pub scope: Vec<String>,
    /// What the scope EXPANDED TO when the record was written, sorted.
    ///
    /// The anti-narrowing baseline. The proof re-expands `scope` and compares:
    /// a recorded file the globs no longer reach is a narrowed scope and a
    /// fatal finding, never a silent shrink.
    pub expansion: Vec<String>,
    /// When the directive was given, so a reader can date the undertaking.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directed_at: Option<Timestamp>,
    /// Why this pattern is forbidden here, in one line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub because: Option<String>,
    /// The authorities this prohibition rests on.
    #[serde(default)]
    pub basis: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Status;

    #[test]
    fn a_prohibition_round_trips_through_yaml() {
        let record = ProhibitionRecord {
            id: ProhibitionId::parse("PRB-0001").unwrap(),
            status: Status::Registered,
            pattern: "rounded-".into(),
            scope: vec!["src/components/body/**/*.tsx".into()],
            expansion: vec!["src/components/body/panel.tsx".into()],
            directed_at: Some(Timestamp::fixed(2026, 8, 1, 10, 0, 0)),
            because: Some("no container radius in body regions".into()),
            basis: vec!["draft S-7B".into()],
            notes: None,
        };
        let text = serde_yaml::to_string(&record).unwrap();
        let back: ProhibitionRecord = serde_yaml::from_str(&text).unwrap();
        assert_eq!(back, record);
    }

    #[test]
    fn an_unknown_field_is_refused_rather_than_ignored() {
        let text =
            "id: PRB-0001\nstatus: registered\npattern: x\nscope: []\nexpansion: []\nsurprise: 1\n";
        assert!(serde_yaml::from_str::<ProhibitionRecord>(text).is_err());
    }
}
