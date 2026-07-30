//! The `states` proof.
//!
//! VDS S-7(5): "every required state of every registered component is drawn."
//! VDS S-5(3) fixes the nine states and [`State`] is an enum, so a tenth is
//! unrepresentable rather than merely invalid and this proof has nothing to say
//! about which states exist. It has one thing to say about each register record:
//! the states the record REQUIRES are the states the record DRAWS.
//!
//! One fatal rule and one informational one:
//!
//!   R1  a record requires a state and does not draw it. The finding NAMES the
//!       states. "3 missing" sends an author to go and look; "hover, focus,
//!       disabled" tells them what to draw.
//!   I1  a record at `built` or `verified` draws a required state it has not
//!       built. That gap is the `parity` proof's to fail on (VDS S-7(5)), so it
//!       is recorded here rather than enforced here, and recording it means
//!       nobody has to remember it exists. It is captured on the proof record,
//!       because a finding that only ever reaches a terminal is a finding nobody
//!       reads twice.
//!
//! A row is one register record. Two statuses are counted and never enforced. A
//! `proposed` record has nothing drawn yet by construction, so enforcing it
//! would fail every new registration and teach the author to skip the stage that
//! VDS S-5(4) makes mandatory. A `retired` record is a tombstone kept forever
//! (VDS S-9(6)(3)), and a tombstone is not a component anyone is drawing.
//!
//! What this proof does NOT establish is stated on every record it captures,
//! because silent narrowing is the defect VDS exists to catch. `states.drawn` is
//! the register's own claim. Confirming it against the decided-target Figma file
//! is a network read, and VDS S-7(2)(1) forbids one inside a proof, so a pass
//! establishes that the contract is complete and never that a frame exists.
//!
//! This proof reads state NAMES, lifecycle STATUSES and component IDENTIFIERS.
//! It reads no design value (VDS S-2(2)), and it could not: what a state looks
//! like lives in a named record VDS reads and does not own (VDS S-2(3)).

use std::io::Write;

use vds_core::{ProofKind, Result, State, Status, Violation};

use crate::ProofContext;
use crate::index::RegisterIndex;
use crate::run::{Outcome, Verdict};

pub const GATE: &str = "crates/vds-proof/src/states.rs";

const RULE_NOT_DRAWN: &str =
    "VDS S-7(5) states R1: every required state of every registered component is drawn";
const RULE_DRAWN_NOT_BUILT: &str = "VDS S-7(5) states I1: a required state is drawn and not built, which the `parity` proof \
     is the gate for";

/// A `proposed` record is counted and not enforced.
///
/// A stable machine key and not a sentence: it becomes a count in
/// `rows_skipped_reasons`, and a per-row sentence would make every count one.
const SKIP_PROPOSED: &str = "proposed_nothing_drawn_by_construction";
const SKIP_RETIRED: &str = "retired_tombstone_vds_s9_6_3";
const SKIP_NO_REQUIREMENT: &str = "record_declares_no_required_state";

const RULE_DRAWN_NOT_IN_FILE: &str = "VDS S-5(5) states R2: a state the register claims is drawn is drawn in the \
     decided-target file";

/// What this run reached, which now depends on whether a ledger was there.
///
/// The old note said this proof could only ever read the register's own claim,
/// because confirming it needs the decided-target file and VDS S-7(2)(1) forbids
/// a network call inside a proof. The first half of that is still true and the
/// conclusion no longer follows: `vds figma pull` generates a LEDGER out of
/// band, exactly as the pin is generated out of band, and a ledger on disk is
/// something this proof may read offline.
///
/// So there are three modes and the record says which one it ran in. A reader
/// who cannot tell a measured `states.drawn` from a claimed one has been handed
/// a pass that means two different things.
const REACH_MEASURED: &str = "[reach] `states.drawn` was MEASURED against the figma ledger, not taken on the register's \
     word. A record claiming a state the decided-target file does not draw is a finding here \
     (R2), which is the claim VDS S-5(5) says a hand-maintained register decays into. What \
     this still does not establish is that a drawn frame is GOOD: taste is reserved to the \
     Principal (VDS S-1(6)).";

