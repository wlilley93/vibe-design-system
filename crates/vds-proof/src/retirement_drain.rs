//! The `retirement_drain` proof. What makes retirement reachable at all.
//!
//! VDS S-7(5): "a component proposed for retirement has zero remaining
//! consumers". VDS S-9(6)(2) fixes how that is established: demand must reach
//! zero, MEASURED by a named command and not asserted, and while any consumer
//! remains retirement is refused. Without this proof the third phase of
//! VDS S-9(6) has no drain proof to cite, `retired` is a status no record can
//! lawfully enter, and the register is left with adding as its only lawful move,
//! which is the state VDS S-9(1) says rots.
//!
//! A row is one register record at status `deprecated`, because those are the
//! components proposed for retirement. Every other status is counted and skipped
//! under a named reason.
//!
//! Two fatal rules and three reports:
//!
//!   D1  a deprecated component that a route still consumes. The finding LISTS
//!       THE ROUTES, because "3 consumers remain" names no work anyone can pick
//!       up, and the work is migrating each route by name.
//!   D2  VDS S-9(7): a deprecated record whose `supersededBy` names a component
//!       that is not itself registered or later. Deprecating toward a component
//!       that does not yet exist is how a library ends up with two incomplete
//!       halves and no whole. `supersededBy: null` is a withdrawal outright and
//!       is correct, not an omission.
//!   W1  the successor is itself deprecated or retired, so the migration target
//!       is on its way out too. Reported and not fatal: VDS S-9(7) says
//!       "registered or later", and deprecated IS later on the VDS S-5(4) path.
//!       Reading a narrower rule into the clause would be VDS settling a point
//!       the clause does not settle, which VDS S-1(2) forbids.
//!   W2  `demand.routes` disagrees with the measurement this run took. The
//!       stored figure is an unmeasured opinion the moment the surface moves, and
//!       the drain is decided on the measurement and never on the figure. Not
//!       fatal, because blocking retirement on the stored number would make that
//!       opinion load-bearing, which is the defect it is reported for.
//!   W3  a deprecated record with no code counterpart, whose demand therefore
//!       cannot be measured at all. Named individually, because a skip count
//!       says how many rows went unchecked and never which ones.
//!
//! VDS S-9(9) is RESERVED (SUBMISSION-VDS-004): whether a component may be
//! retired against a forced-drain deadline while consumers remain is unsettled.
//! Until it is answered the drain condition is ABSOLUTE, and this proof
//! implements no deadline, no override and no grace period. There is no option
//! anywhere in this file that turns D1 into a warning.
//!
//! Staleness of `demand.measured_at` against the ledger's generation is
//! reconciliation's row (VDS S-5(7)), and this proof does not read
//! `generated_at` at all: a finding derived from it would appear and disappear
//! with a no-op regeneration of the ledger, which is the determinism limb of
//! VDS S-7(2)(1) broken by an irrelevance. W2 compares the FIGURE against the
//! measurement instead, which is the stronger statement anyway.
//!
//! This proof reads component NAMES, import PATHS, export NAMES, lifecycle
//! STATUSES and route counts. It reads no design value (VDS S-2(2)).

use std::io::Write;

use vds_core::{ComponentRecord, ProofKind, Result, Status, Violation};

use crate::index::RegisterIndex;
use crate::run::{Outcome, ProofRun, Verdict};
use crate::ProofContext;

pub const GATE: &str = "crates/vds-proof/src/retirement_drain.rs";

const RULE_DRAIN: &str =
    "VDS S-9(6)(2) retirement_drain D1: demand must reach zero, measured and not asserted, and \
     while any consumer remains retirement is refused";
const RULE_SUCCESSOR: &str =
    "VDS S-9(7) retirement_drain D2: the successor named in supersededBy must itself be \
     registered or later before the predecessor may be deprecated";
const RULE_SUCCESSOR_LEAVING: &str =
    "VDS S-9(7) retirement_drain W1: the successor is itself on its way out";
const RULE_DEMAND_DISAGREES: &str =
    "VDS S-5(7) retirement_drain W2: demand is measured, never estimated";
