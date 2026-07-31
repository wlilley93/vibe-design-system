//! The `parity` proof. The register's contract against the code that ships.
//!
//! VDS S-7(5): "each registered component's code counterpart matches its props
//! and states contract". This is the W4 gate (VDS S-6(2)), the one granted when
//! the system is done, so it is the last place a contract and its implementation
//! can be allowed to disagree.
//!
//! It is also the kind whose recorded reason for being unimplemented said the
//! comparison "is a TypeScript analysis and not a digest comparison". That
//! remains true, and the analysis now exists: [`vds_scan::library`] locates every
//! exported component in the configured library directories and reads its prop
//! names, type expressions and requiredness. This proof consumes that and does
//! not repeat it, because two readers of one TypeScript file are two opinions
//! about one shape and two opinions drift (VDS S-4(1) says the same thing about
//! schemas).
//!
//! ## Both directions, and why the second one is the load-bearing half
//!
//! Checking that every contracted prop exists in the code makes the register a
//! SUBSET of the component: the component may accept anything else it likes and
//! the gate is silent. A prop nobody contracted is exactly how a component
//! drifts, so the reverse direction is fatal too (R6). That is the difference
//! between a contract and a wish list.
//!
//! ## The rules
//!
//! Fatal:
//!
//!   R1  a record at `registered` or later whose `code` counterpart is null.
//!       Parity is the evidence W4 rests on, and a registered component with no
//!       built counterpart is precisely what W4 must not be granted over. This
//!       is the one place parity is stricter than `reconciliation`, which lets a
//!       record below `built` carry no code at all.
//!   R2  `code.sourceFile` leaves the repository. Left unchecked,
//!       `root.join("/etc/hostname")` discards the root and the presence test is
//!       satisfied by a file nobody in this project wrote.
//!   R3  `code.sourceFile` is not a file in the tree.
//!   R4  the scanner read that file, found exported components, and none of them
//!       is `code.exportName`.
//!   R5  the contract names a prop the component does not accept.
//!   R6  the component accepts a prop the contract does not name.
//!   R7  the contract and the component disagree about whether a prop is
//!       required.
//!   R8  the two type expressions disagree after the normalisation named below.
//!   R9  the contract requires a state that `states.built` does not carry. VDS
//!       S-7(5) gives that gap to this kind by name, and `states` records it as
//!       informational precisely because this is the gate for it: if parity
//!       declined it too, nothing would ever fail on it.
//!   R10 the contract names one prop twice. Two entries on one name collapse
//!       into one lookup, so the second contract silently never gets compared.
//!
//! Warning, printed and captured, never fatal:
//!
//!   W1  the source file yielded no exported component at all to the scanner.
//!   W2  the contract names props and the source declares no `<Name>Props` type,
//!       so there is nothing to compare them against.
//!
//! Informational, captured and not printed:
//!
//!   I1  a prop type comparison this build cannot decide.
//!   I2  a record whose source file lies outside the scanned library directories.
//!   I3  a source file that declares no `<Name>Props` type where the contract
//!       names no prop either, so nothing is claimed in either direction.
//!
//! ## One row per registered component, and what a row being ENFORCED means
//!
//! A row is enforced where the comparison actually happened or definitively
//! failed, and skipped with a named reason where this build could not tell either
//! way. Nothing in between: a row that could not have produced a fatal finding
//! must not count towards `rows_enforced`, or a register whose sources this build
//! cannot read reports a confident pass over nothing, which is the
//! [2026] VJS-CC-OPBOX 3 D3 defect (VDS S-7(2)(4)). Where every row is skipped
//! the run is VACUOUS and is not evidence for anything.
//!
//! ## A finding never repeats a type expression
//!
//! A type expression can carry a design realisation: `'sm' | 'md'` cannot, and a
//! union of two length literals or two colour literals can. A captured proof
//! record lands under `.vds/`, which is the tree `no_stored_values` scans, so a
//! finding that copied a type expression could put a realisation there
//! permanently and that gate would then fail forever on a file this one wrote
//! (VDS S-2(2), S-2(8)). Findings therefore name the prop, the record, the source
//! file and the SHAPE of the disagreement in counts, and the reader opens the two
//! named places. This is the same rule, and the same reason, as
//! [`crate::no_stored_values::REDACTION_NOTE`].
//!
//! Everything else this proof reads is a prop NAME, a lifecycle STATUS, a state
//! NAME and a path. None of those is a realisation (VDS S-2(4)).

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Component, Path};

use vds_core::{
    ComponentRecord, Drift, Project, ProofKind, PropContract, Result, State, Status, VdsError,
    Violation,
};
use vds_scan::library::{LibraryExport, LibraryProp, scan_library};

use crate::ProofContext;
use crate::index::RegisterIndex;
use crate::run::{Outcome, ProofRun, Verdict};

pub const GATE: &str = "crates/vds-proof/src/parity.rs";

const RULE_NO_CODE: &str = "VDS S-7(5) parity R1 / S-6(2) W4: a registered component has a code counterpart to \
     compare its contract against";
const RULE_SOURCE_NOT_RELATIVE: &str =
    "VDS S-7(5) parity R2: code.sourceFile is a repository-relative path";
const RULE_SOURCE_ABSENT: &str = "VDS S-7(5) parity R3: code.sourceFile is a file in the tree";
const RULE_EXPORT_ABSENT: &str =
    "VDS S-7(5) parity R4: code.exportName is exported by code.sourceFile";
const RULE_REGISTRY_KEYS: &str = "VDS S-7(5) parity R4 (registry arm, [2026] VJS-CC-VIBE-DESIGN-SYSTEM 2): a \
     variant-registry module's keys equal the record's variant union exactly";
const RULE_PROP_NOT_IN_CODE: &str =
    "VDS S-7(5) parity R5: every prop the contract names is accepted by the component";
const RULE_PROP_NOT_CONTRACTED: &str =
    "VDS S-7(5) parity R6: every prop the component accepts is named by the contract";
const RULE_REQUIREDNESS: &str =
    "VDS S-7(5) parity R7: the contract and the component agree on whether a prop is required";
const RULE_TYPE_DISAGREES: &str =
    "VDS S-7(5) parity R8: the contract and the component agree on a prop's type expression";
const RULE_STATE_NOT_BUILT: &str =
    "VDS S-7(5) parity R9: every state the contract requires is carried by states.built";
const RULE_VARIANT_ABSENT: &str =
    "VDS S-7(5) parity R11: a prop's figmaProperty names a variant property the frame declares";
const RULE_VARIANT_VALUES: &str = "VDS S-7(5) parity R12: a prop's legal values and its variant property's legal values are one set";
const RULE_DUPLICATE_PROP: &str =
    "VDS S-5(2) parity R10: a contract names each prop once, so each one is compared";
const RULE_NO_EXPORT_FOUND: &str = "VDS S-7(5) parity W1: the scanner read code.sourceFile and found no exported component in \
     it, so nothing was compared";
const RULE_NO_PROPS_TYPE: &str = "VDS S-7(5) parity W2: the contract names props and the component's source declares no \
     `<Name>Props` type to compare them against";
const RULE_OUTSIDE_LIBRARY: &str = "VDS S-7(5) parity I2: bounded by [surface] library_dirs, which is the only source this \
     proof reads";
const RULE_CARVED_OUT: &str = "VDS S-7(5) parity I2: bounded by the library scan's carve-out, which reads a barrel, a test, \
     a story and a declaration file as something other than a component module";

// Skip reasons. Stable machine keys and never sentences: each becomes a count in
// `rows_skipped_reasons`, and a per-row sentence would make every count one.
const SKIP_BELOW_REGISTERED: &str = "record_below_registered_is_a_candidate_not_a_contract";
const SKIP_RETIRED: &str = "retired_tombstone_absence_from_the_code_is_correct_vds_s9_8";
const SKIP_OUTSIDE_LIBRARY: &str = "source_file_outside_the_scanned_library_dirs";
const SKIP_CARVED_OUT: &str = "source_file_carved_out_of_the_library_scan_by_name";
const SKIP_NO_EXPORT_FOUND: &str = "source_file_yielded_no_exported_component_to_the_scanner";
const SKIP_NO_PROPS_TYPE: &str = "component_source_declares_no_props_type_to_compare";

// Reasons a single prop's TYPE comparison was not decided. Counted per reason
// and reported, because an undecided comparison that printed nothing would be
// indistinguishable from an agreement.
const UNDECIDED_UNTERMINATED: &str = "type_expression_has_an_unterminated_string_literal";
const UNDECIDED_QUOTING: &str = "members_agree_but_one_side_quotes_them_as_string_literals";

pub const SCOPE_NOTE: &str = "[scope] only records at `registered` or later are compared. VDS S-5(4) makes the lifecycle a \
     directed path in which `registered` is the point a contract becomes complete and binding, so \
     a `proposed` or `designed` record is a candidate and there is no contract to hold the code \
     to. A `retired` record is a tombstone kept forever (VDS S-9(6)(3)) and its ABSENCE from the \
     code is the correct state (VDS S-9(8)), so comparing a contract against a file that ought to \
     be gone would invert the rule. A `deprecated` record is still shipped until it drains \
     (VDS S-9(6)(2)) and is compared like any other.";

pub const NORMALISATION_NOTE: &str = "[normalisation] two type expressions are compared as TOKEN SEQUENCES, not as strings. The \
     comparison normalises exactly three things and nothing else: all whitespace between tokens, \
     the quote character around a string literal, and the ORDER of the members of a top-level \
     union. Everything else is a difference. Nested unions inside a generic or a function type \
     are NOT reordered, and `T | undefined` is not folded into an optional `T`, because under \
     exactOptionalPropertyTypes those are different contracts and folding them would erase a \
     distinction the author may have made deliberately. Exact string equality was rejected \
     because it reports a reordered union and a re-wrapped line as drift, and a gate that cries \
     wolf gets disabled.";

pub const EXPORT_LIMB_NOTE: &str = "[export-limb] a record naming an export that the scanner READ THE FILE and did not find \
     among the exports it did find is a fatal finding. A record whose source file yielded NO \
     export at all is counted, skipped and warned about instead. The scanner is not a TypeScript \
     compiler, so an export style it does not recognise would otherwise become a fatal finding \
     against code that is perfectly correct, and that is the direction in which a gate gets \
     turned off.";

pub const PROPS_REACH_NOTE: &str = "[props-reach] the prop list comes from a `<Name>Props` interface or type alias in the same \
     file as the export. The scanner does not follow an `extends`, does not resolve a mapped or \
     utility type, does not read a props type declared in another file and does not expand a \
     spread of another component's props. Where it finds no such type the row is counted, \
     skipped and reported rather than compared, because a contract measured against a prop list \
     that is empty for want of a reader would fail on every prop it names.";

pub const STATES_REACH_NOTE: &str = "[states-reach] the states limb compares the record's `states.required` against the record's \
     own `states.built`. It does NOT establish that the component implements a state: deciding \
     from a static read of a TSX file whether hover, focus or loading is really implemented would \
     need an analysis of styles, variants and event handling that this build does not do, and a \
     heuristic that guessed would be confidently wrong. A pass therefore establishes that the \
     contract's required set is covered by the record's own built claim, and never that the claim \
     is true. The limb runs only on a row that was enforced, so a row skipped for want of a \
     readable source carries no states finding either.";

pub const FIGMA_NOTE_UNREACHED: &str = "[figma] a prop's `figmaProperty` names a variant property in the decided-target file, and \
     NO figma ledger is on disk, so R11 and R12 are unreached and every such prop is skipped. \
     Resolving one live is a call to the Figma API and VDS S-7(2)(1) forbids a network call \
     inside a proof; `vds figma pull` generates the ledger out of band, and a ledger on disk is \
     something this proof may read offline. Until it exists, a contract whose figmaProperty \
     names a variant that was deleted from the file passes this run.";

pub const FIGMA_NOTE_REACHED: &str = "[figma] a figma ledger is on disk and was relied on, so R11 and R12 ran. This limb \
     compares the LEGAL VALUES of a prop against the legal values of the variant property it \
     names: `intent: 'success' | 'warning'` in code and `Intent: Success, Warning` in the frame \
     are two spellings of one set, and until now nothing compared them. What it establishes is \
     bounded by the ledger's freshness, which is `ledger_staleness`'s to hold, and by the fact \
     that a variant property the pull could not read is absent rather than empty.";