const REACH_NO_LEDGER: &str = "[reach] no figma ledger is present, so `states.drawn` is the register's own claim and this \
     run did not check it. A pass establishes that the contract is complete, never that a \
     frame exists in the decided-target file. Generate a ledger with `vds figma pull` and \
     re-run to have the claim measured instead: it is a network read, which is why it happens \
     out of band and not inside this proof (VDS S-7(2)(1)).";

const REACH_STALE_LEDGER: &str = "[reach] a figma ledger is present and this run did NOT rely on it, because it could not be \
     established as current. `states.drawn` is therefore the register's own claim here, exactly \
     as if no ledger existed. Relying on a ledger that cannot be shown current would report \
     the decided-target file as it was at some unknown past moment and call that a \
     measurement.";

const TASTE_NOTE: &str = "[taste] whether a drawn state looks right is reserved to the Principal (VDS S-1(6)). This \
     proof checks that a required state is drawn, never that it is good.";

const PARITY_NOTE: &str = "[parity] a required state that is drawn and not built is recorded here as an informational \
     finding and never fails this gate; the `parity` proof is the gate for that gap \
     (VDS S-7(5)). Informational findings are captured on the proof record and are not printed, \
     so the record is where they are read.";

const EMPTY_REGISTER_NOTE: &str = "[register] the register holds no record, so no row can be enforced and this run is \
     vacuous. A states proof over an empty register establishes nothing about any component \
     (VDS S-7(2)(4)).";

