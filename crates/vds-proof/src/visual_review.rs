//! The `visual_review` proof. Automated eyes, held to their own hashes.
//!
//! Draft S-7D, ENACTMENT PENDING (SUBMISSION-VDS-016 and -017). The fifteenth
//! kind, and the one this whole lane exists for: a migration shipped
//! structurally-green pages that looked nothing like their frames, under
//! twenty-eight source-side gates none of which read the ARTEFACT against the
//! FRAME. The review itself - render, screenshot, export, compare - is a
//! pipeline in the consuming repo; what this proof owns is the VERDICT RECORD,
//! and the three ways a verdict silently stops being true:
//!
//!   - the shipped side moved (the route's source no longer digests to what
//!     was reviewed),
//!   - the frame side moved (the frame no longer digests to what was
//!     reviewed),
//!   - the AUTHORITY moved (no sign-off row matches the frame's current hash).
//!
//! # Authority states, per the constitutional direction
//!
//! The signed-off Figma DEFINES taste; taste is exercised once, at frame
//! sign-off. So every row here lands in one of three states - conform,
//! deviate, no_authority - and the third is DISTINCT: never green, never red,
//! reported as coverage owed. A frame that is unsigned, parked, a proposal, or
//! changed since sign-off carries no authority, and a proof cannot claim
//! conformance against it. There is deliberately NO ACCEPTANCE STATE: an
//! addition the frame omits is a deviation exactly like a missing element,
//! and the resolution path is a proposed redraw closed by a NEW sign-off row,
//! never an engine-side excusal.
//!
//! # The rules
//!
//! One row is one review record, plus one row per redraw record.
//!
//!   R1  the reviewed route is not in the screens ledger. The subject is
//!       unknown; fatal.
//!   R2  the shipped side is STALE: the route's current source digest differs
//!       from the one reviewed. Fatal and named - a stale verdict that stayed
//!       green is the 561-pin-561 instrument again.
//!   R3  the frame side is STALE: the frame's current content digest differs
//!       from the one reviewed. Fatal and named.
//!   R4  a CONFORM verdict against a frame with no current authority. Refused
//!       at validation: conformance to nothing is not a finding, it is the
//!       word "green" wearing a hash.
//!   R5  a DEVIATE verdict. Fatal, naming every delta: a recorded deviation
//!       stays red until the design resolves it (see R8) and the route is
//!       re-reviewed.
//!   R6  a verdict and its deltas disagree: deviate with none, or conform
//!       with some. An incoherent record proves nothing; fatal.
//!   R7  two review records for one route. The engine keeps history, but two
//!       LIVE verdicts for one subject is two answers; the newest governs and
//!       older records must be superseded out of enforceable status.
//!       (Enforced structurally: the newest `reviewed_at` per route is the
//!       row; older ones are skipped and counted.)
//!   R8  a redraw whose status is `signed` without a covering sign-off row:
//!       `resolved_by` absent, naming a missing row, naming another frame, or
//!       naming a sign-off whose hash is not the frame's CURRENT hash. The
//!       band comes back through the design, never through the word "signed".
//!   R13 a CONFORM verdict that earned nothing: no screenshot on disk to
//!       re-hash, a screenshot whose bytes are not the ones reviewed, no
//!       served build, or a checklist in which no band was examined. Fatal in
//!       every authority state, because it is a defect in the RECORD and
//!       curable by the recorder: three of the four were demonstrated on the
//!       downstream estate by MINTING a passing record, and each alone
//!       conjured a route at parity out of nothing.
//!   R14 a reviewer examined a band the SCREEN RECORD does not declare. A
//!       finding about a rail on a screen that has none is a finding about
//!       another screen.
//!   W7  the band correspondence COULD NOT RUN: no screen record for the
//!       route, or one that declares no bands. Counted and reported per route
//!       and summarised in a note, because a rule that cannot run must not
//!       read as a rule that ran - on the estate this was written for, the
//!       same check could only run on 17 rows of 160.
//!   W1  a review whose frame carries NO AUTHORITY (and whose verdict honestly
//!       says `no_authority`, or predates the sign-off register). Coverage
//!       owed: the route has eyes on it and nothing to hold it to.
//!   W2  an open redraw (`proposed` or `drawn`): named so the owed design
//!       work is visible on every run.

use std::collections::BTreeMap;
use std::io::Write;

use vds_core::{
    AuthorityVerdict, FrameAuthority, ProofKind, RedrawStatus, Result, Violation, frame_authority,
};

use crate::ProofContext;
use crate::run::{Outcome, Verdict};

pub const GATE: &str = "crates/vds-proof/src/visual_review.rs";

const RULE_UNKNOWN_ROUTE: &str = "draft S-7D visual_review R1: the reviewed route is unknown";
const RULE_SHIPPED_STALE: &str =
    "draft S-7D visual_review R2: the shipped side moved; the verdict is STALE";
const RULE_FRAME_STALE: &str =
    "draft S-7D visual_review R3: the frame side moved; the verdict is STALE";
const RULE_CONFORM_UNSIGNED: &str =
    "draft S-7D visual_review R4: no conformance claim against an unsigned frame";
const RULE_DEVIATES: &str =
    "draft S-7D visual_review R5: the shipped surface deviates from the signed frame";
const RULE_INCOHERENT: &str =
    "draft S-7D visual_review R6: a verdict and its deltas may not disagree";
const RULE_REDRAW_UNCOVERED: &str =
    "draft S-7D visual_review R8: a redraw is resolved only by a covering sign-off row";
const RULE_NO_AUTHORITY: &str =
    "draft S-7D visual_review W1: no authority; coverage owed, never green, never red";
const RULE_REDRAW_OPEN: &str = "draft S-7D visual_review W2: an open redraw is owed to the design";
const RULE_GROUPED: &str = "draft S-7D(7) visual_review R9: a verdict naming more than one route confers coverage \
     on none of them";
const RULE_REGIONS: &str = "draft S-7D(8) visual_review R10: a verdict records WHICH regions of the pair were \
     examined";
const RULE_NEVER_REVIEWED: &str = "draft S-7D(9) visual_review W3: an enumerated route with no verdict is COVERAGE OWED, \
     named";
const RULE_OUTSIDE_MANIFEST: &str =
    "draft S-7D(9) visual_review W4: a reviewed route the manifest does not enumerate";
const RULE_PARTIAL_DEPTH: &str =
    "draft S-7D(8) visual_review W5: partial study must read as partial study";
const RULE_PARK_UNCOVERED: &str = "[2026] VJS-CA-VDS 1 order 27, visual_review R12: a park is lawful only under a live \
     direction row";
const RULE_PARKED: &str = "[2026] VJS-SC-OPBOX 1 order 29, visual_review W6: parked under a registered direction; \
     reported, never counted a violation";
const RULE_CONTRACT_CITATION: &str = "draft S-7D(10) visual_review R11: a stage-4 verdict cites the stage-1 contract version \
     it was taken against";
const RULE_UNEARNED_CONFORM: &str = "draft S-7D(12) visual_review R13: a conform verdict EARNS its parity claim, or it is not \
     one";
const RULE_BAND_CORRESPONDENCE: &str = "draft S-7D(14) visual_review R14: a reviewer's examined band is one the screen record \
     declares";
const RULE_CORRESPONDENCE_UNRUNNABLE: &str = "draft S-7D(14) visual_review W7: the band correspondence COULD NOT RUN on this route, \
     which is not the same answer as a pass";

/// The four-stage pipeline, stated on every run, and the sentence that would
/// have prevented the estate's failure.
const PIPELINE_NOTE: &str = "[pipeline] This kind is STAGE 4 of four, in fixed order: (1) the \
                             machine-readable contract derived from signed frames, (2) Figma \
                             AND code both built to that contract, (3) source-side gates per \
                             push, (4) this artefact-side visual check per route. STAGE 3 AND \
                             STAGE 4 ARE NOT SUBSTITUTES: gates read SOURCE and cannot see the \
                             page; the visual check reads the PAGE and is slow and sampled. \
                             Neither may be cited as covering the other's ground. On the \
                             motivating estate, 28 green stage-3 gates were read as evidence \
                             about stage 4, and the pages looked nothing like their frames.";

/// Refuse a run that considered fewer rows than it enumerated.
///
/// The instrument checking itself, and it is not ceremony: every population
/// this proof reports is derived from one walk over the enumeration, so a bug
/// that skipped an entry would shrink the denominator and the numerator
/// together, and the coverage report would look perfect while covering less.
/// Counting the enumeration independently and comparing is the only way that
/// failure is visible from inside.
fn refuse_under_enumeration(considered: usize, enumerated: usize) -> Result<()> {
    if considered < enumerated {
        return Err(vds_core::VdsError::precondition(format!(
            "this run enumerated {enumerated} route(s) from the manifest and reported on \
             only {considered}. The {} missing row(s) are routes the proof was supposed to \
             account for and did not, which is the exact defect this kind exists to close - \
             an unreported route reads identically to a route with nothing wrong. The run is \
             REFUSED rather than published, because a coverage report that quietly covers \
             less than it claims is worse than no coverage report.",
            enumerated - considered
        )));
    }
    Ok(())
}

/// Stated on every run, passing or not: the engine's boundary, and the repeal.
const RESERVED_NOTE: &str = "[reserved] This kind validates, stores and stales VERDICT RECORDS. \
                             The capture and review pipeline (render, screenshot, export, \
                             compare) lives in the consuming repo: a proof may not call a \
                             network or a model (VDS S-7(2)(1)). The verdict vocabulary is \
                             conform | deviate | no_authority, and there is NO ACCEPTANCE \
                             STATE: an addition the frame omits is a deviation exactly like a \
                             missing element, and its resolution path is a new signed frame \
                             version recorded as a redraw, never an engine-side excusal. A \
                             delta's DISPOSITION classifies a difference and never disposes of \
                             it: the taxonomy carries no `accepted` and no `wont_fix`, because \
                             either would be a fourth verdict wearing a different field name, \
                             reachable by the recorder rather than by the signer. \
                             BEFORE a surface's frame is entered in the sign-off register \
                             this kind REPORTS AND NEVER BLOCKS ([2026] VJS-SC-OPBOX 1 \
                             orders 20, 23 and 24): registration is the moment a surface \
                             flips from report to block, and there is no estate-wide flag \
                             day.";

