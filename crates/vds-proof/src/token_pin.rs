//! The `token_pin` proof.
//!
//! VDS S-7(5): "the two named records agree where the pin declares them
//! aligned". VDS S-2(3) fixes which two records those are and it is not VDS's to
//! move them: `app/globals.css` is the system of record for what ships, and the
//! decided-target Figma file is the system of record for what is decided.
//!
//! # Why this gate only ever CHECKS a pin and never derives one
//!
//! One of the two records is behind a network call, and VDS S-7(2)(1) forbids a
//! network call inside a proof. A proof that reached Figma would not be
//! re-runnable, would not be deterministic offline, and would therefore not be a
//! proof at all. So the shape is the one `vds figma pull` already uses: a
//! GENERATOR runs out of band, with the network, and writes a pin; this gate
//! reads that pin offline and refuses it wherever it cannot be relied on. That
//! split is the design and it is kept, not worked around.
//!
//! # Why a row carries no value, and why this gate must not put one back
//!
//! VDS S-2(7) as drafted required a pin row to carry `source_value_digest` and
//! `target_value_digest`. That construction was measured and it does not work: a
//! hex colour is a 24-bit domain, an unsalted digest over it is not one-way, and
//! all 52 values came back out of a 26-row pin in 27 seconds. A row therefore
//! carries a NAME and an AGREEMENT and nothing a brute force can invert, and
//! `vds_core::types::pin` holds a test to that.
//!
//! This gate is downstream of that defect, so it inherits the duty. Its findings
//! land in `.vds/proofs/`, a proof record is never deleted (VDS S-9(1) in
//! spirit, and S-3(9) in practice), and `no_stored_values` scans that tree. A
//! finding that copied a realisation out of a pin would make that proof fail
//! forever on a file this one wrote. See [`REDACTION_NOTE`].
//!
//! # The rules
//!
//!   R1  a row the pin declares aligned does not agree. `agrees` is false and no
//!       `not_enforced_because` is present, so the pin is asserting a
//!       disagreement it has not excused.
//!   R2  the pin is NOT current with the local half. Derived, not trusted: the
//!       shipped record is re-digested here and compared with the digest the pin
//!       recorded for it. `generated_at` is read by nothing in this gate, because
//!       a stamp is a claim about when a thing was done and not evidence that it
//!       was.
//!   R3  the pin names a shipped record this build cannot read, so no freshness
//!       can be derived for it and none of its rows can be relied on.
//!   R4  the pin contradicts itself: it does not fail closed, or its own counts
//!       disagree with the rows it holds (`Pin::defects`).
//!   R5  the pin's `digest` does not match what the pin says, so it was edited
//!       after it was generated. A pin is a generated artefact and never
//!       hand-edited, and without this rule flipping `agrees: false` to `true` in
//!       a committed pin produced a PASSING run.
//!   W1  coverage. The pin holds fewer enforced rows than the shipped record
//!       declares custom properties, so it is evidence about at most that many
//!       and about none of the rest. A warning and not a failure: a pin scoped to
//!       one subject is lawful, and failing every partial pin would take this
//!       gate out of service. The NUMBER is what stops a warrant overclaiming.
//!   I1  a row the pin declined to enforce, counted and named with the reason it
//!       gave, so the carve-out is auditable rather than a silent absence.
//!
//! A row is one PIN ROW, not one pin. A pin holding zero rows therefore
//! contributes zero enforceable rows, and a run over nothing else is vacuous at
//! exit 3 rather than a pass (VDS S-7(2)(4)).
//!
//! # The half this gate re-derives, and the half it cannot
//!
//! It re-derives the LOCAL half completely: the shipped record is on disk, so
//! whether the pin still describes it is a fact this gate establishes rather than
//! accepts. It cannot re-derive the DECIDED-TARGET half at all, and says so on
//! every record it captures ([`FIGMA_REACH_NOTE`]). A pin generated once and
//! never regenerated agrees with a Figma file nobody has looked at since, and no
//! amount of offline checking changes that.
//!
//! This proof reads pin row NAMES, agreement FLAGS, record LOCATORS and file
//! DIGESTS, and it counts custom-property NAMES in the shipped record. It reads
//! no design value (VDS S-2(2)), and it writes none.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

use regex::Regex;
use vds_core::{Digest, PathRole, Pin, PinRow, Project, ProofKind, Result, VdsError, Violation};

use crate::ProofContext;
use crate::run::{Outcome, ProofRun, Verdict};

pub const GATE: &str = "crates/vds-proof/src/token_pin.rs";

const RULE_DISAGREES: &str =
    "VDS S-7(5) token_pin R1: the two named records agree where the pin declares them aligned";
const RULE_NOT_CURRENT: &str = "VDS S-7(5) token_pin R2 / S-2(5)(2): a pin is current with the shipped record it was \
     derived from, or it is describing bytes that are gone";
const RULE_UNREADABLE_SOURCE: &str = "VDS S-2(3) token_pin R3: a pin names the shipped record as its source, and this build \
     can read that record";
const RULE_SELF_CONTRADICTORY: &str =
    "VDS S-2(5)(2) token_pin R4: a pin fails closed and its own counts match the rows it holds";
const RULE_DIGEST_MISMATCH: &str = "VDS S-2(5)(4) token_pin R5: a pin's digest matches what the pin says, so a generated \
     agreement can be told from a hand-edited one";
const RULE_COVERAGE: &str = "VDS S-7(2)(4) token_pin W1: a pin is evidence about the rows it holds and about nothing \
     else";
const RULE_DECLINED: &str = "VDS S-7(2)(4) token_pin I1: a row the pin declined to enforce is named and counted, so \
     the carve-out is auditable";

/// A stable machine key, not a sentence. It becomes a count in
/// `rows_skipped_reasons`, and a per-row sentence would make every count one.
const SKIP_DECLINED: &str = "row_the_pin_declined_to_enforce";
const SKIP_NOT_CURRENT: &str = "pin_not_current_with_the_shipped_record";
const SKIP_SOURCE_UNREADABLE: &str = "pin_names_a_shipped_record_this_build_cannot_read";
const SKIP_DIGEST_MISMATCH: &str = "pin_edited_after_it_was_generated";

pub const DERIVATION_NOTE: &str = "[derived] freshness against the local half is DERIVED and not trusted: the shipped record \
     named by the pin is re-digested by this run and compared with the digest the pin recorded \
     for it. `generated_at` is read by nothing here. A stamp says when somebody claims a thing \
     was done, and a pin regenerated against a record that has since moved carries a perfectly \
     recent stamp (VDS S-2(5)(4)).";

pub const FIGMA_REACH_NOTE: &str = "[reach] what this run does NOT reach: the decided-target half. That record is a Figma file \
     behind a network call, VDS S-7(2)(1) forbids a network call inside a proof, and nothing \
     offline can re-derive it. This run therefore establishes that the pin still describes the \
     SHIPPED record accurately, and never that the pin still describes the DECIDED one. A pin \
     generated once and never regenerated agrees with a file nobody has looked at since, and a \
     warrant citing this proof must not be described as covering the decided-target side \
     (VDS S-6(3)).";