pub const FIGMA_NORMALISATION_NOTE: &str = "[figma-normalisation] a member matches across the boundary case-insensitively and \
     ignoring separators, so `partially_paid`, `PARTIALLY_PAID` and `Partially paid` are one \
     member. That is deliberate and it is also the limit of this limb: Figma variant values are \
     typed by a designer in prose case and TypeScript union members are typed by an engineer in \
     code case, and a comparison that failed on the difference would fail on every honest pair \
     and be switched off within a week. The cost is stated rather than hidden: this limb cannot \
     see a member that differs ONLY in case or punctuation, so `partially-paid` against \
     `partially_paid` reads as agreement. It is not the checker for that.";

pub const REDACTION_NOTE: &str = "[redaction] a finding names the prop, the record, the source file and the SHAPE of the \
     disagreement in counts, and never the type expressions themselves. A type expression can \
     carry a realisation, a captured proof record lands under the tree `no_stored_values` scans, \
     and a finding that copied one would put it there permanently and fail that gate forever on a \
     file this proof wrote (VDS S-2(2), S-2(8)). Open the named record and the named source file \
     to read the two expressions.\n\
     ONE EXCEPTION, R12: a finding about a figmaProperty NAMES the variant values on both \
     sides. A variant value is not a realisation, and that is settled where the field is \
     defined rather than asserted here (crates/vds-figma/src/ledger.rs: \"a variant value is a \
     label a designer typed, not a design value\"), which is why the ledger carrying them \
     already passes `no_stored_values`. Naming them costs nothing in safety and saves the \
     reader diffing two files by hand.";

pub const REACH_SUMMARY: &str = "[reach] this run establishes, for every row it enforced, that the contract's prop set and the \
     component's prop set are the same set, that they agree on requiredness, that their type \
     expressions agree wherever agreement was decidable, and that `states.built` covers \
     `states.required`. It establishes nothing about a row it skipped, nothing about a Figma \
     variant, and nothing about whether a built state is really implemented. A warrant citing \
     this proof must not be described more widely than that (VDS S-6(3)).";

pub fn run(ctx: &ProofContext, out: &mut dyn Write) -> Result<Outcome> {
    let project = ctx.project;
    let surface = &project.config.surface;

    // An empty `library_dirs` refuses only where there is something to refuse
    // ABOUT, and the distinction turns on whether any record claims a code
    // counterpart at all.
    //
    // With registered records and no library directory, refusing is right and the
    // reason is the one `vds register import` gives: those records assert a code
    // side, this proof cannot open any of it, and a caller told "no violations"
    // about a library nobody opened has been told nothing.
    //
    // With NO enforceable record either, there is nothing being claimed and
    // nothing being hidden. That is a vacuity, not a refusal, and VDS S-7(2)(4)
    // already has the honest word for it: the record says `vacuous`, is never
    // evidence, and exits 3. Refusing instead cost more than tidiness. VDS's own
    // `.vds/config.toml` sets `library_dirs = []` deliberately, because VDS is a
    // governance kernel with no component library, so `vds proof --all` came back
    // exit 2 here, `--allow-vacuous` does not relax a precondition, and `make
    // check` went red on a kind this repository may lawfully have nothing to say
    // about.
    let enforceable = ctx
        .store()
        .read_register()?
        .iter()
        .filter(|r| r.value.status.is_enforceable())
        .count();
    if surface.library_dirs.is_empty() && enforceable > 0 {
        return Err(VdsError::precondition(format!(
            "[surface] library_dirs is empty and {enforceable} record(s) are at `registered` or \
             later, so there is no component source to compare their contracts against and this \
             proof did not run. Name the directories your components live in, or stop claiming \
             parity: a caller told `no violations` about a library nobody opened has been told \
             nothing."
        )));
    }
    if surface.component_extensions.is_empty() {
        // A contradiction rather than a narrow surface: the configuration names
        // directories to read and then admits no file in them as a component
        // module, so every record would skip and the run would look like a
        // register nobody has built yet.
        return Err(VdsError::precondition(
            "[surface] library_dirs names directories to read and component_extensions is empty, \
             so no file in any of them is treated as a component module and this proof would \
             compare nothing while the configuration reads as though it covers everything. Name \
             the extensions.",
        ));
    }

    let mut run = ctx.new_run(ProofKind::Parity, GATE);
    run.input_file(&project.config_path)?;

    // A configured directory that is not there is a precondition failure inside
    // `scan_library`, and it propagates deliberately: a scan that reports fewer
    // components than exist is worse than one that refuses.
    let scan = scan_library(project)?;
    let library = Library::index(&scan, &surface.library_dirs);

    let index = RegisterIndex::build(&ctx.store())?;
    for record in index.records() {
        run.input_file(&record.path)?;
    }

    // The figma ledger, where one is on disk and can be relied on. Absent is a
    // NARROWING and never an error, exactly as in `reconciliation`: a project
    // with no Figma access still gets every other limb, and the note says which
    // it got. Staleness is checked HERE rather than trusted, because a limb that
    // compares against a ledger nobody regenerated reports the frame as it was,
    // not as it is, and reports it with the same confidence either way.
    let figma_ledger = match vds_figma::pull::read(&ctx.store())? {
        None => {
            run.note(FIGMA_NOTE_UNREACHED);
            None
        }
        Some(found) => match vds_figma::ledger::check_fresh(&found, None) {
            Ok(()) => {
                run.note(FIGMA_NOTE_REACHED);
                run.note(FIGMA_NORMALISATION_NOTE);
                run.input_named("<figma ledger content>", found.compute_content_digest()?);
                Some(found)
            }
            Err(why) => {
                run.note(format!(
                    "{FIGMA_NOTE_UNREACHED} A ledger IS present and was NOT relied on: {why}"
                ));
                None
            }
        },
    };

    run.note(SCOPE_NOTE);
    run.note(NORMALISATION_NOTE);
    run.note(EXPORT_LIMB_NOTE);
    run.note(PROPS_REACH_NOTE);
    run.note(STATES_REACH_NOTE);
    run.note(REDACTION_NOTE);
    run.note(REACH_SUMMARY);
    if index.is_empty() {
        run.note(
            "[register] the register holds no record, so there is no contract to compare against \
             any component and this run is vacuous. A parity proof over an empty register \
             establishes nothing about any component (VDS S-7(2)(4)).",
        );
    }

    let mut undecided: BTreeMap<&'static str, u64> = BTreeMap::new();
    // Counted separately from the type limb's. Folding them together would make
    // one note report "N comparisons were not decided" over two different
    // questions, and a reader could not tell which limb went quiet.
    let mut variant_undecided: BTreeMap<&'static str, u64> = BTreeMap::new();
    let mut without_requirement: u64 = 0;

    for located in index.records() {
        let record = &located.value;
        let location = format!(
            "{} <{} {}>",
            project.rel(&located.path),
            record.id,
            record.name
        );
        let export = match resolve(&mut run, project, record, &location, &library)? {
            Resolved::Nothing => continue,
            // The registry arm settled the export limb (and counted its own
            // enforced row). Props have no single counterpart to compare - each
            // registry entry is its own function - but R11-R13 do not need one,
            // and skipping them here would narrow the proof for exactly the
            // rows the arm exists to serve.
            Resolved::Registry => {
                compare_figma_variants(
                    &mut run,
                    &mut variant_undecided,
                    record,
                    &location,
                    figma_ledger.as_ref(),
                );
                if record.states.required.is_empty() {
                    without_requirement += 1;
                }
                compare_states(&mut run, record, &location);
                continue;
            }
            Resolved::Single(export) => export,
        };

        run.row(Verdict::Enforced);
        compare_props(&mut run, &mut undecided, record, &location, export);
        compare_figma_variants(
            &mut run,
            &mut variant_undecided,
            record,
            &location,
            figma_ledger.as_ref(),
        );
        if record.states.required.is_empty() {
            without_requirement += 1;
        }
        compare_states(&mut run, record, &location);
    }

    if !undecided.is_empty() {
        let total: u64 = undecided.values().sum();
        let by_reason: Vec<String> = undecided
            .iter()
            .map(|(reason, count)| format!("{reason} ({count})"))
            .collect();
        run.note(format!(
            "[type-limb] {total} prop type comparisons were NOT decided, by reason: {}. An \
             undecided comparison is not an agreement: for those props R8 establishes nothing, \
             and this line is where that is visible rather than assumed. The prop and the record \
             are named individually in the informational findings on this record.",
            by_reason.join(", ")
        ));
    }
    if !variant_undecided.is_empty() {
        let total: u64 = variant_undecided.values().sum();
        let by_reason: Vec<String> = variant_undecided
            .iter()
            .map(|(reason, count)| format!("{reason} ({count})"))
            .collect();
        run.note(format!(
            "[figma-limb] {total} figmaProperty comparisons were NOT decided, by reason: {}. An \
             undecided comparison is not an agreement: for those props R11 and R12 establish \
             nothing. `type_expression_is_not_a_closed_union` is the expected majority and is \
             not a defect - a prop typed `string` or `ReactNode` has no legal-value set to \
             compare, and the honest answer to \"do these two sets match\" is that one of them \
             does not exist.",
            by_reason.join(", ")
        ));
    }
    if without_requirement > 0 {
        run.note(format!(
            "[states-limb] {without_requirement} of the records compared require no state at \
             all, so R9 could not fail for them and their pass says nothing about states. This \
             proof cannot tell a component that genuinely requires no state from a contract \
             nobody filled in; that gap belongs to whoever advances a record to `registered`."
        ));
    }

    run.finish(&ctx.capture_options()?, out)
}

/// What the library scan yielded, indexed for the three lookups this proof
/// makes: an export by coordinate, every export of one file, and why a file
/// yielded none.
struct Library<'a> {
    by_coordinate: BTreeMap<(String, String), &'a LibraryExport>,
    by_file: BTreeMap<String, Vec<&'a LibraryExport>>,
    /// The scanner's own reason, kept verbatim so the two classes it collapses
    /// into one list can be told apart here rather than guessed at.
    no_export_because: BTreeMap<String, String>,
    /// The configured directories, in one spelling, worked out once rather than
    /// once per register record.
    dirs: Vec<String>,
}

