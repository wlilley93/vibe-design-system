//! The `composition` proof. The anti-drift proof.
//!
//! VDS S-7(5): "no screen uses an unregistered component". Where
//! `register_completeness` asks whether a record EXISTS, composition asks whether
//! the thing being used is in a state fit to be used. A record sitting at
//! `proposed` or `designed` is not registered, so composing with it is drift, and
//! drift authored before anyone asked whether the thing was registered is the
//! exact failure VDS S-6(2) describes.
//!
//! Three fatal rules and one warning:
//!
//!   R1  a governed component reference with no register record at all
//!   R2  a governed component reference whose record is not in an enforceable
//!       status (registered, built, verified)
//!   R3  a reference to a RETIRED component. VDS S-9(8) inverts the test after
//!       retirement: the code being there is the defect.
//!   W1  a reference to a DEPRECATED component. VDS S-9(6)(1) requires every
//!       consuming site to be reported, per site, by route, and a deprecated
//!       component never passes silently. It is captured as a warning-severity
//!       violation, counted and printed, and it does not fail the gate.
//!
//!   R4  a bare element ABOVE the primitive floor. S-9(10), settled by
//!       [2026] VJS-CC-VIBE-DESIGN-SYSTEM 5: the interactive tags a design
//!       system exists to govern are unregistered components wearing an
//!       element's name. Elements in [`BELOW_THE_FLOOR`] stay informational;
//!       carve-outs in `[surface] element_carveouts` are counted and named.
//!
//! This proof reads component NAMES, import PATHS and lifecycle STATUSES. It
//! reads no design value (VDS S-2(2)).

use std::io::Write;

use vds_core::{ProofKind, Result, Status, Violation};

use crate::ProofContext;
use crate::index::RegisterIndex;
use crate::run::{Outcome, Verdict};

pub const GATE: &str = "crates/vds-proof/src/composition.rs";

const RULE_UNREGISTERED: &str =
    "VDS S-7(5) composition R1: no screen uses an unregistered component";
const RULE_NOT_ENFORCEABLE: &str = "VDS S-7(5) composition R2 / S-5(4): the record exists and its status is not a registered state";
const RULE_RETIRED: &str =
    "VDS S-9(8) composition R3: after retirement the code being there is the defect";
const RULE_DEPRECATED: &str =
    "VDS S-9(6)(1) composition W1: a deprecated component never passes silently";
const RULE_ABOVE_FLOOR: &str = "VDS S-9(10) composition R4 ([2026] VJS-CC-VIBE-DESIGN-SYSTEM 5): a bare element above \
     the primitive floor is an unregistered component wearing an element's name";

/// The enumerated primitive floor (S-9(10)). Elements here are structure and
/// prose - the vocabulary HTML gives every page - and no design system
/// registers them as components. Everything else, above all the INTERACTIVE
/// tags, is what a design system exists to govern: a bare `<button>` in a
/// screen is the hand-rolled control the anti-drift proof was built to catch.
///
/// HELD HERE, in the gate's own source, deliberately: the enforcement lock pins
/// this file, so a change to the set is a re-pin with a rationale - the
/// "ordinary governed edit a diff can show" the order describes, in this
/// repository's strongest form. A per-project floor would let a subscriber
/// quietly widen it; a per-project CARVE-OUT (`[surface] element_carveouts`)
/// is the lawful pressure valve, and every use of one is counted by site.
pub const BELOW_THE_FLOOR: &[&str] = &[
    // Document structure.
    "div",
    "span",
    "main",
    "section",
    "article",
    "header",
    "footer",
    "nav",
    "aside",
    // Prose.
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "p",
    "blockquote",
    "pre",
    "code",
    "small",
    "strong",
    "em",
    "b",
    "i",
    "u",
    "s",
    "sup",
    "sub",
    "mark",
    "abbr",
    "cite",
    "q",
    "time",
    "br",
    "wbr",
    "hr",
    // Lists and definition lists.
    "ul",
    "ol",
    "li",
    "dl",
    "dt",
    "dd",
    // Tables as structure (a governed data table is a COMPONENT; the tags are not).
    "table",
    "thead",
    "tbody",
    "tfoot",
    "tr",
    "td",
    "th",
    "caption",
    "colgroup",
    "col",
    // Media and figures.
    "img",
    "picture",
    "source",
    "figure",
    "figcaption",
    "svg",
    "path",
    "circle",
    "rect",
    "line",
    "polyline",
    "polygon",
    "g",
    "defs",
    "use",
    "title",
    "desc",
    // The anchor: a plain hyperlink is prose. A Button-styled anchor is a
    // component, and that distinction is exactly what a registered Link or
    // Button component encodes - but the bare tag itself is below the floor
    // because forbidding `<a>` forbids writing a paragraph with a link in it.
    "a",
    // Plain labels and captions for forms are structure; the CONTROLS are not.
    "label",
    "legend",
    "fieldset",
    "form",
];

