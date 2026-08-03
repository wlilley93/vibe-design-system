//! The seeds for `staged_write`. EVERY GATE RED AND GREEN.
//!
//! A gate that has only been seen green is not evidence, and a gate that cannot
//! pass gets switched off within a week. So each of G1, G2, G3, G4 and R5
//! carries a fixture that MUST refuse and one that MUST pass, the vacuity case
//! is seeded too, and G4's red fixtures are the two REAL false mappings an audit
//! on the subscribing estate confirmed on 2026-08-02.

use super::*;
use crate::testing::{Harness, run_kind};
use vds_core::{
    BandBox, BandIntent, Digest, EXIT_PASSED, EXIT_VACUOUS, EXIT_VIOLATION, GateVerdict,
    PaintIntent, ProofStatus, STAGE_INTENT_SCHEMA_VERSION, StageIntent, Verification,
};

/// The shipped stylesheet, in two themes, with the two REAL token values the
/// estate ships.
///
/// `--border-control` reads 3.09:1 against the light surface and clears the
/// floor; `--border` reads 1.09:1 and fails it. That is the measured fact the
/// whole contrast programme on the subscribing estate was built around, and it
/// is why this fixture carries those values rather than invented ones.
const SHEET: &str = r#"
:root {
  --surface: #f5f5f5;
  --border: #ebebeb;
  --border-control: #748eaf;
  --ink-hair: #ebebeb;
}
.dark {
  --surface: #0a0a0a;
  --border: #2d2d2d;
  --border-control: #707070;
  --ink-hair: #2d2d2d;
}
"#;

fn box_of(x: f64, y: f64, width: f64, height: f64) -> BandBox {
    BandBox {
        x,
        y,
        width,
        height,
    }
}

fn band(region: ReviewRegion, b: BandBox) -> BandIntent {
    BandIntent {
        band: region,
        box_of: Some(b),
        panes: vec![],
        paint: None,
        order: None,
    }
}

fn rail() -> BandIntent {
    band(ReviewRegion::Rail, box_of(0.0, 48.0, 56.0, 824.0))
}

fn intent(route: &str, node_id: &str, columns: u32, bands: Vec<BandIntent>) -> StageIntent {
    StageIntent {
        schema_version: STAGE_INTENT_SCHEMA_VERSION,
        route: route.into(),
        file_key: "KEY".into(),
        node_id: node_id.into(),
        columns,
        bands,
        authored_by: "a harness".into(),
        authored_at: Timestamp::fixed(2026, 8, 3, 9, 0, 0),
        notes: None,
    }
}

/// A stylesheet, an intent in the subscriber tree, and one staged write.
fn staged(h: &Harness, wanted: &StageIntent, gates: Vec<GateVerdict>) -> vds_core::StageId {
    h.write("app/globals.css", SHEET);
    let rel = h.stage_intent("STG-0001", wanted);
    h.stage_record("STG-0001", &wanted.route, &wanted.node_id, &rel, gates)
}

fn cleared(gate: StageGate) -> GateVerdict {
    GateVerdict {
        gate,
        reading: GateReading::Cleared,
        because: "recorded by the harness".into(),
    }
}

/// Every gate recorded as cleared. Used wherever the SUBJECT of a seed is the
/// gate this run re-derives, so a stored verdict never stands in for the
/// measurement.
fn all_cleared() -> Vec<GateVerdict> {
    StageGate::ALL.into_iter().map(cleared).collect()
}

fn screen(h: &Harness, id: &str, route: &str, node_id: &str, bands: &[ReviewRegion]) {
    h.screen_record(id, route, 1, &["body"], Some(node_id));
    let owned: Vec<ReviewRegion> = bands.to_vec();
    h.amend_screen(id, |record| record.arrangement.bands = owned);
}

fn frames_captured_at(h: &Harness, frames: &[(String, serde_json::Value)], at: &str) {
    let project = h.project();
    let mut nodes = serde_json::Map::new();
    for (node_id, document) in frames {
        nodes.insert(
            node_id.clone(),
            serde_json::json!({"document": document.clone()}),
        );
    }
    let body = serde_json::json!({"nodes": nodes}).to_string();
    let ledger =
        vds_figma::frames::build_ledger("KEY", &[body], &project.config.screens, "a test capture")
            .expect("the frame ledger generator")
            .with_capture_date(Timestamp::parse(at).unwrap())
            .expect("a re-digested ledger");
    vds_figma::frames::write(&project, &ledger).expect("the frame ledger writer");
}

