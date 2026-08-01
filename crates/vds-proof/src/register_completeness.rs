//! The `register_completeness` proof. The EXISTENCE proof.
//!
//! VDS S-7(5): "every component referenced by any declared screen exists in the
//! register." That is the entire question, and keeping it that narrow is
//! deliberate. Where `composition` asks whether the record that exists is in a
//! state fit to be used, this proof asks only whether a record exists at all. The
//! two are kept apart so that W1 REGISTER-COMPLETE can be granted on existence
//! alone, before any design work has happened (VDS S-6(2)). A record sitting at
//! `proposed` therefore SATISFIES this proof and FAILS composition, and folding
//! composition's status test in here would make W1 unobtainable on the very
//! surface it was written for.
//!
//! One fatal rule and three informational ones:
//!
//!   R1  a governed component reference that no register record claims. This is
//!       the whole of VDS S-7(5), and the finding names which half of the code
//!       coordinate is wrong rather than saying "no such record".
//!   I1  the reference resolves to a record whose status is not one composition
//!       will accept. Existence is satisfied, so this proof passes on it. It is
//!       recorded so that nobody reading a W1 evidence record mistakes it for W2
//!       evidence.
//!   I2  a component reference whose import the ledger could not resolve. The row
//!       is skipped rather than enforced, and the site is named, because a
//!       reference this proof never reached is a hole in the claim W1 rests on,
//!       and an unnamed hole is the silent narrowing VDS exists to catch.
//!   I3  a namespaced reference such as `Card.Header`. Only the root binding is a
//!       register coordinate, so the row establishes that `Card` exists and says
//!       nothing at all about `Header`.
//!
//! Bare HTML elements are informational rows, counted in `rows_considered` and
//! excluded from `rows_enforced`, per VDS S-9(10) RESERVED (SUBMISSION-VDS-005).
//!
//! Findings at informational severity are captured on the record and are not
//! printed by the run's reporter, so each class also emits a counted note. A
//! finding that reaches only the record is invisible to the person watching the
//! gate, and a count on the terminal is what tells them to go and read it.
//!
//! This proof reads component NAMES, import PATHS and lifecycle STATUSES. It
//! reads no design value (VDS S-2(2)).

use std::io::Write;

use vds_core::{ProofKind, Result, Violation};

use crate::ProofContext;
use crate::index::RegisterIndex;
use crate::run::{Outcome, Verdict};

pub const GATE: &str = "crates/vds-proof/src/register_completeness.rs";

const RULE_ABSENT: &str = "VDS S-7(5) register_completeness R1: every component referenced by any declared screen \
     exists in the register";
const RULE_NOT_YET_COMPOSABLE: &str = "VDS S-7(5) register_completeness I1 / S-5(4): the record exists, which is all this proof \
     asks, and its status is not one composition accepts";
const RULE_UNREACHED: &str = "VDS S-7(5) register_completeness I2: a reference whose import the ledger could not resolve \
     is outside the completeness claim";
const RULE_ROOT_ONLY: &str = "VDS S-7(5) register_completeness I3: a namespaced reference is established at its root \
     binding only";
const RULE_UNMEASURED: &str = "draft S-5(9) register_completeness R2: a directed record measured by nothing, out of \
     grace";
const RULE_MEASURE_HYGIENE: &str = "draft S-5(9) register_completeness R3: a measure reads shipped code or a rendered \
     artefact, never a plan";

/// What this proof establishes, and the thing a reader will otherwise assume it
/// establishes.
pub const SCOPE_NOTE: &str = "this proof establishes EXISTENCE and nothing else (VDS S-7(5)). A record at `proposed` or \
     `designed` satisfies it and fails `composition`, which is why they are two proofs: W1 \
     REGISTER-COMPLETE is granted on existence alone, before any design work has happened. A \
     warrant citing this run has evidence that the register covers the surface, and no evidence \
     that anything on the surface is fit to be used.";

