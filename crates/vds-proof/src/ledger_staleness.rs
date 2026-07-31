//! The `ledger_staleness` proof.
//!
//! VDS S-7(5): "each generated ledger is current with its source". VDS S-4(2)
//! states that duty in two halves, and the second half is the one everybody
//! forgets: "Each ledger must have a staleness test that fails when its source
//! changed and the generator was not re-run. A ledger with no staleness test
//! decays, and the evidence for that is in this project already."
//!
//! Two fatal rules, one row per ledger:
//!
//!   R1  a ledger this build can test is STALE. Staleness is this proof's
//!       FINDING and never its excuse. Every other gate loads the ledger through
//!       `vds_scan::load_fresh` and is turned away at the door with exit 2,
//!       which is right for a consumer and useless for the one gate whose whole
//!       subject is the staleness. This gate reads the ledger without that guard
//!       and then runs the guard as its check, so a stale ledger exits 1 here and
//!       is reported, rather than exiting 2 and being reported by nobody.
//!   R2  a ledger file with no staleness test in this build AT ALL. R1 only ever
//!       reaches the ledgers somebody already wrote a test for, so a gate that
//!       stopped at R1 would certify a directory of decaying inventories clean by
//!       never looking at them. docs/GOAL.md D8 counts exactly this: "a count of
//!       ledgers with no staleness test. Target: zero."
//!
//! The screens ledger's staleness test is stronger than comparing the source
//! digest the ledger records, and that difference is what makes R1 worth running.
//! `vds_scan::check_fresh` REGENERATES the ledger in memory and compares the
//! content. Comparing sources alone answers "have the screens moved", never "was
//! this file produced from them", so it certifies a hand-edited ledger clean, and
//! a proof reading that ledger can then be flipped from failing to passing by
//! editing the ledger instead of the code (VDS S-2(5)(4)).
//!
//! This proof reads ledger PATHS, ledger DIGESTS and the generator's own verdict.
//! It reads no design value (VDS S-2(2)).

use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};

use vds_core::{Digest, PathRole, Project, ProofKind, Result, VdsError, Violation};
use vds_scan::{GENERATOR_COMMAND, LEDGER_SCHEMA_VERSION, ScreensLedger};

use crate::ProofContext;
use crate::run::{Outcome, ProofRun, Verdict};

pub const GATE: &str = "crates/vds-proof/src/ledger_staleness.rs";

const RULE_STALE: &str =
    "VDS S-7(5) ledger_staleness R1 / S-4(2): each generated ledger is current with its source";
const RULE_NO_STALENESS_TEST: &str = "VDS S-4(2) ledger_staleness R2: every ledger has a staleness test, and a ledger without \
     one decays with nothing to say so";
const RULE_FIGMA_LEDGER: &str = "VDS S-4(2) ledger_staleness R3: the figma ledger is self-consistent, agrees with the file \
     the register names, and is not older than the records that read it";
const RULE_GEOMETRY_READING: &str =
    "VDS S-4(2) ledger_staleness R5: the geometry reading witnesses its own content";
const RULE_FRAME_LEDGER: &str = "VDS S-4(2) ledger_staleness R4: the frame ledger is self-consistent, agrees with the file \
     the screen register names, and is not older than the screen records that read it";
const RULE_CI_LEDGER: &str = "VDS S-4(2) ledger_staleness R6: the CI run ledger's own numbers agree with each other, \
     because a hand-edit flips one field and derive() builds them together";

/// What the figma ledger's staleness test can and cannot establish.
///
/// It has no LOCAL source, which is the whole reason it exists: its source is a
/// Figma file behind a network call. So the three things a local test can settle
/// are settled, and the fourth is named rather than glossed. Three proofs now
/// read this ledger, and a reader of any of them has to know which of the four
/// they are getting.
pub const FIGMA_LEDGER_NOTE: &str = "[figma-ledger] the figma ledger's staleness test settles three things and cannot settle \
     the fourth. It settles that the ledger is SELF-CONSISTENT (its content digest matches its \
     contents, so it was not hand-edited), that it agrees with the decided-target file the \
     register names (two files is two opinions about what is decided), and that it is not \
     OLDER than the register records that read it (a record pointed at a node after the pull \
     is a record the ledger has never seen). It cannot settle whether the decided-target file \
     has changed since the pull, because that is a network read VDS S-7(2)(1) forbids inside a \
     proof. Its `generated_at` is reported so a reader can weigh how old the reading is, and a \
     warrant citing anything downstream of it is bounded by that instant.";

/// Why R1 is worth running rather than being a digest comparison.
pub const REGENERATION_NOTE: &str = "the screens ledger's staleness test REGENERATES the ledger from its sources and compares \
     the content, rather than comparing the source digest the ledger records. A source-digest \
     comparison answers only whether the screens moved, so it certifies a hand-edited ledger \
     clean, and a proof reading that ledger can then be flipped from failing to passing without \
     touching a screen (VDS S-2(5)(4)).";

/// What this run does not reach. Silent narrowing is the defect VDS exists to
/// catch, so the narrowing is written into the record and not left to a reader
/// to infer from the row counts.
/// What the frame ledger's staleness test can and cannot establish.
///
/// The same three limbs as the figma ledger's, and the same fourth it cannot,
/// because it has the same shape of source: a Figma file behind a network call.
/// It is stated separately rather than shared, because the two ledgers are read
/// by different proofs and a reader of `screen_parity` is owed the limit of the
/// ledger IT reads, not a paragraph about a different one.
pub const FRAME_LEDGER_NOTE: &str = "[frame-ledger] the frame ledger's staleness test settles three things and cannot settle the \
     fourth. It settles that the ledger is SELF-CONSISTENT (its content digest matches its \
     contents, so it was not hand-edited), that it agrees with the decided-target file the \
     SCREEN register names, and that it is not OLDER than the screen records that read it. It \
     cannot settle whether the decided-target file has changed since the capture, because that \
     is a network read VDS S-7(2)(1) forbids inside a proof. It also cannot settle whether the \
     capture went DEEP ENOUGH: the ledger records the depth it derived and marks every reading \
     taken at the boundary, and `screen_parity` refuses to enforce those rows, which is the \
     furthest an offline test can go.";

