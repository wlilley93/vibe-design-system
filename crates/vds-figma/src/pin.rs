//! `vds pin generate`: the half of `token_pin` that was missing.
//!
//! `token_pin` has been implemented and VACUOUS everywhere, because nothing
//! produced a pin. This produces one, and the split is the design rather than a
//! workaround: one of the two records VDS S-2(3) names is a Figma file behind a
//! network call, VDS S-7(2)(1) forbids a network call inside a proof, so the
//! comparison happens HERE, out of band, and the gate checks what comes out.
//!
//! # How this holds a realisation without storing one
//!
//! It has to compare two realisations. A hex colour on one side, a Figma RGBA
//! quadruple on the other, and no comparison is possible without both.
//!
//! VDS S-3(9) is the answer and it was written for exactly this. It names
//! `.vds/cache/` and `.vds/private/` as the two ignored directories, and
//! `no_stored_values` skips them BY NAME and counts what it skipped. So the raw
//! Figma response, full of channels and pixel values, lives lawfully in
//! `.vds/cache/figma/`, gitignored, not a record and not evidence. This
//! generator reads it, reads the shipped stylesheet, compares, and writes a pin
//! whose rows carry a NAME and an AGREEMENT and nothing a brute force can
//! invert.
//!
//! Nothing about S-2 needed amending. The cache is the escape hatch the
//! specification already provided, and the reason it exists.
//!
//! # Why VARIABLES and not node fills
//!
//! Figma variables carry a name and a value per MODE, which is one-to-one with a
//! CSS custom property carrying a value per THEME. That is the same shape
//! `contrast` already measures, so a pin row and a contrast floor talk about the
//! same thing. Reading fills off nodes would compare a rendered instance against
//! a declaration and get a different answer for a legitimate reason.
//!
//! # What this generator will not do
//!
//! It will not GUESS a mapping and present the guess as a measurement. A Figma
//! variable `control/border` and a CSS property `--control-border` correspond by
//! a convention nobody has written down, so the derivation is applied, LABELLED
//! as derived, and any variable it cannot place is reported as a row it declined
//! to enforce with the reason on it. A pin full of guessed correspondences would
//! agree with itself and check nothing.

use std::collections::BTreeMap;

use serde::Deserialize;
use vds_core::{
    Digest, Pin, PinDirection, PinId, PinRow, Project, RecordOfTruth, Result, Timestamp, VdsError,
};
use vds_css::colour;
use vds_css::sheet::Sheet;
use vds_store::Store;

/// The command that regenerates a pin byte for byte, less its arguments.
///
/// VDS S-2(5)(4) makes `generated_by` the regeneration limb, so this has to be a
/// command that EXISTS and that reproduces the pin from the same inputs. The
/// cached response is one of those inputs, which is why the cache path is part
/// of what gets recorded.
pub const GENERATOR: &str = "vds pin generate";

/// Where a raw Figma response is cached.
///
/// Under `.vds/cache/`, which VDS S-3(9) makes one of the two ignored
/// directories: gitignored, skipped by name by `no_stored_values`, and counted
/// so the carve-out is a number in the record rather than an omission. This is
/// the only place in VDS where a design value may sit on disk.
pub fn cache_dir(project: &Project) -> std::path::PathBuf {
    project.vds_dir().join("cache").join("figma")
}

pub fn cached_variables_path(project: &Project, file_key: &str) -> std::path::PathBuf {
    cache_dir(project).join(format!("variables-{file_key}.json"))
}

// ---------------------------------------------------------------------------
// The Figma side
// ---------------------------------------------------------------------------

/// The subset of `GET /v1/files/:key/variables/local` this generator reads.
///
/// Deliberately partial. Deserialising the whole payload would make an
/// unrelated API addition a parse failure, and a generator that stops working
/// when Figma ships a field is a generator nobody runs.
#[derive(Debug, Deserialize)]
struct VariablesResponse {
    meta: VariablesMeta,
}

