//! The `burndown` proof. A pinned reading whose only lawful direction is down.
//!
//! Draft S-7C, ENACTMENT PENDING (SUBMISSION-VDS-014). The fourteenth kind, and
//! the consolidation of a pattern every consuming repo was carrying as its own
//! bespoke ratchet script - each with its own storage, its own comparison, and
//! its own quiet way of rotting.
//!
//! # Why a decrease that was not re-pinned is RED
//!
//! This is the clause that distinguishes a burndown from a ratchet, and it came
//! from a measured failure. A ratchet holds a ceiling: the metric falls, the
//! ceiling stays, and the gap between them is invisible headroom - a metric
//! that fell from 100 to 60 under a pin of 100 can regress forty times before
//! the instrument says a word. A stale floor measures the next regression from
//! the wrong place. So here the pin must SIT ON the measured value: any reading
//! above the pin is red (the metric rose), and any reading below it is ALSO red
//! until the pin is lowered to match in the same change (the instrument is
//! stale). Green means exactly one thing: the pin is the truth.
//!
//! # The rules
//!
//! One row is one burndown record.
//!
//!   R1  the reading EXCEEDS the pin. The metric rose. Fatal.
//!   R2  the reading is BELOW the pin and the pin was not re-pinned. Fatal,
//!       and the finding says what to run, because this failure is good news
//!       mis-filed and the cure is one command.
//!   R3  the deadline passed and the metric is not zero. Measured against the
//!       READING's `taken_at`, never the wall clock (VDS S-7(2)(1)).
//!   R4  the pin was RAISED anywhere in its history. A pin that goes up is not
//!       a pin; the honest route to a higher number is a new baseline record
//!       with the reason on it.
//!   R5  a binding record with no reading, or none for its metric. UNKNOWN,
//!       never a pass.
//!   R6  the reading does not match its own content digest. Fatal, and NO row
//!       is enforced against it: this proof's only measurement cannot be
//!       relied on, so every comparison would be against a number somebody
//!       may have typed.
//!   R7  two enforceable records name one metric. Nothing says which pin
//!       governs.
//!   R8  an empty or out-of-order history. An input that does not add up
//!       cannot be compared against anything.

use std::collections::BTreeMap;
use std::io::Write;

use vds_core::{BurndownRecord, ProofKind, Result, Violation};

use crate::ProofContext;
use crate::run::{Outcome, Verdict};

pub const GATE: &str = "crates/vds-proof/src/burndown.rs";

const RULE_ROSE: &str = "draft S-7C burndown R1: the metric rose above its pin";
const RULE_STALE_PIN: &str = "draft S-7C burndown R2: a decrease not re-pinned measures the next regression from the wrong place";
const RULE_DEADLINE: &str =
    "draft S-7C burndown R3: the deadline passed and the metric is not zero";
const RULE_RAISED: &str = "draft S-7C burndown R4: the pin was raised";
const RULE_READING_AGE: &str = "S-7C(5) ([2026] VJS-CA-VDS 1 order 13) burndown R10: a deadline may not be deferred by \
     a reading that stopped moving";
const RULE_NO_AGE_DECLARED: &str =
    "S-7C(5) burndown R11: a deadline without a declared maximum reading age";
const RULE_NO_READING: &str = "draft S-7C burndown R5: nothing was measured";
const RULE_READING_EDITED: &str =
    "VDS S-2(5)(4) burndown R6: the reading witnesses its own content";
const RULE_DUPLICATE_METRIC: &str = "draft S-7C burndown R7: two pins for one metric";
const RULE_INPUT_INCOHERENT: &str = "draft S-7C burndown R8: an input that does not add up";

