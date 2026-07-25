//! The `no_stored_values` proof. The guard on the constraint everything else in
//! VDS is built inside.
//!
//! VDS S-2(1) sets the frame from [2026] VJS-CC-OPBOX 3: an artefact that STORES
//! token values is an authority, one that DERIVES over a named record is a gate,
//! and only the second form is permitted. VDS S-2(2) turns that into a flat
//! prohibition, `.vds/` stores no design value, and VDS S-2(8) makes this proof
//! the only machine check standing behind it. No other kind in the closed
//! registry looks at whether a realisation has leaked into the record, so where
//! this gate is narrow the central constraint is unenforced and nothing says so.
//!
//! The operative distinction is VDS S-2(4). A REQUIREMENT is a duty imposed from
//! outside the design and is lawful here: a contrast floor drawn from WCAG 2.2 SC
//! 1.4.11 is a requirement, and VDS S-2(6) says a numeral is not automatically a
//! value. A REALISATION is the design's own answer to that duty and belongs in
//! the records named at VDS S-2(3), never in `.vds/`.
//!
//! A row is one file under `.vds/**`. Only `.vds/cache/` and `.vds/private/` are
//! outside the scan, because VDS S-3(9) makes those the two ignored directories
//! and a governance record that is gitignored is not a record. Their files are
//! still counted and skipped by name, so the carve-out is a number in the
//! captured record rather than an omission nobody can see.
//!
//! The rules, all fatal except W1:
//!
//!   R1  a colour literal in the hash-sigil form
//!   R2  a CSS colour function
//!   R3  a number carrying a CSS length unit
//!   R4  a number carrying a CSS time unit
//!   R5  a timing function or a named easing keyword
//!   R6  a CSS generic font-family keyword
//!   R7  a field whose NAME names a realisation, whatever its value
//!   R8  a realisation RECOVERED from a reversible encoding, per VDS S-2(8)
//!       limb 2. The limb that matters, because a digest is not a literal and
//!       the literal limb on its own certified the leaking token pin clean.
//!   R9  a file this scan cannot decode as text, and therefore cannot certify
//!   R10 a realisation recovered from a ONE-WAY transform: a digest in the
//!       record whose preimage is a design value. See [`crate::preimage`]. This
//!       is the form the first token pin leaked in, and until it was written
//!       every record this proof captured declared it undischarged.
//!   W1  a symlink, which is counted, not followed, and reported
//!
//! ## Two decisions that shape everything below
//!
//! **A finding never repeats the value.** It names the file, the line, the
//! column, the class and the length in characters, and stops. This is not
//! squeamishness. A captured proof record lands in `.vds/proofs/`, which is
//! inside the very tree this proof scans, so a finding that copied the matched
//! text would put that text under `.vds/**` permanently: the next run would find
//! it, fail on a record this gate wrote itself, and go on failing forever with no
//! lawful way back, because a record is never deleted. Applying the deletion limb
//! of VDS S-2(5) to this proof's own output settles it the same way: a finding
//! that carries the value is itself in the storing form. The reader opens the
//! named file at the named line.
//!
//! It is also why a finding carries no digest OF the matched text. VDS S-2(7)
//! holds that a digest of a low-entropy value is the value, and a colour has a
//! 2^24 domain.
//!
//! **Prose is not exempt.** VDS S-2(8) makes the test recoverability rather than
//! spelling, and a value written into a rationale, a note or a breach report is
//! exactly as recoverable as one written into a field. The alternative, exempting
//! a list of prose-bearing keys, is a hole with a name, and a realisation walks
//! through it by being moved one field to the left. A note that must discuss a
//! realisation names its class instead, which is what every note this proof
//! writes does.

use std::path::{Path, PathBuf};

use regex::Regex;
use std::io::Write;
use vds_core::{ProofKind, Result, VdsError, Violation};
use walkdir::WalkDir;

use crate::ProofContext;
use crate::preimage;
use crate::run::{Outcome, ProofRun, Verdict};

pub const GATE: &str = "crates/vds-proof/src/no_stored_values.rs";

const RULE_COLOUR_LITERAL: &str =
    "VDS S-2(8) limb 1 R1: a colour literal anywhere under the record";
const RULE_COLOUR_FUNCTION: &str =
    "VDS S-2(8) limb 1 R2: a CSS colour function anywhere under the record";
const RULE_LENGTH: &str =
    "VDS S-2(8) limb 1 R3: a number carrying a CSS length unit anywhere under the record";
const RULE_DURATION: &str =
    "VDS S-2(8) limb 1 R4: a number carrying a CSS time unit anywhere under the record";
const RULE_EASING: &str = "VDS S-2(8) limb 1 R5: an easing curve anywhere under the record";
const RULE_FONT: &str = "VDS S-2(8) limb 1 R6: a font family anywhere under the record";
const RULE_FIELD_NAME: &str =
    "VDS S-2(4) R7: a field whose NAME names a realisation rather than a requirement";
const RULE_ENCODED: &str =
    "VDS S-2(8) limb 2 R8: a design value recovered from a reversible encoding under the record";
const RULE_UNREADABLE: &str =
    "VDS S-3(9) R9: a record this scan cannot decode as text, and therefore cannot certify";
const RULE_PREIMAGE: &str = "VDS S-2(8) limb 2 R10: a design value recovered from a ONE-WAY transform, by enumerating \
     the VDS S-2(9) candidate space against a digest held in the record";
const RULE_MANY: &str =
    "VDS S-2(8): one file holds more realisations than this record lists individually";
const RULE_SYMLINK: &str =
    "VDS S-3(9) W1: a symlink under the record, which this scan counts and does not follow";

/// What right would have looked like, for every rule that finds a realisation.
///
/// One sentence, shared, because the answer is the same in every case and a
/// per-rule paraphrase would let the rules drift apart in wording while saying
/// the same thing.
const EXPECTED_REALISATION: &str = "the artefact names the token, the boundary or the requirement BY NAME and leaves the value \
     in the record VDS S-2(3) makes the system of record for it. A requirement may be held here; \
     a realisation may not (VDS S-2(4)), and it is recoverable from the record whether it is \
     written as a literal or as an encoding (VDS S-2(8)).";

const EXPECTED_FIELD_NAME: &str = "a field name that names the DUTY the artefact imposes, not the answer the design gives it. \
     A field whose purpose is to hold a value has no lawful form here and is deleted, not \
     emptied (VDS S-2(4)).";

/// At most this many findings are listed per file. A run over a leaked palette
/// would otherwise capture thousands of near-identical violations into a record
/// nobody reads, which is a different way of hiding them. Nothing is dropped
/// silently: the count of the remainder is reported as its own fatal finding.
const MAX_FINDINGS_PER_FILE: usize = 20;

pub const REDACTION_NOTE: &str = "[redaction] a finding names the file, the line, the column, the class of realisation and \
     its length in characters, and never the matched text. A captured proof record lands under \
     the record this proof scans, so a finding that copied the value would put that value under \
     the record permanently and this gate would then fail forever on a file it wrote itself. \
     Open the named file at the named line to read it. For the same reason a finding carries no \
     digest of the matched text: VDS S-2(7) holds that a digest of a low-entropy value is the \
     value.";