#[derive(Debug, Deserialize)]
struct VariablesMeta {
    #[serde(default)]
    variables: BTreeMap<String, Variable>,
    #[serde(rename = "variableCollections", default)]
    collections: BTreeMap<String, Collection>,
}

#[derive(Debug, Deserialize)]
struct Variable {
    name: String,
    #[serde(rename = "resolvedType")]
    resolved_type: String,
    #[serde(rename = "variableCollectionId")]
    collection_id: String,
    #[serde(rename = "valuesByMode", default)]
    values_by_mode: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct Collection {
    #[serde(default)]
    modes: Vec<Mode>,
}

#[derive(Debug, Deserialize)]
struct Mode {
    #[serde(rename = "modeId")]
    mode_id: String,
    name: String,
}

/// One Figma variable, reduced to what a comparison needs.
#[derive(Debug, Clone)]
struct DecidedValue {
    /// The variable's Figma name, e.g. `control/border`.
    figma_name: String,
    /// The mode's name, e.g. `Dark`.
    mode: String,
    /// The value as CSS text, so both sides are compared in one form.
    ///
    /// Held in memory and never written to a record. It reaches a `PinRow` only
    /// as the boolean that comes out of comparing it.
    as_css: String,
}

/// Everything the decided-target file says, per variable name and mode.
fn decided_values(body: &str) -> Result<Vec<DecidedValue>> {
    let response: VariablesResponse = serde_json::from_str(body).map_err(|e| {
        VdsError::precondition(format!(
            "the Figma variables response does not parse: {e}. A partial parse would produce a \
             pin claiming fewer rows than the file decides, and a pin that covers less than it \
             says is the overclaim the coverage number exists to prevent."
        ))
    })?;

    let mut modes: BTreeMap<&str, &str> = BTreeMap::new();
    for collection in response.meta.collections.values() {
        for mode in &collection.modes {
            modes.insert(mode.mode_id.as_str(), mode.name.as_str());
        }
    }

    let mut out = Vec::new();
    for variable in response.meta.variables.values() {
        // COLOR only, for now, and the omission is named rather than silent:
        // FLOAT variables carry spacing and radii, which is improvement work
        // with a different comparison (a number against a length unit), and
        // STRING and BOOLEAN carry no realisation a stylesheet declares.
        if variable.resolved_type != "COLOR" {
            continue;
        }
        let _ = &variable.collection_id;
        for (mode_id, value) in &variable.values_by_mode {
            let Some(mode) = modes.get(mode_id.as_str()) else {
                continue;
            };
            let Some(as_css) = colour_to_css(value) else {
                continue;
            };
            out.push(DecidedValue {
                figma_name: variable.name.clone(),
                mode: (*mode).to_owned(),
                as_css,
            });
        }
    }
    out.sort_by(|a, b| (&a.figma_name, &a.mode).cmp(&(&b.figma_name, &b.mode)));
    Ok(out)
}

/// A Figma colour `{r, g, b, a}` in 0..1 floats, as CSS text.
///
/// Both sides end up as CSS so ONE parser decides what a colour is. Converting
/// the CSS side into floats instead would put a second colour model in the
/// comparison, and the two would disagree at the edges for reasons that have
/// nothing to do with the design.
fn colour_to_css(value: &serde_json::Value) -> Option<String> {
    let object = value.as_object()?;
    let channel = |key: &str| -> Option<f64> { object.get(key)?.as_f64() };
    let r = channel("r")?;
    let g = channel("g")?;
    let b = channel("b")?;
    let a = channel("a").unwrap_or(1.0);
    let eight = |v: f64| -> u8 { (v.clamp(0.0, 1.0) * 255.0).round() as u8 };
    if (a - 1.0).abs() < f64::EPSILON {
        Some(format!("#{:02x}{:02x}{:02x}", eight(r), eight(g), eight(b)))
    } else {
        Some(format!(
            "rgba({}, {}, {}, {})",
            eight(r),
            eight(g),
            eight(b),
            a
        ))
    }
}

// ---------------------------------------------------------------------------
// The correspondence
// ---------------------------------------------------------------------------

/// How a Figma variable name becomes a CSS custom property name.
///
/// `control/border` -> `--control-border`. Applied, and LABELLED as derived
/// wherever it reaches a report, because it is a convention nobody has written
/// down: a project is free to name its variables `Colour/Control/Border` or its
/// properties `--sf-control-border`, and this derivation would be wrong about
/// both. What it must never do is guess quietly. A variable this cannot place is
/// a row the pin DECLINES to enforce, with the reason on it, which is the shape
/// `PinRow::not_enforced` exists for.
pub fn derived_property_name(figma_name: &str, prefix: &str) -> String {
    let slug: String = figma_name
        .chars()
        .map(|c| match c {
            '/' | ' ' | '_' => '-',
            other => other.to_ascii_lowercase(),
        })
        .collect();
    let slug = slug.trim_matches('-').to_owned();
    format!("--{prefix}{slug}")
}

/// How a Figma mode name becomes a theme selector in the shipped stylesheet.
///
/// Matched by NAME against the scopes the sheet declares, case-insensitively and
/// ignoring punctuation, so `Dark` finds `[data-theme='dark']` and `.dark` alike.
/// A mode that matches nothing is reported rather than assumed to be the base.
fn theme_for_mode(mode: &str, themes: &[String], base: Option<&str>) -> Option<String> {
    let wanted = mode.to_ascii_lowercase();
    if let Some(found) = themes.iter().find(|selector| {
        selector
            .to_ascii_lowercase()
            .contains(&format!("'{wanted}'"))
            || selector
                .to_ascii_lowercase()
                .contains(&format!("\"{wanted}\""))
            || selector.to_ascii_lowercase() == format!(".{wanted}")
            || selector.to_ascii_lowercase() == format!("[data-theme={wanted}]")
    }) {
        return Some(found.clone());
    }
    // A single-mode file, or a mode named for the default: the base scope is the
    // only lawful answer and only where there is exactly one mode in play.
    if matches!(wanted.as_str(), "default" | "light" | "mode 1" | "value") {
        return base.map(str::to_owned);
    }
    None
}

// ---------------------------------------------------------------------------
// Generation
// ---------------------------------------------------------------------------

pub struct Generated {
    pub pin: Pin,
    pub path: std::path::PathBuf,
    /// Per-line report, for the terminal. Carries no design value: each line is
    /// a name, a theme and a verdict.
    pub report: Vec<String>,
}

/// Compare the two named records and write a pin.
///
/// `cached` is the path to the raw Figma response under `.vds/cache/`, and it is
/// recorded in `generated_by` so the command that reproduces this pin is a
/// command that exists and that names its inputs.
/// Where the decided-target bytes came from, which decides what `generated_by`
/// can truthfully say.
///
/// The distinction is the regeneration limb (VDS S-2(5)(4)): the recorded
/// command has to be one that RUNS. Naming the cache path is wrong for a saved
/// response, because `.vds/cache/` is gitignored and a fresh clone has no copy,
/// so the command reproduces nothing. Naming it is the only option for a network
/// pull, and then the record has to say that reproduction needs the network and
/// a token rather than pretending otherwise.
pub enum Source<'a> {
    /// A response committed in the repository, so the command reproduces.
    Saved(&'a std::path::Path),
    /// Pulled over the network; the cache is the only local copy.
    Network,
}

pub fn generate(
    store: &Store,
    file_key: &str,
    cached: &std::path::Path,
    source: Source<'_>,
    prefix: &str,
    subject: &str,
) -> Result<Generated> {
    let project = store.project;
    let body = std::fs::read_to_string(cached).map_err(|e| VdsError::io(cached.display(), e))?;
    let decided = decided_values(&body)?;
    if decided.is_empty() {
        return Err(VdsError::precondition(format!(
            "{} carries no COLOR variable this generator can read, so there is nothing to \
             compare and a pin written from it would assert an agreement about nothing. Check \
             the response is the `variables/local` endpoint and not a file tree.",
            project.rel(cached)
        )));
    }

    let stylesheet = project.root.join(&project.config.surface.stylesheet);
    let text = std::fs::read_to_string(&stylesheet).map_err(|e| {
        VdsError::io(
            format!(
                "{} (the shipped record, [surface] stylesheet)",
                project.rel(&stylesheet)
            ),
            e,
        )
    })?;
    let sheet = Sheet::parse(&text);
    if let Some(damage) = sheet.malformed() {
        return Err(VdsError::precondition(format!(
            "{}: {damage}. A pin derived from a partial read would record agreement about \
             declarations the scanner never saw.",
            project.rel(&stylesheet)
        )));
    }
    let themes: Vec<String> = sheet
        .theme_selectors()
        .into_iter()
        .map(str::to_owned)
        .collect();
    let base = sheet.base_selector().map(str::to_owned);

    let mut rows = Vec::new();
    let mut report = Vec::new();
    for value in &decided {
        let property = derived_property_name(&value.figma_name, prefix);
        let name = format!("{} @ {}", property.trim_start_matches("--"), value.mode);

        let Some(theme) = theme_for_mode(&value.mode, &themes, base.as_deref()) else {
            rows.push(PinRow::not_enforced(
                &name,
                format!(
                    "the decided-target mode {:?} matches no theme scope the shipped record \
                     declares, so there is no side to compare against. The scopes it declares \
                     are: {}",
                    value.mode,
                    if themes.is_empty() {
                        "none".to_owned()
                    } else {
                        themes.join(", ")
                    }
                ),
            ));
            report.push(format!("  declined  {name}: no theme scope for this mode"));
            continue;
        };

        let resolution = sheet.resolve(&theme, &property);
        let Some(shipped) = resolution.value() else {
            rows.push(PinRow::not_enforced(
                &name,
                format!(
                    "the shipped record does not resolve {property} in {theme}: {}. The \
                     property name is DERIVED from the Figma variable name by replacing `/` \
                     with `-` and prefixing `--{prefix}`, which is a convention this generator \
                     applies and does not verify.",
                    resolution
                        .reason()
                        .map(|r| r.to_string())
                        .unwrap_or_else(|| "no reason recorded".into())
                ),
            ));
            report.push(format!("  declined  {name}: {property} does not resolve"));
            continue;
        };

        // Both sides through ONE parser, and compared at 8-bit, which is what a
        // screen paints. Comparing the text would call `#fff` and `#ffffff`
        // different, and comparing at full float precision would call two
        // colours that render identically different.
        let verdict = match (colour::parse(shipped), colour::parse(&value.as_css)) {
            (Ok(left), Ok(right)) => {
                let agrees = left.quantise_8bit().to_css_hex()
                    == right.quantise_8bit().to_css_hex()
                    && (left.alpha() - right.alpha()).abs() < 0.004;
                rows.push(PinRow {
                    name: name.clone(),
                    agrees,
                    not_enforced_because: None,
                });
                if agrees { "agrees" } else { "DISAGREES" }
            }
            (Err(why), _) => {
                rows.push(PinRow::not_enforced(
                    &name,
                    format!("the shipped value could not be read as a colour: {why}"),
                ));
                "declined"
            }
            (_, Err(why)) => {
                rows.push(PinRow::not_enforced(
                    &name,
                    format!("the decided value could not be read as a colour: {why}"),
                ));
                "declined"
            }
        };
        report.push(format!("  {verdict:9} {name}  ({theme})"));
    }

    let enforced = rows.iter().filter(|row| row.is_enforced()).count() as u64;
    let now = Timestamp::now();
    let id = PinId::allocate(&store.pins_dir(), &now)?;
    let pin = Pin {
        id: id.clone(),
        subject: subject.to_owned(),
        direction: PinDirection::OneWayDerived,
        source_of_record: RecordOfTruth {
            authority_for: "what ships".into(),
            locator: project.rel(&stylesheet),
            digest: Digest::of_file(&stylesheet)?,
        },
        target_of_record: RecordOfTruth {
            authority_for: "what is decided".into(),
            locator: file_key.to_owned(),
            // The digest of the CACHED RESPONSE, which is the bytes this
            // comparison actually read. Digesting anything else would record a
            // claim about a document nobody opened.
            digest: Digest::of_text(&body),
        },
        rows_considered: rows.len() as u64,
        rows_enforced: enforced,
        rows,
        fails_closed: true,
        generated_at: now,
        generated_by: match source {
            Source::Saved(path) => format!(
                "{GENERATOR} --file-key {file_key} --from {} --prefix {prefix:?}",
                project.rel(path)
            ),
            Source::Network => format!(
                "{GENERATOR} --file-key {file_key} --prefix {prefix:?}  (pulled over the \
                 network: reproducing this needs FIGMA_TOKEN and the decided-target file \
                 unchanged, because the cached response at {} is gitignored and absent from a \
                 fresh clone. To make it reproducible offline, commit the response outside \
                 `.vds/` and regenerate with --from.)",
                project.rel(cached)
            ),
        },
        digest: Digest::of_text("placeholder"),
        proof_id: None,
    };
    // The digest LAST, over the finished pin. `token_pin` R5 recomputes exactly
    // this and refuses a pin whose digest does not match, so a generator that
    // computed it early would produce a pin its own gate rejects.
    let pin = Pin {
        digest: pin.compute_content_digest()?,
        ..pin
    };

    let path = store.pins_dir().join(format!("{id}.yaml"));
    std::fs::create_dir_all(store.pins_dir())
        .map_err(|e| VdsError::io(project.rel(&store.pins_dir()), e))?;
    store.create(&path, &pin)?;

    Ok(Generated { pin, path, report })
}

#[cfg(test)]
mod tests {
    use super::*;

