//! The `reconciliation` proof. The register against the codebase, both
//! directions.
//!
//! VDS S-7(5): "the register agrees with Figma and with the codebase, both
//! directions". VDS S-5(5) records why the kind exists at all: this project's
//! `component-map.json` carries 56 component entries while the governed library
//! directories hold 90 `.tsx` files, "and nothing today derives either number
//! from the other or reconciles them. That gap is precisely where a register
//! rots." A one-directional check would close half of that gap and read as if it
//! had closed all of it.
//!
//! VDS S-5(6) enumerates four limbs. **This build reaches two of them**, and the
//! run says so in its own output rather than leaving a reader to infer it:
//!
//!   (a) CHECKED. A register entry with no resolvable code counterpart.
//!   (b) CHECKED. A component in a governed library directory with no register
//!       entry.
//!   (c) NOT CHECKED. Whether a register entry's Figma node id resolves in the
//!       pinned file. Resolving a node id is a call to the Figma API, and VDS
//!       S-7(2)(1) forbids a network call inside a proof. Every record is
//!       counted and skipped by name.
//!   (d) NOT CHECKED. Whether prop and state contracts agree between the record
//!       and the code. Comparing them is a TypeScript analysis of the component
//!       source, which this build does not have. Every record with a declared
//!       code counterpart is counted and skipped by name.
//!
//! docs/GOAL.md D1 lists all four, so a passing run of this proof must not be
//! readable as covering the two it never touched. Silent narrowing is the defect
//! VDS exists to catch, and a gate that narrows itself is the worst instance of
//! it.
//!
//! Two rules from elsewhere in the specification land here:
//!
//!   * VDS S-9(8) inverts limb (a) after retirement. A retired record's absence
//!     from the codebase is correct; its PRESENCE is the violation.
//!   * VDS S-5(7) makes a `demand` figure older than its ledger's generation
//!     stale, "and the reconciliation proof says so".
//!
//! This proof reads component NAMES, import PATHS, source-file PATHS, lifecycle
//! STATUSES and two timestamps. It opens no component source and reads no design
//! value (VDS S-2(2)).

use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use vds_core::{
    ComponentRecord, Digest, Project, ProofKind, Result, Status, Timestamp, VdsError, Violation,
};

use crate::ProofContext;
use crate::index::RegisterIndex;
use crate::run::{Outcome, ProofRun, Verdict};

pub const GATE: &str = "crates/vds-proof/src/reconciliation.rs";

const RULE_NO_CODE_COUNTERPART: &str =
    "VDS S-5(6)(a) reconciliation: a register entry with no resolvable code counterpart";
const RULE_BUILT_WITHOUT_CODE: &str = "VDS S-5(6)(a) / component-record schema: code is required non-null once status is built \
     or later";
const RULE_SOURCE_FILE_NOT_RELATIVE: &str =
    "VDS S-5(6)(a) / component-record schema: code.sourceFile is a repository-relative path";
const RULE_RETIRED_STILL_PRESENT: &str =
    "VDS S-9(8) reconciliation: after retirement the code being there is the defect";
const RULE_NO_REGISTER_ENTRY: &str = "VDS S-5(6)(b) reconciliation: a component in a governed library directory with no \
     register entry";
const RULE_STALE_DEMAND: &str =
    "VDS S-5(7): a demand figure older than its ledger's generation is stale";

/// Limb (c) is out of reach from inside a proof, and the reason is statutory
/// rather than a matter of effort.
pub const NOT_REACHED_FIGMA: &str = "[reach] limb (c) of VDS S-5(6), whether each register entry's Figma node id resolves in \
     the pinned file, is NOT checked by this run. Resolving a node id requires a call to the \
     Figma API and VDS S-7(2)(1) forbids a network call inside a proof, so every record was \
     counted and skipped rather than passed. A register entry naming a node that was deleted \
     from the decided-target file passes this run.";

/// Limb (d) is out of reach because this build has no TypeScript analysis, which
/// is a capability gap and is recorded as one.
pub const NOT_REACHED_CONTRACTS: &str = "[reach] limb (d) of VDS S-5(6), whether prop and state contracts agree between the record \
     and the code, is NOT checked by this run. Comparing them requires reading and analysing \
     the component's TypeScript source, which this build does not do, so every record with a \
     declared code counterpart was counted and skipped rather than passed. A record whose \
     props have drifted from its component's signature passes this run.";