pub fn run(ctx: &ProofContext, out: &mut dyn Write) -> Result<Outcome> {
    let project = ctx.project;
    let mut run = ctx.new_run(ProofKind::States, GATE);
    run.input_file(&project.config_path)?;

    // Read through the same index every other proof reads through. An ambiguous
    // register, two records on one identifier or one code coordinate, is refused
    // as a precondition rather than half-proven (VDS S-4(4)), so two proofs
    // never disagree about what the register contains.
    let index = RegisterIndex::build(&ctx.store())?;
    for record in index.records() {
        run.input_file(&record.path)?;
    }

    // The ledger, where there is one that can be relied on. An absent ledger is
    // a NARROWING and never an error: a project with no Figma access still gets
    // the register-internal check, and the note says which of the two it got.
    let ledger = match vds_figma::pull::read(&ctx.store())? {
        None => {
            run.note(REACH_NO_LEDGER);
            None
        }
        Some(ledger) => match vds_figma::ledger::check_fresh(&ledger, None) {
            Ok(()) => {
                run.note(REACH_MEASURED);
                run.input_named("<figma ledger content>", ledger.compute_content_digest()?);
                Some(ledger)
            }
            Err(why) => {
                run.note(format!(
                    "{REACH_STALE_LEDGER} The reason it was not relied on: {why}"
                ));
                None
            }
        },
    };
    run.note(TASTE_NOTE);
    run.note(PARITY_NOTE);
    if index.is_empty() {
        run.note(EMPTY_REGISTER_NOTE);
    }

    let mut without_requirement: u64 = 0;

    for located in index.records() {
        let record = &located.value;
        let location = format!(
            "{} <{} {}>",
            project.rel(&located.path),
            record.id,
            record.name
        );

        // Written out rather than matched with a wildcard: the lifecycle is
        // closed by VDS S-5(4), and a wildcard would silently enforce whatever
        // an eighth status turned out to mean.
        match record.status {
            Status::Proposed => {
                run.row(Verdict::Skipped(SKIP_PROPOSED));
                continue;
            }
            Status::Retired => {
                run.row(Verdict::Skipped(SKIP_RETIRED));
                continue;
            }
            Status::Designed
            | Status::Registered
            | Status::Built
            | Status::Verified
            | Status::Deprecated => {}
        }

        if record.states.required.is_empty() {
            // A row that cannot fail is not a row that was checked. Counting it
            // as enforced is the arithmetic half of the [2026] VJS-CC-OPBOX 3 D3
            // defect: `rows_enforced` rises and nothing was established
            // (VDS S-7(2)(4)).
            without_requirement += 1;
            run.row(Verdict::Skipped(SKIP_NO_REQUIREMENT));
            continue;
        }

        run.row(Verdict::Enforced);

        let not_drawn = record.required_not_drawn();
        if !not_drawn.is_empty() {
            run.fail(Violation::fatal(
                location.clone(),
                RULE_NOT_DRAWN,
                format!(
                    "{} draws every state it requires, so states.drawn contains {}",
                    record.id,
                    named(&in_specification_order(&record.states.required))
                ),
                format!(
                    "{} is at status {} and states.drawn omits {}",
                    record.id,
                    record.status,
                    named(&not_drawn)
                ),
            ));
        }

        // R2. The register's claim, measured against the file that decides it.
        //
        // This is the direction VDS S-5(5) says a hand-maintained register
        // decays in: `states.drawn` is edited by hand, nothing read the Figma
        // file, and over months the record describes frames that were never
        // drawn or were drawn and removed. `required_not_drawn` above catches a
        // record that admits the gap; this catches one that does not.
        if let Some(ledger) = &ledger {
            match ledger.row(&record.id) {
                Some(row) if row.resolved => {
                    let claimed_not_in_file: Vec<vds_core::State> = record
                        .states
                        .drawn
                        .iter()
                        .filter(|state| !row.states_drawn.contains(state))
                        .copied()
                        .collect();
                    if !claimed_not_in_file.is_empty() {
                        run.fail(Violation::fatal(
                            location.clone(),
                            RULE_DRAWN_NOT_IN_FILE,
                            format!(
                                "every state {} claims as drawn is drawn on its node in the \
                                 decided-target file, measured from that node's variants",
                                record.id
                            ),
                            format!(
                                "{} claims {} as drawn and the node draws {}. A register that \
                                 says a state exists in the file when it does not is the decay \
                                 VDS S-5(5) describes, and every gate downstream of \
                                 `states.drawn` has been reading it.",
                                record.id,
                                named(&in_specification_order(&claimed_not_in_file)),
                                if row.states_drawn.is_empty() {
                                    "none".to_owned()
                                } else {
                                    named(&in_specification_order(&row.states_drawn))
                                }
                            ),
                        ));
                    }
                }
                // A record with no node, or a node the ledger could not resolve,
                // is `reconciliation` limb (c)'s to report and not this proof's.
                // Reporting it here as well would put one defect on two records
                // and make a reader think there were two.
                _ => {}
            }
        }

        // Only at `built` or `verified`. Before those a record claims nothing
        // about a built counterpart, and a finding against a claim nobody made
        // is noise that trains a reader to skip the section where the real ones
        // are.
        if matches!(record.status, Status::Built | Status::Verified) {
            let drawn_not_built: Vec<State> = record
                .required_not_built()
                .into_iter()
                // A state that is required and not drawn already failed R1
                // above. Reporting it again here would make one defect look
                // like two.
                .filter(|state| record.states.drawn.contains(state))
                .collect();
            if !drawn_not_built.is_empty() {
                run.inform(Violation::fatal(
                    location,
                    RULE_DRAWN_NOT_BUILT,
                    format!(
                        "{} builds every required state it has drawn, so states.built contains {}",
                        record.id,
                        named(&drawn_not_built)
                    ),
                    format!(
                        "{} is at status {} and states.built omits {}, which are required and \
                         drawn",
                        record.id,
                        record.status,
                        named(&drawn_not_built)
                    ),
                ));
            }
        }
    }

    if without_requirement > 0 {
        run.note(format!(
            "[contract] {without_requirement} register records declare no required state at \
             all, so there is nothing about them for this proof to enforce and each is counted \
             and skipped. This proof cannot tell a component that genuinely requires no state \
             from a contract nobody filled in; a contract that disagrees with the code is \
             `reconciliation`'s to find (VDS S-5(6))."
        ));
    }

    run.finish(&ctx.capture_options()?, out)
}

/// The states, named and never counted.
///
/// A count sends an author to go and look. A list of names tells them what to
/// draw, and two records missing different states then produce two different
/// messages rather than the same number twice.
fn named(states: &[State]) -> String {
    states
        .iter()
        .map(|state| state.as_str())
        .collect::<Vec<&str>>()
        .join(", ")
}