// ------------------------------------------------------------------- vacuity

/// A run with no staged record must report VACUOUS and say it is not evidence.
#[test]
fn a_run_with_no_staged_write_is_vacuous_and_says_it_is_not_evidence() {
    let h = Harness::new();
    let (outcome, text) = run_kind(&h, ProofKind::StagedWrite);
    assert_eq!(outcome.status, ProofStatus::Vacuous, "{text}");
    assert_eq!(outcome.exit_code, EXIT_VACUOUS);
    assert!(text.contains("VACUOUS"), "{text}");
    assert!(text.contains("NOT evidence"), "{text}");
    assert!(
        !text.contains("PASS:"),
        "no PASS may be printed beside a vacuity: {text}"
    );
}

/// The reach note is on every run, and it says what a pass here does NOT
/// establish. The brief this capability was built from asserted the opposite,
/// and the code has to keep refusing it.
#[test]
fn every_run_states_that_a_pass_here_establishes_nothing_about_who_may_write() {
    let h = Harness::new();
    let (_, text) = run_kind(&h, ProofKind::StagedWrite);
    assert!(
        text.contains("ESTABLISHES NOTHING ABOUT WHO MAY WRITE"),
        "the note that stops this kind being read as a write control is missing: {text}"
    );
    assert!(text.contains("ADVISORY"), "{text}");
    assert!(
        text.contains("CLOSED at six"),
        "the vocabulary note carries the reason the closure matters: {text}"
    );
}

// ------------------------------------------------------ G1: the contrast floor

fn painted(property: &str) -> BandIntent {
    BandIntent {
        band: ReviewRegion::Rail,
        box_of: Some(box_of(0.0, 48.0, 56.0, 824.0)),
        panes: vec![],
        paint: Some(PaintIntent {
            property: property.into(),
            role: FloorScope::ControlBoundary,
            backdrop: "--surface".into(),
        }),
        order: None,
    }
}

/// G1 RED, on the real measured value.
#[test]
fn g1_refuses_a_control_boundary_below_the_floor() {
    let h = Harness::new();
    let wanted = intent("/matters", "1:2", 1, vec![painted("--border")]);
    screen(&h, "SCR-0001", "/matters", "1:2", &[ReviewRegion::Rail]);
    staged(&h, &wanted, all_cleared());

    let (outcome, text) = run_kind(&h, ProofKind::StagedWrite);
    assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
    assert!(text.contains("contrast_floor"), "{text}");
    assert!(text.contains("below the 3:1"), "{text}");
    assert!(
        text.contains("MINTED"),
        "the record says cleared and this run measures refused, so R7 must say so too: {text}"
    );
}

/// G1 GREEN, on the real value that passes.
#[test]
fn g1_clears_a_control_boundary_that_makes_its_floor_in_every_theme() {
    let h = Harness::new();
    let wanted = intent("/matters", "1:2", 1, vec![painted("--border-control")]);
    screen(&h, "SCR-0001", "/matters", "1:2", &[ReviewRegion::Rail]);
    staged(&h, &wanted, all_cleared());

    let (outcome, text) = run_kind(&h, ProofKind::StagedWrite);
    assert!(
        !text.contains("<contrast_floor>"),
        "the gate must clear on a boundary that makes its floor in both themes: {text}"
    );
    assert_eq!(outcome.rows_enforced, 1, "{text}");
}

/// A value a binding order RESERVES is refused rather than resolved. This gate
/// measures; it never picks.
#[test]
fn g1_refuses_a_paint_naming_a_property_a_binding_order_reserves() {
    // Edited in place inside `[stage]` rather than appended, because a key
    // appended to the end of a TOML file lands in whatever section is last.
    let config = vds_core::default_config("demo", "DEMO").replace(
        "reserved_paint_properties = []",
        "reserved_paint_properties = [\"--border\"]",
    );
    let h = Harness::with_config(&config);
    let wanted = intent("/matters", "1:2", 1, vec![painted("--border")]);
    screen(&h, "SCR-0001", "/matters", "1:2", &[ReviewRegion::Rail]);
    staged(&h, &wanted, all_cleared());

    let (_, text) = run_kind(&h, ProofKind::StagedWrite);
    assert!(text.contains("RESERVED by a binding order"), "{text}");
    assert!(
        text.contains("measures and never picks"),
        "the refusal must say the gate does not choose a value a court reserved: {text}"
    );
}