/// Where the claim stops. Stated on every run rather than left to be inferred
/// from the skip counts, because docs/GOAL.md is explicit that "no unregistered
/// component anywhere" is not provable by a finite check.
pub const BOUND_NOTE: &str = "the claim is bounded by [surface] screen_globs and [surface] governed_import_prefixes. A \
     screen outside those globs, a reference imported from outside those prefixes, and a \
     reference whose import the ledger could not resolve are each counted and not enforced. \
     This proof cannot say `no unregistered component anywhere`, only `no absent record among \
     the references it reached`.";

pub const RESERVED_NOTE: &str = "relies on VDS S-9(10) RESERVED (SUBMISSION-VDS-005): bare HTML elements are informational \
     rows only, excluded from rows_enforced. Any warrant citing this proof must record that \
     reliance in its `reserved` array.";

pub fn run(ctx: &ProofContext, out: &mut dyn Write) -> Result<Outcome> {
    let project = ctx.project;
    let mut run = ctx.new_run(ProofKind::RegisterCompleteness, GATE);
    run.input_file(&project.config_path)?;

    let ledger = vds_scan::load_fresh(project)?;
    // The ledger's CONTENT digest, not its file digest: the file carries
    // `generated_at`, which moves on a no-op regeneration and would move this
    // proof's evidence digest with it (VDS S-7(2)(1)).
    run.input_named("<screens ledger content>", ledger.content_digest.clone());

    // Every record is an input, not only the ones a lookup happens to hit.
    // Adding a record changes what this proof establishes, and an evidence digest
    // that did not move would tell a warrant reader that nothing had.
    let index = RegisterIndex::build(&ctx.store())?;
    for record in index.records() {
        run.input_file(&record.path)?;
    }

    let prefixes = &project.config.surface.governed_import_prefixes;
    let library_dirs = &project.config.surface.library_dirs;
    if prefixes.is_empty() {
        run.note(
            "[surface] governed_import_prefixes is empty, so no reference can be enforced; \
             every row will be skipped and this run will be vacuous",
        );
    }
    if index.is_empty() {
        // An empty register is an ANSWER, not a missing precondition. The surface
        // was readable and every reference on it is absent, so the run fails
        // loudly rather than exiting 2 and letting a reader conclude the check
        // was unavailable rather than unsatisfied.
        run.note(
            "the register holds no records, so every governed reference on the declared \
             surface is about to be reported absent",
        );
    }
    run.note(SCOPE_NOTE);
    run.note(BOUND_NOTE);
    run.note(RESERVED_NOTE);

    let mut unreached = 0u64;
    let mut root_only = 0u64;
    let mut not_yet_composable = 0u64;

    for screen in &ledger.screens {
        for reference in &screen.references {
            let location = format!("{}:{} <{}>", screen.route, reference.line, reference.name);

            if reference.kind != vds_scan::ReferenceKind::Component {
                run.row(Verdict::Skipped("bare_element_informational_vds_s9_10"));
                continue;
            }

            let Some(import_path) = reference.import_path.as_deref() else {
                run.row(Verdict::Skipped(
                    "component_reference_with_no_resolvable_import",
                ));
                // Named per site, unlike the ungoverned-import skip below. An
                // ungoverned import is a carve-out the project declared in its
                // own config, and a count is enough for it. An unresolved import
                // is a per-site accident of the source, and a count alone would
                // not tell anyone which reference went unchecked.
                unreached += 1;
                run.inform(Violation::fatal(
                    location,
                    RULE_UNREACHED,
                    format!(
                        "{:?} imported from a module the ledger can name, so a register record \
                         can be looked up for it",
                        reference.root
                    ),
                    reference.unresolved_because.clone().unwrap_or_else(|| {
                        "the ledger records no import path and no reason for its absence".to_owned()
                    }),
                ));
                continue;
            };
            // Governed by EITHER the prefix as written or a relative specifier that
            // resolves inside a governed library directory. Rewriting one import to
            // `../../src/components/ui/widget` took a governed component out of
            // enforcement entirely, and the project declared `@/components/`, not that.
            if !reference.is_governed(prefixes, library_dirs) {
                run.row(Verdict::Skipped("import_outside_governed_prefixes"));
                run.inform(Violation::fatal(
                    location.clone(),
                    RULE_UNREACHED,
                    "an import inside a governed prefix, or one resolving into a governed \
                     library directory",
                    format!("imported from {import_path:?}, which is outside both"),
                ));
                continue;
            }

            run.row(Verdict::Enforced);

            if reference.member_is_unverified() {
                // `<Card.Header />` binds `Card` and nothing else, so the lookup
                // below answers a narrower question than the tag suggests. Saying
                // so is the difference between a bounded claim and a false one.
                root_only += 1;
                run.inform(Violation::fatal(
                    location.clone(),
                    RULE_ROOT_ONLY,
                    format!(
                        "a register coordinate for {:?} itself, where the member is a component \
                         in its own right",
                        reference.name
                    ),
                    format!(
                        "only the root binding {:?} is a register coordinate, so this row \
                         establishes that {:?} exists and says nothing about {:?}",
                        reference.root, reference.root, reference.name
                    ),
                ));
            }

            // The EXPORT name, not the local binding. See composition.rs.
            let export_name = reference.lookup_name();
            let Some(record) = index.lookup(import_path, export_name) else {
                let misses = index.near_misses(import_path, export_name);
                let detail = if misses.is_empty() {
                    "no register record claims either half of the coordinate".to_owned()
                } else {
                    misses.join("; ")
                };
                run.fail(Violation::fatal(
                    location,
                    RULE_ABSENT,
                    format!(
                        "a register record whose code.importPath is {import_path:?} and whose \
                         code.exportName is {export_name:?}. Any status satisfies this proof, \
                         `proposed` included: this is the existence question, not the \
                         composition one"
                    ),
                    format!("absent from the register ({detail})"),
                ));
                continue;
            };

            if !record.status.is_enforceable() {
                not_yet_composable += 1;
                run.inform(Violation::fatal(
                    location,
                    RULE_NOT_YET_COMPOSABLE,
                    format!(
                        "{} exists in the register, which is all VDS S-7(5) asks of it here",
                        record.id
                    ),
                    format!(
                        "{} status {}: existence satisfied, and `composition` accepts this site \
                         only once the status is one of registered, built, verified",
                        record.id, record.status
                    ),
                ));
            }
        }
    }

    // The reporter prints fatal findings and warnings, not informational ones, so
    // each informational class is summarised here. Without this the terminal
    // shows a clean PASS while the record carries findings nobody was told about.
    if unreached > 0 {
        run.note(format!(
            "{unreached} component reference(s) were NOT reached, because the ledger could not \
             resolve the module their root name was imported from. Each is captured on this \
             record as an informational finding naming its route and line, and none of them was \
             enforced."
        ));
    }
    if root_only > 0 {
        run.note(format!(
            "{root_only} namespaced reference(s) were checked at their ROOT binding only. Each \
             is captured on this record as an informational finding naming its route and line."
        ));
    }
    if not_yet_composable > 0 {
        run.note(format!(
            "{not_yet_composable} reference(s) resolve to a record that exists and is not in a \
             status `composition` accepts. This proof passes on them by design, because VDS \
             S-7(5) is the existence question; composition is where they fail."
        ));
    }

    // Draft S-5(9), ENACTMENT PENDING (SUBMISSION-VDS-015): measurement
    // coverage of DIRECTED records, and hygiene of the measures themselves.
    // One row per record that carries the metadata; a record carrying none is
    // outside the drafted clause and adds no row.
    //
    // The clock is the ledger's `generated_at` and never the wall clock
    // (VDS S-7(2)(1)). It is normally excluded from this proof's evidence
    // digest precisely because it moves on a no-op regeneration; the grace
    // rule READS it, so where a directed record exists it becomes an input and
    // is digested, keeping findings a function of inputs.
    let directed_exists = index
        .records()
        .iter()
        .any(|r| r.value.directed_at.is_some());
    if directed_exists {
        run.input_named(
            "<screens ledger generated_at>",
            vds_core::Digest::of_text(ledger.generated_at.as_str()),
        );
    }
    for record in index.records() {
        let value = &record.value;
        let has_metadata = value.directed_at.is_some() || !value.measured_by.is_empty();
        if !has_metadata {
            continue;
        }
        run.row(Verdict::Enforced);
        let location = format!("{} [{}]", value.id, value.name);

        // R3, the hygiene rule, checked for EVERY measure regardless of grace:
        // a measure pointing at a plan or an internal doc is measured by
        // prose, and prose is not enforcement. Measures read shipped code or
        // rendered artefacts.
        for measure in &value.measured_by {
            let lowered = measure.to_lowercase();
            let doc_like = lowered.ends_with(".md")
                || lowered.contains("internal-docs/")
                || lowered.starts_with("docs/")
                || lowered.contains("/docs/")
                || lowered.contains("plans/")
                || lowered.contains("readme");
            if doc_like {
                run.fail(Violation::fatal(
                    location.clone(),
                    RULE_MEASURE_HYGIENE,
                    "every measuredBy entry to name shipped code or a rendered-artefact \
                     reader (a gate path, a proof kind, a reader command)",
                    format!(
                        "measuredBy names {measure:?}, which is a document. A rule measured \
                         by a plan is measured by prose: the plan can promise anything and \
                         the row stays green. Point the measure at the gate that reads the \
                         artefact, or remove it and let R2 say the record is unmeasured."
                    ),
                ));
            }
        }

        // R2, the grace rule: directed, unmeasured, and out of grace.
        if let Some(directed_at) = &value.directed_at
            && value.measured_by.is_empty()
        {
            let grace = i64::from(value.grace_days.unwrap_or(0));
            match crate::geometry::days_between(directed_at.as_str(), ledger.generated_at.as_str())
            {
                None => run.fail(Violation::fatal(
                    location.clone(),
                    RULE_UNMEASURED,
                    "two readable UTC dates",
                    format!(
                        "directedAt is {:?} and the ledger's generated_at is {:?}, and the \
                         distance between them could not be computed, so the grace rule is \
                         UNKNOWN rather than met.",
                        directed_at.as_str(),
                        ledger.generated_at.as_str()
                    ),
                )),
                Some(days) if days > grace => run.fail(Violation::fatal(
                    location.clone(),
                    RULE_UNMEASURED,
                    format!(
                        "measuredBy to name at least one measure within {grace} day(s) of \
                         the directive"
                    ),
                    format!(
                        "directed {} day(s) ago (at {}) and measured by NOTHING. A directed \
                         record with an empty measuredBy is a promise nobody checks: it was \
                         registered, it reads as governed, and no instrument would ever say \
                         it failed. This is the row class that shipped structurally-green \
                         pages that looked nothing like their frames.",
                        days,
                        directed_at.as_str()
                    ),
                )),
                Some(_) => {}
            }
        }
    }

    run.finish(&ctx.capture_options()?, out)
}

