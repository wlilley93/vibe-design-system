//! The two governance logs: a decisive call, and a self-reported breach.
//!
//! Both directories existed and both were hand-written YAML with no type, no
//! schema and no command. That is the shape VDS exists to close: an artefact
//! nobody validates is an artefact that drifts, and `vds doctor` D9 counted the
//! decision logs by listing a directory without ever opening one.
//!
//! # Why a breach report is restorative and never punitive
//!
//! VDS S-12(3) fixes the schema and the last sentence of the clause is the whole
//! design: "Remedy is restorative, not punitive: the work is made good and the
//! lawful route resumed." So there is no `blame` field, no `severity`, and
//! nothing that grades the author. What the record holds is what happened, which
//! instrument it fell below, how it was found, what stopped the bleeding, and
//! what was done to make it good.
//!
//! A system that punishes self-reporting stops receiving self-reports, and a
//! breach nobody files is a breach nobody fixes.
//!
//! # Why the decision log carries `court_required` rather than omitting it
//!
//! VDS S-12(2): "A reversible call with low blast radius is a decision log, not a
//! referral. The log carries `court_required: false` and `why`, which is what
//! records that a fork was considered and disposed without a sitting."
//!
//! The field is therefore not redundant with the file's existence. Its value is
//! the CLAIM that a sitting was unnecessary, and `why` is the argument for it,
//! which is a thing a reviewer can disagree with. A log that simply recorded what
//! was done would record no such claim and could not be contested.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ids::{BreachId, DecisionId, SubmissionId};
use crate::timestamp::Timestamp;

/// A fork disposed without a sitting. VDS S-12(2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DecisionLog {
    pub id: DecisionId,
    pub at: Timestamp,
    pub by: String,
    /// What was decided, in one sentence.
    pub decision: String,
    /// Whether this needed the court.
    ///
    /// `false` is the ordinary case and is a CLAIM: that the call was reversible
    /// and its blast radius low. `why` is the argument for that claim, and a
    /// reviewer who disagrees with it has something concrete to disagree with.
    pub court_required: bool,
    /// Why the call was disposable without a sitting, or why it was not.
    pub why: String,
    /// The clauses and rulings this rests on.
    pub basis: Vec<String>,
    /// Where `court_required` is true, the submission that carries the question.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub submission_id: Option<SubmissionId>,
    /// For a decision about re-pinning a gate: the digest the re-pin superseded.
    ///
    /// Redundant with `.vds/enforcement.lock`, which records the same value, and
    /// kept anyway. VDS S-8(5) is explicit that the lock cannot bind an author
    /// who edits a gate and re-locks it in one act; the log is the non-machine
    /// backstop that leaves behind, and a backstop that has to be read alongside
    /// the thing it backs up is not one. Carrying the digest here means the log
    /// says what was superseded even if the lock is the file under suspicion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supersedes_digest: Option<crate::digest::Digest>,
}

impl DecisionLog {
    /// Why this log is not sound, or an empty list.
    ///
    /// Checked rather than asserted, and reported by `vds doctor`, because the
    /// failure mode for a governance log is not being malformed. It is being
    /// well-formed and empty of content: a `why` that says "for clarity" records
    /// a fork nobody can now reconstruct.
    pub fn defects(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.court_required && self.submission_id.is_none() {
            out.push(
                "court_required is true and no submission_id is named, so the question was \
                 referred to nowhere. VDS S-12(2) makes the log the alternative to a referral, \
                 not a substitute for one."
                    .into(),
            );
        }
        if !self.court_required && self.submission_id.is_some() {
            out.push(
                "court_required is false and a submission_id is named. One of the two is \
                 wrong: either the fork went to the court or it did not."
                    .into(),
            );
        }
        if self.why.trim().len() < 60 {
            out.push(format!(
                "why is {} characters. VDS S-12(2) makes `why` the record that a fork was \
                 CONSIDERED and disposed, so a reader has to be able to reconstruct the \
                 argument and disagree with it.",
                self.why.trim().len()
            ));
        }
        if self.basis.is_empty() {
            out.push(
                "basis is empty, so this call rests on nothing anybody can look up. Cite the \
                 clauses or the rulings it stands on."
                    .into(),
            );
        }
        if self.decision.trim().is_empty() {
            out.push("decision is empty".into());
        }
        out
    }
}

/// A self-reported breach. VDS S-12(3) fixes the field set and this type is it.
///
/// There is deliberately no `blame`, no `severity` and nothing grading the
/// author. S-12(3): remedy is restorative, and a system that punishes
/// self-reporting stops receiving self-reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BreachReport {
    pub id: BreachId,
    pub at: Timestamp,
    pub by: String,
    /// What happened, in enough detail to be checked.
    pub what_happened: String,
    /// Each entry cites an instrument. S-12(3) requires the citation, because a
    /// breach of nothing in particular is an apology rather than a record.
    pub law_breached: Vec<String>,
    pub discovered_by: String,
    /// What stopped it getting worse, before the remedy.
    pub containment: String,
    /// What was done to make the work good. Restorative, never punitive.
    pub remedy: Vec<String>,
    /// What stops it recurring, or an honest statement that nothing does.
    ///
    /// Not in the S-12(3) field list, and added because a breach with a remedy
    /// and no prevention is a breach that will be filed again. Optional, so a
    /// record written to the clause's letter still validates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prevention: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision_log_id: Option<DecisionId>,
}

