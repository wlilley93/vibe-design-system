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
    ComponentRecord, Project, ProofKind, PropContract, Result, State, Status, VdsError, Violation,
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

pub const FIGMA_NOTE: &str = "[figma] a prop's `figmaProperty` names a variant property in the decided-target file. \
     Resolving it is a call to the Figma API and VDS S-7(2)(1) forbids a network call inside a \
     proof, so it is NOT checked. A contract whose figmaProperty names a variant that was deleted \
     from the decided-target file passes this run.";

pub const REDACTION_NOTE: &str = "[redaction] a finding names the prop, the record, the source file and the SHAPE of the \
     disagreement in counts, and never the type expressions themselves. A type expression can \
     carry a realisation, a captured proof record lands under the tree `no_stored_values` scans, \
     and a finding that copied one would put it there permanently and fail that gate forever on a \
     file this proof wrote (VDS S-2(2), S-2(8)). Open the named record and the named source file \
     to read the two expressions.";

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

    run.note(SCOPE_NOTE);
    run.note(NORMALISATION_NOTE);
    run.note(EXPORT_LIMB_NOTE);
    run.note(PROPS_REACH_NOTE);
    run.note(STATES_REACH_NOTE);
    run.note(FIGMA_NOTE);
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
    let mut without_requirement: u64 = 0;

    for located in index.records() {
        let record = &located.value;
        let location = format!(
            "{} <{} {}>",
            project.rel(&located.path),
            record.id,
            record.name
        );
        let Some(export) = resolve(&mut run, project, record, &location, &library)? else {
            continue;
        };

        run.row(Verdict::Enforced);
        compare_props(&mut run, &mut undecided, record, &location, export);
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
fn resolve<'a>(
    run: &mut ProofRun<'_>,
    project: &Project,
    record: &ComponentRecord,
    location: &str,
    library: &Library<'a>,
) -> Result<Option<&'a LibraryExport>> {
    // Written out rather than matched with a wildcard: the lifecycle is closed by
    // VDS S-5(4), and a wildcard would silently enforce whatever an eighth status
    // turned out to mean.
    match record.status {
        Status::Proposed | Status::Designed => {
            run.row(Verdict::Skipped(SKIP_BELOW_REGISTERED));
            return Ok(None);
        }
        Status::Retired => {
            run.row(Verdict::Skipped(SKIP_RETIRED));
            return Ok(None);
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
        return Ok(None);
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
        return Ok(None);
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
        return Ok(None);
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
        return Ok(None);
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
        return Ok(None);
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
        ));
        return Ok(None);
    };

    if export.props_incomplete_because.is_some() {
        // The scanner found no `<Name>Props` type in the file, so its prop list
        // is empty for want of a reader rather than because the component takes
        // nothing. Comparing a contract against that would fail on every prop the
        // contract names, and the fault would be the scanner's.
        run.row(Verdict::Skipped(SKIP_NO_PROPS_TYPE));
        if record.props.is_empty() {
            run.inform(Violation::fatal(
                format!("{location} -> {source}"),
                RULE_NO_PROPS_TYPE,
                "a `<Name>Props` interface or type alias beside the export, so that the prop set \
                 can be compared in both directions",
                format!(
                    "{source} declares none, and {} names no prop either. Neither side claims \
                     anything, so this row establishes nothing rather than agreeing with itself",
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
                    "{source} declares no such type, so the {} props {} names were compared \
                     against nothing. This is either drift the scanner cannot see or a props \
                     type declared somewhere this build does not follow, and both need a human",
                    record.props.len(),
                    record.id
                ),
            ));
        }
        return Ok(None);
    }

    Ok(Some(export))
}

/// R5, R6, R7, R8 and R10, in both directions.
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
            run.fail(Violation::fatal(
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
            ));
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
            ));
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
                ));
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
        run.fail(Violation::fatal(
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
        ));
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
    ));
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
            while at < chars.len() && is_word_char(chars[at]) {
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
        ComponentId, EXIT_PASSED, EXIT_VACUOUS, EXIT_VIOLATION, ProofKind, ProofStatus,
        PropContract, Severity, State, StateContract, Status, default_config,
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
}