pub const GEOMETRY_LEDGER_NOTE: &str = "[geometry-reading] the geometry reading's staleness test settles two things and cannot \
     settle the third. It settles that the reading is SELF-CONSISTENT (its content digest \
     matches its contents, so it was not hand-edited) and that it is not OLDER than the \
     geometry bounds measured against it. It cannot settle whether the SHIPPED ARTEFACT has \
     changed since the reading was taken, because that needs the generator re-run against a \
     build, and this proof does not build the subject. That last gap is why the reading carries \
     `takenAt` and why `geometry` measures its window from that field rather than from the \
     clock: a reading nobody regenerated reports the shape as it WAS, and the proof reading it \
     should conclude what was true then, not what is true now.";

pub const CI_LEDGER_NOTE: &str = "[ci-ledger] the CI run ledger's source is `gh run list`, a network fact, so no gate can \
     regenerate it offline - the same settlement as the figma (R3) and frame (R4) ledgers. \
     What IS checked: every internal cross-total (histogram vs runs_concluded, successes vs \
     the success bucket, the never_succeeded flag vs the numbers beside it, window ordering), \
     because those are what a hand-edit breaks - an editor flipping `successes` to make D4 \
     read better does not also rebuild the histogram derive() builds with it. What is NOT \
     checked: currency. A stale-but-consistent ledger passes; `generated_at` is the reader's \
     signal, and D4's own text says a ledger is evidence of the runs it recorded, not of runs \
     since.";

pub const REACH_NOTE: &str = "what this run does NOT reach: this build holds one REGENERATING staleness test, the screens \
     ledger's. The figma ledger (R3), the frame ledger (R4) and the CI run ledger (R6) \
     have self-consistency tests instead, because their source is behind a network call and \
     there is no generator here to re-run against it. For every other file in the ledgers directory it establishes only that \
     no staleness test exists, never that the file is current. It reaches nothing outside the \
     configured ledgers directory. And where the declared screens ledger is absent the run is a \
     precondition failure at exit 2, so the other files in that directory go unexamined rather \
     than passing.";

