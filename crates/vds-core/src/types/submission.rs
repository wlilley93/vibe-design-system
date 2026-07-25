//! Submissions: a question referred to VJS, and the order that answered it.
//!
//! VDS S-10(1): every judgement call routes to VJS. This type is the whole of
//! VDS's answer to a contested question, because VDS has no bench, no citator,
//! no appeal route and no power to resolve one (VDS S-1(2)).
//!
//! VDS S-10(2): before filing, the citator is checked. A submission must list
//! every near authority considered and say why each is not on all fours. The
//! `citator_checked` field is non-optional and must be non-empty for exactly
//! that reason: a submission that skips the citator check re-litigates settled
//! law.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::digest::Digest;
use crate::ids::SubmissionId;
use crate::timestamp::Timestamp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SubmissionStatus {
    Draft,
    Filed,
    Answered,
    Withdrawn,
}

/// The enumerated triggers at VDS S-10(3). Everything else is a decision log
/// under VDS S-12(2), which is what keeps referral cheap enough to actually use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SubmissionTrigger {
    FirstImpression,
    GenuineDistinction,
    ProposalToOverrule,
    InstructionConflictsWithDesignpack,
    DiscoveredBreach,
}

impl SubmissionTrigger {
    pub const ALL: [SubmissionTrigger; 5] = [
        SubmissionTrigger::FirstImpression,
        SubmissionTrigger::GenuineDistinction,
        SubmissionTrigger::ProposalToOverrule,
        SubmissionTrigger::InstructionConflictsWithDesignpack,
        SubmissionTrigger::DiscoveredBreach,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            SubmissionTrigger::FirstImpression => "first_impression",
            SubmissionTrigger::GenuineDistinction => "genuine_distinction",
            SubmissionTrigger::ProposalToOverrule => "proposal_to_overrule",
            SubmissionTrigger::InstructionConflictsWithDesignpack => {
                "instruction_conflicts_with_designpack"
            }
            SubmissionTrigger::DiscoveredBreach => "discovered_breach",
        }
    }

    pub fn parse(raw: &str) -> Option<SubmissionTrigger> {
        SubmissionTrigger::ALL.into_iter().find(|t| t.as_str() == raw)
    }
}

/// One near authority the drafter considered, and why it is not on all fours.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CitatorEntry {
    pub citation: String,
    pub why_not_on_all_fours: String,
}

/// An option the bench may take, with its consequence stated by the drafter.
///
/// A submission that offers one option has not asked a question, it has
/// requested a rubber stamp, which is why the schema requires at least two.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SubmissionOption {
    pub option: String,
    pub consequence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SubmissionAnswer {
    /// The VJS citation, e.g. `[2026] VJS-CC-VDS 1`.
    pub citation: String,
    /// The ratio, which is the only part that binds. Obiter is persuasive.
    pub ratio: String,
    pub answered_at: Timestamp,
}

/// Always `vjs`. VDS refers and never decides, so there is nowhere else to file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FiledTo {
    #[default]
    Vjs,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Submission {
    pub id: SubmissionId,
    pub status: SubmissionStatus,
    pub question: String,
    pub trigger: SubmissionTrigger,
    pub why_first_impression: String,
    pub citator_checked: Vec<CitatorEntry>,
    pub options: Vec<SubmissionOption>,
    pub evidence: Vec<String>,
    pub case_file_digest: Digest,
    /// The VDS clause that depends on this question, where one does.
    #[serde(default)]
    pub reserved_clause: Option<String>,
    /// What VDS does until the question is answered. VDS S-15(3): a reserved
    /// clause fails closed in the meantime, and the fail-closed position is
    /// written down rather than left to whoever reads the clause next.
    #[serde(default)]
    pub fail_closed_interim: Option<String>,
    pub filed_to: FiledTo,
    #[serde(default)]
    pub vjs_submission_id: Option<String>,
    pub created_at: Timestamp,
    #[serde(default)]
    pub filed_at: Option<Timestamp>,
    #[serde(default)]
    pub answer: Option<SubmissionAnswer>,
}

impl Submission {
    /// Why this submission is defective, or `None`. VDS S-10(2).
    pub fn defects(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.citator_checked.is_empty() {
            out.push(
                "no citator entry. A submission must list every near authority considered and \
                 say why each is not on all fours; one that skips the check re-litigates \
                 settled law (VDS S-10(2))."
                    .into(),
            );
        }
        if self.options.len() < 2 {
            out.push(
                "fewer than two options. A submission offering one option has not asked a \
                 question."
                    .into(),
            );
        }
        if self.reserved_clause.is_some() && self.fail_closed_interim.is_none() {
            out.push(
                "a reserved clause with no fail-closed interim. VDS S-15(3) requires the \
                 clause to fail closed until the question is answered, and the interim \
                 position must be written down."
                    .into(),
            );
        }
        if matches!(self.status, SubmissionStatus::Answered) && self.answer.is_none() {
            out.push("status is answered and no answer is recorded".into());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn submission() -> Submission {
        Submission {
            id: SubmissionId::parse("SUBMISSION-VDS-001").unwrap(),
            status: SubmissionStatus::Filed,
            question: "may W1 be granted provisionally on a greenfield surface".into(),
            trigger: SubmissionTrigger::FirstImpression,
            why_first_impression: "no authority reaches the greenfield case".into(),
            citator_checked: vec![CitatorEntry {
                citation: "[2026] VJS-CC-OPBOX 3".into(),
                why_not_on_all_fours: "concerns the token layer, not the register".into(),
            }],
            options: vec![
                SubmissionOption {
                    option: "strict W1".into(),
                    consequence: "greenfield surfaces cannot start".into(),
                },
                SubmissionOption {
                    option: "provisional W1 ratified later".into(),
                    consequence: "a hole large enough to drive the mechanism through".into(),
                },
            ],
            evidence: vec![],
            case_file_digest: Digest::of_text("case"),
            reserved_clause: Some("S-6(5)".into()),
            fail_closed_interim: Some("no provisional registration exists and W1 is strict".into()),
            filed_to: FiledTo::Vjs,
            vjs_submission_id: None,
            created_at: Timestamp::fixed(2026, 7, 25, 10, 0, 0),
            filed_at: None,
            answer: None,
        }
    }

    #[test]
    fn a_well_formed_submission_has_no_defects() {
        assert!(submission().defects().is_empty(), "{:?}", submission().defects());
    }

    #[test]
    fn a_submission_with_no_citator_entry_is_defective() {
        let mut s = submission();
        s.citator_checked.clear();
        assert_eq!(s.defects().len(), 1);
    }

    #[test]
    fn a_submission_offering_one_option_is_defective() {
        let mut s = submission();
        s.options.truncate(1);
        assert_eq!(s.defects().len(), 1);
    }

    #[test]
    fn a_reserved_clause_with_no_fail_closed_interim_is_defective() {
        let mut s = submission();
        s.fail_closed_interim = None;
        assert_eq!(s.defects().len(), 1);
    }

    #[test]
    fn an_answered_submission_with_no_answer_is_defective() {
        let mut s = submission();
        s.status = SubmissionStatus::Answered;
        assert_eq!(s.defects().len(), 1);
    }
}
