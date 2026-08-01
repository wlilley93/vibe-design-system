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
//!
//! ## The third decision: a limb-1 hit is a report of SHAPE
//!
//! Added by [2026] VJS-FI-VDS 1, which held that limb 1 is a shape test and a
//! shape test cannot tell a design value from a string shaped like one. The
//! ruling arose on two sites in a Court of Appeal judgment filed under
//! `.vds/court/`: a compiler's elapsed time quoted in a judge's account of what
//! he measured, and an issue tracker's ordinal written with the number sign,
//! whose three digits are also three hexadecimal digits.
//!
//! Three things that ruling did NOT do, because each is the tempting one:
//!
//! - it did not exempt `.vds/court/**`, by path or by artefact class. S-2(8) is
//!   directory-scoped and S-3(9) closes the exceptions at two, and the carve-out
//!   would have been named after the room the arguments happen in;
//! - it did not tighten a pattern. The two readings of each string are lexically
//!   identical, so any predicate that admits the collision admits the value:
//!   dropping an all-decimal three-digit run blinds R1 to the shorthand spelling
//!   of black, and dropping a fractional-second duration blinds R4 to every
//!   motion duration written in seconds;
//! - it did not order the judgment edited. A judge at first instance has no
//!   power over a superior court's text.
//!
//! What it created instead is the ADJUDICATED COLLISION: one court, one file,
//! one digest, one line, one column, one limb-1 shape class. See
//! [`ADJUDICATED`]. The disposal is reported as a warning and counted on the
//! face of the record, never suppressed, and it dies the moment the artefact it
//! names moves by one byte.

use std::collections::HashSet;
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
const RULE_DISPOSED: &str = "VDS S-2(8) limb 1, DISPOSED: a shape match a court has measured against the VDS S-2(5) \
     limbs and found to hold no design value ([2026] VJS-FI-VDS 1)";
const RULE_ADJUDICATION_SPENT: &str = "[2026] VJS-FI-VDS 1 A1: an adjudicated collision whose artefact has moved since the court \
     measured it. An artefact whose bytes changed is a fresh artefact and no ruling has seen it";
const RULE_ADJUDICATION_INERT: &str = "[2026] VJS-FI-VDS 1 A2: an adjudicated collision that disposes of nothing. A suppression \
     that suppresses nothing is a suppression waiting for something to suppress";

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

pub const SHAPE_NOTE: &str = "[shape] a limb 1 match is a report that a string in the record is SHAPED like a design \
     value. Whether the record is in the STORING FORM is answered by the four limbs of VDS \
     S-2(5), which no pattern evaluates: delete the artefact and see whether a shipped or \
     decided value is lost; change a named record and see whether it serves a second opinion; \
     ask whether a reader can move a shipped pixel by editing it alone; ask whether it carries \
     a value a named record also carries. Limb 1 is a lawful and necessary FLOOR and is not a \
     sufficient test, and this gate must not be read as asserting more than it measured \
     ([2026] VJS-FI-VDS 1 order 6).";

pub const ADJUDICATION_NOTE: &str = "[adjudicated] where a court has applied the VDS S-2(5) limbs to a NAMED SITE and found no \
     design value there, the site is DISPOSED: reported as a warning naming the ruling and the \
     ground, and counted here, never suppressed and never omitted. A disposal binds one file, \
     at one digest, at one line and column, for one limb 1 shape class, and it dies the moment \
     any of those moves. It reaches only the three classes whose spellings collide with \
     ordinary prose, and never a field name, an encoding, an undecodable file or a preimage \
     recovery: a value behind an encoding is concealment and no court disposes of concealment. \
     The distinction from a carve-out is measurable rather than rhetorical - a carve-out's \
     reach grows with the tree, and this one is the integer printed below.";

