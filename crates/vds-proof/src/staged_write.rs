//! The `staged_write` proof. Four gates in ONE kind, plus the bypass rule.
//!
//! Draft S-7E, ENACTMENT PENDING (SUBMISSION-VDS-020). The sixteenth kind, and
//! the first whose subject is an act that has not happened yet.
//!
//! # WHAT A PASS HERE ESTABLISHES, AND WHAT IT CANNOT
//!
//! It establishes that every staged write VDS holds clears four gates, and that
//! every frame the estate names carries a content digest something VDS holds
//! accounts for. It DOES NOT establish that VDS is the only writer to the Figma
//! file, and no finding, note or refusal in this file may say that it does.
//! Three measured facts settle it: the REST API cannot write document nodes, so
//! VDS holds no privileged channel it could withhold; credential custody is the
//! inverse of a control, because VDS needs a token to READ and the plugin bridge
//! needs none to WRITE; and the estate's writer lock says on its own face that
//! it is advisory. `vds stage apply` takes that lock, which makes VDS a
//! co-operating writer where it was not one before. That is the whole of the
//! claim.
//!
//! # THE GATES ARE RE-DERIVED HERE AND NEVER READ OFF THE RECORD
//!
//! A record that carried its own verdicts and was believed could be MINTED: a
//! recorder writes `cleared` four times and the gate that refuses a boundary at
//! a fifth of its floor never runs. Three of `visual_review`'s four earning
//! conditions were found on the downstream estate by exactly that act. So
//! [`read_gates`] recomputes every verdict from the intent, the shipped
//! stylesheet, the screen register and the estate's route bindings, and R7
//! reports a record whose stored verdict disagrees with the one this run
//! measured.
//!
//! All four gates are LOCAL FILE READS. VDS S-7(2)(1) forbids a network call
//! or a model call inside a proof, and the network stays behind `vds-figma`
//! exactly as `pull.rs` puts it behind `FigmaSource`, so nothing in the proof
//! path can accidentally acquire a network dependency. `vds stage apply`
//! re-captures the live frame and is therefore NOT a proof and can never be one.
//!
//! # The rules
//!
//! A row is one STAGED WRITE, or one FRAME the bypass rule examines. Both
//! populations go through one [`Coverage`] tally, checked before anything
//! prints.
//!
//!   R1  a stage record that does not validate. An invalid record carries no
//!       verdict, so its write is unreviewed. Fatal, UNSCORED.
//!   R2  an unapplied stage missing a gate verdict entirely. A gate absent from
//!       the record and a gate that cleared are the same green to anybody
//!       counting refusals, which is how a gate stops running unnoticed.
//!   R3  a gate REFUSES this staged write. Fatal, naming the gate and its
//!       reason. `vds stage plan` refuses to emit against a refusal, so a stage
//!       that reaches here carrying one was either planned before the intent
//!       moved or planned by something that did not use the door.
//!   R4  the intent no longer digests to what the record pinned. The verdicts
//!       were read over a different file.
//!   R5  BYPASS. A frame the estate names whose CURRENT content digest matches
//!       NEITHER the digest at its last sign-off NOR the digest after any
//!       applied stage. Something wrote it that did not come through VDS.
//!       Fatal, named per frame, and NOT curable by re-running.
//!   R6  an apply with no verification, or a verification with a residual. The
//!       bridge caps one call and offers no transaction, so a partial apply is
//!       reachable and only a re-capture that finds the delta EMPTY declares
//!       success.
//!   R7  a recorded gate verdict this run does not reproduce. The record says
//!       one thing and the measurement says another.
//!   W1  a gate that COULD NOT RUN, per stage and per gate, summarised on the
//!       face of every run. A rule that cannot run must not read as a rule that
//!       ran and found nothing, and G4 in particular reports could_not_run on
//!       most rows wherever no binding ledger exists: A SINGLE UNOPPOSED
//!       SELF-CLAIM MUST NEVER READ AS AGREEMENT.
//!   W2  a frame with NO BASELINE: no sign-off row and no applied stage, so
//!       nothing VDS holds records what its content was. Not a bypass, and not
//!       a pass: the bypass rule has no baseline to compare against.
//!   W3  an edited plan: the operation list nobody reviewed.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;

use vds_core::{
    Digest, FloorScope, GateReading, GateVerdict, Project, ProofKind, Result, ReviewRegion,
    RouteBindingLedger, ScreenRecord, StageGate, StageIntent, StageRecord, Timestamp, VdsError,
    Violation,
};
use vds_css::colour::{self, Colour};
use vds_css::sheet::Sheet;
use vds_figma::frames::{self, FrameLedger};

use crate::ProofContext;
use crate::contrast::{custom_property, recorded_ratio, redacted};
use crate::run::{Outcome, ProofRun, Verdict};
use crate::screen_parity::Coverage;

pub const GATE: &str = "crates/vds-proof/src/staged_write.rs";

/// The floor a staged CONTROL BOUNDARY must clear, in every theme.
///
/// WCAG 2.2 SC 1.4.11, and a REQUIREMENT rather than a realisation: it is the
/// same shape `ContrastFloor::min_ratio` already carries in the register, which
/// VDS S-2(6) settles as lawful. It is a constant here rather than a config key
/// because a project that could lower it could lower it to the value that
/// cannot fail, which `contrast` R7 already refuses one level down.
pub const CONTROL_BOUNDARY_FLOOR: f64 = 3.0;

/// How far a staged box may fall outside the canonical shell before G3 refuses.
const SHELL_SLACK: f64 = 1.0;

const RULE_INVALID: &str =
    "draft S-7E(2) staged_write R1: a stage record that does not validate carries no verdict";
const RULE_GATE_NOT_ASKED: &str = "draft S-7E(3) staged_write R2: every gate is asked, because a gate absent from the record \
     and a gate that cleared are the same green to anybody counting refusals";
const RULE_REFUSED: &str = "draft S-7E(8) staged_write R3: a gate refuses this staged write";
const RULE_INTENT_MOVED: &str = "draft S-7E(9) staged_write R4: the verdicts were read over the intent the record pins, \
     and that file has moved";