    const RESPONSE: &str = r#"{
      "meta": {
        "variableCollections": {
          "C:1": { "modes": [
            { "modeId": "1:0", "name": "Light" },
            { "modeId": "1:1", "name": "Dark" }
          ] }
        },
        "variables": {
          "V:1": {
            "name": "control/border",
            "resolvedType": "COLOR",
            "variableCollectionId": "C:1",
            "valuesByMode": {
              "1:0": { "r": 0.1176, "g": 0.2510, "b": 0.6863, "a": 1 },
              "1:1": { "r": 0.4196, "g": 0.5569, "b": 0.9608, "a": 1 }
            }
          },
          "V:2": {
            "name": "spacing/gap",
            "resolvedType": "FLOAT",
            "variableCollectionId": "C:1",
            "valuesByMode": { "1:0": 12 }
          }
        }
      }
    }"#;

    #[test]
    fn only_colour_variables_are_read_and_each_mode_becomes_a_row() {
        let decided = decided_values(RESPONSE).unwrap();
        assert_eq!(decided.len(), 2, "two modes of one colour variable");
        assert!(
            decided.iter().all(|v| v.figma_name == "control/border"),
            "a FLOAT variable is not a colour and must not be compared as one"
        );
        assert_eq!(decided[0].mode, "Dark");
        assert_eq!(decided[1].mode, "Light");
    }