impl<'a> Library<'a> {
    fn index(scan: &'a vds_scan::library::LibraryScan, library_dirs: &[String]) -> Library<'a> {
        let mut library = Library {
            by_coordinate: BTreeMap::new(),
            by_file: BTreeMap::new(),
            no_export_because: scan
                .skipped
                .iter()
                .map(|skipped| (normalise(&skipped.path), skipped.because.clone()))
                .collect(),
            dirs: library_dirs
                .iter()
                .map(|configured| {
                    normalise(configured.trim())
                        .trim_end_matches('/')
                        .to_owned()
                })
                // An empty entry would match every path rather than none, which
                // is the wrong direction for a bound.
                .filter(|directory| !directory.is_empty())
                .collect(),
        };
        for export in &scan.exports {
            let file = normalise(&export.source_file);
            library
                .by_coordinate
                .insert((file.clone(), export.export_name.clone()), export);
            library.by_file.entry(file).or_default().push(export);
        }
        library
    }
}

/// Resolve a record to the export this proof will compare it against, or
/// classify the row and return `None`.
///
/// Every path out of this function classifies the row exactly once EXCEPT the
/// success path, which leaves the row to the caller: the caller is where the
/// enforced row's two limbs run, and splitting the classification from the work
/// would let the two drift apart.
/// What a record's code coordinate resolved to.
///
/// Three outcomes rather than an Option, because "no single export" covers two
/// situations that must not share a disposition: a coordinate that resolved to
/// NOTHING (skip the row's remaining limbs - there is nothing to compare), and a
/// coordinate that resolved to a whole REGISTRY (the export limb is settled and
/// R11-R13 still run). Collapsing the second into the first would silently
/// narrow the proof for exactly the rows the registry arm exists to serve.
enum Resolved<'a> {
    Nothing,
    Single(&'a LibraryExport),
    Registry,
}

fn resolve<'a>(
    run: &mut ProofRun<'_>,
    project: &Project,
    record: &ComponentRecord,
    location: &str,
    library: &Library<'a>,
) -> Result<Resolved<'a>> {
    // Written out rather than matched with a wildcard: the lifecycle is closed by
    // VDS S-5(4), and a wildcard would silently enforce whatever an eighth status
    // turned out to mean.
    match record.status {
        Status::Proposed | Status::Designed => {
            run.row(Verdict::Skipped(SKIP_BELOW_REGISTERED));
            return Ok(Resolved::Nothing);
        }
        Status::Retired => {
            run.row(Verdict::Skipped(SKIP_RETIRED));
            return Ok(Resolved::Nothing);
        }
        Status::Registered | Status::Built | Status::Verified | Status::Deprecated => {}
    }

    let Some(code) = &record.code else {
        run.row(Verdict::Enforced);
        run.fail(Violation::fatal(
            location.to_owned(),
            RULE_NO_CODE,
            format!(
                "{} carries a code counterpart with an importPath, a repository-relative \
                 sourceFile and an exportName, so its contract has something to be compared \
                 against",
                record.id
            ),
            format!(
                "{} is at status {} and code is null. Parity is the evidence W4 rests on \
                 (VDS S-6(2)), and a registered component with no built counterpart is exactly \
                 what W4 must not be granted over",
                record.id, record.status
            ),
        ));
        return Ok(Resolved::Nothing);
    };
    let source = normalise(&code.source_file);

    if !is_repository_relative(&source) {
        // Checked BEFORE the path is joined to the root, and before anything is
        // read. `root.join("/etc/hostname")` discards the root entirely, so an
        // existence test would be satisfied by a file outside the project and a
        // check that can be satisfied from outside the repository is not a check.
        run.row(Verdict::Enforced);
        run.fail(Violation::fatal(
            location.to_owned(),
            RULE_SOURCE_NOT_RELATIVE,
            format!(
                "{}'s code.sourceFile is a path inside the repository, with no leading slash and \
                 no parent segment",
                record.id
            ),
            format!(
                "{} names a sourceFile that leaves the project root. A counterpart resolved \
                 outside the repository is not this project's code and this proof will not open \
                 it",
                record.id
            ),
        ));
        return Ok(Resolved::Nothing);
    }

    let absolute = project.root.join(&source);
    if !absolute.is_file() {
        run.row(Verdict::Enforced);
        run.fail(Violation::fatal(
            format!("{location} -> {source}"),
            RULE_SOURCE_ABSENT,
            format!(
                "{source} present in the tree, exporting {:?}",
                code.export_name
            ),
            format!(
                "{} is at status {} and names {source}, which is not a file. There is no code \
                 counterpart to compare the contract against",
                record.id, record.status
            ),
        ));
        return Ok(Resolved::Nothing);
    }
    // Digested whether or not it produced a finding, so the evidence digest
    // witnesses every source this run read. A file that appears between two runs
    // changes the findings, so it must change the recorded inputs too, or the
    // record claims two answers over one set of inputs (VDS S-7(2)(1)).
    run.input_file(&absolute)?;

    if !inside_library_dirs(&source, &library.dirs) {
        // Counted, named and reported per site. A carve-out being used as an
        // escape hatch and a carve-out working as intended produce the same
        // count, and only the first is a problem, so the site is named.
        run.row(Verdict::Skipped(SKIP_OUTSIDE_LIBRARY));
        run.inform(Violation::fatal(
            format!("{location} -> {source}"),
            RULE_OUTSIDE_LIBRARY,
            "a sourceFile inside one of the directories named by [surface] library_dirs, which \
             are the only directories this proof reads",
            format!(
                "{} names a sourceFile outside all of them, so its contract was not compared \
                 against anything. Widen library_dirs, or take the narrowing on the record",
                record.id
            ),
        ));
        return Ok(Resolved::Nothing);
    }

    if let Some(because) = library.no_export_because.get(&source) {
        // Two classes wearing one skip in the scanner's output, kept apart here.
        // A barrel or a test file is a carve-out by NAME and says nothing about
        // the code; a file the scanner read and found nothing in is a fact about
        // the code that a reader should see.
        if because.starts_with("no exported component-shaped symbol") {
            run.row(Verdict::Skipped(SKIP_NO_EXPORT_FOUND));
            run.warn(Violation::fatal(
                format!("{location} -> {source}"),
                RULE_NO_EXPORT_FOUND,
                format!(
                    "{source} exports {:?}, so the contract has a counterpart to be compared \
                     against",
                    code.export_name
                ),
                format!(
                    "the scanner read {source} and found no exported component in it at all. \
                     This is a warning and not a violation because the scanner is not a \
                     TypeScript compiler: an export style it does not recognise would otherwise \
                     fail a gate against correct code. Nothing about {} was compared",
                    record.id
                ),
            ));
        } else {
            run.row(Verdict::Skipped(SKIP_CARVED_OUT));
            run.inform(Violation::fatal(
                format!("{location} -> {source}"),
                RULE_CARVED_OUT,
                "a sourceFile that the library scan reads as a component module",
                format!(
                    "{source} is carved out of the scan, because it is {because}. Nothing about \
                     {} was compared",
                    record.id
                ),
            ));
        }
        return Ok(Resolved::Nothing);
    }

    // THE REGISTRY ARM ([2026] VJS-CC-VIBE-DESIGN-SYSTEM 2). A component SET
    // realised as a module of variant exports has no single named counterpart:
    // `blocks/nav.js` exports `nav-1` and `nav-2` and the record - one per set,
    // matching the Figma side - names `nav`. The counterpart is THE MODULE, and
    // the claim is not "the module exists" (which would weaken R4) but that the
    // registry's keys equal the record's `variant` union EXACTLY, extra and
    // missing keys each failing by name - which verifies every variant, more
    // than the named arm ever did.
    //
    // ACTIVATION IS THE RECORD'S SHAPE, NEVER THE NAMED ARM'S FAILURE: a closed
    // `variant` union plus an exportName that names the module. A fallback that
    // fires on failure absorbs typos - the silent-absorption class the lawpack
    // fallback was condemned for the same day ([2026] VJS-CC-VJS 12).
    let module_stem = std::path::Path::new(&source)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    // Three conditions, all shape and none failure: the record contracts a
    // closed variant union, the coordinate names the module, and the module IS
    // a registry (the scanner's structured marker, never its prose). The third
    // is what keeps `Button.tsx` exporting `Button` - where stem and exportName
    // collide by ordinary convention - on the named arm it always had.
    let module_is_registry = library
        .by_file
        .get(&source)
        .map(|exports| !exports.is_empty() && exports.iter().all(|export| export.registry))
        .unwrap_or(false);
    if module_is_registry
        && code.export_name == module_stem
        && let Some(mut union_members) = registry_union(record)
    {
        run.row(Verdict::Enforced);
        let mut keys: Vec<String> = library
            .by_file
            .get(&source)
            .map(|exports| {
                exports
                    .iter()
                    .map(|export| export.export_name.clone())
                    .collect()
            })
            .unwrap_or_default();
        keys.sort();
        union_members.sort();

        let missing: Vec<&String> = union_members
            .iter()
            .filter(|member| !keys.contains(member))
            .collect();
        let extra: Vec<&String> = keys
            .iter()
            .filter(|key| !union_members.contains(key))
            .collect();
        for member in &missing {
            run.fail(
                Violation::fatal(
                    format!("{location} -> {source}"),
                    RULE_REGISTRY_KEYS,
                    format!(
                        "{source} exports a registry whose keys are exactly the variant union \
                         of {}: {:?}",
                        record.id, union_members
                    ),
                    format!(
                        "the contracted variant {member:?} is not a key of the registry \
                         {source} exports ({keys:?}). A reader selecting it gets undefined, \
                         which surfaces as a blank section a long way from this record"
                    ),
                )
                .with_drift(Drift::Behind),
            );
        }
        for key in &extra {
            run.fail(
                Violation::fatal(
                    format!("{location} -> {source}"),
                    RULE_REGISTRY_KEYS,
                    format!(
                        "{source} exports a registry whose keys are exactly the variant union \
                         of {}: {:?}",
                        record.id, union_members
                    ),
                    format!(
                        "{source} exports {key:?}, which {} does not contract. An export the \
                         record does not admit is a variant nothing governs",
                        record.id
                    ),
                )
                .with_drift(Drift::Ahead),
            );
        }
        return Ok(Resolved::Registry);
    }

    let Some(export) = library
        .by_coordinate
        .get(&(source.clone(), code.export_name.clone()))
    else {
        // The file yielded exports and this is not one of them, which is the
        // case R4 is for. The finding names what the file DOES export, because
        // "no such export" is true and useless when the cause is a default
        // export written as a named one.
        run.row(Verdict::Enforced);
        let found = library.by_file.get(&source).map(|exports| {
            exports
                .iter()
                .map(|export| match &export.local_name {
                    Some(local) => format!("{} (the local name is {local})", export.export_name),
                    None => export.export_name.clone(),
                })
                .collect::<Vec<String>>()
                .join(", ")
        });
        run.fail(Violation::fatal(
            format!("{location} -> {source}"),
            RULE_EXPORT_ABSENT,
            format!(
                "{source} exports {:?}, which is what {} names in code.exportName",
                code.export_name, record.id
            ),
            match found {
                Some(names) => format!(
                    "{source} exports {names}, and not {:?}. A record pointing at an export that \
                     is not there has no counterpart, so nothing about it was compared",
                    code.export_name
                ),
                None => format!(
                    "{source} does not export {:?}. A record pointing at an export that is not \
                     there has no counterpart, so nothing about it was compared",
                    code.export_name
                ),
            },
        )
            // the record names an export the file does not have, so the CODE is behind the coordinate
            .with_drift(Drift::Behind));
        return Ok(Resolved::Nothing);
    };

    if let Some(because) = &export.props_incomplete_because {
        // The scanner has DECLARED that its prop list is not the whole set, and
        // that declaration is load-bearing in two shapes.
        //
        // The first is an empty list: no `<Name>Props` type in the file, so the
        // list is empty for want of a reader rather than because the component
        // takes nothing.
        //
        // The second was missing until it was found by review, and it is worse,
        // because the list is NON-EMPTY and looks complete: a declaration that
        // `extends` another type or intersects one. Comparing a contract against
        // a subset makes R6, the direction this module's header calls the
        // load-bearing half, fire on every inherited prop as though the code had
        // invented it, and R5 fail on nothing at all while the row is credited as
        // ENFORCED. A subset presented as a contract is worse than an absent one.
        run.row(Verdict::Skipped(SKIP_NO_PROPS_TYPE));
        if record.props.is_empty() {
            run.inform(Violation::fatal(
                format!("{location} -> {source}"),
                RULE_NO_PROPS_TYPE,
                "a `<Name>Props` interface or type alias beside the export, so that the prop set \
                 can be compared in both directions",
                format!(
                    "{because}, and {} names no prop either. Neither side claims anything, so \
                     this row establishes nothing rather than agreeing with itself",
                    record.id
                ),
            ));
        } else {
            run.warn(Violation::fatal(
                format!("{location} -> {source}"),
                RULE_NO_PROPS_TYPE,
                format!(
                    "a `<Name>Props` interface or type alias in {source}, so the {} props {} \
                     names can be compared against what the component accepts",
                    record.props.len(),
                    record.id
                ),
                format!(
                    "{because}, so the {} props {} names were not compared against a complete \
                     set. This is either drift the scanner cannot see or a prop set this build \
                     does not resolve, and both need a human",
                    record.props.len(),
                    record.id
                ),
            ));
        }
        return Ok(Resolved::Nothing);
    }

    Ok(Resolved::Single(export))
}

