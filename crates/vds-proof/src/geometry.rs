//! The `geometry` proof. The twelfth kind, and the only one that carries a
//! DIRECTION rather than a threshold.
//!
//! VDS S-7(5): "each registered surface's SHAPE - radius, boundary weight,
//! density, type scale - is the one the design system specifies, and the count of
//! surfaces that do not comply is BOUNDED AND FALLING". Added by amendment on
//! 2026-07-31 by the route VDS S-7(6) requires.
//!
//! # What the first eleven kinds could not see
//!
//! They answer WHICH components a surface uses, in WHAT state, and in WHAT
//! arrangement. None answers what the surface LOOKS LIKE. A page can compose only
//! registered components, each in an enforceable status, arranged exactly as its
//! frame draws, and still read as the outgoing design, because radius, boundary
//! weight, density and type scale are none of those things.
//!
//! S-7A(1) records the observation that made this visible, and it is not
//! hypothetical: on the subscriber project the token layer was migrated to a new
//! palette across six themes, every proof went green, three separate progress
//! numbers read high, and the application looked substantially unchanged to the
//! person who used it. Every instrument was correct.
//!
//! # Why this proof refuses a ratchet
//!
//! S-7A(2) is the operative clause. The subscriber project HAD a shape
//! instrument: a ratchet holding the count of non-compliant containers where it
//! stood so it could not rise, reporting "561 hand-rolled card-geometry
//! containers, pin 561". That is a FLOOR, and a floor is a different instrument
//! from a target. A number that may only be held can never fall, and this one did
//! not: it went 667 to 561 through work done for other reasons, then stopped for
//! good. A ratchet that never tightens is a record of a defect, presented as a
//! control. R3 is the whole difference.
//!
//! # Why the window is measured against the READING and never the clock
//!
//! R3 asks whether the bound fell recently enough, which is a question about
//! time, and a proof that read the system clock would produce different findings
//! from identical inputs. VDS S-7(2)(1) requires an unchanged check to cite the
//! same evidence, and the evidence digest is taken over findings and inputs.
//! So "recently" is measured from [`GeometryReading::taken_at`], which is an
//! input and is inside the digest. Re-running against yesterday's reading gives
//! yesterday's answer, which is the correct behaviour: it is the reading that
//! went stale, and `ledger_staleness` is the kind that says so.
//!
//! # The rules
//!
//! One row is one geometry bound, which is one surface kind.
//!
//!   R1  the bound cannot be failed: it is at or above the number of surfaces
//!       the reading even considered. Nothing the reading could contain would
//!       exceed it. Fatal, and the row is NOT enforced, because a row that
//!       cannot fail is not a row that was checked. The twin of `contrast` R7
//!       and `screen_parity` R1.
//!   R2  the non-compliant count EXCEEDS the bound. The shape got worse.
//!   R3  the bound has not fallen within the declared window. S-7A(2).
//!   R4  the bound was RAISED. Reported apart from R3 because "held" and
//!       "loosened" are different failures and only one of them is somebody
//!       quietly moving the goalposts.
//!   R5  the reading was taken from a code model. S-7A(4). Fatal, and no row is
//!       enforced against it: nothing was measured.
//!   R6  a binding bound with no reading at all, or none for its kind. UNKNOWN,
//!       never a pass.
//!   R7  the undecided surfaces could carry the count over the bound. The
//!       instrument saying "I DO NOT KNOW" rather than passing on a total it
//!       cannot complete.
//!   R8  two enforceable bounds name the same surface kind. Nothing says which
//!       governs.
//!   R9  the reading's buckets do not partition its own population, or the
//!       bound's history is out of order. An input that does not add up cannot
//!       be compared against anything.
//!   R10 the reading does not match its own content digest. Fatal, and NO row is
//!       enforced against it. This proof reads the reading as its only
//!       measurement, so without it a bound being exceeded could be brought back
//!       inside by editing one integer in a YAML file, flipping the proof from
//!       failed to passed with no surface changed and nothing reporting it.
//!   W1  the reading measures a surface kind no bound record claims. Shape that
//!       is being counted and that nobody has undertaken to reduce.
//!
//! S-7A(3) is enforced by the TYPE and appears in no rule here: `surface_kind` is
//! an enum, so a single undifferentiated bound over the whole estate - the "561"
//! that names no work - is unrepresentable rather than merely refused.

use std::collections::BTreeMap;
use std::io::Write;

use vds_core::{
    Compliance, GeometryBound, GeometryReading, KindReading, ProofKind, ReadFrom, Result, Severity,
    Status, SurfaceKind, Violation,
};

use crate::ProofContext;
use crate::run::{Outcome, Verdict};

pub const GATE: &str = "crates/vds-proof/src/geometry.rs";

const RULE_UNFALSIFIABLE: &str = "VDS S-7(2)(4) geometry R1: a bound nothing can exceed";
const RULE_EXCEEDED: &str = "VDS S-7A(2) geometry R2: the count exceeds the bound";
const RULE_NOT_FALLING: &str = "VDS S-7A(2) geometry R3: the bound must FALL, not merely hold";
const RULE_RAISED: &str = "VDS S-7A(2) geometry R4: the bound was raised";
const RULE_CODE_MODEL: &str = "VDS S-7A(4) geometry R5: read from a code model, not what ships";
const RULE_NO_READING: &str = "VDS S-7A(4) geometry R6: nothing was measured";
const RULE_UNDECIDED: &str = "VDS S-7A geometry R7: the undecided could cross the bound";
const RULE_DUPLICATE_KIND: &str = "VDS S-7A(3) geometry R8: two bounds for one surface kind";
const RULE_INPUT_INCOHERENT: &str = "VDS S-7A geometry R9: an input that does not add up";
const RULE_READING_EDITED: &str =
    "VDS S-2(5)(4) geometry R10: the reading witnesses its own content";
const RULE_UNCLAIMED_KIND: &str = "VDS S-7A(3) geometry W1: a shape nobody has undertaken";
const RULE_AUTHORITY_EDITED: &str =
    "draft S-7A(5) geometry R11: the authority snapshot witnesses its own content";
const RULE_AUTHORITY_STALE: &str =
    "draft S-7A(5) geometry R12: a side of the binding moved; the snapshot is STALE";
const RULE_AUTHORITY_DISAGREES: &str =
    "draft S-7A(5) geometry R13: the shipped shape disagrees with the decided one";
const RULE_AUTHORITY_UNSIGNED: &str =
    "draft S-7D geometry W2: no authority; coverage owed, never green, never red";
const RULE_COMPARATOR_UNPINNED: &str = "[2026] VJS-CA-VDS 1 order 9, geometry W3: the comparator carries no seeded negative \
     control, so this limb claims no primacy";
const RULE_NOT_DRAWN: &str =
    "draft S-7A(5A) geometry W4: the signed frame draws nothing on this dimension";
const RULE_ALL_NOT_DRAWN: &str =
    "VDS S-7(2)(4) geometry R15: a binding whose every row is not_drawn cannot fail";

/// Stated on every run, passing or not.
///
/// A proof that does not publish its own boundary reads as a proof of more than
/// it did, and this one is bounded twice over: by what the subject's generator
/// chose to look at, and by what that generator could resolve.
const RESERVED_NOTE: &str = "[reserved] This kind checks the BOUND and its DIRECTION. What \
                             counts as a compliant radius, boundary weight, spacing step or \
                             type step is the subject's design system talking, and VDS does \
                             not hold those thresholds: deciding them here would make VDS a \
                             fourth design authority. So a reading that calls everything \
                             compliant passes this proof, and the honesty of the reading is \
                             the generator's to establish, per VDS S-4(2), by being \
                             byte-reproducible from the shipped artefact.";