    #[test]
    fn a_figma_colour_becomes_the_hex_a_stylesheet_would_write() {
        let value = serde_json::json!({ "r": 0.1176, "g": 0.2510, "b": 0.6863, "a": 1 });
        assert_eq!(colour_to_css(&value).unwrap(), "#1e40af");
        let translucent = serde_json::json!({ "r": 0.0, "g": 0.0, "b": 0.0, "a": 0.5 });
        assert_eq!(colour_to_css(&translucent).unwrap(), "rgba(0, 0, 0, 0.5)");
    }

    #[test]
    fn the_property_name_derivation_is_the_one_the_report_describes() {
        assert_eq!(
            derived_property_name("control/border", ""),
            "--control-border"
        );
        assert_eq!(
            derived_property_name("Colour/Control/Border", "sf-"),
            "--sf-colour-control-border"
        );
        assert_eq!(derived_property_name("gap_large", ""), "--gap-large");
    }

    #[test]
    fn a_mode_finds_its_theme_by_name_and_says_so_when_it_cannot() {
        let themes = vec![":root".to_owned(), "[data-theme='dark']".to_owned()];
        assert_eq!(
            theme_for_mode("Dark", &themes, Some(":root")),
            Some("[data-theme='dark']".to_owned())
        );
        assert_eq!(
            theme_for_mode("Light", &themes, Some(":root")),
            Some(":root".to_owned())
        );
        assert_eq!(
            theme_for_mode("Ember", &themes, Some(":root")),
            None,
            "a mode matching no scope must be declined, never quietly measured against the base"
        );
    }