/// R5, R6, R7, R8 and R10, in both directions.
/// Fold a member name to the form both sides of the boundary can be compared in.
///
/// Case and separators are removed. See [`FIGMA_NORMALISATION_NOTE`] for why,
/// and for what that costs: this is the point past which the limb cannot see a
/// difference, and it is stated on every run rather than left in this comment.
fn fold_member(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// TypeScript type names that can never be a legal VALUE of a variant property.
///
/// Not exhaustive and it does not need to be: it is the tail guard behind the
/// two structural rules in [`is_legal_value_set`], and a builtin missing from
/// this list only reaches the "two or more bare words" case, which is already
/// the shorthand the contract publishes.
const NOT_A_VALUE: [&str; 14] = [
    "string",
    "number",
    "boolean",
    "bigint",
    "symbol",
    "object",
    "any",
    "unknown",
    "never",
    "void",
    "null",
    "undefined",
    "ReactNode",
    "ReactElement",
];

/// Whether a type expression's members are a set of legal VALUES, rather than a
/// type that merely happens to be one token long.
///
/// `member_names` answers a structural question: is every member of this union a
/// single token. That is necessary and not sufficient. `PropContract` publishes
/// its field as "the TypeScript type expression, OR a closed union written
/// `a|b|c`", so both of these are lawful contents and they need different
/// answers:
///
///   `'success' | 'warning'`   quoted literals, unambiguously values
///   `primary|ghost`           the documented shorthand, values
///   `string`                  the TYPE, and not a one-member enum
///   `InvoiceStatus`           an imported alias this build cannot resolve
///
/// The three rules, in order:
///
///   1. every member quoted: values, whatever the words are. `'string'` really
///      is the one-member set containing "string".
///   2. any member is a TypeScript builtin: undecided. `'success' | string` is a
///      widened union and its legal values are unbounded.
///   3. a SINGLE bare word: undecided. A one-value enum is vanishingly rare and
///      a bare identifier is overwhelmingly a type name, so guessing "value"
///      here is guessing wrong nearly every time.
///
/// Two or more bare words that are not builtins fall through to true, which is
/// the shorthand the field documents.
fn is_legal_value_set(members: &[Vec<Token>], names: &[String]) -> bool {
    let all_quoted = members
        .iter()
        .all(|m| matches!(m.as_slice(), [Token::Text(_)]));
    if all_quoted {
        return true;
    }
    if names.iter().any(|n| NOT_A_VALUE.contains(&n.as_str())) {
        return false;
    }
    names.len() > 1
}

/// The record's closed `variant` union, where it has one.
///
/// `None` for anything else - a widened union, a type alias, no variant prop at
/// all - so the registry arm cannot activate on a record that never contracted
/// a closed set of variants. The same three rules as [`is_legal_value_set`],
/// through the same tokeniser, so "closed union" means one thing in this file.
fn registry_union(record: &ComponentRecord) -> Option<Vec<String>> {
    let prop = record.props.iter().find(|prop| prop.name == "variant")?;
    let tokens = tokenise(&prop.type_expr)?;
    let members = top_level_union(&tokens);
    let names = member_names(&members)?;
    if !is_legal_value_set(&members, &names) {
        return None;
    }
    Some(names)
}

/// R11 and R12: the legal values a prop admits, against the legal values of the
/// Figma variant property it names.
///
/// # The field this makes real
///
/// `PropContract.figma_property` has existed since the first Rust build and was
/// read by nothing. Every write site set it to `None` and no proof consumed it,
/// so a field whose NAME asserts a correspondence carried no evidence that the
/// correspondence held. That is the shape a capability claim takes when there is
/// no eval behind it.
///
/// # Why this is not a network call
///
/// Both halves were already on disk and had never been introduced. The ledger
/// records `variant_properties: {name: [values]}`, which is the property AND its
/// legal options (`crates/vds-figma/src/ledger.rs`), and the contract records
/// `type_expr`, which for a closed union IS the legal values of the code prop.
/// `reconciliation` already settles that a ledger on disk may be read offline
/// under VDS S-7(2)(1); this limb relies on the same reading.
///
/// The values are derived from `type_expr` rather than stored a second time. A
/// second copy of a set that is already written down is a copy that drifts, and
/// the derive-don't-store ratio is on all fours.
///
/// # What is UNDECIDED, and why that is most of it
///
/// A prop typed `string`, `ReactNode` or an imported alias has no legal-value
/// set, so there is nothing to compare and the honest answer is that one side
/// does not exist. Counting that as agreement would be a comparison that cannot
/// go red on the majority of props, which is the defect this codebase has paid
/// for more than once.
fn compare_figma_variants(
    run: &mut ProofRun<'_>,
    undecided: &mut BTreeMap<&'static str, u64>,
    record: &ComponentRecord,
    location: &str,
    ledger: Option<&vds_figma::ledger::FigmaLedger>,
) {
    let claimed: Vec<&PropContract> = record
        .props
        .iter()
        .filter(|p| p.figma_property.is_some())
        .collect();
    if claimed.is_empty() {
        return;
    }

    let Some(ledger) = ledger else {
        *undecided.entry("no_figma_ledger_on_disk").or_insert(0) += claimed.len() as u64;
        return;
    };
    let Some(row) = ledger.nodes.iter().find(|n| n.component_id == record.id) else {
        *undecided
            .entry("record_has_no_row_in_the_figma_ledger")
            .or_insert(0) += claimed.len() as u64;
        return;
    };
    if !row.resolved {
        // The node id did not resolve in the pinned file. Distinct from "the
        // property is missing": nothing was read, so nothing is absent.
        *undecided
            .entry("node_did_not_resolve_in_the_pinned_file")
            .or_insert(0) += claimed.len() as u64;
        return;
    }
    if !row.is_component_set {
        // A component set is what carries variants. A plain frame has none, so
        // "the property is absent" would be true of every property and would say
        // nothing about this one.
        *undecided
            .entry("node_is_not_a_component_set_so_carries_no_variants")
            .or_insert(0) += claimed.len() as u64;
        return;
    }

    // The declared variant properties, folded once so a lookup does not depend on
    // the designer's capitalisation of the PROPERTY name any more than of its
    // values.
    let declared: BTreeMap<String, (&String, &Vec<String>)> = row
        .variant_properties
        .iter()
        .map(|(name, values)| (fold_member(name), (name, values)))
        .collect();

    for prop in claimed {
        let wanted = prop
            .figma_property
            .as_deref()
            .expect("filtered on is_some above");

        let Some((declared_name, values)) = declared.get(&fold_member(wanted)) else {
            run.fail(Violation::fatal(
                format!("{location} prop {}", prop.name),
                RULE_VARIANT_ABSENT,
                format!(
                    "the frame to declare a variant property named {wanted:?}, which \
                     {}'s prop {:?} says it corresponds to",
                    record.id, prop.name
                ),
                format!(
                    "the frame declares {} variant propert{} and none of them is {wanted:?}: {}. \
                     Either the property was renamed or deleted in the decided-target file and \
                     the contract still names the old one, or the contract named a property that \
                     never existed. Both read identically from the code side, which is why \
                     nothing caught this while the field went unread.",
                    row.variant_properties.len(),
                    if row.variant_properties.len() == 1 {
                        "y"
                    } else {
                        "ies"
                    },
                    if row.variant_properties.is_empty() {
                        "it declares none at all".to_owned()
                    } else {
                        row.variant_properties
                            .keys()
                            .cloned()
                            .collect::<Vec<String>>()
                            .join(", ")
                    }
                ),
            ));
            continue;
        };

        // The code side's legal values, DERIVED from the type expression with the
        // reader the type limb already uses. A second parser here would be a
        // second opinion about one expression, and two opinions drift.
        let Some(tokens) = tokenise(&prop.type_expr) else {
            *undecided.entry(UNDECIDED_UNTERMINATED).or_insert(0) += 1;
            continue;
        };
        let members = top_level_union(&tokens);
        let Some(code_members) = member_names(&members) else {
            *undecided
                .entry("type_expression_is_not_a_closed_union")
                .or_insert(0) += 1;
            continue;
        };
        // `member_names` answers "is every member a single token", which is not
        // the same question as "is this a set of legal VALUES", and the gap is
        // not academic: it read the type `string` as a one-member union whose
        // member is the literal "string", and reported the frame's real values
        // as absent from it. A confident false finding is worse than no finding,
        // because the first thing a reader does is edit the contract to match it.
        if !is_legal_value_set(&members, &code_members) {
            *undecided
                .entry("type_expression_is_not_a_closed_union")
                .or_insert(0) += 1;
            continue;
        }

        let code_folded: BTreeMap<String, &String> =
            code_members.iter().map(|m| (fold_member(m), m)).collect();
        let frame_folded: BTreeMap<String, &String> =
            values.iter().map(|v| (fold_member(v), v)).collect();

        // A member whose fold is EMPTY carries no comparable content (a value of
        // `"-"`, or `''`). Counting it as a mismatch would report a difference
        // this limb cannot actually see either way.
        if code_folded.contains_key("") || frame_folded.contains_key("") {
            *undecided
                .entry("a_member_folds_to_nothing_comparable")
                .or_insert(0) += 1;
            continue;
        }

        let only_in_code: Vec<&str> = code_folded
            .iter()
            .filter(|(k, _)| !frame_folded.contains_key(*k))
            .map(|(_, v)| v.as_str())
            .collect();
        let only_in_frame: Vec<&str> = frame_folded
            .iter()
            .filter(|(k, _)| !code_folded.contains_key(*k))
            .map(|(_, v)| v.as_str())
            .collect();

        if only_in_code.is_empty() && only_in_frame.is_empty() {
            continue;
        }

        // The members ARE named, and that is a considered departure from the
        // redaction discipline the type limb follows. A variant value is not a
        // realisation: `crates/vds-figma/src/ledger.rs` records the same
        // conclusion on the field itself ("a variant value is a label a designer
        // typed, not a design value"), which is why the ledger carrying them
        // passes `no_stored_values` today. A finding that said only "2 members
        // differ" would send the reader to open two files and diff them by hand,
        // for no gain in safety.
        let mut parts: Vec<String> = Vec::new();
        if !only_in_frame.is_empty() {
            parts.push(format!(
                "the frame offers {} the code cannot be given ({})",
                only_in_frame.len(),
                only_in_frame.join(", ")
            ));
        }
        if !only_in_code.is_empty() {
            parts.push(format!(
                "the code admits {} the frame does not draw ({})",
                only_in_code.len(),
                only_in_code.join(", ")
            ));
        }
        run.fail(Violation::fatal(
            format!("{location} prop {}", prop.name),
            RULE_VARIANT_VALUES,
            format!(
                "prop {:?} and variant property {declared_name:?} to admit ONE set of values",
                prop.name
            ),
            format!(
                "they do not: {}. A value the frame draws and the code cannot accept is a state \
                 nobody can ship; a value the code accepts and the frame does not draw is a \
                 state nobody has designed, and it will be rendered by whatever the fallback \
                 happens to be.",
                parts.join(", and ")
            ),
        ));
    }
}

fn compare_props(
    run: &mut ProofRun<'_>,
    undecided: &mut BTreeMap<&'static str, u64>,
    record: &ComponentRecord,
    location: &str,
    export: &LibraryExport,
) {
    let mut contract: BTreeMap<&str, &PropContract> = BTreeMap::new();
    for prop in &record.props {
        if contract.insert(prop.name.as_str(), prop).is_some() {
            // Two entries on one name collapse into one lookup, so without this
            // the second contract is silently never compared. The same defect the
            // register index refuses at the coordinate level (VDS S-4(4)).
            run.fail(Violation::fatal(
                format!("{location} prop {}", prop.name),
                RULE_DUPLICATE_PROP,
                format!("{} names each prop exactly once", record.id),
                format!(
                    "{} names prop {} more than once. One of them is never compared against the \
                     component, so the contract that loses is unenforced",
                    record.id, prop.name
                ),
            ));
        }
    }
    let code: BTreeMap<&str, &LibraryProp> = export
        .props
        .iter()
        .map(|prop| (prop.name.as_str(), prop))
        .collect();

    for (name, prop) in &contract {
        let Some(actual) = code.get(name) else {
            run.fail(
                Violation::fatal(
                    format!("{location} prop {name}"),
                    RULE_PROP_NOT_IN_CODE,
                    format!(
                        "{} accepts prop {name}, which the contract of {} names",
                        export.source_file, record.id
                    ),
                    format!(
                        "{} does not accept it. Either the component dropped the prop or the \
                     contract names one it never had, and a contract nothing implements binds \
                     nobody",
                        export.source_file
                    ),
                )
                // the contract names a prop the component does not accept, so the CODE is behind: an implementation is owed
                .with_drift(Drift::Behind),
            );
            continue;
        };

        if prop.required != actual.required {
            let (contract_side, code_side) = if prop.required {
                ("required", "optional")
            } else {
                ("optional", "required")
            };
            run.fail(Violation::fatal(
                format!("{location} prop {name}"),
                RULE_REQUIREDNESS,
                format!(
                    "prop {name} is {contract_side} in {} and {contract_side} in {}",
                    record.id, export.source_file
                ),
                format!(
                    "it is {contract_side} in {} and {code_side} in {}. A prop the contract makes \
                     required and the component makes optional is a promise the component does \
                     not keep, and the reverse breaks every caller the contract told not to pass \
                     it",
                    record.id, export.source_file
                ),
            )
                // both sides name the prop and disagree about requiredness
                .with_drift(Drift::Mismatch));
        }

        match compare_types(&prop.type_expr, &actual.type_expr) {
            TypeVerdict::Agrees => {}
            TypeVerdict::Undecided(reason) => {
                *undecided.entry(reason).or_insert(0) += 1;
                run.inform(Violation::fatal(
                    format!("{location} prop {name}"),
                    RULE_TYPE_DISAGREES,
                    "two type expressions this build can decide are the same, or two it can \
                     decide are different",
                    format!(
                        "neither: {reason}. The type limb establishes NOTHING about prop {name}, \
                         and an undecided comparison is not an agreement. Neither expression is \
                         repeated here; see the redaction note, and open {} and {} to compare \
                         them",
                        record.id, export.source_file
                    ),
                )
                    // both sides name the prop and disagree about its type
                    .with_drift(Drift::Mismatch));
            }
            TypeVerdict::Differs { shape } => {
                run.fail(Violation::fatal(
                    format!("{location} prop {name}"),
                    RULE_TYPE_DISAGREES,
                    format!(
                        "prop {name} carries one type expression in {} and in {}, agreeing once \
                         whitespace, string-literal quoting and top-level union ordering are \
                         normalised",
                        record.id, export.source_file
                    ),
                    format!(
                        "they disagree, and {shape}. Neither expression is repeated here, because \
                         a type expression can carry a realisation and this finding lands under \
                         the tree `no_stored_values` scans; open {} and {} to compare them",
                        record.id, export.source_file
                    ),
                ));
            }
        }
    }

    for name in code.keys() {
        if contract.contains_key(name) {
            continue;
        }
        // The direction that makes this a contract rather than a subset. There
        // is deliberately no carve-out list of "common" props here: a list that
        // exempted className, children or style would be a hole a real prop
        // walks through by being called one of those.
        run.fail(
            Violation::fatal(
                format!("{location} prop {name}"),
                RULE_PROP_NOT_CONTRACTED,
                format!(
                    "{} names prop {name}, which {} accepts",
                    record.id, export.source_file
                ),
                format!(
                    "{} accepts it and the contract does not name it. A prop nobody contracted is \
                 exactly how a component drifts, and letting it pass would make the register a \
                 subset of the component rather than a contract over it",
                    export.source_file
                ),
            )
            // the component accepts a prop no contract names, so the CODE is ahead: an amendment is owed, not a fix
            .with_drift(Drift::Ahead),
        );
    }
}

/// R9. The gap `states` records as informational and names this proof for.
fn compare_states(run: &mut ProofRun<'_>, record: &ComponentRecord, location: &str) {
    let not_built = record.required_not_built();
    if not_built.is_empty() {
        return;
    }
    run.fail(Violation::fatal(
        location.to_owned(),
        RULE_STATE_NOT_BUILT,
        format!(
            "{} builds every state it requires, so states.built contains {}",
            record.id,
            named(&in_specification_order(&record.states.required))
        ),
        format!(
            "{} is at status {} and states.built omits {}. The built counterpart does not carry a \
             state the contract requires, which is the gap VDS S-7(5) gives to this kind and \
             `states` records without failing on",
            record.id,
            record.status,
            named(&not_built)
        ),
    )
        // the contract requires a state states.built does not carry
        .with_drift(Drift::Behind));
}

/// The states, named and never counted. A count sends an author to go and look;
/// a list of names tells them what to build.
fn named(states: &[State]) -> String {
    states
        .iter()
        .map(|state| state.as_str())
        .collect::<Vec<&str>>()
        .join(", ")
}

/// A state set in the order VDS S-5(3) fixes, whatever order the record wrote it
/// in, so two records declaring the same set in different orders produce the
/// same message and the evidence digest does not move on a reordered YAML list.
fn in_specification_order(states: &[State]) -> Vec<State> {
    State::ALL
        .into_iter()
        .filter(|state| states.contains(state))
        .collect()
}

// -- the type comparison ------------------------------------------------------

/// What a comparison of two type expressions established.
///
/// A closed enum and not a boolean with a comment: "different" and "could not
/// tell" are the two answers a boolean collapses into one, and collapsing them is
/// how an undecided comparison becomes a silent pass.
enum TypeVerdict {
    Agrees,
    /// Not decidable, with a stable machine key for why. Counted, never a pass.
    Undecided(&'static str),
    /// Decidably different. Carries the shape of the difference in counts, and
    /// never the text: see the redaction note.
    Differs {
        shape: String,
    },
}

/// One token of a type expression.
///
/// Comparing token sequences rather than strings is what makes the whitespace and
/// quoting normalisations exact rather than approximate. A regex that stripped
/// whitespace would also glue `readonly string[]` into one identifier and declare
/// it equal to a type that is not the same.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Token {
    /// An identifier run: letters, digits, `_`, `$` and `.`.
    Word(String),
    /// A string literal, reduced to its CONTENTS, so `'a'` and `"a"` are one
    /// token and a re-quoted union is not drift.
    Text(String),
    /// `=>`, recognised as one token so that its `>` is not read as the close of
    /// a generic. Without this, `() => A | B` splits at a union member boundary
    /// that is not there.
    Arrow,
    Punct(char),
}

fn compare_types(contract: &str, code: &str) -> TypeVerdict {
    let (Some(left), Some(right)) = (tokenise(contract), tokenise(code)) else {
        return TypeVerdict::Undecided(UNDECIDED_UNTERMINATED);
    };
    if left == right {
        return TypeVerdict::Agrees;
    }

    let mut left_members = top_level_union(&left);
    let mut right_members = top_level_union(&right);
    left_members.sort();
    right_members.sort();
    if left_members == right_members {
        return TypeVerdict::Agrees;
    }

    // The one remaining spelling difference this build will not rule on. VDS's
    // PropContract publishes "the TypeScript type expression, OR a closed union
    // written a|b|c", so a contract may lawfully write `primary|ghost` where the
    // component writes `'primary' | 'ghost'`. Those denote the same closed set
    // under the contract's shorthand and two different types under TypeScript,
    // and deciding which the author meant is not this proof's call. Treating it
    // as agreement would also declare `string|number` equal to `'string'|'number'`,
    // which is a genuine difference.
    if let (Some(left_names), Some(right_names)) =
        (member_names(&left_members), member_names(&right_members))
        && left_names == right_names
    {
        return TypeVerdict::Undecided(UNDECIDED_QUOTING);
    }

    TypeVerdict::Differs {
        shape: difference(&left_members, &right_members),
    }
}

/// `None` where a quote opens and never closes, which is the one input this
/// tokeniser cannot make sense of and therefore refuses to guess at.
fn tokenise(expression: &str) -> Option<Vec<Token>> {
    let chars: Vec<char> = expression.chars().collect();
    let mut out = Vec::new();
    let mut at = 0;

    while at < chars.len() {
        let current = chars[at];
        if current.is_whitespace() {
            at += 1;
            continue;
        }
        if current == '\'' || current == '"' || current == '`' {
            let mut content = String::new();
            let mut closed = false;
            at += 1;
            while at < chars.len() {
                if chars[at] == '\\' && at + 1 < chars.len() {
                    content.push(chars[at + 1]);
                    at += 2;
                    continue;
                }
                if chars[at] == current {
                    at += 1;
                    closed = true;
                    break;
                }
                content.push(chars[at]);
                at += 1;
            }
            if !closed {
                return None;
            }
            out.push(Token::Text(content));
            continue;
        }
        if is_word_char(current) {
            let mut word = String::new();
            // An INTERIOR hyphen followed by a word character stays in the word,
            // so the contract shorthand `nav-1|nav-2` reads as two members and
            // not six tokens. PropContract documents "a closed union written
            // a|b|c" and a kebab-cased variant key is a lawful member of it -
            // measured on every generated project, where the bridge writes the
            // registry keys this way. A leading hyphen, or one followed by
            // anything else, is still punctuation: `A - B` and `-1` tokenise as
            // they always did.
            while at < chars.len()
                && (is_word_char(chars[at])
                    || (chars[at] == '-'
                        && !word.is_empty()
                        && at + 1 < chars.len()
                        && is_word_char(chars[at + 1])))
            {
                word.push(chars[at]);
                at += 1;
            }
            out.push(Token::Word(word));
            continue;
        }
        if current == '=' && chars.get(at + 1) == Some(&'>') {
            out.push(Token::Arrow);
            at += 2;
            continue;
        }
        out.push(Token::Punct(current));
        at += 1;
    }
    Some(out)
}

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '$' || c == '.'
}