/// A literal cannot be measured against the shipped record in any theme.
#[test]
fn g1_refuses_a_paint_that_names_a_literal_rather_than_a_property() {
    let h = Harness::new();
    let mut painted_band = painted("--border-control");
    painted_band.paint.as_mut().unwrap().property = "#748eaf".into();
    let wanted = intent("/matters", "1:2", 1, vec![painted_band]);
    screen(&h, "SCR-0001", "/matters", "1:2", &[ReviewRegion::Rail]);
    staged(&h, &wanted, all_cleared());

    let (_, text) = run_kind(&h, ProofKind::StagedWrite);
    assert!(text.contains("not a custom property name"), "{text}");
    assert!(
        !text.contains("748eaf"),
        "a finding must not repeat the value it refused: a captured record lands under the tree \
         no_stored_values scans, and the gate would then fail forever on a file it wrote. {text}"
    );
}

/// An intent staging no control boundary makes the gate UNRUNNABLE, not passed.
#[test]
fn g1_reports_could_not_run_where_no_control_boundary_is_staged() {
    let h = Harness::new();
    let wanted = intent("/matters", "1:2", 1, vec![rail()]);
    screen(&h, "SCR-0001", "/matters", "1:2", &[ReviewRegion::Rail]);
    staged(&h, &wanted, all_cleared());

    let (_, text) = run_kind(&h, ProofKind::StagedWrite);
    assert!(
        text.contains("[gate contrast_floor] could not run on 1"),
        "{text}"
    );
    assert!(text.contains("never a pass"), "{text}");
}

// ---------------------------------------------------------- G2: band naming

/// G2(a). The strongest available form: a band outside the CLOSED seven-value
/// set is UNREPRESENTABLE, so an intent naming one fails to DESERIALISE. A rule
/// would have left the shape writable and merely disapproved of.
#[test]
fn g2_refuses_a_band_outside_the_closed_vocabulary_at_the_type_and_not_by_a_rule() {
    let h = Harness::new();
    let wanted = intent("/matters", "1:2", 1, vec![rail()]);
    let text = serde_yaml::to_string(&wanted).unwrap();
    assert!(
        serde_yaml::from_str::<StageIntent>(&text).is_ok(),
        "the fixture must parse, or the negative below proves nothing"
    );
    let invented = text.replace("band: rail", "band: hero");
    assert_ne!(invented, text, "the seed did not change the band name");
    assert!(
        serde_yaml::from_str::<StageIntent>(&invented).is_err(),
        "an invented band name must be unrepresentable, not merely invalid: the diff is KEYED on \
         this value, so a name outside the set is a key nothing on the far side answers"
    );

    let project = h.project();
    let path = h.write("design/stage/STG-0001.intent.yaml", &invented);
    assert!(
        vds_core::read_intent(&project, &path).is_err(),
        "and the engine refuses to read one rather than reading a partial intent"
    );
}

/// G2(b) RED. A staged rail on a screen that has none is a write about another
/// screen.
#[test]
fn g2_refuses_a_band_the_screen_record_does_not_declare() {
    let h = Harness::new();
    let wanted = intent("/matters", "1:2", 1, vec![rail()]);
    screen(&h, "SCR-0001", "/matters", "1:2", &[ReviewRegion::Header]);
    staged(&h, &wanted, all_cleared());

    let (outcome, text) = run_kind(&h, ProofKind::StagedWrite);
    assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
    assert!(text.contains("band_naming"), "{text}");
    assert!(text.contains("about another screen"), "{text}");
}

