//! TEMPORARY adversarial probes. Delete before commit.

#[cfg(test)]
mod tests {
    use crate::contrast::SHIPPED_STYLESHEET;
    use crate::testing::{Harness, run_kind};
    use vds_core::{
        ComponentId, ContrastFloor, EXIT_PASSED, EXIT_VIOLATION, FloorScope, ProofKind, Status,
    };

    fn sheet(h: &Harness, css: &str) {
        h.write(SHIPPED_STYLESHEET, css);
    }

    fn with_floor(
        h: &Harness,
        name: &str,
        status: Status,
        boundary: &str,
        against: &str,
        min_ratio: f64,
    ) -> ComponentId {
        let id = h.register(name, status);
        h.amend(&id, |record| {
            record.a11y.contrast_floors = vec![ContrastFloor {
                boundary: boundary.into(),
                against: against.into(),
                min_ratio,
                basis: "WCAG 2.2 SC 1.4.11".into(),
                scope: Some(FloorScope::ControlBoundary),
            }];
        });
        id
    }

    fn subject(h: &Harness, status: Status) -> ComponentId {
        with_floor(h, "Button", status, "control-border", "surface", 3.0)
    }

    /// PROBE A: no `:root` at all. A non-root-like palette scope should be
    /// reported by R6. Is it?
    #[test]
    fn probe_a_no_root_hides_a_non_root_like_palette() {
        let h = Harness::new();
        sheet(
            &h,
            "\
html[data-theme='light'] { --surface: #ffffff; --control-border: #767676; }
html[data-theme='dark'] { --surface: #1a1a1a; --control-border: #9a9a9a; }
.dark .panel { --control-border: #1c1c1c; --surface: #1a1a1a; }
",
        );
        subject(&h, Status::Registered);
        let (outcome, text) = run_kind(&h, ProofKind::Contrast);
        println!(
            "PROBE A status={:?} exit={}",
            outcome.status, outcome.exit_code
        );
        println!(
            "PROBE A rows_considered={} rows_enforced={}",
            outcome.rows_considered, outcome.rows_enforced
        );
        println!("{text}");
    }

    /// PROBE B: a theme selector that carries an easing keyword. Does the
    /// captured contrast record fail no_stored_values?
    #[test]
    fn probe_b_easing_named_theme_selector_poisons_the_record() {
        let h = Harness::new();
        sheet(
            &h,
            "\
:root { --surface: #ffffff; --control-border: #767676; }
.ease-in-out { --surface: #ffffff; --control-border: #6a6a6a; }
",
        );
        subject(&h, Status::Registered);
        let (outcome, text) = run_kind(&h, ProofKind::Contrast);
        println!(
            "PROBE B contrast status={:?} exit={}",
            outcome.status, outcome.exit_code
        );
        println!("{text}");
        let (guard, gtext) = run_kind(&h, ProofKind::NoStoredValues);
        println!(
            "PROBE B guard status={:?} exit={}",
            guard.status, guard.exit_code
        );
        if guard.exit_code != EXIT_PASSED {
            println!("{gtext}");
        }
    }

    /// PROBE B2: a generic font family in a theme selector.
    #[test]
    fn probe_b2_monospace_theme_selector() {
        let h = Harness::new();
        sheet(
            &h,
            "\
:root { --surface: #ffffff; --control-border: #767676; }
[data-font='monospace'] { --surface: #ffffff; --control-border: #6a6a6a; }
",
        );
        subject(&h, Status::Registered);
        let (outcome, _t) = run_kind(&h, ProofKind::Contrast);
        println!("PROBE B2 contrast status={:?}", outcome.status);
        let (guard, gtext) = run_kind(&h, ProofKind::NoStoredValues);
        println!(
            "PROBE B2 guard status={:?} exit={}",
            guard.status, guard.exit_code
        );
        if guard.exit_code != EXIT_PASSED {
            println!("{gtext}");
        }
    }

    /// PROBE C: an rgb() colour inside a selector. Does the record keep the
    /// channel numbers?
    #[test]
    fn probe_c_rgb_channels_in_a_selector() {
        let h = Harness::new();
        sheet(
            &h,
            "\
:root { --surface: #ffffff; --control-border: #767676; }
[data-tint='rgb(12,34,56)'] { --surface: #ffffff; --control-border: #f4f4f4; }
",
        );
        subject(&h, Status::Registered);
        let (outcome, text) = run_kind(&h, ProofKind::Contrast);
        println!(
            "PROBE C status={:?} exit={}",
            outcome.status, outcome.exit_code
        );
        println!("{text}");
        let record = h.last_proof(ProofKind::Contrast);
        let rendered = format!("{record:?}");
        println!(
            "PROBE C contains 12,34,56 = {}",
            rendered.contains("12,34,56")
        );
        let (guard, _g) = run_kind(&h, ProofKind::NoStoredValues);
        println!("PROBE C guard status={:?}", guard.status);
    }

    /// PROBE D: a malformed value. What lands in the record?
    #[test]
    fn probe_d_malformed_value_detail() {
        let h = Harness::new();
        sheet(
            &h,
            "\
:root { --surface: #ffffff; --control-border: var(--a; }
",
        );
        subject(&h, Status::Registered);
        let (outcome, text) = run_kind(&h, ProofKind::Contrast);
        println!(
            "PROBE D status={:?} exit={}",
            outcome.status, outcome.exit_code
        );
        println!("{text}");
    }

    /// PROBE E: a root-like scope declaring only NON-base properties, where a
    /// floor property lives there.
    #[test]
    fn probe_e_theme_that_only_owns_its_own_properties() {
        let h = Harness::new();
        sheet(
            &h,
            "\
:root { --surface: #ffffff; --control-border: #767676; --other: #000000; }
.dark { --other: #ffffff; }
.dark-real { --surface: #1a1a1a; --control-border: #1c1c1c; }
",
        );
        subject(&h, Status::Registered);
        let (outcome, text) = run_kind(&h, ProofKind::Contrast);
        println!(
            "PROBE E status={:?} exit={} rows={}",
            outcome.status, outcome.exit_code, outcome.rows_enforced
        );
        println!("{text}");
    }

    /// PROBE F: exactly-on-the-floor and just-under, and what gets printed.
    #[test]
    fn probe_f_boundary_printing_vs_verdict() {
        // find a pair whose ratio is just under 3.0 but prints 3.00? impossible
        // with truncation. Try just over 4.5 for the AA text floor.
        for (fg, bg, floor) in [
            ("#767676", "#ffffff", 4.54_f64),
            ("#767676", "#ffffff", 4.55_f64),
        ] {
            let h = Harness::new();
            sheet(
                &h,
                &format!(":root {{ --surface: {bg}; --control-border: {fg}; }}\n"),
            );
            with_floor(
                &h,
                "Button",
                Status::Registered,
                "control-border",
                "surface",
                floor,
            );
            let (outcome, text) = run_kind(&h, ProofKind::Contrast);
            println!("PROBE F floor={floor} status={:?}", outcome.status);
            for line in text.lines().filter(|l| l.contains(":1")) {
                println!("   {line}");
            }
        }
    }

    /// PROBE G: does the margin note or a finding survive a second theme whose
    /// name is a length?
    #[test]
    fn probe_g_selector_carrying_a_length() {
        let h = Harness::new();
        sheet(
            &h,
            "\
:root { --surface: #ffffff; --control-border: #767676; }
.p-12px { --surface: #ffffff; --control-border: #6a6a6a; }
",
        );
        subject(&h, Status::Registered);
        let (outcome, text) = run_kind(&h, ProofKind::Contrast);
        println!("PROBE G status={:?}", outcome.status);
        println!("{text}");
        let (guard, gtext) = run_kind(&h, ProofKind::NoStoredValues);
        println!("PROBE G guard status={:?}", guard.status);
        if guard.exit_code != EXIT_PASSED {
            println!("{gtext}");
        }
    }

    /// PROBE H: two floors on one record, one bad and one good: do the rows add
    /// up?
    #[test]
    fn probe_h_rows_add_up_with_mixed_floors() {
        let h = Harness::new();
        sheet(
            &h,
            "\
:root { --surface: #ffffff; --control-border: #767676; }
.dark { --surface: #1a1a1a; --control-border: #9a9a9a; }
",
        );
        let id = h.register("Button", Status::Registered);
        h.amend(&id, |record| {
            record.a11y.contrast_floors = vec![
                ContrastFloor {
                    boundary: "control-border".into(),
                    against: "surface".into(),
                    min_ratio: 3.0,
                    basis: "WCAG".into(),
                    scope: Some(FloorScope::ControlBoundary),
                },
                ContrastFloor {
                    boundary: "#ebebeb".into(),
                    against: "surface".into(),
                    min_ratio: 3.0,
                    basis: "WCAG".into(),
                    scope: Some(FloorScope::ControlBoundary),
                },
                ContrastFloor {
                    boundary: "control-border".into(),
                    against: "surface".into(),
                    min_ratio: 0.5,
                    basis: "WCAG".into(),
                    scope: Some(FloorScope::ControlBoundary),
                },
            ];
        });
        let (outcome, text) = run_kind(&h, ProofKind::Contrast);
        let record = h.last_proof(ProofKind::Contrast);
        let skipped: u64 = record.rows_skipped_reasons.values().sum();
        println!(
            "PROBE H considered={} enforced={} skipped={} sum_ok={}",
            outcome.rows_considered,
            outcome.rows_enforced,
            skipped,
            outcome.rows_considered == outcome.rows_enforced + skipped
        );
        println!("{text}");
        let _ = EXIT_VIOLATION;
    }
}

#[cfg(test)]
mod tests2 {
    use crate::testing::{Harness, run_kind};
    use vds_core::{ContrastFloor, FloorScope, ProofKind, Status, default_config};

    /// PROBE I: a project that moved `[surface] stylesheet`. Do the R6 and
    /// "too many findings" locations name the file that was actually measured?
    #[test]
    fn probe_i_finding_location_after_the_stylesheet_moves() {
        let config = default_config("demo", "DEMO")
            .replace("stylesheet = \"app/globals.css\"", "stylesheet = \"src/theme.css\"");
        let h = Harness::with_config(&config);
        h.reload();
        h.write(
            "src/theme.css",
            "\
:root { --surface: #ffffff; --control-border: #767676; }
:root:not(.compact) { --control-border: #eeeeee; }
",
        );
        let id = h.register("Button", Status::Registered);
        h.amend(&id, |record| {
            record.a11y.contrast_floors = vec![ContrastFloor {
                boundary: "control-border".into(),
                against: "surface".into(),
                min_ratio: 3.0,
                basis: "WCAG".into(),
                scope: Some(FloorScope::ControlBoundary),
            }];
        });
        let (outcome, text) = run_kind(&h, ProofKind::Contrast);
        println!("PROBE I status={:?} exit={}", outcome.status, outcome.exit_code);
        for line in text.lines().filter(|l| l.contains("[1]") || l.contains("note: [record]") || l.contains("actual:")) {
            println!("   {line}");
        }
        println!("PROBE I app/globals.css exists = {}", h.root().join("app/globals.css").exists());
    }

    /// PROBE J: no `:root`, an unclassified palette scope, and the founding
    /// defect's ratio hiding in it.
    #[test]
    fn probe_j_no_root_with_a_conditional_palette() {
        let h = Harness::new();
        h.write(
            "app/globals.css",
            "\
html[data-theme='light'] { --surface: #ffffff; --control-border: #767676; }
:root:not(.compact) { --control-border: #eeeeee; }
.dark .panel { --control-border: #1c1c1c; --surface: #1a1a1a; }
",
        );
        let id = h.register("Button", Status::Registered);
        h.amend(&id, |record| {
            record.a11y.contrast_floors = vec![ContrastFloor {
                boundary: "control-border".into(),
                against: "surface".into(),
                min_ratio: 3.0,
                basis: "WCAG".into(),
                scope: Some(FloorScope::ControlBoundary),
            }];
        });
        let (outcome, text) = run_kind(&h, ProofKind::Contrast);
        println!("PROBE J status={:?} exit={}", outcome.status, outcome.exit_code);
        for line in text.lines().filter(|l| l.contains("themes-measured") || l.contains("[1]")) {
            println!("   {line}");
        }
    }
}