pub const PROSE_NOTE: &str = "[prose] prose is NOT exempt. VDS S-2(8) makes the test recoverability rather than spelling, \
     so a value written into a rationale, a note or a breach report is as recoverable as one \
     written into a field. Exempting a list of prose-bearing keys would be a hole a realisation \
     walks through by moving one field to the left. A note that must discuss a realisation names \
     its class, as every note in this run does.";

pub const IGNORED_NOTE: &str = "[scope] every file under the record is scanned except those in the two directories VDS \
     S-3(9) ignores. Their files are counted and skipped by name, so the carve-out is a number \
     in this record. They are also not recorded as inputs, because a write to a scratch \
     directory must not move the evidence digest a warrant cites.";

/// What the preimage limb now reaches, and what it still does not.
///
/// Stated where a reader of the RECORD will see it rather than in a comment only
/// the author reads. This note said "undischarged" for as long as the limb was,
/// and the sentence it used to carry is kept below, because a note that quietly
/// changed from an admission to a claim would leave a warrant citing the old
/// wording with no way to tell which run it relied on.
pub const PREIMAGE_NOTE: &str = "[preimage] this run discharges VDS S-2(8) limb 1 in full, and limb 2 for BOTH the reversible \
     and the one-way transforms. A hexadecimal or base64 encoding is decoded and re-tested (R8), \
     and every sha256, sha1 and md5 digest harvested from the record is tested against the VDS \
     S-2(9) candidate space by enumerating it (R10), which is the recovery that took 27 seconds \
     against the first token pin. What limb 2 still does NOT reach is a SALTED digest, an HMAC, \
     a digest of a value concatenated with anything, an iterated or key-derived digest, and a \
     digest of a value spelled outside the enumerated space - the largest named omission being \
     the eight-digit hex colour with an alpha channel, whose 2^32 domain would add twenty \
     seconds to every run. A warrant citing this proof may rely on the preimage limb for the \
     plain digest of a plain value and must not describe it as covering the salted forms \
     (VDS S-6(3)).";

pub const PATTERN_FLOOR_NOTE: &str = "[reach] the pattern set is a floor and is named in the gate: colour literals in the \
     hash-sigil form, the CSS colour functions, numbers carrying a CSS length or time unit, \
     timing functions and named easing keywords, the CSS generic font-family keywords, and a \
     closed list of field names that name a realisation. A bare family name under a field name \
     outside that list is NOT reached, because the only way to reach it is a list of the \
     project's font families, and holding that list here would itself be the storing form this \
     proof exists to prevent.";

pub const SELF_SUBJECT_NOTE: &str = "[self-reference] this proof captures its own record into the tree it scans, so the NEXT run \
     reads one more file than this one did. Two consecutive runs over an otherwise unchanged \
     tree therefore cite different evidence digests. That is the subject moving, not the check \
     (VDS S-7(2)(1)).";

pub fn run(ctx: &ProofContext, out: &mut dyn Write) -> Result<Outcome> {
    let project = ctx.project;
    let mut run = ctx.new_run(ProofKind::NoStoredValues, GATE);

    let root = project.vds_dir();
    if !root.is_dir() {
        // A precondition failure, not a pass. A caller told "no realisation
        // found" about a tree that was never opened has been told nothing.
        return Err(VdsError::precondition(format!(
            "{} is not a directory, so there is no record to scan. This proof did not run.",
            project.rel(&root)
        )));
    }

    let patterns = Patterns::new();
    run.note(REDACTION_NOTE);
    run.note(PROSE_NOTE);
    run.note(IGNORED_NOTE);
    run.note(PREIMAGE_NOTE);
    run.note(preimage::SPACE_NOTE);
    run.note(PATTERN_FLOOR_NOTE);
    run.note(SELF_SUBJECT_NOTE);

    // Every digest-shaped run in the record, gathered across the whole walk and
    // swept ONCE at the end. The sweep costs one pass of the candidate space
    // however many digests it is looking for, so sweeping per file would pay that
    // pass once per file for no extra reach.
    let mut sites: Vec<preimage::Site> = Vec::new();

    for path in entries(&root, project)? {
        let location = project.rel(&path);

        if outside_the_record(&root, &path) {
            run.row(Verdict::Skipped("outside_the_record_vds_s3_9"));
            continue;
        }

        if !path.is_file() || path.symlink_metadata().is_ok_and(|m| m.is_symlink()) {
            // Counted, reported and not followed. A link's target is not part of
            // the diff a reviewer reads, so following it would certify bytes
            // nobody reviewed.
            run.row(Verdict::Skipped("not_a_regular_file"));
            run.warn(Violation::fatal(
                location,
                RULE_SYMLINK,
                "a regular file. The record is committed text (VDS S-3(9)), and every byte a \
                 proof certifies is a byte a reviewer can see in a diff.",
                "a symlink or a non-regular file: counted, not scanned, and therefore not \
                 certified free of a realisation",
            ));
            continue;
        }

        // Digested BEFORE the scan and through the same path the rest of VDS
        // uses, so the evidence digest witnesses every file that was in scope
        // whether or not it produced a finding. An unreadable file fails here,
        // loudly, rather than being silently scanned as empty.
        run.input_file(&path)?;
        let bytes = std::fs::read(&path).map_err(|e| VdsError::io(project.rel(&path), e))?;
        run.row(Verdict::Enforced);

        if std::str::from_utf8(&bytes).is_err() {
            run.fail(Violation::fatal(
                location.clone(),
                RULE_UNREADABLE,
                "every file under the record is UTF-8 text a reader can read in a diff \
                 (VDS S-3(9)), so that this scan can certify it holds no realisation.",
                "bytes that are not valid UTF-8. The lossy decoding below was scanned as well, \
                 but finding nothing in it proves nothing about the bytes that did not decode.",
            ));
        }

        let text = String::from_utf8_lossy(&bytes);
        report(&mut run, &location, &text, &patterns);
        if sites.len() < MAX_DIGEST_SITES {
            sites.extend(preimage::harvest(&location, &text));
        }
    }

    report_preimage(&mut run, &sites);

    run.finish(&ctx.capture_options()?, out)
}

/// The most digests one run will sweep for.
///
/// Not a performance guard: the sweep costs one pass whatever the target count
/// is, and a hash-set lookup per candidate per distinct digest is the only thing
/// that grows. It is a memory guard against a pathological tree, and it is loud:
/// exceeding it is a fatal finding, because a run that quietly swept part of the
/// record would report a pass over a tree it did not read.
const MAX_DIGEST_SITES: usize = 200_000;