/// G2(b) GREEN.
#[test]
fn g2_clears_where_every_staged_band_is_one_the_screen_declares() {
    let h = Harness::new();
    let wanted = intent("/matters", "1:2", 1, vec![rail()]);
    screen(
        &h,
        "SCR-0001",
        "/matters",
        "1:2",
        &[ReviewRegion::Rail, ReviewRegion::Header],
    );
    staged(&h, &wanted, all_cleared());

    let (_, text) = run_kind(&h, ProofKind::StagedWrite);
    assert!(!text.contains("<band_naming>"), "{text}");
}

/// A screen declaring no band makes the correspondence UNRUNNABLE, which is not
/// the same answer as a pass. On the estate this was written for, the same class
/// of check could only run on 17 rows of 160.
#[test]
fn g2_reports_could_not_run_where_the_screen_declares_no_band() {
    let h = Harness::new();
    let wanted = intent("/matters", "1:2", 1, vec![rail()]);
    h.screen_record("SCR-0001", "/matters", 1, &["body"], Some("1:2"));
    staged(&h, &wanted, all_cleared());

    let (_, text) = run_kind(&h, ProofKind::StagedWrite);
    assert!(
        text.contains("[gate band_naming] could not run on 1"),
        "{text}"
    );
    assert!(text.contains("not a pass"), "{text}");
}

// ---------------------------------------------------- G3: canonical geometry

fn body(columns: u32) -> BandIntent {
    let panes: Vec<BandBox> = (0..columns)
        .map(|i| box_of(f64::from(i) * 440.0, 48.0, 400.0, 800.0))
        .collect();
    BandIntent {
        band: ReviewRegion::BodyRows,
        box_of: Some(box_of(0.0, 48.0, 1344.0, 824.0)),
        panes,
        paint: None,
        order: None,
    }
}

/// G3 RED. The derivation that will run AFTER the write, run over the boxes
/// BEFORE it, reads a different count from the one the intent declares.
#[test]
fn g3_refuses_boxes_that_do_not_derive_the_declared_column_count() {
    let h = Harness::new();
    let wanted = intent("/matters", "1:2", 3, vec![body(2)]);
    screen(&h, "SCR-0001", "/matters", "1:2", &[ReviewRegion::BodyRows]);
    staged(&h, &wanted, all_cleared());

    let (outcome, text) = run_kind(&h, ProofKind::StagedWrite);
    assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
    assert!(text.contains("canonical_geometry"), "{text}");
    assert!(text.contains("declares 3 content column(s)"), "{text}");
    assert!(text.contains("reads 2"), "{text}");
}

/// G3 GREEN.
#[test]
fn g3_clears_boxes_that_derive_exactly_what_the_intent_declares() {
    let h = Harness::new();
    let wanted = intent("/matters", "1:2", 3, vec![body(3)]);
    screen(&h, "SCR-0001", "/matters", "1:2", &[ReviewRegion::BodyRows]);
    staged(&h, &wanted, all_cleared());

    let (_, text) = run_kind(&h, ProofKind::StagedWrite);
    assert!(!text.contains("<canonical_geometry>"), "{text}");
}

/// A box outside the canonical shell would be written off the frame.
#[test]
fn g3_refuses_a_box_outside_the_canonical_shell() {
    let h = Harness::new();
    let mut off = body(1);
    off.box_of = Some(box_of(0.0, 48.0, 4000.0, 824.0));
    let wanted = intent("/matters", "1:2", 1, vec![off]);
    screen(&h, "SCR-0001", "/matters", "1:2", &[ReviewRegion::BodyRows]);
    staged(&h, &wanted, all_cleared());

    let (_, text) = run_kind(&h, ProofKind::StagedWrite);
    assert!(text.contains("outside the canonical shell"), "{text}");
}

// ------------------------------------------------------- G4: route binding