/// The two limbs a warrant may actually rely on, stated in the run so the
/// warrant cannot be written wider than the evidence.
pub const REACH_SUMMARY: &str = "[reach] this run establishes limbs (a) and (b) of VDS S-5(6) over the declared surface, \
     and neither (c) nor (d). docs/GOAL.md D1 lists all four, and a warrant citing this proof \
     must not be described as covering the two it did not reach.";

pub const CARVE_OUT_NOTE: &str = "[carve-out] a library file named index.*, *.test.*, *.spec.* or *.stories.* is counted \
     and NOT enforced against the register under limb (b): a barrel re-exports components \
     rather than defining one, and a test or a story is not a shipped component. A component \
     defined inside a file with one of those names is therefore outside this proof, and the \
     skip counts above are where that carve-out is visible rather than assumed.";

// Skip reasons. Stable machine keys, never sentences: each becomes a count in
// `rows_skipped_reasons`, and a per-row sentence would make every count one.
const SKIP_UNBUILT: &str = "record_below_built_declares_no_code_counterpart";
const SKIP_FIGMA: &str = "figma_node_resolution_needs_network_vds_s7_2_1";
const SKIP_NO_FIGMA_NODE: &str = "record_declares_no_figma_node_to_resolve";
const SKIP_CONTRACTS: &str = "prop_and_state_parity_needs_typescript_analysis";
const SKIP_NO_CODE_TO_COMPARE: &str = "no_code_counterpart_to_compare_contracts_against";
const SKIP_DEMAND: &str = "demand_currency_reported_not_enforced_vds_s5_7";
const SKIP_BARREL: &str = "library_file_is_a_barrel_index";
const SKIP_TEST: &str = "library_file_is_a_test_or_spec";
const SKIP_STORY: &str = "library_file_is_a_story";

pub fn run(ctx: &ProofContext, out: &mut dyn Write) -> Result<Outcome> {
    let project = ctx.project;
    let mut run = ctx.new_run(ProofKind::Reconciliation, GATE);
    run.input_file(&project.config_path)?;

    let ledger = vds_scan::load_fresh(project)?;
    // The ledger's CONTENT digest, not its file digest: the file carries
    // `generated_at`, which moves on a no-op regeneration and would move this
    // proof's evidence digest with it (VDS S-7(2)(1)).
    run.input_named("<screens ledger content>", ledger.content_digest.clone());
    // And `generated_at` separately, because VDS S-5(7) compares a demand figure
    // against it. It is deliberately excluded from `content_digest`, so without
    // this line two runs could record identical inputs and report different
    // staleness findings: the determinism limb broken in the one direction a
    // reader would never think to check.
    run.input_named(
        "<screens ledger generated_at>",
        Digest::of_text(ledger.generated_at.as_str()),
    );

    let index = RegisterIndex::build(&ctx.store())?;
    for record in index.records() {
        run.input_file(&record.path)?;
    }

    let library = library_files(project)?;
    let names: Vec<String> = library.iter().map(|path| project.rel(path)).collect();
    // The PATHS, not the contents. This proof reads which files exist and what
    // they are called; digesting their bodies would move the evidence digest
    // every time a component's implementation changed, and make every warrant
    // citing this run look spent for a reason the run never read.
    run.input_named("<governed library file set>", Digest::of_value(&names)?);

    run.note(NOT_REACHED_FIGMA);
    run.note(NOT_REACHED_CONTRACTS);
    run.note(REACH_SUMMARY);
    run.note(CARVE_OUT_NOTE);
    if project.config.surface.library_dirs.is_empty() {
        run.note(
            "[surface] library_dirs is empty, so limb (b) of VDS S-5(6) walks nothing and can \
             enforce nothing. The direction that finds code with no register entry is not \
             covered by this run.",
        );
    }
    if index.is_empty() {
        run.note(
            "[register] the register holds no records, so limb (a) of VDS S-5(6) enforces \
             nothing. Every finding below is code with no register entry.",
        );
    }

    // Limb (a), the two unreachable limbs and the demand report, per register
    // entry.
    for located in index.records() {
        let record = &located.value;
        let record_at = project.rel(&located.path);
        code_counterpart_row(&mut run, project, record, &record_at);
        figma_row(&mut run, record);
        contract_row(&mut run, record);
        demand_row(&mut run, record, &ledger.generated_at, &record_at);
    }

    // Limb (b). A file named by ANY record is covered, including a retired one:
    // the retired case is already reported against the record under VDS S-9(8)
    // above, and reporting it a second time from the other direction makes one
    // defect look like two.
    let claimed: BTreeSet<String> = index
        .records()
        .iter()
        .filter_map(|located| located.value.code.as_ref())
        .map(|code| normalise(&code.source_file))
        .collect();

    for (path, relative) in library.iter().zip(&names) {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if let Some(reason) = carve_out(file_name) {
            run.row(Verdict::Skipped(reason));
            continue;
        }
        run.row(Verdict::Enforced);
        if !claimed.contains(&normalise(relative)) {
            run.fail(Violation::fatal(
                relative.clone(),
                RULE_NO_REGISTER_ENTRY,
                format!(
                    "a register record whose code.sourceFile is {relative:?}, so this file is \
                     covered by a contract and reconciles in both directions"
                ),
                "no register record names this file in code.sourceFile, so it ships from a \
                 governed library directory under no contract at all",
            ));
        }
    }

    run.finish(&ctx.capture_options()?, out)
}