/// The members of a union written at the OUTERMOST level, in source order.
///
/// Only the outermost level is reordered later, so a union nested inside a
/// generic keeps its order and a difference there is still a difference. That is
/// the floor, and it is named in the normalisation note rather than left for a
/// reader to discover.
fn top_level_union(tokens: &[Token]) -> Vec<Vec<Token>> {
    let mut members: Vec<Vec<Token>> = vec![Vec::new()];
    let mut depth = 0i32;

    for token in tokens {
        match token {
            Token::Punct('(' | '[' | '{' | '<') => depth += 1,
            Token::Punct(')' | ']' | '}' | '>') => depth -= 1,
            // `depth <= 0` and not `== 0`, so a stray closing bracket cannot push
            // the counter negative and hide every later union boundary.
            Token::Punct('|') if depth <= 0 => {
                members.push(Vec::new());
                continue;
            }
            _ => {}
        }
        members
            .last_mut()
            .expect("there is always a member under construction")
            .push(token.clone());
    }
    // Drops the empty member a leading `|` produces, which TypeScript permits and
    // which is not a member of anything.
    members.retain(|member| !member.is_empty());
    members
}

/// The name of every member, where every member is a single word or string
/// literal. `None` as soon as one is not, because the quoting question only
/// arises between two lists of bare names.
fn member_names(members: &[Vec<Token>]) -> Option<Vec<String>> {
    let mut names: Vec<String> = Vec::new();
    for member in members {
        match member.as_slice() {
            [Token::Word(name)] | [Token::Text(name)] => names.push(name.clone()),
            _ => return None,
        }
    }
    names.sort();
    Some(names)
}

/// How two type expressions differ, in counts and structure only.
///
/// The alternative, printing the two expressions, would write a realisation into
/// a record under the tree `no_stored_values` scans the moment one appears in a
/// union. See the redaction note. Where the two have the SAME structure the
/// sentence says so, because "the record holds one type expression of one token
/// and the component holds one type expression of one token" reads like a bug in
/// the gate rather than a finding about the code.
fn difference(contract: &[Vec<Token>], code: &[Vec<Token>]) -> String {
    let left = shape(contract);
    let right = shape(code);
    if left == right {
        format!("both sides are {left} and the tokens themselves differ")
    } else {
        format!("the record holds {left} and the component holds {right}")
    }
}

fn shape(members: &[Vec<Token>]) -> String {
    if members.is_empty() {
        return "no type expression at all".to_owned();
    }
    let tokens: usize = members.iter().map(Vec::len).sum();
    let counted = if tokens == 1 {
        "one token".to_owned()
    } else {
        format!("{tokens} tokens")
    };
    if members.len() == 1 {
        format!("one type expression of {counted}")
    } else {
        format!("a union of {} members over {counted}", members.len())
    }
}

// -- paths --------------------------------------------------------------------

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

/// Whether a source path lies under one of the directories the scan read.
///
/// Compared on a segment boundary, so `src/components/uix/a.tsx` is not inside
/// `src/components/ui`. A plain `starts_with` on the bare directory name would
/// pull a neighbouring directory into a bound that does not cover it.
fn inside_library_dirs(source: &str, dirs: &[String]) -> bool {
    dirs.iter()
        .any(|directory| source.starts_with(&format!("{directory}/")))
}

#[cfg(test)]
mod tests {
    use crate::testing::{Harness, run_kind};
    use vds_core::{
        CodeCounterpart, ComponentId, EXIT_PASSED, EXIT_VACUOUS, EXIT_VIOLATION, ProofKind,
        ProofStatus, PropContract, Severity, State, StateContract, Status, default_config,
    };

    /// A component file carrying a `<Name>Props` type, which is the one shape
    /// the scanner reads a prop list from. The harness's own `component_file`
    /// deliberately writes a component with no props type, which is a different
    /// case and has its own test below.
    fn component_with_props(h: &Harness, name: &str, members: &str) {
        h.write(
            &format!("src/components/ui/{}.tsx", name.to_lowercase()),
            &format!(
                "interface {name}Props {{\n{members}\n}}\n\
                 export function {name}(p: {name}Props) {{ return <div />; }}\n"
            ),
        );
    }

    /// A CommonJS variant-registry block, the site-factory shape, at a path the
    /// default scan reads (`.jsx` is in the default extensions; the registry
    /// reader does not care which extension carried it).
    fn registry_block(h: &Harness, stem: &str, keys: &[&str]) {
        let body: String = keys.iter().map(|k| format!("  '{k}': render,\n")).collect();
        h.write(
            &format!("src/components/ui/{stem}.jsx"),
            &format!(
                "'use strict';\nfunction render(content) {{ return '<div></div>'; }}\n\
                 module.exports = {{\n{body}}};\n"
            ),
        );
    }