/// G4 RED, WITH THE TWO REAL FALSE MAPPINGS.
///
/// An audit completed on 2026-08-02 confirmed two false mappings out of
/// thirty-three disputed on the subscribing estate:
/// `/stakeholders/members/[userId]` is registered to 669:171003 and the correct
/// node is 669:171342, and `/stakeholders/settings` is registered to 669:172814
/// and the correct node is 669:173031. One of those drove a live code change on
/// a shipped route. They are this gate's red fixtures because they are real, and
/// because they are the reason the gate exists.
#[test]
fn g4_refuses_the_two_real_false_mappings_and_names_both_claims() {
    for (route, staged_node, correct_node) in [
        ("/stakeholders/members/[userId]", "669:171003", "669:171342"),
        ("/stakeholders/settings", "669:172814", "669:173031"),
    ] {
        let h = Harness::new();
        let wanted = intent(route, staged_node, 1, vec![rail()]);
        screen(&h, "SCR-0001", route, staged_node, &[ReviewRegion::Rail]);
        h.route_bindings(
            "internal-docs/design/frame-registry.json",
            &[(route, correct_node)],
        );
        staged(&h, &wanted, all_cleared());

        let (outcome, text) = run_kind(&h, ProofKind::StagedWrite);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{route}: {text}");
        assert!(text.contains("route_binding"), "{route}: {text}");
        assert!(
            text.contains(staged_node) && text.contains(correct_node),
            "the refusal must name BOTH claims, because VDS does not decide which is true: \
             {route}: {text}"
        );
        assert!(
            text.contains("DOES NOT DECIDE WHICH IS TRUE"),
            "{route}: {text}"
        );
    }
}

/// G4 GREEN.
#[test]
fn g4_clears_where_the_estates_own_record_agrees_with_the_target() {
    let h = Harness::new();
    let route = "/stakeholders/settings";
    let wanted = intent(route, "669:173031", 1, vec![rail()]);
    screen(&h, "SCR-0001", route, "669:173031", &[ReviewRegion::Rail]);
    h.route_bindings(
        "internal-docs/design/frame-registry.json",
        &[(route, "669:173031")],
    );
    staged(&h, &wanted, all_cleared());

    let (_, text) = run_kind(&h, ProofKind::StagedWrite);
    assert!(!text.contains("<route_binding>"), "{text}");
}

/// WHERE NO BINDING LEDGER EXISTS, G4 REPORTS COULD_NOT_RUN AND THE COVERAGE
/// LINE SAYS SO OUT LOUD. A single unopposed self-claim must never read as
/// agreement.
#[test]
fn g4_says_out_loud_that_an_unopposed_self_claim_is_not_agreement() {
    let h = Harness::new();
    let wanted = intent("/matters", "1:2", 1, vec![rail()]);
    screen(&h, "SCR-0001", "/matters", "1:2", &[ReviewRegion::Rail]);
    staged(&h, &wanted, all_cleared());

    let (_, text) = run_kind(&h, ProofKind::StagedWrite);
    assert!(
        text.contains("[gate route_binding] could not run on 1 of 1"),
        "the coverage line must name the gate that could not run, per gate: {text}"
    );
    assert!(
        text.contains("unopposed self-claim must never read as agreement"),
        "{text}"
    );
}

// ------------------------------------------------------- R5: bypass detection

/// R5 GREEN. The frame's current digest is the one it was signed off at, so
/// nothing has written it since.
#[test]
fn r5_clears_a_frame_whose_current_content_is_the_digest_it_was_signed_off_at() {
    let h = Harness::new();
    frames_captured_at(
        &h,
        &[Harness::frame("1:2", "Screen /matters", &["body"], 1)],
        "2026-08-03T08:00:00Z",
    );
    h.screen_record("SCR-0001", "/matters", 1, &["body"], Some("1:2"));
    h.signoff("KEY", "1:2");

    let (outcome, text) = run_kind(&h, ProofKind::StagedWrite);
    assert_eq!(outcome.exit_code, EXIT_PASSED, "{text}");
    assert_eq!(outcome.rows_enforced, 1, "{text}");
    assert!(!text.contains("did not come through VDS"), "{text}");
}

/// R5 RED. The frame moved after sign-off and no applied stage accounts for it.
#[test]
fn r5_names_a_frame_written_by_something_that_did_not_come_through_vds() {
    let h = Harness::new();
    frames_captured_at(
        &h,
        &[Harness::frame("1:2", "Screen /matters", &["body"], 1)],
        "2026-08-03T08:00:00Z",
    );
    h.screen_record("SCR-0001", "/matters", 1, &["body"], Some("1:2"));
    h.signoff("KEY", "1:2");
    // Somebody draws a second column into the frame, outside VDS.
    frames_captured_at(
        &h,
        &[Harness::frame("1:2", "Screen /matters", &["body"], 2)],
        "2026-08-03T08:00:00Z",
    );

    let (outcome, text) = run_kind(&h, ProofKind::StagedWrite);
    assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
    assert!(text.contains("did not come through VDS"), "{text}");
    assert!(
        text.contains("NOT CURED BY RE-RUNNING"),
        "the finding must say the cure is staging the state the frame is now in, or reverting: \
         {text}"
    );
}