pub const REDACTION_NOTE: &str = "[redaction] a finding names the pin file, the 1-based row position, the class of defect and \
     the numbers, and it repeats a string the pin authored only where that string provably \
     cannot be a realisation. A captured proof record lands under the tree `no_stored_values` \
     scans and is never deleted, so a finding that copied a realisation out of a pin would make \
     that proof fail forever on a file this one wrote. The screen is deliberately over-eager: \
     withholding a safe name costs a reader one file open, and repeating an unsafe one costs a \
     gate that can never go green again.";

pub const SELF_DIGEST_NOTE: &str = "[integrity] a hand-edited agreement flag is REFUSED, not merely detected. Every pin's \
     `digest` is recomputed here from what the pin says (its subject, direction, both records, \
     rows, counts, fails_closed and generated_by, and deliberately not its `generated_at`), and \
     a pin whose digest does not match its own contents has its rows skipped rather than \
     enforced (R5). Flipping `agrees: false` to `true` in a committed pin therefore produces a \
     run that enforces nothing about that pin and says why, instead of a pass. What this does \
     NOT establish is that the generator computed the rows correctly in the first place: it \
     binds the pin to itself, not to the two records it claims to have compared. The source \
     side is separately re-derived (see the derived note); the decided-target side cannot be.";

pub const READER_NOTE: &str = "[scope] the pins directory is read as `*.yaml` exactly, so a pin in a subdirectory, or \
     one saved as `.yml`, is invisible to this run rather than reported by it. The register \
     reader refuses such an entry and the pin reader does not, and until it does, an absent pin \
     and an unreadable-by-this-reader pin look identical from here.";

pub fn run(ctx: &ProofContext, out: &mut dyn Write) -> Result<Outcome> {
    let project = ctx.project;
    let mut run = ctx.new_run(ProofKind::TokenPin, GATE);
    run.input_file(&project.config_path)?;

    // An unparseable pin fails HERE, as a precondition, rather than being
    // skipped. A reader of a partial pin set is reading a set that says
    // something the directory does not, and every count taken from it is wrong
    // by an unknown amount.
    let pins = ctx.store().read_pins()?;

    run.note(DERIVATION_NOTE);
    run.note(FIGMA_REACH_NOTE);
    run.note(REDACTION_NOTE);
    run.note(SELF_DIGEST_NOTE);
    run.note(READER_NOTE);

    // An empty pins directory is a VACUITY, not a precondition failure, and the
    // distinction is the one VDS S-7(2)(4) exists to draw. A precondition failure
    // means the check could not run; a vacuity means it ran, walked its subject
    // and found nothing in an enforceable state. This walked the directory. Every
    // sibling proof settles the same shape the same way: `states` over an empty
    // register and `retirement_drain` over a project with nothing deprecated are
    // both vacuous, and a vacuous record is never evidence, so the caller is told
    // exactly as little as it should be.
    //
    // Getting this wrong once cost the whole run. As a precondition it aggregated
    // to exit 2, `--allow-vacuous` does not relax a precondition, and every
    // project without a pin went red on a kind it may lawfully have nothing to
    // say about.
    if pins.is_empty() {
        run.note(format!(
            "[vacuous] no pin exists under {}, so there are no two named records to report the \
             agreement of and this run enforces nothing. A pin is GENERATED OUT OF BAND and \
             only CHECKED here: one of the two records VDS S-2(3) names is a Figma file behind \
             a network call, and VDS S-7(2)(1) forbids a network call inside a proof, so this \
             gate cannot derive one however it is invoked. This record is `vacuous` and is not \
             evidence for any warrant (VDS S-7(2)(4)).",
            project.rel(&project.path(PathRole::Pins))
        ));
    }

    // One reading per distinct shipped record. Two pins naming one record must be
    // judged against ONE reading of it: a file edited between two reads would
    // otherwise let the first pin pass and the second fail on bytes that were
    // never both true at once.
    let mut shipped: BTreeMap<PathBuf, std::result::Result<Shipped, String>> = BTreeMap::new();

    for located in &pins {
        let pin = &located.value;
        let where_from = project.rel(&located.path);

        // The pin's CONTENT, not its file. The file carries `generated_at`, which
        // moves every time the generator runs over an unchanged pair of records,
        // and digesting the file would move this proof's evidence digest with it
        // and make every warrant citing it look spent (VDS S-7(2)(1)).
        run.input_named(
            format!("<pin {} content>", pin.id),
            content_digest(pin, &where_from)?,
        );
        // Recorded and NOT verified. Nothing offline can re-derive the decided
        // target, so the most this run can do is bind its evidence to the value
        // the pin recorded: a re-pull that moved the target moves this digest,
        // and a warrant pinned to the old one stops matching.
        run.input_named(
            format!(
                "<pin {} decided-target digest, as the pin recorded it>",
                pin.id
            ),
            pin.target_of_record.digest.clone(),
        );

        // R4 first. A pin whose counts disagree with its rows is unreliable about
        // everything below, and reporting the rows without reporting that would
        // hand a reader numbers from a file that already contradicts itself.
        for defect in pin.defects() {
            run.fail(Violation::fatal(
                where_from.clone(),
                RULE_SELF_CONTRADICTORY,
                "a pin that fails closed, whose rows_considered is the number of rows it holds, \
                 and whose rows_enforced is the number of those rows carrying no \
                 not_enforced_because",
                defect,
            ));
        }

        // R5. Before anything is read on the pin's authority: a pin whose digest
        // does not match what it says was edited after it was generated, and the
        // edit that matters is `agrees: false` becoming `agrees: true`. Its rows
        // are skipped rather than enforced, because enforcing a row that a hand
        // may have written is the storing form wearing a boolean.
        //
        // Skipped and not failed-then-enforced: crediting rows that establish
        // nothing is the arithmetic half of the [2026] VJS-CC-OPBOX 3 D3 defect.
        match pin.digest_matches() {
            Ok(true) => {}
            Ok(false) => {
                run.fail(Violation::fatal(
                    where_from.clone(),
                    RULE_DIGEST_MISMATCH,
                    "a `digest` equal to the digest of what the pin says: its subject, \
                     direction, both records, rows, counts, fails_closed and generated_by. A pin \
                     is a GENERATED artefact (VDS S-2(5)(4)) and is regenerated, never edited.",
                    format!(
                        "the pin's digest does not match its own contents, so it was edited \
                         after it was generated. None of its {} row(s) is relied on by this \
                         run. Regenerate it with the command in its own `generated_by` field.",
                        pin.rows.len()
                    ),
                ));
                for _ in &pin.rows {
                    run.row(Verdict::Skipped(SKIP_DIGEST_MISMATCH));
                }
                continue;
            }
            Err(error) => {
                run.fail(Violation::fatal(
                    where_from.clone(),
                    RULE_DIGEST_MISMATCH,
                    "a pin this build can digest, so that a generated agreement can be told \
                     from a hand-edited one.",
                    format!(
                        "the pin could not be digested ({error}), so whether it was edited \
                         cannot be established and none of its {} row(s) is relied on.",
                        pin.rows.len()
                    ),
                ));
                for _ in &pin.rows {
                    run.row(Verdict::Skipped(SKIP_DIGEST_MISMATCH));
                }
                continue;
            }
        }

        let record = match resolve_shipped(project, &pin.source_of_record.locator) {
            Err(reason) => Err(reason),
            Ok(path) => shipped
                .entry(path.clone())
                .or_insert_with(|| read_shipped(project, &path))
                .clone(),
        };

        let record = match record {
            Ok(record) => record,
            Err(reason) => {
                // A finding and not a precondition failure. This run DID run, and
                // it can still report every other pin; refusing here would report
                // one pin's unreadable source by saying nothing about any of them.
                // The same reasoning `ledger_staleness` uses for R1.
                run.fail(Violation::fatal(
                    where_from.clone(),
                    RULE_UNREADABLE_SOURCE,
                    "a source_of_record.locator naming a readable UTF-8 file inside this \
                     repository. VDS S-2(3) makes `app/globals.css` the system of record for \
                     what ships, and it is the SOURCE side of a pin; the decided-target Figma \
                     file is the TARGET side.",
                    format!(
                        "{reason}. No freshness can be derived for this pin, so none of its \
                         {} row(s) is relied on by this run.",
                        pin.rows.len()
                    ),
                ));
                for _ in &pin.rows {
                    run.row(Verdict::Skipped(SKIP_SOURCE_UNREADABLE));
                }
                continue;
            }
        };

        // The digest recorded as the input is the one this run COMPARED, not a
        // second reading taken afterwards. A file edited between the two would
        // otherwise leave the record witnessing bytes the finding is not about.
        run.input_named(project.rel(&record.path), record.digest.clone());

        let enforceable = pin.rows.iter().filter(|row| row.is_enforced()).count();
        report_coverage(&mut run, project, pin, &where_from, &record, enforceable);

        if record.digest != pin.source_of_record.digest {
            run.fail(Violation::fatal(
                where_from.clone(),
                RULE_NOT_CURRENT,
                format!(
                    "{} digests to the value the pin recorded for it, so every row in the pin is \
                     an assertion about the bytes that are there now",
                    project.rel(&record.path)
                ),
                format!(
                    "{} has moved since this pin was generated: the pin recorded {} and it now \
                     digests to {}. Every row here describes bytes that are gone, so none of the \
                     {} row(s) is relied on by this run. Regenerate the pin against both named \
                     records.",
                    project.rel(&record.path),
                    pin.source_of_record.digest,
                    record.digest,
                    pin.rows.len()
                ),
            ));
            for _ in &pin.rows {
                run.row(Verdict::Skipped(SKIP_NOT_CURRENT));
            }
            continue;
        }

        for (index, row) in pin.rows.iter().enumerate() {
            report_row(&mut run, &where_from, index, row);
        }
    }

    run.finish(&ctx.capture_options()?, out)
}