/// Run the preimage limb and report what it recovered, and what it searched.
///
/// The second half matters as much as the first. A limb that reported only its
/// findings would be indistinguishable from one that enumerated nothing, and this
/// note is the whole reason `PREIMAGE_NOTE` may now say "discharges" where it
/// used to say "does not".
fn report_preimage(run: &mut ProofRun, sites: &[preimage::Site]) {
    if sites.len() >= MAX_DIGEST_SITES {
        run.fail(Violation::fatal(
            ".vds/".to_owned(),
            RULE_PREIMAGE,
            "a record holding few enough digests that every one of them can be tested against \
             the candidate space in one run.",
            format!(
                "at least {MAX_DIGEST_SITES} digest-shaped runs, which is where this scan stops \
                 collecting. The digests beyond that point were NOT tested, so this run does not \
                 certify them and the preimage limb is undischarged for this tree."
            ),
        ));
    }

    let sweep = preimage::sweep(sites);
    run.note(format!(
        "[preimage-run] {} digest-shaped runs harvested, {} distinct, tested against {} \
         enumerated candidates under {}. A digest whose preimage is a design value is a stored \
         design value (VDS S-2(7)).",
        sweep.sites_tested,
        sweep.distinct_digests,
        sweep.candidates_enumerated,
        if sweep.algorithms.is_empty() {
            "no algorithm, because the record holds no digest".to_owned()
        } else {
            sweep
                .algorithms
                .iter()
                .map(|a| a.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        }
    ));

    for found in &sweep.recovered {
        let site = &sites[found.site];
        run.fail(Violation::fatal(
            format!("{}:{}:{}", site.location, site.line, site.column),
            RULE_PREIMAGE,
            EXPECTED_REALISATION,
            format!(
                "{}, recovered from the {} digest at this position by enumerating {} candidates. \
                 A digest of a low-entropy value IS that value (VDS S-2(7)); this one took a \
                 fraction of a second. Neither the value nor a narrower description of it is \
                 repeated here; see the redaction note.",
                found.class,
                site.algo.as_str(),
                sweep.candidates_enumerated
            ),
        ));
    }
}

/// Every non-directory entry under the record, sorted.
///
/// A walk error is a precondition failure and never a smaller result set. The
/// defect that rule prevents is the one `vds-scan` names: a partial walk reports
/// a surface smaller than the one that exists, and a pass over it looks
/// identical to a pass over the whole tree.
fn entries(root: &Path, project: &vds_core::Project) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for entry in WalkDir::new(root).follow_links(false) {
        let entry = entry.map_err(|e| {
            VdsError::precondition(format!(
                "could not walk {}: {e}. A partial walk would certify a tree smaller than the \
                 one that exists, so this proof did not run.",
                project.rel(root)
            ))
        })?;
        if entry.file_type().is_dir() {
            continue;
        }
        out.push(entry.into_path());
    }
    // Sorted so two runs over one tree read the files in one order. WalkDir's
    // order is the filesystem's, and a record whose contents depend on that is
    // not reproducible by a named command (VDS S-2(5)(4)).
    out.sort();
    Ok(out)
}

/// The two directories VDS S-3(9) ignores, and nothing else.
fn outside_the_record(root: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    matches!(
        relative
            .components()
            .next()
            .and_then(|c| c.as_os_str().to_str()),
        Some("cache") | Some("private")
    )
}

/// Emit one file's findings, capped, with the remainder counted rather than
/// dropped.
fn report(run: &mut ProofRun, location: &str, text: &str, patterns: &Patterns) {
    let mut found: Vec<(usize, Hit)> = Vec::new();
    for (index, line) in text.lines().enumerate() {
        for hit in literals_in(line, patterns) {
            found.push((index + 1, hit));
        }
        for hit in recovered_in(line, patterns) {
            found.push((index + 1, hit));
        }
    }
    found.sort_by_key(|(line, hit)| (*line, hit.column));

    let total = found.len();
    for (line, hit) in found.into_iter().take(MAX_FINDINGS_PER_FILE) {
        run.fail(Violation::fatal(
            format!("{location}:{line}:{}", hit.column),
            hit.rule,
            hit.expected,
            format!(
                "{}, {} characters. The text is not repeated here; see the redaction note.",
                hit.class, hit.span
            ),
        ));
    }
    if total > MAX_FINDINGS_PER_FILE {
        run.fail(Violation::fatal(
            location.to_owned(),
            RULE_MANY,
            EXPECTED_REALISATION,
            format!(
                "{total} realisations in this one file. The first {MAX_FINDINGS_PER_FILE} are \
                 listed individually and the remaining {} are counted here, so that a leaked \
                 palette does not become a record nobody reads.",
                total - MAX_FINDINGS_PER_FILE
            ),
        ));
    }
}

// -- the patterns ------------------------------------------------------------

/// One realisation found in one line, described without being quoted.
struct Hit {
    /// A stable machine key for what kind of realisation this is. Owned, because
    /// the encoded limb composes its key from the encoding and the class it
    /// recovered.
    class: String,
    rule: &'static str,
    expected: &'static str,
    /// 1-based character column, so a reader can find it in an editor.
    column: usize,
    /// Length in characters. The class and the length narrow nothing: every
    /// colour in the domain is the same length.
    span: usize,
}

struct Patterns {
    colour_literal: Regex,
    colour_function: Regex,
    number_unit: Regex,
    easing_function: Regex,
    easing_keyword: Regex,
    generic_family: Regex,
    field_name: Regex,
    hex_run: Regex,
    base64_run: Regex,
}

/// CSS length units.
///
/// Every one of them contains a letter outside the hexadecimal alphabet, which
/// is not a coincidence to be relied on quietly: it is why a sha256 digest can
/// never produce a length or duration match, and the test
/// `no_unit_is_spelled_from_the_hexadecimal_alphabet` holds the property in
/// place if the list is ever extended.
/// The easing function names, and the generic font families, as DATA.
///
/// Public within the crate for the same reason `LENGTH_UNITS` is: `contrast`
/// redacts its own findings before capturing them, so that a theme selector
/// carrying a realisation does not land under the tree this proof scans and make
/// it fail forever on a file another gate wrote. That redactor used to keep its
/// own hand-written copy of three of these six shapes and silently missed the
/// other three, so a selector named `.ease-in-out` or `[data-font='monospace']`
/// passed `contrast` and then failed `no_stored_values` fatally.
///
/// Deriving the patterns from these lists means widening the guard widens the
/// redactor, and a test holds every list in step.
pub(crate) const EASING_FUNCTIONS: &[&str] = &["cubic-bezier", "steps", "linear"];

pub(crate) const EASING_KEYWORDS: &[&str] = &["ease-in-out", "ease-out", "ease-in"];

/// Longest first: alternation is leftmost-first, so a shorter keyword nested in
/// a longer one must come second or one token reports twice.
pub(crate) const GENERIC_FAMILIES: &[&str] = &[
    "ui-sans-serif",
    "ui-monospace",
    "ui-serif",
    "ui-rounded",
    "sans-serif",
    "system-ui",
    "monospace",
];

pub(crate) const LENGTH_UNITS: &[&str] = &[
    "px", "rem", "em", "ex", "ch", "vh", "vw", "vmin", "vmax", "pt", "pc", "cm", "mm", "in",
];

const TIME_UNITS: &[&str] = &["s", "ms"];