/// A state set in the order VDS S-5(3) fixes, whatever order the record wrote it
/// in.
///
/// Two records declaring the same set in different orders must produce the same
/// message, or the evidence digest moves when a YAML list is reordered and no
/// contract changed (VDS S-7(2)(1)).
fn in_specification_order(states: &[State]) -> Vec<State> {
    State::ALL
        .into_iter()
        .filter(|state| states.contains(state))
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::testing::{Harness, run_kind};
    use vds_core::{
        ComponentId, EXIT_PASSED, EXIT_VACUOUS, EXIT_VIOLATION, ProofKind, ProofStatus, Severity,
        State, StateContract, Status,
    };

    /// One register record at `status` whose three state buckets are exactly as
    /// given. Distinct names, because two records on one code coordinate are a
    /// fail-closed error and not the thing under test here.
    fn with_states(
        h: &Harness,
        name: &str,
        status: Status,
        required: &[State],
        drawn: &[State],
        built: &[State],
    ) -> ComponentId {
        let id = h.register(name, status);
        h.amend(&id, |record| {
            record.states = StateContract {
                required: required.to_vec(),
                drawn: drawn.to_vec(),
                built: built.to_vec(),
            };
        });
        id
    }

    #[test]
    fn a_record_that_draws_every_state_it_requires_passes() {
        let h = Harness::new();
        with_states(
            &h,
            "Button",
            Status::Registered,
            &[State::Default, State::Hover, State::Focus],
            &[State::Default, State::Hover, State::Focus],
            &[],
        );
        let (outcome, text) = run_kind(&h, ProofKind::States);
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
        assert_eq!(outcome.exit_code, EXIT_PASSED);
        assert_eq!(outcome.rows_enforced, 1);
    }

    /// The failing-direction test VDS S-7(2)(2) requires, and the one the
    /// enforcement lock names. It seeds a registered record that requires
    /// `hover` and `focus` and draws neither, and asserts the non-zero exit.
    #[test]
    fn states_fails_on_a_required_state_that_is_not_drawn() {
        let h = Harness::new();
        with_states(
            &h,
            "Button",
            Status::Registered,
            &[State::Default, State::Hover, State::Focus],
            &[State::Default],
            &[],
        );
        let (outcome, text) = run_kind(&h, ProofKind::States);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert_eq!(outcome.status, ProofStatus::Failed);
        assert!(text.contains("omits hover, focus"), "{text}");
        assert!(text.contains(".vds/register/CMP-0001.yaml"), "{text}");
    }

    /// VDS S-7(5) is about which states are drawn, so the finding says which,
    /// and says it in the order VDS S-5(3) fixes rather than the order the YAML
    /// happened to use.
    #[test]
    fn the_finding_names_the_missing_states_in_specification_order() {
        let h = Harness::new();
        with_states(
            &h,
            "Button",
            Status::Registered,
            &[State::Success, State::Hover, State::Default],
            &[State::Default],
            &[],
        );
        let (_, text) = run_kind(&h, ProofKind::States);
        assert!(text.contains("omits hover, success"), "{text}");
        assert!(
            text.contains("states.drawn contains default, hover, success"),
            "expected says what right would have looked like: {text}"
        );
        assert!(
            !text.contains("2 states"),
            "a count sends an author to go and look: {text}"
        );
    }

    /// A `proposed` record has nothing drawn by construction. This one would
    /// fail R1 if it were enforced, which is what makes the carve-out real
    /// rather than decorative.
    #[test]
    fn a_proposed_record_is_counted_and_never_enforced() {
        let h = Harness::new();
        with_states(
            &h,
            "Sketch",
            Status::Proposed,
            &[State::Default, State::Hover],
            &[],
            &[],
        );
        let (outcome, text) = run_kind(&h, ProofKind::States);
        assert_eq!(outcome.status, ProofStatus::Vacuous, "{text}");
        assert_eq!(outcome.rows_considered, 1);
        assert_eq!(outcome.rows_enforced, 0);
        assert!(text.contains(super::SKIP_PROPOSED), "{text}");
    }

    /// VDS S-9(6)(3): a retired record is a tombstone kept forever, and a
    /// tombstone is not a component anyone is drawing.
    #[test]
    fn a_retired_tombstone_is_counted_and_never_enforced() {
        let h = Harness::new();
        with_states(
            &h,
            "OldChip",
            Status::Retired,
            &[State::Default, State::Hover],
            &[],
            &[],
        );
        let (outcome, text) = run_kind(&h, ProofKind::States);
        assert_eq!(outcome.status, ProofStatus::Vacuous, "{text}");
        assert_eq!(outcome.rows_enforced, 0);
        assert!(text.contains(super::SKIP_RETIRED), "{text}");
    }

    /// A deprecated component is still on screen until it drains
    /// (VDS S-9(6)(2)), so its drawn states are still enforceable.
    #[test]
    fn a_deprecated_record_is_still_enforced_because_it_is_still_drawn() {
        let h = Harness::new();
        with_states(
            &h,
            "OldButton",
            Status::Deprecated,
            &[State::Default, State::Focus],
            &[State::Default],
            &[],
        );
        let (outcome, text) = run_kind(&h, ProofKind::States);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert_eq!(outcome.rows_enforced, 1);
        assert!(text.contains("omits focus"), "{text}");
    }

    #[test]
    fn a_record_declaring_no_required_state_is_counted_and_never_enforced() {
        let h = Harness::new();
        with_states(&h, "Divider", Status::Registered, &[], &[], &[]);
        let (outcome, text) = run_kind(&h, ProofKind::States);
        assert_eq!(
            outcome.status,
            ProofStatus::Vacuous,
            "a row that cannot fail is not a row that was checked: {text}"
        );
        assert_eq!(outcome.rows_considered, 1);
        assert_eq!(outcome.rows_enforced, 0);
        assert!(text.contains(super::SKIP_NO_REQUIREMENT), "{text}");
        assert!(text.contains("[contract]"), "{text}");
    }

    #[test]
    fn an_empty_register_is_vacuous_and_never_passed() {
        let h = Harness::new();
        let (outcome, text) = run_kind(&h, ProofKind::States);
        assert_eq!(outcome.status, ProofStatus::Vacuous, "{text}");
        assert_eq!(outcome.exit_code, EXIT_VACUOUS);
        assert!(!text.contains("PASS:"), "no PASS beside a vacuity: {text}");
        assert!(text.contains("[register]"), "{text}");
    }

    /// I1. The gap belongs to `parity`, so it does not fail this gate, and it is
    /// written onto the record rather than printed, because a finding that only
    /// ever reaches a terminal is a finding nobody reads twice.
    #[test]
    fn a_built_record_that_has_not_built_a_drawn_state_is_captured_as_informational() {
        let h = Harness::new();
        with_states(
            &h,
            "Button",
            Status::Built,
            &[State::Default, State::Focus],
            &[State::Default, State::Focus],
            &[State::Default],
        );
        let (outcome, text) = run_kind(&h, ProofKind::States);
        assert_eq!(outcome.exit_code, EXIT_PASSED, "{text}");
        assert_eq!(outcome.status, ProofStatus::Passed);

        let record = h.last_proof(ProofKind::States);
        assert_eq!(record.violations.len(), 1, "{:?}", record.violations);
        assert_eq!(record.violations[0].severity, Severity::Informational);
        assert!(
            record.violations[0]
                .actual
                .contains("states.built omits focus"),
            "{:?}",
            record.violations[0]
        );
        assert!(
            record.violations[0].location.contains("CMP-0001"),
            "a finding that names no file is half a finding: {:?}",
            record.violations[0]
        );
    }

    #[test]
    fn a_registered_record_with_nothing_built_yet_is_not_reported_as_a_build_gap() {
        let h = Harness::new();
        with_states(
            &h,
            "Button",
            Status::Registered,
            &[State::Default, State::Focus],
            &[State::Default, State::Focus],
            &[],
        );
        let (outcome, text) = run_kind(&h, ProofKind::States);
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
        assert!(
            h.last_proof(ProofKind::States).violations.is_empty(),
            "a record at registered claims nothing about a built counterpart, and a finding \
             against a claim nobody made trains a reader to skip the section"
        );
    }

    #[test]
    fn a_state_that_is_neither_drawn_nor_built_is_reported_once_and_as_fatal() {
        let h = Harness::new();
        with_states(
            &h,
            "Button",
            Status::Verified,
            &[State::Default, State::Focus],
            &[State::Default],
            &[State::Default],
        );
        let (outcome, text) = run_kind(&h, ProofKind::States);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        let record = h.last_proof(ProofKind::States);
        assert_eq!(
            record.violations.len(),
            1,
            "one undrawn state reported twice makes a reader think there are two defects: {:?}",
            record.violations
        );
        assert_eq!(record.violations[0].severity, Severity::Fatal);
    }

    #[test]
    fn every_record_is_one_row_and_the_counts_add_up() {
        let h = Harness::new();
        with_states(
            &h,
            "Button",
            Status::Registered,
            &[State::Default],
            &[State::Default],
            &[],
        );
        with_states(&h, "Card", Status::Proposed, &[State::Default], &[], &[]);
        with_states(&h, "OldChip", Status::Retired, &[State::Default], &[], &[]);
        let (outcome, text) = run_kind(&h, ProofKind::States);
        assert_eq!(outcome.rows_considered, 3, "{text}");
        assert_eq!(outcome.rows_enforced, 1);

        let record = h.last_proof(ProofKind::States);
        let skipped: u64 = record.rows_skipped_reasons.values().sum();
        assert_eq!(record.rows_considered, record.rows_enforced + skipped);
    }

    /// A precondition failure is exit 2 and means the proof DID NOT RUN. It is
    /// never a pass and never a violation: a reader of a partial register is
    /// reading a register that says something the directory does not.
    #[test]
    fn an_unreadable_register_record_is_a_precondition_failure_and_not_a_pass() {
        let h = Harness::new();
        with_states(
            &h,
            "Button",
            Status::Registered,
            &[State::Default],
            &[State::Default],
            &[],
        );
        h.write(".vds/register/CMP-0002.yaml", "status: registered\n");
        let error = h.run_kind_err(ProofKind::States);
        assert!(error.to_string().contains("CMP-0002"), "{error}");
    }

    #[test]
    fn the_run_records_what_it_cannot_reach() {
        let h = Harness::new();
        with_states(
            &h,
            "Button",
            Status::Registered,
            &[State::Default],
            &[State::Default],
            &[],
        );
        run_kind(&h, ProofKind::States);
        let record = h.last_proof(ProofKind::States);
        for marker in ["[reach]", "[taste]", "[parity]"] {
            assert!(
                record.notes.iter().any(|note| note.contains(marker)),
                "{marker} is missing from {:?}",
                record.notes
            );
        }
        assert!(
            record
                .notes
                .iter()
                .any(|note| note.contains("never that a frame exists in the decided-target file")),
            "{:?}",
            record.notes
        );
    }

    /// R2, and the direction VDS S-5(5) says a hand-maintained register decays
    /// in: `states.drawn` is edited by hand, nothing read the Figma file, and
    /// over months the record describes frames that were never drawn.
    ///
    /// Until a ledger existed this proof could only read the register's own
    /// claim, and it said so on every record it captured. It can now measure it.
    #[test]
    fn a_state_the_register_claims_as_drawn_and_the_file_does_not_draw_is_a_finding() {
        let h = Harness::new();
        let id = h.register("Button", Status::Registered);
        h.amend(&id, |record| {
            record.states.required = vec![State::Default, State::Hover];
            record.states.drawn = vec![State::Default, State::Hover];
            record.figma = Some(vds_core::FigmaNode {
                file_key: "KEY".into(),
                node_id: "12:34".into(),
                captured_at: vds_core::Timestamp::fixed(2026, 7, 25, 10, 0, 0),
            });
        });

        // Without a ledger the claim stands unchecked, which is the old
        // behaviour and is still correct where there is nothing to check it
        // against.
        let (before, text) = run_kind(&h, ProofKind::States);
        assert_eq!(before.exit_code, EXIT_PASSED, "{text}");

        // With one that draws only `default`, the claim about `hover` is a
        // finding.
        write_ledger(&h, &id, &[State::Default]);
        let (after, text) = run_kind(&h, ProofKind::States);
        assert_eq!(
            after.exit_code, EXIT_VIOLATION,
            "a register claiming a state the file does not draw passed: {text}"
        );
        assert!(text.contains("R2"), "{text}");
        assert!(text.contains("hover"), "{text}");
        assert!(text.contains("decay"), "{text}");
    }

    /// And the agreeing direction, so the measurement is not simply a way of
    /// failing everything.
    #[test]
    fn a_claim_the_file_supports_passes() {
        let h = Harness::new();
        let id = h.register("Button", Status::Registered);
        h.amend(&id, |record| {
            record.states.required = vec![State::Default, State::Hover];
            record.states.drawn = vec![State::Default, State::Hover];
            record.figma = Some(vds_core::FigmaNode {
                file_key: "KEY".into(),
                node_id: "12:34".into(),
                captured_at: vds_core::Timestamp::fixed(2026, 7, 25, 10, 0, 0),
            });
        });
        write_ledger(&h, &id, &[State::Default, State::Hover, State::Focus]);

        let (outcome, text) = run_kind(&h, ProofKind::States);
        assert_eq!(outcome.exit_code, EXIT_PASSED, "{text}");
        let record = h.last_proof(ProofKind::States);
        assert!(
            record
                .notes
                .iter()
                .any(|note| note.contains("was MEASURED against the figma ledger")),
            "the record has to say which mode the run was in, or a pass means two different \
             things: {:?}",
            record.notes
        );
    }

    /// A ledger that cannot be shown current is not relied on at all.
    ///
    /// Trusting one would resolve the register's claims against the file as it
    /// was at some unknown past moment and report that as a measurement.
    #[test]
    fn a_ledger_that_cannot_be_shown_current_is_not_relied_on() {
        let h = Harness::new();
        let id = h.register("Button", Status::Registered);
        h.amend(&id, |record| {
            record.states.required = vec![State::Default, State::Hover];
            record.states.drawn = vec![State::Default, State::Hover];
        });
        write_ledger(&h, &id, &[State::Default]);

        // Hand-edit the ledger, which is the thing a generated inventory is
        // never supposed to have done to it (VDS S-4(2)).
        let path = h.root().join(".vds/ledgers/figma.yaml");
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, text.replace("resolved: true", "resolved: false")).unwrap();

        let (outcome, text) = run_kind(&h, ProofKind::States);
        assert_eq!(outcome.exit_code, EXIT_PASSED, "{text}");
        let record = h.last_proof(ProofKind::States);
        assert!(
            record
                .notes
                .iter()
                .any(|note| note.contains("did NOT rely on it")),
            "{:?}",
            record.notes
        );
    }

    /// A figma ledger naming one component, drawing the states given.
    fn write_ledger(h: &Harness, id: &vds_core::ComponentId, drawn: &[State]) {
        use vds_figma::ledger::{FigmaLedger, FigmaNodeRow, LEDGER_SCHEMA_VERSION};

        let ledger = FigmaLedger {
            schema_version: LEDGER_SCHEMA_VERSION,
            generated_at: vds_core::Timestamp::fixed(2026, 7, 25, 11, 0, 0),
            generated_by: "vds figma pull".into(),
            file_key: "KEY".into(),
            file_version: "1".into(),
            file_name: "decided".into(),
            nodes: vec![FigmaNodeRow {
                component_id: id.clone(),
                node_id: "12:34".into(),
                resolved: true,
                figma_name: Some("Button".into()),
                is_component_set: true,
                variant_properties: Default::default(),
                states_drawn: drawn.to_vec(),
                unresolved_because: None,
            }],
            unclaimed: vec![],
            notes: vec![],
            content_digest: vds_core::Digest::of_text("placeholder"),
        };
        let ledger = FigmaLedger {
            content_digest: ledger.compute_content_digest().unwrap(),
            ..ledger
        };
        let path = h.root().join(".vds/ledgers/figma.yaml");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, serde_yaml::to_string(&ledger).unwrap()).unwrap();
    }
}