const RULE_UNMEASURABLE: &str =
    "VDS S-9(6)(2) retirement_drain W3: a drain must be measured, so a record whose demand \
     cannot be measured cannot be drained";

pub const RESERVED_NOTE: &str =
    "relies on VDS S-9(9) RESERVED (SUBMISSION-VDS-004): whether a component may be retired \
     against a forced-drain deadline while consumers remain is unsettled, so the drain \
     condition is ABSOLUTE here and no deadline, override or grace period overrides a non-zero \
     measured demand. Any warrant citing this proof must record that reliance in its `reserved` \
     array.";

/// What the measurement does NOT cover, recorded on every run.
///
/// AGENTS.md forbids the claim "no unregistered component anywhere", and the
/// same discipline binds a drain: a measured zero is zero over a declared
/// surface, and a reader who is not told which surface will read it as zero
/// everywhere. Naming the three consumers this cannot see is what keeps the
/// narrowing from being silent.
const REACH_NOTE: &str =
    "[reach] demand is measured from the screens ledger and from nothing else, so a zero here \
     is zero consumers on the DECLARED SURFACE ([surface] screen_globs) and not zero in the \
     repository. Three kinds of consumer are outside its reach and are NOT counted: a \
     component consumed by another component rather than by a screen; a route that reaches the \
     component through a re-export or an aliased import path rather than the one the record \
     names; and a dynamic or computed import the scanner cannot resolve to a module. A warrant \
     citing this proof is bounded by the ledger content digest recorded on it and must say so.";

const NO_SCREENS_NOTE: &str =
    "[surface] the screens ledger declares no screens, so a zero-consumer measurement here \
     would be the ABSENCE of a measurement rather than one, and a drain certified against it \
     would certify nothing. Every drain row is skipped and this run is vacuous \
     (VDS S-7(2)(4)).";

pub fn run(ctx: &ProofContext, out: &mut dyn Write) -> Result<Outcome> {
    let project = ctx.project;
    let mut run = ctx.new_run(ProofKind::RetirementDrain, GATE);
    run.input_file(&project.config_path)?;

    // An absent or stale ledger is a precondition failure and exit 2. A drain
    // measured against a ledger that no longer describes the screens reports the
    // demand of a surface that is gone, and a retirement granted on it is
    // granted on nothing.
    let ledger = vds_scan::load_fresh(project)?;
    // The ledger's CONTENT digest, not its file digest: the file carries
    // `generated_at`, which moves on a no-op regeneration and would move this
    // proof's evidence digest with it (VDS S-7(2)(1)).
    run.input_named("<screens ledger content>", ledger.content_digest.clone());

    let index = RegisterIndex::build(&ctx.store())?;
    for record in index.records() {
        run.input_file(&record.path)?;
    }

    run.note(RESERVED_NOTE);
    run.note(REACH_NOTE);

    let no_screens = ledger.screens.is_empty();
    if no_screens {
        run.note(NO_SCREENS_NOTE);
    }

    for located in index.records() {
        let record: &ComponentRecord = &located.value;
        let location = format!(
            "{} ({} {:?})",
            project.rel(&located.path),
            record.id,
            record.name
        );

        if record.status != Status::Deprecated {
            run.row(Verdict::Skipped(match record.status {
                // VDS S-9(8) puts a retired record's consumers on composition's
                // row, where the presence of the code is itself the defect.
                // Draining it again would be a second opinion on a settled
                // question.
                Status::Retired => "already_retired",
                _ => "not_proposed_for_retirement",
            }));
            continue;
        }

        // D2 is checked before the row is classified, because it needs no ledger
        // and therefore holds even on rows whose demand cannot be measured. A
        // fatal that has been found is never suppressed by the row it was found
        // on turning out to be unenforceable.
        check_successor(&mut run, &index, record, &location);

        let Some(code) = &record.code else {
            run.row(Verdict::Skipped("deprecated_record_has_no_code_counterpart"));
            run.warn(Violation::fatal(
                location,
                RULE_UNMEASURABLE,
                format!(
                    "a code counterpart on {}, so `{}` can measure its consumers; or the record \
                     withdrawn rather than deprecated, if it was never built",
                    record.id,
                    vds_scan::GENERATOR_COMMAND
                ),
                format!(
                    "{} is deprecated with code: null, so it names no (importPath, exportName) \
                     pair and its demand is unmeasurable. Recording zero would be asserting a \
                     drain, which VDS S-9(6)(2) forbids. Whether the record or the codebase is \
                     wrong is reconciliation's row (VDS S-5(6)).",
                    record.id
                ),
            ));
            continue;
        };

        if no_screens {
            run.row(Verdict::Skipped("no_screens_on_the_declared_surface"));
            continue;
        }

        run.row(Verdict::Enforced);

        let routes = ledger.routes_consuming(&code.import_path, &code.export_name);
        if !routes.is_empty() {
            run.fail(Violation::fatal(
                location.clone(),
                RULE_DRAIN,
                format!(
                    "zero routes consuming {}::{}. Migrate every route named below to {}, delete \
                     the reference, then re-run `{}` and re-measure.",
                    code.import_path,
                    code.export_name,
                    successor_phrase(record),
                    vds_scan::GENERATOR_COMMAND
                ),
                format!(
                    "{} route(s) still consume {}: {}",
                    routes.len(),
                    record.id,
                    routes.join(", ")
                ),
            ));
        }

        if u64::from(record.demand.routes) != routes.len() as u64 {
            run.warn(Violation::fatal(
                location,
                RULE_DEMAND_DISAGREES,
                format!(
                    "demand.routes: {}, which is what this run measured from the screens ledger",
                    routes.len()
                ),
                format!(
                    "demand.routes: {}, measuredBy {:?}. The record and the ledger disagree, so \
                     the stored figure is an unmeasured opinion. The drain was decided on the \
                     measurement and never on the figure.",
                    record.demand.routes, record.demand.measured_by
                ),
            ));
        }
    }

    run.finish(&ctx.capture_options()?, out)
}