/// Field names that name a realisation rather than a requirement.
///
/// Closed, normalised (lowercased with `-` and `_` removed) and deliberately
/// short. Each entry has to be a name no governance record legitimately uses,
/// because a false positive here fires on a file that holds nothing at all, and
/// a gate that cries wolf gets disabled. See [`EXCLUDED_FIELD_NAMES`] for the
/// words that failed that test.
const REALISATION_FIELD_NAMES: &[&str] = &[
    "color",
    "colour",
    "backgroundcolor",
    "backgroundcolour",
    "bordercolor",
    "bordercolour",
    "textcolor",
    "textcolour",
    "fill",
    "stroke",
    "font",
    "fontfamily",
    "fontsize",
    "fontweight",
    "typeface",
    "family",
    "letterspacing",
    "lineheight",
    "radius",
    "borderradius",
    "cornerradius",
    "easing",
    "easingcurve",
    "timingfunction",
    "cubicbezier",
    "boxshadow",
    "dropshadow",
    "elevation",
];

/// Words that look like they belong in [`REALISATION_FIELD_NAMES`] and do not.
///
/// Recorded as data rather than as a comment so the omission is reviewable. The
/// decisive one is `duration_ms`: it is the wall-clock duration of a proof run
/// and appears on EVERY captured proof record, so a rule that fired on it would
/// fail on this proof's own output the first time it ran. `size`, `type`, `key`
/// and `name` are prop and contract vocabulary. `width` and `height` describe a
/// Figma frame. The literal rules still catch any of these the moment the value
/// carries a unit, which is the form a realisation almost always takes.
pub const EXCLUDED_FIELD_NAMES: &[&str] = &[
    "duration",
    "durationms",
    "delay",
    "size",
    "width",
    "height",
    "opacity",
    "background",
    "foreground",
    "shadow",
    "spacing",
    "gap",
    "padding",
    "margin",
    "key",
    "type",
    "name",
    "scope",
];