pub const RESERVED_NOTE: &str = "S-9(10) is SETTLED ([2026] VJS-CC-VIBE-DESIGN-SYSTEM 5, answering SUBMISSION-VDS-005): \
     elements in the enumerated floor are informational; a bare element above it fails R4 \
     unless named in [surface] element_carveouts, where every use is counted by site.";

pub fn run(ctx: &ProofContext, out: &mut dyn Write) -> Result<Outcome> {
    let project = ctx.project;
    let mut run = ctx.new_run(ProofKind::Composition, GATE);
    run.input_file(&project.config_path)?;

    let ledger = vds_scan::load_fresh(project)?;
    // The ledger's CONTENT digest, not its file digest: the file carries
    // `generated_at`, which moves on a no-op regeneration and would move this
    // proof's evidence digest with it (VDS S-7(2)(1)).
    run.input_named("<screens ledger content>", ledger.content_digest.clone());

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
    run.note(RESERVED_NOTE);

    for screen in &ledger.screens {
        for reference in &screen.references {
            let location = format!("{}:{} <{}>", screen.route, reference.line, reference.name);

            if reference.kind != vds_scan::ReferenceKind::Component {
                // Case-insensitive on the tag: JSX lower-cases intrinsics, but a
                // scanner that ever reported `IMG` must not push the tag above
                // the floor on spelling.
                let tag = reference.name.to_ascii_lowercase();
                if BELOW_THE_FLOOR.contains(&tag.as_str()) {
                    run.row(Verdict::Skipped("bare_element_below_the_floor_vds_s9_10"));
                    continue;
                }
                if project
                    .config
                    .surface
                    .element_carveouts
                    .iter()
                    .any(|c| c.eq_ignore_ascii_case(&tag))
                {
                    run.row(Verdict::Skipped("element_carveout_named_in_config"));
                    // Counted AND named per site, like the import carve-out
                    // above: a carve-out working as intended and one being used
                    // as an escape hatch produce the same count.
                    run.inform(Violation::fatal(
                        location.clone(),
                        RULE_ABOVE_FLOOR,
                        "a registered component, or this named carve-out",
                        format!(
                            "<{tag}> is above the primitive floor and carved out by \
                             [surface] element_carveouts. Nothing about it is governed"
                        ),
                    ));
                    continue;
                }
                run.row(Verdict::Enforced);
                run.fail(Violation::fatal(
                    location.clone(),
                    RULE_ABOVE_FLOOR,
                    format!(
                        "a registered component in place of the bare <{tag}>, or a named \
                         carve-out in [surface] element_carveouts"
                    ),
                    format!(
                        "<{tag}> is an interactive element above the primitive floor, used \
                         bare. A hand-rolled control is exactly the drift this proof exists \
                         to catch: nothing governs its states, its contrast or its keyboard \
                         contract, and every proof about the registered equivalent stays \
                         green while users get this instead"
                    ),
                ));
                continue;
            }

            let Some(import_path) = reference.import_path.as_deref() else {
                run.row(Verdict::Skipped(
                    "component_reference_with_no_resolvable_import",
                ));
                continue;
            };
            // Governed by EITHER the prefix as written or a relative specifier that
            // resolves inside a governed library directory. Rewriting one import to
            // `../../src/components/ui/widget` took a governed component out of
            // enforcement entirely, and the project declared `@/components/`, not that.
            if !reference.is_governed(prefixes, library_dirs) {
                run.row(Verdict::Skipped("import_outside_governed_prefixes"));
                // Name what escaped, per site. A carve-out being used as an
                // escape hatch and a carve-out working as intended produce the
                // same count, and only the first is a problem.
                run.inform(Violation::fatal(
                    location.clone(),
                    "VDS S-7(5) composition: bounded by [surface] governed_import_prefixes",
                    "an import inside a governed prefix, or one resolving into a governed \
                     library directory",
                    format!("imported from {import_path:?}, which is outside both"),
                ));
                continue;
            }

            run.row(Verdict::Enforced);

            // The EXPORT name, not the local one. An alias
            // (`import { Button as Btn }`) or a namespace member
            // (`<Icons.Chevron />`) makes the two differ, and looking up the
            // local name reports a registered component as unregistered, or
            // matches the wrong record entirely.
            let export_name = reference.lookup_name();
            let Some(record) = index.lookup(import_path, export_name) else {
                let misses = index.near_misses(import_path, export_name);
                let detail = if misses.is_empty() {
                    "no register record names it at all".to_owned()
                } else {
                    misses.join("; ")
                };
                run.fail(Violation::fatal(
                    location,
                    RULE_UNREGISTERED,
                    format!(
                        "a register record with code.importPath {import_path:?} and \
                         code.exportName {export_name:?}, in status one of registered, built, \
                         verified"
                    ),
                    format!("unregistered: no such record ({detail})"),
                ));
                continue;
            };

            match record.status {
                Status::Retired => run.fail(Violation::fatal(
                    location,
                    RULE_RETIRED,
                    format!("{} is retired, so no screen may reference it", record.id),
                    format!(
                        "{} status retired, retiredAt {:?}, still consumed here",
                        record.id,
                        record.retired_at.as_ref().map(|t| t.as_str())
                    ),
                )),
                Status::Deprecated => {
                    let successor = match &record.superseded_by {
                        Some(id) => id.to_string(),
                        None => "nothing (withdrawn outright)".to_owned(),
                    };
                    run.warn(Violation::fatal(
                        location,
                        RULE_DEPRECATED,
                        format!(
                            "no consuming site of {} ({:?}); migrate to {successor}",
                            record.id, record.name
                        ),
                        format!(
                            "{} deprecated at {:?}, superseded by {successor}, still consumed here",
                            record.id,
                            record.deprecated_at.as_ref().map(|t| t.as_str())
                        ),
                    ));
                }
                status if !status.is_enforceable() => run.fail(Violation::fatal(
                    location,
                    RULE_NOT_ENFORCEABLE,
                    format!(
                        "{} in status one of registered, built, verified before any screen \
                         composes with it",
                        record.id
                    ),
                    format!(
                        "{} status {status}: the record exists but the component is not \
                         registered, so this is drift",
                        record.id
                    ),
                )),
                _ => {}
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

    #[test]
    fn a_screen_using_only_registered_components_passes() {
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        h.register("Button", Status::Registered);
        h.ledger();
        let (outcome, _) = run_kind(&h, ProofKind::Composition);
        assert_eq!(outcome.status, ProofStatus::Passed);
        assert_eq!(outcome.exit_code, EXIT_PASSED);
        assert!(outcome.rows_enforced > 0);
    }

    /// The failing-direction test VDS S-7(2)(2) requires, and the one the
    /// enforcement lock names. It seeds a screen importing a component with no
    /// register record and asserts the non-zero exit.
    #[test]
    fn composition_fails_on_an_unregistered_component() {
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        h.ledger();
        let (outcome, text) = run_kind(&h, ProofKind::Composition);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert_eq!(outcome.status, ProofStatus::Failed);
        assert!(text.contains("unregistered"), "{text}");
        assert!(text.contains("app/dash/page.tsx"), "{text}");
    }

    #[test]
    fn composition_fails_on_a_component_that_is_only_proposed() {
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        h.register("Button", Status::Proposed);
        h.ledger();
        let (outcome, text) = run_kind(&h, ProofKind::Composition);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("this is drift"), "{text}");
    }

    /// VDS S-9(8): the test inverts after retirement. The code being there is
    /// the defect.
    #[test]
    fn composition_fails_on_a_retired_component() {
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        h.register("Button", Status::Retired);
        h.ledger();
        let (outcome, text) = run_kind(&h, ProofKind::Composition);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("retired"), "{text}");
    }

    #[test]
    fn a_deprecated_component_warns_by_route_and_does_not_fail_the_gate() {
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        h.register("Button", Status::Deprecated);
        h.ledger();
        let (outcome, text) = run_kind(&h, ProofKind::Composition);
        assert_eq!(outcome.exit_code, EXIT_PASSED, "{text}");
        assert!(text.contains("WARNINGS"), "{text}");
        assert!(text.contains("app/dash/page.tsx"), "{text}");

        let record = h.last_proof(ProofKind::Composition);
        assert_eq!(
            record.violations.len(),
            1,
            "a warning printed and not captured passes silently to anyone reading the record"
        );
        assert_eq!(record.violations[0].severity, Severity::Warning);
    }

    #[test]
    fn an_ungoverned_import_is_counted_and_not_enforced() {
        let h = Harness::new();
        h.screen_from("dash", &["Chart"], "third-party-charts");
        h.ledger();
        let (outcome, text) = run_kind(&h, ProofKind::Composition);
        assert_eq!(
            outcome.exit_code, EXIT_VACUOUS,
            "nothing was in scope, so the run proves nothing and says so: {text}"
        );
        assert!(text.contains("import_outside_governed_prefixes"), "{text}");
    }

    #[test]
    fn bare_elements_alone_make_the_run_vacuous_rather_than_passing() {
        let h = Harness::new();
        h.write(
            "app/plain/page.tsx",
            "export default function P(){ return <div><span /></div>; }\n",
        );
        h.ledger();
        let (outcome, text) = run_kind(&h, ProofKind::Composition);
        assert_eq!(outcome.status, ProofStatus::Vacuous, "{text}");
        assert!(
            text.contains("bare_element_below_the_floor_vds_s9_10"),
            "{text}"
        );
    }

    /// R4, both failing directions and both lawful exits, in one screen
    /// ([2026] VJS-CC-VIBE-DESIGN-SYSTEM 5 and task #49's seeded directions).
    #[test]
    fn a_bare_interactive_element_above_the_floor_fails_by_name() {
        let h = Harness::new();
        h.write(
            "app/settings/page.tsx",
            "export default function P(){ return <div><button>Save</button></div>; }\n",
        );
        h.ledger();
        let (outcome, text) = run_kind(&h, ProofKind::Composition);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("R4"), "{text}");
        assert!(
            text.contains("<button> is an interactive element above the primitive floor"),
            "the tag must be named, not counted: {text}"
        );
        assert!(
            text.contains("element_carveouts"),
            "the finding must name the lawful exit: {text}"
        );
        // The negative control inside the same screen: the <div> stayed below
        // the floor and produced no finding.
        assert!(
            !text.contains("<div> is an interactive"),
            "a below-floor element must never trip R4: {text}"
        );
    }

    /// A carve-out is a lawful exit that stays VISIBLE: skipped, counted, and
    /// reported by site - never a silent pass.
    #[test]
    fn a_carved_out_element_is_skipped_counted_and_named_by_site() {
        let h = Harness::new();
        let config = std::fs::read_to_string(h.root().join(".vds/config.toml")).unwrap();
        std::fs::write(
            h.root().join(".vds/config.toml"),
            config.replace("[surface]", "[surface]\nelement_carveouts = [\"button\"]"),
        )
        .unwrap();
        h.reload();
        // A registered component alongside the carve-out, so the run has an
        // enforceable row and the verdict is a PASS rather than vacuous - which
        // is also the shape of a real screen: mostly governed, one exception.
        h.register("Button", Status::Registered);
        h.write(
            "app/settings/page.tsx",
            "import { Button } from '@/components/ui';\n\
             export default function P(){ return <div><Button /><button>Save</button></div>; }\n",
        );
        h.ledger();
        let (outcome, text) = run_kind(&h, ProofKind::Composition);
        assert_eq!(outcome.exit_code, EXIT_PASSED, "{text}");
        assert!(text.contains("element_carveout_named_in_config"), "{text}");
        assert!(
            text.contains("carved out by"),
            "every carve-out use is reported by site, or an escape hatch and an \
             intended exception read identically: {text}"
        );
    }

    #[test]
    fn a_stale_ledger_is_a_precondition_failure_and_not_a_pass() {
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        h.register("Button", Status::Registered);
        h.ledger();
        h.screen("dash", &["Button", "Card"]);
        let error = h.run_kind_err(ProofKind::Composition);
        assert!(error.to_string().contains("STALE"), "{error}");
    }

    #[test]
    fn the_run_records_its_reliance_on_the_reserved_primitive_floor() {
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        h.register("Button", Status::Registered);
        h.ledger();
        run_kind(&h, ProofKind::Composition);
        let record = h.last_proof(ProofKind::Composition);
        assert!(
            record
                .notes
                .iter()
                .any(|n| n.contains("SUBMISSION-VDS-005")),
            "{:?}",
            record.notes
        );
    }
}