pub fn run(ctx: &ProofContext, out: &mut dyn Write) -> Result<Outcome> {
    let project = ctx.project;
    let store = ctx.store();
    let mut run = ctx.new_run(ProofKind::Burndown, GATE);

    let records = store.read_burndowns()?;
    for record in &records {
        run.input_file(&record.path)?;
    }

    let reading = vds_core::read_burndown_reading(project)?;
    if reading.is_some() {
        run.input_file(&vds_core::burndown_reading_path(project))?;
    }

    // THE INDEPENDENT WITNESS (S-7C(5), [2026] VJS-CA-VDS 1 order 13). The
    // freshest `generatedAt` among the ledgers this run can read, which the
    // subject cannot hold still by declining to regenerate the burndown
    // reading. `register_completeness` already measures its grace against this
    // same witness, so no new input is introduced and determinism holds: the
    // finding is still a function of artefacts on disk and never of the clock.
    let witness: Option<vds_core::Timestamp> = vds_scan::load_fresh(project)
        .ok()
        .map(|ledger| ledger.generated_at);
    if let Some(witness) = &witness {
        run.input_named(
            "<screens ledger generated_at>",
            vds_core::Digest::of_text(witness.as_str()),
        );
    }

    // R6 first, for geometry R10's reason: an edited reading's every number is
    // untrustworthy, so nothing may be compared against it.
    let mut reading = reading;
    if let Some(found) = reading.as_ref()
        && let Some(why) = found.untrustworthy_because()?
    {
        run.fail(Violation::fatal(
            project.rel(&vds_core::burndown_reading_path(project)),
            RULE_READING_EDITED,
            "a reading whose contentDigest matches its own content",
            why,
        ));
        reading = None;
    }

    // R7's census, built before any row is classified.
    let mut by_metric: BTreeMap<&str, u32> = BTreeMap::new();
    for record in &records {
        if record.value.status.is_enforceable() {
            *by_metric.entry(record.value.metric.as_str()).or_default() += 1;
        }
    }

    for record in &records {
        let burndown: &BurndownRecord = &record.value;
        let location = format!("{} [{}]", burndown.id, burndown.metric);

        if !burndown.status.is_enforceable() {
            run.row(Verdict::Skipped("burndown_not_in_an_enforceable_status"));
            continue;
        }

        if by_metric
            .get(burndown.metric.as_str())
            .copied()
            .unwrap_or(0)
            > 1
        {
            run.row(Verdict::Skipped("two_enforceable_pins_for_one_metric"));
            run.fail(Violation::fatal(
                location.clone(),
                RULE_DUPLICATE_METRIC,
                format!("exactly one enforceable pin for {:?}", burndown.metric),
                "more than one enforceable record names this metric, and nothing says which \
                 pin governs. Deprecate the superseded one rather than deleting it: the \
                 history is the evidence the number ever fell."
                    .to_owned(),
            ));
            continue;
        }

        if !burndown.is_chronological() {
            run.row(Verdict::Skipped("history_out_of_order"));
            run.fail(Violation::fatal(
                location.clone(),
                RULE_INPUT_INCOHERENT,
                "a history in chronological order, oldest first",
                "the history is out of order, so the pin in force is read from the wrong \
                 moment."
                    .to_owned(),
            ));
            continue;
        }

        let Some(current) = burndown.current() else {
            run.row(Verdict::Skipped("no_pin_ever_declared"));
            run.fail(Violation::fatal(
                location.clone(),
                RULE_INPUT_INCOHERENT,
                "at least one entry in the pin history",
                "the history is empty, so there is no pin in force and nothing to compare a \
                 reading against."
                    .to_owned(),
            ));
            continue;
        };

        // R4 before the comparison: a raised pin makes any later "pass" a pass
        // against a number that was moved to make it one.
        if let Some(raise) = burndown.first_raise() {
            run.row(Verdict::Enforced);
            run.fail(Violation::fatal(
                location.clone(),
                RULE_RAISED,
                "a history in which the pin only ever falls",
                format!(
                    "the pin was RAISED to {} on {}. A pin that goes up is not a pin. If the \
                     population genuinely grew, the honest record is a new baseline with the \
                     reason on it, after deprecating this one.",
                    raise.value,
                    raise.at.as_str()
                ),
            ));
            continue;
        }

        let Some(reading) = reading.as_ref() else {
            run.row(Verdict::Skipped("no_reading_generated"));
            run.fail(Violation::fatal(
                location.clone(),
                RULE_NO_READING,
                format!(
                    "a burndown reading at {}",
                    project.rel(&vds_core::burndown_reading_path(project))
                ),
                format!(
                    "no reading has been generated, so the pin of {} is compared against \
                     nothing. UNKNOWN, not a pass.",
                    current.value
                ),
            ));
            continue;
        };

        let Some(row) = reading.row(&burndown.metric) else {
            run.row(Verdict::Skipped("reading_covers_no_such_metric"));
            run.fail(Violation::fatal(
                location.clone(),
                RULE_NO_READING,
                format!("the reading to cover {:?}", burndown.metric),
                format!(
                    "the reading covers {}, and not this metric. A pin over a number nothing \
                     measures is UNKNOWN, not met.",
                    if reading.rows.is_empty() {
                        "nothing".to_owned()
                    } else {
                        reading
                            .rows
                            .iter()
                            .map(|r| r.metric.clone())
                            .collect::<Vec<_>>()
                            .join(", ")
                    }
                ),
            ));
            continue;
        };

        run.row(Verdict::Enforced);

        // R1: the metric rose. ANY increase: there is no slack, because the
        // pin is the last measurement, not an allowance above it.
        if row.value > current.value {
            run.fail(Violation::fatal(
                location.clone(),
                RULE_ROSE,
                format!("at most {} (the pin)", current.value),
                format!(
                    "the reading is {}, which is {} over the pin. The metric rose.{}",
                    row.value,
                    row.value - current.value,
                    row.measured_by
                        .as_deref()
                        .map(|m| format!(" Measured by: {m}."))
                        .unwrap_or_default()
                ),
            ));
        } else if row.value < current.value {
            // R2: good news mis-filed. Red, and the cure is one command.
            run.fail(Violation::fatal(
                location.clone(),
                RULE_STALE_PIN,
                format!("the pin to sit ON the measured value ({})", row.value),
                format!(
                    "the reading is {} and the pin is {}. The metric fell and the pin did \
                     not follow, so the next {} regressions are invisible headroom: a stale \
                     floor measures the next regression from the wrong place. Re-pin in the \
                     same change: vds burndown pin --id {} --to {} --because <what fell>",
                    row.value,
                    current.value,
                    current.value - row.value,
                    burndown.id,
                    row.value
                ),
            ));
        }

        // R11 then R10, both only where a deadline is declared: a record with
        // no deadline has nothing for a stale reading to defer.
        if burndown.deadline.is_some() && burndown.max_reading_age_days.is_none() {
            run.fail(Violation::fatal(
                location.clone(),
                RULE_NO_AGE_DECLARED,
                "a declared maxReadingAgeDays beside every deadline",
                "the record carries a deadline and no maximum reading age, so the only \
                 clock the deadline could be measured against is the reading it gates, and \
                 a deadline measured only against the input it gates is a deadline the \
                 subject sets (S-7C(5))."
                    .to_owned(),
            ));
        }
        if let (Some(_), Some(max_age), Some(witness)) =
            (&burndown.deadline, burndown.max_reading_age_days, &witness)
            && let Some(age) =
                crate::geometry::days_between(reading.taken_at.as_str(), witness.as_str())
            && age > i64::from(max_age)
        {
            run.fail(Violation::fatal(
                location.clone(),
                RULE_READING_AGE,
                format!("a reading no more than {max_age} day(s) behind the run's freshest independent input"),
                format!(
                    "the reading was taken {} and the freshest ledger this run read was \
                     generated {}, which is {age} days later - past the declared maximum of \
                     {max_age}. The deadline cannot be measured against a reading that \
                     stopped moving, and a subject that stops regenerating cannot thereby \
                     outlive the undertaking that reading witnesses. Regenerate: vds ledger \
                     burndown --from <reading.json>",
                    reading.taken_at.as_str(),
                    witness.as_str()
                ),
            ));
        }

        // R3: the deadline, measured from the reading's moment. UNDISTURBED by
        // order 14: that discipline is what keeps a dated fence from producing
        // different findings from identical inputs.
        if let Some(deadline) = &burndown.deadline
            && reading.taken_at.as_str() > deadline.as_str()
            && row.value > 0
        {
            run.fail(Violation::fatal(
                location.clone(),
                RULE_DEADLINE,
                format!("{} to be 0 by {}", burndown.metric, deadline.as_str()),
                format!(
                    "the reading, taken {}, is {} and the deadline was {}. The undertaking \
                     was not met, and the record keeps saying so until the metric is zero \
                     or the deadline is re-negotiated as a NEW record with the reason on it.",
                    reading.taken_at.as_str(),
                    row.value,
                    deadline.as_str()
                ),
            ));
        }
    }

    if let Some(reading) = reading.as_ref() {
        let claimed: Vec<&str> = records
            .iter()
            .filter(|r| r.value.status.is_enforceable())
            .map(|r| r.value.metric.as_str())
            .collect();
        for row in &reading.rows {
            if !claimed.contains(&row.metric.as_str()) && row.value > 0 {
                run.warn(Violation::fatal(
                    format!("reading [{}]", row.metric),
                    RULE_NO_READING,
                    format!("a burndown record pinning {:?}", row.metric),
                    format!(
                        "the reading measures {} at {} and no record pins it, so the number \
                         is measured and unowned.",
                        row.metric, row.value
                    ),
                ));
            }
        }
        if !reading.does_not_cover.is_empty() {
            run.note(format!(
                "[reading] the generator states it does NOT cover: {}",
                reading.does_not_cover.join("; ")
            ));
        }
    }

    if records.is_empty() {
        run.note(
            "[scope] no burndown is registered, so every row is skipped and this run is \
             vacuous. That is the honest state of a project with no pinned metrics, and it \
             is NOT evidence (VDS S-7(2)(4)).",
        );
    }

    run.finish(&ctx.capture_options()?, out)
}