pub fn run(ctx: &ProofContext, out: &mut dyn Write) -> Result<Outcome> {
    let project = ctx.project;
    let mut run = ctx.new_run(ProofKind::LedgerStaleness, GATE);
    run.input_file(&project.config_path)?;

    let ledgers_dir = project.path(PathRole::Ledgers);
    let screens_path = project.screens_ledger_path();
    let screens_rel = project.rel(&screens_path);

    // Recursively, because a ledger one directory down is still a ledger. A walk
    // that read only the top level would report a clean directory while an
    // untested inventory rotted inside it, which is the R2 defect wearing a
    // shallower disguise.
    let found = files_under(project, &ledgers_dir)?;

    // An ABSENT declared ledger is a precondition failure and not a finding:
    // there is no ledger to report the currency of, so the proof did not run.
    // The refusal names what it therefore did not examine, because a reader who
    // is told only "absent" would reasonably assume the rest of the directory was
    // checked.
    if !screens_path.is_file() {
        return Err(VdsError::precondition(format!(
            "{screens_rel} is absent, so there is no ledger to report the currency of and this \
             proof did not run.\n  The declared screens ledger is a generated inventory \
             (VDS S-4(2)); run: {GENERATOR_COMMAND}\n  {} further file(s) under {} were \
             therefore not examined for a staleness test either.",
            found.len(),
            project.rel(&ledgers_dir)
        )));
    }

    let ledger = read_without_its_staleness_test(project, &screens_path)?;
    // The ledger's CONTENT digest, not its file digest: the file carries
    // `generated_at`, which moves on a no-op regeneration and would move this
    // proof's evidence digest with it (VDS S-7(2)(1)).
    run.input_named("<screens ledger content>", ledger.content_digest.clone());

    // The live screens are an input too. Whether the ledger is current is a fact
    // about them, so a screen edit must move this proof's evidence digest even
    // though it moves nothing under `.vds/`. Reading them here also surfaces a
    // genuine read failure (a screen outside the root, an unreadable file) as a
    // precondition BEFORE `check_fresh` runs, so what `check_fresh` reports below
    // is staleness and not an IO fault wearing staleness's name.
    let screens = vds_scan::screen_files(project)?;
    run.input_named(
        "<live screen sources>",
        vds_scan::source_digest(project, &screens)?,
    );

    // The R2 finding depends on WHICH files sit in the ledgers directory, not on
    // what they contain, so the listing is the input. Digesting their bytes would
    // drag every other generator's `generated_at` into this proof's evidence
    // digest and make an unchanged run look changed (VDS S-7(2)(1)).
    let listing: Vec<String> = found.iter().map(|path| project.rel(path)).collect();
    run.input_named("<ledgers directory listing>", Digest::of_value(&listing)?);

    run.note(REGENERATION_NOTE);
    run.note(REACH_NOTE);

    // One row is one ledger. The declared screens ledger is a row whether or not
    // it sits inside the configured ledgers directory, because the duty attaches
    // to the ledger and not to where a config happens to put it.
    let mut ledgers: BTreeSet<String> = BTreeSet::new();
    ledgers.insert(screens_rel.clone());
    for path in &found {
        let rel = project.rel(path);
        if path.extension().and_then(|e| e.to_str()) == Some("yaml") {
            ledgers.insert(rel);
            continue;
        }
        // Counted, named, and not enforced. A file this gate declined to read as
        // a ledger is a carve-out, and a carve-out visible only as a number in a
        // skip count is a carve-out nobody can act on.
        run.row(Verdict::Skipped("not_a_yaml_file_so_not_read_as_a_ledger"));
        run.inform(Violation::fatal(
            rel.clone(),
            RULE_NO_STALENESS_TEST,
            format!(
                "{} holds generated ledgers, each a *.yaml artefact with a staleness test \
                 (VDS S-4(2))",
                project.rel(&ledgers_dir)
            ),
            format!(
                "{rel} is not a *.yaml file, so this proof did not read it as a ledger. If it \
                 is one, it sits outside every staleness test and outside this row count."
            ),
        ));
    }

    let figma_rel = project.rel(&vds_figma::pull::ledger_path(&ctx.store()));
    let frame_rel = project.rel(&vds_figma::frames::ledger_path(project));
    let geometry_rel = project.rel(&vds_core::reading_path(project));
    let ci_rel = project.rel(&project.path(vds_core::PathRole::Ledgers).join("ci.yaml"));

    for rel in &ledgers {
        run.row(Verdict::Enforced);

        // R3. The figma ledger has a staleness test now, and it needs one badly:
        // `states` measures `states.drawn` against it and `reconciliation`
        // resolves limb (c) against it, so a stale one is three proofs reporting
        // the decided-target file as it was at some unknown past moment.
        if rel == &figma_rel {
            run.note(FIGMA_LEDGER_NOTE);
            report_figma_ledger(&mut run, ctx, rel)?;
            continue;
        }

        // R4. The frame ledger, added with the eleventh proof kind. Without
        // this arm it would land in R2's territory and fail as a ledger with no
        // staleness test, which is the correct answer to a question nobody had
        // yet answered and the wrong one now that it has a test.
        if rel == &frame_rel {
            run.note(FRAME_LEDGER_NOTE);
            report_frame_ledger(&mut run, ctx, rel)?;
            continue;
        }

        // R5. The geometry reading, added with the TWELFTH proof kind, and it
        // needs an arm here more urgently than either of the two above: it is
        // the SOLE measurement `geometry` reads, so a reading that went stale or
        // was edited turns a bound being exceeded into a bound being met, with
        // no surface changed. Without this arm it lands in R2's territory and
        // fails as a ledger with no staleness test, which was the right answer
        // until it had one.
        if rel == &geometry_rel {
            run.note(GEOMETRY_LEDGER_NOTE);
            report_geometry_reading(&mut run, ctx, rel)?;
            continue;
        }

        // R6. The CI run ledger, BREACH-0011's remedy. D4 reads it, so an edited
        // copy claiming a success that never happened flips the one criterion
        // whose whole subject is whether the gates actually run. Its source is a
        // network fact, so this is the self-consistency settlement, not a
        // regeneration - the note says exactly what that does and does not prove.
        if rel == &ci_rel {
            run.note(CI_LEDGER_NOTE);
            report_ci_ledger(&mut run, ctx, rel)?;
            continue;
        }

        if rel != &screens_rel {
            run.fail(Violation::fatal(
                rel.clone(),
                RULE_NO_STALENESS_TEST,
                format!(
                    "a staleness test in this build that regenerates {rel} from its source and \
                     fails where the two disagree, as {screens_rel} has (VDS S-4(2))"
                ),
                format!(
                    "{rel} has no staleness test in this build, so nothing would notice it \
                     going out of date with whatever generated it. Either add its generator and \
                     its staleness test to this gate, or remove the file."
                ),
            ));
            continue;
        }

        if let Err(error) = vds_scan::check_fresh(project, &ledger) {
            run.fail(Violation::fatal(
                rel.clone(),
                RULE_STALE,
                format!(
                    "{rel} regenerates byte-identically from the screens its own source_globs \
                     name, so every proof that reads it reads the live surface (regenerate \
                     with: {GENERATOR_COMMAND})"
                ),
                one_line(&error.to_string()),
            ));
        }
    }

    run.finish(&ctx.capture_options()?, out)
}

/// Read the screens ledger WITHOUT running its staleness test.
///
/// `vds_scan::load_fresh` refuses a stale ledger with a precondition error. That
/// is correct for every gate that CONSUMES the ledger, and wrong for the one gate
/// whose subject IS the staleness, which would otherwise exit 2 and report
/// nothing. Splitting the read from the test is what lets the two outcomes differ:
/// a ledger that cannot be read at all is a precondition failure, and a ledger
/// that reads and is out of date is this proof's finding.
fn read_without_its_staleness_test(project: &Project, path: &Path) -> Result<ScreensLedger> {
    let text = std::fs::read_to_string(path).map_err(|e| VdsError::io(path.display(), e))?;
    let raw: serde_yaml::Value = serde_yaml::from_str(&text).map_err(|e| VdsError::Artefact {
        path: project.rel(path),
        message: format!("is not readable YAML, so its currency cannot be reported on: {e}"),
    })?;

    // Refused rather than partially read (VDS S-11(2)). A loader that skips the
    // fields it cannot parse would compare a ledger it only half understood and
    // call the difference staleness.
    let found = raw
        .get("schema_version")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    if found > LEDGER_SCHEMA_VERSION {
        return Err(VdsError::SchemaVersionAhead {
            path: project.rel(path),
            kind: "screens ledger",
            found,
            understood: LEDGER_SCHEMA_VERSION,
        });
    }

    serde_yaml::from_value(raw).map_err(|e| VdsError::Artefact {
        path: project.rel(path),
        message: format!("is not a screens ledger, so its currency cannot be reported on: {e}"),
    })
}

/// Every file under a directory, recursively, in a stable order.
///
/// An absent directory is empty and not an error: a project that has generated no
/// ledger yet has no ledgers, which is a fact and not a fault. A directory that
/// exists and cannot be enumerated IS an error, because an empty result there
/// would be a silent claim that it held nothing.
fn files_under(project: &Project, directory: &Path) -> Result<Vec<PathBuf>> {
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    let mut found = Vec::new();
    for entry in walkdir::WalkDir::new(directory).sort_by_file_name() {
        let entry = entry.map_err(|e| {
            VdsError::precondition(format!(
                "could not enumerate {}: {e}. A ledgers directory this proof cannot read is a \
                 directory whose contents it cannot report on, so this is a precondition \
                 failure rather than an empty result (VDS S-4(2)).",
                project.rel(directory)
            ))
        })?;
        if entry.file_type().is_file() {
            found.push(entry.into_path());
        }
    }
    found.sort();
    Ok(found)
}