#[cfg(test)]
mod tests {
    use crate::testing::{Harness, run_kind};
    use vds_core::{
        EXIT_PASSED, EXIT_VACUOUS, EXIT_VIOLATION, ProofKind, ProofStatus, Severity, Status,
    };

    const KIND: ProofKind = ProofKind::RegisterCompleteness;

    #[test]
    fn a_screen_whose_components_are_all_in_the_register_passes() {
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        h.register("Button", Status::Registered);
        h.ledger();
        let (outcome, text) = run_kind(&h, KIND);
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
        assert_eq!(outcome.exit_code, EXIT_PASSED);
        assert!(outcome.rows_enforced > 0);
        assert!(
            h.last_proof(KIND).violations.is_empty(),
            "a complete register produces no finding of any severity"
        );
    }

    /// The failing-direction test VDS S-7(2)(2) requires, and the one
    /// `.vds/enforcement.lock` names. It seeds a screen importing a governed
    /// component that no register record claims, and asserts the non-zero exit.
    #[test]
    fn register_completeness_fails_on_a_component_absent_from_the_register() {
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        h.ledger();
        let (outcome, text) = run_kind(&h, KIND);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert_eq!(outcome.status, ProofStatus::Failed);
        assert!(text.contains("absent from the register"), "{text}");
        assert!(text.contains("app/dash/page.tsx"), "{text}");
    }