    /// The whole round trip, end to end, in both directions.
    ///
    /// This is the test that makes `token_pin` mean something: before it, the
    /// gate was implemented and vacuous everywhere, because nothing produced a
    /// pin for it to check.
    #[test]
    fn a_generated_pin_agrees_until_the_decided_target_moves() {
        use vds_core::Status;

        let h = crate::testing::Fixture::new();
        h.write(
            "app/globals.css",
            ":root { --control-bg: #1d4ed8; --surface: #ffffff; }\n\
             [data-theme='dark'] { --control-bg: #4f79f0; --surface: #101215; }\n",
        );
        h.register("Button", Status::Registered);
        let saved = h.write("figma/variables.json", RESPONSE_TWO_MODES);
        let store = h.store();
        let cached = cached_variables_path(store.project, "KEY");
        std::fs::create_dir_all(cache_dir(store.project)).unwrap();
        std::fs::copy(&saved, &cached).unwrap();

        let generated = generate(
            &store,
            "KEY",
            &cached,
            Source::Saved(std::path::Path::new("figma/variables.json")),
            "",
            "the control palette",
        )
        .unwrap();

        assert_eq!(generated.pin.rows_considered, 4, "two variables, two modes");
        assert_eq!(
            generated.pin.rows_enforced, 4,
            "every row resolved on both sides: {:?}",
            generated.report
        );
        assert!(
            generated.pin.rows.iter().all(|row| row.agrees),
            "the fixture is built to agree: {:?}",
            generated.report
        );

        // The pin holds a NAME and an AGREEMENT and nothing else. This is the
        // clause the whole architecture turns on (VDS S-2(7)): the first pin
        // carried per-value digests and all 52 values came back out in 27
        // seconds.
        let text = serde_yaml::to_string(&generated.pin).unwrap();
        for value in ["1d4ed8", "4f79f0", "ffffff", "101215", "0.1137", "255"] {
            assert!(
                !text.contains(value),
                "the generated pin carries {value:?}, which is a realisation or a channel of \
                 one:\n{text}"
            );
        }

        // And the pin its own gate checks: R5 recomputes this digest, so a
        // generator that got it wrong would produce a pin the gate rejects.
        assert!(
            generated.pin.digest_matches().unwrap(),
            "the generated pin fails its own gate's integrity check"
        );

        // The failing direction: the designer moves a value in Figma and the
        // code has not caught up.
        let moved = RESPONSE_TWO_MODES.replace("0.3098", "0.5500");
        std::fs::write(&cached, &moved).unwrap();
        std::fs::remove_file(&generated.path).unwrap();
        let after = generate(
            &store,
            "KEY",
            &cached,
            Source::Saved(std::path::Path::new("figma/variables.json")),
            "",
            "the control palette",
        )
        .unwrap();
        assert_eq!(
            after.pin.rows.iter().filter(|row| !row.agrees).count(),
            1,
            "a moved decided value must produce exactly one disagreement: {:?}",
            after.report
        );
    }