    /// One record per SET, the coordinate naming the module, the `variant` prop
    /// carrying the closed union - the exact shape the bridge writes.
    fn registry_record(h: &Harness, name: &str, stem: &str, union: &str) -> ComponentId {
        let id = h.register_as("CMP-0001", name, stem, Status::Registered);
        h.amend(&id, |record| {
            record.code.as_mut().unwrap().source_file = format!("src/components/ui/{stem}.jsx");
        });
        set_props(h, &id, &[("variant", union, false)]);
        id
    }

    fn set_props(h: &Harness, id: &ComponentId, props: &[(&str, &str, bool)]) {
        h.amend(id, |record| {
            record.props = props
                .iter()
                .map(|(name, type_expr, required)| PropContract {
                    name: (*name).to_owned(),
                    type_expr: (*type_expr).to_owned(),
                    required: *required,
                    figma_property: None,
                })
                .collect();
        });
    }

    /// A registered Button whose contract and whose source say the same thing.
    /// Every test that is about one disagreement starts here and introduces
    /// exactly that one.
    fn agreeing(h: &Harness, status: Status) -> ComponentId {
        let id = h.register("Button", status);
        component_with_props(
            h,
            "Button",
            "  variant?: 'primary' | 'ghost';\n  onClick: () => void;",
        );
        set_props(
            h,
            &id,
            &[
                ("variant", "'primary' | 'ghost'", false),
                ("onClick", "() => void", true),
            ],
        );
        id
    }

    #[test]
    fn a_contract_that_matches_its_component_passes() {
        let h = Harness::new();
        agreeing(&h, Status::Registered);
        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
        assert_eq!(outcome.exit_code, EXIT_PASSED);
        assert_eq!(outcome.rows_enforced, 1, "one row per registered component");
    }

    /// The failing-direction test VDS S-7(2)(2) requires, and the one the
    /// enforcement lock names. It seeds a component that accepts a prop no
    /// contract names, which is the direction that makes the register a contract
    /// rather than a subset of the component, and asserts the non-zero exit.
    #[test]
    fn parity_fails_on_a_prop_the_component_accepts_that_no_contract_names() {
        let h = Harness::new();
        let id = h.register("Button", Status::Registered);
        component_with_props(
            &h,
            "Button",
            "  variant?: 'primary' | 'ghost';\n  tone?: 'warn';",
        );
        set_props(&h, &id, &[("variant", "'primary' | 'ghost'", false)]);

        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert_eq!(outcome.status, ProofStatus::Failed);
        assert!(text.contains("prop tone"), "{text}");
        assert!(text.contains("exactly how a component drifts"), "{text}");
    }