/// Limb (a): the register entry resolves to code on disk.
///
/// The row is enforced wherever a FATAL finding is possible from it, which is
/// what `rows_enforced` has to mean for the vacuity test at VDS S-7(2)(4) to
/// answer the question it exists to answer: could this run have failed?
fn code_counterpart_row(
    run: &mut ProofRun<'_>,
    project: &Project,
    record: &ComponentRecord,
    record_at: &str,
) {
    let Some(code) = &record.code else {
        // The schema holds code to be "Required non-null once status is built or
        // later". A record claiming to be built while nothing in the codebase
        // answers to it is the register asserting a component that may not
        // exist, which is the class of gap limb (a) is for.
        if record.status.index() < Status::Built.index() {
            run.row(Verdict::Skipped(SKIP_UNBUILT));
            return;
        }
        run.row(Verdict::Enforced);
        run.fail(Violation::fatal(
            record_at.to_owned(),
            RULE_BUILT_WITHOUT_CODE,
            format!(
                "{} at status {} carries code with an importPath, a repository-relative \
                 sourceFile and an exportName",
                record.id, record.status
            ),
            format!(
                "{} is at status {} and code is null, so the register asserts a built \
                 component that nothing in the codebase answers to",
                record.id, record.status
            ),
        ));
        return;
    };

    run.row(Verdict::Enforced);
    let source = code.source_file.as_str();

    if !is_repository_relative(source) {
        // Left unchecked, `root.join("/etc/hostname")` discards the root
        // entirely and the presence test is satisfied by a file outside the
        // project. A check that can be satisfied from outside the repository is
        // not a check.
        run.fail(Violation::fatal(
            format!("{record_at} -> {source}"),
            RULE_SOURCE_FILE_NOT_RELATIVE,
            format!(
                "{}'s code.sourceFile is a path inside the repository, with no leading slash \
                 and no `..` segment",
                record.id
            ),
            format!(
                "{} names {source:?}, which leaves the project root. A counterpart resolved \
                 outside the repository is not this project's code",
                record.id
            ),
        ));
        return;
    }

    let present = project.root.join(source).is_file();
    match (record.status, present) {
        // VDS S-9(8): the test inverts after retirement.
        (Status::Retired, true) => run.fail(Violation::fatal(
            format!("{record_at} -> {source}"),
            RULE_RETIRED_STILL_PRESENT,
            format!(
                "{source} deleted, because {} is retired and a retired record's absence from \
                 the codebase is the correct state",
                record.id
            ),
            format!(
                "{} is retired (retiredAt {:?}) and {source} is still in the tree, so the code \
                 being there is the defect",
                record.id,
                record.retired_at.as_ref().map(|at| at.as_str())
            ),
        )),
        (Status::Retired, false) => {}
        (_, false) => run.fail(Violation::fatal(
            format!("{record_at} -> {source}"),
            RULE_NO_CODE_COUNTERPART,
            format!(
                "{source} present in the tree, exporting {:?} from {:?}",
                code.export_name, code.import_path
            ),
            format!(
                "{} at status {} names {source}, which is not a file. The entry has no \
                 resolvable code counterpart",
                record.id, record.status
            ),
        )),
        (_, true) => {}
    }
}