const RULE_BYPASS: &str = "draft S-7E(10) staged_write R5: a frame whose current content matches neither its sign-off \
     nor any applied stage was written by something that did not come through VDS";
const RULE_UNVERIFIED: &str = "draft S-7E(11) staged_write R6: an apply is an ATTEMPT, and only a re-capture that finds \
     the delta EMPTY declares success";
const RULE_VERDICT_DISAGREES: &str = "draft S-7E(12) staged_write R7: a recorded gate verdict this run does not reproduce. A \
     record believed rather than recomputed can be MINTED";
const RULE_COULD_NOT_RUN: &str = "draft S-7E(13) staged_write W1: a gate that COULD NOT RUN, which is not the same answer \
     as a gate that cleared";
const RULE_NO_BASELINE: &str = "draft S-7E(14) staged_write W2: a frame with no sign-off and no applied stage, so the \
     bypass rule has no baseline to compare against";
const RULE_EDITED_PLAN: &str =
    "draft S-7E(15) staged_write W3: an edited plan is an operation list nobody reviewed";
const RULE_CAPTURE_AGE: &str = "draft S-7E(10)(b) staged_write R5: the bypass rule refuses on an OVER-AGE CAPTURE, because \
     a bypass check that is silent against a stale reading is a check that cannot fail";

// Skip reasons. Stable machine keys and never sentences: each becomes a count
// in `rows_skipped_reasons`, and a per-row sentence would make every count one.
const SKIP_INVALID: &str = "stage_record_does_not_validate";
const SKIP_APPLIED_AND_VERIFIED: &str = "applied_and_verified_at_the_frame";
const SKIP_APPLIED_UNVERIFIED: &str = "applied_and_never_verified_at_the_frame";
const SKIP_INTENT_UNREADABLE: &str = "the_pinned_intent_could_not_be_read";
const SKIP_NOTHING_MEASURED: &str = "every_gate_could_not_run_so_nothing_was_measured";
const SKIP_NO_FRAME_FOR_ROUTE: &str = "the_estate_enumerates_a_route_no_record_binds_to_a_frame";
const SKIP_NO_LEDGER_ROW: &str = "no_row_in_the_frames_ledger_for_the_named_frame";
const SKIP_NO_CURRENT_DIGEST: &str = "the_frames_ledger_row_carries_no_content_digest";
const SKIP_NO_CAPTURE_DATE: &str = "the_frames_ledger_states_no_capture_date";
const SKIP_STALE_CAPTURE: &str = "the_capture_is_older_than_the_declared_maximum";
const SKIP_NO_BASELINE: &str = "no_signoff_and_no_applied_stage_so_there_is_no_baseline";

pub const REACH_NOTE: &str = "[reach] this run establishes that every staged write VDS holds clears its four gates, and \
     that every frame the estate names carries a content digest a sign-off or an applied stage \
     accounts for. IT ESTABLISHES NOTHING ABOUT WHO MAY WRITE TO THE FIGMA FILE. The REST API \
     cannot write document nodes, so VDS holds no privileged channel it could withhold; VDS \
     needs a token to READ and the plugin bridge needs none to WRITE, so credential custody is \
     the inverse of a control here; and the writer lock `vds stage apply` takes says on its own \
     face that it is ADVISORY and cannot stop a writer that does not ask for it. What is new is \
     that the operation list exists on disk before anything reaches the canvas, that the \
     operation vocabulary is CLOSED, and that a write nobody staged is now visible after the \
     fact.";

pub const VOCABULARY_NOTE: &str = "[vocabulary] the operation set is CLOSED at six - create-band, set-box, set-name, \
     set-paint, reorder and delete-band - and there is no page-level and no frame-level delete. \
     The 2026-07-25 loss on the subscribing estate came from a build step whose correct and \
     documented behaviour is to delete a page of a given name and recreate it; a second writer \
     ran it and discarded work the first had landed. Neither agent was at fault and the step was \
     not wrong: the destructive verb existed and two writers reached it. WIDENING THE VOCABULARY \
     MAKES THAT LOSS REPEATABLE THROUGH THE SANCTIONED PATH. A delete reaches one band, and only \
     where its name is in the closed review vocabulary AND the intent no longer declares it; a \
     node VDS did not create and the intent does not name is never touched.";

pub const NO_AUTHORITY_NOTE: &str = "[authority] a machine act creates no authority ([2026] VJS-SC-OPBOX 1 order 16). VDS may \
     stage, plan, apply and verify. It may not sign, may not set a signed status, and emits \
     nothing that reads as a signature: an applied stage adds a digest to the set the bypass \
     rule accepts and confers no taste on the drawing. Taste is exercised once, at frame \
     sign-off, by the signer.";

pub const ATOMICITY_NOTE: &str = "[atomicity] there is none, and this run does not claim any. The plugin bridge caps one \
     call's code at a fixed character budget and offers no transaction, so a large frame's plan \
     goes over in ordered, digest-pinned chunks and the third can fail after the first two \
     landed. That is why an apply records an ATTEMPT and a separate re-capture declares success: \
     idempotence is MEASURED AT THE DESTINATION and never asserted in a comment.";

// ------------------------------------------------------------------ the gates

/// Everything the four gates read. All of it local.
pub struct GateInputs<'a> {
    pub intent: &'a StageIntent,
    /// The shipped stylesheet, parsed, or `None` where it could not be read.
    pub sheet: Option<&'a Sheet>,
    /// Where that stylesheet was read from, for a finding a reader can open.
    pub stylesheet_path: &'a str,
    /// The screen register, for G2's second limb.
    pub screens: &'a [ScreenRecord],
    /// The estate's own route-to-frame claims, for G4.
    pub bindings: Option<&'a RouteBindingLedger>,
    /// Custom properties a binding order has reserved.
    pub reserved_properties: &'a [String],
}

/// Read all four gates over one intent. ONE implementation, two callers.
///
/// `vds stage plan` calls this to decide whether it may emit, and the proof
/// calls it to decide whether a staged write holds. Two implementations would
/// be two front doors and two walls, and the second wall is the one nobody
/// maintains.
pub fn read_gates(inputs: &GateInputs) -> Vec<GateVerdict> {
    vec![
        contrast_floor(inputs),
        band_naming(inputs),
        canonical_geometry(inputs),
        route_binding(inputs),
    ]
}