#[cfg(test)]
mod proof_tests {
    use crate::testing::{Harness, run_kind};
    use vds_core::{EXIT_PASSED, EXIT_VACUOUS, EXIT_VIOLATION, ProofKind, ProofStatus};

    #[test]
    fn a_pin_sitting_on_the_measured_value_passes() {
        let h = Harness::new();
        h.burndown(
            "legacy_rule_blocks",
            None,
            &[("2026-07-01", 376), ("2026-07-20", 200)],
        );
        h.burndown_reading("2026-08-01", &[("legacy_rule_blocks", 200)]);
        let (outcome, text) = run_kind(&h, ProofKind::Burndown);
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
        assert_eq!(outcome.exit_code, EXIT_PASSED, "{text}");
        assert_eq!(outcome.rows_enforced, 1, "{text}");
    }

    /// THE failing-direction test VDS S-7(2)(2) requires, and the one the
    /// enforcement lock names: ANY increase over the pin is red.
    #[test]
    fn a_reading_above_the_pin_fails_on_any_increase() {
        let h = Harness::new();
        h.burndown("legacy_rule_blocks", None, &[("2026-07-20", 200)]);
        h.burndown_reading("2026-08-01", &[("legacy_rule_blocks", 201)]);
        let (outcome, text) = run_kind(&h, ProofKind::Burndown);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert_eq!(outcome.status, ProofStatus::Failed);
        assert!(text.contains("1 over the pin"), "{text}");
        assert_eq!(outcome.rows_enforced, 1, "{text}");
    }