    /// A variable the derivation cannot place is DECLINED with the reason, never
    /// guessed and never silently dropped.
    #[test]
    fn a_variable_the_stylesheet_does_not_declare_is_declined_with_its_reason() {
        use vds_core::Status;

        let h = crate::testing::Fixture::new();
        // Both theme scopes exist, so the MODE resolves and the decline is
        // about the property rather than about the theme: `--control-bg` is
        // decided in Figma and declared nowhere in the shipped record.
        h.write(
            "app/globals.css",
            ":root { --surface: #ffffff; }\n[data-theme='dark'] { --surface: #101215; }\n",
        );
        h.register("Button", Status::Registered);
        let store = h.store();
        let cached = cached_variables_path(store.project, "KEY");
        std::fs::create_dir_all(cache_dir(store.project)).unwrap();
        std::fs::write(&cached, RESPONSE_TWO_MODES).unwrap();

        let generated = generate(&store, "KEY", &cached, Source::Network, "", "subject").unwrap();
        let declined: Vec<&PinRow> = generated
            .pin
            .rows
            .iter()
            .filter(|row| !row.is_enforced())
            .collect();
        assert!(!declined.is_empty(), "{:?}", generated.report);
        let because = declined
            .iter()
            .find_map(|row| row.not_enforced_because.as_deref())
            .filter(|reason| reason.contains("does not resolve"))
            .unwrap_or_else(|| panic!("{:?}", generated.report));
        assert!(
            because.contains("DERIVED"),
            "the reason has to say the correspondence was derived rather than measured, or a \
             reader takes a declined row for a missing token: {because}"
        );
    }