    #[test]
    fn parity_fails_on_a_contracted_prop_the_component_does_not_accept() {
        let h = Harness::new();
        let id = agreeing(&h, Status::Registered);
        set_props(
            &h,
            &id,
            &[
                ("variant", "'primary' | 'ghost'", false),
                ("onClick", "() => void", true),
                ("size", "'sm' | 'md'", false),
            ],
        );
        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("prop size"), "{text}");
        assert!(text.contains("binds nobody"), "{text}");
    }

    #[test]
    fn a_prop_the_contract_makes_required_and_the_component_makes_optional_fails() {
        let h = Harness::new();
        let id = agreeing(&h, Status::Registered);
        set_props(
            &h,
            &id,
            &[
                ("variant", "'primary' | 'ghost'", true),
                ("onClick", "() => void", true),
            ],
        );
        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("prop variant"), "{text}");
        assert!(
            text.contains("a promise the component does not keep"),
            "{text}"
        );
    }

    /// R8, and the redaction rule that goes with it. A type expression can carry
    /// a realisation, so the finding describes the disagreement in counts and
    /// repeats neither expression (VDS S-2(2)).
    #[test]
    fn two_type_expressions_that_differ_are_a_violation_that_repeats_neither_of_them() {
        let h = Harness::new();
        let id = h.register("Button", Status::Registered);
        component_with_props(&h, "Button", "  tone: Alpha;");
        set_props(&h, &id, &[("tone", "Beta", true)]);

        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("prop tone"), "{text}");

        let record = h.last_proof(ProofKind::Parity);
        let finding = record
            .violations
            .iter()
            .find(|violation| violation.rule.contains("R8"))
            .unwrap_or_else(|| panic!("no R8 finding in {:?}", record.violations));
        for expression in ["Alpha", "Beta"] {
            assert!(
                !finding.actual.contains(expression) && !finding.expected.contains(expression),
                "a finding that copied a type expression would write a realisation under the \
                 tree `no_stored_values` scans the first time one appeared in a union: \
                 {finding:?}"
            );
        }
        assert!(
            finding
                .actual
                .contains("both sides are one type expression of one token"),
            "a finding that reads like a bug in the gate teaches a reader to skip the section: \
             {finding:?}"
        );
    }

    /// The normalisations the comparison makes, and the reason it makes them:
    /// exact string equality reports a reordered union and a re-wrapped line as
    /// drift, and a gate that cries wolf gets disabled.
    #[test]
    fn whitespace_and_top_level_union_ordering_are_not_reported_as_drift() {
        let h = Harness::new();
        let id = h.register("Button", Status::Registered);
        component_with_props(&h, "Button", "  variant?:   'primary'  |  'ghost'  ;");
        set_props(&h, &id, &[("variant", "'ghost'|'primary'", false)]);

        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(outcome.exit_code, EXIT_PASSED, "{text}");
        assert_eq!(outcome.rows_enforced, 1);
    }

    /// [2026] VJS-CC-VIBE-DESIGN-SYSTEM 2: the code counterpart of a component
    /// SET realised as a variant registry is the MODULE, and the registry arm
    /// proves the keys equal the record's variant union exactly. This is the
    /// ruling that turned 13-of-13 red rows on every generated project into a
    /// pass earned by rule change, not by data change.
    #[test]
    fn a_variant_registry_record_resolves_to_the_module_and_passes() {
        let h = Harness::new();
        registry_block(&h, "nav", &["nav-1", "nav-2"]);
        registry_record(&h, "Nav", "nav", "'nav-1' | 'nav-2'");

        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(outcome.exit_code, EXIT_PASSED, "{text}");
        assert_eq!(outcome.rows_enforced, 1, "{text}");
    }

    /// Failing direction one (order D4): a contracted variant the registry does
    /// not export, refused by name with the code marked BEHIND the record.
    #[test]
    fn a_contracted_variant_missing_from_the_registry_fails_by_name() {
        let h = Harness::new();
        registry_block(&h, "nav", &["nav-1"]);
        registry_record(&h, "Nav", "nav", "'nav-1' | 'nav-2'");

        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("registry arm"), "{text}");
        assert!(
            text.contains("\"nav-2\" is not a key"),
            "the missing variant must be named, not counted: {text}"
        );
    }

    /// Failing direction two (order D4): an export the record does not
    /// contract - a variant nothing governs - with the code marked AHEAD.
    #[test]
    fn a_registry_key_the_record_does_not_contract_fails_by_name() {
        let h = Harness::new();
        registry_block(&h, "nav", &["nav-1", "nav-2", "nav-3"]);
        registry_record(&h, "Nav", "nav", "'nav-1' | 'nav-2'");

        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(
            text.contains("\"nav-3\", which CMP-0001 does not contract"),
            "{text}"
        );
    }

    /// The no-absorption control the order makes explicit: the registry arm
    /// activates on the RECORD'S SHAPE (a closed variant union), never on the
    /// named arm's failure. A typo'd exportName that happens to equal the
    /// module stem, on a record with no variant union, must still fail the
    /// NAMED arm - a fallback that fires on failure absorbs typos.
    #[test]
    fn a_typoed_export_name_without_a_variant_union_is_not_absorbed() {
        let h = Harness::new();
        let id = h.register_as("CMP-0001", "Button", "button", Status::Registered);
        component_with_props(&h, "Button", "  label: string;");
        set_props(&h, &id, &[("label", "string", true)]);

        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(
            text.contains("exports Button"),
            "the named arm must answer, naming what the file DOES export: {text}"
        );
        assert!(
            !text.contains("registry arm"),
            "the registry arm must not activate without a closed variant union: {text}"
        );
    }

    /// The one spelling difference this build will not rule on. It is counted
    /// and named, never passed silently, and never reported as drift either.
    #[test]
    fn a_union_one_side_quotes_and_the_other_does_not_is_undecided_counted_and_named() {
        let h = Harness::new();
        let id = h.register("Button", Status::Registered);
        component_with_props(&h, "Button", "  variant?: 'primary' | 'ghost';");
        set_props(&h, &id, &[("variant", "primary|ghost", false)]);

        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(
            outcome.exit_code, EXIT_PASSED,
            "an undecided comparison is not a violation: {text}"
        );
        assert!(text.contains("[type-limb]"), "{text}");
        assert!(text.contains(super::UNDECIDED_QUOTING), "{text}");

        let record = h.last_proof(ProofKind::Parity);
        let finding = record
            .violations
            .iter()
            .find(|violation| violation.rule.contains("R8"))
            .unwrap_or_else(|| panic!("no R8 finding in {:?}", record.violations));
        assert_eq!(
            finding.severity,
            Severity::Informational,
            "not deciding is not a violation, and it is not an agreement either"
        );
        assert!(
            finding
                .actual
                .contains("establishes NOTHING about prop variant"),
            "{finding:?}"
        );
    }

    /// The other undecided class. A quote that opens and never closes is the one
    /// input the tokeniser cannot make sense of, and guessing at it would decide
    /// a comparison on a reading nobody wrote.
    #[test]
    fn an_unterminated_string_literal_in_a_type_is_undecided_and_not_guessed_at() {
        let h = Harness::new();
        let id = h.register("Button", Status::Registered);
        component_with_props(&h, "Button", "  variant?: 'primary';");
        set_props(&h, &id, &[("variant", "'primary", false)]);

        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(outcome.exit_code, EXIT_PASSED, "{text}");
        assert!(text.contains(super::UNDECIDED_UNTERMINATED), "{text}");
    }

    /// Stricter than `reconciliation` on purpose: parity is the evidence W4
    /// rests on, and a registered component with no built counterpart is exactly
    /// what W4 must not be granted over (VDS S-6(2)).
    #[test]
    fn a_registered_record_with_no_code_counterpart_is_a_violation_and_not_a_skip() {
        let h = Harness::new();
        h.register_unbuilt("Sketch", Status::Registered);
        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert_eq!(outcome.rows_enforced, 1);
        assert!(text.contains("code is null"), "{text}");
        assert!(text.contains("W4 must not be granted over"), "{text}");
    }

    #[test]
    fn a_record_naming_a_source_file_that_is_not_there_is_a_violation() {
        let h = Harness::new();
        h.register("Button", Status::Registered);
        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("src/components/ui/button.tsx"), "{text}");
        assert!(text.contains("which is not a file"), "{text}");
    }

    /// R4, and the near-miss detail that goes with it. "No such export" is true
    /// and useless when the cause is a default export written as a named one.
    #[test]
    fn a_record_naming_an_export_the_file_does_not_export_is_a_violation() {
        let h = Harness::new();
        h.register("Button", Status::Registered);
        h.write(
            "src/components/ui/button.tsx",
            "export function Card() { return <div />; }\n",
        );
        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(
            text.contains("exports Card, and not \"Button\""),
            "a finding that names what the file DOES export saves the reader a search: {text}"
        );
    }

    #[test]
    fn a_source_file_that_leaves_the_repository_is_refused_before_it_is_opened() {
        let h = Harness::new();
        let id = h.register("Button", Status::Registered);
        h.amend(&id, |record| {
            record.code.as_mut().unwrap().source_file = "/etc/hostname".into();
        });
        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(
            outcome.exit_code, EXIT_VIOLATION,
            "a comparison satisfied by a file outside the repository is not a comparison: {text}"
        );
        assert!(text.contains("leaves the project root"), "{text}");
    }

    /// VDS S-5(4): a `proposed` or `designed` record is a candidate and there is
    /// no contract to hold the code to. This one would fail R5 if it were
    /// enforced, which is what makes the carve-out real rather than decorative.
    #[test]
    fn a_record_below_registered_is_counted_and_never_enforced() {
        let h = Harness::new();
        let id = agreeing(&h, Status::Designed);
        set_props(&h, &id, &[("nothingLikeThis", "never", true)]);

        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(outcome.status, ProofStatus::Vacuous, "{text}");
        assert_eq!(outcome.rows_considered, 1);
        assert_eq!(outcome.rows_enforced, 0);
        assert!(text.contains(super::SKIP_BELOW_REGISTERED), "{text}");
    }

    /// VDS S-9(8): a retired record's ABSENCE from the code is the correct
    /// state, so comparing its contract against a file that ought to be gone
    /// would invert the rule. Its presence is `reconciliation`'s to fail on.
    #[test]
    fn a_retired_tombstone_is_counted_and_never_enforced() {
        let h = Harness::new();
        agreeing(&h, Status::Retired);
        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(outcome.status, ProofStatus::Vacuous, "{text}");
        assert_eq!(outcome.rows_enforced, 0);
        assert!(text.contains(super::SKIP_RETIRED), "{text}");
    }

    /// A deprecated component is still shipped until it drains (VDS S-9(6)(2)),
    /// so its contract still binds the code that is still there.
    #[test]
    fn a_deprecated_record_is_still_compared_because_it_is_still_shipped() {
        let h = Harness::new();
        let id = agreeing(&h, Status::Deprecated);
        set_props(&h, &id, &[("variant", "'primary' | 'ghost'", false)]);

        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert_eq!(outcome.rows_enforced, 1);
        assert!(text.contains("prop onClick"), "{text}");
    }

    /// R9. VDS S-7(5) gives this gap to parity by name, and `states` records it
    /// as informational precisely because this is the gate for it.
    #[test]
    fn a_required_state_that_states_built_does_not_carry_is_a_violation() {
        let h = Harness::new();
        let id = agreeing(&h, Status::Registered);
        h.amend(&id, |record| {
            record.states = StateContract {
                required: vec![State::Default, State::Focus],
                drawn: vec![State::Default, State::Focus],
                built: vec![State::Default],
            };
        });
        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("states.built omits focus"), "{text}");
    }

    /// The scanner's prop list is empty for want of a reader, not because the
    /// component takes nothing, so the row is counted, skipped and warned about
    /// rather than failed on every prop the contract names.
    #[test]
    fn a_source_declaring_no_props_type_is_counted_skipped_and_warned_about() {
        let h = Harness::new();
        let id = h.register("Button", Status::Registered);
        h.component_file("Button");
        set_props(&h, &id, &[("variant", "'primary' | 'ghost'", false)]);

        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(
            outcome.status,
            ProofStatus::Vacuous,
            "nothing was compared, so the run proves nothing and says so: {text}"
        );
        assert_eq!(outcome.exit_code, EXIT_VACUOUS);
        assert!(text.contains(super::SKIP_NO_PROPS_TYPE), "{text}");
        assert!(text.contains("WARNINGS"), "{text}");

        let record = h.last_proof(ProofKind::Parity);
        assert_eq!(record.violations[0].severity, Severity::Warning);
    }

    #[test]
    fn a_source_outside_the_scanned_library_dirs_is_counted_and_not_enforced() {
        let h = Harness::new();
        let id = agreeing(&h, Status::Registered);
        h.write(
            "src/legacy/button.tsx",
            "export function Button() { return <div />; }\n",
        );
        h.amend(&id, |record| {
            record.code.as_mut().unwrap().source_file = "src/legacy/button.tsx".into();
        });

        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(outcome.status, ProofStatus::Vacuous, "{text}");
        assert!(text.contains(super::SKIP_OUTSIDE_LIBRARY), "{text}");

        let record = h.last_proof(ProofKind::Parity);
        assert_eq!(record.violations[0].severity, Severity::Informational);
        assert!(
            record.violations[0].actual.contains("was not compared"),
            "{:?}",
            record.violations[0]
        );
    }

    #[test]
    fn a_barrel_named_as_a_source_file_is_counted_and_not_enforced() {
        let h = Harness::new();
        let id = agreeing(&h, Status::Registered);
        h.write(
            "src/components/ui/index.tsx",
            "export { Button } from \"./button\";\n",
        );
        h.amend(&id, |record| {
            record.code.as_mut().unwrap().source_file = "src/components/ui/index.tsx".into();
        });

        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(outcome.status, ProofStatus::Vacuous, "{text}");
        assert!(text.contains(super::SKIP_CARVED_OUT), "{text}");
    }

    /// A file the scanner read and found nothing in is a fact about the code a
    /// reader should see, and it is NOT a fatal finding: the scanner is not a
    /// TypeScript compiler, and an export style it does not recognise would
    /// otherwise fail a gate against correct code.
    #[test]
    fn a_source_that_yields_no_export_at_all_is_a_warning_and_not_a_violation() {
        let h = Harness::new();
        h.register("Button", Status::Registered);
        h.write(
            "src/components/ui/button.tsx",
            "const Button = () => <div />;\nexport * from \"./other\";\n",
        );

        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(outcome.status, ProofStatus::Vacuous, "{text}");
        assert!(text.contains(super::SKIP_NO_EXPORT_FOUND), "{text}");
        assert!(text.contains("not a TypeScript compiler"), "{text}");
    }

    #[test]
    fn a_contract_naming_one_prop_twice_is_a_violation() {
        let h = Harness::new();
        let id = agreeing(&h, Status::Registered);
        set_props(
            &h,
            &id,
            &[
                ("variant", "'primary' | 'ghost'", false),
                ("variant", "'primary' | 'ghost'", false),
                ("onClick", "() => void", true),
            ],
        );
        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("more than once"), "{text}");
    }

    /// A comparison with no code side is not a narrow run, it is no run at all,
    /// and a caller told "no violations" about a library nobody opened has been
    /// told nothing. That holds where a record CLAIMS a code side.
    /// The other half of that rule, and the half a repository like this one lives
    /// in: no library directory AND no record claiming a code side is a VACUITY,
    /// not a refusal. Nothing is being asserted, so nothing is being hidden, and
    /// S-7(2)(4) already has the honest word for a run over zero enforceable
    /// rows.
    #[test]
    fn an_empty_library_dirs_with_nothing_registered_is_vacuous_and_not_a_refusal() {
        let h = Harness::with_config(&default_config("demo", "DEMO").replace(
            r#"library_dirs = ["src/components/ui"]"#,
            "library_dirs = []",
        ));

        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(outcome.status, ProofStatus::Vacuous, "{text}");
        assert_eq!(outcome.exit_code, EXIT_VACUOUS, "{text}");
        assert_eq!(outcome.rows_enforced, 0);
    }

    #[test]
    fn an_empty_library_dirs_is_a_precondition_failure_and_not_a_pass() {
        let h = Harness::with_config(&default_config("demo", "DEMO").replace(
            r#"library_dirs = ["src/components/ui"]"#,
            "library_dirs = []",
        ));
        h.register("Button", Status::Registered);
        let error = h.run_kind_err(ProofKind::Parity);
        assert!(
            error.to_string().contains("library_dirs is empty"),
            "{error}"
        );
        assert!(
            error.to_string().contains("has been told nothing"),
            "{error}"
        );
    }

    #[test]
    fn library_dirs_with_no_component_extensions_is_a_precondition_failure() {
        let h = Harness::with_config(&default_config("demo", "DEMO").replace(
            r#"component_extensions = ["tsx", "jsx"]"#,
            "component_extensions = []",
        ));
        let error = h.run_kind_err(ProofKind::Parity);
        assert!(
            error.to_string().contains("component_extensions is empty"),
            "{error}"
        );
    }

    #[test]
    fn a_library_directory_that_does_not_exist_is_a_precondition_failure() {
        let h = Harness::with_config(&default_config("demo", "DEMO").replace(
            r#"library_dirs = ["src/components/ui"]"#,
            r#"library_dirs = ["src/components/absent"]"#,
        ));
        let error = h.run_kind_err(ProofKind::Parity);
        assert!(
            error.to_string().contains("src/components/absent"),
            "{error}"
        );
    }

    #[test]
    fn an_empty_register_is_vacuous_and_never_passed() {
        let h = Harness::new();
        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(outcome.status, ProofStatus::Vacuous, "{text}");
        assert_eq!(outcome.exit_code, EXIT_VACUOUS);
        assert!(!text.contains("PASS:"), "no PASS beside a vacuity: {text}");
        assert!(text.contains("[register]"), "{text}");
    }

    /// The redaction rule, tested against the proof that would catch a breach of
    /// it rather than against my own reading of my own strings.
    ///
    /// A parity record lands in `.vds/proofs/`, which is inside the tree
    /// `no_stored_values` scans. If a finding or a note of this proof carried a
    /// realisation, that gate would fail from then on, on a file this proof
    /// wrote, with no lawful way back because a record is never deleted
    /// (VDS S-2(2), S-2(8)). So the failing run below is captured and the other
    /// gate is pointed at it.
    #[test]
    fn a_captured_parity_record_does_not_itself_become_a_stored_design_value() {
        let h = Harness::new();
        let id = agreeing(&h, Status::Registered);
        // A record that trips as many rules as possible at once, so as much of
        // this proof's vocabulary as possible lands in the captured record.
        component_with_props(
            &h,
            "Button",
            "  variant?: 'primary' | 'ghost';\n  padding: '4px';\n  fade: '200ms';",
        );
        h.amend(&id, |record| {
            record.states = StateContract {
                required: vec![State::Default, State::Focus],
                drawn: vec![State::Default, State::Focus],
                built: vec![State::Default],
            };
        });
        set_props(
            &h,
            &id,
            &[("variant", "Alpha", true), ("onClick", "() => void", true)],
        );

        let (parity, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(parity.exit_code, EXIT_VIOLATION, "{text}");

        let (stored, stored_text) = run_kind(&h, ProofKind::NoStoredValues);
        assert_ne!(
            stored.exit_code, EXIT_VIOLATION,
            "this parity run wrote a realisation into the record it captured, and that gate now \
             fails forever on a file this proof wrote: {stored_text}"
        );
    }

    #[test]
    fn every_record_is_one_row_and_the_counts_add_up() {
        let h = Harness::new();
        agreeing(&h, Status::Registered);
        h.register_as("CMP-0002", "Sketch", "Sketch", Status::Proposed);
        h.register_as("CMP-0003", "OldChip", "OldChip", Status::Retired);

        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(outcome.rows_considered, 3, "{text}");
        assert_eq!(outcome.rows_enforced, 1);

        let record = h.last_proof(ProofKind::Parity);
        let skipped: u64 = record.rows_skipped_reasons.values().sum();
        assert_eq!(
            record.rows_considered,
            record.rows_enforced + skipped,
            "a row classified twice makes the vacuity check unable to see its own arithmetic"
        );
    }

    /// Every narrowing this build makes is written onto the record, because a
    /// reader of a passing parity proof must not take it for more than it is
    /// (VDS S-6(3)).
    #[test]
    fn the_run_records_what_it_cannot_reach() {
        let h = Harness::new();
        agreeing(&h, Status::Registered);
        run_kind(&h, ProofKind::Parity);
        let record = h.last_proof(ProofKind::Parity);

        for marker in [
            "[scope]",
            "[normalisation]",
            "[export-limb]",
            "[props-reach]",
            "[states-reach]",
            "[figma]",
            "[redaction]",
            "[reach]",
        ] {
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
                .any(|note| note.contains("never that the claim is true")),
            "the states limb reads the record's own built claim, and the record is where that \
             has to be said: {:?}",
            record.notes
        );
        assert!(
            record
                .notes
                .iter()
                .any(|note| note.contains("does not follow an `extends`")),
            "{:?}",
            record.notes
        );
    }

    /// VDS S-7(2)(1). This proof reads component source, so a change to that
    /// source must move the recorded inputs, or the record claims two answers
    /// over one set of inputs.
    #[test]
    fn a_change_to_the_component_source_moves_the_recorded_inputs() {
        let h = Harness::new();
        agreeing(&h, Status::Registered);
        run_kind(&h, ProofKind::Parity);
        let before = h.last_proof(ProofKind::Parity);

        component_with_props(
            &h,
            "Button",
            "  variant?: 'primary' | 'ghost';\n  onClick: () => void;\n  tone?: 'warn';",
        );
        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        let after = h.last_proof(ProofKind::Parity);

        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert_ne!(
            before.inputs_digest, after.inputs_digest,
            "the finding set moved, so the recorded inputs must move with it"
        );
    }

    /// The critical direction R6 was blind to.
    ///
    /// A component whose props declaration `extends` another type accepts every
    /// inherited member, and the shallow reader sees only the inline block. It
    /// used to report that subset as a complete prop list, so R6 fired on every
    /// inherited prop as though the code had invented it, or credited the row as
    /// ENFORCED over a comparison that could never have been complete.
    #[test]
    fn a_props_type_that_extends_another_is_skipped_and_never_enforced() {
        for head in [
            "interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement>",
            "type ButtonProps = BaseProps &",
            "type ButtonProps = Omit<BaseProps, 'size'> &",
        ] {
            let h = Harness::new();
            h.write(
                "src/components/ui/Button.tsx",
                &format!(
                    "{head} {{\n  variant?: 'primary' | 'ghost';\n}}\n\
                     export function Button(p: ButtonProps) {{ return <button />; }}\n"
                ),
            );
            let id = h.register("Button", Status::Registered);
            h.amend(&id, |record| {
                record.props = vec![PropContract {
                    name: "variant".into(),
                    type_expr: "'primary' | 'ghost'".into(),
                    required: false,
                    figma_property: None,
                }];
                record.code = Some(CodeCounterpart {
                    import_path: "@/components/ui/Button".into(),
                    source_file: "src/components/ui/Button.tsx".into(),
                    export_name: "Button".into(),
                });
            });

            let (outcome, text) = run_kind(&h, ProofKind::Parity);
            assert_eq!(
                outcome.rows_enforced, 0,
                "a row whose prop set is a SUBSET was credited as enforced, so the comparison \
                 that could never have been complete counts towards the vacuity check: {text}"
            );
            assert!(
                text.contains(super::SKIP_NO_PROPS_TYPE),
                "the skip must be named and counted: {text}"
            );
            assert!(
                text.contains("SUBSET"),
                "the reason has to say what was missed, not merely that something was: {text}"
            );
        }
    }

    /// And the ordinary case still enforces, so the fix did not turn every
    /// component into a skip.
    #[test]
    fn an_inline_props_type_is_still_compared_rather_than_skipped() {
        let h = Harness::new();
        h.write(
            "src/components/ui/Button.tsx",
            "type ButtonProps = {\n  variant?: 'primary' | 'ghost';\n};\n\
             export function Button(p: ButtonProps) { return <button />; }\n",
        );
        let id = h.register("Button", Status::Registered);
        h.amend(&id, |record| {
            record.props = vec![PropContract {
                name: "variant".into(),
                type_expr: "'primary' | 'ghost'".into(),
                required: false,
                figma_property: None,
            }];
            record.code = Some(CodeCounterpart {
                import_path: "@/components/ui/Button".into(),
                source_file: "src/components/ui/Button.tsx".into(),
                export_name: "Button".into(),
            });
        });

        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(outcome.rows_enforced, 1, "{text}");
    }
}