/// Fold a multi-line refusal onto one line, keeping its parts separable.
///
/// A violation is one row in a captured record and one line in the printed
/// report, so a multi-line `actual` makes both ragged. Joining on a visible
/// separator rather than a space is what keeps "changed since generation: x"
/// and "Regenerate with: y" from reading as one sentence.
fn one_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<&str>>()
        .join(" | ")
}

/// R3: everything about the figma ledger a local check can settle.
fn report_figma_ledger(run: &mut ProofRun, ctx: &ProofContext, rel: &str) -> Result<()> {
    let store = ctx.store();
    let Some(ledger) = vds_figma::pull::read(&store)? else {
        // Listed in the directory and unreadable as a ledger: R2's territory
        // rather than R3's, and reported there.
        return Ok(());
    };

    // The register's own view of which file is decided. Two files is two
    // opinions, and a ledger pulled from the wrong one answers every question
    // about the wrong document.
    let declared = vds_figma::pull::declared_file_key(&store)?;
    if let Err(why) = vds_figma::ledger::check_fresh(&ledger, declared.as_deref()) {
        run.fail(Violation::fatal(
            rel.to_owned(),
            RULE_FIGMA_LEDGER,
            "a figma ledger whose content digest matches its own contents and whose file key              is the one the register names",
            format!("{why}"),
        ));
        return Ok(());
    }

    // A record pointed at a node AFTER the pull is a record this ledger has
    // never seen, so every proof reading it is reading an inventory of a
    // different register. The same shape as the demand check at VDS S-5(7).
    let mut newer: Vec<String> = Vec::new();
    for located in store.read_register()? {
        if let Some(figma) = &located.value.figma
            && figma.captured_at.as_str() > ledger.generated_at.as_str()
        {
            newer.push(format!("{} at {}", located.value.id, figma.captured_at));
        }
    }
    if !newer.is_empty() {
        run.fail(Violation::fatal(
            rel.to_owned(),
            RULE_FIGMA_LEDGER,
            format!(
                "a figma ledger generated no earlier than every register record that reads it,                  so every record has a row in it. This one was generated at {}",
                ledger.generated_at
            ),
            format!(
                "{} record(s) name a node captured after the pull: {}. The ledger has never                  seen them, so `states` cannot measure their drawn states and `reconciliation`                  cannot resolve their nodes. Regenerate with: vds figma pull",
                newer.len(),
                newer.join(", ")
            ),
        ));
        return Ok(());
    }

    run.note(format!(
        "[figma-ledger-age] this ledger was generated at {}, from file {} version {}. Nothing          offline can establish whether that file has changed since.",
        ledger.generated_at, ledger.file_key, ledger.file_version
    ));
    Ok(())
}

/// R4: everything about the frame ledger a local check can settle.
/// R5: the geometry reading is self-consistent and not older than what reads it.
fn report_geometry_reading(run: &mut ProofRun, ctx: &ProofContext, rel: &str) -> Result<()> {
    let project = ctx.project;
    let Some(reading) = vds_core::read_reading(project)? else {
        // Listed in the directory and unreadable as a reading: R2's territory
        // rather than R5's, and reported there.
        return Ok(());
    };

    if let Some(why) = reading.untrustworthy_because()? {
        run.fail(Violation::fatal(
            rel.to_owned(),
            RULE_GEOMETRY_READING,
            "a geometry reading whose content digest matches its own contents",
            why,
        ));
        return Ok(());
    }

    // A bound whose last entry is NEWER than the reading is a bound the reading
    // has never seen. `geometry` R3 measures the window from the reading's
    // moment, so a bound lowered after the reading was taken would be measured
    // against a moment before it moved, and would read as not having moved.
    // Reported here rather than there, because "your inputs are out of order" is
    // a staleness fact and not a conformance one.
    let mut newer: Vec<String> = Vec::new();
    for located in ctx.store().read_geometry()? {
        if let Some(current) = located.value.current()
            && current.at.as_str() > reading.taken_at.as_str()
        {
            newer.push(format!("{} at {}", located.value.id, current.at));
        }
    }
    if !newer.is_empty() {
        run.fail(Violation::fatal(
            rel.to_owned(),
            RULE_GEOMETRY_READING,
            format!(
                "a geometry reading taken no earlier than every bound measured against it. This \
                 one was taken at {}",
                reading.taken_at
            ),
            format!(
                "{} bound(s) moved after it: {}. `geometry` measures the direction rule from \
                 the READING's moment, so a bound lowered after this reading was taken is \
                 measured against a moment before it moved and reads as though it never did. \
                 Regenerate the reading.",
                newer.len(),
                newer.join(", ")
            ),
        ));
    }
    Ok(())
}

fn report_ci_ledger(run: &mut ProofRun, ctx: &ProofContext, rel: &str) -> Result<()> {
    let project = ctx.project;
    let path = project.path(vds_core::PathRole::Ledgers).join("ci.yaml");
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        // Listed and then unreadable is an IO fault, not staleness; surface it
        // as the violation it is rather than letting the row pass silently.
        Err(error) => {
            run.fail(Violation::fatal(
                rel.to_owned(),
                RULE_CI_LEDGER,
                "a readable CI run ledger",
                format!("could not be read: {error}"),
            ));
            return Ok(());
        }
    };
    // Parse + schema check are the TYPE's (vds_core::types::ci), the same
    // implementation `vds ledger ci` writes through - one copy of the rule, per
    // the two-copies failure this repo's sibling measured the same day this arm
    // was written. A parse failure is a failure here, not a skip: a ledger that
    // stopped parsing is a ledger something edited.
    let ledger = match vds_core::types::ci::CiLedger::parse(&text) {
        Ok(ledger) => ledger,
        Err(why) => {
            run.fail(Violation::fatal(
                rel.to_owned(),
                RULE_CI_LEDGER,
                "a CI run ledger this build's schema reads",
                why,
            ));
            return Ok(());
        }
    };
    for finding in ledger.self_check() {
        run.fail(Violation::fatal(
            rel.to_owned(),
            RULE_CI_LEDGER,
            "a ledger whose summary fields follow from the histogram beside them, as \
             derive() constructs them",
            finding,
        ));
    }
    Ok(())
}

