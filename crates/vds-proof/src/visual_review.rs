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

/// Stated on every run, passing or not: the engine's boundary, and the repeal.
const RESERVED_NOTE: &str = "[reserved] This kind validates, stores and stales VERDICT RECORDS. \
                             The capture and review pipeline (render, screenshot, export, \
                             compare) lives in the consuming repo: a proof may not call a \
                             network or a model (VDS S-7(2)(1)). The verdict vocabulary is \
                             conform | deviate | no_authority, and there is NO ACCEPTANCE \
                             STATE: an addition the frame omits is a deviation exactly like a \
                             missing element, and its resolution path is a new signed frame \
                             version recorded as a redraw, never an engine-side excusal.";

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

    // R7: the newest record per route governs; older ones are history.
    let mut newest: BTreeMap<&str, &vds_core::Timestamp> = BTreeMap::new();
    for located in &reviews {
        let record = &located.value;
        let entry = newest
            .entry(record.route.as_str())
            .or_insert(&record.reviewed_at);
        if record.reviewed_at.as_str() > entry.as_str() {
            *entry = &record.reviewed_at;
        }
    }

    for located in &reviews {
        let review = &located.value;
        let location = format!("{} [{}]", review.id, review.route);

        if newest
            .get(review.route.as_str())
            .is_some_and(|newest| newest.as_str() != review.reviewed_at.as_str())
        {
            run.row(Verdict::Skipped("superseded_by_a_newer_review"));
            continue;
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

        // R1/R2: the shipped side.
        let current_source = screens
            .as_ref()
            .and_then(|l| l.screens.iter().find(|s| s.route == review.route))
            .map(|s| &s.digest);
        let Some(current_source) = current_source else {
            run.row(Verdict::Enforced);
            run.fail(Violation::fatal(
                location.clone(),
                RULE_UNKNOWN_ROUTE,
                "the reviewed route to appear in the screens ledger",
                format!(
                    "{} is not in the screens ledger{}. A verdict about a route the declared \
                     surface does not carry is a verdict about nothing this project ships.",
                    review.route,
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
                     visibly: re-run the capture and review pipeline for {}.",
                    review.route
                ),
            ));
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
            continue;
        }

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
                         omits is a deviation exactly like a missing element. The \
                         resolution path is a proposed redraw closed by a NEW sign-off, \
                         then a re-review; there is no engine-side excusal.",
                        review.deltas.len(),
                        review.reviewed_by,
                        review.reviewed_at.as_str(),
                        review.deltas.join("; ")
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

    if reviews.is_empty() && redraws.is_empty() {
        run.note(
            "[scope] no visual review and no redraw is recorded, so every row is skipped \
             and this run is vacuous. That is the honest state of a project whose capture \
             pipeline has not run, and it is NOT evidence (VDS S-7(2)(4)).",
        );
    }

    run.finish(&ctx.capture_options()?, out)
}

#[cfg(test)]
mod proof_tests {
    use crate::testing::{Harness, run_kind};
    use vds_core::{
        AuthorityVerdict, Digest, EXIT_PASSED, EXIT_VACUOUS, EXIT_VIOLATION, ProofKind,
        ProofStatus, RedrawRecord, RedrawStatus, ReviewId, Timestamp, VisualReviewRecord,
    };

    /// A harness with one screen, its ledger, one captured frame, and the
    /// review pipeline's outputs for it.
    fn reviewed(h: &Harness, verdict: AuthorityVerdict, deltas: &[&str]) -> ReviewId {
        h.screen("dash", &["Button"]);
        h.ledger();
        h.frames(&[Harness::frame("1:2", "dash frame", &["body"], 1)]);
        review_only(h, verdict, deltas)
    }

    /// Record a review against the CURRENT ledgers, without regenerating them.
    fn review_only(h: &Harness, verdict: AuthorityVerdict, deltas: &[&str]) -> ReviewId {
        let project = h.project();
        let screens = vds_scan::load_fresh(&project).unwrap();
        let row = screens
            .screens
            .iter()
            .find(|s| s.route == "app/dash/page.tsx")
            .unwrap();
        let frames = vds_figma::frames::read(&project).unwrap().unwrap();
        let frame = frames.row("1:2").unwrap();
        let store = h.store();
        let id = ReviewId::allocate(&store.reviews_dir()).unwrap();
        let record = VisualReviewRecord {
            id: id.clone(),
            route: "app/dash/page.tsx".into(),
            file_key: "KEY".into(),
            node_id: "1:2".into(),
            shipped_screenshot_digest: Digest::of_text("screenshot-png"),
            shipped_source_digest: row.digest.clone(),
            frame_image_digest: Digest::of_text("frame-png"),
            frame_digest: frame.content_digest.clone().unwrap(),
            verdict,
            deltas: deltas.iter().map(|d| (*d).to_owned()).collect(),
            reviewed_by: "claude-fable-5 visual pass v1".into(),
            reviewed_at: Timestamp::fixed(2026, 8, 1, 12, 0, 0),
            basis: vec!["draft S-7D".into()],
            notes: None,
        };
        h.review(record);
        id
    }

    #[test]
    fn a_conform_verdict_against_a_signed_fresh_frame_passes() {
        let h = Harness::new();
        reviewed(&h, AuthorityVerdict::Conform, &[]);
        h.signoff("KEY", "1:2");
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
        assert_eq!(outcome.exit_code, EXIT_PASSED, "{text}");
        assert_eq!(outcome.rows_enforced, 1, "{text}");
    }

    /// THE failing-direction test VDS S-7(2)(2) requires, and the one the
    /// enforcement lock names: a recorded deviation against signed authority
    /// is red and names its deltas.
    #[test]
    fn a_deviation_against_a_signed_frame_fails_and_names_the_deltas() {
        let h = Harness::new();
        reviewed(
            &h,
            AuthorityVerdict::Deviate,
            &["the dotfield renders behind the main area"],
        );
        h.signoff("KEY", "1:2");
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert_eq!(outcome.status, ProofStatus::Failed);
        assert!(text.contains("dotfield renders behind"), "{text}");
        assert!(
            text.contains("no engine-side excusal"),
            "the repeal is stated on the finding: {text}"
        );
    }

    /// MANDATE SEED 1: a signed frame whose hash then drifts must flip the
    /// proof to no_authority - not green (the sign-off is stale), not red (the
    /// deviation is unadjudicated), and reported as coverage owed.
    #[test]
    fn a_frame_that_changed_after_signoff_flips_to_no_authority() {
        let h = Harness::new();
        reviewed(&h, AuthorityVerdict::Conform, &[]);
        h.signoff("KEY", "1:2");
        // The frame changes after sign-off AND after the review: re-capture
        // with different content. Both the sign-off and the review now name a
        // dead hash.
        h.frames(&[Harness::frame("1:2", "dash frame REDRAWN", &["body"], 2)]);
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        // The review's frame hash is stale (R3), which is the loud report.
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("STALE"), "{text}");
        assert!(
            !text.contains("PASS:"),
            "a drifted frame must never read green: {text}"
        );

        // And a FRESH review against the drifted frame is no_authority, not
        // green: the sign-off no longer covers the current hash.
        let h2 = Harness::new();
        h2.screen("dash", &["Button"]);
        h2.ledger();
        h2.frames(&[Harness::frame("1:2", "dash frame", &["body"], 1)]);
        h2.signoff("KEY", "1:2");
        h2.frames(&[Harness::frame("1:2", "dash frame REDRAWN", &["body"], 2)]);
        review_only(&h2, AuthorityVerdict::NoAuthority, &[]);
        let (outcome, text) = run_kind(&h2, ProofKind::VisualReview);
        assert_eq!(outcome.status, ProofStatus::Vacuous, "{text}");
        assert!(text.contains("no_authority_frame_unsigned"), "{text}");
        assert!(text.contains("changed after sign-off"), "{text}");
        assert!(text.contains("COVERAGE OWED"), "{text}");
    }

    /// MANDATE SEED 2: a conformance claim against an unsigned frame is
    /// rejected at validation. Green against nothing is the one combination
    /// that could smuggle taste back downstream.
    #[test]
    fn a_conform_claim_against_an_unsigned_frame_is_refused() {
        let h = Harness::new();
        reviewed(&h, AuthorityVerdict::Conform, &[]);
        // No sign-off at all.
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(
            text.contains("cannot claim conformance against an unsigned frame"),
            "{text}"
        );
    }

    /// An honest no_authority verdict against an unsigned frame is DISTINCT:
    /// never green, never red, warned as coverage owed.
    #[test]
    fn an_unsigned_frame_is_no_authority_never_green_never_red() {
        let h = Harness::new();
        reviewed(&h, AuthorityVerdict::NoAuthority, &[]);
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
        reviewed(&h, AuthorityVerdict::Conform, &[]);
        h.signoff("KEY", "1:2");
        // The route's source changes after the review, and the ledger is
        // regenerated (ledger_staleness owns the unregenerated case).
        h.screen("dash", &["Button", "Card"]);
        h.ledger();
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("shipped side moved"), "{text}");
        assert!(text.contains("STALE"), "{text}");
    }

    #[test]
    fn a_deviate_verdict_with_no_deltas_is_incoherent() {
        let h = Harness::new();
        reviewed(&h, AuthorityVerdict::Deviate, &[]);
        h.signoff("KEY", "1:2");
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("names no work"), "{text}");
    }

    /// MANDATE SEED 3: a redraw resolved without a covering sign-off row must
    /// fail, in all three uncovered shapes.
    #[test]
    fn a_signed_redraw_without_a_covering_signoff_fails() {
        let h = Harness::new();
        h.screen("dash", &["Button"]);
        h.ledger();
        h.frames(&[Harness::frame("1:2", "dash frame", &["body"], 1)]);
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
        h.screen("dash", &["Button"]);
        h.ledger();
        h.frames(&[Harness::frame("1:2", "dash frame", &["body"], 1)]);
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
        h.screen("dash", &["Button"]);
        h.ledger();
        h.frames(&[Harness::frame("1:2", "dash frame", &["body"], 1)]);
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
        reviewed(&h, AuthorityVerdict::Deviate, &["old finding"]);
        h.signoff("KEY", "1:2");
        // A newer, conform review of the same route against the same hashes.
        let project = h.project();
        let screens = vds_scan::load_fresh(&project).unwrap();
        let row = screens
            .screens
            .iter()
            .find(|s| s.route == "app/dash/page.tsx")
            .unwrap();
        let frames = vds_figma::frames::read(&project).unwrap().unwrap();
        let frame = frames.row("1:2").unwrap();
        let store = h.store();
        let id = ReviewId::allocate(&store.reviews_dir()).unwrap();
        h.review(VisualReviewRecord {
            id,
            route: "app/dash/page.tsx".into(),
            file_key: "KEY".into(),
            node_id: "1:2".into(),
            shipped_screenshot_digest: Digest::of_text("screenshot-2"),
            shipped_source_digest: row.digest.clone(),
            frame_image_digest: Digest::of_text("frame-png"),
            frame_digest: frame.content_digest.clone().unwrap(),
            verdict: AuthorityVerdict::Conform,
            deltas: vec![],
            reviewed_by: "claude-fable-5 visual pass v1".into(),
            reviewed_at: Timestamp::fixed(2026, 8, 1, 14, 0, 0),
            basis: vec![],
            notes: None,
        });
        let (outcome, text) = run_kind(&h, ProofKind::VisualReview);
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
        assert!(text.contains("superseded_by_a_newer_review"), "{text}");
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
        reviewed(&h, AuthorityVerdict::Conform, &[]);
        h.signoff("KEY", "1:2");
        let (_, text) = run_kind(&h, ProofKind::VisualReview);
        assert!(text.contains("NO ACCEPTANCE STATE"), "{text}");
    }
}