/// How many days a bound may stand before R3 bites, given the reading's moment.
///
/// Both timestamps are RFC 3339 UTC, which `Timestamp` guarantees, so the day
/// arithmetic is done on the date prefix rather than by parsing a full instant.
/// A bound's window is declared in days and no rule here is sensitive to hours;
/// pulling in a date library to be wrong by less than a day would be precision
/// nobody asked for.
pub(crate) fn days_between(earlier: &str, later: &str) -> Option<i64> {
    let day = |s: &str| -> Option<i64> {
        let date = s.get(..10)?;
        let mut parts = date.split('-');
        let y: i64 = parts.next()?.parse().ok()?;
        let m: i64 = parts.next()?.parse().ok()?;
        let d: i64 = parts.next()?.parse().ok()?;
        if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
            return None;
        }
        // Days since a fixed epoch, by the civil-from-days algorithm. Exact for
        // every proleptic Gregorian date and needs no table.
        let y = if m <= 2 { y - 1 } else { y };
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = y - era * 400;
        let mp = (m + 9) % 12;
        let doy = (153 * mp + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        Some(era * 146_097 + doe - 719_468)
    };
    Some(day(later)? - day(earlier)?)
}

pub fn run(ctx: &ProofContext, out: &mut dyn Write) -> Result<Outcome> {
    let project = ctx.project;
    let store = ctx.store();
    let mut run = ctx.new_run(ProofKind::Geometry, GATE);
    run.note(RESERVED_NOTE);

    let bounds = store.read_geometry()?;
    for record in &bounds {
        run.input_file(&record.path)?;
    }

    let reading = vds_core::read_reading(project)?;
    if reading.is_some() {
        let path = vds_core::reading_path(project);
        run.input_file(&path)?;
    }

    // R10 BEFORE R5, because "what did this reading say" has to be answerable
    // before "was it taken from an admissible source" means anything. An edited
    // reading's `readFrom` is as untrustworthy as its counts.
    let mut reading = reading;
    if let Some(found) = reading.as_ref()
        && let Some(why) = found.untrustworthy_because()?
    {
        run.fail(Violation::fatal(
            project.rel(&vds_core::reading_path(project)),
            RULE_READING_EDITED,
            "a reading whose contentDigest matches its own content",
            why,
        ));
        // Dropped entirely. Continuing with it would compare bounds against
        // numbers this proof has just said it cannot rely on, and every row
        // would carry a verdict derived from them.
        reading = None;
    }

    // R5 next, and once. A reading taken from a code model is not a weaker
    // reading, it is a different subject, so every row that would be measured
    // against it is unmeasured rather than uncertain. Checking it per row would
    // print the same finding four times and let a reader think four things went
    // wrong.
    let admissible = match &reading {
        // The design file is refused for its OWN reason, never under
        // code_model's ([2026] VJS-CC-VIBE-DESIGN-SYSTEM 3, forbidden clause):
        // a code model is a third artefact that drifts, while the design file
        // is the system of record for what was DECIDED. Its reading is a
        // different SUBJECT - no browser renders it - so the shipped bound has
        // nothing to say about it, and a drawing-side bound is where it will
        // be measured when one is declared.
        Some(r) if r.read_from == vds_core::ReadFrom::DesignFile => {
            run.fail(Violation::fatal(
                project.rel(&vds_core::reading_path(project)),
                RULE_CODE_MODEL,
                "a reading taken from the shipped stylesheet or the shipped source",
                "readFrom is design_file. The design file is the record of what was \
                 DECIDED, and this proof's subject is what SHIPPED: a reading of the \
                 drawing is a fact about the drawing, and reporting it under the shipped \
                 bound would be two numbers wearing one name. Measure it against a bound \
                 declared over the drawing, and name the drawing as the subject in the \
                 output ([2026] VJS-CC-VIBE-DESIGN-SYSTEM 3)."
                    .to_owned(),
            ));
            false
        }
        Some(r) if !r.read_from.is_shipped() => {
            run.fail(Violation::fatal(
                project.rel(&vds_core::reading_path(project)),
                RULE_CODE_MODEL,
                "a reading taken from the shipped stylesheet or the shipped source",
                format!(
                    "readFrom is {}. VDS S-7A(4): a code model of the intended design is a \
                     legitimate design tool and is NOT admissible as the subject of this \
                     proof. It is a third artefact that drifts, and on the subject this \
                     amendment came from a 17-page code model drifted so completely that it \
                     came to model the OUTGOING system it was built to replace.",
                    r.read_from
                ),
            ));
            false
        }
        _ => true,
    };

    // R8. Built before any row is classified, so a duplicate is reported once
    // for the KIND rather than once per record, and neither copy is silently
    // taken as the governing one.
    let mut by_kind: BTreeMap<SurfaceKind, Vec<&GeometryBound>> = BTreeMap::new();
    for record in &bounds {
        if record.value.status.is_enforceable() {
            by_kind
                .entry(record.value.surface_kind)
                .or_default()
                .push(&record.value);
        }
    }

    for record in &bounds {
        let bound = &record.value;
        let location = format!("{} [{}]", bound.id, bound.surface_kind);

        if !bound.status.is_enforceable() {
            run.row(Verdict::Skipped("bound_not_in_an_enforceable_status"));
            continue;
        }

        let siblings = by_kind.get(&bound.surface_kind).map_or(1, Vec::len);
        if siblings > 1 {
            run.row(Verdict::Skipped("two_enforceable_bounds_for_one_kind"));
            run.fail(Violation::fatal(
                location.clone(),
                RULE_DUPLICATE_KIND,
                format!("exactly one enforceable bound for {}", bound.surface_kind),
                format!(
                    "{siblings} enforceable bounds name {}, and nothing says which governs. \
                     Deprecate the superseded one rather than deleting it: a bound's history \
                     is the only evidence the count ever fell.",
                    bound.surface_kind
                ),
            ));
            continue;
        }

        if !bound.is_chronological() {
            run.row(Verdict::Skipped("history_out_of_order"));
            run.fail(Violation::fatal(
                location.clone(),
                RULE_INPUT_INCOHERENT,
                "a history in chronological order, oldest first",
                "the history is out of order. Every question this record answers - what is in \
                 force, did it fall, when - reads the LAST entry, so an out-of-order history \
                 answers all three about the wrong moment."
                    .to_owned(),
            ));
            continue;
        }

        let Some(current) = bound.current() else {
            run.row(Verdict::Skipped("no_bound_ever_declared"));
            run.fail(Violation::fatal(
                location.clone(),
                RULE_NO_READING,
                "at least one entry in the bound's history",
                "the history is empty, so there is no bound in force and nothing to compare a \
                 reading against."
                    .to_owned(),
            ));
            continue;
        };

        if !admissible {
            run.row(Verdict::Skipped("reading_inadmissible_see_r5"));
            continue;
        }

        // R6. The absence of a measurement is not a pass. A bound with no
        // reading is a promise nobody checked, and the row must say UNKNOWN.
        let Some(reading) = reading.as_ref() else {
            run.row(Verdict::Skipped("no_reading_generated"));
            run.fail(Violation::fatal(
                location.clone(),
                RULE_NO_READING,
                format!(
                    "a geometry reading at {}",
                    project.rel(&vds_core::reading_path(project))
                ),
                format!(
                    "no reading has been generated, so the bound of {} is compared against \
                     nothing. This is UNKNOWN and not a pass: the count could be anything.",
                    current.bound
                ),
            ));
            continue;
        };

        let Some(kind_reading) = reading.kind(bound.surface_kind) else {
            run.row(Verdict::Skipped("reading_covers_no_such_kind"));
            run.fail(Violation::fatal(
                location.clone(),
                RULE_NO_READING,
                format!("the reading to cover {}", bound.surface_kind),
                format!(
                    "the reading covers {}, and not this kind. A bound over a shape nothing \
                     measures is UNKNOWN, not met.",
                    if reading.kinds.is_empty() {
                        "nothing".to_owned()
                    } else {
                        reading
                            .kinds
                            .iter()
                            .map(|k| k.surface_kind.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    }
                ),
            ));
            continue;
        };

        if !kind_reading.buckets_partition() {
            run.row(Verdict::Skipped("reading_buckets_do_not_partition"));
            run.fail(Violation::fatal(
                location.clone(),
                RULE_INPUT_INCOHERENT,
                "nonCompliant + undecided <= considered",
                format!(
                    "the reading says {} considered, {} non-compliant and {} undecided, which \
                     is {} surfaces out of {}. The generator is counting something twice or \
                     counting outside its own population, and neither can be compared against \
                     a bound.",
                    kind_reading.considered,
                    kind_reading.non_compliant,
                    kind_reading.undecided,
                    kind_reading.worst_case(),
                    kind_reading.considered
                ),
            ));
            continue;
        }

        // R1, the no-op guard. Checked BEFORE the count comparison, because a
        // bound nothing can exceed would otherwise print a pass, and a pass is
        // exactly what it must not print.
        if current.bound >= kind_reading.considered {
            run.row(Verdict::Skipped("bound_cannot_be_exceeded"));
            run.fail(Violation::fatal(
                location.clone(),
                RULE_UNFALSIFIABLE,
                format!(
                    "a bound below the {} surfaces the reading considered",
                    kind_reading.considered
                ),
                format!(
                    "the bound is {} and only {} surfaces of this kind exist, so no reading \
                     could ever exceed it and this row cannot fail. A bound at or above the \
                     population is not a control, it is a record of one.",
                    current.bound, kind_reading.considered
                ),
            ));
            continue;
        }

        run.row(Verdict::Enforced);

        // R2. The plain violation: more non-compliant surfaces than admitted.
        if kind_reading.non_compliant > current.bound {
            run.fail(Violation::fatal(
                location.clone(),
                RULE_EXCEEDED,
                format!(
                    "at most {} non-compliant {} surfaces",
                    current.bound, bound.surface_kind
                ),
                format!(
                    "{} do not comply, which is {} over the bound.{}",
                    kind_reading.non_compliant,
                    kind_reading.non_compliant - current.bound,
                    sample_tail(kind_reading)
                ),
            ));
        } else if kind_reading.worst_case() > current.bound {
            // R7. Within the bound on what is KNOWN bad, and only because some
            // surfaces were not resolved. Passing here would be the instrument
            // reporting conformance it did not establish.
            run.fail(Violation::fatal(
                location.clone(),
                RULE_UNDECIDED,
                format!(
                    "the non-compliant count to be at most {} once every surface is resolved",
                    current.bound
                ),
                format!(
                    "{} are known non-compliant and {} could not be resolved, so the true \
                     count is anywhere from {} to {} and the bound of {} sits inside that \
                     range. UNDECIDED, not met: the instrument does not know.{}",
                    kind_reading.non_compliant,
                    kind_reading.undecided,
                    kind_reading.non_compliant,
                    kind_reading.worst_case(),
                    current.bound,
                    sample_tail(kind_reading)
                ),
            ));
        }

        // R3 and R4, the direction. Measured from the READING's moment, not the
        // clock, so the finding is a function of the inputs.
        let window = i64::from(bound.declared_window_days);
        match bound.last_reduction() {
            None => run.fail(Violation::fatal(
                location.clone(),
                RULE_NOT_FALLING,
                format!(
                    "the bound to have fallen at least once within {} days",
                    bound.declared_window_days
                ),
                format!(
                    "the bound has NEVER fallen: {} entr{} in the history and not one lower \
                     than the one before it. VDS S-7A(2): a bound that may only be held is a \
                     floor, and a floor is a different instrument from a target.",
                    bound.history.len(),
                    if bound.history.len() == 1 { "y" } else { "ies" }
                ),
            )),
            Some(reduction) => {
                let age = days_between(reduction.at.as_str(), reading.taken_at.as_str());
                match age {
                    None => run.fail(Violation::fatal(
                        location.clone(),
                        RULE_INPUT_INCOHERENT,
                        "two readable UTC dates",
                        format!(
                            "the last reduction is stamped {:?} and the reading {:?}, and the \
                             distance between them could not be computed, so the direction \
                             rule is UNKNOWN rather than met.",
                            reduction.at.as_str(),
                            reading.taken_at.as_str()
                        ),
                    )),
                    Some(days) if days > window => run.fail(Violation::fatal(
                        location.clone(),
                        RULE_NOT_FALLING,
                        format!(
                            "the bound to have fallen within the declared window of {} days",
                            bound.declared_window_days
                        ),
                        format!(
                            "the last reduction was {days} days before this reading, to {}. \
                             The window is {} days. This is the 561-pin-561 shape: a number \
                             that moved once through work done for other reasons and then \
                             stopped.{}",
                            reduction.bound,
                            bound.declared_window_days,
                            reduction
                                .because
                                .as_deref()
                                .map(|w| format!(" That reduction was recorded as: {w}."))
                                .unwrap_or_default()
                        ),
                    )),
                    Some(_) => {}
                }
            }
        }

        // R4. A raise is not a failure of direction, it is a reversal of it, and
        // a reader needs to be told which happened.
        if let Some(previous) = bound.history.iter().rev().nth(1)
            && current.bound > previous.bound
        {
            run.fail(Violation::fatal(
                location.clone(),
                RULE_RAISED,
                format!("a bound at or below the previous {}", previous.bound),
                format!(
                    "the bound was RAISED from {} to {} on {}. A bound that goes up is not a \
                     bound. If the population genuinely grew, the honest record is a new \
                     baseline with the reason on it, not a higher number in the same series.",
                    previous.bound,
                    current.bound,
                    current.at.as_str()
                ),
            ));
        }
    }

    // W1. Shape that IS being measured and that nobody has undertaken to reduce.
    // A warning and not fatal: measuring more than you have promised to fix is
    // the right direction to be wrong in, and failing on it would teach a
    // generator to measure less.
    if let Some(reading) = reading.as_ref() {
        let claimed: Vec<SurfaceKind> = bounds
            .iter()
            .filter(|r| r.value.status.is_enforceable())
            .map(|r| r.value.surface_kind)
            .collect();
        for kind_reading in &reading.kinds {
            if !claimed.contains(&kind_reading.surface_kind) && kind_reading.non_compliant > 0 {
                run.warn(Violation::fatal(
                    format!("reading [{}]", kind_reading.surface_kind),
                    RULE_UNCLAIMED_KIND,
                    format!(
                        "a geometry bound claiming {}, so the count is somebody's to drive down",
                        kind_reading.surface_kind
                    ),
                    format!(
                        "{} of {} surfaces do not comply and no bound record claims this \
                         shape, so the number is measured and unowned.",
                        kind_reading.non_compliant, kind_reading.considered
                    ),
                ));
            }
        }

        if !reading.does_not_cover.is_empty() {
            run.note(format!(
                "[reading] the generator states it does NOT cover: {}",
                reading.does_not_cover.join("; ")
            ));
        }
        run.note(format!(
            "[reading] taken {} from {} ({})",
            reading.taken_at.as_str(),
            reading.read_from,
            if reading.sources.is_empty() {
                "no source named".to_owned()
            } else {
                reading.sources.join(", ")
            }
        ));
    }

    // Draft S-7A(5), ENACTMENT PENDING: the two-sided binding. The snapshot
    // binds the AUTHORITY reading (the signed frame's decided values, fetched
    // out of band and saved as a capture) to the ARTEFACT reading above. The
    // proof never re-derives the agreement - that would need the values - it
    // verifies the BINDING: both input hashes must still match what is on
    // disk, and the frame must carry authority under the sign-off register.
    if let Some(snapshot) = vds_core::read_authority(project)? {
        run.input_file(&vds_core::authority_path(project))?;

        if let Some(why) = snapshot.untrustworthy_because()? {
            // R11. Nothing in an edited snapshot can be relied on.
            for _ in &snapshot.rows {
                run.row(Verdict::Skipped("authority_snapshot_edited"));
            }
            run.fail(Violation::fatal(
                project.rel(&vds_core::authority_path(project)),
                RULE_AUTHORITY_EDITED,
                "a snapshot whose contentDigest matches its own content",
                why,
            ));
        } else {
            // R12, both sides. STALE is a failure, never a quiet green: the
            // whole point of the binding is that it EXPIRES when either side
            // moves, visibly.
            let mut stale: Vec<String> = Vec::new();
            match reading.as_ref() {
                None => stale.push(
                    "the ARTEFACT side: no geometry reading exists, so the reading this \
                     snapshot was bound to is gone"
                        .to_owned(),
                ),
                Some(r) if r.content_digest != snapshot.reading_digest => stale.push(format!(
                    "the ARTEFACT side: the geometry reading now digests to {} and the \
                     snapshot was bound to {}",
                    r.content_digest, snapshot.reading_digest
                )),
                Some(_) => {}
            }
            let capture_path = project.root.join(&snapshot.capture);
            match vds_core::Digest::of_file(&capture_path) {
                Err(_) => stale.push(format!(
                    "the AUTHORITY side: the capture {} cannot be read",
                    snapshot.capture
                )),
                Ok(now) if now != snapshot.capture_digest => stale.push(format!(
                    "the AUTHORITY side: the capture {} now digests to {now} and the \
                     snapshot was bound to {}",
                    snapshot.capture, snapshot.capture_digest
                )),
                Ok(_) => {}
            }
            // R14, the THIRD side ([2026] VJS-CA-VDS 1 order 8). The agreement
            // rows are the comparator's assertion and the engine cannot
            // re-derive them, so a comparator that moved after the comparison
            // is an expired input exactly like a moved capture, and a snapshot
            // that outlives it is an agreement measured by a program that no
            // longer exists.
            let comparator_path = project.root.join(&snapshot.comparator);
            match vds_core::Digest::of_file(&comparator_path) {
                Err(_) => stale.push(format!(
                    "the COMPARATOR side: {} cannot be read, so the program that produced \
                     these agreement rows is gone",
                    snapshot.comparator
                )),
                Ok(now) if now != snapshot.comparator_digest => stale.push(format!(
                    "the COMPARATOR side: {} now digests to {now} and the snapshot was \
                     bound to {}",
                    snapshot.comparator, snapshot.comparator_digest
                )),
                Ok(_) => {}
            }

            if !stale.is_empty() {
                for _ in &snapshot.rows {
                    run.row(Verdict::Skipped("authority_snapshot_stale"));
                }
                run.fail(Violation::fatal(
                    project.rel(&vds_core::authority_path(project)),
                    RULE_AUTHORITY_STALE,
                    "both input hashes to still match what is on disk",
                    format!(
                        "{}. The binding expired: regenerate the snapshot from the current \
                         capture and reading rather than trusting an agreement measured \
                         between artefacts that no longer exist.",
                        stale.join("; ")
                    ),
                ));
            } else {
                // The authority question: does a sign-off row cover the
                // frame's CURRENT hash? Draft S-7D: no_authority is distinct,
                // and an agreement CLAIM against an unsigned frame is refused.
                let signoffs: Vec<vds_core::SignOff> = store
                    .read_signoffs()?
                    .into_iter()
                    .map(|l| l.value)
                    .collect();
                let frames = vds_figma::frames::read(project)?;
                let current_frame = frames
                    .as_ref()
                    .filter(|l| l.file_key == snapshot.file_key)
                    .and_then(|l| l.row(&snapshot.node_id))
                    .and_then(|r| r.content_digest.as_ref());
                let authority = vds_core::frame_authority(
                    &snapshot.file_key,
                    &snapshot.node_id,
                    current_frame,
                    &signoffs,
                );

                // W3 ([2026] VJS-CA-VDS 1 order 9). The comparator's
                // assertion is the one input the engine cannot falsify, so
                // until the comparator itself is pinned with a seeded negative
                // control, this limb claims no primacy over any repo-local
                // gate on the same ground (SC-OPBOX 1 orders 2 and 3).
                let comparator_pinned = store.read_lock()?.is_some_and(|lock| {
                    lock.entries.iter().any(|entry| {
                        entry.path == snapshot.comparator
                            && !entry.failing_direction_tests.is_empty()
                    })
                });
                if !comparator_pinned {
                    run.warn(Violation::fatal(
                        snapshot.comparator.clone(),
                        RULE_COMPARATOR_UNPINNED,
                        "the comparator pinned in .vds/enforcement.lock with a \
                         failing-direction test",
                        format!(
                            "{} produces the agreement rows this limb rests on and carries \
                             no seeded negative control in the lock, so `agrees` is an \
                             assertion the engine cannot falsify. NO WARRANT MAY CLAIM \
                             PRIMACY for this limb over a repo-local gate on the same \
                             ground, and no such gate may be retired in reliance on it \
                             ([2026] VJS-SC-OPBOX 1 orders 2 and 3; [2026] VJS-CA-VDS 1 \
                             order 9).",
                            snapshot.comparator
                        ),
                    ));
                }

                // Draft S-7A(5A): a snapshot whose every row is `not_drawn` is
                // a binding nothing can fail, refused under VDS S-7(2)(4).
                let mut all_not_drawn = false;
                if !snapshot.rows.is_empty()
                    && snapshot
                        .rows
                        .iter()
                        .all(|r| r.agrees == vds_core::AgreementState::NotDrawn)
                {
                    for _ in &snapshot.rows {
                        run.row(Verdict::Skipped("every_row_not_drawn"));
                    }
                    run.fail(Violation::fatal(
                        project.rel(&vds_core::authority_path(project)),
                        RULE_ALL_NOT_DRAWN,
                        "at least one surface kind the signed frame actually draws",
                        format!(
                            "all {} row(s) report not_drawn, so the signed frame draws \
                             nothing this binding could compare and no reading could ever \
                             fail it. A binding over silence is not a control, it is the \
                             appearance of one.",
                            snapshot.rows.len()
                        ),
                    ));
                    all_not_drawn = true;
                }

                for row in snapshot.rows.iter().filter(|_| !all_not_drawn) {
                    let location = format!(
                        "authority [{}] {}/{}",
                        row.surface_kind, snapshot.file_key, snapshot.node_id
                    );
                    // NOT_DRAWN first, and it can never reach a conformance
                    // arm: a frame binds only for what it draws in the states
                    // it draws, and a comparator that found nothing to compare
                    // has recorded silence, not agreement (draft S-7A(5A),
                    // [2026] VJS-SC-OPBOX 1 orders 6 and 14).
                    if row.agrees == vds_core::AgreementState::NotDrawn {
                        run.row(Verdict::Skipped("surface_kind_not_drawn_by_the_frame"));
                        run.warn(Violation::fatal(
                            location,
                            RULE_NOT_DRAWN,
                            format!(
                                "the signed frame to draw {} if this binding is to speak \
                                 about it",
                                row.surface_kind
                            ),
                            format!(
                                "no_authority for {}: the signed frame draws nothing on \
                                 this dimension{}. COVERAGE OWED, and it can never \
                                 contribute conformance.",
                                row.surface_kind,
                                row.because
                                    .as_deref()
                                    .map(|b| format!(" ({b})"))
                                    .unwrap_or_default()
                            ),
                        ));
                        continue;
                    }
                    match &authority {
                        vds_core::FrameAuthority::Unsigned { because }
                            if row.agrees.is_conformance() =>
                        {
                            run.row(Verdict::Enforced);
                            run.fail(Violation::fatal(
                                location,
                                RULE_AUTHORITY_UNSIGNED,
                                "agreement claimed only against a frame whose CURRENT hash \
                                 matches a sign-off row",
                                format!(
                                    "the snapshot claims the shipped {} agrees with the \
                                     decided one, and the frame carries no authority: \
                                     {because} A proof cannot claim conformance against an \
                                     unsigned frame (draft S-7D).",
                                    row.surface_kind
                                ),
                            ));
                        }
                        vds_core::FrameAuthority::Unsigned { because } => {
                            run.row(Verdict::Skipped("no_authority_frame_unsigned"));
                            run.warn(Violation::fatal(
                                location,
                                RULE_AUTHORITY_UNSIGNED,
                                "a sign-off row matching the frame's current content hash",
                                format!(
                                    "no_authority: {because} COVERAGE OWED, and neither a \
                                     pass nor a failure."
                                ),
                            ));
                        }
                        vds_core::FrameAuthority::Signed { .. } if !row.agrees.is_conformance() => {
                            run.row(Verdict::Enforced);
                            run.fail(Violation::fatal(
                                location,
                                RULE_AUTHORITY_DISAGREES,
                                format!(
                                    "the shipped {} to agree with the decided one \
                                     (capture {}, fetched {})",
                                    row.surface_kind,
                                    snapshot.capture,
                                    snapshot.fetched_at.as_str()
                                ),
                                format!(
                                    "the comparator found disagreement{}. Taste was \
                                     exercised at sign-off; the resolution is to move the \
                                     shipped shape, or to redraw and RE-SIGN the frame - \
                                     never an engine-side excusal (draft S-7D).",
                                    row.because
                                        .as_deref()
                                        .map(|b| format!(": {b}"))
                                        .unwrap_or_default()
                                ),
                            ));
                        }
                        vds_core::FrameAuthority::Signed { .. } => {
                            run.row(Verdict::Enforced);
                        }
                    }
                }
            }
        }
    }

    if bounds.is_empty() {
        run.note(
            "[scope] no geometry bound is declared, so every row is skipped and this run is \
             vacuous. That is the honest state of a project that has not yet undertaken to \
             reduce any shape, and it is NOT evidence: VDS S-7(2)(4) refuses a vacuous run as \
             warrant evidence for exactly this reason.",
        );
    }

    run.finish(&ctx.capture_options()?, out)
}

fn sample_tail(reading: &KindReading) -> String {
    if reading.sample.is_empty() {
        String::new()
    } else {
        format!(" Worst offenders: {}.", reading.sample.join(", "))
    }
}

/// Kept so the unused-import lint does not force a reader to guess why
/// [`Compliance`] and [`Status`] are named in this file's imports.
#[allow(dead_code)]
fn _type_witnesses(_: Compliance, _: Status, _: ReadFrom, _: GeometryReading, _: Severity) {}

#[cfg(test)]
mod tests {
    use super::days_between;

    #[test]
    fn the_day_distance_is_exact_across_a_month_and_a_leap_year() {
        assert_eq!(
            days_between("2026-07-01T00:00:00Z", "2026-07-31T00:00:00Z"),
            Some(30)
        );
        assert_eq!(
            days_between("2026-07-31T00:00:00Z", "2026-07-01T00:00:00Z"),
            Some(-30),
            "a reading OLDER than the reduction gives a negative distance rather than \
             wrapping, so a stale reading cannot look like an overdue bound"
        );
        // 2024 was a leap year. A naive 365-day year gets this wrong by one, and
        // one day is the whole margin on a 30-day window declared on the 30th.
        assert_eq!(
            days_between("2024-02-28T00:00:00Z", "2024-03-01T00:00:00Z"),
            Some(2)
        );
        assert_eq!(
            days_between("2023-02-28T00:00:00Z", "2023-03-01T00:00:00Z"),
            Some(1)
        );
        assert_eq!(
            days_between("2025-12-31T00:00:00Z", "2026-01-01T00:00:00Z"),
            Some(1)
        );
    }

    #[test]
    fn an_unreadable_date_is_none_rather_than_zero() {
        // Zero would read as "the bound fell today", which is the direction that
        // turns an unreadable input into a pass.
        assert_eq!(days_between("not-a-date", "2026-07-31T00:00:00Z"), None);
        assert_eq!(
            days_between("2026-13-01T00:00:00Z", "2026-07-31T00:00:00Z"),
            None
        );
        assert_eq!(days_between("2026-07", "2026-07-31T00:00:00Z"), None);
    }
}

#[cfg(test)]
mod proof_tests {
    use crate::testing::{Harness, run_kind};
    use vds_core::{
        EXIT_PASSED, EXIT_VACUOUS, EXIT_VIOLATION, ProofKind, ProofStatus, ReadFrom, SurfaceKind,
    };

    /// A bound that fell inside its window, over a reading that is under it.
    /// The only shape that passes.
    fn compliant(h: &Harness) {
        h.geometry_bound(
            SurfaceKind::Radius,
            30,
            &[("2026-06-01", 667), ("2026-07-20", 561)],
        );
        h.geometry_reading(
            "2026-07-31",
            ReadFrom::ShippedStylesheet,
            &[(SurfaceKind::Radius, 900, 540, 0)],
        );
    }

    #[test]
    fn a_bound_that_fell_inside_its_window_over_a_reading_under_it_passes() {
        let h = Harness::new();
        compliant(&h);
        let (outcome, text) = run_kind(&h, ProofKind::Geometry);
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
        assert_eq!(outcome.exit_code, EXIT_PASSED, "{text}");
        assert_eq!(outcome.rows_enforced, 1, "{text}");
    }

    /// THE failing-direction test VDS S-7(2)(2) requires, and the one the
    /// enforcement lock names. It is R3, the operative clause, because R3 is the
    /// whole reason the kind exists: everything else here a ratchet could also
    /// have caught.
    #[test]
    fn geometry_fails_when_the_bound_only_ever_held() {
        let h = Harness::new();
        // 667 to 561, then nothing for 71 days. The subscriber project's own
        // instrument, reproduced: a number that moved once through work done for
        // other reasons and then stopped.
        h.geometry_bound(
            SurfaceKind::Radius,
            30,
            &[("2026-05-01", 667), ("2026-05-21", 561)],
        );
        h.geometry_reading(
            "2026-07-31",
            ReadFrom::ShippedStylesheet,
            &[(SurfaceKind::Radius, 900, 540, 0)],
        );
        let (outcome, text) = run_kind(&h, ProofKind::Geometry);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert_eq!(outcome.status, ProofStatus::Failed);
        assert!(text.contains("71 days"), "{text}");
        assert!(text.contains("561-pin-561"), "{text}");
        // The row is still ENFORCED. The bound was checked and found wanting,
        // which is different from a row that could not be checked.
        assert_eq!(outcome.rows_enforced, 1, "{text}");
    }

    #[test]
    fn a_bound_that_never_fell_at_all_fails_and_says_so_differently() {
        let h = Harness::new();
        h.geometry_bound(SurfaceKind::Radius, 30, &[("2026-07-30", 561)]);
        h.geometry_reading(
            "2026-07-31",
            ReadFrom::ShippedStylesheet,
            &[(SurfaceKind::Radius, 900, 540, 0)],
        );
        let (outcome, text) = run_kind(&h, ProofKind::Geometry);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        // Declaring a baseline yesterday must not read as a reduction. If it
        // did, every project could satisfy the direction rule by registering.
        assert!(text.contains("NEVER fallen"), "{text}");
    }

    #[test]
    fn a_count_over_the_bound_fails() {
        let h = Harness::new();
        h.geometry_bound(
            SurfaceKind::Radius,
            30,
            &[("2026-06-01", 667), ("2026-07-20", 561)],
        );
        h.geometry_reading(
            "2026-07-31",
            ReadFrom::ShippedStylesheet,
            &[(SurfaceKind::Radius, 900, 600, 0)],
        );
        let (outcome, text) = run_kind(&h, ProofKind::Geometry);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("39 over the bound"), "{text}");
    }

    #[test]
    fn a_bound_at_or_above_the_population_cannot_fail_and_is_refused() {
        let h = Harness::new();
        // Under the bound, fell recently: it would otherwise be a clean pass.
        h.geometry_bound(
            SurfaceKind::Radius,
            30,
            &[("2026-06-01", 2000), ("2026-07-20", 1000)],
        );
        h.geometry_reading(
            "2026-07-31",
            ReadFrom::ShippedStylesheet,
            &[(SurfaceKind::Radius, 900, 540, 0)],
        );
        let (outcome, text) = run_kind(&h, ProofKind::Geometry);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("cannot fail"), "{text}");
        // NOT enforced. A row that cannot fail is not a row that was checked,
        // and counting it would let a project satisfy the non-vacuity condition
        // with rows that establish nothing.
        assert_eq!(outcome.rows_enforced, 0, "{text}");
    }

    #[test]
    fn undecided_surfaces_that_could_cross_the_bound_are_undecided_and_not_a_pass() {
        let h = Harness::new();
        h.geometry_bound(
            SurfaceKind::Radius,
            30,
            &[("2026-06-01", 667), ("2026-07-20", 561)],
        );
        // 540 known bad is UNDER 561. 30 unresolved could take it to 570, which
        // is over. The instrument must say it does not know.
        h.geometry_reading(
            "2026-07-31",
            ReadFrom::ShippedStylesheet,
            &[(SurfaceKind::Radius, 900, 540, 30)],
        );
        let (outcome, text) = run_kind(&h, ProofKind::Geometry);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("UNDECIDED"), "{text}");
        assert!(text.contains("540 to 570"), "{text}");
    }

    #[test]
    fn undecided_surfaces_that_cannot_cross_the_bound_still_pass() {
        // The other half, and the one that stops R7 crying wolf. An instrument
        // that failed on ANY undecided surface would be unusable against a real
        // codebase and would be switched off.
        let h = Harness::new();
        h.geometry_bound(
            SurfaceKind::Radius,
            30,
            &[("2026-06-01", 667), ("2026-07-20", 561)],
        );
        h.geometry_reading(
            "2026-07-31",
            ReadFrom::ShippedStylesheet,
            &[(SurfaceKind::Radius, 900, 500, 30)],
        );
        let (outcome, text) = run_kind(&h, ProofKind::Geometry);
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
    }

    #[test]
    fn a_design_file_reading_is_refused_for_its_own_reason_not_code_models() {
        // [2026] VJS-CC-VIBE-DESIGN-SYSTEM 3 D2, and its forbidden clause. The
        // drawing's reading is a DIFFERENT SUBJECT, not an unreliable one, and
        // the refusal must say so - a reader told their design file "drifts"
        // has been told something false about the system of record.
        let h = Harness::new();
        h.geometry_bound(
            SurfaceKind::Radius,
            30,
            &[("2026-06-01", 667), ("2026-07-20", 561)],
        );
        h.geometry_reading(
            "2026-07-31",
            ReadFrom::DesignFile,
            &[(SurfaceKind::Radius, 900, 100, 0)],
        );
        let (outcome, text) = run_kind(&h, ProofKind::Geometry);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert_eq!(
            outcome.rows_enforced, 0,
            "a drawing enforces no shipped row: {text}"
        );
        assert!(
            text.contains("two numbers wearing one name"),
            "the refusal must state the subject distinction: {text}"
        );
        assert!(
            !text.contains("third artefact that drifts"),
            "refusing the design file under code_model's reason is forbidden by the order: {text}"
        );
    }

    #[test]
    fn a_reading_taken_from_a_code_model_is_refused_and_enforces_nothing() {
        let h = Harness::new();
        h.geometry_bound(
            SurfaceKind::Radius,
            30,
            &[("2026-06-01", 667), ("2026-07-20", 561)],
        );
        h.geometry_reading(
            "2026-07-31",
            ReadFrom::CodeModel,
            &[(SurfaceKind::Radius, 900, 100, 0)],
        );
        let (outcome, text) = run_kind(&h, ProofKind::Geometry);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("S-7A(4)"), "{text}");
        // The point of the rule: a code model can report ANY number, and 100 of
        // 900 would have passed every other limb.
        assert_eq!(outcome.rows_enforced, 0, "{text}");
    }

    #[test]
    fn a_bound_with_no_reading_is_unknown_and_never_a_pass() {
        let h = Harness::new();
        h.geometry_bound(
            SurfaceKind::Radius,
            30,
            &[("2026-06-01", 667), ("2026-07-20", 561)],
        );
        let (outcome, text) = run_kind(&h, ProofKind::Geometry);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("UNKNOWN"), "{text}");
        assert_eq!(outcome.rows_enforced, 0, "{text}");
    }

    #[test]
    fn a_bound_whose_kind_the_reading_does_not_cover_is_unknown() {
        let h = Harness::new();
        h.geometry_bound(
            SurfaceKind::Density,
            30,
            &[("2026-06-01", 90), ("2026-07-20", 40)],
        );
        h.geometry_reading(
            "2026-07-31",
            ReadFrom::ShippedStylesheet,
            &[(SurfaceKind::Radius, 900, 540, 0)],
        );
        let (outcome, text) = run_kind(&h, ProofKind::Geometry);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("not this kind"), "{text}");
    }

    #[test]
    fn a_raised_bound_is_reported_as_a_raise_and_not_merely_as_a_hold() {
        let h = Harness::new();
        h.geometry_bound(
            SurfaceKind::Radius,
            30,
            &[("2026-06-01", 400), ("2026-07-20", 561)],
        );
        h.geometry_reading(
            "2026-07-31",
            ReadFrom::ShippedStylesheet,
            &[(SurfaceKind::Radius, 900, 540, 0)],
        );
        let (outcome, text) = run_kind(&h, ProofKind::Geometry);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("was RAISED from 400 to 561"), "{text}");
    }

    #[test]
    fn two_enforceable_bounds_for_one_kind_are_refused_rather_than_one_being_chosen() {
        let h = Harness::new();
        h.geometry_bound(
            SurfaceKind::Radius,
            30,
            &[("2026-06-01", 667), ("2026-07-20", 561)],
        );
        h.geometry_bound(
            SurfaceKind::Radius,
            30,
            &[("2026-06-01", 100), ("2026-07-20", 50)],
        );
        h.geometry_reading(
            "2026-07-31",
            ReadFrom::ShippedStylesheet,
            &[(SurfaceKind::Radius, 900, 540, 0)],
        );
        let (outcome, text) = run_kind(&h, ProofKind::Geometry);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("nothing says which governs"), "{text}");
        assert_eq!(outcome.rows_enforced, 0, "{text}");
    }

    #[test]
    fn a_reading_whose_buckets_do_not_partition_is_refused_rather_than_wrapped() {
        let h = Harness::new();
        h.geometry_bound(
            SurfaceKind::Radius,
            30,
            &[("2026-06-01", 667), ("2026-07-20", 561)],
        );
        h.geometry_reading(
            "2026-07-31",
            ReadFrom::ShippedStylesheet,
            &[(SurfaceKind::Radius, 100, 80, 50)],
        );
        let (outcome, text) = run_kind(&h, ProofKind::Geometry);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("counting something twice"), "{text}");
    }

    #[test]
    fn a_measured_shape_nobody_has_undertaken_warns_and_does_not_block() {
        let h = Harness::new();
        h.geometry_bound(
            SurfaceKind::Radius,
            30,
            &[("2026-06-01", 667), ("2026-07-20", 561)],
        );
        h.geometry_reading(
            "2026-07-31",
            ReadFrom::ShippedStylesheet,
            &[
                (SurfaceKind::Radius, 900, 540, 0),
                (SurfaceKind::TypeScale, 400, 120, 0),
            ],
        );
        let (outcome, text) = run_kind(&h, ProofKind::Geometry);
        // PASSES, and still reports. A warning that blocked would teach a
        // generator to measure less, which is the wrong direction to push.
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
        assert!(text.contains("W1"), "{text}");
        assert!(text.contains("measured and unowned"), "{text}");
    }

    #[test]
    fn a_project_with_no_bound_is_vacuous_and_says_it_is_not_evidence() {
        let h = Harness::new();
        let (outcome, text) = run_kind(&h, ProofKind::Geometry);
        assert_eq!(outcome.status, ProofStatus::Vacuous, "{text}");
        assert_eq!(outcome.exit_code, EXIT_VACUOUS, "{text}");
        assert_eq!(outcome.rows_enforced, 0);
        assert!(text.contains("NOT evidence"), "{text}");
    }

    #[test]
    fn the_reserved_note_is_printed_on_a_passing_run() {
        // A proof that publishes its own boundary only when it fails is a proof
        // whose readers learn the boundary at the worst moment.
        let h = Harness::new();
        compliant(&h);
        let (_, text) = run_kind(&h, ProofKind::Geometry);
        assert!(text.contains("fourth design authority"), "{text}");
    }
}