/// Limb (c). Counted, never enforced, and the reason is named per row so the
/// count is diagnosable rather than a single undifferentiated skip.
fn figma_row(run: &mut ProofRun<'_>, record: &ComponentRecord) {
    match record.figma {
        Some(_) => run.row(Verdict::Skipped(SKIP_FIGMA)),
        None => run.row(Verdict::Skipped(SKIP_NO_FIGMA_NODE)),
    }
}

/// Limb (d). Counted, never enforced, for the same reason.
fn contract_row(run: &mut ProofRun<'_>, record: &ComponentRecord) {
    match record.code {
        Some(_) => run.row(Verdict::Skipped(SKIP_CONTRACTS)),
        None => run.row(Verdict::Skipped(SKIP_NO_CODE_TO_COMPARE)),
    }
}

/// VDS S-5(7): a `demand` figure older than its ledger's generation is stale,
/// and this proof says so.
///
/// It says so as a captured WARNING, per record, and both halves of that are
/// deliberate. VDS S-5(6) enumerates the four sets whose non-emptiness is a
/// violation and demand staleness is not among them; S-5(7) requires the proof
/// to say so, which a counted, printed, per-record finding does.
///
/// The row is counted and NOT enforced for a separate reason: a row whose only
/// possible finding cannot fail the gate could not have made this run fail, so
/// counting it towards `rows_enforced` would let a register full of stale
/// figures report a non-vacuous reconciliation while neither direction of VDS
/// S-5(6) was checked at all. That is the [2026] VJS-CC-OPBOX 3 D3 defect
/// wearing a different hat.
fn demand_row(
    run: &mut ProofRun<'_>,
    record: &ComponentRecord,
    ledger_generated_at: &Timestamp,
    record_at: &str,
) {
    run.row(Verdict::Skipped(SKIP_DEMAND));
    if &record.demand.measured_at >= ledger_generated_at {
        return;
    }
    run.warn(Violation::fatal(
        record_at.to_owned(),
        RULE_STALE_DEMAND,
        format!(
            "{}'s demand re-measured by {:?} at or after the screens ledger was generated, so \
             the figure is about the inventory that exists now",
            record.id, record.demand.measured_by
        ),
        format!(
            "{} carries demand.routes {} measured at {}, and the screens ledger was generated \
             at {}. The figure predates the inventory it describes, so build order and \
             retirement drain would be decided on a number nobody has re-measured",
            record.id, record.demand.routes, record.demand.measured_at, ledger_generated_at
        ),
    ));
}

/// Why a library file is counted and not enforced against the register, or
/// `None` where it is a component module like any other.
fn carve_out(file_name: &str) -> Option<&'static str> {
    if file_name.contains(".test.") || file_name.contains(".spec.") {
        return Some(SKIP_TEST);
    }
    if file_name.contains(".stories.") {
        return Some(SKIP_STORY);
    }
    if file_name.starts_with("index.") {
        return Some(SKIP_BARREL);
    }
    None
}