fn verdict(gate: StageGate, reading: GateReading, because: impl Into<String>) -> GateVerdict {
    GateVerdict {
        gate,
        reading,
        because: because.into(),
    }
}

/// G1. A staged CONTROL BOUNDARY clears its floor in every theme the shipped
/// stylesheet declares.
///
/// A paint must name a CUSTOM PROPERTY plus a role plus its backdrop property.
/// A literal is refused, and not out of pedantry: a literal cannot be measured
/// against the shipped record in every theme, so a boundary spelled as a value
/// is a boundary nothing can check, which is the founding defect of this whole
/// system wearing a Figma hat.
///
/// A RESERVED property is refused rather than resolved. Where a binding order
/// has reserved what a property takes in some theme, choosing a value for it
/// would be VDS legislating over a court's reservation, and this gate measures;
/// it never picks.
fn contrast_floor(inputs: &GateInputs) -> GateVerdict {
    let staged: Vec<(&ReviewRegion, &vds_core::PaintIntent)> = inputs
        .intent
        .bands
        .iter()
        .filter_map(|b| b.paint.as_ref().map(|p| (&b.band, p)))
        .filter(|(_, p)| p.role == FloorScope::ControlBoundary)
        .collect();

    if staged.is_empty() {
        return verdict(
            StageGate::ContrastFloor,
            GateReading::CouldNotRun,
            "this intent stages no paint in the control_boundary role, so there is no boundary \
             to measure. That is a rule with no subject and never a pass: a paint in another \
             role carries no floor here, and one in this role would.",
        );
    }

    let Some(sheet) = inputs.sheet else {
        return verdict(
            StageGate::ContrastFloor,
            GateReading::Refused,
            format!(
                "{} staged control boundary(ies) and {} could not be read as a stylesheet, so \
                 not one ratio could be measured. A boundary VDS cannot measure is not a \
                 boundary that passed.",
                staged.len(),
                inputs.stylesheet_path
            ),
        );
    };
    let themes: Vec<String> = sheet
        .theme_selectors()
        .into_iter()
        .map(str::to_owned)
        .collect();
    if themes.is_empty() {
        return verdict(
            StageGate::ContrastFloor,
            GateReading::Refused,
            format!(
                "{} declares no theme scope at all, and a staged boundary has to clear its floor \
                 in every theme. Either this is not the record the paints are written against, \
                 or the palette does not live in custom properties; either way nothing here can \
                 be measured, and a cleared reading would read as a clean sheet.",
                inputs.stylesheet_path
            ),
        );
    }

    let mut cleared = 0usize;
    for (band, paint) in &staged {
        for raw in [&paint.property, &paint.backdrop] {
            if custom_property(raw).is_none() {
                return verdict(
                    StageGate::ContrastFloor,
                    GateReading::Refused,
                    format!(
                        "band {band} stages a paint whose property or backdrop is not a custom \
                         property name. The text is not repeated here: where it is a value, this \
                         paint is the storing form VDS S-2(2) forbids, and a literal cannot be \
                         measured against the shipped record in any theme."
                    ),
                );
            }
        }
        for raw in [&paint.property, &paint.backdrop] {
            let key = normalised_property(raw);
            if inputs
                .reserved_properties
                .iter()
                .any(|r| normalised_property(r) == key)
            {
                return verdict(
                    StageGate::ContrastFloor,
                    GateReading::Refused,
                    format!(
                        "band {band} stages {}, whose value in some theme is RESERVED by a \
                         binding order this project declares in `[stage] \
                         reserved_paint_properties`. This gate measures and never picks: \
                         choosing a value a court reserved would be VDS legislating. Stage a \
                         property the order leaves open, or move the order.",
                        redacted(raw)
                    ),
                );
            }
        }

        let boundary = custom_property(&paint.property).expect("checked above");
        let against = custom_property(&paint.backdrop).expect("checked above");
        for theme in &themes {
            match measure(sheet, theme, &boundary, &against) {
                Err(why) => {
                    return verdict(
                        StageGate::ContrastFloor,
                        GateReading::Refused,
                        format!(
                            "band {band}: {} against {} could not be measured in {}: {why}. A \
                             boundary VDS cannot measure is not a boundary that passed.",
                            redacted(&boundary),
                            redacted(&against),
                            redacted(theme)
                        ),
                    );
                }
                Ok(ratio) if !colour::meets_floor(ratio, CONTROL_BOUNDARY_FLOOR) => {
                    return verdict(
                        StageGate::ContrastFloor,
                        GateReading::Refused,
                        format!(
                            "band {band}: {} against {} reads {}:1 in {}, below the \
                             {CONTROL_BOUNDARY_FLOOR}:1 a control boundary carries under WCAG 2.2 \
                             SC 1.4.11. Neither resolved value is repeated here; open those two \
                             properties in that scope.",
                            redacted(&boundary),
                            redacted(&against),
                            recorded_ratio(ratio),
                            redacted(theme)
                        ),
                    );
                }
                Ok(_) => cleared += 1,
            }
        }
    }

    verdict(
        StageGate::ContrastFloor,
        GateReading::Cleared,
        format!(
            "{} staged control boundary(ies) measured in {} theme scope(s), {cleared} \
             (boundary, theme) reading(s), all at or above the {CONTROL_BOUNDARY_FLOOR}:1 floor.",
            staged.len(),
            themes.len()
        ),
    )
}

fn normalised_property(raw: &str) -> String {
    raw.trim().trim_start_matches("--").to_lowercase()
}

/// One (boundary, backdrop, theme) reading, or why it could not be taken.
///
/// The reason is a stable CLASS and never the CSS layer's own message: several
/// of those quote the value they refused, and a captured proof record lands
/// under the tree `no_stored_values` scans.
fn measure(
    sheet: &Sheet,
    theme: &str,
    boundary: &str,
    against: &str,
) -> std::result::Result<f64, &'static str> {
    let foreground = resolved(sheet, theme, boundary)?;
    let background = resolved(sheet, theme, against)?;
    let Ok(backdrop) = background.require_opaque() else {
        return Err(
            "the backdrop resolves to a translucent value, so it has no luminance of its own and \
             the boundary has no ratio until the surface behind it is named",
        );
    };
    let painted = foreground.composite_over(&backdrop);
    Ok(colour::contrast_ratio(&painted, &backdrop))
}