impl Patterns {
    fn new() -> Patterns {
        Patterns {
            // A hex run after a hash sigil. The run length is checked in code
            // rather than in the pattern, because a run of 5, 7 or 11 is not a
            // colour and matching it would be a false positive with a number
            // attached. Requiring the sigil is also what makes a sha256 digest
            // structurally incapable of matching this rule: a digest carries no
            // hash sigil.
            colour_literal: Regex::new(r"#[0-9A-Fa-f]+").expect("a constant pattern"),
            // No whitespace is permitted before the parenthesis, because CSS
            // function notation permits none either, and allowing it would make
            // the prose "the color (see below)" a fatal finding.
            colour_function: Regex::new(
                r"(?i)\b(?:rgba|rgb|hsla|hsl|hwb|oklch|oklab|lch|lab|color-mix|color)\(",
            )
            .expect("a constant pattern"),
            // A number and the letters immediately after it. Which letters count
            // is decided in code against the two unit lists, so a version like
            // `2026-07-25T10` and an identifier like `sha256` fall out rather
            // than needing a special case each.
            number_unit: Regex::new(r"([0-9]+(?:\.[0-9]+)?)([A-Za-z]+)")
                .expect("a constant pattern"),
            easing_function: Regex::new(&format!(r"(?i)\b(?:{})\(", EASING_FUNCTIONS.join("|")))
                .expect("a constant pattern"),
            easing_keyword: Regex::new(&format!(r"(?i)\b(?:{})\b", EASING_KEYWORDS.join("|")))
                .expect("a constant pattern"),
            // Longest first: alternation is leftmost-first, so a shorter keyword
            // nested in a longer one must come second or one token reports twice.
            generic_family: Regex::new(&format!(r"(?i)\b(?:{})\b", GENERIC_FAMILIES.join("|")))
                .expect("a constant pattern"),
            // Anchored to the start of a line, so the word "color" inside a
            // sentence is not read as a field name. YAML sequence dashes and
            // quoting are tolerated; TOML uses `=` where YAML uses `:`.
            field_name: Regex::new(r#"^[\s-]*['"]?([A-Za-z][A-Za-z0-9_.-]*)['"]?[ \t]*[:=]"#)
                .expect("a constant pattern"),
            hex_run: Regex::new(r"[0-9A-Fa-f]{6,}").expect("a constant pattern"),
            base64_run: Regex::new(r"[A-Za-z0-9+/]{8,}={0,2}").expect("a constant pattern"),
        }
    }
}

/// VDS S-2(8) limb 1: realisations written out as themselves.
fn literals_in(line: &str, patterns: &Patterns) -> Vec<Hit> {
    let mut hits = Vec::new();

    for found in patterns.colour_literal.find_iter(line) {
        let digits = found.as_str().len() - 1;
        // 3, 4, 6 and 8 are the four srgb spellings. Anything else is a run of
        // hex that happens to follow a sigil.
        if !matches!(digits, 3 | 4 | 6 | 8) || wordish_at(line, found.end()) {
            continue;
        }
        hits.push(hit(
            "colour_literal",
            RULE_COLOUR_LITERAL,
            EXPECTED_REALISATION,
            line,
            found.start(),
            found.as_str(),
        ));
    }

    for found in patterns.colour_function.find_iter(line) {
        hits.push(hit(
            "colour_function",
            RULE_COLOUR_FUNCTION,
            EXPECTED_REALISATION,
            line,
            found.start(),
            found.as_str(),
        ));
    }

    for captures in patterns.number_unit.captures_iter(line) {
        let whole = captures.get(0).expect("group 0 always matches");
        let unit = captures
            .get(2)
            .expect("the unit group is not optional")
            .as_str();
        // A number glued to a letter is only a length if nothing else is glued
        // to either end. `sha256` fails on the left, `12px3` on the right.
        if wordish_before(line, whole.start()) || wordish_at(line, whole.end()) {
            continue;
        }
        let lowered = unit.to_ascii_lowercase();
        let (class, rule) = if LENGTH_UNITS.contains(&lowered.as_str()) {
            ("length_literal", RULE_LENGTH)
        } else if TIME_UNITS.contains(&lowered.as_str()) {
            ("duration_literal", RULE_DURATION)
        } else {
            continue;
        };
        hits.push(hit(
            class,
            rule,
            EXPECTED_REALISATION,
            line,
            whole.start(),
            whole.as_str(),
        ));
    }

    for found in patterns
        .easing_function
        .find_iter(line)
        .chain(patterns.easing_keyword.find_iter(line))
    {
        hits.push(hit(
            "easing_curve",
            RULE_EASING,
            EXPECTED_REALISATION,
            line,
            found.start(),
            found.as_str(),
        ));
    }

    for found in patterns.generic_family.find_iter(line) {
        hits.push(hit(
            "font_family_keyword",
            RULE_FONT,
            EXPECTED_REALISATION,
            line,
            found.start(),
            found.as_str(),
        ));
    }

    if let Some(captures) = patterns.field_name.captures(line) {
        let name = captures.get(1).expect("the name group is not optional");
        let normalised = normalise_field_name(name.as_str());
        if REALISATION_FIELD_NAMES.contains(&normalised.as_str()) {
            hits.push(hit(
                "realisation_named_field",
                RULE_FIELD_NAME,
                EXPECTED_FIELD_NAME,
                line,
                name.start(),
                name.as_str(),
            ));
        }
    }

    hits
}

/// VDS S-2(8) limb 2, for the reversible transforms only.
///
/// S-2(8) makes the rule recoverability rather than spelling: an artefact is in
/// the storing form if a design value can be reconstructed from `.vds/**`,
/// "whether it is written as a literal, an encoding, a digest, an index into an
/// ordered set, or any other reversible representation". Hexadecimal and base64
/// are two of the transforms VDS S-2(9) names, and they are reversible, so this
/// limb reverses them and re-runs limb 1 over the result.
///
/// The false-positive defence is the printable-ASCII filter. A sha256 digest
/// decodes to 32 bytes that are uniform over 256 values, so the chance that all
/// of them land in the printable range is about one in ten to the fourteenth
/// before the decoded text also has to match a realisation pattern. That is why
/// a digest does not have to be special-cased out of this limb.
fn recovered_in(line: &str, patterns: &Patterns) -> Vec<Hit> {
    let mut hits = Vec::new();

    for found in patterns.hex_run.find_iter(line) {
        let token = found.as_str();
        if !token.len().is_multiple_of(2) || token.len() > MAX_ENCODED_TOKEN {
            continue;
        }
        if let Some(decoded) = decode_hex(token).and_then(printable)
            && let Some(inner) = literals_in(&decoded, patterns).into_iter().next()
        {
            hits.push(encoded_hit(
                "hexadecimal",
                &inner.class,
                line,
                found.start(),
                token,
            ));
        }
    }

    for found in patterns.base64_run.find_iter(line) {
        let token = found.as_str();
        if !token.len().is_multiple_of(4) || token.len() > MAX_ENCODED_TOKEN {
            continue;
        }
        if let Some(decoded) = decode_base64(token).and_then(printable)
            && let Some(inner) = literals_in(&decoded, patterns).into_iter().next()
        {
            hits.push(encoded_hit(
                "base64",
                &inner.class,
                line,
                found.start(),
                token,
            ));
        }
    }

    hits
}

/// A cap on how long a token this limb will try to reverse. A governance record
/// holds no legitimate kilobyte-long hex run, and an unbounded decode is work an
/// author can make arbitrarily large by pasting one.
const MAX_ENCODED_TOKEN: usize = 4096;

fn hit(
    class: &str,
    rule: &'static str,
    expected: &'static str,
    line: &str,
    byte_offset: usize,
    matched: &str,
) -> Hit {
    Hit {
        class: class.to_owned(),
        rule,
        expected,
        column: column_of(line, byte_offset),
        span: matched.chars().count(),
    }
}

/// A recovered realisation names the encoding and the class it recovered, and
/// nothing else. The class of the RECOVERED value is what tells a reader what
/// was hidden; the value itself would put it back under the record.
fn encoded_hit(
    encoding: &'static str,
    recovered: &str,
    line: &str,
    byte_offset: usize,
    token: &str,
) -> Hit {
    Hit {
        class: format!("encoded_realisation:{encoding}:{recovered}"),
        rule: RULE_ENCODED,
        expected: EXPECTED_REALISATION,
        column: column_of(line, byte_offset),
        span: token.chars().count(),
    }
}

// -- small helpers -----------------------------------------------------------

/// A 1-based CHARACTER column. A byte offset would be wrong the moment a line
/// carries a non-ASCII character, and a column a reader cannot find in an editor
/// is a column that sends them looking in the wrong place.
fn column_of(line: &str, byte_offset: usize) -> usize {
    line[..byte_offset].chars().count() + 1
}

fn wordish(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn wordish_at(line: &str, byte_offset: usize) -> bool {
    line[byte_offset..].chars().next().is_some_and(wordish)
}

fn wordish_before(line: &str, byte_offset: usize) -> bool {
    line[..byte_offset].chars().next_back().is_some_and(wordish)
}

/// Lowercase, with the two separators a field name is spelled with removed, so
/// that `fontFamily`, `font-family` and `font_family` are one entry rather than
/// three.
fn normalise_field_name(raw: &str) -> String {
    raw.chars()
        .filter(|c| *c != '-' && *c != '_')
        .flat_map(char::to_lowercase)
        .collect()
}

fn decode_hex(token: &str) -> Option<Vec<u8>> {
    let bytes = token.as_bytes();
    // An odd-length run cannot be a hex encoding of anything. Refused here as
    // well as at the call site, because a decoder that panics on a shape its
    // caller happens to filter out is a decoder waiting for a second caller.
    if !bytes.len().is_multiple_of(2) {
        return None;
    }
    let mut out = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks(2) {
        let high = (pair[0] as char).to_digit(16)?;
        let low = (pair[1] as char).to_digit(16)?;
        out.push((high * 16 + low) as u8);
    }
    Some(out)
}

fn decode_base64(token: &str) -> Option<Vec<u8>> {
    fn sextet(c: u8) -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a') as u32 + 26),
            b'0'..=b'9' => Some((c - b'0') as u32 + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let bytes = token.as_bytes();
    let padding = bytes.iter().rev().take_while(|b| **b == b'=').count();
    if padding > 2 {
        return None;
    }
    let body = &bytes[..bytes.len() - padding];
    let mut out = Vec::with_capacity(body.len() / 4 * 3);
    for quad in body.chunks(4) {
        // A trailing group of one sextet encodes no whole byte and is not
        // producible by any encoder, so the token is not base64 at all.
        if quad.len() < 2 {
            return None;
        }
        let mut packed = 0u32;
        for (index, byte) in quad.iter().enumerate() {
            packed |= sextet(*byte)? << (18 - 6 * index);
        }
        // A short final group already carries its own truncation: three sextets
        // encode two bytes and two encode one. Subtracting the padding again
        // here would drop bytes the encoder never wrote.
        for index in 0..quad.len() - 1 {
            out.push(((packed >> (16 - 8 * index)) & 0xff) as u8);
        }
    }
    Some(out)
}

/// Decoded bytes, but only where every one of them is printable ASCII.
///
/// This is the whole false-positive defence of the encoded limb. A design
/// realisation is human-readable text by construction, and random bytes are not.
fn printable(bytes: Vec<u8>) -> Option<String> {
    if bytes.is_empty() || !bytes.iter().all(|b| (0x20..=0x7e).contains(b)) {
        return None;
    }
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{Harness, run_kind};
    use vds_core::{
        EXIT_PASSED, EXIT_PRECONDITION, EXIT_VACUOUS, EXIT_VIOLATION, ProofKind, ProofStatus,
        Severity, Status,
    };

    /// A tree of requirements only: a contrast floor drawn from WCAG, a
    /// timestamp, a digest, identifiers, routes and a version. Everything VDS
    /// S-2(6) calls a numeral that is not a value.
    #[test]
    fn a_record_of_requirements_only_passes_over_real_rows() {
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        h.register("Button", Status::Registered);
        h.ledger();
        let (outcome, text) = run_kind(&h, ProofKind::NoStoredValues);
        assert_eq!(outcome.exit_code, EXIT_PASSED, "{text}");
        assert_eq!(outcome.status, ProofStatus::Passed);
        assert!(
            outcome.rows_enforced >= 3,
            "the config, the register record and the ledger are all in scope: {text}"
        );
    }

    /// The failing-direction test VDS S-7(2)(2) requires, and the one
    /// `.vds/enforcement.lock` names. It seeds a real colour literal into a real
    /// register record, written through the store rather than by hand, and
    /// asserts the non-zero exit.
    #[test]
    fn no_stored_values_fails_on_a_hex_colour_in_a_register_record() {
        let h = Harness::new();
        let id = h.register("Button", Status::Registered);
        h.amend(&id, |record| record.notes = Some("#ebebeb".into()));

        let (outcome, text) = run_kind(&h, ProofKind::NoStoredValues);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert_eq!(outcome.status, ProofStatus::Failed);
        assert!(text.contains("colour_literal"), "{text}");
        assert!(text.contains(".vds/register/CMP-0001.yaml"), "{text}");
    }

    /// The failing-direction test for R10, and the one that decides whether the
    /// preimage limb is really closed.
    ///
    /// It stores the colour in the form VDS S-2(7) as drafted REQUIRED: not the
    /// literal, which the test above already catches, but the sha256 of it, under
    /// a field name that gives nothing away. Every other rule in this proof passes
    /// this record. A 64-character hexadecimal string carries no hash sigil, no
    /// unit, no easing keyword and no font family, and `value_digest` is not in
    /// the closed list of realisation field names. Before R10 existed, this exact
    /// record was certified clean, which is how the leak got into the
    /// specification in the first place.
    #[test]
    fn no_stored_values_fails_on_a_colour_stored_as_a_digest_of_itself() {
        use sha2::{Digest as _, Sha256};

        let h = Harness::new();
        let id = h.register("Button", Status::Registered);
        let digest = hex::encode(Sha256::digest(b"#ebebeb"));
        h.amend(&id, |record| {
            record.notes = Some(format!("value_digest: {digest}"))
        });

        let (outcome, text) = run_kind(&h, ProofKind::NoStoredValues);
        assert_eq!(
            outcome.exit_code, EXIT_VIOLATION,
            "a colour stored as its own digest passed the scan, which is the exact form the \
             first token pin leaked in: {text}"
        );
        assert_eq!(outcome.status, ProofStatus::Failed);
        assert!(text.contains("R10"), "{text}");
        assert!(text.contains("a hex colour"), "{text}");
        assert!(text.contains(".vds/register/CMP-0001.yaml"), "{text}");
    }

    /// The other half of R10, and the half that keeps it switched on.
    ///
    /// Every proof record VDS captures is full of sha256 digests: an
    /// `inputs_digest`, an `evidence_digest`, a per-input digest for every file
    /// read, the designpack digest. If any of those were reported as a recovered
    /// design value, the proof would fail on its own output the first time it ran
    /// and the rule would last a day.
    #[test]
    fn the_digests_vds_writes_itself_are_not_reported_as_recovered_values() {
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        h.register("Button", Status::Registered);
        h.ledger();

        // Run once to capture a record, then again so the second run scans the
        // first run's digests.
        let (first, _) = run_kind(&h, ProofKind::NoStoredValues);
        assert_eq!(first.exit_code, EXIT_PASSED);
        let (second, text) = run_kind(&h, ProofKind::NoStoredValues);
        assert_eq!(
            second.exit_code, EXIT_PASSED,
            "the preimage limb fired on a digest this proof wrote itself: {text}"
        );
        assert!(
            text.contains("digest-shaped runs harvested"),
            "the run must report what it searched, or a limb that searched nothing looks \
             identical to one that found nothing: {text}"
        );
    }

    /// VDS S-7(2)(4). Nothing scannable in scope is recorded as vacuous and
    /// never as passed, even though every row was accounted for.
    ///
    /// Reaching this state takes a symlinked anchor, because `.vds/config.toml`
    /// is normally a real file and is normally scanned, which is why
    /// `the_scan_cannot_be_vacuous_while_the_anchor_is_a_real_file` exists
    /// beside this test.
    #[test]
    #[cfg(unix)]
    fn the_scan_is_vacuous_when_nothing_under_the_record_is_scannable() {
        let h = Harness::new();
        let root = h.root();
        let real = root.join("config-real.toml");
        std::fs::rename(root.join(".vds/config.toml"), &real).unwrap();
        std::os::unix::fs::symlink("../config-real.toml", root.join(".vds/config.toml")).unwrap();
        h.reload();

        let (outcome, text) = run_kind(&h, ProofKind::NoStoredValues);
        assert_eq!(outcome.status, ProofStatus::Vacuous, "{text}");
        assert_eq!(outcome.exit_code, EXIT_VACUOUS, "{text}");
        assert!(!text.contains("PASS:"), "no PASS beside a vacuity: {text}");
        assert!(text.contains("not_a_regular_file"), "{text}");
    }

    #[test]
    fn the_scan_cannot_be_vacuous_while_the_anchor_is_a_real_file() {
        let h = Harness::new();
        let (outcome, text) = run_kind(&h, ProofKind::NoStoredValues);
        assert_eq!(
            outcome.rows_enforced, 1,
            "an empty project still has its config anchor in scope: {text}"
        );
        assert_eq!(outcome.status, ProofStatus::Passed);
    }

    // -- one test per rule ----------------------------------------------------

    /// Run the proof over one hand-written file under the record.
    fn scan_with(contents: &str) -> (crate::Outcome, String) {
        let h = Harness::new();
        h.write(".vds/register/CMP-0001.yaml", contents);
        run_kind(&h, ProofKind::NoStoredValues)
    }

    fn fails_with(contents: &str, class: &str) {
        let (outcome, text) = scan_with(contents);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(
            text.contains(class),
            "expected a {class} finding in:\n{text}"
        );
    }

    fn passes(contents: &str) {
        let (outcome, text) = scan_with(contents);
        assert_eq!(
            outcome.exit_code, EXIT_PASSED,
            "this is a false positive, and a gate that cries wolf gets disabled:\n{text}"
        );
    }

    #[test]
    fn r1_a_colour_literal_is_fatal_in_every_srgb_spelling() {
        for spelling in ["#fff", "#ffff", "#ebebeb", "#ebebebff", "#EBEBEB"] {
            fails_with(
                &format!("id: CMP-0001\nnotes: '{spelling}'\n"),
                "colour_literal",
            );
        }
    }

    #[test]
    fn r2_a_colour_function_is_fatal() {
        for spelling in [
            "rgb(1,2,3)",
            "rgba(1,2,3,0.5)",
            "hsl(1,2,3)",
            "oklch(1 2 3)",
        ] {
            fails_with(&format!("notes: '{spelling}'\n"), "colour_function");
        }
    }

    #[test]
    fn r3_a_length_carrying_a_unit_is_fatal() {
        for spelling in ["12px", "1.5rem", "2em", "100vh", "8pt"] {
            fails_with(&format!("notes: '{spelling}'\n"), "length_literal");
        }
    }

    #[test]
    fn r4_a_duration_carrying_a_time_unit_is_fatal() {
        for spelling in ["160ms", "0.2s"] {
            fails_with(&format!("notes: '{spelling}'\n"), "duration_literal");
        }
    }

    #[test]
    fn r5_an_easing_curve_is_fatal() {
        for spelling in ["cubic-bezier(0.4, 0, 0.2, 1)", "steps(4)", "ease-in-out"] {
            fails_with(&format!("notes: '{spelling}'\n"), "easing_curve");
        }
    }

    #[test]
    fn r6_a_generic_font_family_keyword_is_fatal() {
        for spelling in ["sans-serif", "monospace", "ui-monospace"] {
            fails_with(&format!("notes: '{spelling}'\n"), "font_family_keyword");
        }
    }

    /// VDS S-2(4): a field that exists to hold a realisation is in the storing
    /// form whatever it currently holds, so the finding is on the NAME.
    #[test]
    fn r7_a_field_whose_name_names_a_realisation_is_fatal_whatever_its_value() {
        for line in [
            "color: surface-1",
            "fontFamily: brand",
            "border-radius: token-a",
        ] {
            fails_with(
                &format!("id: CMP-0001\n{line}\n"),
                "realisation_named_field",
            );
        }
    }

    #[test]
    fn r8_a_colour_hidden_in_a_hexadecimal_encoding_is_recovered() {
        // The hexadecimal encoding of a six-digit colour literal.
        fails_with(
            "notes: '23656265626562'\n",
            "encoded_realisation:hexadecimal",
        );
    }

    #[test]
    fn r8_a_colour_hidden_in_a_base64_encoding_is_recovered() {
        // The base64 encoding of the same literal.
        fails_with("notes: 'I2ViZWJlYg=='\n", "encoded_realisation:base64");
    }

    #[test]
    fn r9_a_file_that_is_not_utf8_text_is_reported_rather_than_scanned_as_clean() {
        let h = Harness::new();
        std::fs::write(
            h.root().join(".vds/register/CMP-0001.yaml"),
            [0xff, 0xfe, 0x00],
        )
        .unwrap();
        let (outcome, text) = run_kind(&h, ProofKind::NoStoredValues);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("not valid UTF-8"), "{text}");
    }

    /// W1. A symlink is counted and reported and does not fail the gate, because
    /// its presence is a defect of the record and not a realisation.
    #[test]
    #[cfg(unix)]
    fn w1_a_symlink_is_counted_reported_and_not_followed() {
        let h = Harness::new();
        std::fs::write(h.root().join("elsewhere.yaml"), "notes: '#ebebeb'\n").unwrap();
        std::os::unix::fs::symlink(
            "../elsewhere.yaml",
            h.root().join(".vds/register/CMP-0001.yaml"),
        )
        .unwrap();

        let (outcome, text) = run_kind(&h, ProofKind::NoStoredValues);
        assert_eq!(
            outcome.exit_code, EXIT_PASSED,
            "the link is a finding about the record, not a realisation: {text}"
        );
        assert!(text.contains("WARNINGS"), "{text}");
        assert!(text.contains("not_a_regular_file"), "{text}");

        let record = h.last_proof(ProofKind::NoStoredValues);
        assert_eq!(record.violations.len(), 1);
        assert_eq!(record.violations[0].severity, Severity::Warning);
    }

    // -- the false positives that would kill this proof -----------------------

    #[test]
    fn a_digest_is_not_a_colour_even_though_it_is_a_run_of_hex() {
        passes(
            "inputs_digest: sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855\n",
        );
    }

    #[test]
    fn a_timestamp_an_identifier_and_a_citation_hold_no_realisation() {
        passes(
            "measuredAt: 2026-07-25T10:00:00Z\n\
             id: CMP-0001\n\
             retirementProofId: PROOF-20260725-100000\n\
             basis: '[2026] VJS-CC-OPBOX 3'\n\
             version: v1\n\
             schemaVersion: 1\n\
             route: app/dash/page.tsx\n\
             line: 12\n",
        );
    }

    /// VDS S-2(6): `minRatio: 3.0` is a duty drawn from WCAG 2.2 SC 1.4.11 and is
    /// lawful. If this ever fails, the proof has started rejecting requirements,
    /// which is the opposite of its job.
    #[test]
    fn a_contrast_floor_is_a_requirement_and_passes() {
        passes(
            "contrastFloors:\n\
             - boundary: control-border\n\
             \x20 against: surface\n\
             \x20 minRatio: 3.0\n\
             \x20 basis: WCAG 2.2 SC 1.4.11\n\
             \x20 scope: control_boundary\n",
        );
    }

    /// The captured output of another proof is the most common file under the
    /// record, so a false positive on one would fire on every project.
    #[test]
    fn a_genuine_captured_proof_record_from_another_kind_is_clean() {
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        h.register("Button", Status::Registered);
        h.ledger();
        let (composition, text) = run_kind(&h, ProofKind::Composition);
        assert_eq!(composition.status, ProofStatus::Passed, "{text}");

        let (outcome, text) = run_kind(&h, ProofKind::NoStoredValues);
        assert_eq!(outcome.exit_code, EXIT_PASSED, "{text}");
    }

    /// The fixed point. This proof writes its record into the tree it scans, so
    /// a rule that fired on its own output would be unfixable: a record is never
    /// deleted, and the gate would fail forever on a file it wrote itself.
    #[test]
    fn the_proofs_own_captured_record_does_not_fail_the_next_run() {
        let h = Harness::new();
        h.register("Button", Status::Registered);

        let (first, text) = run_kind(&h, ProofKind::NoStoredValues);
        assert_eq!(first.exit_code, EXIT_PASSED, "{text}");
        let (second, text) = run_kind(&h, ProofKind::NoStoredValues);
        assert_eq!(
            second.exit_code, EXIT_PASSED,
            "the second run scanned the first run's record and its own notes:\n{text}"
        );
        assert!(second.rows_enforced > first.rows_enforced);
    }

    /// The same fixed point in the failing direction, which is the one that
    /// matters. A finding that copied the matched text would land under the
    /// record and this run would never come back.
    #[test]
    fn a_captured_failure_record_does_not_carry_the_value_it_reported() {
        let h = Harness::new();
        let id = h.register("Button", Status::Registered);
        h.amend(&id, |record| record.notes = Some("#ebebeb".into()));
        let (failed, text) = run_kind(&h, ProofKind::NoStoredValues);
        assert_eq!(failed.exit_code, EXIT_VIOLATION, "{text}");

        // Remove the seeded value. Only the captured failure record remains.
        h.amend(&id, |record| record.notes = None);
        let (after, text) = run_kind(&h, ProofKind::NoStoredValues);
        assert_eq!(
            after.exit_code, EXIT_PASSED,
            "the failure record it wrote is still under the record it scans:\n{text}"
        );
    }

    #[test]
    fn a_finding_names_the_place_and_never_the_value() {
        let h = Harness::new();
        h.write(
            ".vds/register/CMP-0001.yaml",
            "id: CMP-0001\nnotes: '#ebebeb'\n",
        );
        run_kind(&h, ProofKind::NoStoredValues);

        let record = h.last_proof(ProofKind::NoStoredValues);
        let finding = record
            .violations
            .iter()
            .find(|v| v.actual.contains("colour_literal"))
            .expect("a colour finding");
        assert!(finding.location.contains(":2:"), "{}", finding.location);
        let whole = format!("{finding:?}");
        assert!(
            !whole.contains("ebebeb"),
            "the finding repeated the value it found: {whole}"
        );
        assert!(
            !whole.contains("sha256:"),
            "a digest of a low-entropy value is the value (VDS S-2(7)): {whole}"
        );
    }

    #[test]
    fn the_two_ignored_directories_are_skipped_by_name_and_counted() {
        let h = Harness::new();
        h.write(".vds/cache/scratch.yaml", "notes: '#ebebeb'\n");
        h.write(".vds/private/local.yaml", "notes: '12px'\n");

        let (outcome, text) = run_kind(&h, ProofKind::NoStoredValues);
        assert_eq!(outcome.exit_code, EXIT_PASSED, "{text}");
        assert!(text.contains("outside_the_record_vds_s3_9"), "{text}");

        let record = h.last_proof(ProofKind::NoStoredValues);
        assert_eq!(
            record
                .rows_skipped_reasons
                .get("outside_the_record_vds_s3_9"),
            Some(&2),
            "the carve-out has to be a number in the record, not an omission"
        );
    }

    /// VDS S-7(2): a precondition failure is exit 2 and means the proof did not
    /// run. An unreadable file is not a file with nothing in it.
    #[test]
    #[cfg(unix)]
    fn an_unreadable_record_is_a_precondition_failure_and_not_a_pass() {
        use std::os::unix::fs::PermissionsExt;

        let h = Harness::new();
        let path = h.write(".vds/register/CMP-0001.yaml", "id: CMP-0001\n");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
        if std::fs::read(&path).is_ok() {
            // Running as root, where the mode is advisory. The assertion below
            // would prove nothing, so it is skipped rather than weakened.
            return;
        }

        let error = h.run_kind_err(ProofKind::NoStoredValues);
        assert_eq!(error.exit_code(), EXIT_PRECONDITION);
        assert!(error.to_string().contains("CMP-0001"), "{error}");
    }

    #[test]
    fn many_findings_in_one_file_are_capped_and_the_remainder_is_counted() {
        let seeded = 25;
        let body: String = (0..seeded).map(|_| "notes: '#ebebeb'\n").collect();
        let h = Harness::new();
        h.write(".vds/register/CMP-0001.yaml", &body);
        let (outcome, text) = run_kind(&h, ProofKind::NoStoredValues);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");

        let record = h.last_proof(ProofKind::NoStoredValues);
        assert_eq!(record.violations.len(), MAX_FINDINGS_PER_FILE + 1);
        assert!(
            record
                .violations
                .iter()
                .any(|v| v.actual.contains(&format!("{seeded} realisations"))),
            "the remainder has to be counted, not dropped: {:?}",
            record.violations
        );
    }

    /// VDS S-2(8) limb 2 is the limb that decides whether this proof is honest.
    /// Both halves of it are now discharged, and the record has to say so in a
    /// way that still names what is out of reach, because a note that changed
    /// from an admission into an unqualified claim would be the overclaim about
    /// the enforcement surface that VDS S-8(5) forbids.
    #[test]
    fn the_run_records_what_the_limb_reaches_and_what_it_does_not() {
        let h = Harness::new();
        run_kind(&h, ProofKind::NoStoredValues);
        let record = h.last_proof(ProofKind::NoStoredValues);

        let preimage = record
            .notes
            .iter()
            .find(|n| n.starts_with("[preimage]"))
            .unwrap_or_else(|| panic!("no preimage note: {:?}", record.notes));
        assert!(
            preimage.contains("BOTH the reversible and the one-way"),
            "{preimage}"
        );
        for out_of_reach in ["SALTED", "HMAC", "eight-digit hex colour"] {
            assert!(
                preimage.contains(out_of_reach),
                "the note stopped naming {out_of_reach} as out of reach, so a reader would take \
                 the limb as covering more than it does: {preimage}"
            );
        }

        assert!(
            record.notes.iter().any(|n| n.starts_with("[preimage-run]")),
            "the run must record what it SEARCHED and not only what it found, or a limb that \
             enumerated nothing is indistinguishable from one that found nothing: {:?}",
            record.notes
        );
        assert!(
            record.notes.iter().any(|n| n.starts_with("[space]")),
            "the candidate space has to be described in the record, or a reader cannot tell \
             what a pass covered without reading the gate: {:?}",
            record.notes
        );
        assert!(
            record
                .notes
                .iter()
                .any(|n| n.contains("prose is NOT exempt")),
            "the carve-out that was NOT taken is as much a note as one that was: {:?}",
            record.notes
        );
    }

    // -- properties the pattern set rests on ----------------------------------

    /// The reason a sha256 digest cannot produce a length or duration finding is
    /// not a special case, it is arithmetic: no unit is spelled entirely from
    /// the hexadecimal alphabet. Extending the unit lists with one that is would
    /// re-open that false positive silently, so the property is held here.
    #[test]
    fn no_unit_is_spelled_from_the_hexadecimal_alphabet() {
        for unit in LENGTH_UNITS.iter().chain(TIME_UNITS) {
            assert!(
                unit.chars().any(|c| !c.is_ascii_hexdigit()),
                "{unit:?} is spelled from hex characters, so every sha256 digest is now a \
                 candidate for a false positive"
            );
        }
    }

    /// The excluded names are excluded on purpose and must stay excluded. The
    /// decisive one is the wall-clock duration on every captured proof record.
    #[test]
    fn no_excluded_field_name_is_also_a_realisation_field_name() {
        for excluded in EXCLUDED_FIELD_NAMES {
            assert!(
                !REALISATION_FIELD_NAMES.contains(excluded),
                "{excluded:?} is in both lists; if it fires, this proof fails on its own record"
            );
        }
        assert!(EXCLUDED_FIELD_NAMES.contains(&"durationms"));
    }

    #[test]
    fn the_encodings_round_trip_the_way_the_recovery_limb_assumes() {
        assert_eq!(
            decode_hex("23656265626562").and_then(printable).as_deref(),
            Some("#ebebeb")
        );
        assert_eq!(
            decode_base64("I2ViZWJlYg==").and_then(printable).as_deref(),
            Some("#ebebeb")
        );
        assert_eq!(
            decode_base64("YWJjZA==").and_then(printable).as_deref(),
            Some("abcd")
        );
        assert!(
            decode_hex("2365626562656").is_none(),
            "an odd-length run is not hex bytes"
        );
    }
}