    /// The clause that distinguishes a burndown from a ratchet: a decrease NOT
    /// re-pinned is red too, because a stale floor measures the next
    /// regression from the wrong place.
    #[test]
    fn a_decrease_that_was_not_repinned_fails_and_names_the_cure() {
        let h = Harness::new();
        h.burndown("legacy_rule_blocks", None, &[("2026-07-20", 200)]);
        h.burndown_reading("2026-08-01", &[("legacy_rule_blocks", 60)]);
        let (outcome, text) = run_kind(&h, ProofKind::Burndown);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("invisible headroom"), "{text}");
        assert!(
            text.contains("vds burndown pin"),
            "good news mis-filed must name the one-command cure: {text}"
        );
    }

    #[test]
    fn a_deadline_that_passed_with_the_metric_nonzero_fails() {
        let h = Harness::new();
        h.burndown(
            "legacy_rule_blocks",
            Some("2026-07-25"),
            &[("2026-07-20", 200)],
        );
        h.burndown_reading("2026-08-01", &[("legacy_rule_blocks", 200)]);
        let (outcome, text) = run_kind(&h, ProofKind::Burndown);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("deadline was 2026-07-25"), "{text}");
    }

    #[test]
    fn a_deadline_in_the_future_does_not_fail_a_true_pin() {
        let h = Harness::new();
        h.burndown(
            "legacy_rule_blocks",
            Some("2026-09-01"),
            &[("2026-07-20", 200)],
        );
        h.burndown_reading("2026-08-01", &[("legacy_rule_blocks", 200)]);
        let (outcome, text) = run_kind(&h, ProofKind::Burndown);
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
    }

    #[test]
    fn a_raised_pin_fails_whatever_the_reading_says() {
        let h = Harness::new();
        h.burndown(
            "legacy_rule_blocks",
            None,
            &[("2026-07-01", 200), ("2026-07-20", 376)],
        );
        h.burndown_reading("2026-08-01", &[("legacy_rule_blocks", 376)]);
        let (outcome, text) = run_kind(&h, ProofKind::Burndown);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("RAISED to 376"), "{text}");
    }

    #[test]
    fn a_pin_with_no_reading_is_unknown_and_never_a_pass() {
        let h = Harness::new();
        h.burndown("legacy_rule_blocks", None, &[("2026-07-20", 200)]);
        let (outcome, text) = run_kind(&h, ProofKind::Burndown);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("UNKNOWN"), "{text}");
        assert_eq!(outcome.rows_enforced, 0, "{text}");
    }

    /// The realistic attack that is not an attack: the reading is over the
    /// pin, somebody opens the YAML and lowers the number.
    #[test]
    fn an_edited_reading_is_refused_and_buys_no_pass() {
        let h = Harness::new();
        h.burndown("legacy_rule_blocks", None, &[("2026-07-20", 200)]);
        let path = h.burndown_reading("2026-08-01", &[("legacy_rule_blocks", 250)]);

        let (before, text) = run_kind(&h, ProofKind::Burndown);
        assert_eq!(before.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("50 over the pin"), "{text}");

        let original = std::fs::read_to_string(&path).unwrap();
        let edited = original.replace("value: 250", "value: 200");
        assert_ne!(edited, original, "the seed did not change the reading");
        std::fs::write(&path, &edited).unwrap();

        let (after, text) = run_kind(&h, ProofKind::Burndown);
        assert_eq!(after.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("edited after it was generated"), "{text}");
        assert!(!text.contains("50 over the pin"), "{text}");
        assert_eq!(after.rows_enforced, 0, "{text}");
    }

    #[test]
    fn two_enforceable_pins_for_one_metric_are_refused() {
        let h = Harness::new();
        h.burndown("legacy_rule_blocks", None, &[("2026-07-20", 200)]);
        h.burndown("legacy_rule_blocks", None, &[("2026-07-21", 100)]);
        h.burndown_reading("2026-08-01", &[("legacy_rule_blocks", 100)]);
        let (outcome, text) = run_kind(&h, ProofKind::Burndown);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("which pin governs"), "{text}");
        assert_eq!(outcome.rows_enforced, 0, "{text}");
    }

    #[test]
    fn a_measured_metric_nobody_pinned_warns_and_does_not_block() {
        let h = Harness::new();
        h.burndown(
            "legacy_rule_blocks",
            None,
            &[("2026-07-01", 376), ("2026-07-20", 200)],
        );
        h.burndown_reading(
            "2026-08-01",
            &[("legacy_rule_blocks", 200), ("hand_rolled_cards", 561)],
        );
        let (outcome, text) = run_kind(&h, ProofKind::Burndown);
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
        assert!(text.contains("measured and unowned"), "{text}");
    }

    // -- S-7C(5), [2026] VJS-CA-VDS 1 order 13 --------------------------------

    /// THE SEED the bench's Finding B names: a subject stops regenerating its
    /// reading and thereby outlives its own undertaking in silence. The pin
    /// still equals the reading and the deadline is still in the future BY THE
    /// READING'S OWN CLOCK, so every pre-order rule passes.
    #[test]
    fn a_reading_that_stopped_moving_cannot_defer_its_deadline() {
        let h = Harness::new();
        // The ledger is generated "now"; the reading is from a month before.
        h.screen("dash", &["Button"]);
        h.ledger();
        h.burndown_aged(
            "legacy_rule_blocks",
            Some("2099-01-01"),
            Some(7),
            &[("2026-07-01", 200)],
        );
        h.burndown_reading("2026-07-01", &[("legacy_rule_blocks", 200)]);
        let (outcome, text) = run_kind(&h, ProofKind::Burndown);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("stopped moving"), "{text}");
        assert!(
            text.contains("past the declared maximum of 7"),
            "the finding names the age and the bound: {text}"
        );
    }

    #[test]
    fn a_reading_inside_its_declared_age_passes() {
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        h.ledger();
        // A generous age, so the fixture's ledger-vs-reading gap is inside it.
        h.burndown_aged(
            "legacy_rule_blocks",
            Some("2099-01-01"),
            Some(36500),
            &[("2026-07-01", 200)],
        );
        h.burndown_reading("2026-07-01", &[("legacy_rule_blocks", 200)]);
        let (outcome, text) = run_kind(&h, ProofKind::Burndown);
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
    }

    /// A deadline with no declared age has only the clock its own subject
    /// winds, which is the defect stated as a rule.
    #[test]
    fn a_deadline_with_no_declared_reading_age_is_refused() {
        let h = Harness::new();
        h.burndown_aged(
            "legacy_rule_blocks",
            Some("2099-01-01"),
            None,
            &[("2026-07-20", 200)],
        );
        h.burndown_reading("2026-08-01", &[("legacy_rule_blocks", 200)]);
        let (outcome, text) = run_kind(&h, ProofKind::Burndown);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("a deadline the subject sets"), "{text}");
    }

    /// A record with NO deadline has nothing to defer, so neither rule fires:
    /// the amendment must not turn every undated burndown red.
    #[test]
    fn a_record_with_no_deadline_needs_no_reading_age() {
        let h = Harness::new();
        h.burndown_aged("legacy_rule_blocks", None, None, &[("2026-07-20", 200)]);
        h.burndown_reading("2026-08-01", &[("legacy_rule_blocks", 200)]);
        let (outcome, text) = run_kind(&h, ProofKind::Burndown);
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
    }

    #[test]
    fn a_project_with_no_burndown_is_vacuous_and_says_it_is_not_evidence() {
        let h = Harness::new();
        let (outcome, text) = run_kind(&h, ProofKind::Burndown);
        assert_eq!(outcome.status, ProofStatus::Vacuous, "{text}");
        assert_eq!(outcome.exit_code, EXIT_VACUOUS, "{text}");
        assert!(text.contains("NOT evidence"), "{text}");
    }
}