#[cfg(test)]
mod authority_binding_tests {
    use crate::testing::{Harness, run_kind};
    use vds_core::{AgreementState, EXIT_VIOLATION, ProofKind, ProofStatus, ReadFrom, SurfaceKind};

    /// A compliant bound and reading, a captured frame, and a sign-off at the
    /// frame's current hash.
    fn signed(h: &Harness) {
        h.geometry_bound(
            SurfaceKind::Radius,
            30,
            &[("2026-06-01", 667), ("2026-07-20", 561)],
        );
        h.geometry_reading(
            "2026-07-31",
            ReadFrom::ShippedStylesheet,
            &[(SurfaceKind::Radius, 900, 540, 0)],
        );
        h.frames(&[Harness::frame("1:2", "kit frame", &["body"], 1)]);
        h.signoff("KEY", "1:2");
    }

    #[test]
    fn a_fresh_agreeing_snapshot_against_a_signed_frame_passes() {
        let h = Harness::new();
        signed(&h);
        h.geometry_authority(
            "KEY",
            "1:2",
            &[(SurfaceKind::Radius, AgreementState::Agrees)],
        );
        let (outcome, text) = run_kind(&h, ProofKind::Geometry);
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
        // One bound row plus one authority row.
        assert_eq!(outcome.rows_enforced, 2, "{text}");
    }