/// One row: enforced and checked, or declined, counted and named.
fn report_row(run: &mut ProofRun, where_from: &str, index: usize, row: &PinRow) {
    // 1-based, because the reader is being sent to a position in a YAML sequence
    // and a 0-based one sends them to the row before the defect.
    let position = index + 1;
    let location = match repeatable(&row.name) {
        Some(name) => format!("{where_from} <row {position}: {name}>"),
        None => format!("{where_from} <row {position}>"),
    };

    let Some(because) = &row.not_enforced_because else {
        run.row(Verdict::Enforced);
        if !row.agrees {
            run.fail(Violation::fatal(
                location,
                RULE_DISAGREES,
                "agrees: true, or a not_enforced_because saying why this row is outside the \
                 pin's reach. A row is one of the two, and a bare disagreement is the pin \
                 reporting that the shipped record and the decided target have come apart.",
                "agrees: false, with no not_enforced_because. The two named records disagree \
                 about this row and the pin does not excuse it. Neither value is repeated here \
                 or anywhere in the record; read them in the two records VDS S-2(3) names.",
            ));
        }
        return;
    };

    run.row(Verdict::Skipped(SKIP_DECLINED));
    // Informational rather than a warning: an excused row is the pin working as
    // designed, and a warning per excused row would bury the ones that are not.
    // It is captured rather than printed, because a carve-out visible only as a
    // number in a skip count is a carve-out nobody can act on.
    run.inform(Violation::fatal(
        location,
        RULE_DECLINED,
        "a row this pin enforces, or one it declines for a reason a reader can weigh",
        format!(
            "not enforced, because: {}{}",
            quoted(because),
            if row.agrees {
                ". The row also carries agrees: true, and the two cannot both be relied on: a \
                 row the pin declined to enforce did not establish the agreement it claims."
            } else {
                ""
            }
        ),
    ));
}

/// W1, and the number VDS S-7(2)(4) needs in the record whether or not it fires.
///
/// The note is unconditional and the warning is not. A pin scoped to one subject
/// is lawful and failing it would take this gate out of service, but a reader of
/// the record has to be able to see the size of the claim without reading the
/// pin, or a warrant will be written as though the pin covered the record.
fn report_coverage(
    run: &mut ProofRun,
    project: &Project,
    pin: &Pin,
    where_from: &str,
    record: &Shipped,
    enforceable: usize,
) {
    let declared = record.custom_properties;
    let outside = declared.saturating_sub(enforceable);

    run.note(format!(
        "[coverage] {}: the shipped record it names ({}) declares {declared} distinct custom \
         properties and the pin holds {enforceable} row(s) it enforces, so this pin is evidence \
         about at most {enforceable} of them and about none of the other {outside}. The \
         denominator is a name-only scan of the shipped record: it can overcount a declaration \
         that is commented out, which understates coverage, and it does not undercount, so the \
         claim this number bounds is bounded downward.",
        pin.id,
        project.rel(&record.path)
    ));

    if outside > 0 {
        run.warn(Violation::fatal(
            where_from.to_owned(),
            RULE_COVERAGE,
            format!(
                "either a pin whose enforced rows reach all {declared} custom properties the \
                 shipped record declares, or a warrant that records the shortfall instead of \
                 reading this pin as evidence about the whole record (VDS S-6(3))"
            ),
            format!(
                "{declared} declared, {enforceable} enforced: this pin says nothing at all about \
                 the other {outside}. It is a warning and not a failure, because a pin scoped to \
                 one subject is lawful; the number is here so a warrant cannot be written as \
                 though it were not."
            ),
        ));
    }
}

// -- the shipped half ---------------------------------------------------------

/// One reading of the local record a pin names.
#[derive(Clone)]
struct Shipped {
    path: PathBuf,
    /// Over the raw bytes, which is what a generator digesting the same file
    /// produces.
    digest: Digest,
    /// How many DISTINCT custom-property names the record declares. The names
    /// themselves are counted and dropped: see [`declared_custom_properties`].
    custom_properties: usize,
}

/// The path a pin's source locator names, refused where it leaves the repository.
///
/// An absolute locator, or one climbing out through `..`, names a file no
/// reviewer sees in the diff that changes it. Pinning against bytes nobody
/// reviews is the storing form wearing a path, so it is refused rather than read.
fn resolve_shipped(project: &Project, locator: &str) -> std::result::Result<PathBuf, String> {
    let trimmed = locator.trim();
    if trimmed.is_empty() {
        return Err(
            "the pin's source_of_record.locator is empty, so it names no record".to_owned(),
        );
    }
    let candidate = Path::new(trimmed);
    if candidate.is_absolute() {
        return Err(format!(
            "the pin's source_of_record.locator {} is absolute, and every record path is \
             repository-relative",
            quoted(trimmed)
        ));
    }
    if candidate
        .components()
        .any(|part| matches!(part, Component::ParentDir))
    {
        return Err(format!(
            "the pin's source_of_record.locator {} climbs out of the repository, so it names \
             bytes no reviewer sees in the diff that changes them",
            quoted(trimmed)
        ));
    }
    Ok(project.root.join(candidate))
}