/// Every component module in the governed library directories, sorted.
///
/// A configured directory that is not there is a PRECONDITION failure and not an
/// empty result. Limb (b) exists to find code the register does not cover, and a
/// directory that yields no rows because it was moved or renamed would report a
/// clean run over a surface smaller than the configuration declares, with
/// nothing saying so.
fn library_files(project: &Project) -> Result<Vec<PathBuf>> {
    let surface = &project.config.surface;
    let mut patterns = Vec::new();

    if !surface.library_dirs.is_empty() && surface.component_extensions.is_empty() {
        // A contradiction rather than a narrow surface, so it is refused rather
        // than noted: the configuration names directories to cover and then
        // admits no file in them as a component module, which makes limb (b)
        // enumerate nothing while the config reads as though it covers
        // everything.
        return Err(VdsError::precondition(
            "[surface] library_dirs names directories to cover and component_extensions is \
             empty, so no file in any of them is treated as a component module and limb (b) \
             of VDS S-5(6) would enumerate nothing. Name the extensions, or empty \
             library_dirs and take the narrowing on the record.",
        ));
    }

    for configured in &surface.library_dirs {
        let directory = configured.trim().trim_end_matches('/');
        if directory.is_empty() {
            return Err(VdsError::precondition(
                "[surface] library_dirs holds an empty entry. An empty directory name matches \
                 nothing and reads like a directory name that matches everything.",
            ));
        }
        if !project.root.join(directory).is_dir() {
            return Err(VdsError::precondition(format!(
                "[surface] library_dirs names {directory:?}, which is not a directory in this \
                 project. Limb (b) of VDS S-5(6) walks these directories to find code with no \
                 register entry, so a directory that is not there yields no rows and the run \
                 would report a surface smaller than the configuration declares. Fix the path, \
                 or remove it from library_dirs and take the narrowing on the record."
            )));
        }
        for configured_extension in &surface.component_extensions {
            let extension = configured_extension.trim().trim_start_matches('.');
            if extension.is_empty() {
                return Err(VdsError::precondition(
                    "[surface] component_extensions holds an empty entry, which would match \
                     every file in a library directory rather than none.",
                ));
            }
            patterns.push(format!("{directory}/**/*.{extension}"));
        }
    }

    if patterns.is_empty() {
        return Ok(Vec::new());
    }
    vds_scan::glob::match_globs(&project.root, &patterns)
}

/// One spelling for a repository-relative path, so `./a/b.tsx` and `a/b.tsx` are
/// the same file rather than two rows that never meet.
fn normalise(path: &str) -> String {
    path.replace('\\', "/").trim_start_matches("./").to_owned()
}

fn is_repository_relative(source_file: &str) -> bool {
    let path = Path::new(source_file);
    !source_file.trim().is_empty()
        && !source_file.starts_with('/')
        && !path.is_absolute()
        && !path
            .components()
            .any(|component| component == Component::ParentDir)
}

#[cfg(test)]
mod tests {
    use crate::testing::{Harness, run_kind};
    use vds_core::{
        EXIT_PASSED, EXIT_VACUOUS, EXIT_VIOLATION, FigmaNode, ProofKind, ProofStatus, Severity,
        Status, Timestamp, default_config,
    };

    /// Put every record's `demand` at the ledger's generation, so a test about
    /// limb (a) or limb (b) is not also a test about VDS S-5(7).
    fn freshen_demand(h: &Harness) {
        let ledger = vds_scan::load_fresh(&h.project()).expect("a fresh ledger");
        for record in h.store().read_register().unwrap() {
            let mut value = record.value;
            value.demand.measured_at = ledger.generated_at.clone();
            h.replace(value);
        }
    }

    /// Rewrite the ledger's `generated_at` in place.
    ///
    /// It is excluded from `content_digest` by design, so the ledger stays FRESH
    /// through this edit. That is exactly the property the determinism test
    /// below needs, and exactly the trap the `generated_at` input line guards.
    fn set_ledger_generated_at(h: &Harness, at: &str) {
        let text = h.read(".vds/ledgers/screens.yaml");
        let edited: Vec<String> = text
            .lines()
            .map(|line| {
                if line.starts_with("generated_at:") {
                    format!("generated_at: '{at}'")
                } else {
                    line.to_owned()
                }
            })
            .collect();
        h.write(
            ".vds/ledgers/screens.yaml",
            &format!("{}\n", edited.join("\n")),
        );
    }

    #[test]
    fn a_register_and_a_library_that_agree_pass() {
        let h = Harness::new();
        h.register("Button", Status::Registered);
        h.component_file("Button");
        h.ledger();
        freshen_demand(&h);

        let (outcome, text) = run_kind(&h, ProofKind::Reconciliation);
        assert_eq!(outcome.exit_code, EXIT_PASSED, "{text}");
        assert_eq!(outcome.status, ProofStatus::Passed);
        assert_eq!(
            outcome.rows_enforced, 2,
            "one row each way: the entry resolves to code, and the file is claimed by an \
             entry. {text}"
        );
    }