/// VDS S-9(7): a component may not be deprecated toward a successor that is not
/// itself registered or later.
///
/// The existence limb is checked first and separately, because "CMP-0042 is not
/// registered" and "there is no CMP-0042" are different defects with different
/// remedies, and collapsing them sends the reader to look at a record that is
/// not there.
fn check_successor(
    run: &mut ProofRun<'_>,
    index: &RegisterIndex,
    record: &ComponentRecord,
    location: &str,
) {
    // VDS S-9(6)(1): a null successor is a withdrawal outright, which is a
    // lawful shape and not a missing field.
    let Some(successor_id) = &record.superseded_by else {
        return;
    };

    let Some(successor) = index.by_id(successor_id) else {
        run.fail(Violation::fatal(
            location,
            RULE_SUCCESSOR,
            format!(
                "a register record {successor_id} in status registered or later before {} is \
                 deprecated toward it; or supersededBy: null, if {} is withdrawn outright with \
                 no replacement",
                record.id, record.id
            ),
            format!(
                "{successor_id} resolves to no register record at all, so {} is deprecated \
                 toward a component that does not exist and the migration it names has no \
                 destination",
                record.id
            ),
        ));
        return;
    };

    if successor.status.index() < Status::Registered.index() {
        run.fail(Violation::fatal(
            location,
            RULE_SUCCESSOR,
            format!(
                "{successor_id} in status registered or later before {} is deprecated toward \
                 it. Complete the successor's contract first, or withdraw {} outright with \
                 supersededBy: null.",
                record.id, record.id
            ),
            format!(
                "{successor_id} ({:?}) is {}, so {} is deprecated toward a component that does \
                 not yet exist: two incomplete halves and no whole",
                successor.name, successor.status, record.id
            ),
        ));
        return;
    }

    if matches!(successor.status, Status::Deprecated | Status::Retired) {
        run.warn(Violation::fatal(
            location,
            RULE_SUCCESSOR_LEAVING,
            format!(
                "a migration target that is staying. Point {} at a successor in status \
                 registered, built or verified.",
                record.id
            ),
            format!(
                "{successor_id} ({:?}) is itself {}, so every route migrating off {} arrives \
                 somewhere that is also leaving. Reported and not fatal: VDS S-9(7) requires the \
                 successor to be registered or later, and {} is later on the VDS S-5(4) path.",
                successor.name, successor.status, record.id, successor.status
            ),
        ));
    }
}