/// An APPLIED AND VERIFIED stage adds its post-write digest to the set the rule
/// accepts, so a lawful VDS write is not reported as a bypass.
#[test]
fn r5_accepts_the_digest_an_applied_and_verified_stage_recorded() {
    let h = Harness::new();
    frames_captured_at(
        &h,
        &[Harness::frame("1:2", "Screen /matters", &["body"], 2)],
        "2026-08-03T08:00:00Z",
    );
    h.screen_record("SCR-0001", "/matters", 1, &["body"], Some("1:2"));
    h.signoff_at("KEY", "1:2", Digest::of_text("the frame before the write"));

    let current = vds_figma::frames::read(&h.project())
        .unwrap()
        .unwrap()
        .row("1:2")
        .unwrap()
        .content_digest
        .clone()
        .unwrap();

    let wanted = intent("/matters", "1:2", 1, vec![rail()]);
    let id = staged(&h, &wanted, all_cleared());
    h.amend_stage(&id, |record| {
        record.apply = Some(vds_core::ApplyOutcome {
            applied_at: Timestamp::fixed(2026, 8, 3, 10, 30, 0),
            applied_by: "a harness".into(),
            lock_holder: "vds-stage".into(),
            chunks: 1,
            operations: 1,
            plan_digest: Digest::of_text("plan"),
            verification: Some(Verification {
                verified_at: Timestamp::fixed(2026, 8, 3, 10, 35, 0),
                frame_digest_after: current.clone(),
                residual_operations: 0,
            }),
        });
    });

    let (outcome, text) = run_kind(&h, ProofKind::StagedWrite);
    assert!(
        !text.contains("did not come through VDS"),
        "a frame whose current content is what a verified apply left is not a bypass: {text}"
    );
    assert_eq!(outcome.exit_code, EXIT_PASSED, "{text}");
}

/// THE TRAP THE DESIGN NAMED, CLOSED.
///
/// A ledger that states no capture date makes this rule silent against a stale
/// reading, which is exactly the failure the subscribing estate hit on
/// 2026-08-02 on 23 of 188 routes. It REFUSES rather than reporting no bypass.
#[test]
fn r5_refuses_a_frames_ledger_that_states_no_capture_date() {
    let h = Harness::new();
    // The default generator path, which records no capture date.
    h.frames(&[Harness::frame("1:2", "Screen /matters", &["body"], 1)]);
    h.screen_record("SCR-0001", "/matters", 1, &["body"], Some("1:2"));
    h.signoff("KEY", "1:2");

    let (outcome, text) = run_kind(&h, ProofKind::StagedWrite);
    assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
    assert!(text.contains("states no capture date"), "{text}");
    assert!(
        text.contains("check that cannot fail"),
        "the refusal must name the failure class it is closing: {text}"
    );
    assert_eq!(
        outcome.rows_enforced, 0,
        "a run that could not tell fresh from stale must enforce nothing: {text}"
    );
}

/// It refuses on the CAPTURE date and never on the ledger's generated_at.
///
/// The ledger here was regenerated NOW, so `generated_at` is fresh; only the
/// capture is old, which is the exact shape that made the estate's reading stale
/// while every instrument looked current.
#[test]
fn r5_refuses_an_over_age_capture_even_though_the_ledger_was_generated_now() {
    let h = Harness::new();
    frames_captured_at(
        &h,
        &[Harness::frame("1:2", "Screen /matters", &["body"], 1)],
        "2026-07-01T08:00:00Z",
    );
    h.screen_record("SCR-0001", "/matters", 1, &["body"], Some("1:2"));
    // The freshest independent record this run reads is dated a month later.
    h.signoff("KEY", "1:2");

    let ledger = vds_figma::frames::read(&h.project()).unwrap().unwrap();
    assert!(
        ledger.generated_at.as_str() > "2026-08-01T00:00:00Z",
        "the ledger must have been generated NOW, or this test does not distinguish the two \
         dates at all"
    );

    let (outcome, text) = run_kind(&h, ProofKind::StagedWrite);
    assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
    assert!(text.contains("past the declared maximum"), "{text}");
    assert!(text.contains("2026-07-01"), "{text}");
}