pub fn run(ctx: &ProofContext, out: &mut dyn Write) -> Result<Outcome> {
    let project = ctx.project;
    let store = ctx.store();
    let mut run = ctx.new_run(ProofKind::VisualReview, GATE);
    run.note(RESERVED_NOTE);

    let reviews = store.read_reviews()?;
    let redraws = store.read_redraws()?;
    let signoffs = store.read_signoffs()?;
    for located in &reviews {
        run.input_file(&located.path)?;
    }
    for located in &redraws {
        run.input_file(&located.path)?;
    }
    for located in &signoffs {
        run.input_file(&located.path)?;
    }
    let signoffs: Vec<vds_core::SignOff> = signoffs.into_iter().map(|l| l.value).collect();
    let directions: Vec<vds_core::DirectionRecord> = {
        let located = store.read_directions()?;
        for row in &located {
            run.input_file(&row.path)?;
        }
        located.into_iter().map(|l| l.value).collect()
    };

    // The two ledgers the staleness rules read. The screens ledger is loaded
    // FRESH and only where a review exists to hold against it: comparing a
    // verdict's hash to a stale ledger would answer the staleness question
    // with stale evidence. The frames ledger is verified against its own
    // digest for the same reason.
    let screens = if reviews.is_empty() {
        None
    } else {
        Some(vds_scan::load_fresh(project)?)
    };
    if let Some(ledger) = &screens {
        run.input_named("screens-ledger", ledger.content_digest.clone());
    }
    let frames = vds_figma::frames::read(project)?;
    if let Some(ledger) = &frames {
        vds_figma::frames::check_fresh(ledger, None)?;
        run.input_named("frames-ledger", ledger.content_digest.clone());
    }

    // The SCREEN REGISTER, read here rather than in a sixteenth kind. This
    // proof already reads every input the band correspondence needs, and it
    // already walks the coverage denominator once; a second kind would walk it
    // again and the two could disagree about which routes exist, which is the
    // two-sources-of-truth failure the manifest limb exists to prevent.
    let screen_records = store.read_screens()?;
    for located in &screen_records {
        run.input_file(&located.path)?;
    }

    // The ESTATE's enumeration. Read before any verdict is looked at, because
    // the population this proof reports on is the estate's list and not the
    // set of records that happen to exist: a run that reports only on the
    // reviews it holds cannot distinguish "reviewed and clean" from "never
    // looked at", which is precisely the defect (draft S-7D(9)).
    let manifest = vds_core::read_route_manifest(project)?;
    if let Some(manifest) = &manifest {
        run.input_file(&vds_core::route_manifest_path(project))?;
        if let Some(why) = manifest.untrustworthy_because()? {
            return Err(vds_core::VdsError::precondition(format!(
                "the route manifest cannot be relied on, so this proof cannot say what it \
                 was supposed to cover, and a coverage report over an unreliable \
                 enumeration is worse than none: {why}"
            )));
        }
    }

    // PHASE A: validate every record, and decide which ones confer coverage.
    //
    // R9/R10 live here rather than inside the per-route pass, because an
    // invalid record must not be able to occupy a route's coverage slot: a
    // family verdict that took the slot would leave the route reading as
    // covered, which is the whole defect this amendment closes.
    let mut live: BTreeMap<&str, &vds_core::VisualReviewRecord> = BTreeMap::new();
    for located in &reviews {
        let review = &located.value;
        let location = format!("{} [{}]", review.id, review.routes.join(" + "));

        let defects = review.defects();
        if !defects.is_empty() {
            run.row(Verdict::Skipped("invalid_record_confers_no_coverage"));
            for defect in defects {
                run.fail(Violation::fatal(
                    location.clone(),
                    if review.routes.len() > 1 {
                        RULE_GROUPED
                    } else {
                        RULE_REGIONS
                    },
                    "one route per verdict, and a non-empty region checklist in which every \
                     row carries a finding",
                    defect,
                ));
            }
            continue;
        }

        // A valid record names exactly one route: `defects` refused every other
        // shape above, so this cannot be `None`.
        let Some(route) = review.covered_route() else {
            continue;
        };
        match live.get(route) {
            // R7: the newest record per route governs; older ones are history.
            Some(existing) if existing.reviewed_at.as_str() >= review.reviewed_at.as_str() => {
                run.row(Verdict::Skipped("superseded_by_a_newer_review"));
            }
            Some(existing) => {
                run.row(Verdict::Skipped("superseded_by_a_newer_review"));
                let _ = existing;
                live.insert(route, review);
            }
            None => {
                live.insert(route, review);
            }
        }
    }

    // PHASE B: one row per ENUMERATED route. The enumeration is the manifest
    // where the estate supplied one, plus any route a live verdict names that
    // the manifest does not carry (which is warned, not swallowed).
    let manifest_routes: Vec<String> = match &manifest {
        Some(manifest) => {
            let mut routes = manifest.routes.clone();
            routes.sort();
            routes.dedup();
            routes
        }
        None => Vec::new(),
    };
    let mut enumerated: Vec<String> = manifest_routes.clone();
    for route in live.keys() {
        if !enumerated.iter().any(|r| r == route) {
            enumerated.push((*route).to_owned());
        }
    }
    enumerated.sort();
    enumerated.dedup();

    let mut population_current = 0u32;
    let mut population_stale = 0u32;
    let mut population_missing = 0u32;
    // The band correspondence's own two numbers, kept apart for the reason W7
    // exists: "found no foreign band" and "had no basis to look" are different
    // facts, and one number holding both reports the second as the first.
    let mut correspondence_ran = 0u32;
    let mut correspondence_unrunnable = 0u32;
    // Counted independently of `rows_considered` so the two can be compared:
    // a check that derives its own denominator from its own numerator cannot
    // fail (draft S-7D(9), and the vacuity discipline at VDS S-7(2)(4)).
    let mut manifest_rows_considered = 0usize;

    for route in &enumerated {
        let in_manifest = manifest_routes.iter().any(|r| r == route);
        if in_manifest {
            manifest_rows_considered += 1;
        }

        let Some(review) = live.get(route.as_str()).copied() else {
            // THE POPULATION THAT MATTERS. A route the estate enumerated and
            // no valid verdict covers is a NAMED LINE of coverage owed, never
            // an absence. Warned rather than failed, following
            // [2026] VJS-SC-OPBOX 1 Q3: no_authority neither blocks nor
            // licenses; coverage owed is that state made countable.
            population_missing += 1;
            run.row(Verdict::Skipped("no_verdict_coverage_owed"));
            run.warn(Violation::fatal(
                route.clone(),
                RULE_NEVER_REVIEWED,
                "one route-scoped visual verdict covering this route",
                "NEVER REVIEWED: the estate enumerates this route and no valid verdict \
                 covers it. This is COVERAGE OWED, and it is named rather than counted \
                 because a family-level pass once scored a route it had not studied and \
                 nothing recorded that it had not."
                    .to_owned(),
            ));
            continue;
        };
        let location = format!("{} [{}]", review.id, route);

        if !in_manifest {
            run.warn(Violation::fatal(
                location.clone(),
                RULE_OUTSIDE_MANIFEST,
                "every reviewed route to appear in the estate's route manifest",
                format!(
                    "{route} carries a verdict and the manifest does not enumerate it. The \
                     verdict is still checked; what is unknown is whether the manifest is \
                     narrower than the estate."
                ),
            ));
        }

        // Triage, made visible as triage: a verdict that examined two bands
        // must not read like one that examined seven.
        let unaccounted = review.unaccounted_regions();
        if review.examined_count() < review.regions.len() || !unaccounted.is_empty() {
            run.warn(Violation::fatal(
                location.clone(),
                RULE_PARTIAL_DEPTH,
                "every region of the closed set examined, or recorded as not examined with \
                 its reason",
                format!(
                    "{} of {} listed region(s) examined{}. Depth is on the face of the \
                     record so that partial study reads as partial study.",
                    review.examined_count(),
                    review.regions.len(),
                    if unaccounted.is_empty() {
                        String::new()
                    } else {
                        format!(
                            ", and {} region(s) are accounted for in neither direction: {}",
                            unaccounted.len(),
                            unaccounted
                                .iter()
                                .map(|r| r.to_string())
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    }
                ),
            ));
        }

        // R6 first: an incoherent record proves nothing, whatever the hashes say.
        let incoherent = match review.verdict {
            AuthorityVerdict::Deviate if review.deltas.is_empty() => Some(
                "the verdict is deviate and the delta list is empty. A deviation naming \
                 nothing names no work, and cannot be resolved by any redraw."
                    .to_owned(),
            ),
            AuthorityVerdict::Conform if !review.deltas.is_empty() => Some(format!(
                "the verdict is conform and {} delta(s) are recorded. A conform verdict \
                 listing differences is two answers in one record.",
                review.deltas.len()
            )),
            _ => None,
        };
        if let Some(actual) = incoherent {
            run.row(Verdict::Enforced);
            run.fail(Violation::fatal(
                location.clone(),
                RULE_INCOHERENT,
                "a verdict whose delta list agrees with it",
                actual,
            ));
            continue;
        }

        // ORDER 20: the frame's authority is resolved BEFORE the staleness
        // rules, and where it is Unsigned they WARN AND SKIP rather than fail.
        // Pre-registration those rules protect nothing - their whole purpose
        // is to stop a stale GREEN, and no green is available while R4 refuses
        // conform against an unsigned frame - so a fatal R2/R3 there is pure
        // blocking cost at zero protective value, on a surface
        // [2026] VJS-SC-OPBOX 1 order 24 holds nothing may block on. R1 and R6
        // stay fatal in every authority state: they are defects in the RECORD,
        // curable by the recorder, and not facts about any surface.
        let current_frame_early = frames
            .as_ref()
            .filter(|l| l.file_key == review.file_key)
            .and_then(|l| l.row(&review.node_id))
            .and_then(|r| r.content_digest.as_ref());
        let signed = frame_authority(
            &review.file_key,
            &review.node_id,
            current_frame_early,
            &signoffs,
        )
        .is_signed();

        // R1/R2: the shipped side.
        let current_source = screens
            .as_ref()
            .and_then(|l| l.screens.iter().find(|s| &s.route == route))
            .map(|s| &s.digest);
        let Some(current_source) = current_source else {
            run.row(Verdict::Enforced);
            run.fail(Violation::fatal(
                location.clone(),
                RULE_UNKNOWN_ROUTE,
                "the reviewed route to appear in the screens ledger",
                format!(
                    "{route} is not in the screens ledger{}. A verdict about a route the \
                     declared surface does not carry is a verdict about nothing this project \
                     ships.",
                    if screens.is_none() {
                        " (no ledger has been generated; run `vds ledger screens`)"
                    } else {
                        ""
                    }
                ),
            ));
            continue;
        };
        if current_source != &review.shipped_source_digest {
            if !signed {
                population_stale += 1;
                run.row(Verdict::Skipped("no_authority_verdict_stale"));
                run.warn(Violation::fatal(
                    location.clone(),
                    RULE_SHIPPED_STALE,
                    "a re-review once this surface's frame is registered",
                    format!(
                        "the verdict is STALE (the route's source moved) and the surface \
                         carries NO AUTHORITY, so the staleness is coverage owed and not a \
                         breach: before registration this kind reports and never blocks \
                         ([2026] VJS-SC-OPBOX 1 order 24; [2026] VJS-CA-VDS 1 order 20). \
                         {route} now digests to {current_source}."
                    ),
                ));
                continue;
            }
            run.row(Verdict::Enforced);
            run.fail(Violation::fatal(
                location.clone(),
                RULE_SHIPPED_STALE,
                format!(
                    "the route's source to still digest to {} (as reviewed)",
                    review.shipped_source_digest
                ),
                format!(
                    "it now digests to {current_source}. The shipped side moved after the \
                     review, so this verdict describes a page that no longer exists. STALE, \
                     visibly: re-run the capture and review pipeline for {route}."
                ),
            ));
            population_stale += 1;
            continue;
        }

        // R3: the frame side.
        let current_frame = frames
            .as_ref()
            .filter(|l| l.file_key == review.file_key)
            .and_then(|l| l.row(&review.node_id))
            .and_then(|r| r.content_digest.as_ref());
        if let Some(current_frame) = current_frame
            && current_frame != &review.frame_digest
        {
            if !signed {
                population_stale += 1;
                run.row(Verdict::Skipped("no_authority_verdict_stale"));
                run.warn(Violation::fatal(
                    location.clone(),
                    RULE_FRAME_STALE,
                    "a re-review once this surface's frame is registered",
                    format!(
                        "the verdict is STALE (the frame moved to {current_frame}) and the \
                         surface carries NO AUTHORITY, so the staleness is coverage owed and \
                         not a breach ([2026] VJS-CA-VDS 1 order 20)."
                    ),
                ));
                continue;
            }
            run.row(Verdict::Enforced);
            run.fail(Violation::fatal(
                location.clone(),
                RULE_FRAME_STALE,
                format!(
                    "the frame {}/{} to still digest to {} (as reviewed)",
                    review.file_key, review.node_id, review.frame_digest
                ),
                format!(
                    "it now digests to {current_frame}. The frame moved after the review, \
                     so this verdict compares the page to a drawing that no longer exists. \
                     STALE, visibly: re-review against the current frame once it is signed."
                ),
            ));
            population_stale += 1;
            continue;
        }

        // R11: THE STAGE LINK. A stage-4 verdict cites the stage-1 contract
        // version it was taken against, and the citation is CHECKED against
        // the register: the row must exist, sign THIS frame, and carry the
        // hash the review says it compared to. Unchecked, the citation is a
        // field, and a field is how "we checked it" survives a contract change
        // with nobody noticing.
        if let Some(cited) = &review.contract_signoff {
            let contract_defect = match signoffs.iter().find(|s| &s.id == cited) {
                None => Some(format!(
                    "cites contract {cited}, which does not exist in the sign-off register."
                )),
                Some(signoff)
                    if signoff.file_key != review.file_key || signoff.node_id != review.node_id =>
                {
                    Some(format!(
                        "cites contract {cited}, which signs {}/{}, not the frame this \
                         verdict was rendered over ({}/{}).",
                        signoff.file_key, signoff.node_id, review.file_key, review.node_id
                    ))
                }
                Some(signoff) if signoff.frame_digest != review.frame_digest => Some(format!(
                    "cites contract {cited}, whose signed hash is {} while the verdict says \
                     it compared against {}. The verdict was rendered against a different \
                     version of the contract from the one it names.",
                    signoff.frame_digest, review.frame_digest
                )),
                Some(_) => None,
            };
            if let Some(actual) = contract_defect {
                run.row(Verdict::Enforced);
                run.fail(Violation::fatal(
                    location.clone(),
                    RULE_CONTRACT_CITATION,
                    "the cited stage-1 contract to exist, to sign this frame, and to carry \
                     the hash the verdict compared against",
                    actual,
                ));
                continue;
            }
        }

        // R13: A CONFORM VERDICT EARNS ITS CLAIM, OR IT IS NOT ONE.
        //
        // Sited with R11 rather than in the conform arm below, and fatal in
        // every authority state, for the reason `a_record_level_defect_stays_
        // fatal_on_an_unsigned_surface` holds: order 20 degrades the rules that
        // are FACTS ABOUT A SURFACE, and a minted record is not one. It is a
        // defect in the RECORD, curable by the recorder.
        //
        // The whole conform branch below used to be one line marking the row
        // enforced, so a record naming no screenshot, naming no build and
        // examining no band read exactly like a route at parity - and three of
        // the four conditions were demonstrated downstream by writing such a
        // record, each one alone enough.
        let screenshot_rehashed = match &review.screenshot_path {
            None => false,
            Some(relative) => {
                let path = project.root.join(relative);
                if path.is_file() {
                    // Recorded as an INPUT as well as re-hashed, so the
                    // evidence digest witnesses the picture the verdict rests
                    // on: a warrant citing this run then cites the screenshot,
                    // and a screenshot swapped after the fact moves the run.
                    run.input_file(&path)?;
                    vds_core::Digest::of_file(&path)? == review.shipped_screenshot_digest
                } else {
                    false
                }
            }
        };
        let unearned = review.unearned_conformance(screenshot_rehashed);
        if !unearned.is_empty() {
            run.row(Verdict::Enforced);
            for actual in unearned {
                run.fail(Violation::fatal(
                    location.clone(),
                    RULE_UNEARNED_CONFORM,
                    "a conform verdict to name a screenshot that still re-hashes to the digest \
                     it records, to name the build that screenshot was taken of, and to have \
                     examined at least one band",
                    format!("UNEARNED CONFORMANCE: the record {actual}"),
                ));
            }
            continue;
        }

        // R14 and W7: THE BAND CORRESPONDENCE, and the declaration it needs.
        //
        // Asked in two steps, and the order is the rule. First: CAN this run?
        // A route with no screen record, or one that declares no bands, has no
        // basis for the comparison, and that is reported as a rule that DID NOT
        // RUN rather than as a rule that found nothing. Only then: did the
        // reviewer study a band this screen does not have?
        //
        // The key is the screen record's `bands` and never its `regions`: the
        // region vocabulary is deliberately OPEN (it is the subject's own, and
        // closing it would make VDS the authority on what a screen is made of),
        // and an open set cannot refuse a name that is not a band.
        let screen = screen_records
            .iter()
            .map(|located| &located.value)
            .find(|screen| screen.route == *route);
        let unrunnable = match screen {
            None => Some(format!(
                "no screen record names {route}, so nothing declares which bands this screen \
                 has and the reviewer's checklist can be compared to nothing. Register the \
                 screen with `vds screen add --route {route} --band <band>,<band>`."
            )),
            Some(screen) => screen.band_correspondence_unrunnable_because(),
        };
        match unrunnable {
            Some(why) => {
                correspondence_unrunnable += 1;
                run.row(Verdict::Skipped("region_correspondence_did_not_run"));
                run.warn(Violation::fatal(
                    location.clone(),
                    RULE_CORRESPONDENCE_UNRUNNABLE,
                    "a screen record for this route declaring the bands it has",
                    format!(
                        "REGION CORRESPONDENCE DID NOT RUN on {route}: {why} This route's \
                         checklist is therefore unchecked against the screen, and that is a \
                         gap in what this run measured, not a clean result."
                    ),
                ));
            }
            None => {
                let screen = screen.expect("a runnable correspondence has a screen record");
                correspondence_ran += 1;
                let examined = review.examined_regions();
                let foreign = screen.bands_not_drawn(&examined);
                if !foreign.is_empty() {
                    // A row here and none on the clean path, exactly as R1, R6
                    // and R11 do it: this route already gets its row from the
                    // verdict below, and adding a second enforced row for a
                    // rule that found nothing would put a GREEN row on a
                    // surface whose frame may carry no authority at all. The
                    // reach of the rule is reported in the note instead.
                    //
                    // Fatal in every authority state, with R1/R6/R11: a
                    // checklist claiming study of a band nothing says the
                    // screen has is a defect in the RECORD, curable by the
                    // recorder, and not a fact about the surface. Order 20
                    // degrades the rules that ARE facts about a surface.
                    run.row(Verdict::Enforced);
                    run.fail(Violation::fatal(
                        location.clone(),
                        RULE_BAND_CORRESPONDENCE,
                        format!(
                            "every EXAMINED band to be one {} declares: {}",
                            screen.id,
                            screen
                                .arrangement
                                .bands
                                .iter()
                                .map(|b| b.to_string())
                                .collect::<Vec<String>>()
                                .join(", ")
                        ),
                        format!(
                            "the reviewer examined {} band(s) this screen does not have: {}. A \
                             finding about a band the screen does not draw is a finding about \
                             another screen, and it reads in the ledger as depth. The rule \
                             reads STUDY and not paperwork: a row recorded `examined: false` \
                             is a lawful answer about a band and is never foreign.",
                            foreign.len(),
                            foreign
                                .iter()
                                .map(|b| b.to_string())
                                .collect::<Vec<String>>()
                                .join(", ")
                        ),
                    ));
                }
            }
        }

        // Neither side moved and the contract citation resolves: this verdict
        // still describes the pair it was rendered over, against the contract
        // version it names.
        population_current += 1;

        // The authority question, from the sign-off register and the frame's
        // CURRENT hash. Never from trust.
        let authority =
            frame_authority(&review.file_key, &review.node_id, current_frame, &signoffs);

        match (&authority, review.verdict) {
            // R4: a conformance claim against an unsigned frame is refused at
            // validation. It is the one combination that could smuggle taste
            // back downstream: green against nothing.
            (FrameAuthority::Unsigned { because }, AuthorityVerdict::Conform) => {
                run.row(Verdict::Enforced);
                run.fail(Violation::fatal(
                    location.clone(),
                    RULE_CONFORM_UNSIGNED,
                    "conformance claimed only against a frame whose CURRENT hash matches a \
                     sign-off row",
                    format!(
                        "the verdict is conform and the frame carries no authority: \
                         {because} A proof cannot claim conformance against an unsigned \
                         frame; the row is no_authority until the frame is signed."
                    ),
                ));
            }
            // no_authority: DISTINCT. Never green, never red. Coverage owed.
            (FrameAuthority::Unsigned { because }, _) => {
                run.row(Verdict::Skipped("no_authority_frame_unsigned"));
                run.warn(Violation::fatal(
                    location.clone(),
                    RULE_NO_AUTHORITY,
                    "a sign-off row matching the frame's current content hash",
                    format!(
                        "no_authority: {because} The route has eyes on it and nothing to \
                         hold it to; this is COVERAGE OWED, and it is neither a pass nor a \
                         failure."
                    ),
                ));
            }
            (FrameAuthority::Signed { .. }, AuthorityVerdict::NoAuthority) => {
                // The record says no-authority and the register says signed:
                // the record predates the sign-off or refused stale knowledge.
                // Not a pass - the reviewer held the page to nothing - and not
                // a deviation either. Re-review.
                run.row(Verdict::Skipped("verdict_predates_the_sign_off"));
                run.warn(Violation::fatal(
                    location.clone(),
                    RULE_NO_AUTHORITY,
                    "a verdict rendered against the now-signed frame",
                    "the record says no_authority and the frame is now signed at its \
                     current hash. The review held the page to nothing; re-run it against \
                     the signed frame."
                        .to_owned(),
                ));
            }
            // R5: a deviation against signed authority. Red, and every delta
            // is named: taste was exercised at sign-off, so a difference is a
            // difference, whichever direction it points.
            (FrameAuthority::Signed { signoff }, AuthorityVerdict::Deviate) => {
                run.row(Verdict::Enforced);
                run.fail(Violation::fatal(
                    location.clone(),
                    RULE_DEVIATES,
                    format!(
                        "the shipped surface to match the frame signed under {signoff} \
                         (screenshot {} against frame image {})",
                        review.shipped_screenshot_digest, review.frame_image_digest
                    ),
                    format!(
                        "{} deviation(s), reviewed by {} at {}: {}. An addition the frame \
                         omits is a deviation exactly like a missing element. Each delta \
                         carries a DISPOSITION saying what KIND of difference it is, and no \
                         disposition closes one: there is no `accepted` and no `wont_fix`, \
                         because an acceptance state is taste exercised after sign-off and \
                         taste is exercised once, AT sign-off. THIS FINDING \
                         CLASSIFIES AND DOES NOT DISPOSE: it is a docket entry and never an \
                         execution order, the difference remains LIVE pending disposition, \
                         no instrument may auto-remove, auto-hide or unrender the surface on \
                         the strength of it, and removal is governed exclusively by \
                         [2026] VJS-CC-OPBOX 155 O7, unimpaired ([2026] VJS-SC-OPBOX 1 order \
                         21). It resolves by exactly three routes: (i) a covering sign-off \
                         adopting it, (ii) an express registered direction parking it, or \
                         (iii) a deletion that independently discharges O7 by an \
                         informed-deletion signature reciting the live function destroyed or \
                         proving it dead or homed. Its sole automatic consequence is a \
                         proposed redraw.",
                        review.deltas.len(),
                        review.reviewed_by,
                        review.reviewed_at.as_str(),
                        review
                            .deltas
                            .iter()
                            .map(|delta| delta.to_string())
                            .collect::<Vec<String>>()
                            .join("; ")
                    ),
                ));
            }
            (FrameAuthority::Signed { .. }, AuthorityVerdict::Conform) => {
                run.row(Verdict::Enforced);
            }
        }
    }

    // The redraw rows: R8, W2.
    for located in &redraws {
        let redraw = &located.value;
        let location = format!("{} [{}/{}]", redraw.id, redraw.file_key, redraw.node_id);
        match redraw.status {
            RedrawStatus::Withdrawn => {
                run.row(Verdict::Skipped("redraw_withdrawn"));
            }
            // PARKED under a registered direction ([2026] VJS-CA-VDS 1 order
            // 27). Refused without a covering direction row in the same terms
            // `signed` is refused without a covering sign-off - the word is
            // not the row - and where the row stands, the surface is SKIPPED:
            // while the registered direction stands no gate may count it a
            // violation (SC-OPBOX 1 order 29).
            RedrawStatus::Parked => {
                let covering = redraw
                    .directed_by
                    .as_ref()
                    .and_then(|id| directions.iter().find(|d| &d.id == id));
                match covering {
                    None => {
                        run.row(Verdict::Enforced);
                        run.fail(Violation::fatal(
                            location.clone(),
                            RULE_PARK_UNCOVERED,
                            "a `parked` redraw to name a direction row whose decisionDigest                              still matches its log entry",
                            match &redraw.directed_by {
                                None => "status is `parked` and directedBy names no \
                                         direction row. A park is a PRINCIPAL DIRECTION \
                                         recorded at the register, and the word is not the \
                                         row."
                                    .to_owned(),
                                Some(id) => format!(
                                    "status is `parked` and directedBy names {id}, which                                      does not exist in the direction register."
                                ),
                            },
                        ));
                    }
                    Some(direction) => {
                        let current = vds_core::decision_log_digest(project, &direction.log_id);
                        match vds_core::direction_authority(direction, current.as_ref()) {
                            vds_core::FrameAuthority::Signed { .. } => {
                                run.row(Verdict::Skipped("parked_under_a_live_direction"));
                                run.warn(Violation::fatal(
                                    location.clone(),
                                    RULE_PARKED,
                                    "the frame record to converge on the directed state",
                                    format!(
                                        "PARKED under {} ({}): {} / {}. While the registered                                          direction stands no gate may count this a violation                                          ([2026] VJS-SC-OPBOX 1 order 29); the subject keeps                                          its render rights and the redraw duty stands.",
                                        direction.id,
                                        direction.log_id,
                                        direction.direction,
                                        direction.magnitude
                                    ),
                                ));
                            }
                            vds_core::FrameAuthority::Unsigned { because } => {
                                run.row(Verdict::Enforced);
                                run.fail(Violation::fatal(
                                    location.clone(),
                                    RULE_PARK_UNCOVERED,
                                    "the covering direction's decisionDigest to match its                                      log entry",
                                    format!(
                                        "the park rests on a direction that no longer                                          carries authority: {because}"
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
            RedrawStatus::Proposed | RedrawStatus::Drawn => {
                run.row(Verdict::Enforced);
                run.warn(Violation::fatal(
                    location.clone(),
                    RULE_REDRAW_OPEN,
                    "the proposed change to be drawn, signed, and re-reviewed",
                    format!(
                        "open ({}) since {}: {}. Deviation: {}. The band comes back through \
                         the design.",
                        redraw.status,
                        redraw.opened_at.as_str(),
                        redraw.proposed,
                        redraw.deviation
                    ),
                ));
            }
            RedrawStatus::Signed => {
                run.row(Verdict::Enforced);
                let covering = redraw
                    .resolved_by
                    .as_ref()
                    .and_then(|id| signoffs.iter().find(|s| &s.id == id));
                let current_frame = frames
                    .as_ref()
                    .filter(|l| l.file_key == redraw.file_key)
                    .and_then(|l| l.row(&redraw.node_id))
                    .and_then(|r| r.content_digest.as_ref());
                let failure = match covering {
                    None => Some(match &redraw.resolved_by {
                        None => "status is `signed` and resolved_by names no sign-off row. \
                                 The word is not the row."
                            .to_owned(),
                        Some(id) => format!(
                            "status is `signed` and resolved_by names {id}, which does not \
                             exist in the sign-off register."
                        ),
                    }),
                    Some(signoff)
                        if signoff.file_key != redraw.file_key
                            || signoff.node_id != redraw.node_id =>
                    {
                        Some(format!(
                            "resolved_by names {}, which signs {}/{}, not this redraw's \
                             frame.",
                            signoff.id, signoff.file_key, signoff.node_id
                        ))
                    }
                    Some(signoff) => match current_frame {
                        None => Some(format!(
                            "resolved_by names {}, and the frame has no current content \
                             digest in the frames ledger, so the sign-off cannot be shown \
                             to cover what the frame now draws.",
                            signoff.id
                        )),
                        Some(current) if current != &signoff.frame_digest => Some(format!(
                            "resolved_by names {}, whose hash {} is not the frame's \
                             current hash {current}. The frame moved after that sign-off; \
                             the change is not covered.",
                            signoff.id, signoff.frame_digest
                        )),
                        Some(_) => None,
                    },
                };
                if let Some(actual) = failure {
                    run.fail(Violation::fatal(
                        location.clone(),
                        RULE_REDRAW_UNCOVERED,
                        "a `signed` redraw to name a sign-off row whose hash is the \
                         frame's CURRENT content hash",
                        actual,
                    ));
                }
            }
        }
    }

    // THE COVERAGE REPORT, in pipeline terms and over the estate's own
    // enumeration. Three populations, every one of them a count of ROUTES and
    // not of records: a run that reported on the reviews it happens to hold
    // could never distinguish "reviewed and clean" from "never looked at".
    match &manifest {
        Some(found) => {
            refuse_under_enumeration(manifest_rows_considered, manifest_routes.len())?;
            run.note(format!(
                "[stage-4 coverage] {} route(s) enumerated by the estate ({}): {} CURRENT (a \
                 verdict against the current artefact AND the current contract), {} OWED BY \
                 DRIFT (the code or the frame moved since the verdict, so the route is not \
                 reviewed - it is owed), {} NEVER REVIEWED (named above, one warning each). \
                 A route is covered at stage 4 only while all three hashes hold.",
                manifest_routes.len(),
                found.source,
                population_current,
                population_stale,
                population_missing
            ));
            if !found.does_not_cover.is_empty() {
                run.note(format!(
                    "[manifest] the estate states its enumeration does NOT cover: {}",
                    found.does_not_cover.join("; ")
                ));
            }
        }
        None => run.note(
            "[stage-4 coverage] UNKNOWN: no route manifest is supplied, so this run can \
             report on the verdicts it holds and CANNOT say what it was supposed to cover. \
             A never-reviewed route is invisible in this state, which is the defect the \
             manifest exists to close. Supply the estate's enumeration with `vds ledger \
             routes --from <list>`.",
        ),
    }
    // THE REACH OF THE BAND CORRESPONDENCE, on every run, passing or not.
    //
    // Load-bearing and not decoration: a rule that could not run must not read
    // as a rule that ran and found nothing. On the estate this lane was written
    // for, the same check could only run on 17 rows of 160, and a report that
    // printed one number would have described 143 unmeasured routes as clean.
    run.note(format!(
        "[region correspondence] R14 ran on {correspondence_ran} route(s) and COULD NOT RUN \
         on {correspondence_unrunnable} route(s), each named above. It runs only where a \
         screen record claims the route AND declares its bands, so a route missing either has \
         an UNCHECKED checklist rather than a clean one. The key is the screen record's \
         `bands` and never its `regions`: the region vocabulary is the subject's own and is \
         deliberately open, and an open set cannot refuse a name that is not a band.",
    ));
    run.note(PIPELINE_NOTE);

    if reviews.is_empty() && redraws.is_empty() && manifest.is_none() {
        run.note(
            "[scope] no visual review and no redraw is recorded, so every row is skipped \
             and this run is vacuous. That is the honest state of a project whose capture \
             pipeline has not run, and it is NOT evidence (VDS S-7(2)(4)).",
        );
    }

    run.finish(&ctx.capture_options()?, out)
}

#[cfg(test)]
mod unit_tests {
    use super::refuse_under_enumeration;

    /// The instrument checking itself. Seeded with the shape of the bug it
    /// exists to catch: the walk skipped an entry, so the numerator and the
    /// denominator shrank together and the coverage report looked perfect.
    #[test]
    fn a_run_that_reported_on_fewer_routes_than_it_enumerated_is_refused() {
        let error = refuse_under_enumeration(6, 7).expect_err("under-enumeration must refuse");
        let text = error.to_string();
        assert!(text.contains("only 6"), "{text}");
        assert!(text.contains("1 missing row"), "{text}");
        assert!(text.contains("REFUSED"), "{text}");
    }

    #[test]
    fn a_run_that_reported_on_every_enumerated_route_is_allowed() {
        assert!(refuse_under_enumeration(7, 7).is_ok());
        // More rows than the manifest is lawful: a verdict may name a route the
        // manifest does not enumerate, and that is warned rather than refused.
        assert!(refuse_under_enumeration(9, 7).is_ok());
    }
}

#[cfg(test)]
mod proof_tests {
    use crate::testing::{Harness, run_kind};
    use vds_core::{
        AuthorityVerdict, DeltaDisposition, Digest, EXIT_PASSED, EXIT_VACUOUS, EXIT_VIOLATION,
        ProofKind, ProofStatus, RedrawRecord, RedrawStatus, RegionFinding, ReviewDelta, ReviewId,
        ReviewRegion, SignoffId, Timestamp, VisualReviewRecord,
    };

    const ROUTE: &str = "app/dash/page.tsx";
    /// Where `record_review` writes the screenshot the earning rule re-hashes.
    const SHOT: &str = "design/captures/dash.png";

    /// A delta of the only kind that needs no citation, so a test whose subject
    /// is something else does not have to register a redraw to say "it differs".
    fn delta(describes: &str) -> ReviewDelta {
        ReviewDelta {
            describes: describes.to_owned(),
            disposition: DeltaDisposition::CodeDefect,
            owed_by: None,
        }
    }

    /// Edit a review record in place, through the store.
    fn amend_review(h: &Harness, id: &ReviewId, edit: impl FnOnce(&mut VisualReviewRecord)) {
        let store = h.store();
        let path = store.review_path(id);
        let mut record: VisualReviewRecord = store.read(&path).unwrap();
        edit(&mut record);
        store.replace(&path, &record).unwrap();
    }

    /// A full region checklist, every band examined. The shape a complete pass
    /// produces, spelled out so a partial one is visibly different.
    fn every_region() -> Vec<RegionFinding> {
        ReviewRegion::ALL
            .into_iter()
            .map(|region| RegionFinding {
                region,
                examined: true,
                finding: format!("{region} matches the frame"),
            })
            .collect()
    }

    /// The estate: one screen, its ledger, one captured frame, and the SCREEN
    /// RECORD whose band declaration the correspondence rule reads.
    ///
    /// The band list is spelled out and it is all seven, because these fixtures
    /// review all seven. That is not padding: when R14 first ran over this
    /// suite the estate declared `[header, body_rows]` and the rule went red on
    /// five bands - facets, rail, footer, empty state and keyboard - in VDS's
    /// own fixtures, on every test that used `every_region`. The fixtures were
    /// claiming to have studied bands nothing said the screen had, which is
    /// precisely the finding the rule exists to make, and the cure is the one
    /// the rule asks for: declare the bands, or stop claiming to examine them.
    fn estate(h: &Harness) {
        h.screen("dash", &["Button"]);
        h.ledger();
        h.frames(&[Harness::frame("1:2", "dash frame", &["body"], 1)]);
        h.screen_record("SCR-0001", ROUTE, 1, &["body"], Some("1:2"));
        h.amend_screen("SCR-0001", |record| {
            record.arrangement.bands = ReviewRegion::ALL.to_vec();
        });
    }

    /// Narrow the estate's band declaration, for the correspondence seeds.
    fn declare_bands(h: &Harness, bands: &[ReviewRegion]) {
        h.amend_screen("SCR-0001", |record| {
            record.arrangement.bands = bands.to_vec();
        });
    }

    /// Record a verdict against the CURRENT ledgers.
    #[allow(clippy::too_many_arguments)]
    fn record_review(
        h: &Harness,
        routes: &[&str],
        verdict: AuthorityVerdict,
        deltas: &[&str],
        regions: Vec<RegionFinding>,
        contract: Option<SignoffId>,
        hour: u32,
    ) -> ReviewId {
        let project = h.project();
        let screens = vds_scan::load_fresh(&project).unwrap();
        let row = screens.screens.iter().find(|s| s.route == ROUTE).unwrap();
        let frames = vds_figma::frames::read(&project).unwrap().unwrap();
        let frame = frames.row("1:2").unwrap();
        // A REAL FILE on disk, digested from its bytes. The earning rule
        // re-hashes it, so a fixture that recorded a digest of nothing would
        // test the rule against a case no pipeline can produce - and the two
        // seeds below (an absent file, and bytes that moved) would then have
        // nothing to be different from.
        let shot = h.write_bytes(SHOT, b"the shipped screenshot, as reviewed\n");
        let store = h.store();
        let id = ReviewId::allocate(&store.reviews_dir()).unwrap();
        h.review(VisualReviewRecord {
            id: id.clone(),
            routes: routes.iter().map(|r| (*r).to_owned()).collect(),
            file_key: "KEY".into(),
            node_id: "1:2".into(),
            shipped_screenshot_digest: Digest::of_file(&shot).unwrap(),
            screenshot_path: Some(SHOT.to_owned()),
            served_build: Some("build 3a4a316d".into()),
            shipped_source_digest: row.digest.clone(),
            frame_image_digest: Digest::of_text("frame-png"),
            frame_digest: frame.content_digest.clone().unwrap(),
            verdict,
            regions,
            deltas: deltas.iter().map(|d| delta(d)).collect(),
            contract_signoff: contract,
            reviewed_by: "claude-fable-5 visual pass v1".into(),
            reviewed_at: Timestamp::fixed(2026, 8, 1, hour, 0, 0),
            basis: vec!["draft S-7D".into()],
            notes: None,
        });
        id
    }

    /// The whole pipeline for one route: contract signed at stage 1, verdict
    /// rendered at stage 4 and citing it.
    fn signed_and_reviewed(
        h: &Harness,
        verdict: AuthorityVerdict,
        deltas: &[&str],
    ) -> (SignoffId, ReviewId) {
        estate(h);
        let signoff = h.signoff("KEY", "1:2");
        let review = record_review(
            h,
            &[ROUTE],
            verdict,
            deltas,
            every_region(),
            Some(signoff.clone()),
            12,
        );
        (signoff, review)
    }

    #[test]
    fn a_conform_verdict_against_a_signed_fresh_frame_passes() {
        let h = Harness::new();
        signed_and_reviewed(&h, AuthorityVerdict::Conform, &[]);
        h.route_manifest(&[ROUTE]);
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
        assert_eq!(outcome.exit_code, EXIT_PASSED, "{text}");
        assert_eq!(outcome.rows_enforced, 1, "{text}");
        assert!(
            text.contains("1 CURRENT") && text.contains("0 NEVER REVIEWED"),
            "the coverage report is stated in pipeline terms: {text}"
        );
    }

    /// THE failing-direction test VDS S-7(2)(2) requires, and the one the
    /// enforcement lock names: a recorded deviation against signed authority
    /// is red and names its deltas.
    #[test]
    fn a_deviation_against_a_signed_frame_fails_and_names_the_deltas() {
        let h = Harness::new();
        signed_and_reviewed(
            &h,
            AuthorityVerdict::Deviate,
            &["the dotfield renders behind the main area"],
        );
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert_eq!(outcome.status, ProofStatus::Failed);
        assert!(text.contains("dotfield renders behind"), "{text}");
        // ORDER 22: the severance is on the FACE of the finding, because the
        // finding is the interface an agent acts on. A remedy line that reads
        // as an instruction to delete is itself the defect.
        assert!(text.contains("CLASSIFIES AND DOES NOT DISPOSE"), "{text}");
        assert!(text.contains("155 O7"), "{text}");
        assert!(
            text.contains("exactly three routes"),
            "order 25: one route was the drafted text and three is the law: {text}"
        );
    }

    // -- draft S-7D(7): route-scoped, never family-scoped ---------------------

    /// THE SEED FOR THIS MANDATE, reproduced from the estate's own failure: a
    /// family-level pass names two routes, scores them, and moves on. It must
    /// confer coverage on NEITHER, and both routes must be reported as never
    /// reviewed rather than quietly covered.
    #[test]
    fn a_grouped_verdict_confers_coverage_on_no_route_and_both_read_as_owed() {
        let h = Harness::new();
        estate(&h);
        h.signoff("KEY", "1:2");
        record_review(
            &h,
            &[ROUTE, "app/dash/inbox/page.tsx"],
            AuthorityVerdict::Conform,
            &[],
            every_region(),
            None,
            12,
        );
        h.route_manifest(&[ROUTE, "app/dash/inbox/page.tsx"]);
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("confers coverage on NONE"), "{text}");
        // And the coverage report must not have absorbed the family verdict:
        // both routes are owed.
        assert!(text.contains("2 NEVER REVIEWED"), "{text}");
        assert!(text.contains("app/dash/inbox/page.tsx"), "{text}");
    }

    #[test]
    fn a_route_scoped_verdict_covers_its_route_and_leaves_the_others_owed() {
        let h = Harness::new();
        signed_and_reviewed(&h, AuthorityVerdict::Conform, &[]);
        h.route_manifest(&[ROUTE, "app/dash/inbox/page.tsx"]);
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert!(text.contains("1 CURRENT"), "{text}");
        assert!(text.contains("1 NEVER REVIEWED"), "{text}");
        assert!(
            text.contains("NEVER REVIEWED: the estate enumerates this route"),
            "the owed route must be a NAMED line, not a count: {text}"
        );
        // Coverage owed neither blocks nor licenses ([2026] VJS-SC-OPBOX 1 Q3).
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
    }

    #[test]
    fn a_run_with_no_manifest_says_it_cannot_report_coverage() {
        let h = Harness::new();
        signed_and_reviewed(&h, AuthorityVerdict::Conform, &[]);
        let (_, text) = run_kind(&h, ProofKind::VisualReview);
        assert!(
            text.contains("[stage-4 coverage] UNKNOWN"),
            "a run that cannot enumerate must say so rather than reporting on what it holds: \
             {text}"
        );
    }

    /// The realistic cure for an owed route: delete its line from the
    /// manifest, and the coverage report shrinks to fit.
    #[test]
    fn an_edited_manifest_refuses_the_run_rather_than_shrinking_the_report() {
        let h = Harness::new();
        signed_and_reviewed(&h, AuthorityVerdict::Conform, &[]);
        let path = h.route_manifest(&[ROUTE, "app/dash/inbox/page.tsx"]);
        let original = std::fs::read_to_string(&path).unwrap();
        let edited = original.replace("- app/dash/inbox/page.tsx\n", "");
        assert_ne!(edited, original, "the seed did not change the manifest");
        std::fs::write(&path, &edited).unwrap();
        let error = h.run_kind_err(ProofKind::VisualReview).to_string();
        assert!(error.contains("cannot be relied on"), "{error}");
        assert!(error.contains("stops being reported as owed"), "{error}");
    }

    #[test]
    fn a_reviewed_route_outside_the_manifest_is_checked_and_warned() {
        let h = Harness::new();
        signed_and_reviewed(&h, AuthorityVerdict::Conform, &[]);
        h.route_manifest(&["app/dash/inbox/page.tsx"]);
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert!(
            text.contains("the manifest does not enumerate it"),
            "{text}"
        );
        assert!(
            outcome.rows_enforced >= 1,
            "the verdict is still checked: {text}"
        );
    }

    // -- draft S-7D(8): depth on the face of the artefact ---------------------

    /// A verdict about "the page" is a glance at the page. Seeded with the
    /// empty checklist the previous shape allowed.
    #[test]
    fn a_verdict_with_no_region_checklist_is_refused() {
        let h = Harness::new();
        estate(&h);
        let signoff = h.signoff("KEY", "1:2");
        record_review(
            &h,
            &[ROUTE],
            AuthorityVerdict::Conform,
            &[],
            vec![],
            Some(signoff),
            12,
        );
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("empty region checklist"), "{text}");
    }

    /// Triage must read as triage: two bands examined and seven examined are
    /// different facts, and the ledger has to be able to tell them apart.
    #[test]
    fn a_partial_pass_is_warned_and_names_the_unstudied_bands() {
        let h = Harness::new();
        estate(&h);
        let signoff = h.signoff("KEY", "1:2");
        record_review(
            &h,
            &[ROUTE],
            AuthorityVerdict::Conform,
            &[],
            vec![
                RegionFinding {
                    region: ReviewRegion::Header,
                    examined: true,
                    finding: "matches the frame".into(),
                },
                RegionFinding {
                    region: ReviewRegion::BodyRows,
                    examined: false,
                    finding: "not examined: capture debt, needs seeded data".into(),
                },
            ],
            Some(signoff),
            12,
        );
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(
            outcome.status,
            ProofStatus::Passed,
            "a partial pass is not a failure; it is a partial pass: {text}"
        );
        assert!(text.contains("1 of 2 listed region(s) examined"), "{text}");
        assert!(
            text.contains("facets") && text.contains("footer"),
            "the bands accounted for in neither direction must be named: {text}"
        );
    }

    // -- draft S-7D(12): a conform verdict earns its claim ---------------------

    /// SEED R13(a). The minted record: a conform verdict naming no screenshot,
    /// so the one hash that could be re-derived from an artefact is absent and
    /// the verdict rests on two fields somebody typed.
    #[test]
    fn a_conform_verdict_naming_no_screenshot_has_earned_nothing() {
        let h = Harness::new();
        let (_, review) = signed_and_reviewed(&h, AuthorityVerdict::Conform, &[]);
        amend_review(&h, &review, |record| record.screenshot_path = None);
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("UNEARNED CONFORMANCE"), "{text}");
        assert!(text.contains("names no screenshot"), "{text}");
        assert!(text.contains("MINTED"), "{text}");
    }

    /// SEED R13(b), and DISTINCT from the one above: the path is there, the
    /// file is there, and its bytes are not the bytes that were reviewed. A
    /// rule that only checked for the field would call this green, and this is
    /// the case where the field is present and the evidence is gone.
    #[test]
    fn a_screenshot_whose_bytes_moved_no_longer_backs_the_verdict() {
        let h = Harness::new();
        signed_and_reviewed(&h, AuthorityVerdict::Conform, &[]);
        h.write_bytes(SHOT, b"a different capture, taken later\n");
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("does not re-hash"), "{text}");
        assert!(
            text.contains("picture nobody can now produce"),
            "the finding says what is gone, not merely that a hash differs: {text}"
        );
    }

    /// SEED R13(c). A checklist where every row says `examined: false` is a
    /// WELL-FORMED record - full checklist, a reason on every row - and it says
    /// nobody looked. The record-validation rules pass it, which is exactly why
    /// the earning rule has to be the one that catches it.
    #[test]
    fn a_conform_verdict_that_examined_no_band_is_refused_though_the_record_is_valid() {
        let h = Harness::new();
        estate(&h);
        let signoff = h.signoff("KEY", "1:2");
        let review = record_review(
            &h,
            &[ROUTE],
            AuthorityVerdict::Conform,
            &[],
            ReviewRegion::ALL
                .into_iter()
                .map(|region| RegionFinding {
                    region,
                    examined: false,
                    finding: "not examined: capture debt, needs seeded data".into(),
                })
                .collect(),
            Some(signoff),
            12,
        );
        let store = h.store();
        let record: VisualReviewRecord = store.read(&store.review_path(&review)).unwrap();
        assert!(
            record.defects().is_empty(),
            "the record is WELL FORMED and says nobody looked, which is the whole difficulty: \
             {:?}",
            record.defects()
        );
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("examined NO band"), "{text}");
    }

    /// SEED R13(d), the one found in LANDED records rather than by minting one:
    /// the shipped source digest is taken from the local tree, so with no
    /// served build the record can pair a picture of one build with the hash of
    /// another and nothing in it says so.
    #[test]
    fn a_conform_verdict_naming_no_served_build_has_earned_nothing() {
        let h = Harness::new();
        let (_, review) = signed_and_reviewed(&h, AuthorityVerdict::Conform, &[]);
        amend_review(&h, &review, |record| record.served_build = None);
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("LOCAL TREE"), "{text}");
    }

    /// THE OTHER DIRECTION, and the one that keeps the rule switched on: a
    /// DEVIATE verdict claims no parity, so none of the four conditions is
    /// asked of it. A rule that fired on verdicts claiming nothing would make
    /// recording a deviation dearer than recording nothing, and an estate that
    /// learns that stops recording deviations.
    #[test]
    fn a_deviate_verdict_missing_every_earning_condition_fails_r5_and_never_r13() {
        let h = Harness::new();
        let (_, review) = signed_and_reviewed(
            &h,
            AuthorityVerdict::Deviate,
            &["the counted facet row is drawn in the frame and not shipped"],
        );
        amend_review(&h, &review, |record| {
            record.screenshot_path = None;
            record.served_build = None;
            for finding in &mut record.regions {
                finding.examined = false;
                finding.finding = "not examined: capture debt".into();
            }
        });
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("deviation(s), reviewed by"), "{text}");
        assert!(
            !text.contains("UNEARNED"),
            "the earning rule must not reach a verdict that claims no parity: {text}"
        );
    }

    // -- draft S-7D(14): the band correspondence ------------------------------

    /// SEED R14. A reviewer examines five bands this screen does not have. In
    /// the ledger that reads as depth; what it is, is a finding about another
    /// screen.
    #[test]
    fn an_examined_band_the_screen_does_not_declare_is_foreign_and_fails() {
        let h = Harness::new();
        signed_and_reviewed(&h, AuthorityVerdict::Conform, &[]);
        declare_bands(&h, &[ReviewRegion::Header, ReviewRegion::BodyRows]);
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(
            text.contains("5 band(s) this screen does not have"),
            "{text}"
        );
        assert!(text.contains("rail") && text.contains("keyboard"), "{text}");
    }

    /// The rule reads STUDY and not PAPERWORK. The same five bands, recorded
    /// as not examined with a reason, are a lawful triage answer and not a
    /// claim about anything, so nothing is foreign.
    #[test]
    fn a_foreign_band_recorded_as_unexamined_is_lawful_and_passes() {
        let h = Harness::new();
        estate(&h);
        declare_bands(&h, &[ReviewRegion::Header, ReviewRegion::BodyRows]);
        let signoff = h.signoff("KEY", "1:2");
        record_review(
            &h,
            &[ROUTE],
            AuthorityVerdict::Conform,
            &[],
            ReviewRegion::ALL
                .into_iter()
                .map(|region| RegionFinding {
                    region,
                    examined: matches!(region, ReviewRegion::Header | ReviewRegion::BodyRows),
                    finding: "matches the frame, or: this screen has no such band".into(),
                })
                .collect(),
            Some(signoff),
            12,
        );
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(
            outcome.status,
            ProofStatus::Passed,
            "a band recorded as NOT examined is a lawful answer about a band the screen does \
             not have: {text}"
        );
        assert!(!text.contains("does not have"), "{text}");
    }

    /// SEED W7, and the reason this rule is asked in two steps. A screen that
    /// declares no bands makes the correspondence UNRUNNABLE, and the run must
    /// SAY SO: a rule that cannot run and says nothing is indistinguishable
    /// from one that ran and found nothing.
    #[test]
    fn a_screen_declaring_no_bands_reports_that_the_rule_did_not_run() {
        let h = Harness::new();
        signed_and_reviewed(&h, AuthorityVerdict::Conform, &[]);
        declare_bands(&h, &[]);
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(
            outcome.status,
            ProofStatus::Passed,
            "an unrunnable rule is not a failure; it is an unrunnable rule: {text}"
        );
        assert!(text.contains("REGION CORRESPONDENCE DID NOT RUN"), "{text}");
        // THE NOTE, not merely the exit code. The reach is the load-bearing
        // half: on the estate this was written for the same check could only
        // run on 17 rows of 160, and one number holding both facts would have
        // reported 143 unmeasured routes as clean.
        assert!(
            text.contains("R14 ran on 0 route(s)"),
            "the run must report what the rule could NOT do, on its own face: {text}"
        );
        assert!(text.contains("COULD NOT RUN on 1 route(s)"), "{text}");
    }

    /// The other unrunnable shape: no screen record claims the route at all.
    #[test]
    fn a_route_no_screen_record_claims_reports_that_the_rule_did_not_run() {
        let h = Harness::new();
        signed_and_reviewed(&h, AuthorityVerdict::Conform, &[]);
        h.amend_screen("SCR-0001", |record| {
            record.route = "app/somewhere/else/page.tsx".into();
        });
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
        assert!(text.contains("no screen record names"), "{text}");
        assert!(text.contains("R14 ran on 0 route(s)"), "{text}");
    }

    // -- draft S-7D(13): a delta is classified, and cites its row -------------

    /// An uncited delta REFUSES THE RECORD, and the consequence is the one that
    /// matters: the record confers no coverage, so its route falls back into
    /// the never-reviewed population by name. A rule that only warned would
    /// leave the route reading as covered by a verdict nobody can act on.
    #[test]
    fn an_uncited_delta_confers_no_coverage_and_its_route_reads_as_owed() {
        let h = Harness::new();
        let (_, review) = signed_and_reviewed(
            &h,
            AuthorityVerdict::Deviate,
            &["the shipped empty state is preferred to the drawn one"],
        );
        amend_review(&h, &review, |record| {
            record.deltas = vec![ReviewDelta {
                describes: "the shipped empty state is preferred to the drawn one".into(),
                disposition: DeltaDisposition::ShippedIsBetter,
                owed_by: None,
            }];
        });
        h.route_manifest(&[ROUTE]);
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("cites no row"), "{text}");
        assert!(
            text.contains("NEVER REVIEWED"),
            "an invalid record confers no coverage, so the route is owed and NAMED: {text}"
        );
        assert!(text.contains("1 NEVER REVIEWED"), "{text}");
    }

    #[test]
    fn a_delta_citing_the_wrong_series_is_refused() {
        let h = Harness::new();
        let (_, review) = signed_and_reviewed(
            &h,
            AuthorityVerdict::Deviate,
            &["the command bar belongs to the shell, not to this route"],
        );
        amend_review(&h, &review, |record| {
            record.deltas = vec![ReviewDelta {
                describes: "the command bar belongs to the shell, not to this route".into(),
                disposition: DeltaDisposition::BelongsToAppChrome,
                owed_by: Some("RDW-0001".into()),
            }];
        });
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("not a SCR- row"), "{text}");
    }

    /// A disposition CLASSIFIES and never DISPOSES. Every one of the seven
    /// leaves the verdict deviate and leaves R5 red, and the finding says so on
    /// its face, because the face of the finding is the interface an agent acts
    /// on.
    #[test]
    fn every_disposition_still_leaves_the_deviation_red() {
        for disposition in DeltaDisposition::ALL {
            let h = Harness::new();
            let (_, review) = signed_and_reviewed(
                &h,
                AuthorityVerdict::Deviate,
                &["the footer window statement is not shipped"],
            );
            let owed_by = disposition.owed_by_prefixes().first().map(|prefix| {
                // A citation in the right series. Whether the row it names
                // EXISTS is another rule's question; this test is about
                // whether a well-cited disposition can close a deviation, and
                // the answer must be no in all seven.
                format!("{prefix}0001")
            });
            amend_review(&h, &review, |record| {
                record.deltas = vec![ReviewDelta {
                    describes: "the footer window statement is not shipped".into(),
                    disposition,
                    owed_by,
                }];
            });
            let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
            assert_eq!(
                outcome.exit_code, EXIT_VIOLATION,
                "{disposition} closed a deviation, and no disposition may: {text}"
            );
            assert!(text.contains(disposition.as_str()), "{text}");
            assert!(text.contains("CLASSIFIES AND DOES NOT DISPOSE"), "{text}");
        }
    }

    // -- draft S-7D(10): the stage link ---------------------------------------

    /// A verdict that cannot name its contract version is how "we checked it"
    /// survives a contract change. Seeded with a citation that resolves to a
    /// sign-off of a DIFFERENT version of the frame.
    #[test]
    fn a_verdict_citing_a_contract_it_was_not_taken_against_is_refused() {
        let h = Harness::new();
        estate(&h);
        // A sign-off at a hash that is not the frame's: the contract version
        // the verdict names is not the one it compared against.
        let wrong = h.signoff_at("KEY", "1:2", Digest::of_text("an earlier contract"));
        record_review(
            &h,
            &[ROUTE],
            AuthorityVerdict::Conform,
            &[],
            every_region(),
            Some(wrong),
            12,
        );
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("different version of the contract"), "{text}");
    }

    #[test]
    fn a_conform_verdict_naming_no_contract_at_all_is_refused() {
        let h = Harness::new();
        estate(&h);
        h.signoff("KEY", "1:2");
        record_review(
            &h,
            &[ROUTE],
            AuthorityVerdict::Conform,
            &[],
            every_region(),
            None,
            12,
        );
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("names no contract version"), "{text}");
    }

    #[test]
    fn the_pipeline_note_states_that_stage_three_and_stage_four_are_not_substitutes() {
        let h = Harness::new();
        signed_and_reviewed(&h, AuthorityVerdict::Conform, &[]);
        h.route_manifest(&[ROUTE]);
        let (_, text) = run_kind(&h, ProofKind::VisualReview);
        assert!(text.contains("NOT SUBSTITUTES"), "{text}");
        assert!(text.contains("28 green stage-3 gates"), "{text}");
    }

    // -- authority and staleness (mandate 1, re-asserted on the new shape) -----

    /// MANDATE SEED 1: a signed frame whose hash then drifts must flip the
    /// proof to no_authority - not green (the sign-off is stale), not red (the
    /// deviation is unadjudicated), and reported as coverage owed.
    #[test]
    fn a_frame_that_changed_after_signoff_flips_to_no_authority() {
        let h = Harness::new();
        signed_and_reviewed(&h, AuthorityVerdict::Conform, &[]);
        h.frames(&[Harness::frame("1:2", "dash frame REDRAWN", &["body"], 2)]);
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        // NEITHER GREEN NOR RED. The sign-off no longer covers the frame's
        // current hash, so the surface is unsigned again, and under
        // [2026] VJS-CA-VDS 1 order 20 a stale verdict on an unsigned surface
        // WARNS AND SKIPS: before registration this kind reports and never
        // blocks. The row is not enforced and nothing reads as a pass over it.
        assert_eq!(outcome.rows_enforced, 0, "{text}");
        assert!(text.contains("STALE"), "{text}");
        assert!(text.contains("no_authority_verdict_stale"), "{text}");
        assert!(
            !text.contains("PASS:"),
            "a drifted frame must never read green: {text}"
        );

        // And a FRESH review against the drifted frame is no_authority, not
        // green: the sign-off no longer covers the current hash.
        let h2 = Harness::new();
        estate(&h2);
        h2.signoff("KEY", "1:2");
        h2.frames(&[Harness::frame("1:2", "dash frame REDRAWN", &["body"], 2)]);
        record_review(
            &h2,
            &[ROUTE],
            AuthorityVerdict::NoAuthority,
            &[],
            every_region(),
            None,
            12,
        );
        let (outcome, text) = run_kind(&h2, ProofKind::VisualReview);
        assert_eq!(outcome.status, ProofStatus::Vacuous, "{text}");
        assert!(text.contains("no_authority_frame_unsigned"), "{text}");
        assert!(text.contains("changed after sign-off"), "{text}");
        assert!(text.contains("COVERAGE OWED"), "{text}");
    }

    /// MANDATE SEED 2: a conformance claim against an unsigned frame is
    /// rejected at validation.
    #[test]
    fn a_conform_claim_against_an_unsigned_frame_is_refused() {
        let h = Harness::new();
        estate(&h);
        // A sign-off exists for ANOTHER frame, so the verdict can cite a
        // contract while the reviewed frame itself carries no authority.
        let elsewhere = h.signoff_at("KEY", "9:9", Digest::of_text("another frame"));
        let _ = elsewhere;
        record_review(
            &h,
            &[ROUTE],
            AuthorityVerdict::Conform,
            &[],
            every_region(),
            None,
            12,
        );
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(
            text.contains("names no contract version")
                || text.contains("cannot claim conformance against an unsigned frame"),
            "{text}"
        );
    }

    #[test]
    fn an_unsigned_frame_is_no_authority_never_green_never_red() {
        let h = Harness::new();
        estate(&h);
        record_review(
            &h,
            &[ROUTE],
            AuthorityVerdict::NoAuthority,
            &[],
            every_region(),
            None,
            12,
        );
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.status, ProofStatus::Vacuous, "{text}");
        assert_eq!(outcome.exit_code, EXIT_VACUOUS, "{text}");
        assert_eq!(outcome.rows_enforced, 0, "{text}");
        assert!(text.contains("never signed"), "{text}");
        assert!(text.contains("COVERAGE OWED"), "{text}");
    }

    #[test]
    fn a_stale_shipped_side_ends_the_verdict_visibly() {
        let h = Harness::new();
        signed_and_reviewed(&h, AuthorityVerdict::Conform, &[]);
        h.route_manifest(&[ROUTE]);
        // The route's source changes after the review, and the ledger is
        // regenerated (ledger_staleness owns the unregenerated case).
        h.screen("dash", &["Button", "Card"]);
        h.ledger();
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("shipped side moved"), "{text}");
        assert!(
            text.contains("1 OWED BY DRIFT"),
            "the pipeline report must count a drifted route as owed, not reviewed: {text}"
        );
    }

    #[test]
    fn a_deviate_verdict_with_no_deltas_is_incoherent() {
        let h = Harness::new();
        signed_and_reviewed(&h, AuthorityVerdict::Deviate, &[]);
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("names no work"), "{text}");
    }

    /// MANDATE SEED 3: a redraw resolved without a covering sign-off row must
    /// fail, in all three uncovered shapes.
    #[test]
    fn a_signed_redraw_without_a_covering_signoff_fails() {
        let h = Harness::new();
        estate(&h);
        let store = h.store();

        // Shape 1: the word `signed` with no row at all.
        let id = vds_core::RedrawId::allocate(&store.redraws_dir()).unwrap();
        h.redraw(RedrawRecord {
            id,
            deviation: "VRW-0001: the band is missing".into(),
            review_id: None,
            proposed: "add the band back neatly, as a new frame version".into(),
            status: RedrawStatus::Signed,
            file_key: "KEY".into(),
            node_id: "1:2".into(),
            resolved_by: None,
            directed_by: None,
            opened_at: Timestamp::fixed(2026, 8, 1, 12, 0, 0),
            basis: vec![],
            notes: None,
        });
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("The word is not the row"), "{text}");

        // Shape 2: a row that exists but whose hash is not the frame's
        // CURRENT hash (the frame moved after that sign-off).
        let stale = h.signoff_at("KEY", "1:2", Digest::of_text("an old frame version"));
        let id2 = vds_core::RedrawId::allocate(&store.redraws_dir()).unwrap();
        h.redraw(RedrawRecord {
            id: id2,
            deviation: "VRW-0001: the band is missing".into(),
            review_id: None,
            proposed: "add the band back neatly".into(),
            status: RedrawStatus::Signed,
            file_key: "KEY".into(),
            node_id: "1:2".into(),
            resolved_by: Some(stale),
            directed_by: None,
            opened_at: Timestamp::fixed(2026, 8, 1, 12, 0, 0),
            basis: vec![],
            notes: None,
        });
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("not the frame's"), "{text}");
    }

    #[test]
    fn a_signed_redraw_with_a_covering_signoff_passes() {
        let h = Harness::new();
        estate(&h);
        let covering = h.signoff("KEY", "1:2");
        let store = h.store();
        let id = vds_core::RedrawId::allocate(&store.redraws_dir()).unwrap();
        h.redraw(RedrawRecord {
            id,
            deviation: "VRW-0001: the band is missing".into(),
            review_id: None,
            proposed: "the band, drawn back into the frame".into(),
            status: RedrawStatus::Signed,
            file_key: "KEY".into(),
            node_id: "1:2".into(),
            resolved_by: Some(covering),
            directed_by: None,
            opened_at: Timestamp::fixed(2026, 8, 1, 12, 0, 0),
            basis: vec![],
            notes: None,
        });
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
        assert_eq!(outcome.rows_enforced, 1, "{text}");
    }

    #[test]
    fn an_open_redraw_warns_and_does_not_block() {
        let h = Harness::new();
        estate(&h);
        let store = h.store();
        let id = vds_core::RedrawId::allocate(&store.redraws_dir()).unwrap();
        h.redraw(RedrawRecord {
            id,
            deviation: "VRW-0001: the band is missing".into(),
            review_id: None,
            proposed: "add it neatly later, through the design".into(),
            status: RedrawStatus::Proposed,
            file_key: "KEY".into(),
            node_id: "1:2".into(),
            resolved_by: None,
            directed_by: None,
            opened_at: Timestamp::fixed(2026, 8, 1, 12, 0, 0),
            basis: vec![],
            notes: None,
        });
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
        assert!(text.contains("comes back through the design"), "{text}");
    }

    #[test]
    fn an_older_review_is_superseded_by_the_newest_for_its_route() {
        let h = Harness::new();
        let (signoff, _) = signed_and_reviewed(
            &h,
            AuthorityVerdict::Deviate,
            &["old finding: the facet row is missing"],
        );
        record_review(
            &h,
            &[ROUTE],
            AuthorityVerdict::Conform,
            &[],
            every_region(),
            Some(signoff),
            14,
        );
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
        assert!(text.contains("superseded_by_a_newer_review"), "{text}");
    }

    // -- [2026] VJS-CA-VDS 1 orders 20 and 27 ---------------------------------

    /// ORDER 20's seed. On enactment morning the register is empty, so every
    /// surface is no_authority; the first time a reviewed route's source moves
    /// the gate would have gone RED on a surface the Council held nothing may
    /// block on. The rule must warn and skip - and R1/R6 must still be fatal,
    /// which the two tests below hold.
    #[test]
    fn a_stale_verdict_on_an_unsigned_surface_warns_and_never_blocks() {
        let h = Harness::new();
        estate(&h);
        record_review(
            &h,
            &[ROUTE],
            AuthorityVerdict::NoAuthority,
            &[],
            every_region(),
            None,
            12,
        );
        // The shipped side moves, with no sign-off anywhere.
        h.screen("dash", &["Button", "Card"]);
        h.ledger();
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(
            outcome.exit_code, EXIT_VACUOUS,
            "an unsigned surface may not block: {text}"
        );
        assert!(text.contains("no_authority_verdict_stale"), "{text}");
        assert!(text.contains("reports and never blocks"), "{text}");
    }

    #[test]
    fn a_stale_verdict_on_a_signed_surface_still_blocks() {
        // The other half. Order 20 degrades the rule PER SURFACE; it does not
        // switch it off, and a registered surface is exactly where the stale
        // green the rule exists to stop becomes available.
        let h = Harness::new();
        signed_and_reviewed(&h, AuthorityVerdict::Conform, &[]);
        h.screen("dash", &["Button", "Card"]);
        h.ledger();
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("shipped side moved"), "{text}");
    }

    #[test]
    fn a_record_level_defect_stays_fatal_on_an_unsigned_surface() {
        // R1, R6 and the record-validation rules are defects in the RECORD,
        // curable by the recorder, and not facts about any surface: order 20
        // leaves them fatal in every authority state, and a rule that degraded
        // with the surface would let a broken record hide behind an empty
        // register. Seeded with a deviate verdict that cites no contract and
        // names no delta - two record defects at once, on a surface nothing
        // has signed.
        let h = Harness::new();
        estate(&h);
        record_review(
            &h,
            &[ROUTE],
            AuthorityVerdict::Deviate,
            &[],
            every_region(),
            None,
            12,
        );
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(
            outcome.exit_code, EXIT_VIOLATION,
            "a broken record must not hide behind an empty register: {text}"
        );
        assert!(text.contains("names no contract version"), "{text}");
    }

    /// ORDER 27's seed: `parked` is the word, and the word is not the row.
    #[test]
    fn a_parked_redraw_without_a_direction_row_is_refused() {
        let h = Harness::new();
        estate(&h);
        let store = h.store();
        let id = vds_core::RedrawId::allocate(&store.redraws_dir()).unwrap();
        h.redraw(RedrawRecord {
            id,
            deviation: "VRW-0001: the band is off-screen".into(),
            review_id: None,
            proposed: "the band, drawn into the frame".into(),
            status: RedrawStatus::Parked,
            file_key: "KEY".into(),
            node_id: "1:2".into(),
            resolved_by: None,
            directed_by: None,
            opened_at: Timestamp::fixed(2026, 8, 1, 12, 0, 0),
            basis: vec![],
            notes: None,
        });
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("the word is not the row"), "{text}");
    }

    /// A park under a LIVE direction is reported and never counted a
    /// violation: while the registered direction stands, no gate may.
    #[test]
    fn a_parked_redraw_under_a_live_direction_is_reported_and_not_fatal() {
        let h = Harness::new();
        estate(&h);
        let direction = h.direction(vds_core::DirectedSurface::Frame {
            file_key: "KEY".into(),
            node_id: "1:2".into(),
        });
        let store = h.store();
        let id = vds_core::RedrawId::allocate(&store.redraws_dir()).unwrap();
        h.redraw(RedrawRecord {
            id,
            deviation: "VRW-0001: the band is off-screen".into(),
            review_id: None,
            proposed: "the band, drawn into the frame".into(),
            status: RedrawStatus::Parked,
            file_key: "KEY".into(),
            node_id: "1:2".into(),
            resolved_by: None,
            directed_by: Some(direction),
            opened_at: Timestamp::fixed(2026, 8, 1, 12, 0, 0),
            basis: vec![],
            notes: None,
        });
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_ne!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("PARKED under DIR-0001"), "{text}");
        assert!(
            text.contains("no gate may count this a violation"),
            "{text}"
        );
    }

    /// Staleness by hash reaches a direction too: rewrite the logged decision
    /// and the park lapses, exactly as a frame's sign-off lapses.
    #[test]
    fn a_park_resting_on_an_edited_decision_log_lapses() {
        let h = Harness::new();
        estate(&h);
        let direction = h.direction(vds_core::DirectedSurface::Frame {
            file_key: "KEY".into(),
            node_id: "1:2".into(),
        });
        let store = h.store();
        let id = vds_core::RedrawId::allocate(&store.redraws_dir()).unwrap();
        h.redraw(RedrawRecord {
            id,
            deviation: "VRW-0001: the band is off-screen".into(),
            review_id: None,
            proposed: "the band, drawn into the frame".into(),
            status: RedrawStatus::Parked,
            file_key: "KEY".into(),
            node_id: "1:2".into(),
            resolved_by: None,
            directed_by: Some(direction.clone()),
            opened_at: Timestamp::fixed(2026, 8, 1, 12, 0, 0),
            basis: vec![],
            notes: None,
        });
        h.move_direction_log(&direction);
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("edited after registration"), "{text}");
    }

    #[test]
    fn a_project_with_no_review_is_vacuous_and_says_it_is_not_evidence() {
        let h = Harness::new();
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.status, ProofStatus::Vacuous, "{text}");
        assert!(text.contains("NOT evidence"), "{text}");
    }

    #[test]
    fn the_reserved_note_states_the_repeal_on_a_passing_run() {
        let h = Harness::new();
        signed_and_reviewed(&h, AuthorityVerdict::Conform, &[]);
        let (_, text) = run_kind(&h, ProofKind::VisualReview);
        assert!(text.contains("NO ACCEPTANCE STATE"), "{text}");
    }
}