    /// R1's finding says WHICH HALF of the code coordinate is wrong. "No such
    /// record" is true and useless when the cause is a typo in one half.
    #[test]
    fn a_near_miss_names_which_half_of_the_coordinate_is_wrong() {
        let wrong_export = Harness::new();
        wrong_export.screen("dash", &["Button"]);
        wrong_export.register_as("CMP-0001", "Button", "Buton", Status::Registered);
        wrong_export.ledger();
        let (outcome, text) = run_kind(&wrong_export, KIND);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("but exports"), "{text}");

        let wrong_module = Harness::new();
        wrong_module.screen_from("dash", &["Button"], "@/components/legacy");
        wrong_module.register("Button", Status::Registered);
        wrong_module.ledger();
        let (outcome, text) = run_kind(&wrong_module, KIND);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("but from"), "{text}");
    }

    /// The distinction this proof exists to preserve. `composition` carries the
    /// mirror test, `composition_fails_on_a_component_that_is_only_proposed`, and
    /// the two must disagree: W1 is granted on existence alone.
    #[test]
    fn a_record_at_proposed_satisfies_existence_and_is_recorded_as_not_yet_composable() {
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        h.register("Button", Status::Proposed);
        h.ledger();
        let (outcome, text) = run_kind(&h, KIND);
        assert_eq!(
            outcome.exit_code, EXIT_PASSED,
            "existence is satisfied by a record at any status: {text}"
        );
        assert_eq!(outcome.status, ProofStatus::Passed);

        let record = h.last_proof(KIND);
        assert_eq!(record.violations.len(), 1, "{:?}", record.violations);
        assert_eq!(record.violations[0].severity, Severity::Informational);
        assert!(
            record.violations[0].actual.contains("composition"),
            "{:?}",
            record.violations[0]
        );
        assert!(
            text.contains("status `composition` accepts"),
            "an informational finding is not printed by the reporter, so its count must be: \
             {text}"
        );
    }

    /// VDS S-9(8) inverts the test for `composition` after retirement. It does
    /// not invert this one: a tombstone is kept forever (VDS S-9(6)(3)), and a
    /// tombstone is a record that exists.
    #[test]
    fn a_deprecated_or_retired_record_still_exists_and_satisfies_this_proof() {
        for status in [Status::Deprecated, Status::Retired] {
            let h = Harness::new();
            h.screen("dash", &["Button"]);
            h.register("Button", status);
            h.ledger();
            let (outcome, text) = run_kind(&h, KIND);
            assert_eq!(outcome.exit_code, EXIT_PASSED, "{status}: {text}");
            assert_eq!(
                h.last_proof(KIND).violations[0].severity,
                Severity::Informational,
                "{status}: a retirement is composition's to refuse, not this proof's"
            );
        }
    }

    /// VDS S-7(2)(4). Nothing in scope is a vacuity, never a pass.
    #[test]
    fn bare_elements_alone_make_the_run_vacuous_rather_than_passing() {
        let h = Harness::new();
        h.write(
            "app/plain/page.tsx",
            "export default function P(){ return <div><span /></div>; }\n",
        );
        h.ledger();
        let (outcome, text) = run_kind(&h, KIND);
        assert_eq!(outcome.status, ProofStatus::Vacuous, "{text}");
        assert_eq!(outcome.exit_code, EXIT_VACUOUS);
        assert!(
            text.contains("bare_element_informational_vds_s9_10"),
            "{text}"
        );
        assert!(!text.contains("PASS:"), "no PASS beside a vacuity: {text}");
    }

    #[test]
    fn an_ungoverned_import_is_counted_and_not_enforced() {
        let h = Harness::new();
        h.screen_from("dash", &["Chart"], "third-party-charts");
        h.ledger();
        let (outcome, text) = run_kind(&h, KIND);
        assert_eq!(
            outcome.exit_code, EXIT_VACUOUS,
            "nothing was in scope, so the run proves nothing and says so: {text}"
        );
        assert!(text.contains("import_outside_governed_prefixes"), "{text}");
    }

    /// I2. A reference the ledger could not resolve is a hole in the claim, so it
    /// is skipped with a named reason, counted on the terminal, and named per
    /// site on the record.
    #[test]
    fn an_unresolvable_import_is_skipped_and_the_site_is_named() {
        let h = Harness::new();
        h.write(
            "app/dash/page.tsx",
            "import { Button } from \"@/components/ui\";\n\
             function Local() { return <span />; }\n\
             export default function P(){ return <div><Button /><Local /></div>; }\n",
        );
        h.register("Button", Status::Registered);
        h.ledger();
        let (outcome, text) = run_kind(&h, KIND);
        assert_eq!(outcome.exit_code, EXIT_PASSED, "{text}");
        assert_eq!(
            outcome.rows_enforced, 1,
            "only Button was reachable: {text}"
        );
        assert!(
            text.contains("component_reference_with_no_resolvable_import"),
            "{text}"
        );
        assert!(text.contains("were NOT reached"), "{text}");

        let record = h.last_proof(KIND);
        assert_eq!(record.violations.len(), 1, "{:?}", record.violations);
        assert_eq!(record.violations[0].severity, Severity::Informational);
        assert!(
            record.violations[0].location.contains("app/dash/page.tsx"),
            "{:?}",
            record.violations[0]
        );
        assert!(
            record.violations[0].actual.contains("not imported"),
            "the row must say WHY it could not be reached: {:?}",
            record.violations[0]
        );
    }

    /// I3. `<Card.Header />` binds `Card`, so the row proves `Card` exists and
    /// nothing about `Header`. Claiming otherwise is the silent narrowing this
    /// specification exists to catch.
    #[test]
    fn a_namespaced_reference_is_established_at_its_root_only() {
        let h = Harness::new();
        h.write(
            "app/dash/page.tsx",
            "import { Card } from \"@/components/ui\";\n\
             export default function P(){ return <Card.Header />; }\n",
        );
        h.register("Card", Status::Registered);
        h.ledger();
        let (outcome, text) = run_kind(&h, KIND);
        assert_eq!(outcome.exit_code, EXIT_PASSED, "{text}");
        assert_eq!(outcome.rows_enforced, 1);
        assert!(text.contains("ROOT binding only"), "{text}");

        let record = h.last_proof(KIND);
        assert_eq!(record.violations.len(), 1, "{:?}", record.violations);
        assert!(
            record.violations[0].actual.contains("Card.Header"),
            "{:?}",
            record.violations[0]
        );
    }

    /// A missing precondition is exit 2 and means the proof DID NOT RUN. It is
    /// never a pass and never a violation.
    #[test]
    fn an_absent_ledger_is_a_precondition_failure_and_not_a_pass() {
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        h.register("Button", Status::Registered);
        let error = h.run_kind_err(KIND);
        assert!(error.to_string().contains("vds ledger screens"), "{error}");
    }

    /// A register in which one coordinate is claimed twice cannot answer the
    /// existence question: a lookup finds one record and never the other. That is
    /// a precondition failure, not a pass over the visible half.
    #[test]
    fn two_records_claiming_one_coordinate_are_a_precondition_failure() {
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        h.register_as("CMP-0001", "Button", "Button", Status::Registered);
        h.register_as("CMP-0002", "Button", "Button", Status::Retired);
        h.ledger();
        let error = h.run_kind_err(KIND);
        assert!(error.to_string().contains("also claims"), "{error}");
    }

    #[test]
    fn the_run_records_what_it_did_not_establish() {
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        h.register("Button", Status::Registered);
        h.ledger();
        run_kind(&h, KIND);
        let notes = h.last_proof(KIND).notes;
        assert!(
            notes.iter().any(|n| n.contains("SUBMISSION-VDS-005")),
            "{notes:?}"
        );
        assert!(
            notes
                .iter()
                .any(|n| n.contains("EXISTENCE and nothing else")),
            "a reader of this record must not mistake W1 evidence for W2 evidence: {notes:?}"
        );
        assert!(
            notes
                .iter()
                .any(|n| n.contains("no unregistered component anywhere")),
            "the bound on the claim is stated on every run: {notes:?}"
        );
    }

    /// VDS S-7(2)(1): the evidence digest must not move when nothing did.
    #[test]
    fn two_runs_over_an_unchanged_surface_cite_the_same_evidence_digest() {
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        h.register("Button", Status::Registered);
        h.ledger();
        run_kind(&h, KIND);
        let first = h.last_proof(KIND).digest;
        h.ledger();
        run_kind(&h, KIND);
        let second = h.last_proof(KIND).digest;
        assert_eq!(
            first, second,
            "a digest that moves on a no-op regeneration makes every warrant look spent"
        );
    }

    // -- draft S-5(9): measurement coverage and measure hygiene ---------------

    /// THE failing-direction seed for R2: the row class that shipped
    /// structurally-green pages. Directed, registered, measured by nothing,
    /// and out of grace.
    #[test]
    fn a_directed_record_measured_by_nothing_goes_red_after_its_grace() {
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        let id = h.register("Button", Status::Registered);
        h.amend(&id, |r| {
            r.directed_at = Some(vds_core::Timestamp::fixed(2026, 7, 1, 10, 0, 0));
            r.grace_days = Some(14);
        });
        h.ledger();
        let (outcome, text) = run_kind(&h, KIND);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("measured by NOTHING"), "{text}");
    }

    #[test]
    fn a_directed_record_inside_its_grace_does_not_fail_yet() {
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        let id = h.register("Button", Status::Registered);
        h.amend(&id, |r| {
            // Directed "now": the ledger regenerates below, so its
            // generated_at sits within any non-trivial grace of today.
            r.directed_at = Some(vds_core::Timestamp::now());
            r.grace_days = Some(14);
        });
        h.ledger();
        let (outcome, text) = run_kind(&h, KIND);
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
    }

    #[test]
    fn a_directed_record_with_a_real_measure_passes() {
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        let id = h.register("Button", Status::Registered);
        h.amend(&id, |r| {
            r.directed_at = Some(vds_core::Timestamp::fixed(2026, 7, 1, 10, 0, 0));
            r.grace_days = Some(14);
            r.measured_by = vec!["crates/vds-proof/src/contrast.rs".into()];
        });
        h.ledger();
        let (outcome, text) = run_kind(&h, KIND);
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
    }

    /// THE failing-direction seed for R3: a measure pointing at a plan
    /// document is measured by prose, whatever the grace says.
    #[test]
    fn a_measure_pointing_at_a_plan_document_is_refused() {
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        let id = h.register("Button", Status::Registered);
        h.amend(&id, |r| {
            r.measured_by = vec!["internal-docs/design-migration-plan.md".into()];
        });
        h.ledger();
        let (outcome, text) = run_kind(&h, KIND);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("measured by prose"), "{text}");
    }

    #[test]
    fn an_undirected_unmeasured_record_is_outside_the_drafted_clause() {
        // The clause reaches records that were DIRECTED. A record with neither
        // field is the pre-draft world and must not be retroactively red.
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        h.register("Button", Status::Registered);
        h.ledger();
        let (outcome, text) = run_kind(&h, KIND);
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
    }
}