/// A frame with no sign-off and no applied stage has NO BASELINE. That is not a
/// bypass and it is not a pass, and reporting it as a bypass would redden every
/// estate on the day it adopts this kind.
#[test]
fn r5_reports_a_frame_with_no_baseline_rather_than_calling_it_a_bypass() {
    let h = Harness::new();
    frames_captured_at(
        &h,
        &[Harness::frame("1:2", "Screen /matters", &["body"], 1)],
        "2026-08-03T08:00:00Z",
    );
    h.screen_record("SCR-0001", "/matters", 1, &["body"], Some("1:2"));

    let (outcome, text) = run_kind(&h, ProofKind::StagedWrite);
    assert!(text.contains("NO BASELINE"), "{text}");
    assert!(
        !text.contains("did not come through VDS"),
        "a bypass claim needs a baseline, and a permanently red gate is one people switch off: \
         {text}"
    );
    assert_eq!(
        outcome.status,
        ProofStatus::Vacuous,
        "nothing could be measured, so this is a vacuity and never a pass: {text}"
    );
}

// -------------------------------------------------- the record's own defects

/// An apply is an ATTEMPT. Only a re-capture that finds the delta EMPTY declares
/// success.
#[test]
fn an_apply_with_no_verification_is_not_a_finished_apply() {
    let h = Harness::new();
    let wanted = intent("/matters", "1:2", 1, vec![rail()]);
    screen(&h, "SCR-0001", "/matters", "1:2", &[ReviewRegion::Rail]);
    let id = staged(&h, &wanted, all_cleared());
    h.amend_stage(&id, |record| {
        record.apply = Some(vds_core::ApplyOutcome {
            applied_at: Timestamp::fixed(2026, 8, 3, 10, 30, 0),
            applied_by: "a harness".into(),
            lock_holder: "vds-stage".into(),
            chunks: 3,
            operations: 9,
            plan_digest: Digest::of_text("plan"),
            verification: None,
        });
    });

    let (outcome, text) = run_kind(&h, ProofKind::StagedWrite);
    assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
    assert!(text.contains("ATTEMPT"), "{text}");
    assert!(text.contains("vds stage verify"), "{text}");
}

/// A verification carrying a residual is a PARTIAL apply, which is reachable by
/// construction because the bridge caps one call and offers no transaction.
#[test]
fn a_verification_with_a_residual_is_a_partial_apply_and_says_so() {
    let h = Harness::new();
    let wanted = intent("/matters", "1:2", 1, vec![rail()]);
    screen(&h, "SCR-0001", "/matters", "1:2", &[ReviewRegion::Rail]);
    let id = staged(&h, &wanted, all_cleared());
    h.amend_stage(&id, |record| {
        record.apply = Some(vds_core::ApplyOutcome {
            applied_at: Timestamp::fixed(2026, 8, 3, 10, 30, 0),
            applied_by: "a harness".into(),
            lock_holder: "vds-stage".into(),
            chunks: 3,
            operations: 9,
            plan_digest: Digest::of_text("plan"),
            verification: Some(Verification {
                verified_at: Timestamp::fixed(2026, 8, 3, 10, 35, 0),
                frame_digest_after: Digest::of_text("after"),
                residual_operations: 4,
            }),
        });
    });

    let (_, text) = run_kind(&h, ProofKind::StagedWrite);
    assert!(text.contains("still emits 4 operation(s)"), "{text}");
    assert!(text.contains("do not record this as done"), "{text}");
}