impl BreachReport {
    pub fn defects(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.law_breached.is_empty() {
            out.push(
                "law_breached is empty. VDS S-12(3) requires each entry to cite an instrument, \
                 because a breach of nothing in particular is an apology rather than a record."
                    .into(),
            );
        }
        if self.remedy.is_empty() {
            out.push(
                "remedy is empty. VDS S-12(3) makes remedy restorative: the work is made good \
                 and the lawful route resumed. A breach filed with no remedy has recorded a \
                 fault and repaired nothing."
                    .into(),
            );
        }
        if self.what_happened.trim().len() < 60 {
            out.push(format!(
                "what_happened is {} characters, which is too short to be checked by anybody \
                 who was not there.",
                self.what_happened.trim().len()
            ));
        }
        if self.discovered_by.trim().is_empty() {
            out.push(
                "discovered_by is empty. How a breach was found decides whether the same class \
                 gets found again."
                    .into(),
            );
        }
        if self.containment.trim().is_empty() {
            out.push("containment is empty".into());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decision() -> DecisionLog {
        DecisionLog {
            id: DecisionId::parse("DECISION-0001").unwrap(),
            at: Timestamp::fixed(2026, 7, 25, 10, 0, 0),
            by: "lexby".into(),
            decision: "pin jsx.rs and glob.rs in the enforcement lock".into(),
            court_required: false,
            why: "Reversible and low blast radius, and it TIGHTENS rather than relaxes: it \
                  removes false positives and adds no exemption."
                .into(),
            basis: vec!["VDS S-8(1)".into()],
            submission_id: None,
            supersedes_digest: None,
        }
    }

    #[test]
    fn a_sound_decision_log_has_no_defects() {
        assert!(
            decision().defects().is_empty(),
            "{:?}",
            decision().defects()
        );
    }

    /// The two halves of `court_required` must agree with the submission field,
    /// in both directions. A log claiming a referral that names none has referred
    /// the question to nowhere.
    #[test]
    fn court_required_and_the_submission_must_agree() {
        let mut referred = decision();
        referred.court_required = true;
        assert!(
            referred.defects().iter().any(|d| d.contains("nowhere")),
            "{:?}",
            referred.defects()
        );

        let mut both = decision();
        both.submission_id = Some(SubmissionId::parse("SUBMISSION-VDS-001").unwrap());
        assert!(
            both.defects().iter().any(|d| d.contains("One of the two")),
            "{:?}",
            both.defects()
        );
    }

    /// The failure mode for a governance log is being well-formed and empty of
    /// content, not being malformed.
    #[test]
    fn a_why_too_short_to_reconstruct_the_argument_is_a_defect() {
        let mut thin = decision();
        thin.why = "for clarity".into();
        assert!(
            thin.defects().iter().any(|d| d.contains("why is")),
            "{:?}",
            thin.defects()
        );
    }

    #[test]
    fn a_decision_resting_on_nothing_anybody_can_look_up_is_a_defect() {
        let mut ungrounded = decision();
        ungrounded.basis = vec![];
        assert!(
            ungrounded.defects().iter().any(|d| d.contains("basis")),
            "{:?}",
            ungrounded.defects()
        );
    }

    fn breach() -> BreachReport {
        BreachReport {
            id: BreachId::parse("BREACH-0001").unwrap(),
            at: Timestamp::fixed(2026, 7, 25, 10, 0, 0),
            by: "lexby".into(),
            what_happened: "A control-boundary token declared aligned between the decided \
                            target and production was measured at 1.20:1 against both planes."
                .into(),
            law_breached: vec!["WCAG 2.2 SC 1.4.11".into()],
            discovered_by: "a hand audit".into(),
            containment: "the audit was written down".into(),
            remedy: vec!["the contrast proof".into()],
            prevention: None,
            decision_log_id: None,
        }
    }

    #[test]
    fn a_sound_breach_report_has_no_defects() {
        assert!(breach().defects().is_empty(), "{:?}", breach().defects());
    }

    /// The two fields S-12(3) makes load-bearing.
    #[test]
    fn a_breach_citing_no_instrument_and_a_breach_with_no_remedy_are_both_defective() {
        let mut uncited = breach();
        uncited.law_breached = vec![];
        assert!(
            uncited.defects().iter().any(|d| d.contains("an apology")),
            "{:?}",
            uncited.defects()
        );

        let mut unremedied = breach();
        unremedied.remedy = vec![];
        assert!(
            unremedied
                .defects()
                .iter()
                .any(|d| d.contains("repaired nothing")),
            "{:?}",
            unremedied.defects()
        );
    }

    /// A breach report has nowhere to put blame, and that is the point.
    #[test]
    fn a_breach_report_has_no_field_for_blame() {
        let text = serde_yaml::to_string(&breach()).unwrap();
        for punitive in ["blame", "fault", "severity", "responsible", "penalty"] {
            assert!(
                !text.contains(punitive),
                "the serialised breach carries {punitive:?}. VDS S-12(3) makes remedy \
                 restorative, and a system that punishes self-reporting stops receiving \
                 self-reports: {text}"
            );
        }
    }
}