/// Read the local record once: its digest, and how much of it there is to cover.
///
/// Non-UTF-8 bytes are a refusal rather than a lossy read. The digest would still
/// be exact, but the coverage count would be taken over characters the file does
/// not contain, and a number derived from a guess is worse in a captured record
/// than no number at all.
fn read_shipped(project: &Project, path: &Path) -> std::result::Result<Shipped, String> {
    if !path.is_file() {
        return Err(format!(
            "{} is not a file in this repository",
            project.rel(path)
        ));
    }
    let bytes =
        std::fs::read(path).map_err(|e| format!("{} could not be read: {e}", project.rel(path)))?;
    let digest = Digest::of_bytes(&bytes);
    let text = String::from_utf8(bytes).map_err(|_| {
        format!(
            "{} is not UTF-8 text, so this build cannot count what the pin covers in it",
            project.rel(path)
        )
    })?;
    Ok(Shipped {
        path: path.to_path_buf(),
        digest,
        custom_properties: declared_custom_properties(&text),
    })
}

/// How many DISTINCT custom-property names a stylesheet declares.
///
/// A name-only scan, and deliberately not a parse. `vds_css::Sheet` models
/// comments, the cascade, layers and conditional at-rules exactly, and it is the
/// right tool for resolving a VALUE; it is not a dependency of this crate and
/// this number is not a value. What the number has to be is a denominator that
/// bounds a claim downward, and the two ways this scan can be wrong are not
/// symmetric:
///
///   - it OVERCOUNTS a declaration that is commented out, or a fragment of a
///     selector that looks like one. That inflates the denominator, understates
///     coverage, and makes W1 fire when it need not. Loud and safe.
///   - it could UNDERCOUNT only a declaration whose name and colon are separated
///     by something other than whitespace, which no stylesheet writes. `\s`
///     spans newlines, so a name and colon on two lines are still one match.
///
/// The pattern captures the NAME and nothing after the colon, so no value is read
/// even transiently, and only the COUNT of distinct names leaves this function.
/// A custom-property name can itself spell a realisation (`--radius-8px`), so
/// carrying the names out of here would be carrying a realisation out of here.
fn declared_custom_properties(text: &str) -> usize {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    // The leading class is CONSUMED rather than looked behind, because the regex
    // crate has no lookbehind. It is what keeps `.card--body:hover` and
    // `var(--x)` out: a name glued to a word character or opening a function call
    // is not a declaration.
    let pattern = PATTERN.get_or_init(|| {
        Regex::new(r"(?m)(?:^|[\s;{])(--[A-Za-z0-9_-]+)\s*:").expect("a constant pattern")
    });
    let names: BTreeSet<&str> = pattern
        .captures_iter(text)
        .filter_map(|found| found.get(1).map(|name| name.as_str()))
        .collect();
    names.len()
}

// -- the redaction screen -----------------------------------------------------

/// What a withheld string is replaced with.
///
/// It names the reason rather than printing an opaque marker, so a reader who
/// finds one knows to go and look rather than assuming the field was empty.
const WITHHELD: &str = "<withheld: this text could itself be a realisation, and a proof record is never deleted. \
     Read it at the row named above, in the pin file named above.>";

/// The longest authored string this gate will copy into a permanent record.
///
/// A pin row name is a token name and a reason is a phrase. Anything longer is a
/// paste, and copying a paste into a record that is never deleted is exactly the
/// surface the redaction rule closes.
const MAX_REPEATED_CHARS: usize = 240;

/// CSS time units. Two entries, mirroring the private list in
/// [`crate::no_stored_values`]; the length units are shared from there directly
/// rather than copied, so the two cannot drift.
const TIME_UNITS: &[&str] = &["s", "ms"];

/// Words that would themselves trip `no_stored_values` if they landed in a
/// captured record.
///
/// Calibrated to the patterns that gate actually fires on (R2 colour functions,
/// R5 easing, R6 generic font families) rather than to a wider idea of what
/// looks like a design value: rejecting more than that costs names in findings
/// and buys nothing, and rejecting less would let a record through that makes
/// `no_stored_values` fail forever.
///
/// This is a second list saying what a private list in that module says. It
/// exists because `literals_in` and its `Patterns` are private there, and the
/// right fix is to expose them and call them from here rather than to keep two
/// lists in step by hand.
const REPEAT_UNSAFE_WORDS: &[&str] = &[
    "rgb(",
    "rgba(",
    "hsl(",
    "hsla(",
    "hwb(",
    "oklch(",
    "oklab(",
    "lch(",
    "lab(",
    "color(",
    "color-mix(",
    "cubic-bezier",
    "steps(",
    "linear(",
    "ease-in",
    "ease-out",
    "ui-sans-serif",
    "ui-monospace",
    "ui-serif",
    "ui-rounded",
    "sans-serif",
    "system-ui",
    "monospace",
];

/// The string, where copying it into a permanent record provably cannot plant a
/// realisation there.
///
/// Over-eager on purpose. The two costs are not comparable: withholding a name
/// that was safe sends a reader to open one file, and repeating one that was not
/// writes a realisation into `.vds/proofs/`, where `no_stored_values` finds it on
/// the next run and keeps finding it, on a record that is never deleted and a
/// gate with no lawful way back to green.
fn repeatable(text: &str) -> Option<&str> {
    if text.is_empty() || text.chars().count() > MAX_REPEATED_CHARS {
        return None;
    }
    // Every colour literal spelling carries the sigil, so this one test closes
    // the whole class without needing to count hex digits.
    if text.contains('#') {
        return None;
    }
    let lowered = text.to_ascii_lowercase();
    if REPEAT_UNSAFE_WORDS
        .iter()
        .any(|word| lowered.contains(word))
    {
        return None;
    }
    if carries_a_unit(&lowered) || carries_an_encoded_run(text) {
        return None;
    }
    Some(text)
}

/// Whether a number in this text is glued to a CSS length or time unit.
///
/// The unit lists decide it, not the mere presence of letters after digits:
/// `--text-2xl` is a name and `--radius-8px` holds a length, and a screen that
/// could not tell them apart would withhold half the names in a real pin.
fn carries_a_unit(lowered: &str) -> bool {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    let pattern = PATTERN
        .get_or_init(|| Regex::new(r"[0-9]+(?:\.[0-9]+)?([a-z]+)").expect("a constant pattern"));
    pattern.captures_iter(lowered).any(|found| {
        let unit = found
            .get(1)
            .expect("the unit group is not optional")
            .as_str();
        crate::no_stored_values::LENGTH_UNITS.contains(&unit) || TIME_UNITS.contains(&unit)
    })
}

/// Whether this text holds a hexadecimal run long enough and even enough to be a
/// reversible encoding of a realisation, or a digest of one.
///
/// `no_stored_values` R8 decodes an even-length hex run and re-tests the result,
/// and R10 sweeps the candidate space against any 32, 40 or 64 character run.
/// The shortest realisation is four characters, so an eight-character even run is
/// the floor at which either limb can fire. Rejecting from six is one notch under
/// that floor, which costs a name like `--cafebabe` and cannot cost a gate.
fn carries_an_encoded_run(text: &str) -> bool {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    let pattern = PATTERN.get_or_init(|| Regex::new(r"[0-9A-Fa-f]+").expect("a constant pattern"));
    pattern
        .find_iter(text)
        .any(|found| found.len() >= 6 && found.len().is_multiple_of(2))
}