/// One site a court has adjudicated to be a lexical collision rather than a
/// design value.
///
/// Held in the GATE and deliberately not in data under `.vds/`. That placement is
/// most of the safety: this file is `permit_required` under VDS S-3(8), it is
/// digest-pinned in `.vds/enforcement.lock` under VDS S-8(1) so a new row trips
/// the drift finding and forces a deliberate re-pin with a recorded rationale
/// under VDS S-8(4), and every row is a line in a diff a reviewer reads rather
/// than a file a script can append to.
///
/// A row NEVER carries the matched text, for the same reason a finding does not.
pub(crate) struct Adjudicated {
    /// Project-relative path, as this proof spells a location.
    pub location: &'static str,
    pub line: usize,
    /// 1-based character column, matching [`Hit::column`].
    pub column: usize,
    /// The finding class, which must be one of [`ADJUDICABLE_CLASSES`].
    pub class: &'static str,
    /// The sha256 of the whole file as the court measured it. Any change to any
    /// byte of the artefact spends every adjudication over it.
    pub file_digest: &'static str,
    /// The ruling that disposed of this site, by citation.
    pub ruling: &'static str,
    /// The ground, by class, never quoting the matched text.
    pub because: &'static str,
}

/// The limb 1 classes a court may dispose of.
///
/// Exactly the three whose spellings genuinely collide with ordinary English:
/// the number sign followed by three or six digits, and a numeral carrying a
/// letter that is also a CSS unit. A CSS colour function, an easing curve and a
/// generic font family keyword are words with one meaning, so a collision is not
/// credible and none may be adjudicated. Neither may R7 (a field name), R8 (an
/// encoding), R9 (an undecodable file) or R10 (a preimage recovery).
pub(crate) const ADJUDICABLE_CLASSES: &[&str] =
    &["colour_literal", "length_literal", "duration_literal"];

/// Every adjudicated collision in force. Two.
///
/// Both are in the Court of Appeal's enactment judgment, and both were disposed
/// by [2026] VJS-FI-VDS 1 order 5 on the VDS S-2(5) analysis at section IV of
/// that judgment: delete the artefact and no shipped or decided design value is
/// lost, it serves no second opinion about any token, no reader can move a
/// shipped pixel by editing it, and it carries intent rather than value.
///
/// Neither could be cured any other way. The scan could not be narrowed
/// (VDS S-2(8) is directory-scoped and VDS S-3(9) closes the exceptions at two),
/// the matcher could not be tightened (the two readings of each string are
/// lexically identical), and the judgment could not be edited (a judge at first
/// instance has no power over a superior court's text).
pub(crate) const ADJUDICATED: &[Adjudicated] = &[
    Adjudicated {
        location: ".vds/court/2026-VJS-CA-VDS-1-enactment.md",
        line: 337,
        column: 212,
        class: "duration_literal",
        file_digest: "sha256:78e0b9fce5ae47ce0c182c3618e01ccd6b05a50ef43ef08870fedfc88c658c5b",
        ruling: "[2026] VJS-FI-VDS 1 order 5",
        because: "the elapsed wall-clock time of a workspace type-check, reported by a judge in \
                  his account of what he measured. No design answers a duty with the time a \
                  compiler took (VDS S-2(4)), and a numeral is not automatically a value \
                  (VDS S-2(6)). The gate already holds this same quantity out of R7 under the \
                  excluded field names, on the same ground",
    },
    Adjudicated {
        location: ".vds/court/2026-VJS-CA-VDS-1-enactment.md",
        line: 375,
        column: 56,
        class: "colour_literal",
        file_digest: "sha256:78e0b9fce5ae47ce0c182c3618e01ccd6b05a50ef43ef08870fedfc88c658c5b",
        ruling: "[2026] VJS-FI-VDS 1 order 5",
        because: "an ordinal allocated by a subscriber project's issue tracker, written with the \
                  number sign. It reads as a colour only by an accident of the hexadecimal \
                  alphabet, and the sentence around it settles which reading is meant. \
                  VDS S-2(6) speaks of a literal with one reading; this string has two, and \
                  VDS S-2(5) chooses between them",
    },
];

pub fn run(ctx: &ProofContext, out: &mut dyn Write) -> Result<Outcome> {
    run_with(ctx, out, ADJUDICATED)
}