    /// THE failing-direction seed for R13: the comparator found the shipped
    /// shape disagreeing with the decided one.
    #[test]
    fn a_disagreement_with_the_decided_shape_fails_and_names_the_resolution() {
        let h = Harness::new();
        signed(&h);
        h.geometry_authority(
            "KEY",
            "1:2",
            &[(SurfaceKind::Radius, AgreementState::Disagrees)],
        );
        let (outcome, text) = run_kind(&h, ProofKind::Geometry);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("off the decided scale"), "{text}");
        assert!(
            text.contains("never an engine-side excusal"),
            "the repeal is stated on the finding: {text}"
        );
    }

    /// R12, artefact side: the reading regenerates after the comparison, and
    /// the binding must EXPIRE visibly rather than stay green.
    #[test]
    fn a_snapshot_whose_reading_moved_is_stale_and_fails() {
        let h = Harness::new();
        signed(&h);
        h.geometry_authority(
            "KEY",
            "1:2",
            &[(SurfaceKind::Radius, AgreementState::Agrees)],
        );
        // The artefact side moves: a new reading with different counts.
        h.geometry_reading(
            "2026-08-01",
            ReadFrom::ShippedStylesheet,
            &[(SurfaceKind::Radius, 900, 500, 0)],
        );
        let (outcome, text) = run_kind(&h, ProofKind::Geometry);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("ARTEFACT side"), "{text}");
        assert!(text.contains("The binding expired"), "{text}");
    }

    /// R12, authority side: the saved capture moves.
    #[test]
    fn a_snapshot_whose_capture_moved_is_stale_and_fails() {
        let h = Harness::new();
        signed(&h);
        h.geometry_authority(
            "KEY",
            "1:2",
            &[(SurfaceKind::Radius, AgreementState::Agrees)],
        );
        h.write(
            "design/captures/geometry-authority.json",
            "{\"nodes\":{\"1:2\":{\"document\":{\"name\":\"REDRAWN\"}}}}\n",
        );
        let (outcome, text) = run_kind(&h, ProofKind::Geometry);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("AUTHORITY side"), "{text}");
    }

    /// Draft S-7D: an agreement CLAIM against an unsigned frame is refused.
    #[test]
    fn an_agreement_claim_against_an_unsigned_frame_is_refused() {
        let h = Harness::new();
        h.geometry_bound(
            SurfaceKind::Radius,
            30,
            &[("2026-06-01", 667), ("2026-07-20", 561)],
        );
        h.geometry_reading(
            "2026-07-31",
            ReadFrom::ShippedStylesheet,
            &[(SurfaceKind::Radius, 900, 540, 0)],
        );
        h.frames(&[Harness::frame("1:2", "kit frame", &["body"], 1)]);
        // NO sign-off.
        h.geometry_authority(
            "KEY",
            "1:2",
            &[(SurfaceKind::Radius, AgreementState::Agrees)],
        );
        let (outcome, text) = run_kind(&h, ProofKind::Geometry);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(
            text.contains("cannot claim conformance against an unsigned frame"),
            "{text}"
        );
    }

    /// MANDATE SEED 1 for this limb: a signed frame whose hash then drifts
    /// flips the authority rows to no_authority - never green, never red.
    #[test]
    fn a_frame_that_changed_after_signoff_flips_the_binding_to_no_authority() {
        let h = Harness::new();
        signed(&h);
        // The frame is REDRAWN after sign-off: its current hash moves.
        h.frames(&[Harness::frame("1:2", "kit frame REDRAWN", &["body"], 2)]);
        // An honest snapshot against the redrawn frame, not claiming agreement.
        h.geometry_authority(
            "KEY",
            "1:2",
            &[(SurfaceKind::Radius, AgreementState::Disagrees)],
        );
        let (outcome, text) = run_kind(&h, ProofKind::Geometry);
        // The bound row still passes; the authority row is coverage owed, and
        // the disagreement is NOT adjudicated red against a dead sign-off.
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
        assert!(text.contains("no_authority_frame_unsigned"), "{text}");
        assert!(text.contains("changed after sign-off"), "{text}");
        assert!(text.contains("COVERAGE OWED"), "{text}");
    }

    // -- [2026] VJS-CA-VDS 1 orders 8, 9 and Schedule B 1 ---------------------

    /// ORDER 8's seed (R14, the THIRD side). The comparator is rewritten after
    /// the comparison, and the agreement rows it produced are now an assertion
    /// by a program that no longer exists. Both the other two hashes still
    /// match, so nothing else in the limb notices.
    #[test]
    fn a_comparator_that_moved_after_the_comparison_stales_the_binding() {
        let h = Harness::new();
        signed(&h);
        h.geometry_authority(
            "KEY",
            "1:2",
            &[(SurfaceKind::Radius, AgreementState::Agrees)],
        );
        h.write(
            "scripts/geometry-comparator.py",
            "# rewritten after the comparison ran\n",
        );
        let (outcome, text) = run_kind(&h, ProofKind::Geometry);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("COMPARATOR side"), "{text}");
        assert!(text.contains("The binding expired"), "{text}");
    }

    #[test]
    fn a_comparator_that_cannot_be_read_stales_the_binding() {
        let h = Harness::new();
        signed(&h);
        h.geometry_authority(
            "KEY",
            "1:2",
            &[(SurfaceKind::Radius, AgreementState::Agrees)],
        );
        std::fs::remove_file(h.root().join("scripts/geometry-comparator.py")).unwrap();
        let (outcome, text) = run_kind(&h, ProofKind::Geometry);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("cannot be read"), "{text}");
    }

    /// ORDER 9's seed (W3). The comparator is the one input the engine cannot
    /// falsify, so until it is pinned with its own negative control this limb
    /// claims no primacy and says so on every run. A warning, not a failure:
    /// the order carried the warning form and not the voiding form.
    #[test]
    fn an_unpinned_comparator_warns_that_the_limb_claims_no_primacy() {
        let h = Harness::new();
        signed(&h);
        h.geometry_authority(
            "KEY",
            "1:2",
            &[(SurfaceKind::Radius, AgreementState::Agrees)],
        );
        let (outcome, text) = run_kind(&h, ProofKind::Geometry);
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
        assert!(text.contains("NO WARRANT MAY CLAIM PRIMACY"), "{text}");
        assert!(text.contains("cannot falsify"), "{text}");
    }

    /// Schedule B 1's seed, built behind SUBMISSION-VDS-019: a comparator that
    /// finds NOTHING TO COMPARE records silence, and silence can never reach a
    /// conformance arm. Before the third state existed the only honest-looking
    /// value was `true`, and the proof recorded conformance against ground the
    /// frame never drew.
    #[test]
    fn a_dimension_the_frame_does_not_draw_is_no_authority_and_never_conformance() {
        let h = Harness::new();
        signed(&h);
        h.geometry_authority(
            "KEY",
            "1:2",
            &[
                (SurfaceKind::Radius, AgreementState::Agrees),
                (SurfaceKind::TypeScale, AgreementState::NotDrawn),
            ],
        );
        let (outcome, text) = run_kind(&h, ProofKind::Geometry);
        assert!(
            text.contains("surface_kind_not_drawn_by_the_frame"),
            "{text}"
        );
        assert!(text.contains("draws nothing on this dimension"), "{text}");
        assert!(text.contains("can never contribute conformance"), "{text}");
        // The row is NOT enforced: the bound row plus the one drawn dimension.
        assert_eq!(outcome.rows_enforced, 2, "{text}");
    }

    /// And a binding whose every row is silence is a binding nothing can fail.
    #[test]
    fn a_snapshot_whose_every_row_is_not_drawn_is_refused_as_vacuous() {
        let h = Harness::new();
        signed(&h);
        h.geometry_authority(
            "KEY",
            "1:2",
            &[
                (SurfaceKind::Radius, AgreementState::NotDrawn),
                (SurfaceKind::TypeScale, AgreementState::NotDrawn),
            ],
        );
        let (outcome, text) = run_kind(&h, ProofKind::Geometry);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("A binding over silence"), "{text}");
        // The bound row still enforced; no authority row did.
        assert_eq!(outcome.rows_enforced, 1, "{text}");
    }

    #[test]
    fn an_edited_snapshot_is_refused_by_its_own_digest() {
        let h = Harness::new();
        signed(&h);
        let path = h.geometry_authority(
            "KEY",
            "1:2",
            &[(SurfaceKind::Radius, AgreementState::Disagrees)],
        );
        let original = std::fs::read_to_string(&path).unwrap();
        let edited = original.replace("agrees: disagrees", "agrees: agrees");
        assert_ne!(edited, original, "the seed did not change the snapshot");
        std::fs::write(&path, &edited).unwrap();
        let (outcome, text) = run_kind(&h, ProofKind::Geometry);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("edited after it was generated"), "{text}");
    }
}