/// An authored string, quoted for a finding, or the withheld marker.
fn quoted(text: &str) -> String {
    match repeatable(text) {
        Some(safe) => format!("{safe:?}"),
        None => WITHHELD.to_owned(),
    }
}

// -- the pin's content --------------------------------------------------------

/// The digest of what a pin SAYS, excluding when it was written.
///
/// `generated_at` moves every time the generator runs, and the pin's `id` is
/// derived from it, so digesting the file would move this proof's evidence digest
/// on a regeneration that changed no verdict, and every warrant citing it would
/// look spent (VDS S-7(2)(1)). The pin's own `digest` field is excluded for a
/// different reason: nothing in this build derives it, so it is a value this run
/// neither checks nor relies on, and binding evidence to an unchecked field would
/// make a hand-edited one look like a change of subject.
///
/// This is emphatically NOT a recomputation of `pin.digest`. The generator's
/// canonicalisation is unknown here, and asserting that this one is it would fail
/// every real pin for a reason that has nothing to do with the records.
/// The pin's own content digest, computed by `vds_core::Pin` and never here.
///
/// One definition. This module used to hold its own canonicalisation, and two
/// canonicalisations of one shape drift: the drift shows up as a pin that passes
/// this gate and fails its own generator, which is the worst possible place for
/// a disagreement about what a record says.
fn content_digest(pin: &Pin, where_from: &str) -> Result<Digest> {
    pin.compute_content_digest()
        .map_err(|e| VdsError::Artefact {
            path: where_from.to_owned(),
            message: format!("could not be digested, so this run cannot witness what it read: {e}"),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{Harness, run_kind};
    use vds_core::{
        EXIT_PASSED, EXIT_VACUOUS, EXIT_VIOLATION, PinDirection, PinId, ProofKind, ProofStatus,
        RecordOfTruth, Severity, Timestamp,
    };

    const SHIPPED: &str = "app/globals.css";

    /// The shipped record, declaring one custom property per name given.
    ///
    /// The VALUES are placeholders and not colours, because this proof never
    /// reads one: a pin row carries a name and a verdict, so a fixture that
    /// spelled real colours here would be testing nothing extra and putting
    /// realisations in this file for no reason.
    fn shipped_record(h: &Harness, properties: &[&str]) {
        let body: String = properties
            .iter()
            .map(|name| format!("  {name}: token-a;\n"))
            .collect();
        h.write(SHIPPED, &format!(":root {{\n{body}}}\n"));
    }

    /// A pin over the shipped record as it stands right now.
    fn pin_over_shipped(h: &Harness, rows: Vec<PinRow>) -> PinId {
        pin_with(h, SHIPPED, rows, None)
    }

    /// A pin, with every knob a test might need to turn.
    fn pin_with(
        h: &Harness,
        locator: &str,
        rows: Vec<PinRow>,
        source_digest: Option<Digest>,
    ) -> PinId {
        let store = h.store();
        let id = PinId::allocate(&store.pins_dir(), &Timestamp::fixed(2026, 7, 25, 10, 0, 0))
            .expect("a pin id");
        let live = Digest::of_file(&h.root().join(SHIPPED))
            .unwrap_or_else(|_| Digest::of_text("no shipped record in this fixture"));
        let enforced = rows.iter().filter(|row| row.is_enforced()).count() as u64;
        let pin = Pin {
            id: id.clone(),
            subject: "the control boundary".into(),
            direction: PinDirection::OneWayDerived,
            source_of_record: RecordOfTruth {
                authority_for: "what ships".into(),
                locator: locator.into(),
                digest: source_digest.unwrap_or(live),
            },
            target_of_record: RecordOfTruth {
                authority_for: "what is decided".into(),
                locator: "FIGMAKEY".into(),
                // Not a file digest: it is whatever the out-of-band generator
                // recorded for the decided-target file, and nothing offline can
                // check it.
                digest: Digest::of_text("the decided target, as the generator saw it"),
            },
            rows_considered: rows.len() as u64,
            rows_enforced: enforced,
            rows,
            fails_closed: true,
            generated_at: Timestamp::fixed(2026, 7, 25, 10, 0, 0),
            generated_by: "the out-of-band pin generator".into(),
            // A placeholder, replaced below. A generator computes this from what
            // it wrote, and R5 refuses a pin whose digest does not match, so a
            // fixture that skipped it would be testing a hand-edited pin.
            digest: Digest::of_text("placeholder"),
            proof_id: None,
        };
        let pin = Pin {
            digest: pin.compute_content_digest().expect("a pin digests"),
            ..pin
        };
        store
            .create(&store.pins_dir().join(format!("{id}.yaml")), &pin)
            .expect("the pin writes");
        id
    }

    /// One shipped record with one custom property, and a pin that agrees about
    /// it. The state most tests start from.
    fn seeded() -> Harness {
        let h = Harness::new();
        shipped_record(&h, &["--control-border"]);
        pin_over_shipped(&h, vec![PinRow::compare("control-border", "a", "a")]);
        h
    }

    #[test]
    fn a_pin_whose_rows_agree_over_a_current_shipped_record_passes() {
        let h = seeded();
        let (outcome, text) = run_kind(&h, ProofKind::TokenPin);
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
        assert_eq!(outcome.exit_code, EXIT_PASSED);
        assert_eq!(outcome.rows_enforced, 1, "one pin row is one row: {text}");
    }

    /// The failing-direction test VDS S-7(2)(2) requires, and the one
    /// `.vds/enforcement.lock` will name. It seeds a pin row that the pin
    /// declares aligned and that does not agree, and asserts the non-zero exit.
    #[test]
    fn token_pin_fails_on_a_row_the_pin_declares_aligned_that_does_not_agree() {
        let h = Harness::new();
        shipped_record(&h, &["--control-border"]);
        pin_over_shipped(
            &h,
            vec![
                PinRow::compare("control-border", "a", "b"),
                PinRow::compare("surface", "a", "a"),
            ],
        );

        let (outcome, text) = run_kind(&h, ProofKind::TokenPin);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert_eq!(outcome.status, ProofStatus::Failed);
        assert_eq!(outcome.rows_enforced, 2, "both rows were checked: {text}");
        assert!(text.contains("row 1: control-border"), "{text}");
        assert!(text.contains("agrees: false"), "{text}");
    }

    /// R2, and the reason this proof is not simply a reader of the pin's own
    /// verdicts. The pin is untouched and internally perfect; the record it was
    /// derived from is not the record that ships now.
    #[test]
    fn token_pin_fails_on_a_pin_whose_shipped_record_has_moved_since_it_was_generated() {
        let h = seeded();
        shipped_record(&h, &["--control-border", "--surface"]);

        let (outcome, text) = run_kind(&h, ProofKind::TokenPin);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(
            text.contains("has moved since this pin was generated"),
            "{text}"
        );
        assert!(text.contains(SHIPPED), "{text}");
    }

    /// Rows of a pin that is not current are counted and never enforced. Crediting
    /// them would raise `rows_enforced` for rows that establish nothing, which is
    /// the arithmetic half of the [2026] VJS-CC-OPBOX 3 D3 defect.
    #[test]
    fn the_rows_of_a_pin_that_is_not_current_are_counted_and_never_enforced() {
        let h = seeded();
        shipped_record(&h, &["--control-border", "--surface"]);

        let (outcome, text) = run_kind(&h, ProofKind::TokenPin);
        assert_eq!(outcome.rows_considered, 1, "{text}");
        assert_eq!(outcome.rows_enforced, 0, "{text}");
        assert!(text.contains(SKIP_NOT_CURRENT), "{text}");
    }

    /// Derive, do not trust. The pin's `generated_at` is moved forward to a time
    /// after the shipped record was edited, which is exactly what a generator
    /// that was never re-run would look like if anyone believed the stamp.
    #[test]
    fn a_freshly_stamped_pin_over_a_moved_record_is_still_reported_as_not_current() {
        let h = seeded();
        shipped_record(&h, &["--control-border", "--surface"]);
        let path = std::fs::read_dir(h.store().pins_dir())
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let text = std::fs::read_to_string(&path).unwrap();
        let restamped = text.replace("2026-07-25T10:00:00Z", "2099-01-01T00:00:00Z");
        assert_ne!(text, restamped, "the fixture must actually move the stamp");
        std::fs::write(&path, &restamped).unwrap();

        let (outcome, printed) = run_kind(&h, ProofKind::TokenPin);
        assert_eq!(
            outcome.exit_code, EXIT_VIOLATION,
            "a stamp says when somebody claims a thing was done, not that it was: {printed}"
        );
    }

    /// Every pin in the directory is a subject. A run that stopped at the first
    /// pin it could not rely on would report less than it measured, and the
    /// reader could not tell which.
    #[test]
    fn a_pin_that_cannot_be_relied_on_does_not_silence_the_next_one() {
        let h = Harness::new();
        shipped_record(&h, &["--a"]);
        pin_with(
            &h,
            "app/nowhere.css",
            vec![PinRow::compare("a", "x", "x")],
            Some(Digest::of_text("what the generator saw")),
        );
        pin_over_shipped(&h, vec![PinRow::compare("a", "x", "y")]);

        let (outcome, text) = run_kind(&h, ProofKind::TokenPin);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert_eq!(outcome.rows_considered, 2, "{text}");
        assert_eq!(
            outcome.rows_enforced, 1,
            "the second pin is current, so its row is still checked: {text}"
        );
        assert!(text.contains("app/nowhere.css"), "{text}");
        assert!(text.contains("agrees: false"), "{text}");
    }

    /// R1 is about disagreement; I1 is about a row the pin took out of scope on
    /// purpose. The second is counted, named and never fatal.
    #[test]
    fn a_row_the_pin_declined_to_enforce_is_counted_named_and_never_a_violation() {
        let h = Harness::new();
        shipped_record(&h, &["--control-border"]);
        pin_over_shipped(
            &h,
            vec![
                PinRow::compare("control-border", "a", "a"),
                PinRow::not_enforced("elevation", "the decided target does not name this token"),
            ],
        );

        let (outcome, text) = run_kind(&h, ProofKind::TokenPin);
        assert_eq!(outcome.exit_code, EXIT_PASSED, "{text}");
        assert_eq!(outcome.rows_considered, 2, "{text}");
        assert_eq!(outcome.rows_enforced, 1, "{text}");
        assert!(text.contains(SKIP_DECLINED), "{text}");

        let record = h.last_proof(ProofKind::TokenPin);
        let declined = record
            .violations
            .iter()
            .find(|v| v.severity == Severity::Informational)
            .unwrap_or_else(|| panic!("no declined-row finding: {:?}", record.violations));
        assert!(
            declined.actual.contains("does not name this token"),
            "the reason the pin gave has to reach the record, or the carve-out is a number \
             nobody can act on: {declined:?}"
        );
        assert!(
            declined.location.contains("row 2: elevation"),
            "{declined:?}"
        );
    }

    /// A row carrying both a reason and `agrees: true` is a contradiction only a
    /// hand-written pin produces, and the two cannot both be relied on: a row the
    /// pin declined to enforce did not establish the agreement it claims. It is
    /// still a skip, because an unenforced row is an unenforced row whatever it
    /// says about itself.
    #[test]
    fn a_declined_row_that_also_claims_agreement_is_reported_as_the_contradiction_it_is() {
        let h = Harness::new();
        shipped_record(&h, &["--a"]);
        pin_over_shipped(
            &h,
            vec![PinRow {
                name: "elevation".into(),
                agrees: true,
                not_enforced_because: Some("the decided target does not name it".into()),
            }],
        );

        let (outcome, text) = run_kind(&h, ProofKind::TokenPin);
        assert_eq!(outcome.rows_enforced, 0, "{text}");
        let record = h.last_proof(ProofKind::TokenPin);
        assert!(
            record
                .violations
                .iter()
                .any(|v| v.actual.contains("the two cannot both be relied on")),
            "{:?}",
            record.violations
        );
    }

    /// R3. The pin is intact and its rows are all agreements; there is simply no
    /// record on the local side to check them against.
    #[test]
    fn token_pin_fails_on_a_pin_naming_a_shipped_record_that_is_not_there() {
        let h = Harness::new();
        pin_with(
            &h,
            "app/globals.css",
            vec![PinRow::compare("control-border", "a", "a")],
            Some(Digest::of_text("what the generator saw")),
        );

        let (outcome, text) = run_kind(&h, ProofKind::TokenPin);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert_eq!(outcome.rows_enforced, 0, "{text}");
        assert!(text.contains("is not a file in this repository"), "{text}");
        assert!(text.contains(SKIP_SOURCE_UNREADABLE), "{text}");
    }

    #[test]
    fn a_pin_naming_a_record_outside_the_repository_is_refused_rather_than_read() {
        let h = Harness::new();
        shipped_record(&h, &["--control-border"]);
        pin_with(
            &h,
            "../elsewhere/globals.css",
            vec![PinRow::compare("control-border", "a", "a")],
            None,
        );

        let (outcome, text) = run_kind(&h, ProofKind::TokenPin);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(
            text.contains("climbs out of the repository"),
            "pinning against bytes nobody reviews is the storing form wearing a path: {text}"
        );
    }

    /// R4. `Pin::defects` already knows what a self-consistent pin looks like, so
    /// this gate asks it rather than forming a second opinion about it.
    #[test]
    fn token_pin_fails_on_a_pin_whose_counts_disagree_with_the_rows_it_holds() {
        let h = seeded();
        let path = std::fs::read_dir(h.store().pins_dir())
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let text = std::fs::read_to_string(&path)
            .unwrap()
            .replace("rows_enforced: 1", "rows_enforced: 99");
        std::fs::write(&path, &text).unwrap();

        let (outcome, printed) = run_kind(&h, ProofKind::TokenPin);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{printed}");
        assert!(printed.contains("rows_enforced says 99"), "{printed}");
    }

    #[test]
    fn token_pin_fails_on_a_pin_that_does_not_fail_closed() {
        let h = seeded();
        let path = std::fs::read_dir(h.store().pins_dir())
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let text = std::fs::read_to_string(&path)
            .unwrap()
            .replace("fails_closed: true", "fails_closed: false");
        std::fs::write(&path, &text).unwrap();

        let (outcome, printed) = run_kind(&h, ProofKind::TokenPin);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{printed}");
        assert!(printed.contains("third opinion"), "{printed}");
    }

    /// An absent pin is VACUOUS, not a precondition failure, and the distinction
    /// is the one VDS S-7(2)(4) exists to draw.
    ///
    /// A precondition failure means the check could not run. A vacuity means it
    /// ran, walked its subject and found nothing in an enforceable state, and
    /// that is what an empty pins directory is: the walk happened. Every sibling
    /// settles the same shape the same way, `states` over an empty register and
    /// `retirement_drain` over a project with nothing deprecated among them.
    ///
    /// Getting it wrong cost the whole run rather than this one kind. A
    /// precondition aggregates to exit 2, `--allow-vacuous` does not relax a
    /// precondition, and every project without a pin went red on a kind it may
    /// lawfully have nothing to say about.
    #[test]
    fn an_absent_pin_is_vacuous_and_never_a_pass() {
        let h = Harness::new();
        shipped_record(&h, &["--control-border"]);

        let (outcome, text) = run_kind(&h, ProofKind::TokenPin);
        assert_eq!(outcome.status, ProofStatus::Vacuous, "{text}");
        assert_eq!(outcome.exit_code, EXIT_VACUOUS, "{text}");
        assert_eq!(outcome.rows_enforced, 0);

        let record = h.last_proof(ProofKind::TokenPin);
        let vacuity = record
            .notes
            .iter()
            .find(|n| n.starts_with("[vacuous]"))
            .unwrap_or_else(|| panic!("no note saying why: {:?}", record.notes));
        assert!(
            vacuity.contains("VDS S-7(2)(1)"),
            "the note has to say why the pin is generated out of band, or a reader will try to \
             make this gate fetch it: {vacuity}"
        );
        assert!(vacuity.contains("not evidence"), "{vacuity}");
    }

    #[test]
    fn a_pin_with_zero_rows_is_vacuous_and_never_passed() {
        let h = Harness::new();
        shipped_record(&h, &["--control-border"]);
        pin_over_shipped(&h, vec![]);

        let (outcome, text) = run_kind(&h, ProofKind::TokenPin);
        assert_eq!(outcome.status, ProofStatus::Vacuous, "{text}");
        assert_eq!(outcome.exit_code, EXIT_VACUOUS, "{text}");
        assert!(!text.contains("PASS:"), "no PASS beside a vacuity: {text}");
    }

    #[test]
    fn an_unparseable_pin_is_a_precondition_failure_and_not_a_finding() {
        let h = seeded();
        h.write(".vds/pins/PIN-20260725-110000.yaml", "rows: [\n");
        let error = h.run_kind_err(ProofKind::TokenPin);
        assert!(error.to_string().contains("PIN-20260725-110000"), "{error}");
    }

    // -- coverage -------------------------------------------------------------

    /// "A pin with three rows over a hundred custom properties is not evidence
    /// about the other ninety-seven, and the record must say so with a number."
    #[test]
    fn the_record_says_with_a_number_how_much_of_the_shipped_record_the_pin_covers() {
        let h = Harness::new();
        shipped_record(&h, &["--a", "--b", "--c", "--d"]);
        pin_over_shipped(&h, vec![PinRow::compare("a", "x", "x")]);

        let (outcome, text) = run_kind(&h, ProofKind::TokenPin);
        assert_eq!(
            outcome.exit_code, EXIT_PASSED,
            "a pin scoped to one subject is lawful, so partial coverage warns and does not \
             fail: {text}"
        );
        assert!(text.contains("WARNINGS"), "{text}");

        let record = h.last_proof(ProofKind::TokenPin);
        assert!(
            record.notes.iter().any(
                |note| note.contains("declares 4 distinct custom properties")
                    && note.contains("about none of the other 3")
            ),
            "{:?}",
            record.notes
        );
        assert!(
            record
                .violations
                .iter()
                .any(|v| v.severity == Severity::Warning
                    && v.actual.contains("4 declared, 1 enforced")),
            "{:?}",
            record.violations
        );
    }

    #[test]
    fn a_pin_reaching_every_declared_property_raises_no_coverage_warning() {
        let h = Harness::new();
        shipped_record(&h, &["--a"]);
        pin_over_shipped(&h, vec![PinRow::compare("a", "x", "x")]);

        let (outcome, text) = run_kind(&h, ProofKind::TokenPin);
        assert_eq!(outcome.exit_code, EXIT_PASSED, "{text}");
        assert!(
            h.last_proof(ProofKind::TokenPin).violations.is_empty(),
            "a gate that warns when nothing is wrong is a gate somebody switches off"
        );
    }

    /// The denominator is a name-only scan, and the two things most likely to
    /// corrupt it are a `var()` reference and a selector that reads like a
    /// declaration. Neither is a declaration and neither may be counted.
    #[test]
    fn the_denominator_counts_declarations_and_not_references_or_selectors() {
        assert_eq!(declared_custom_properties(":root { --a: 1; }"), 1);
        assert_eq!(
            declared_custom_properties(":root { --a: var(--b); }"),
            1,
            "a var() reference is a use and not a declaration"
        );
        assert_eq!(
            declared_custom_properties(".card--body:hover { color: red; }"),
            0,
            "a class name that ends in two dashes is not a custom property"
        );
        assert_eq!(
            declared_custom_properties(":root{--a:1;--b:2}"),
            2,
            "a minified sheet declares the same properties as a formatted one"
        );
        assert_eq!(
            declared_custom_properties(":root { --a: 1 }\n.dark { --a: 2 }"),
            1,
            "one property redeclared in two scopes is one property to cover"
        );
        assert_eq!(
            declared_custom_properties("--a\n  : 1;"),
            1,
            "a name and a colon on two lines are still one declaration"
        );
    }

    // -- redaction ------------------------------------------------------------

    /// The defect this whole file sits downstream of, in the failing direction.
    ///
    /// A pin row named with a colour is already a broken pin, and
    /// `no_stored_values` would fail on the pin file. What must not happen is
    /// this gate copying that name into `.vds/proofs/`, because a proof record is
    /// never deleted and the copy would keep that gate red forever on a file this
    /// one wrote.
    #[test]
    fn a_row_name_that_could_itself_be_a_realisation_is_withheld_from_the_captured_record() {
        let h = Harness::new();
        shipped_record(&h, &["--control-border"]);
        pin_over_shipped(&h, vec![PinRow::compare("#ebebeb", "a", "b")]);

        let (outcome, _) = run_kind(&h, ProofKind::TokenPin);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION);

        let record = h.last_proof(ProofKind::TokenPin);
        let rendered = format!("{record:?}");
        assert!(
            !rendered.contains("ebebeb"),
            "the finding copied a realisation out of the pin into a record that is never \
             deleted: {rendered}"
        );
        assert!(
            rendered.contains("<row 1>"),
            "the reader still has to be sent to the exact row: {rendered}"
        );
    }

    #[test]
    fn a_declined_reason_that_could_itself_be_a_realisation_is_withheld_too() {
        let h = Harness::new();
        shipped_record(&h, &["--control-border"]);
        pin_over_shipped(
            &h,
            vec![PinRow::not_enforced(
                "elevation",
                "the target still says 12px here",
            )],
        );

        run_kind(&h, ProofKind::TokenPin);
        let rendered = format!("{:?}", h.last_proof(ProofKind::TokenPin));
        assert!(!rendered.contains("12px"), "{rendered}");
        assert!(rendered.contains("withheld"), "{rendered}");
    }

    /// The screen has to withhold what would break `no_stored_values` and keep
    /// what would not. Both directions are the test: a screen that withheld
    /// everything would pass the paragraph above and make every finding useless.
    #[test]
    fn the_screen_withholds_a_realisation_and_keeps_an_ordinary_token_name() {
        for safe in [
            "control-border",
            "--surface-1",
            "--text-2xl",
            "--space-4",
            "the decided target does not name this token",
            "--font-brand",
            "--z-index-modal",
        ] {
            assert_eq!(repeatable(safe), Some(safe), "{safe:?} was withheld");
        }
        for unsafe_text in [
            "#ebebeb",
            "#fff",
            "--radius-8px",
            "--duration-160ms",
            "ease-in-out",
            "sans-serif",
            "rgba(1,2,3,0.5)",
            "cubic-bezier(0.4, 0, 0.2, 1)",
            // The hexadecimal encoding of a six-digit colour literal, which
            // `no_stored_values` R8 decodes and re-tests.
            "23656265626562",
            // A sha256 digest, which R10 sweeps the candidate space against.
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        ] {
            assert_eq!(
                repeatable(unsafe_text),
                None,
                "{unsafe_text:?} would be copied into a record that is never deleted"
            );
        }
    }

    /// The end-to-end statement of the rule, run through the gate that enforces
    /// it. Every finding, note and skip key this proof can produce goes into one
    /// captured record, and `no_stored_values` reads that record and passes.
    #[test]
    fn nothing_this_proof_captures_makes_no_stored_values_fail() {
        let h = Harness::new();
        shipped_record(&h, &["--a", "--b"]);
        pin_over_shipped(
            &h,
            vec![
                PinRow::compare("control-border", "a", "b"),
                PinRow::not_enforced("elevation", "the decided target does not name it"),
            ],
        );

        let (token, text) = run_kind(&h, ProofKind::TokenPin);
        assert_eq!(token.exit_code, EXIT_VIOLATION, "{text}");

        let (guard, text) = run_kind(&h, ProofKind::NoStoredValues);
        assert_eq!(
            guard.exit_code, EXIT_PASSED,
            "this proof wrote a realisation into the tree it is judged in, and there is no \
             lawful way back: a record is never deleted:\n{text}"
        );
        assert!(
            guard.rows_enforced >= 3,
            "the guard has to have actually READ the config, the pin and the captured token_pin \
             record; a pass over a tree it did not open would make this test say nothing: {text}"
        );
    }

    // -- determinism ----------------------------------------------------------

    /// VDS S-7(2)(1). Regenerating a pin restamps `generated_at`, and a proof
    /// that digested the pin FILE would move its evidence digest every time the
    /// generator ran over an unchanged pair of records.
    #[test]
    fn a_restamped_pin_over_an_unchanged_verdict_does_not_move_the_evidence_digest() {
        let h = seeded();
        let (first, _) = run_kind(&h, ProofKind::TokenPin);

        let path = std::fs::read_dir(h.store().pins_dir())
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let text = std::fs::read_to_string(&path).unwrap();
        let restamped = text.replace(
            "generated_at: 2026-07-25T10:00:00Z",
            "generated_at: 2026-08-01T09:00:00Z",
        );
        assert_ne!(text, restamped, "the fixture must actually move the stamp");
        std::fs::write(&path, &restamped).unwrap();

        let (second, printed) = run_kind(&h, ProofKind::TokenPin);
        assert_eq!(second.status, ProofStatus::Passed, "{printed}");

        let store = h.store();
        assert_eq!(
            store
                .read_proof(&first.record_id.unwrap())
                .unwrap()
                .value
                .digest,
            store
                .read_proof(&second.record_id.unwrap())
                .unwrap()
                .value
                .digest,
            "a digest that moves on unchanged input makes every warrant look spent"
        );
    }

    /// The other direction: the shipped record is the subject, so an edit to it
    /// must move the evidence digest even though nothing under `.vds/` moved.
    #[test]
    fn an_edited_shipped_record_moves_the_evidence_digest() {
        let h = seeded();
        let (before, _) = run_kind(&h, ProofKind::TokenPin);
        shipped_record(&h, &["--control-border", "--surface"]);
        let (after, _) = run_kind(&h, ProofKind::TokenPin);

        let store = h.store();
        assert_ne!(
            store
                .read_proof(&before.record_id.unwrap())
                .unwrap()
                .value
                .digest,
            store
                .read_proof(&after.record_id.unwrap())
                .unwrap()
                .value
                .digest
        );
    }

    #[test]
    fn the_run_records_what_it_derives_and_what_it_cannot_reach() {
        let h = seeded();
        run_kind(&h, ProofKind::TokenPin);
        let record = h.last_proof(ProofKind::TokenPin);
        for marker in [
            "[derived]",
            "[reach]",
            "[redaction]",
            "[integrity]",
            "[scope]",
            "[coverage]",
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
                .any(|note| note.contains("never that the pin still describes the DECIDED one")),
            "the Figma half is the whole limitation of this gate, and it has to be on the \
             record rather than in a comment: {:?}",
            record.notes
        );
        let integrity = record
            .notes
            .iter()
            .find(|note| note.starts_with("[integrity]"))
            .unwrap_or_else(|| panic!("no integrity note: {:?}", record.notes));
        assert!(
            integrity.contains("REFUSED, not merely detected"),
            "{integrity}"
        );
        assert!(
            integrity.contains("binds the pin to itself, not to the two records"),
            "the note must not overclaim: R5 establishes that the pin was not edited, and \
             nothing about whether the generator compared the records correctly: {integrity}"
        );
    }

    /// The failing-direction test for R5, and the reason the rule exists.
    ///
    /// A pin recording a disagreement, edited by hand so it records agreement.
    /// Every other rule passes this pin: the shipped record is current, the
    /// counts add up, it fails closed, and its source resolves. Before R5 the run
    /// came back PASSED, which is the whole of the gate defeated by one boolean.
    #[test]
    fn a_pin_edited_from_disagreement_to_agreement_is_refused_rather_than_believed() {
        let h = Harness::new();
        shipped_record(&h, &["--control-border"]);
        let id = pin_over_shipped(
            &h,
            vec![PinRow {
                name: "control-border".into(),
                agrees: false,
                not_enforced_because: None,
            }],
        );

        // It fails honestly first, which is what makes the edit worth making.
        let (before, text) = run_kind(&h, ProofKind::TokenPin);
        assert_eq!(before.exit_code, EXIT_VIOLATION, "{text}");

        let path = h.store().pins_dir().join(format!("{id}.yaml"));
        let edited = std::fs::read_to_string(&path)
            .unwrap()
            .replace("agrees: false", "agrees: true");
        std::fs::write(&path, edited).unwrap();

        let (after, text) = run_kind(&h, ProofKind::TokenPin);
        assert_eq!(
            after.exit_code, EXIT_VIOLATION,
            "a pin edited from disagreement to agreement passed, so one boolean defeats the \
             gate: {text}"
        );
        assert!(text.contains("R5"), "{text}");
        assert!(text.contains("edited after it was generated"), "{text}");
        assert_eq!(
            after.rows_enforced, 0,
            "an edited pin's rows must be SKIPPED and not enforced: crediting a row that \
             establishes nothing is the arithmetic half of the defect: {text}"
        );
    }
}