fn report_frame_ledger(run: &mut ProofRun, ctx: &ProofContext, rel: &str) -> Result<()> {
    let project = ctx.project;
    let store = ctx.store();
    let Some(ledger) = vds_figma::frames::read(project)? else {
        // Listed in the directory and unreadable as a frame ledger: R2's
        // territory rather than R4's, and reported there.
        return Ok(());
    };

    let declared = vds_figma::frames::declared_file_key(&store)?;
    if let Err(why) = vds_figma::frames::check_fresh(&ledger, declared.as_deref()) {
        run.fail(Violation::fatal(
            rel.to_owned(),
            RULE_FRAME_LEDGER,
            "a frame ledger whose content digest matches its own contents and whose file key is \
             the one the screen register names",
            format!("{why}"),
        ));
        return Ok(());
    }

    // A screen record pointed at a frame AFTER the capture is a record this
    // ledger has never seen, so `screen_parity` would report R3 (no row) on a
    // frame that exists and was simply captured too early. The same shape as
    // the figma ledger's check above.
    let mut newer: Vec<String> = Vec::new();
    for located in store.read_screens()? {
        if let Some(frame) = &located.value.frame
            && frame.captured_at.as_str() > ledger.generated_at.as_str()
        {
            newer.push(format!("{} at {}", located.value.id, frame.captured_at));
        }
    }
    if !newer.is_empty() {
        run.fail(Violation::fatal(
            rel.to_owned(),
            RULE_FRAME_LEDGER,
            format!(
                "a frame ledger generated no earlier than every screen record that reads it, so \
                 every record has a row in it. This one was generated at {}",
                ledger.generated_at
            ),
            format!(
                "{} screen record(s) name a frame captured after it: {}. The ledger has never \
                 seen them, so screen_parity would report them as frames the capture does not \
                 reach. Regenerate with: {} --from <capture.json>",
                newer.len(),
                newer.join(", "),
                vds_figma::frames::GENERATOR_COMMAND
            ),
        ));
        return Ok(());
    }

    run.note(format!(
        "[frame-ledger-age] this ledger was derived at {} from a capture of file {} at depth \
         {}, in which {} leaf node(s) sat on the boundary. Nothing offline can establish \
         whether that file has changed since, nor whether the depth was enough.",
        ledger.generated_at, ledger.file_key, ledger.capture_depth, ledger.truncated_leaves
    ));
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::testing::{Harness, run_kind};
    use vds_core::{
        EXIT_PASSED, EXIT_PRECONDITION, EXIT_VACUOUS, EXIT_VIOLATION, ProofKind, ProofStatus,
        Severity, Status,
    };

    const LEDGER: &str = ".vds/ledgers/screens.yaml";

    /// One screen, one freshly generated ledger. The state every test starts from.
    fn seeded() -> Harness {
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        h.ledger();
        h
    }

    #[test]
    fn a_freshly_generated_ledger_passes_over_one_enforced_row() {
        let h = seeded();
        let (outcome, text) = run_kind(&h, ProofKind::LedgerStaleness);
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
        assert_eq!(outcome.exit_code, EXIT_PASSED);
        assert_eq!(outcome.rows_enforced, 1, "one ledger is one row: {text}");
    }

    /// The failing-direction test VDS S-7(2)(2) requires, and the one
    /// `.vds/enforcement.lock` names. It edits a screen after the ledger was
    /// generated, leaving the ledger describing a surface that no longer exists,
    /// and asserts the non-zero exit.
    #[test]
    fn ledger_staleness_fails_on_a_stale_screens_ledger() {
        let h = seeded();
        h.screen("dash", &["Button", "Card"]);

        let (outcome, text) = run_kind(&h, ProofKind::LedgerStaleness);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert_eq!(outcome.status, ProofStatus::Failed);
        assert!(text.contains("STALE"), "{text}");
        assert!(
            text.contains(LEDGER),
            "the finding names the ledger: {text}"
        );
        assert!(
            text.contains("app/dash/page.tsx"),
            "and the source that moved: {text}"
        );
    }

    /// Staleness is this proof's finding and not its excuse. Every consumer of
    /// the ledger is refused at the door with exit 2, which says "did not run".
    /// This gate DID run, so it owes exit 1 and a named finding.
    #[test]
    fn a_stale_ledger_is_a_violation_here_and_a_precondition_failure_everywhere_else() {
        let h = seeded();
        h.register("Button", Status::Registered);
        h.ledger();
        h.screen("dash", &["Button", "Card"]);

        let elsewhere = h.run_kind_err(ProofKind::Composition);
        assert_eq!(elsewhere.exit_code(), EXIT_PRECONDITION, "{elsewhere}");
        assert!(elsewhere.to_string().contains("STALE"), "{elsewhere}");

        let (outcome, text) = run_kind(&h, ProofKind::LedgerStaleness);
        assert_eq!(
            outcome.exit_code, EXIT_VIOLATION,
            "a proof that reported staleness as a precondition failure would report it to \
             nobody: {text}"
        );
    }

    /// R1's strength, and the reason [`super::REGENERATION_NOTE`] claims it. The
    /// sources are untouched and `source_digest` is intact, so a source-digest
    /// comparison passes this file. Regeneration does not.
    #[test]
    fn ledger_staleness_fails_on_a_hand_edited_ledger_whose_sources_are_untouched() {
        let h = seeded();
        let mut ledger: vds_scan::ScreensLedger =
            serde_yaml::from_str(&h.read(LEDGER)).expect("the generated ledger parses");
        // Delete every reference, exactly as someone would to silence a
        // composition failure without touching a screen.
        ledger.screens[0].references.clear();
        h.write(LEDGER, &serde_yaml::to_string(&ledger).unwrap());

        let (outcome, text) = run_kind(&h, ProofKind::LedgerStaleness);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("was edited"), "{text}");
    }

    /// The staleness test is against the CONFIGURED surface, not the surface the
    /// ledger remembers. Widening or narrowing `screen_globs` without
    /// regenerating leaves a ledger that describes a different project.
    #[test]
    fn ledger_staleness_fails_on_a_ledger_generated_under_different_screen_globs() {
        let h = seeded();
        let config = h
            .read(".vds/config.toml")
            .replace(r#"["app/**/page.tsx"]"#, r#"["app/**/view.tsx"]"#);
        h.write(".vds/config.toml", &config);
        h.reload();

        let (outcome, text) = run_kind(&h, ProofKind::LedgerStaleness);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("screen_globs changed"), "{text}");
    }

    /// R2, the half of VDS S-4(2) that gets forgotten. The file is untouched and
    /// perfectly consistent with itself, and that is exactly the problem: nothing
    /// in this build would ever notice it going out of date.
    #[test]
    fn ledger_staleness_fails_on_a_ledger_with_no_staleness_test() {
        let h = seeded();
        h.write(".vds/ledgers/tokens.yaml", "schema_version: 1\nrows: []\n");

        let (outcome, text) = run_kind(&h, ProofKind::LedgerStaleness);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert_eq!(outcome.rows_enforced, 2, "one row per ledger: {text}");
        assert!(text.contains(".vds/ledgers/tokens.yaml"), "{text}");
        assert!(text.contains("no staleness test in this build"), "{text}");
    }

    #[test]
    fn an_unattested_ledger_one_directory_down_is_still_found() {
        let h = seeded();
        h.write(".vds/ledgers/archive/tokens.yaml", "rows: []\n");

        let (outcome, text) = run_kind(&h, ProofKind::LedgerStaleness);
        assert_eq!(
            outcome.exit_code, EXIT_VIOLATION,
            "a top-level-only walk reports a clean directory while an untested inventory rots \
             inside it: {text}"
        );
        assert!(text.contains("archive/tokens.yaml"), "{text}");
    }

    /// A carve-out visible only as a number is a carve-out nobody can act on, so
    /// the skipped row is counted AND the file is named in the record.
    #[test]
    fn a_non_yaml_file_in_the_ledgers_directory_is_counted_named_and_not_enforced() {
        let h = seeded();
        h.write(".vds/ledgers/README.md", "notes\n");

        let (outcome, text) = run_kind(&h, ProofKind::LedgerStaleness);
        assert_eq!(outcome.exit_code, EXIT_PASSED, "{text}");
        assert_eq!(outcome.rows_considered, 2, "{text}");
        assert_eq!(outcome.rows_enforced, 1, "{text}");
        assert!(
            text.contains("not_a_yaml_file_so_not_read_as_a_ledger"),
            "{text}"
        );

        let record = h.last_proof(ProofKind::LedgerStaleness);
        assert!(
            record.violations.iter().any(|v| {
                v.severity == Severity::Informational && v.location.contains("README.md")
            }),
            "{:?}",
            record.violations
        );
    }

    /// An absent ledger is nothing to report on rather than nothing to report, so
    /// it is exit 2. The refusal says what it therefore did not examine, because
    /// a reader told only "absent" would assume the rest of the directory was
    /// checked.
    #[test]
    fn an_absent_ledger_is_a_precondition_failure_that_names_what_it_did_not_examine() {
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        h.write(".vds/ledgers/tokens.yaml", "rows: []\n");

        let error = h.run_kind_err(ProofKind::LedgerStaleness);
        assert_eq!(error.exit_code(), EXIT_PRECONDITION, "{error}");
        assert!(
            error.to_string().contains("this proof did not run"),
            "{error}"
        );
        assert!(error.to_string().contains("1 further file(s)"), "{error}");
    }

    #[test]
    fn an_unreadable_ledger_is_a_precondition_failure_and_not_a_staleness_finding() {
        let h = seeded();
        h.write(LEDGER, "screens: [\n");

        let error = h.run_kind_err(ProofKind::LedgerStaleness);
        assert!(
            error.to_string().contains("is not readable YAML"),
            "{error}"
        );
    }

    #[test]
    fn a_ledger_from_the_future_is_refused_rather_than_reported_as_stale() {
        let h = seeded();
        let text = h
            .read(LEDGER)
            .replace("schema_version: 1", "schema_version: 99");
        h.write(LEDGER, &text);

        let error = h.run_kind_err(ProofKind::LedgerStaleness);
        assert!(
            error.to_string().contains("VDS S-11(2)"),
            "a ledger this build only half understands would be compared and the difference \
             called staleness: {error}"
        );
    }

    /// VDS S-7(2)(4), the vacuity limb. This proof cannot come out vacuous while
    /// it runs at all: the declared ledger is always a row, and an absent one is
    /// exit 2 rather than a pass over nothing. The vacuous branch is asserted
    /// unreachable here rather than left untested, because "it never happens" is
    /// the sentence that precedes it happening.
    #[test]
    fn the_run_is_never_vacuous_because_an_absent_ledger_is_exit_two_not_exit_three() {
        let h = Harness::new();
        h.screen("dash", &["Button"]);

        let error = h.run_kind_err(ProofKind::LedgerStaleness);
        assert_eq!(error.exit_code(), EXIT_PRECONDITION, "{error}");
        assert_ne!(
            error.exit_code(),
            EXIT_VACUOUS,
            "nothing to report on is not a run over zero rows: {error}"
        );

        h.ledger();
        let (outcome, text) = run_kind(&h, ProofKind::LedgerStaleness);
        assert_ne!(outcome.status, ProofStatus::Vacuous, "{text}");
        assert_ne!(outcome.exit_code, EXIT_VACUOUS, "{text}");
        assert!(outcome.rows_enforced >= 1, "{text}");
    }

    /// VDS S-7(2)(1). The generator stamps `generated_at`, so a proof digesting
    /// the ledger FILE would move its evidence digest every time the generator ran
    /// over unchanged screens, and every warrant citing it would look spent.
    #[test]
    fn a_restamped_ledger_over_unchanged_screens_does_not_move_the_evidence_digest() {
        let h = seeded();
        let (first, _) = run_kind(&h, ProofKind::LedgerStaleness);

        let original = h.read(LEDGER);
        let restamped: Vec<String> = original
            .lines()
            .map(|line| {
                if line.starts_with("generated_at:") {
                    "generated_at: 2000-01-01T00:00:00Z".to_owned()
                } else {
                    line.to_owned()
                }
            })
            .collect();
        let restamped = format!("{}\n", restamped.join("\n"));
        assert_ne!(
            original, restamped,
            "the fixture must actually move the ledger's bytes, or this test asserts nothing"
        );
        h.write(LEDGER, &restamped);

        let (second, text) = run_kind(&h, ProofKind::LedgerStaleness);
        assert_eq!(
            second.status,
            ProofStatus::Passed,
            "re-stamping when the generator ran is not staleness: {text}"
        );

        let store = h.store();
        let before = store.read_proof(&first.record_id.unwrap()).unwrap().value;
        let after = store.read_proof(&second.record_id.unwrap()).unwrap().value;
        assert_eq!(
            before.digest, after.digest,
            "a digest that moves on unchanged input makes every warrant look spent"
        );
    }

    /// A changed screen must move the evidence digest even though nothing under
    /// `.vds/` moved, which is why the live sources are recorded as an input.
    #[test]
    fn a_changed_screen_moves_the_evidence_digest() {
        let h = seeded();
        let (before, _) = run_kind(&h, ProofKind::LedgerStaleness);
        h.screen("dash", &["Button", "Card"]);
        let (after, _) = run_kind(&h, ProofKind::LedgerStaleness);

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
    fn the_run_records_how_it_tests_and_what_it_cannot_reach() {
        let h = seeded();
        run_kind(&h, ProofKind::LedgerStaleness);
        let record = h.last_proof(ProofKind::LedgerStaleness);
        assert!(
            record.notes.iter().any(|n| n.contains("REGENERATES")),
            "{:?}",
            record.notes
        );
        assert!(
            record.notes.iter().any(|n| n.contains("does NOT reach")),
            "{:?}",
            record.notes
        );
    }

    /// R3, and the reason it matters now rather than when the ledger was
    /// written: `states` measures `states.drawn` against this file and
    /// `reconciliation` resolves limb (c) against it, so a stale one is three
    /// proofs reporting the decided-target file as it was at some unknown past
    /// moment and calling it a measurement.
    #[test]
    fn ledger_staleness_fails_on_a_figma_ledger_a_record_is_newer_than() {
        let h = seeded();
        write_figma_ledger(&h, "2026-07-25T11:00:00Z");

        let (before, text) = run_kind(&h, ProofKind::LedgerStaleness);
        assert_eq!(
            before.exit_code, EXIT_PASSED,
            "a figma ledger with no record newer than it is current: {text}"
        );

        // A record pointed at a node AFTER the pull. The ledger has never seen
        // it, so the proofs downstream are reading an inventory of a different
        // register.
        let id = h.register("Late", Status::Registered);
        h.amend(&id, |record| {
            record.figma = Some(vds_core::FigmaNode {
                file_key: "KEY".into(),
                node_id: "99:99".into(),
                captured_at: vds_core::Timestamp::fixed(2026, 7, 25, 14, 0, 0),
            });
        });

        let (after, text) = run_kind(&h, ProofKind::LedgerStaleness);
        assert_eq!(after.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("R3"), "{text}");
        // The printed finding wraps, so the assertion is on a fragment that
        // survives wrapping. Matching a whole sentence here would fail on a
        // change to the terminal width rather than on a change to the rule.
        assert!(
            text.contains("name a node captured after the pull"),
            "{text}"
        );
        assert!(text.contains("vds figma pull"), "{text}");
    }

    /// A hand-edited figma ledger is refused, which is what S-4(2) means by a
    /// generated inventory.
    #[test]
    fn a_hand_edited_figma_ledger_is_refused() {
        let h = seeded();
        write_figma_ledger(&h, "2026-07-25T11:00:00Z");
        let path = h.root().join(".vds/ledgers/figma.yaml");
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(
            &path,
            text.replace("file_name: decided", "file_name: something else"),
        )
        .unwrap();

        let (outcome, text) = run_kind(&h, ProofKind::LedgerStaleness);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("edited after it was generated"), "{text}");
    }

    /// The note has to name what a local test CANNOT settle, because three
    /// proofs read this ledger and a reader of any of them needs to know.
    #[test]
    fn the_figma_ledger_note_names_the_limb_no_offline_test_can_reach() {
        let h = seeded();
        write_figma_ledger(&h, "2026-07-25T11:00:00Z");
        run_kind(&h, ProofKind::LedgerStaleness);
        let record = h.last_proof(ProofKind::LedgerStaleness);
        let note = record
            .notes
            .iter()
            .find(|note| note.starts_with("[figma-ledger]"))
            .unwrap_or_else(|| panic!("{:?}", record.notes));
        assert!(note.contains("cannot settle the fourth"), "{note}");
        assert!(note.contains("network read"), "{note}");
        assert!(
            record
                .notes
                .iter()
                .any(|n| n.starts_with("[figma-ledger-age]")),
            "the age has to be on the record so a reader can weigh it: {:?}",
            record.notes
        );
    }

    /// R4. Without its own arm the frame ledger would fall into R2 and be
    /// reported as a ledger with no staleness test, which was the right answer
    /// until it had one.
    #[test]
    fn a_frame_ledger_with_a_test_is_not_reported_as_untested() {
        let h = seeded();
        h.frames(&[Harness::frame("1:1", "Screen · /x", &["body"], 2)]);

        let (outcome, text) = run_kind(&h, ProofKind::LedgerStaleness);
        assert_eq!(outcome.exit_code, EXIT_PASSED, "{text}");
        assert_eq!(outcome.rows_enforced, 2, "one row per ledger: {text}");
        assert!(text.contains("[frame-ledger]"), "{text}");
        assert!(text.contains("[frame-ledger-age]"), "{text}");
    }

    /// R6, both directions, driven through the REAL gate over a REAL derive()
    /// output - not the self_check function alone, because a defect between the
    /// gate and the type (a wrong path, a skipped parse) is invisible to a
    /// unit test of the type.
    #[test]
    fn a_consistent_ci_ledger_passes_and_a_hand_edited_one_is_refused_by_name() {
        let h = seeded();
        let raw = r#"[
          {"conclusion":"failure","createdAt":"2026-07-31T07:38:23Z","headSha":"abc","workflowName":"vds-enforce"},
          {"conclusion":"failure","createdAt":"2026-07-25T14:06:34Z","headSha":"def","workflowName":"vds-enforce"}
        ]"#;
        let ledger = vds_core::types::ci::derive(
            ".github/workflows/vds-enforce.yml",
            Some("vds-enforce"),
            "test",
            raw,
            "test",
        )
        .unwrap();
        let path = h.root().join(".vds/ledgers/ci.yaml");
        std::fs::write(&path, serde_yaml::to_string(&ledger).unwrap()).unwrap();

        let (outcome, text) = run_kind(&h, ProofKind::LedgerStaleness);
        assert_eq!(outcome.exit_code, EXIT_PASSED, "{text}");
        assert!(
            text.contains("[ci-ledger]"),
            "the note must say what a pass proves: {text}"
        );

        // SEED: flip the one field an editor wanting D4 green would flip, and
        // assert the seed LANDED before reading the verdict - a seed that
        // silently missed reads exactly like a dead gate.
        let text_before = std::fs::read_to_string(&path).unwrap();
        let seeded_text = text_before.replace("successes: 0", "successes: 1");
        assert_ne!(seeded_text, text_before, "the seed did not land");
        std::fs::write(&path, seeded_text).unwrap();

        let (outcome, text) = run_kind(&h, ProofKind::LedgerStaleness);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("R6"), "{text}");
        assert!(
            text.contains("the summary and the evidence disagree")
                || text.contains("does not follow from the numbers"),
            "the refusal must name the contradiction, not just count it: {text}"
        );
    }

    /// A ledger from a FUTURE schema must fail, not skip: an unreadable file in
    /// the ledgers directory is a file something edited or something newer
    /// wrote, and neither is a pass.
    #[test]
    fn a_ci_ledger_from_an_unknown_schema_is_a_violation_not_a_skip() {
        let h = seeded();
        let path = h.root().join(".vds/ledgers/ci.yaml");
        std::fs::write(
            &path,
            "schema_version: 999\ngenerated_at: 2026-07-31T00:00:00Z\ngenerated_by: t\nsource: t\nworkflows: []\nnotes: []\n",
        )
        .unwrap();

        let (outcome, text) = run_kind(&h, ProofKind::LedgerStaleness);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("schema_version 999"), "{text}");
    }

    #[test]
    fn ledger_staleness_fails_on_a_hand_edited_frame_ledger() {
        let h = seeded();
        h.frames(&[Harness::frame("1:1", "Screen · /x", &["body"], 2)]);
        let path = h.root().join(".vds/ledgers/frames.yaml");
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, text.replace("columns: 2", "columns: 1")).unwrap();

        let (outcome, text) = run_kind(&h, ProofKind::LedgerStaleness);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("R4"), "{text}");
        assert!(text.contains("edited after it was generated"), "{text}");
    }

    /// The same defect R3 catches for the component register: a record pointed
    /// at a node after the capture is a record the ledger has never seen, and
    /// `screen_parity` would blame the capture for a frame that exists.
    #[test]
    fn ledger_staleness_fails_on_a_frame_ledger_a_screen_record_is_newer_than() {
        let h = seeded();
        h.frames(&[Harness::frame("1:1", "Screen · /x", &["body"], 2)]);
        h.screen_record("SCR-0001", "/x", 2, &[], Some("1:1"));
        h.amend_screen("SCR-0001", |record| {
            if let Some(frame) = record.frame.as_mut() {
                // Far enough ahead to beat the ledger's `generated_at`, which is
                // `Timestamp::now()` at the moment the fixture wrote it.
                frame.captured_at = vds_core::Timestamp::fixed(2099, 1, 1, 0, 0, 0);
            }
        });

        let (outcome, text) = run_kind(&h, ProofKind::LedgerStaleness);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("captured after it"), "{text}");
        assert!(text.contains("vds figma frames"), "{text}");
    }

    fn write_figma_ledger(h: &Harness, at: &str) {
        use vds_figma::ledger::{FigmaLedger, LEDGER_SCHEMA_VERSION};

        let parts: Vec<u32> = at
            .trim_end_matches('Z')
            .split(['-', 'T', ':'])
            .map(|p| p.parse().unwrap())
            .collect();
        let ledger = FigmaLedger {
            schema_version: LEDGER_SCHEMA_VERSION,
            generated_at: vds_core::Timestamp::fixed(
                parts[0] as i32,
                parts[1],
                parts[2],
                parts[3],
                parts[4],
                parts[5],
            ),
            generated_by: "vds figma pull".into(),
            file_key: "KEY".into(),
            file_version: "1".into(),
            file_name: "decided".into(),
            nodes: vec![],
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