fn resolved(
    sheet: &Sheet,
    theme: &str,
    property: &str,
) -> std::result::Result<Colour, &'static str> {
    let resolution = sheet.resolve(theme, property);
    if !resolution.conditional.is_empty() {
        return Err(
            "the property is declared again under a conditional at-rule, so the value measured \
             here is not the only value it takes",
        );
    }
    let Some(value) = resolution.value() else {
        return Err("the property does not resolve in this theme or in the base");
    };
    colour::parse(value)
        .map_err(|_| "the property resolves to something this instrument cannot read as a colour")
}

/// G2. Band naming, in three limbs.
///
/// (a) A band name outside the CLOSED seven-value review vocabulary is
/// UNREPRESENTABLE rather than refused: [`vds_core::BandIntent::band`] is a
/// [`ReviewRegion`], so an intent naming a band the vocabulary does not carry
/// fails to DESERIALISE. That is the strongest available form of the rule and
/// it is why this limb has no branch here; the red seed for it is a YAML
/// fixture that will not parse.
///
/// (b) A band the SCREEN RECORD does not declare is refused. A staged rail on a
/// screen that has none is a write about another screen.
///
/// (c) THE LIMB THAT MAKES THE DIFF KEY STABLE. The diff is keyed on band name,
/// not on node id, because node ids change when a node is recreated and a
/// node-keyed diff sees every band as missing on the second run. A duplicate
/// band declaration is therefore two answers for one key, and the apply would
/// write one of them without saying which.
fn band_naming(inputs: &GateInputs) -> GateVerdict {
    let intent = inputs.intent;

    // (c) first: a broken key makes the other two limbs answer about a diff
    // that could not be run.
    let defects = intent.defects();
    if !defects.is_empty() {
        return verdict(
            StageGate::BandNaming,
            GateReading::Refused,
            format!(
                "the intent does not hold together, so the diff key is not stable: {}",
                defects.join(" ")
            ),
        );
    }

    let Some(screen) = inputs.screens.iter().find(|s| s.route == intent.route) else {
        return verdict(
            StageGate::BandNaming,
            GateReading::CouldNotRun,
            format!(
                "no screen record names route {:?}, so there is nothing to compare the staged \
                 bands against. This is NOT a pass: it is the correspondence limb having no \
                 basis to run on. Register the screen with `vds screen add`.",
                intent.route
            ),
        );
    };
    if let Some(why) = screen.band_correspondence_unrunnable_because() {
        return verdict(StageGate::BandNaming, GateReading::CouldNotRun, why);
    }

    let staged = intent.declared_bands();
    let foreign = screen.bands_not_drawn(&staged);
    if !foreign.is_empty() {
        return verdict(
            StageGate::BandNaming,
            GateReading::Refused,
            format!(
                "this write stages {} that {} does not declare: {}. A staged band the screen \
                 does not have is a write about another screen, and it would also put a key in \
                 the diff that nothing on the far side answers.",
                if foreign.len() == 1 {
                    "a band"
                } else {
                    "bands"
                },
                screen.id,
                foreign
                    .iter()
                    .map(|b| b.as_str())
                    .collect::<Vec<&str>>()
                    .join(", ")
            ),
        );
    }

    verdict(
        StageGate::BandNaming,
        GateReading::Cleared,
        format!(
            "{} staged band(s), every one in the closed review vocabulary and every one declared \
             by {}. The diff key is therefore stable across a recreate, which is what makes the \
             second apply emit nothing.",
            staged.len(),
            screen.id
        ),
    )
}

/// G3. The SAME derivation `vds figma frames` runs AFTER the write, run over the
/// intent's boxes BEFORE it.
fn canonical_geometry(inputs: &GateInputs) -> GateVerdict {
    let intent = inputs.intent;
    let boxed = intent.bands.iter().filter(|b| b.box_of.is_some()).count();
    if boxed == 0 {
        return verdict(
            StageGate::CanonicalGeometry,
            GateReading::CouldNotRun,
            "this intent declares no box on any band, so there is nothing to derive a column \
             count from. Not a pass: an intent that positions nothing states no geometry, and \
             the derivation has no input.",
        );
    }

    let shell = vds_core::BandBox {
        x: 0.0,
        y: 0.0,
        width: vds_figma::stage::SHELL_WIDTH,
        height: vds_figma::stage::SHELL_HEIGHT,
    };
    for band in &intent.bands {
        let Some(box_of) = band.box_of else { continue };
        if !box_of.fits_inside(&shell, SHELL_SLACK) {
            return verdict(
                StageGate::CanonicalGeometry,
                GateReading::Refused,
                format!(
                    "band {} is positioned outside the canonical shell, so the write would put \
                     it off the frame. The shell's dimensions are constants in the generator and \
                     not a config key, because they are lengths and a length under the record is \
                     the storing form VDS S-2(2) prohibits.",
                    band.band
                ),
            );
        }
    }

    let document = vds_figma::stage::synthetic_document(intent);
    let config = vds_core::ScreensConfig::default();
    let (derived, truncated) = frames::columns_of(&document, &config);
    if truncated {
        return verdict(
            StageGate::CanonicalGeometry,
            GateReading::CouldNotRun,
            "the derivation read the boundary of the document built from these boxes, so its \
             count states an absence it could not observe. Declare the panes each band draws.",
        );
    }
    if derived != intent.columns {
        return verdict(
            StageGate::CanonicalGeometry,
            GateReading::Refused,
            format!(
                "the intent declares {} content column(s) and the SAME derivation `vds figma \
                 frames` will run after the write reads {derived} from the boxes it is about to \
                 write. One of the two is wrong, and finding that out after the write means \
                 finding it out from a ledger that already disagrees with the drawing.",
                intent.columns
            ),
        );
    }

    verdict(
        StageGate::CanonicalGeometry,
        GateReading::Cleared,
        format!(
            "{boxed} positioned band(s), all inside the canonical shell, and the ledger's own \
             derivation reads {derived} content column(s) from them, which is what the intent \
             declares."
        ),
    )
}