/// The scan, over a named table of adjudicated collisions.
///
/// Production always passes [`ADJUDICATED`]; the seeded controls pass their own
/// rows. The seam takes DATA and not behaviour, so every line of the mechanism a
/// control exercises is the same line production runs. It exists because the
/// shipped rows name a court record of a hundred thousand-odd bytes that no
/// fixture can reproduce, and a control that could only be written against the
/// real artefact could not seed the failing directions at all. The shipped ROWS
/// are held separately, by `every_shipped_adjudication_is_well_formed` and by
/// the three controls that copy the real artefact in byte for byte.
fn run_with(ctx: &ProofContext, out: &mut dyn Write, acks: &[Adjudicated]) -> Result<Outcome> {
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
    run.note(SHAPE_NOTE);
    run.note(ADJUDICATION_NOTE);

    // Which adjudicated rows this run actually disposed of, and which files it
    // saw. Both are needed at the end: a row that disposed of nothing in a
    // present file is a fatal finding, and a row naming a file this tree does
    // not hold is inapplicable rather than either.
    let mut disposed: HashSet<usize> = HashSet::new();
    let mut seen: HashSet<&str> = HashSet::new();

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

        // Digested from the bytes already in hand rather than re-read from
        // disk: an adjudication is bound to the artefact the court measured, and
        // a second read could witness a different file than the one just
        // scanned.
        let digest = vds_core::Digest::of_bytes(&bytes);
        for row in acks {
            if row.location == location {
                seen.insert(row.location);
            }
        }

        let text = String::from_utf8_lossy(&bytes);
        report(
            &mut run,
            &location,
            &text,
            &patterns,
            digest.as_str(),
            acks,
            &mut disposed,
        );
        if sites.len() < MAX_DIGEST_SITES {
            sites.extend(preimage::harvest(&location, &text));
        }
    }

    report_preimage(&mut run, &sites);
    report_adjudications(&mut run, acks, &disposed, &seen);

    run.finish(&ctx.capture_options()?, out)
}