/// Where consumers are being asked to migrate to, in words a reader can act on.
fn successor_phrase(record: &ComponentRecord) -> String {
    match &record.superseded_by {
        Some(id) => id.to_string(),
        None => "nothing (the component is withdrawn outright)".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::{Harness, run_kind};
    use vds_core::{
        ComponentId, EXIT_PASSED, EXIT_VACUOUS, EXIT_VIOLATION, ProofKind, ProofStatus, Severity,
        Status,
    };

    /// A screen that exists only so the declared surface is not empty, and that
    /// consumes nothing under test.
    fn unrelated_screen(h: &Harness) {
        h.screen("home", &["Unrelated"]);
    }

    #[test]
    fn a_deprecated_component_with_no_remaining_consumers_passes() {
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        h.register("Button", Status::Registered);
        // Withdrawn outright: supersededBy is null, which VDS S-9(6)(1) treats
        // as a lawful shape and not an omission.
        h.register("Card", Status::Deprecated);
        h.ledger();

        let (outcome, text) = run_kind(&h, ProofKind::RetirementDrain);
        assert_eq!(outcome.exit_code, EXIT_PASSED, "{text}");
        assert_eq!(outcome.status, ProofStatus::Passed);
        assert_eq!(outcome.rows_enforced, 1, "{text}");
        assert!(text.contains("not_proposed_for_retirement"), "{text}");
    }

    /// The failing-direction test VDS S-7(2)(2) requires, and the one named in
    /// `.vds/enforcement.lock`. It seeds a route that still consumes a
    /// deprecated component and asserts the non-zero exit.
    #[test]
    fn retirement_drain_fails_on_a_remaining_consumer() {
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        h.register("Button", Status::Deprecated);
        h.ledger();

        let (outcome, text) = run_kind(&h, ProofKind::RetirementDrain);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert_eq!(outcome.status, ProofStatus::Failed);
        assert!(text.contains("app/dash/page.tsx"), "{text}");
        assert!(text.contains("still consume"), "{text}");
    }

    /// "3 consumers remain" names no work anyone can pick up. Every remaining
    /// route is printed, so the finding is a task list.
    #[test]
    fn every_remaining_route_is_named_in_the_finding() {
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        h.screen("settings", &["Button"]);
        h.register("Button", Status::Deprecated);
        h.ledger();

        let (outcome, text) = run_kind(&h, ProofKind::RetirementDrain);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("app/dash/page.tsx"), "{text}");
        assert!(text.contains("app/settings/page.tsx"), "{text}");
        assert!(text.contains("2 route(s)"), "{text}");
    }

    /// VDS S-7(2)(4): nothing is proposed for retirement, so this run could not
    /// have failed and is not evidence that anything drained.
    #[test]
    fn a_register_with_nothing_deprecated_is_vacuous_and_never_passed() {
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        h.register("Button", Status::Registered);
        h.ledger();

        let (outcome, text) = run_kind(&h, ProofKind::RetirementDrain);
        assert_eq!(outcome.status, ProofStatus::Vacuous, "{text}");
        assert_eq!(outcome.exit_code, EXIT_VACUOUS, "{text}");
        assert!(!text.contains("PASS:"), "no PASS beside a vacuity: {text}");
        assert!(text.contains("not_proposed_for_retirement"), "{text}");
    }

    /// VDS S-9(7). Deprecating toward a component that does not yet exist is how
    /// a library ends up with two incomplete halves and no whole.
    #[test]
    fn retirement_drain_fails_on_a_successor_that_is_not_yet_registered() {
        let h = Harness::new();
        unrelated_screen(&h);
        let next = h.register("ButtonNext", Status::Designed);
        let old = h.register("Button", Status::Deprecated);
        h.amend(&old, |record| record.superseded_by = Some(next.clone()));
        h.ledger();

        let (outcome, text) = run_kind(&h, ProofKind::RetirementDrain);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("does not yet exist"), "{text}");
        assert!(text.contains(next.as_str()), "{text}");
    }

    #[test]
    fn retirement_drain_fails_on_a_successor_that_resolves_to_no_record() {
        let h = Harness::new();
        unrelated_screen(&h);
        let old = h.register("Button", Status::Deprecated);
        h.amend(&old, |record| {
            record.superseded_by = Some(ComponentId::parse("CMP-9999").unwrap())
        });
        h.ledger();

        let (outcome, text) = run_kind(&h, ProofKind::RetirementDrain);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(
            text.contains("resolves to no register record"),
            "an absent successor and an unregistered one are different defects with different \
             remedies: {text}"
        );
    }

    #[test]
    fn a_successor_that_is_registered_satisfies_the_supersession_rule() {
        let h = Harness::new();
        unrelated_screen(&h);
        let next = h.register("ButtonNext", Status::Registered);
        let old = h.register("Button", Status::Deprecated);
        h.amend(&old, |record| record.superseded_by = Some(next.clone()));
        h.ledger();

        let (outcome, text) = run_kind(&h, ProofKind::RetirementDrain);
        assert_eq!(outcome.exit_code, EXIT_PASSED, "{text}");
        assert_eq!(outcome.rows_enforced, 1, "{text}");
    }

    /// VDS S-9(7) says "registered or later", and deprecated is later on the
    /// VDS S-5(4) path. Reading a narrower rule into the clause would be VDS
    /// settling a point the clause does not settle (VDS S-1(2)), so the chain is
    /// reported and does not fail the gate.
    #[test]
    fn a_successor_that_is_itself_deprecated_warns_and_does_not_fail_the_gate() {
        let h = Harness::new();
        unrelated_screen(&h);
        let next = h.register("ButtonNext", Status::Deprecated);
        let old = h.register("Button", Status::Deprecated);
        h.amend(&old, |record| record.superseded_by = Some(next.clone()));
        h.ledger();

        let (outcome, text) = run_kind(&h, ProofKind::RetirementDrain);
        assert_eq!(outcome.exit_code, EXIT_PASSED, "{text}");
        assert_eq!(outcome.rows_enforced, 2, "both are proposed for retirement");
        assert!(text.contains("WARNINGS"), "{text}");
        assert!(text.contains("also leaving"), "{text}");

        let record = h.last_proof(ProofKind::RetirementDrain);
        assert_eq!(record.violations.len(), 1);
        assert_eq!(record.violations[0].severity, Severity::Warning);
    }

    /// AGENTS.md: an unmeasured number is an opinion. The record says five
    /// routes consume it and the ledger says none do.
    #[test]
    fn a_stored_demand_figure_that_disagrees_with_the_measurement_is_reported() {
        let h = Harness::new();
        unrelated_screen(&h);
        let old = h.register("Button", Status::Deprecated);
        h.amend(&old, |record| record.demand.routes = 5);
        h.ledger();

        let (outcome, text) = run_kind(&h, ProofKind::RetirementDrain);
        assert_eq!(
            outcome.exit_code, EXIT_PASSED,
            "the drain is decided on the measurement, so a wrong stored figure must not block a \
             component that genuinely drained: {text}"
        );
        assert!(text.contains("unmeasured opinion"), "{text}");

        let record = h.last_proof(ProofKind::RetirementDrain);
        assert_eq!(record.violations[0].severity, Severity::Warning);
    }

    /// VDS S-9(8) puts a retired record's consumers on composition's row, where
    /// the presence of the code is itself the defect. Re-draining here would be
    /// a second opinion on a settled question.
    #[test]
    fn a_retired_record_is_not_re_drained_and_the_run_says_why() {
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        h.register("Button", Status::Retired);
        h.ledger();

        let (outcome, text) = run_kind(&h, ProofKind::RetirementDrain);
        assert_eq!(outcome.status, ProofStatus::Vacuous, "{text}");
        assert!(text.contains("already_retired"), "{text}");
        assert!(!text.contains("VIOLATIONS"), "{text}");
    }

    /// The proof cannot reach a record with no code coordinate, and says which
    /// record rather than only how many.
    #[test]
    fn a_deprecated_record_with_no_code_counterpart_is_skipped_by_name_and_reported() {
        let h = Harness::new();
        unrelated_screen(&h);
        h.register_unbuilt("Sketch", Status::Deprecated);
        h.ledger();

        let (outcome, text) = run_kind(&h, ProofKind::RetirementDrain);
        assert_eq!(
            outcome.status,
            ProofStatus::Vacuous,
            "an unmeasurable row is not a drained row: {text}"
        );
        assert!(
            text.contains("deprecated_record_has_no_code_counterpart"),
            "{text}"
        );
        assert!(text.contains("unmeasurable"), "{text}");
        assert!(
            text.contains("CMP-0001"),
            "a count says how many rows went unchecked and never which: {text}"
        );
    }

    /// The [2026] VJS-CC-OPBOX 3 D3 shape in this proof's own terms: measuring
    /// zero consumers across zero screens is the absence of a measurement, and a
    /// drain certified against it certifies nothing.
    #[test]
    fn an_empty_screen_surface_makes_the_run_vacuous_rather_than_draining_by_default() {
        let h = Harness::new();
        h.register("Button", Status::Deprecated);
        h.ledger();

        let (outcome, text) = run_kind(&h, ProofKind::RetirementDrain);
        assert_eq!(outcome.status, ProofStatus::Vacuous, "{text}");
        assert_eq!(outcome.exit_code, EXIT_VACUOUS, "{text}");
        assert!(text.contains("no_screens_on_the_declared_surface"), "{text}");
        assert!(text.contains("ABSENCE of a measurement"), "{text}");
    }

    /// A drain measured against a ledger that no longer describes the screens
    /// reports the demand of a surface that is gone. Exit 2, and the proof did
    /// not run.
    #[test]
    fn a_stale_ledger_is_a_precondition_failure_and_not_a_pass() {
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        h.register("Button", Status::Deprecated);
        h.ledger();
        h.screen("dash", &["Button", "Card"]);

        let error = h.run_kind_err(ProofKind::RetirementDrain);
        assert!(error.to_string().contains("STALE"), "{error}");
    }

    #[test]
    fn the_run_records_its_reliance_on_the_reserved_forced_drain_question() {
        let h = Harness::new();
        unrelated_screen(&h);
        h.register("Button", Status::Deprecated);
        h.ledger();
        run_kind(&h, ProofKind::RetirementDrain);

        let record = h.last_proof(ProofKind::RetirementDrain);
        assert!(
            record.notes.iter().any(|n| n.contains("SUBMISSION-VDS-004")),
            "{:?}",
            record.notes
        );
        assert!(
            record.notes.iter().any(|n| n.contains("ABSOLUTE")),
            "{:?}",
            record.notes
        );
    }

    #[test]
    fn the_run_records_what_its_measurement_cannot_reach() {
        let h = Harness::new();
        unrelated_screen(&h);
        h.register("Button", Status::Deprecated);
        h.ledger();
        run_kind(&h, ProofKind::RetirementDrain);

        let record = h.last_proof(ProofKind::RetirementDrain);
        let reach = record
            .notes
            .iter()
            .find(|n| n.starts_with("[reach]"))
            .expect("a run that does not say what it cannot see has narrowed silently");
        assert!(reach.contains("DECLARED SURFACE"), "{reach}");
        assert!(reach.contains("re-export"), "{reach}");
    }

    /// VDS S-7(2)(1): same inputs, same output, same digest. A drain proof whose
    /// digest moved on a re-run would make every warrant citing it look spent,
    /// and a reader who is told to ignore that field stops reading it.
    #[test]
    fn two_runs_over_an_unchanged_register_cite_the_same_evidence_digest() {
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        h.register("Button", Status::Deprecated);
        h.register("Card", Status::Registered);
        h.ledger();

        run_kind(&h, ProofKind::RetirementDrain);
        let first = h.last_proof(ProofKind::RetirementDrain);
        run_kind(&h, ProofKind::RetirementDrain);
        let second = h.last_proof(ProofKind::RetirementDrain);

        assert_ne!(first.id, second.id, "two runs are two records");
        assert_eq!(first.digest, second.digest);
    }
}