    /// The failing-direction test VDS S-7(2)(2) requires, and the one
    /// `.vds/enforcement.lock` names. It seeds a component file in a governed
    /// library directory that no register record claims, which is the "56
    /// entries against 90 files" gap of VDS S-5(5), and asserts the non-zero
    /// exit.
    #[test]
    fn reconciliation_fails_on_a_library_file_with_no_register_entry() {
        let h = Harness::new();
        h.component_file("Orphan");
        h.ledger();

        let (outcome, text) = run_kind(&h, ProofKind::Reconciliation);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert_eq!(outcome.status, ProofStatus::Failed);
        assert!(text.contains("src/components/ui/orphan.tsx"), "{text}");
        assert!(
            text.contains("no register record names this file"),
            "{text}"
        );
    }

    #[test]
    fn reconciliation_fails_on_a_register_entry_whose_source_file_is_absent() {
        let h = Harness::new();
        h.register("Button", Status::Registered);
        h.ledger();
        freshen_demand(&h);

        let (outcome, text) = run_kind(&h, ProofKind::Reconciliation);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("no resolvable code counterpart"), "{text}");
        assert!(text.contains("src/components/ui/button.tsx"), "{text}");
    }

    #[test]
    fn reconciliation_fails_on_a_built_record_carrying_no_code_counterpart() {
        let h = Harness::new();
        h.register_unbuilt("Sketch", Status::Built);
        h.ledger();
        freshen_demand(&h);

        let (outcome, text) = run_kind(&h, ProofKind::Reconciliation);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("code is null"), "{text}");
        assert!(text.contains("required non-null"), "{text}");
    }

    #[test]
    fn a_record_below_built_with_no_code_counterpart_is_counted_and_not_enforced() {
        let h = Harness::new();
        h.register_unbuilt("Sketch", Status::Designed);
        h.ledger();
        freshen_demand(&h);

        let (_, text) = run_kind(&h, ProofKind::Reconciliation);
        assert!(
            text.contains("record_below_built_declares_no_code_counterpart"),
            "{text}"
        );
    }

    /// VDS S-9(8): after retirement the code being there is the defect.
    #[test]
    fn reconciliation_fails_on_a_retired_component_still_in_the_library() {
        let h = Harness::new();
        h.register("Button", Status::Retired);
        h.component_file("Button");
        h.ledger();
        freshen_demand(&h);

        let (outcome, text) = run_kind(&h, ProofKind::Reconciliation);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(
            text.contains("the code being there is the defect"),
            "{text}"
        );
    }

    #[test]
    fn a_retired_component_absent_from_the_library_is_the_correct_state() {
        let h = Harness::new();
        h.register("Button", Status::Retired);
        h.component_file("Keeper");
        h.register_as("CMP-0002", "Keeper", "Keeper", Status::Registered);
        h.ledger();
        freshen_demand(&h);

        let (outcome, text) = run_kind(&h, ProofKind::Reconciliation);
        assert_eq!(
            outcome.exit_code, EXIT_PASSED,
            "a retired record's absence from the codebase is correct (VDS S-9(8)): {text}"
        );
    }

    #[test]
    fn reconciliation_fails_on_a_source_file_that_leaves_the_repository() {
        let h = Harness::new();
        let id = h.register("Button", Status::Registered);
        h.amend(&id, |record| {
            record.code.as_mut().unwrap().source_file = "/etc/hostname".into();
        });
        h.ledger();
        freshen_demand(&h);

        let (outcome, text) = run_kind(&h, ProofKind::Reconciliation);
        assert_eq!(
            outcome.exit_code, EXIT_VIOLATION,
            "a presence test satisfied by a file outside the repository is not a test: {text}"
        );
        assert!(text.contains("leaves the project root"), "{text}");
    }

    #[test]
    fn a_barrel_a_test_and_a_story_are_counted_and_not_enforced() {
        let h = Harness::new();
        h.register("Button", Status::Registered);
        h.component_file("Button");
        h.write(
            "src/components/ui/index.tsx",
            "export * from \"./button\";\n",
        );
        h.write(
            "src/components/ui/button.test.tsx",
            "test(\"x\", () => {});\n",
        );
        h.write(
            "src/components/ui/button.spec.tsx",
            "test(\"x\", () => {});\n",
        );
        h.write(
            "src/components/ui/button.stories.tsx",
            "export const A = {};\n",
        );
        h.ledger();
        freshen_demand(&h);

        let (outcome, text) = run_kind(&h, ProofKind::Reconciliation);
        assert_eq!(
            outcome.exit_code, EXIT_PASSED,
            "none of the four is a component with a missing register entry: {text}"
        );
        assert!(text.contains("library_file_is_a_barrel_index: 1"), "{text}");
        assert!(text.contains("library_file_is_a_test_or_spec: 2"), "{text}");
        assert!(text.contains("library_file_is_a_story: 1"), "{text}");
        assert!(text.contains("[carve-out]"), "{text}");
    }

    /// VDS S-5(7). The finding is captured, not merely printed: a warning nobody
    /// captured passes silently the moment a reader opens the record instead of
    /// the terminal.
    #[test]
    fn a_demand_figure_older_than_the_ledger_is_reported_as_stale() {
        let h = Harness::new();
        let id = h.register("Button", Status::Registered);
        h.component_file("Button");
        h.ledger();
        freshen_demand(&h);
        h.amend(&id, |record| {
            record.demand.measured_at = Timestamp::fixed(2020, 1, 1, 0, 0, 0);
            record.demand.routes = 7;
        });

        let (outcome, text) = run_kind(&h, ProofKind::Reconciliation);
        assert_eq!(
            outcome.exit_code, EXIT_PASSED,
            "VDS S-5(6) does not make staleness one of the four sets whose non-emptiness is a \
             violation, and S-5(7) requires the proof to say so, which it does: {text}"
        );
        assert!(
            text.contains("predates the inventory it describes"),
            "{text}"
        );

        let record = h.last_proof(ProofKind::Reconciliation);
        let stale: Vec<_> = record
            .violations
            .iter()
            .filter(|violation| violation.rule.contains("S-5(7)"))
            .collect();
        assert_eq!(stale.len(), 1, "{:?}", record.violations);
        assert_eq!(stale[0].severity, Severity::Warning);
    }

    #[test]
    fn a_demand_figure_measured_at_the_ledger_generation_is_not_stale() {
        let h = Harness::new();
        h.register("Button", Status::Registered);
        h.component_file("Button");
        h.ledger();
        freshen_demand(&h);

        run_kind(&h, ProofKind::Reconciliation);
        let record = h.last_proof(ProofKind::Reconciliation);
        assert!(
            !record
                .violations
                .iter()
                .any(|violation| violation.rule.contains("S-5(7)")),
            "{:?}",
            record.violations
        );
    }

    /// VDS S-7(2)(4). Rows were considered and none was enforceable, so the run
    /// proves nothing and says so rather than printing a pass.
    #[test]
    fn a_register_of_unbuilt_records_over_an_empty_library_is_vacuous() {
        let h = Harness::new();
        h.register_unbuilt("Sketch", Status::Designed);
        h.ledger();
        freshen_demand(&h);

        let (outcome, text) = run_kind(&h, ProofKind::Reconciliation);
        assert_eq!(outcome.status, ProofStatus::Vacuous, "{text}");
        assert_eq!(outcome.exit_code, EXIT_VACUOUS);
        assert!(outcome.rows_considered > 0, "{text}");
        assert!(!text.contains("PASS:"), "no PASS beside a vacuity: {text}");
    }

    /// The two limbs this build cannot reach must be unmissable in the record,
    /// or a reader takes a passing reconciliation for all four of VDS S-5(6).
    #[test]
    fn the_run_records_the_two_limbs_it_does_not_reach() {
        let h = Harness::new();
        let id = h.register("Button", Status::Registered);
        h.component_file("Button");
        h.amend(&id, |record| {
            record.figma = Some(FigmaNode {
                file_key: "FILEKEY".into(),
                node_id: "12:34".into(),
                captured_at: Timestamp::fixed(2026, 7, 25, 10, 0, 0),
            });
        });
        h.ledger();
        freshen_demand(&h);

        let (_, text) = run_kind(&h, ProofKind::Reconciliation);
        assert!(text.contains("limb (c)"), "{text}");
        assert!(text.contains("limb (d)"), "{text}");

        let record = h.last_proof(ProofKind::Reconciliation);
        assert!(
            record
                .notes
                .iter()
                .any(|note| note.contains("network call")),
            "{:?}",
            record.notes
        );
        assert!(
            record.notes.iter().any(|note| note.contains("TypeScript")),
            "{:?}",
            record.notes
        );
        assert!(
            record
                .notes
                .iter()
                .any(|note| note.contains("docs/GOAL.md D1")),
            "{:?}",
            record.notes
        );
        assert_eq!(
            record
                .rows_skipped_reasons
                .get("figma_node_resolution_needs_network_vds_s7_2_1"),
            Some(&1),
            "an unreachable limb is COUNTED, not silently dropped"
        );
        assert_eq!(
            record
                .rows_skipped_reasons
                .get("prop_and_state_parity_needs_typescript_analysis"),
            Some(&1)
        );
    }

    #[test]
    fn a_stale_ledger_is_a_precondition_failure_and_not_a_pass() {
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        h.register("Button", Status::Registered);
        h.component_file("Button");
        h.ledger();
        h.screen("dash", &["Button", "Card"]);

        let error = h.run_kind_err(ProofKind::Reconciliation);
        assert!(error.to_string().contains("STALE"), "{error}");
    }

    /// A configured library directory that is not there must not read as a clean
    /// run over an empty directory.
    #[test]
    fn a_library_directory_that_does_not_exist_is_a_precondition_failure() {
        let h = Harness::with_config(&default_config("demo", "DEMO").replace(
            r#"library_dirs = ["src/components/ui"]"#,
            r#"library_dirs = ["src/components/absent"]"#,
        ));
        h.ledger();

        let error = h.run_kind_err(ProofKind::Reconciliation);
        assert!(
            error.to_string().contains("src/components/absent"),
            "{error}"
        );
        assert!(
            error
                .to_string()
                .contains("smaller than the configuration declares"),
            "{error}"
        );
    }

    /// Directories to cover and no extension that counts as a component is a
    /// contradiction, and a contradiction that silently narrows limb (b) to
    /// nothing is the defect this proof exists to catch, turned on itself.
    #[test]
    fn library_dirs_with_no_component_extensions_is_a_precondition_failure() {
        let h = Harness::with_config(&default_config("demo", "DEMO").replace(
            r#"component_extensions = ["tsx", "jsx"]"#,
            "component_extensions = []",
        ));
        h.ledger();

        let error = h.run_kind_err(ProofKind::Reconciliation);
        assert!(
            error.to_string().contains("component_extensions is empty"),
            "{error}"
        );
    }

    #[test]
    fn an_empty_library_dirs_list_says_that_limb_b_enforces_nothing() {
        let h = Harness::with_config(&default_config("demo", "DEMO").replace(
            r#"library_dirs = ["src/components/ui"]"#,
            "library_dirs = []",
        ));
        h.register("Button", Status::Registered);
        h.component_file("Button");
        h.ledger();
        freshen_demand(&h);

        let (_, text) = run_kind(&h, ProofKind::Reconciliation);
        assert!(text.contains("library_dirs is empty"), "{text}");
        assert!(text.contains("enforce nothing"), "{text}");
    }

    /// VDS S-7(2)(1). The demand check reads the ledger's generation time, and
    /// that time is excluded from `content_digest`, so a run that did not record
    /// it separately could report two different finding sets over inputs it
    /// declares identical.
    #[test]
    fn the_generation_time_the_demand_check_reads_is_recorded_as_an_input() {
        let h = Harness::new();
        h.register("Button", Status::Registered);
        h.component_file("Button");
        h.ledger();
        freshen_demand(&h);

        run_kind(&h, ProofKind::Reconciliation);
        let before = h.last_proof(ProofKind::Reconciliation);
        assert!(
            !before
                .violations
                .iter()
                .any(|violation| violation.rule.contains("S-5(7)")),
            "{:?}",
            before.violations
        );

        // The ledger stays fresh through this edit, because `content_digest`
        // excludes `generated_at`.
        set_ledger_generated_at(&h, "2030-01-01T00:00:00Z");
        let (_, text) = run_kind(&h, ProofKind::Reconciliation);
        let after = h.last_proof(ProofKind::Reconciliation);

        assert!(
            text.contains("predates the inventory it describes"),
            "{text}"
        );
        assert_ne!(
            before.inputs_digest, after.inputs_digest,
            "the finding set moved, so the recorded inputs must move with it, or the record \
             claims two different answers over one set of inputs"
        );
    }
}