/// The totals for the adjudicated collisions, on the face of the record.
///
/// The count is the whole difference between an adjudication and a carve-out. A
/// carve-out's reach grows with the tree and is visible only as an absence; this
/// one is an integer a reader can compare against the ruling that authorised it.
/// A row naming a file this tree does not hold is INAPPLICABLE and is counted as
/// such, which is what keeps a subscriber project - which holds none of this
/// repository's court records - from inheriting either a disposal or a spurious
/// failure.
fn report_adjudications(
    run: &mut ProofRun,
    acks: &[Adjudicated],
    disposed: &HashSet<usize>,
    seen: &HashSet<&str>,
) {
    let inapplicable = acks
        .iter()
        .filter(|row| !seen.contains(row.location))
        .count();
    let rulings = {
        let mut named: Vec<&str> = acks.iter().map(|row| row.ruling).collect();
        named.sort_unstable();
        named.dedup();
        named.join(", ")
    };
    run.note(format!(
        "[adjudicated-run] {} adjudicated site(s) in force, {} disposed in this run, {} naming \
         an artefact this tree does not hold. Authorised by: {}. Each disposal is named \
         individually above as a warning, with its file, line, column, class, ruling and \
         ground. A disposal that stopped matching would be a fatal finding and not a silent \
         absence.",
        acks.len(),
        disposed.len(),
        inapplicable,
        if rulings.is_empty() {
            "no ruling, because no site is adjudicated".to_owned()
        } else {
            rulings
        }
    ));
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
/// dropped, and with any site a court has adjudicated reported as disposed.
fn report(
    run: &mut ProofRun,
    location: &str,
    text: &str,
    patterns: &Patterns,
    digest: &str,
    acks: &[Adjudicated],
    disposed: &mut HashSet<usize>,
) {
    let mut all: Vec<(usize, Hit)> = Vec::new();
    for (index, line) in text.lines().enumerate() {
        for hit in literals_in(line, patterns) {
            all.push((index + 1, hit));
        }
        for hit in recovered_in(line, patterns) {
            all.push((index + 1, hit));
        }
    }
    all.sort_by_key(|(line, hit)| (*line, hit.column));

    // Split before capping, so a disposed site never consumes one of the twenty
    // slots a genuine leak needs.
    let mut found: Vec<(usize, Hit)> = Vec::new();
    for (line, hit) in all {
        match adjudication_for(acks, location, digest, line, &hit) {
            Some(index) => {
                disposed.insert(index);
                let row = &acks[index];
                run.warn(Violation::fatal(
                    format!("{location}:{line}:{}", hit.column),
                    RULE_DISPOSED,
                    EXPECTED_REALISATION,
                    format!(
                        "{}, {} characters, DISPOSED by {}: {}. The text is not repeated here; \
                         see the redaction note.",
                        hit.class, hit.span, row.ruling, row.because
                    ),
                ));
            }
            None => found.push((line, hit)),
        }
    }

    audit_adjudications(run, location, digest, acks, disposed);

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

// -- the adjudicated collisions ----------------------------------------------

/// The index of the row that disposes of this hit, if any.
///
/// Every one of the five conditions is necessary. The digest is what makes the
/// disposal die when the artefact moves; the coordinates are what make it a site
/// rather than a file; and the class check against [`ADJUDICABLE_CLASSES`] is
/// defence in depth, so that even a badly drafted row that shipped could not
/// dispose of an encoding, a field name or a preimage recovery.
fn adjudication_for(
    acks: &[Adjudicated],
    location: &str,
    digest: &str,
    line: usize,
    hit: &Hit,
) -> Option<usize> {
    acks.iter().position(|row| {
        row.location == location
            && row.line == line
            && row.column == hit.column
            && row.class == hit.class
            && row.file_digest == digest
            && ADJUDICABLE_CLASSES.contains(&row.class)
    })
}

/// The two ways an adjudication over a PRESENT file fails, both fatal.
///
/// SPENT: the artefact's bytes are not the bytes the court measured. The
/// disposal does not apply, the underlying findings have already come back as
/// fatal above, and the mismatch is reported in its own right so that an author
/// who edits an adjudicated file gets a red light rather than a quiet
/// re-arming.
///
/// INERT: the artefact is present at the pinned digest and the row matched
/// nothing. Unreachable while the shipped rows and the matcher agree, which is
/// exactly why it is here: it is the alarm that fires if a future change to a
/// pattern silently orphans a row, rather than leaving a suppression in the tree
/// with nothing left to suppress.
fn audit_adjudications(
    run: &mut ProofRun,
    location: &str,
    digest: &str,
    acks: &[Adjudicated],
    disposed: &HashSet<usize>,
) {
    for (index, row) in acks.iter().enumerate() {
        if row.location != location {
            continue;
        }
        if row.file_digest != digest {
            run.fail(Violation::fatal(
                format!("{location}:{}:{}", row.line, row.column),
                RULE_ADJUDICATION_SPENT,
                "the artefact holds the bytes the court measured, so the ruling that disposed of \
                 this site is a ruling about THIS text. A disposal is bound to one digest \
                 ([2026] VJS-FI-VDS 1 order 4, bound 2).",
                format!(
                    "an artefact whose digest is not the one {} measured, so every adjudication \
                     over it is SPENT and the findings it disposed of are fatal again. Re-measure \
                     the artefact and return to the court; the digest is not repeated here.",
                    row.ruling
                ),
            ));
        } else if !disposed.contains(&index) {
            run.fail(Violation::fatal(
                format!("{location}:{}:{}", row.line, row.column),
                RULE_ADJUDICATION_INERT,
                "every adjudication in force disposes of a finding this run made. A row that \
                 disposes of nothing is withdrawn through the court that made it, not left in \
                 the gate ([2026] VJS-FI-VDS 1 order 4, bound 3).",
                format!(
                    "a {} adjudication at this position that matched no finding, while the \
                     artefact is present at the digest {} measured. Either a pattern changed and \
                     orphaned the row, or the row was drafted against coordinates that never \
                     held a finding.",
                    row.class, row.ruling
                ),
            ));
        }
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
        // A run that is a SEGMENT of a hyphenated or underscored identifier is
        // an id, not an encoding. The incident: this gate failed the build on
        // `PROOF-20260731-213153` - its own capture clock - because
        // hex("213153") is "!1S" and "1S" reads as a duration. A scan that can
        // fire on its own timestamp is the cry-wolf failure, and generated ids
        // put digit runs after a `-` on every governed line.
        //
        // The first fix skipped ALL-DIGIT tokens, and the seeded R8 test
        // killed it in one run: hex("#ebebeb") is `23656265626562`, pure
        // digits, because `#` and the letters a-e all encode to digit-only
        // pairs. Purity of digits does not separate numbers from encodings;
        // POSITION does. The residue that remains is an author hiding a value
        // directly behind a hyphen ("x-23656265626562"), which is deliberate
        // concealment, and S-8(5) already concedes no lock binds a determined
        // author.
        if found.start() > 0 && matches!(line.as_bytes()[found.start() - 1], b'-' | b'_') {
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

    /// The incident of 2026-07-31: the gate failed the build on its own
    /// capture clock, because hex("213153") is "!1S" and "1S" reads as a
    /// duration. An id segment after a hyphen is a number in identifier
    /// position, never an encoding - and the R8 test below is what keeps this
    /// carve-out honest, because hex("#ebebeb") is pure digits too and MUST
    /// still be recovered when it stands alone.
    #[test]
    fn r8_an_id_segment_that_decodes_to_a_duration_is_not_an_encoding() {
        passes("id: PROOF-20260731-213153\n");
        passes("captured: RETENTION-0002_213153\n");
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

    // -- the adjudicated collisions, and the controls that keep them honest ---
    //
    // [2026] VJS-FI-VDS 1 order 7. A narrowing without a control is a gate
    // switched off, so the ruling names four seeded tests by name and makes
    // orders 4 and 5 conditional on them. They are these, in the order the
    // judgment lists them, plus the static test the ruling required alongside.

    /// The path of the artefact the two shipped rows name, relative to a project
    /// root. Named once, because three controls copy it in and a typo would make
    /// all three test a file the table has never heard of.
    const ADJUDICATED_ARTEFACT: &str = ".vds/court/2026-VJS-CA-VDS-1-enactment.md";

    /// The real adjudicated artefact's bytes, read from this repository.
    ///
    /// The controls copy the REAL file rather than a stand-in, because an
    /// adjudication is bound to a digest and a stand-in has a different one. A
    /// test against a fixture the table does not name would prove that the
    /// mechanism works on something nobody ships.
    fn the_adjudicated_artefact() -> Vec<u8> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(ADJUDICATED_ARTEFACT);
        std::fs::read(&path).unwrap_or_else(|e| {
            panic!(
                "the artefact the shipped adjudications name is not readable at {}: {e}. Two \
                 rows in ADJUDICATED point at it, so if it has moved or been deleted the rows \
                 are stale and must go back to the court that made them.",
                path.display()
            )
        })
    }

    /// Control 1. The positive direction: the two adjudicated sites are DISPOSED,
    /// named individually as warnings citing the ruling, and counted in the run's
    /// note. Nothing is suppressed and nothing is silently absent.
    #[test]
    fn an_adjudicated_collision_is_disposed_and_named_rather_than_suppressed() {
        let h = Harness::new();
        h.write_bytes(ADJUDICATED_ARTEFACT, &the_adjudicated_artefact());

        let (outcome, text) = run_kind(&h, ProofKind::NoStoredValues);
        assert_eq!(
            outcome.exit_code, EXIT_PASSED,
            "the two sites [2026] VJS-FI-VDS 1 order 5 disposed of are still fatal:\n{text}"
        );

        let record = h.last_proof(ProofKind::NoStoredValues);
        let disposals: Vec<_> = record
            .violations
            .iter()
            .filter(|v| v.actual.contains("DISPOSED"))
            .collect();
        assert_eq!(disposals.len(), 2, "{:?}", record.violations);
        for finding in &disposals {
            assert_eq!(
                finding.severity,
                Severity::Warning,
                "a disposal is a reported finding, not an omission: {finding:?}"
            );
            assert!(
                finding.actual.contains("[2026] VJS-FI-VDS 1"),
                "{finding:?}"
            );
            assert!(
                finding.location.starts_with(ADJUDICATED_ARTEFACT),
                "{finding:?}"
            );
            let whole = format!("{finding:?}");
            assert!(
                !whole.contains("ebebeb"),
                "a disposal must not repeat the text it disposed of: {whole}"
            );
        }
        assert!(
            record
                .notes
                .iter()
                .any(|n| n.starts_with("[adjudicated-run]") && n.contains("2 disposed")),
            "the count is the whole difference between an adjudication and a carve-out, and it \
             has to be on the face of the record: {:?}",
            record.notes
        );
    }

    /// Control 2, and the one that decides whether this mechanism is safe.
    ///
    /// An author must not be able to inherit a disposal by editing an
    /// adjudicated file. One appended byte moves the digest, every adjudication
    /// over the artefact is SPENT, the two findings return as FATAL, and the
    /// spending is itself a fatal finding rather than a quiet re-arming.
    #[test]
    fn an_adjudication_dies_when_the_artefact_it_names_moves() {
        let h = Harness::new();
        let mut bytes = the_adjudicated_artefact();
        bytes.push(b'\n');
        h.write_bytes(ADJUDICATED_ARTEFACT, &bytes);

        let (outcome, text) = run_kind(&h, ProofKind::NoStoredValues);
        assert_eq!(
            outcome.exit_code, EXIT_VIOLATION,
            "an adjudication survived an edit to the artefact it names, so a disposal can be \
             inherited by content no court has read:\n{text}"
        );

        let record = h.last_proof(ProofKind::NoStoredValues);
        let fatal: Vec<_> = record.fatal_violations().collect();
        assert!(
            fatal.iter().any(|v| v.rule.contains("A1")),
            "the spending has to be a finding in its own right: {fatal:?}"
        );
        for class in ["duration_literal", "colour_literal"] {
            assert!(
                fatal
                    .iter()
                    .any(|v| v.actual.contains(class) && !v.actual.contains("DISPOSED")),
                "the {class} finding did not come back as fatal once the disposal was spent: \
                 {fatal:?}"
            );
        }
        assert!(
            !record
                .violations
                .iter()
                .any(|v| v.actual.contains("DISPOSED")),
            "a spent adjudication still disposed of something: {:?}",
            record.violations
        );
    }

    /// Control 3. The adjudication disposes of a SITE and never switches a rule
    /// off. With the adjudicated artefact present and unmodified, a genuine
    /// colour literal seeded anywhere else is still fatal.
    #[test]
    fn an_adjudication_does_not_switch_limb_one_off() {
        let h = Harness::new();
        h.write_bytes(ADJUDICATED_ARTEFACT, &the_adjudicated_artefact());
        let id = h.register("Button", Status::Registered);
        h.amend(&id, |record| record.notes = Some("#ebebeb".into()));

        let (outcome, text) = run_kind(&h, ProofKind::NoStoredValues);
        assert_eq!(
            outcome.exit_code, EXIT_VIOLATION,
            "limb 1 stopped finding a real colour while an adjudication was in force:\n{text}"
        );
        assert!(text.contains(".vds/register/CMP-0001.yaml"), "{text}");

        let record = h.last_proof(ProofKind::NoStoredValues);
        assert_eq!(
            record
                .violations
                .iter()
                .filter(|v| v.actual.contains("DISPOSED"))
                .count(),
            2,
            "the two adjudicated sites should still be disposed: {:?}",
            record.violations
        );
    }

    /// Control 4. A row that disposes of nothing is fatal.
    ///
    /// Unreachable while the shipped rows and the matcher agree, which is the
    /// point: it is the alarm that fires if a future change to a pattern
    /// silently orphans a row, leaving a suppression in the gate with nothing
    /// left to suppress. Seeded with a row whose column is one to the left of a
    /// real finding, at the artefact's true digest.
    #[test]
    fn an_adjudication_that_disposes_of_nothing_is_fatal() {
        let h = Harness::new();
        let body = "id: CMP-0001\nnotes: '#ebebeb'\n";
        h.write(".vds/register/CMP-0001.yaml", body);

        // Column 9 is where the finding really is (`notes: '` is eight
        // characters, so the sigil is the ninth). One to the left holds nothing,
        // which is exactly the orphaned row this control seeds.
        let acks = vec![Adjudicated {
            location: ".vds/register/CMP-0001.yaml",
            line: 2,
            column: 8,
            class: "colour_literal",
            file_digest: leaked(vds_core::Digest::of_text(body).as_str()),
            ruling: "[2026] VJS-FI-VDS 1, seeded control",
            because: "a seeded row at coordinates that hold no finding",
        }];

        let (outcome, text) = run_with_acks(&h, &acks);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");

        let record = h.last_proof(ProofKind::NoStoredValues);
        let fatal: Vec<_> = record.fatal_violations().collect();
        assert!(
            fatal.iter().any(|v| v.rule.contains("A2")),
            "an inert adjudication passed unreported: {fatal:?}"
        );
        assert!(
            fatal
                .iter()
                .any(|v| v.actual.contains("colour_literal") && !v.actual.contains("DISPOSED")),
            "the seeded colour must still be fatal: {fatal:?}"
        );
    }

    /// The same seam in the passing direction, so control 4 is not the only
    /// evidence that a hand-built row can dispose of anything at all. Without
    /// this, control 4 would pass equally well against a mechanism that ignored
    /// its table entirely.
    #[test]
    fn a_seeded_adjudication_at_the_right_coordinates_disposes() {
        let h = Harness::new();
        let body = "id: CMP-0001\nnotes: '#ebebeb'\n";
        h.write(".vds/register/CMP-0001.yaml", body);

        let acks = vec![Adjudicated {
            location: ".vds/register/CMP-0001.yaml",
            line: 2,
            column: 9,
            class: "colour_literal",
            file_digest: leaked(vds_core::Digest::of_text(body).as_str()),
            ruling: "[2026] VJS-FI-VDS 1, seeded control",
            because: "a seeded row at the coordinates the finding actually holds",
        }];
        // The column the scan reports for this fixture, proved rather than
        // assumed: `notes: '` is eight characters, so the sigil is the ninth.
        let (outcome, text) = run_with_acks(&h, &acks);
        assert_eq!(
            outcome.exit_code, EXIT_PASSED,
            "a well-aimed seeded row disposed of nothing, so control 4 proves nothing:\n{text}"
        );
    }

    /// Every shipped row, held to the ruling's bounds.
    ///
    /// The three controls above exercise the MECHANISM. This holds the DATA: a
    /// row outside the three permitted classes, or carrying a malformed digest,
    /// or duplicating another's coordinates, is refused here rather than
    /// discovered when it disposes of something it should not have.
    #[test]
    fn every_shipped_adjudication_is_well_formed() {
        let mut coordinates = std::collections::HashSet::new();
        for row in ADJUDICATED {
            assert!(
                ADJUDICABLE_CLASSES.contains(&row.class),
                "{:?} is outside the three classes [2026] VJS-FI-VDS 1 permits a court to \
                 adjudicate. A field name, an encoding, an undecodable file and a preimage \
                 recovery may never be disposed of: a value behind an encoding is concealment.",
                row.class
            );
            assert!(
                vds_core::Digest::parse(row.file_digest).is_ok(),
                "{:?} is not a digest, so the row is bound to nothing",
                row.file_digest
            );
            assert!(
                row.ruling.starts_with('[') && row.ruling.contains(']'),
                "{:?} does not cite a ruling. A disposal with no authority behind it is a \
                 carve-out with a comment.",
                row.ruling
            );
            assert!(
                !row.because.trim().is_empty(),
                "a row must state its ground: {:?}",
                row.location
            );
            assert!(
                coordinates.insert((row.location, row.line, row.column, row.class)),
                "two rows adjudicate the same site: {:?}",
                row.location
            );
        }
    }

    /// A `'static` string built at test time, so a control can pin a digest it
    /// only learns by computing it.
    fn leaked(value: &str) -> &'static str {
        Box::leak(value.to_owned().into_boxed_str())
    }

    /// Run the scan against a harness with a seeded adjudication table.
    ///
    /// Calls the same [`run_with`] production calls, so the mechanism under test
    /// is the mechanism that ships; only the rows differ.
    fn run_with_acks(harness: &Harness, acks: &[Adjudicated]) -> (crate::Outcome, String) {
        let ctx = harness.context();
        let mut out: Vec<u8> = Vec::new();
        let outcome = run_with(&ctx, &mut out, acks).expect("the scan ran");
        (outcome, String::from_utf8(out).expect("utf-8 output"))
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