/// A gate absent from the record and a gate that cleared are the same green to
/// anybody counting refusals.
#[test]
fn a_gate_never_asked_is_a_finding_and_not_a_silence() {
    let h = Harness::new();
    let wanted = intent("/matters", "1:2", 1, vec![rail()]);
    screen(&h, "SCR-0001", "/matters", "1:2", &[ReviewRegion::Rail]);
    staged(&h, &wanted, vec![cleared(StageGate::BandNaming)]);

    let (outcome, text) = run_kind(&h, ProofKind::StagedWrite);
    assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
    assert!(text.contains("carries no reading for"), "{text}");
    assert!(
        text.contains("same green to anybody counting refusals"),
        "{text}"
    );
}

/// The MINTING defect, closed: a record whose verdicts were typed is refuted by
/// the run that recomputes them.
#[test]
fn a_recorded_verdict_this_run_does_not_reproduce_is_a_finding() {
    let h = Harness::new();
    let wanted = intent("/matters", "1:2", 3, vec![body(2)]);
    screen(&h, "SCR-0001", "/matters", "1:2", &[ReviewRegion::BodyRows]);
    staged(&h, &wanted, all_cleared());

    let (_, text) = run_kind(&h, ProofKind::StagedWrite);
    assert!(text.contains("can be MINTED"), "{text}");
    assert!(
        text.contains("record says cleared and this run measures refused"),
        "{text}"
    );
}

/// The verdicts were read over the intent the record pins, and that file moved.
#[test]
fn an_intent_that_moved_since_it_was_staged_refuses_the_row() {
    let h = Harness::new();
    let wanted = intent("/matters", "1:2", 1, vec![rail()]);
    screen(&h, "SCR-0001", "/matters", "1:2", &[ReviewRegion::Rail]);
    staged(&h, &wanted, all_cleared());
    // The author edits the intent after staging it.
    let moved = intent(
        "/matters",
        "1:2",
        1,
        vec![band(ReviewRegion::Rail, box_of(0.0, 48.0, 200.0, 824.0))],
    );
    h.stage_intent("STG-0001", &moved);

    let (outcome, text) = run_kind(&h, ProofKind::StagedWrite);
    assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
    assert!(text.contains("taken over a different file"), "{text}");
}

/// THE ONE PRECONDITION THAT IS A RULE OF LAW. The proof will not read an intent
/// it would be unlawful to have written.
#[test]
fn the_proof_refuses_an_intent_root_under_the_record() {
    let h = Harness::new();
    // The config LOADER refuses this too. What this test holds is the PROOF's
    // own guard, which has to stand whether or not the loader ever changes:
    // one gate on one artefact is one place it can be removed.
    let project = h.project();
    let mut config = project.config.clone();
    config.stage.intent_root = ".vds/stages".into();
    let project = vds_core::Project {
        root: project.root.clone(),
        config,
        config_path: project.config_path.clone(),
    };
    let leaked: &'static vds_core::Project = Box::leak(Box::new(project));
    let ctx = crate::ProofContext {
        project: leaked,
        invoked_by: vds_core::InvokedBy::CiWorkflow,
        allow_vacuous: false,
        capture: true,
    };
    let error = crate::run(ProofKind::StagedWrite, &ctx, &mut Vec::new())
        .expect_err("an unlawful intent root is a precondition failure");
    assert!(error.to_string().contains("no_stored_values"), "{error}");
    assert!(
        error.to_string().contains("did not run"),
        "a precondition failure must say the proof proved nothing: {error}"
    );
}

/// The coverage tally is checked BEFORE anything prints, and it names both
/// populations this kind reports on.
#[test]
fn the_coverage_line_names_the_two_populations_this_kind_reports_on() {
    let h = Harness::new();
    frames_captured_at(
        &h,
        &[Harness::frame("1:2", "Screen /matters", &["body"], 1)],
        "2026-08-03T08:00:00Z",
    );
    let wanted = intent("/matters", "1:2", 1, vec![rail()]);
    screen(&h, "SCR-0001", "/matters", "1:2", &[ReviewRegion::Rail]);
    h.signoff("KEY", "1:2");
    staged(&h, &wanted, all_cleared());

    let (outcome, text) = run_kind(&h, ProofKind::StagedWrite);
    assert!(
        text.contains("staged write(s) and named frame(s) were SCORED"),
        "{text}"
    );
    assert_eq!(
        outcome.rows_considered, 2,
        "one staged write and one named frame: {text}"
    );
}