/// G4. VDS DOES NOT DECIDE WHICH CLAIM IS TRUE.
///
/// Deciding needs eyes on the drawing, and VDS has none: it reads names, node
/// ids and digests. So a contradiction is REFUSED and BOTH claims are named,
/// and the reader opens the two artefacts.
///
/// This is not hypothetical. An audit on the subscribing estate confirmed two
/// false mappings out of thirty-three disputed, and one of them drove a live
/// code change on a shipped route. Those two are this gate's red fixtures.
fn route_binding(inputs: &GateInputs) -> GateVerdict {
    let intent = inputs.intent;
    let staged = frames::normalise_node_id(&intent.node_id);

    let Some(ledger) = inputs.bindings else {
        return verdict(
            StageGate::RouteBinding,
            GateReading::CouldNotRun,
            "this project supplies no route binding ledger, so nothing in the repository is in a \
             position to contradict this write's target. THAT IS NOT AGREEMENT: it is one \
             unopposed self-claim, and an unopposed self-claim must never read as agreement. \
             Generate one with `vds ledger route-bindings --from <the estate's own registry>`.",
        );
    };
    match ledger.untrustworthy_because() {
        Ok(Some(why)) => {
            return verdict(StageGate::RouteBinding, GateReading::Refused, why);
        }
        Err(_) => {
            return verdict(
                StageGate::RouteBinding,
                GateReading::Refused,
                "the route binding ledger could not be digested, so its claims cannot be relied \
                 on and this gate cannot tell a contradiction from a transcription error."
                    .to_owned(),
            );
        }
        Ok(None) => {}
    }

    let claims = ledger.claims_for(&intent.route);
    if claims.is_empty() {
        return verdict(
            StageGate::RouteBinding,
            GateReading::CouldNotRun,
            format!(
                "the route binding ledger carries no claim about {:?}, so nothing opposes this \
                 write's target. Not a pass: it is one unopposed self-claim, and the ledger says \
                 what it does not cover on its own face.",
                intent.route
            ),
        );
    }
    let contradicting: Vec<&vds_core::RouteBinding> = claims
        .iter()
        .copied()
        .filter(|c| frames::normalise_node_id(&c.node_id) != staged)
        .collect();
    if !contradicting.is_empty() {
        let named = contradicting
            .iter()
            .map(|c| format!("{} (claimed at {})", c.node_id, c.claimed_at))
            .collect::<Vec<String>>()
            .join("; ");
        return verdict(
            StageGate::RouteBinding,
            GateReading::Refused,
            format!(
                "two artefacts disagree about which frame draws {:?}. This write targets {staged} \
                 ({}); the estate's own record says {named}. VDS DOES NOT DECIDE WHICH IS TRUE, \
                 because deciding needs eyes on the drawing and this engine reads names, node ids \
                 and digests. Open both and settle it, then re-stage. Writing to the wrong frame \
                 is not a cosmetic error: on the estate this gate was written for, a false \
                 mapping of exactly this shape drove a live code change on a shipped route.",
                intent.route, ledger.source
            ),
        );
    }

    verdict(
        StageGate::RouteBinding,
        GateReading::Cleared,
        format!(
            "{} claim(s) in {} bind {:?} to this write's target and none contradicts it.",
            claims.len(),
            ledger.source,
            intent.route
        ),
    )
}

// ------------------------------------------------------------------- the proof