/// R11 and R12: the legal values on each side of the design/code boundary.
///
/// The limb exists because `PropContract.figma_property` had been in the type
/// since the first Rust build and was read by NOTHING: every write site set it to
/// `None`, no proof consumed it, and a field whose name asserts a correspondence
/// carried no evidence that the correspondence held.
#[cfg(test)]
mod figma_variant_tests {
    use crate::testing::{Harness, run_kind};
    use vds_core::{ComponentId, EXIT_PASSED, EXIT_VIOLATION, ProofKind, ProofStatus, Status};

    /// A Button whose contract and source agree, carrying ONE prop with a
    /// `figmaProperty`, and a figma ledger declaring that variant property with
    /// the given legal values.
    ///
    /// Both sides are spelled out at the call site. A fixture that supplied a
    /// plausible value set would decide the very question the limb asks.
    fn harness_with_intent(code_union: &[&str], frame_values: &[&str]) -> Harness {
        let h = Harness::new();
        let id = h.register("Button", Status::Registered);
        h.write(
            "src/components/ui/button.tsx",
            &format!(
                "interface ButtonProps {{\n  intent: {};\n}}\n\
                 export function Button(p: ButtonProps) {{ return <div />; }}\n",
                code_union[0]
            ),
        );
        h.prop_with_variant(&id, "intent", code_union[0], "Intent");
        h.figma_variants(&[(&id, &[("Intent", frame_values)])]);
        h
    }

    /// A registered component with no `figmaProperty` on any prop.
    fn agreeing_component(h: &Harness, status: Status) -> ComponentId {
        let id = h.register("Button", status);
        h.write(
            "src/components/ui/button.tsx",
            "interface ButtonProps {\n  onClick: () => void;\n}\n\
             export function Button(p: ButtonProps) { return <div />; }\n",
        );
        h.amend(&id, |record| {
            record.props = vec![vds_core::PropContract {
                name: "onClick".into(),
                type_expr: "() => void".into(),
                required: true,
                figma_property: None,
            }];
        });
        id
    }

    #[test]
    fn two_spellings_of_one_set_agree_across_the_boundary() {
        // `'success' | 'warning'` in code and `Success, Warning` in the frame.
        // A designer types prose case and an engineer types code case, and this
        // is the pair the limb has to call equal or be switched off.
        let h = harness_with_intent(&["'success' | 'warning'"], &["Success", "Warning"]);
        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
        assert_eq!(outcome.exit_code, EXIT_PASSED, "{text}");
        assert!(text.contains("R11 and R12 ran"), "{text}");
    }

    /// The failing direction, and the one that matters: the code admits a value
    /// the frame does not draw.
    #[test]
    fn parity_fails_when_the_code_admits_a_value_the_frame_does_not_draw() {
        let h = harness_with_intent(
            &["'success' | 'warning' | 'danger'"],
            &["Success", "Warning"],
        );
        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert_eq!(outcome.status, ProofStatus::Failed);
        assert!(
            text.contains("the code admits 1 the frame does not draw"),
            "{text}"
        );
        assert!(text.contains("danger"), "{text}");
        // The consequence, not just the count. A value nobody designed renders
        // as whatever the fallback happens to be.
        assert!(text.contains("nobody has designed"), "{text}");
    }

    #[test]
    fn parity_fails_when_the_frame_draws_a_value_the_code_cannot_accept() {
        let h = harness_with_intent(&["'success'"], &["Success", "Warning"]);
        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(
            text.contains("the frame offers 1 the code cannot be given"),
            "{text}"
        );
        assert!(text.contains("nobody can ship"), "{text}");
    }

    #[test]
    fn both_directions_are_reported_in_one_finding_and_not_as_two() {
        // A renamed member is ONE change and produces a member on each side. Two
        // findings would double-count it and make a rename look worse than a
        // deletion plus an addition.
        let h = harness_with_intent(&["'success' | 'neutral'"], &["Success", "Default"]);
        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("the frame offers 1"), "{text}");
        assert!(text.contains("and the code admits 1"), "{text}");
    }

    #[test]
    fn a_figma_property_the_frame_does_not_declare_at_all_is_named_as_such() {
        let h = harness_with_intent(&["'success'"], &["Success"]);
        // Rename the variant property in the frame only, which is exactly what a
        // designer tidying a component set does.
        let id = vds_core::ComponentId::parse("CMP-0001").unwrap();
        h.figma_variants(&[(&id, &[("Tone", &["Success"])])]);
        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("none of them is \"Intent\""), "{text}");
        // Both causes read identically from the code side, and the finding says
        // so rather than picking one.
        assert!(text.contains("renamed or deleted"), "{text}");
    }

    #[test]
    fn a_prop_with_no_closed_union_is_undecided_and_never_an_agreement() {
        // The expected majority. `string` has no legal-value set, so the honest
        // answer to "do these two sets match" is that one of them does not exist.
        let h = harness_with_intent(&["string"], &["Success", "Warning"]);
        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
        assert!(
            text.contains("type_expression_is_not_a_closed_union"),
            "an undecided comparison must be VISIBLE, or a limb that decided nothing \
             reads exactly like a limb that agreed: {text}"
        );
        assert!(text.contains("is not an agreement"), "{text}");
    }

    #[test]
    fn with_no_figma_ledger_the_limb_is_unreached_and_says_so() {
        let h = harness_with_intent(&["'success'"], &["Success"]);
        std::fs::remove_file(h.root().join(".vds/ledgers/figma.yaml")).ok();
        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
        assert!(text.contains("R11 and R12 are unreached"), "{text}");
        assert!(text.contains("no_figma_ledger_on_disk"), "{text}");
    }

    /// The predicate directly, because the bug it fixes was silent at the unit
    /// level and only visible as a wrong finding two layers up.
    #[test]
    fn a_bare_type_name_is_not_read_as_a_one_member_enum() {
        use super::{is_legal_value_set, member_names, tokenise, top_level_union};
        let decide = |expr: &str| {
            let tokens = tokenise(expr).expect("tokenises");
            let members = top_level_union(&tokens);
            member_names(&members).is_some_and(|names| is_legal_value_set(&members, &names))
        };

        // Values.
        assert!(decide("'success' | 'warning'"));
        assert!(
            decide("'string'"),
            "a QUOTED 'string' really is the one-member set"
        );
        assert!(
            decide("primary|ghost"),
            "the shorthand PropContract publishes"
        );

        // Not values. Each of these produced a confident false finding before
        // the predicate existed: the limb read the type name as a member and
        // reported every real variant value as missing from it.
        assert!(!decide("string"));
        assert!(!decide("number"));
        assert!(!decide("ReactNode"));
        assert!(
            !decide("'success' | string"),
            "a widened union has unbounded values"
        );
        assert!(
            !decide("InvoiceStatus"),
            "a single bare word is overwhelmingly an imported alias, and guessing \"value\" \
             here guesses wrong nearly every time"
        );
    }

    #[test]
    fn a_record_with_no_figma_property_is_not_touched_by_this_limb() {
        // The limb must be silent on the ordinary case, or every existing record
        // acquires a finding the day it lands.
        let h = crate::testing::Harness::new();
        let id = agreeing_component(&h, Status::Registered);
        let _ = id;
        let (outcome, text) = run_kind(&h, ProofKind::Parity);
        assert!(!text.contains("figma-limb"), "{text}");
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
    }
}

#[cfg(test)]
mod drift_direction_tests {
    use vds_core::Drift;

    /// The direction must follow from WHICH SIDE IS MISSING THE THING, and the
    /// two prop rules must point OPPOSITE ways. Asserting them together is the
    /// point: if a later edit gave both the same direction the classification
    /// would still be present, still populated, and useless.
    #[test]
    fn the_two_prop_rules_point_in_opposite_directions() {
        let source = include_str!("parity.rs");
        // R5: the contract names a prop the code lacks. The CODE is behind.
        let r5 = source
            .find("RULE_PROP_NOT_IN_CODE,")
            .expect("R5 is emitted somewhere");
        let after_r5 = &source[r5..r5 + 900];
        assert!(
            after_r5.contains("Drift::Behind"),
            "R5 fires when the contract names a prop the component does not accept. That is \
             the CODE being behind, and an implementation is owed. It no longer says so."
        );

        // R6: the code accepts a prop no contract names. The CODE is ahead.
        let r6 = source
            .find("RULE_PROP_NOT_CONTRACTED,")
            .expect("R6 is emitted somewhere");
        let after_r6 = &source[r6..r6 + 900];
        assert!(
            after_r6.contains("Drift::Ahead"),
            "R6 fires when the component accepts a prop no contract names. That is the CODE \
             being ahead, and an AMENDMENT is owed rather than a fix - which is a different \
             job for a different person. It no longer says so."
        );

        // And they must not be the same. A classification where every finding
        // points one way carries no information at all.
        assert_ne!(
            Drift::Behind,
            Drift::Ahead,
            "the two directions have collapsed into one"
        );
    }

    /// `Undetermined` is the default and must stay silent rather than guessing.
    #[test]
    fn an_unclassified_finding_says_nothing_rather_than_guessing() {
        let plain = vds_core::Violation::fatal("l", "r", "e", "a");
        assert_eq!(plain.drift, Drift::Undetermined);
        assert!(
            !plain.drift.is_determined(),
            "a finding whose emitter has no opinion must not read as having one: a wrong \
             direction sends the work to the wrong person with a proof's authority behind it"
        );
        assert!(
            plain
                .clone()
                .with_drift(Drift::Mismatch)
                .drift
                .is_determined()
        );
        // The three real directions are all determined, or the printer drops them.
        for d in [Drift::Ahead, Drift::Behind, Drift::Mismatch] {
            assert!(d.is_determined(), "{d} would be printed as nothing");
        }
    }
}