    /// A network pull cannot claim a command that reproduces offline, because
    /// the cache it would read is gitignored and absent from a fresh clone.
    #[test]
    fn generated_by_names_a_command_that_can_actually_run() {
        use vds_core::Status;

        let h = crate::testing::Fixture::new();
        h.write(
            "app/globals.css",
            ":root { --control-bg: #1d4ed8; --surface: #ffffff; }\n\
             [data-theme='dark'] { --control-bg: #4f79f0; --surface: #101215; }\n",
        );
        h.register("Button", Status::Registered);
        let store = h.store();
        let cached = cached_variables_path(store.project, "KEY");
        std::fs::create_dir_all(cache_dir(store.project)).unwrap();
        std::fs::write(&cached, RESPONSE_TWO_MODES).unwrap();

        let network = generate(&store, "KEY", &cached, Source::Network, "", "s").unwrap();
        assert!(
            network.pin.generated_by.contains("gitignored"),
            "a pin pulled over the network must say that reproducing it needs the network, \
             rather than naming a cache path a fresh clone does not have: {}",
            network.pin.generated_by
        );
        std::fs::remove_file(&network.path).unwrap();

        let saved = generate(
            &store,
            "KEY",
            &cached,
            Source::Saved(std::path::Path::new("figma/variables.json")),
            "",
            "s",
        )
        .unwrap();
        assert!(
            saved
                .pin
                .generated_by
                .contains("--from figma/variables.json"),
            "{}",
            saved.pin.generated_by
        );
        assert!(
            !saved.pin.generated_by.contains(".vds/cache"),
            "{}",
            saved.pin.generated_by
        );
    }

    const RESPONSE_TWO_MODES: &str = r#"{
      "meta": {
        "variableCollections": {
          "C:1": { "modes": [
            { "modeId": "1:0", "name": "Light" },
            { "modeId": "1:1", "name": "Dark" }
          ] }
        },
        "variables": {
          "V:1": {
            "name": "control/bg",
            "resolvedType": "COLOR",
            "variableCollectionId": "C:1",
            "valuesByMode": {
              "1:0": { "r": 0.1137, "g": 0.3059, "b": 0.8471, "a": 1 },
              "1:1": { "r": 0.3098, "g": 0.4745, "b": 0.9412, "a": 1 }
            }
          },
          "V:2": {
            "name": "surface",
            "resolvedType": "COLOR",
            "variableCollectionId": "C:1",
            "valuesByMode": {
              "1:0": { "r": 1, "g": 1, "b": 1, "a": 1 },
              "1:1": { "r": 0.0627, "g": 0.0706, "b": 0.0824, "a": 1 }
            }
          }
        }
      }
    }"#;
}