/// What one row turned out to be. Consumed by [`score`], which is the only
/// place a row is counted, so the coverage tally and `rows_considered` cannot
/// drift apart.
enum Scoring {
    Scored,
    Unscored(&'static str),
    Excluded(&'static str),
}

fn score(run: &mut ProofRun, coverage: &mut Coverage, scoring: Scoring) {
    match scoring {
        Scoring::Scored => {
            coverage.scored += 1;
            run.row(Verdict::Enforced);
        }
        Scoring::Unscored(reason) => {
            coverage.unscored += 1;
            run.row(Verdict::Skipped(reason));
        }
        Scoring::Excluded(reason) => {
            coverage.excluded += 1;
            run.row(Verdict::Skipped(reason));
        }
    }
}

pub fn run(ctx: &ProofContext, out: &mut dyn Write) -> Result<Outcome> {
    let project = ctx.project;
    let store = ctx.store();
    let mut run = ctx.new_run(ProofKind::StagedWrite, GATE);
    run.input_file(&project.config_path)?;
    run.note(REACH_NOTE);
    run.note(VOCABULARY_NOTE);
    run.note(NO_AUTHORITY_NOTE);
    run.note(ATOMICITY_NOTE);

    // THE ONE PRECONDITION THAT IS A RULE OF LAW, asked before any file is
    // read. An intent carries boxes and paints; under `.vds/` they are the
    // storing form VDS S-2(2) prohibits, `no_stored_values` R1 and R3 would
    // fail on them forever on a file VDS wrote itself, and a record is never
    // deleted, so there is no lawful way back.
    if let Some(why) = vds_core::intent_root_defect(project) {
        return Err(VdsError::precondition(format!(
            "[stage] intent_root {why}\n  This proof did not run: it will not read an intent it \
             would be unlawful to have written."
        )));
    }

    let stages = store.read_stages()?;
    for located in &stages {
        run.input_file(&located.path)?;
    }
    let screen_records = store.read_screens()?;
    for located in &screen_records {
        run.input_file(&located.path)?;
    }
    let signoffs = store.read_signoffs()?;
    for located in &signoffs {
        run.input_file(&located.path)?;
    }
    let screens: Vec<ScreenRecord> = screen_records.iter().map(|l| l.value.clone()).collect();

    let bindings = vds_core::read_route_bindings(project)?;
    if bindings.is_some() {
        run.input_file(&vds_core::route_bindings_path(project))?;
    }
    let manifest = vds_core::read_route_manifest(project)?;
    if manifest.is_some() {
        run.input_file(&vds_core::route_manifest_path(project))?;
    }

    // The stylesheet, read only where something stages a paint. Demanding it
    // unconditionally would make this kind exit 2 on every project that has
    // staged nothing, and an exit 2 that means "nothing to do" teaches a reader
    // to ignore the one that means "the record was never opened".
    let stylesheet_path = project.root.join(&project.config.surface.stylesheet);
    let stylesheet_rel = project.rel(&stylesheet_path);
    let sheet = if stages.is_empty() || !stylesheet_path.is_file() {
        None
    } else {
        let text = std::fs::read_to_string(&stylesheet_path)
            .map_err(|e| VdsError::io(&stylesheet_rel, e))?;
        let parsed = Sheet::parse(&text);
        if parsed.malformed().is_some() {
            None
        } else {
            run.input_file(&stylesheet_path)?;
            Some(parsed)
        }
    };

    let frames_ledger = frames::read(project)?;
    if let Some(ledger) = &frames_ledger {
        frames::check_fresh(ledger, None)?;
        run.input_named("frames-ledger", ledger.content_digest.clone());
    }

    let mut coverage = Coverage::default();
    let mut could_not_run: BTreeMap<StageGate, u32> = BTreeMap::new();

    // ------------------------------------------------ population A: the stages
    for located in &stages {
        let record = &located.value;
        let at = format!("{} [{}]", record.id, record.route);

        let defects = record.defects();
        if !defects.is_empty() {
            score(&mut run, &mut coverage, Scoring::Unscored(SKIP_INVALID));
            for defect in defects {
                run.fail(Violation::fatal(
                    at.clone(),
                    RULE_INVALID,
                    "a stage record naming a route, a target frame and an intent outside `.vds/`, \
                     with one reading per gate and a sentence on each",
                    defect,
                ));
            }
            continue;
        }

        if let Some(apply) = &record.apply {
            match &apply.verification {
                Some(v) if v.succeeded() => {
                    score(
                        &mut run,
                        &mut coverage,
                        Scoring::Excluded(SKIP_APPLIED_AND_VERIFIED),
                    );
                    continue;
                }
                Some(v) => {
                    score(
                        &mut run,
                        &mut coverage,
                        Scoring::Unscored(SKIP_APPLIED_UNVERIFIED),
                    );
                    run.fail(Violation::fatal(
                        at.clone(),
                        RULE_UNVERIFIED,
                        "a re-capture that recomputes the delta and finds it EMPTY",
                        format!(
                            "the re-capture still emits {} operation(s), so this apply did not \
                             finish. The bridge caps one call and offers no transaction, so a \
                             chunk can fail after earlier chunks landed. Re-run `vds stage apply` \
                             and verify again; do not record this as done.",
                            v.residual_operations
                        ),
                    ));
                    continue;
                }
                None => {
                    score(
                        &mut run,
                        &mut coverage,
                        Scoring::Unscored(SKIP_APPLIED_UNVERIFIED),
                    );
                    run.fail(Violation::fatal(
                        at.clone(),
                        RULE_UNVERIFIED,
                        "a re-capture that recomputes the delta and finds it EMPTY",
                        "this stage records an apply and no verification. An apply is an \
                         ATTEMPT: there is no atomicity anywhere on this path, so a partial \
                         write is reachable and only a measurement at the destination declares \
                         success. Run: vds stage verify --id <id> --from <a fresh capture>"
                            .to_owned(),
                    ));
                    continue;
                }
            }
        }

        // UNAPPLIED. Re-derive every gate rather than believing the record.
        let intent_path = project.root.join(&record.intent_path);
        let intent: StageIntent = match vds_core::read_intent(project, &intent_path) {
            Ok(intent) => intent,
            Err(error) => {
                score(
                    &mut run,
                    &mut coverage,
                    Scoring::Unscored(SKIP_INTENT_UNREADABLE),
                );
                run.fail(Violation::fatal(
                    at.clone(),
                    RULE_INTENT_MOVED,
                    format!("a readable intent at {}", record.intent_path),
                    format!(
                        "{error}. A staged write whose intent cannot be read states nothing, and \
                         the verdicts on the record were taken over a file this run cannot see."
                    ),
                ));
                continue;
            }
        };
        run.input_file(&intent_path)?;

        let current = Digest::of_file(&intent_path)?;
        if current != record.intent_digest {
            score(
                &mut run,
                &mut coverage,
                Scoring::Unscored(SKIP_INTENT_UNREADABLE),
            );
            run.fail(Violation::fatal(
                at.clone(),
                RULE_INTENT_MOVED,
                format!(
                    "the intent at {} to still digest to {}",
                    record.intent_path, record.intent_digest
                ),
                format!(
                    "it digests to {current}. The gate readings on this record were taken over a \
                     different file, so they establish nothing about what would now be written. \
                     Re-stage: vds stage add re-reads the intent and re-derives every gate."
                ),
            ));
            continue;
        }

        // The plan, where one has been emitted. An edited plan is the operation
        // list nobody reviewed, which is the one thing this whole capability
        // buys.
        let plan_path = vds_core::plan_path(project, &record.id);
        if let Some(plan) = vds_core::read_plan(project, &plan_path)?
            && let Some(why) = plan.untrustworthy_because()?
        {
            run.warn(Violation::fatal(
                at.clone(),
                RULE_EDITED_PLAN,
                "a plan that still digests to what was emitted",
                why,
            ));
        }

        let measured = read_gates(&GateInputs {
            intent: &intent,
            sheet: sheet.as_ref(),
            stylesheet_path: &stylesheet_rel,
            screens: &screens,
            bindings: bindings.as_ref(),
            reserved_properties: &project.config.stage.reserved_paint_properties,
        });

        // R2: a gate absent from the record reads as green to anybody counting
        // refusals. Reported per gate.
        for gate in record.gates_not_asked() {
            run.fail(Violation::fatal(
                at.clone(),
                RULE_GATE_NOT_ASKED,
                format!("a recorded reading for gate {gate} ({})", gate.limb()),
                format!(
                    "the record carries no reading for {gate} at all. A gate absent from the \
                     record and a gate that cleared are the same green to anybody counting \
                     refusals, which is exactly how a gate stops running without anybody \
                     noticing. Re-stage: vds stage add asks all four."
                ),
            ));
        }

        let mut refusals = 0usize;
        let mut ran = 0usize;
        for reading in &measured {
            match reading.reading {
                GateReading::Refused => {
                    refusals += 1;
                    ran += 1;
                    run.fail(Violation::fatal(
                        format!("{at} <{}>", reading.gate),
                        RULE_REFUSED,
                        format!(
                            "{} to clear before anything reaches the canvas ({})",
                            reading.gate,
                            reading.gate.limb()
                        ),
                        reading.because.clone(),
                    ));
                }
                GateReading::Cleared => ran += 1,
                GateReading::CouldNotRun => {
                    *could_not_run.entry(reading.gate).or_default() += 1;
                    run.warn(Violation::fatal(
                        format!("{at} <{}>", reading.gate),
                        RULE_COULD_NOT_RUN,
                        format!("{} to have a basis to run on", reading.gate),
                        reading.because.clone(),
                    ));
                }
            }

            // R7: the record's own verdict, against the one this run measured.
            if let Some(stored) = record.verdict(reading.gate)
                && stored.reading != reading.reading
            {
                run.fail(Violation::fatal(
                    format!("{at} <{}>", reading.gate),
                    RULE_VERDICT_DISAGREES,
                    format!(
                        "the recorded reading for {} to be the one this run measures",
                        reading.gate
                    ),
                    format!(
                        "the record says {} and this run measures {}. A record believed rather \
                         than recomputed can be MINTED, and a minted `cleared` is a gate that \
                         never ran wearing the word green.",
                        stored.reading, reading.reading
                    ),
                ));
            }
        }

        if ran == 0 {
            // Nothing was measured at all. Never a pass.
            score(
                &mut run,
                &mut coverage,
                Scoring::Unscored(SKIP_NOTHING_MEASURED),
            );
        } else {
            score(&mut run, &mut coverage, Scoring::Scored);
        }
        let _ = refusals;
    }

    // ------------------------------------------------- population B: R5 bypass
    bypass(
        &mut run,
        &mut coverage,
        project,
        frames_ledger.as_ref(),
        &stages,
        &signoffs,
        &screen_records,
        manifest.as_ref(),
    )?;

    // The four gates' own coverage, on the face of every run. A gate that could
    // not run on most rows has to SAY SO: a single unopposed self-claim must
    // never read as agreement.
    for gate in StageGate::ALL {
        let n = could_not_run.get(&gate).copied().unwrap_or(0);
        run.note(format!(
            "[gate {gate}] could not run on {n} of {} staged write(s). A rule that cannot run is \
             not a rule that ran and found nothing.",
            stages.len()
        ));
    }

    // FINDING 7's discipline: the tally is checked BEFORE it is printed, so a
    // number that does not add up is a refusal rather than a figure nobody
    // adds up. `run.finish` has not been called, so nothing is captured.
    coverage.check_against(run.rows_considered())?;
    run.note(coverage.line_for("staged write(s) and named frame(s)"));

    run.finish(&ctx.capture_options()?, out)
}

/// R5. The limb that changes behaviour without needing anyone's co-operation.
///
/// For every frame the estate names, the frame's CURRENT content digest is
/// compared against the union of {the digest at its last sign-off} and {the
/// digest after every applied stage that targeted it}. A frame matching NEITHER
/// was written by something that did not come through VDS.
///
/// NOT CURABLE BY RE-RUNNING. It is cured by staging the state the frame is now
/// in, or by reverting the drawing.
///
/// # The trap, and how it is closed
///
/// This rule is SILENT AGAINST A STALE CAPTURE, which is the exact failure the
/// subscribing estate hit on 2026-08-02 on 23 of 188 routes: a ledger derived
/// from an old capture reports yesterday's digests, every one of them matches
/// yesterday's baseline, and today's bypass is invisible. So the age is measured
/// against the CAPTURE DATE and never against the ledger's `generated_at` -
/// regenerating from an old capture moves the latter and not the former - and a
/// ledger that states no capture date REFUSES rather than reporting no bypass.
#[allow(clippy::too_many_arguments)]
fn bypass(
    run: &mut ProofRun,
    coverage: &mut Coverage,
    project: &Project,
    ledger: Option<&FrameLedger>,
    stages: &[vds_store::Located<StageRecord>],
    signoffs: &[vds_store::Located<vds_core::SignOff>],
    screens: &[vds_store::Located<ScreenRecord>],
    manifest: Option<&vds_core::RouteManifest>,
) -> Result<()> {
    // Every frame the register names, plus every frame a stage targets.
    let mut named: BTreeMap<String, String> = BTreeMap::new();
    for located in screens {
        if let Some(frame) = &located.value.frame {
            named.insert(
                frames::normalise_node_id(&frame.node_id),
                located.value.route.clone(),
            );
        }
    }
    for located in stages {
        named
            .entry(frames::normalise_node_id(&located.value.target.node_id))
            .or_insert_with(|| located.value.route.clone());
    }

    // A route the estate enumerates and nothing binds to a frame. The manifest
    // enumerates ROUTES; the register is what maps a route to a frame, so a
    // manifest row with no screen record contributes no subject and says so
    // rather than disappearing.
    if let Some(manifest) = manifest {
        let bound: BTreeSet<&str> = screens
            .iter()
            .filter(|s| s.value.frame.is_some())
            .map(|s| s.value.route.as_str())
            .collect();
        for route in &manifest.routes {
            if !bound.contains(route.as_str()) {
                score(run, coverage, Scoring::Excluded(SKIP_NO_FRAME_FOR_ROUTE));
                run.inform(Violation::fatal(
                    route.clone(),
                    RULE_NO_BASELINE,
                    "a screen record naming the frame that draws this route",
                    "the estate enumerates this route and nothing VDS holds binds it to a frame, \
                     so the bypass rule has no subject here."
                        .to_owned(),
                ));
            }
        }
    }

    if named.is_empty() {
        return Ok(());
    }

    let Some(ledger) = ledger else {
        for (node_id, route) in &named {
            score(run, coverage, Scoring::Unscored(SKIP_NO_LEDGER_ROW));
            run.warn(Violation::fatal(
                format!("{node_id} [{route}]"),
                RULE_BYPASS,
                "a frame ledger carrying this frame's current content digest",
                format!(
                    "there is no frame ledger at all, so nothing measures what {node_id} \
                     currently contains and a write outside VDS would be invisible. Derive one: \
                     {} --from <capture> --captured-at <when the capture was taken>",
                    frames::GENERATOR_COMMAND
                ),
            ));
        }
        return Ok(());
    };

    // THE FRESHNESS LIMB, measured against the freshest INDEPENDENT input this
    // run read rather than against the clock. Reading the clock would make two
    // runs over identical inputs produce different findings, which is the
    // determinism limb of VDS S-7(2)(1) broken by an irrelevance; `burndown`
    // settles the same question the same way.
    let witness: Option<Timestamp> = stages
        .iter()
        .map(|s| s.value.staged_at.clone())
        .chain(signoffs.iter().map(|s| s.value.signed_at.clone()))
        .chain(manifest.map(|m| m.taken_at.clone()))
        .max();

    let stale = match (&ledger.captured_at, &witness) {
        (None, _) => Some(format!(
            "the frame ledger states no capture date, so this rule cannot tell a reading taken \
             this morning from one taken four days ago. IT IS THEREFORE REFUSED RATHER THAN \
             REPORTED CLEAN: a bypass check that is silent against a stale capture is a check \
             that cannot fail, and that is not hypothetical - it is how 23 of 188 routes on the \
             subscribing estate were read against a stale capture. Regenerate with: {} --from \
             <capture> --captured-at <when the capture was taken>",
            frames::GENERATOR_COMMAND
        )),
        (Some(captured), Some(witness)) => {
            let max = i64::from(project.config.stage.max_capture_age_days);
            match crate::geometry::days_between(captured.as_str(), witness.as_str()) {
                Some(age) if age > max => Some(format!(
                    "the capture was taken {} and the freshest independent record this run read \
                     is dated {}, which is {age} day(s) later and past the declared maximum of \
                     {max}. A bypass rule reading a stale capture reports yesterday's digests, \
                     every one of them matches yesterday's baseline, and today's write outside \
                     VDS is invisible. Re-capture and regenerate.",
                    captured.as_str(),
                    witness.as_str()
                )),
                _ => None,
            }
        }
        (Some(_), None) => None,
    };
    if let Some(why) = &stale {
        // ONE finding about the ledger, not one per frame: a stale capture is a
        // single fact and naming it per frame would bury it under itself.
        run.fail(Violation::fatal(
            project.rel(&frames::ledger_path(project)),
            RULE_CAPTURE_AGE,
            "a frame ledger stating WHEN its capture was taken, no older than the declared \
             maximum",
            why.clone(),
        ));
    }

    for (node_id, route) in &named {
        let at = format!("{node_id} [{route}]");
        if stale.is_some() {
            score(
                run,
                coverage,
                Scoring::Unscored(if ledger.captured_at.is_none() {
                    SKIP_NO_CAPTURE_DATE
                } else {
                    SKIP_STALE_CAPTURE
                }),
            );
            continue;
        }

        let Some(row) = ledger.row(node_id) else {
            score(run, coverage, Scoring::Unscored(SKIP_NO_LEDGER_ROW));
            run.warn(Violation::fatal(
                at,
                RULE_BYPASS,
                format!("a row in the frame ledger for {node_id}"),
                "the capture does not reach this frame, so nothing measures what it currently \
                 contains and a write outside VDS would be invisible here. Re-capture including \
                 it."
                .to_owned(),
            ));
            continue;
        };
        let Some(current) = &row.content_digest else {
            score(run, coverage, Scoring::Unscored(SKIP_NO_CURRENT_DIGEST));
            run.fail(Violation::fatal(
                at,
                RULE_BYPASS,
                "a content digest on the frame ledger row",
                "this row carries no current content digest, so the frame cannot be shown to \
                 match anything. Fail-closed: the rule refuses rather than reporting no bypass, \
                 because a frame with no measurable content is not a frame nobody wrote to. \
                 Regenerate the ledger with this build."
                    .to_owned(),
            ));
            continue;
        };

        let mut baseline: Vec<(Digest, String)> = Vec::new();
        for located in signoffs {
            if frames::normalise_node_id(&located.value.node_id) == *node_id {
                baseline.push((
                    located.value.frame_digest.clone(),
                    format!("sign-off {}", located.value.id),
                ));
            }
        }
        for located in stages {
            if frames::normalise_node_id(&located.value.target.node_id) == *node_id
                && let Some(apply) = &located.value.apply
                && let Some(v) = &apply.verification
                && v.succeeded()
            {
                baseline.push((
                    v.frame_digest_after.clone(),
                    format!("applied stage {}", located.value.id),
                ));
            }
        }

        if baseline.is_empty() {
            // NOT a bypass. A bypass claim needs a baseline, and this frame has
            // none: nothing VDS holds records what its content ever was.
            // Reporting it as a bypass would make this rule permanently red on
            // every estate on the day it is adopted, and a permanently red gate
            // is one people switch off.
            score(run, coverage, Scoring::Unscored(SKIP_NO_BASELINE));
            run.warn(Violation::fatal(
                at,
                RULE_NO_BASELINE,
                "a sign-off row or an applied stage recording what this frame's content was",
                "nothing VDS holds records a content digest for this frame, so the bypass rule \
                 has NO BASELINE and cannot tell a VDS write from any other. This is not a pass. \
                 Sign the frame off, or stage and apply through VDS, and the next run can tell \
                 them apart."
                    .to_owned(),
            ));
            continue;
        }

        score(run, coverage, Scoring::Scored);
        if !baseline.iter().any(|(digest, _)| digest == current) {
            run.fail(Violation::fatal(
                at,
                RULE_BYPASS,
                format!(
                    "this frame's current content to match one of {} recorded digest(s): {}",
                    baseline.len(),
                    baseline
                        .iter()
                        .map(|(_, what)| what.as_str())
                        .collect::<Vec<&str>>()
                        .join(", ")
                ),
                "it matches none of them, so this frame was written by something that did not \
                 come through VDS. THIS IS NOT CURED BY RE-RUNNING: the next run reads the same \
                 digest and says the same thing. It is cured by staging the state the frame is \
                 NOW in and applying it, or by reverting the drawing to a recorded one. Neither \
                 is a paperwork act, which is the point."
                    .to_owned(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