#[cfg(test)]
mod reading_integrity_tests {
    use crate::testing::{Harness, run_kind};
    use vds_core::{EXIT_VIOLATION, ProofKind, ProofStatus, ReadFrom, SurfaceKind};

    /// R10, and it is the rule that makes the other nine worth running.
    ///
    /// `geometry` reads the reading as its ONLY measurement. The seed here is
    /// the realistic attack and it is not an attack at all: somebody opens the
    /// YAML, sees a count over the bound, and edits the count. No surface
    /// changes, no gate is touched, and before this rule the proof went from
    /// failed to passed in silence.
    #[test]
    fn geometry_refuses_a_reading_that_was_edited_after_it_was_generated() {
        let h = Harness::new();
        h.geometry_bound(
            SurfaceKind::Radius,
            30,
            &[("2026-06-01", 667), ("2026-07-20", 561)],
        );
        let path = h.geometry_reading(
            "2026-07-31",
            ReadFrom::ShippedStylesheet,
            // 600 non-compliant against a bound of 561: failing, correctly.
            &[(SurfaceKind::Radius, 900, 600, 0)],
        );

        // Confirm it really does fail BEFORE the edit, or the test proves that a
        // passing thing still passes.
        let (before, text) = run_kind(&h, ProofKind::Geometry);
        assert_eq!(before.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("39 over the bound"), "{text}");

        // The edit: 600 becomes 500, comfortably inside the bound.
        let original = std::fs::read_to_string(&path).unwrap();
        let edited = original.replace("nonCompliant: 600", "nonCompliant: 500");
        assert_ne!(edited, original, "the seed did not change the reading");
        std::fs::write(&path, &edited).unwrap();

        let (after, text) = run_kind(&h, ProofKind::Geometry);
        assert_eq!(
            after.exit_code, EXIT_VIOLATION,
            "an edited reading must not buy a pass: {text}"
        );
        assert_eq!(after.status, ProofStatus::Failed);
        assert!(text.contains("edited after it was generated"), "{text}");
        // And it must not report the ORIGINAL violation either. The proof no
        // longer knows what the count is, and saying "39 over the bound" would
        // be quoting a number it has just declared unreliable.
        assert!(!text.contains("39 over the bound"), "{text}");
        // No row enforced: nothing was measured.
        assert_eq!(after.rows_enforced, 0, "{text}");
    }

    #[test]
    fn an_unedited_reading_passes_r10_without_comment() {
        let h = Harness::new();
        h.geometry_bound(
            SurfaceKind::Radius,
            30,
            &[("2026-06-01", 667), ("2026-07-20", 561)],
        );
        h.geometry_reading(
            "2026-07-31",
            ReadFrom::ShippedStylesheet,
            &[(SurfaceKind::Radius, 900, 540, 0)],
        );
        let (outcome, text) = run_kind(&h, ProofKind::Geometry);
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
        assert!(!text.contains("edited after it was generated"), "{text}");
    }
}
