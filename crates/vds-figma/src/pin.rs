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

/// What kind of realisation a decided value is, which decides how it compares.
///
/// Two kinds, because a colour and a length are compared by different rules and
/// a single comparison that tried to do both would be wrong about one of them. A
/// colour is compared at 8-bit through one parser; a length is a number that
/// carries a unit on the CSS side and none on the Figma side.
#[derive(Debug, Clone)]
enum Decided {
    /// A colour, as CSS text so both sides go through one parser.
    Colour(String),
    /// A Figma FLOAT: a bare number, which Figma means in pixels for anything
    /// spatial and unitless for anything proportional (an opacity, a
    /// line-height multiplier). Which of the two it is cannot be read off the
    /// value, so the comparison decides from the CSS side, where the unit is
    /// written down.
    Number(f64),
}

/// One Figma variable, reduced to what a comparison needs.
#[derive(Debug, Clone)]
struct DecidedValue {
    /// The variable's Figma name, e.g. `control/border`.
    figma_name: String,
    /// The mode's name, e.g. `Dark`.
    mode: String,
    /// The value.
    ///
    /// Held in memory and never written to a record. It reaches a `PinRow` only
    /// as the boolean that comes out of comparing it.
    decided: Decided,
}

/// How the decided-target values reached this machine.
///
/// # Why this is a field and not a comment
///
/// The REST `variables/local` endpoint needs an Enterprise plan and the
/// `file_variables:read` scope, and returns 403 rather than an empty result
/// without them. That is a real wall, and the way round it is that the BYTES do
/// not have to come from REST: the plugin API and the Figma MCP server both
/// expose variables on any plan, and an export from either is a perfectly good
/// decided-target reading.
///
/// What must not happen is the obvious shortcut. A file of variable values
/// somebody TYPED is a third system of record, and VDS S-2(1) and
/// [2026] VJS-CC-OPBOX 3 are flat about it: an artefact that STORES token values
/// is an authority, and only a deriving gate is permitted. A pin between the
/// shipped stylesheet and a hand-authored copy of what Figma is believed to say
/// agrees with itself and checks nothing, which is the failure this generator's
/// own module doc warns about.
///
/// So the distinction between EXPORTED and AUTHORED is recorded, per pin, in a
/// place a reader of the pin sees. An export that declares its provenance is a
/// reading of the decided target; one that declares none is a file of unknown
/// origin, and the pin says so rather than presenting it as a measurement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Provenance {
    /// The REST `variables/local` response, which carries its own shape.
    RestApi,
    /// An export declaring where it came from and when.
    Export {
        tool: String,
        file_version: Option<String>,
        exported_at: Option<String>,
    },
    /// A flat map of names to values with nothing saying where it came from.
    Undeclared,
}

impl Provenance {
    /// What the pin records about how the decided target was read.
    pub fn describe(&self) -> String {
        match self {
            Provenance::RestApi => "the Figma REST variables/local endpoint".to_owned(),
            Provenance::Export {
                tool,
                file_version,
                exported_at,
            } => format!(
                "an export by {tool}{}{}",
                file_version
                    .as_deref()
                    .map(|v| format!(", of file version {v}"))
                    .unwrap_or_default(),
                exported_at
                    .as_deref()
                    .map(|t| format!(", at {t}"))
                    .unwrap_or_default()
            ),
            Provenance::Undeclared => "a source that declares no provenance".to_owned(),
        }
    }

    /// Whether this reading declares where it came from.
    ///
    /// `false` does not make a pin unlawful. It makes it a pin whose
    /// decided-target side is a file of unknown origin, which is a fact a reader
    /// has to be told rather than left to assume.
    pub fn is_declared(&self) -> bool {
        !matches!(self, Provenance::Undeclared)
    }
}

/// The envelope an export should be wrapped in.
///
/// Deliberately small, and deliberately NOT a VDS artefact type: it describes a
/// file that lives in the subject project, outside `.vds/`, alongside the
/// stylesheet. VDS reads it and does not own it, which is the same relationship
/// S-2(3) fixes for `app/globals.css`.
#[derive(Debug, Deserialize)]
struct Envelope {
    /// What produced it: a plugin name, `figma-mcp`, whatever ran.
    #[serde(rename = "exportedBy", alias = "exported_by")]
    exported_by: String,
    #[serde(rename = "fileVersion", alias = "file_version", default)]
    file_version: Option<String>,
    #[serde(rename = "exportedAt", alias = "exported_at", default)]
    exported_at: Option<String>,
    /// `{"Light": {"control/bg": "#1d4ed8"}, "Dark": {...}}`, or a single
    /// unnamed mode as a flat map.
    variables: serde_json::Value,
}

/// Everything the decided-target file says, per variable name and mode.
///
/// Three accepted shapes, tried in order of how much they declare about
/// themselves. Trying the richest first matters: an envelope also parses as a
/// flat map if the flat reader is tolerant, and taking it as one would throw
/// away the provenance the author went to the trouble of writing.
fn decided_values(body: &str) -> Result<(Vec<DecidedValue>, Provenance)> {
    match rest_values(body) {
        Ok(values) => Ok((values, Provenance::RestApi)),
        Err(rest_error) => {
            if let Some(found) = envelope_values(body)? {
                return Ok(found);
            }
            match flat_values(body) {
                Some(values) => Ok((values, Provenance::Undeclared)),
                None => Err(rest_error),
            }
        }
    }
}

/// An export wrapped in the envelope, which is the shape to prefer.
fn envelope_values(body: &str) -> Result<Option<(Vec<DecidedValue>, Provenance)>> {
    let Ok(envelope) = serde_json::from_str::<Envelope>(body) else {
        return Ok(None);
    };
    let provenance = Provenance::Export {
        tool: envelope.exported_by.clone(),
        file_version: envelope.file_version.clone(),
        exported_at: envelope.exported_at.clone(),
    };
    let text = serde_json::to_string(&envelope.variables).map_err(|e| {
        VdsError::precondition(format!(
            "the export's `variables` could not be re-read: {e}"
        ))
    })?;
    let Some(values) = flat_values(&text) else {
        return Err(VdsError::precondition(
            "the export declares its provenance and its `variables` is not a map this reader              understands. Two shapes are accepted: a flat map of name to value for a              single-mode file, and a map of MODE NAME to such a map for a multi-mode one.",
        ));
    };
    Ok(Some((values, provenance)))
}

/// A flat map, or a map of mode names to flat maps.
///
/// This is what the plugin API and the Figma MCP server produce, and neither
/// carries the REST envelope. A value is a string (`"#1d4ed8"`, `"12px"`) or a
/// number (`12`), which is the difference between a colour and a FLOAT and is
/// read the same way it is read from REST.
fn flat_values(body: &str) -> Option<Vec<DecidedValue>> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    let object = value.as_object()?;

    let read = |name: &str, raw: &serde_json::Value| -> Option<Decided> {
        if let Some(number) = raw.as_f64() {
            return Some(Decided::Number(number));
        }
        let text = raw.as_str()?.trim();
        let _ = name;
        // A string that parses as a bare number is a FLOAT written as text,
        // which every JSON exporter does at least sometimes.
        if let Ok(number) = text.parse::<f64>() {
            return Some(Decided::Number(number));
        }
        Some(Decided::Colour(text.to_owned()))
    };

    // Multi-mode: every value is itself an object.
    let nested = !object.is_empty() && object.values().all(|v| v.is_object());
    let mut out = Vec::new();
    if nested {
        for (mode, entries) in object {
            for (name, raw) in entries.as_object()? {
                if let Some(decided) = read(name, raw) {
                    out.push(DecidedValue {
                        figma_name: name.clone(),
                        mode: mode.clone(),
                        decided,
                    });
                }
            }
        }
    } else {
        for (name, raw) in object {
            if raw.is_object() || raw.is_array() {
                continue;
            }
            if let Some(decided) = read(name, raw) {
                out.push(DecidedValue {
                    figma_name: name.clone(),
                    // A flat map declares no mode. `Default` is the name
                    // `theme_for_mode` maps onto the base scope, which is the
                    // only lawful reading of a file that names none.
                    mode: "Default".to_owned(),
                    decided,
                });
            }
        }
    }
    (!out.is_empty()).then(|| {
        out.sort_by(|a, b| (&a.figma_name, &a.mode).cmp(&(&b.figma_name, &b.mode)));
        out
    })
}

fn rest_values(body: &str) -> Result<Vec<DecidedValue>> {
    let response: VariablesResponse = serde_json::from_str(body).map_err(|e| {
        VdsError::precondition(format!(
            "the decided-target reading is not a shape this generator understands: {e}\n  \
             Three are accepted:\n  \
             (1) the REST `GET /v1/files/:key/variables/local` response, which needs an \
             Enterprise plan and the `file_variables:read` scope and returns 403 without \
             them;\n  \
             (2) an export declaring its provenance: an object with `exportedBy`, optionally \
             `fileVersion` and `exportedAt`, and `variables` holding either a flat map of name \
             to value or a map of mode name to such a map;\n  \
             (3) a bare flat map, which the plugin API and the Figma MCP server both produce. \
             This one is accepted and RECORDED as declaring no provenance, because a file of \
             variable values that says nothing about where it came from could equally have \
             been typed, and a pin against a typed file agrees with itself and checks nothing \
             (VDS S-2(1))."
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
        // COLOR and FLOAT. STRING and BOOLEAN are skipped and the skip is named
        // rather than silent: neither carries a realisation a stylesheet
        // declares, so there is nothing on the shipped side to compare against.
        //
        // FLOAT is the one that matters for pixel parity. A design system drifts
        // in SPACING long before it drifts in hue: a gap that is 12 in the
        // decided target and 0.75rem in the code agrees, and one that is 12 and
        // 14px does not, and until this read FLOAT nothing in VDS could tell
        // those two apart.
        let _ = &variable.collection_id;
        for (mode_id, value) in &variable.values_by_mode {
            let Some(mode) = modes.get(mode_id.as_str()) else {
                continue;
            };
            let decided = match variable.resolved_type.as_str() {
                "COLOR" => match colour_to_css(value) {
                    Some(css) => Decided::Colour(css),
                    None => continue,
                },
                "FLOAT" => match value.as_f64() {
                    Some(number) => Decided::Number(number),
                    None => continue,
                },
                _ => continue,
            };
            out.push(DecidedValue {
                figma_name: variable.name.clone(),
                mode: (*mode).to_owned(),
                decided,
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

/// The CSS initial root font size, and the only value a `rem` can be resolved
/// against without being told.
///
/// It is the CSS default, and it is a REQUIREMENT rather than a realisation: it
/// comes from the specification, not from the design. A project that changes it
/// declares so with `--root-px`, and a comparison made against the wrong one
/// would be confidently wrong about every `rem` in the sheet.
pub const DEFAULT_ROOT_PX: f64 = 16.0;

/// How close two lengths have to be to agree.
///
/// A hundredth of a pixel. Figma writes a float and a stylesheet writes a
/// decimal, and the two round differently at the last place for reasons that
/// have nothing to do with the design; a tolerance below what any screen can
/// paint keeps that from becoming a finding. It is a REQUIREMENT and it is named
/// here so it is contestable rather than hidden in a comparison.
pub const LENGTH_TOLERANCE_PX: f64 = 0.01;

/// Compare a CSS length against a bare Figma number.
///
/// Figma FLOATs carry no unit. For anything spatial Figma means PIXELS, which is
/// the convention its own UI shows; for anything proportional (an opacity, a
/// line-height multiplier) it means a bare ratio. Which of the two a variable is
/// cannot be read off the value, so the CSS side decides, because that is the
/// side where the unit is written down.
///
/// `px` and a bare number compare directly. `rem` and `em` convert through the
/// root size, which is declared rather than guessed. Every other unit is
/// DECLINED with the reason: a `vw` against a bare number needs a viewport, and
/// a comparison that invented one would be a measurement of nothing.
fn compare_length(shipped: &str, decided: f64, root_px: f64) -> std::result::Result<bool, String> {
    let text = shipped.trim();
    let split = text
        .char_indices()
        .find(|(_, c)| c.is_ascii_alphabetic() || *c == '%')
        .map(|(index, _)| index)
        .unwrap_or(text.len());
    let (number, unit) = text.split_at(split);
    let Ok(number) = number.trim().parse::<f64>() else {
        return Err(
            "the shipped value is not a number this comparison can read. The decided \
                    target holds a FLOAT, which is a length or a ratio, and the shipped record \
                    holds something that is neither."
                .to_owned(),
        );
    };
    let unit = unit.trim().to_ascii_lowercase();
    let shipped_px = match unit.as_str() {
        "" | "px" => number,
        "rem" | "em" => number * root_px,
        other => {
            return Err(format!(
                "the shipped value carries the unit `{other}`, which cannot be compared against \
                 a bare Figma FLOAT without a context this run does not have: a viewport for \
                 `vw` and `vh`, a font for `ch` and `ex`, a containing box for `%`. Declining \
                 is the honest answer; converting would invent the context and report the \
                 invention as a measurement."
            ));
        }
    };
    Ok((shipped_px - decided).abs() < LENGTH_TOLERANCE_PX)
}

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

#[allow(clippy::too_many_arguments)]
pub fn generate(
    store: &Store,
    file_key: &str,
    cached: &std::path::Path,
    source: Source<'_>,
    prefix: &str,
    subject: &str,
    root_px: f64,
) -> Result<Generated> {
    let project = store.project;
    let body = std::fs::read_to_string(cached).map_err(|e| VdsError::io(cached.display(), e))?;
    let (decided, provenance) = decided_values(&body)?;
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

        let verdict = match &value.decided {
            // Both sides through ONE parser, and compared at 8-bit, which is
            // what a screen paints. Comparing the text would call `#fff` and
            // `#ffffff` different, and comparing at full float precision would
            // call two colours that render identically different.
            Decided::Colour(as_css) => match (colour::parse(shipped), colour::parse(as_css)) {
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
            },
            // The half that was missing, and the half a design system drifts in
            // first: spacing, radii and sizes.
            Decided::Number(number) => match compare_length(shipped, *number, root_px) {
                Ok(agrees) => {
                    rows.push(PinRow {
                        name: name.clone(),
                        agrees,
                        not_enforced_because: None,
                    });
                    if agrees { "agrees" } else { "DISAGREES" }
                }
                Err(why) => {
                    rows.push(PinRow::not_enforced(&name, why));
                    "declined"
                }
            },
        };
        report.push(format!("  {verdict:9} {name}  ({theme})"));
    }

    // The provenance goes in the SUBJECT, which is the one free-text field a
    // reader of the pin meets first and the one `token_pin` prints. A pin whose
    // decided-target side came from a file of unknown origin has to say so
    // there, not in a comment: a hand-authored file of variable values is a
    // third system of record (VDS S-2(1)), and a pin against one agrees with
    // itself and checks nothing.
    let subject = format!(
        "{subject} (decided target read from {}{})",
        provenance.describe(),
        if provenance.is_declared() {
            ""
        } else {
            "; NOTHING here establishes that those values came out of Figma rather than being \
             typed, and a pin against a typed file is a pin against a copy of itself"
        }
    );
    report.push(String::new());
    report.push(format!("  decided target: {}", provenance.describe()));
    if !provenance.is_declared() {
        report.push(
            "  WARNING: that reading declares no provenance. Wrap it in an envelope carrying"
                .to_owned(),
        );
        report.push(
            "  `exportedBy`, `fileVersion` and `exportedAt` so the pin records a READING of the"
                .to_owned(),
        );
        report.push("  decided target rather than a file of unknown origin.".to_owned());
    }

    let enforced = rows.iter().filter(|row| row.is_enforced()).count() as u64;
    let now = Timestamp::now();
    let id = PinId::allocate(&store.pins_dir(), &now)?;
    let pin = Pin {
        id: id.clone(),
        subject,
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

    /// COLOR and FLOAT are both read; STRING and BOOLEAN are not, because
    /// neither carries a realisation a stylesheet declares.
    #[test]
    fn colour_and_float_variables_are_both_read_and_each_mode_becomes_a_row() {
        let (decided, provenance) = decided_values(RESPONSE).unwrap();
        assert_eq!(provenance, Provenance::RestApi);
        assert_eq!(
            decided.len(),
            3,
            "two modes of a colour and one of a float: {decided:?}"
        );
        let float = decided
            .iter()
            .find(|v| v.figma_name == "spacing/gap")
            .expect("the FLOAT variable is read, not skipped");
        assert!(matches!(float.decided, Decided::Number(n) if (n - 12.0).abs() < f64::EPSILON));
    }

    /// A design system drifts in SPACING before it drifts in hue, and until this
    /// compared FLOATs nothing in VDS could tell `0.75rem` from `14px`.
    #[test]
    fn a_length_agrees_across_units_and_disagrees_on_the_value() {
        // px against a bare Figma number: the ordinary case.
        assert_eq!(compare_length("12px", 12.0, 16.0), Ok(true));
        assert_eq!(compare_length("14px", 12.0, 16.0), Ok(false));
        // rem, through the declared root. 0.75rem IS 12px, and calling those two
        // different would make every project using rem fail on agreement.
        assert_eq!(compare_length("0.75rem", 12.0, 16.0), Ok(true));
        assert_eq!(compare_length("0.75rem", 12.0, 20.0), Ok(false));
        // A bare number: a ratio on both sides, an opacity or a line height.
        assert_eq!(compare_length("1.5", 1.5, 16.0), Ok(true));
        // Tolerance below what any screen can paint, so a last-place rounding
        // difference is not a finding.
        assert_eq!(compare_length("12.001px", 12.0, 16.0), Ok(true));
        assert_eq!(compare_length("12.5px", 12.0, 16.0), Ok(false));
    }

    /// A unit that needs a context this run does not have is DECLINED, never
    /// converted against an invented one.
    #[test]
    fn a_unit_needing_a_context_this_run_lacks_is_declined_with_the_reason() {
        for unit in ["50vw", "10ch", "80%", "3ex"] {
            let refused = compare_length(unit, 12.0, 16.0).unwrap_err();
            assert!(refused.contains("cannot be compared"), "{unit}: {refused}");
            assert!(
                refused.contains("invent"),
                "the reason has to say why declining is the honest answer: {refused}"
            );
        }
        assert!(compare_length("solid", 1.0, 16.0).is_err());
    }

    /// The wall the REST endpoint puts up, and the way round it.
    ///
    /// `variables/local` needs an Enterprise plan and returns 403 without one.
    /// The plugin API and the Figma MCP server both produce a flat map on any
    /// plan, so that shape is read too. What must NOT happen is a file somebody
    /// typed passing as a reading of Figma: a hand-authored set of values is a
    /// third system of record (VDS S-2(1)), and a pin against one agrees with
    /// itself. So an undeclared source is accepted and RECORDED as undeclared.
    #[test]
    fn an_export_is_read_and_its_provenance_is_recorded_either_way() {
        let declared = r##"{
          "exportedBy": "figma-mcp get_variable_defs",
          "fileVersion": "123456",
          "exportedAt": "2026-07-25T10:00:00Z",
          "variables": {
            "Light": { "control/bg": "#1d4ed8", "spacing/gap": 12 },
            "Dark":  { "control/bg": "#4f79f0", "spacing/gap": 12 }
          }
        }"##;
        let (values, provenance) = decided_values(declared).unwrap();
        assert_eq!(values.len(), 4);
        assert!(provenance.is_declared());
        assert!(
            provenance.describe().contains("figma-mcp"),
            "{provenance:?}"
        );
        assert!(provenance.describe().contains("123456"), "{provenance:?}");

        // The same values with nothing saying where they came from.
        let bare = r##"{ "control/bg": "#1d4ed8", "spacing/gap": 12 }"##;
        let (values, provenance) = decided_values(bare).unwrap();
        assert_eq!(values.len(), 2);
        assert!(
            !provenance.is_declared(),
            "a bare map could equally have been typed, and a pin must say so"
        );
        assert_eq!(values[0].mode, "Default", "a flat map declares no mode");
    }

    #[test]
    fn a_reading_this_generator_cannot_understand_names_all_three_shapes() {
        let error = decided_values("[1, 2, 3]").unwrap_err().to_string();
        for expected in ["Enterprise", "exportedBy", "typed"] {
            assert!(error.contains(expected), "{error}");
        }
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
            DEFAULT_ROOT_PX,
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
            DEFAULT_ROOT_PX,
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

        let generated = generate(
            &store,
            "KEY",
            &cached,
            Source::Network,
            "",
            "subject",
            DEFAULT_ROOT_PX,
        )
        .unwrap();
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

        let network = generate(
            &store,
            "KEY",
            &cached,
            Source::Network,
            "",
            "s",
            DEFAULT_ROOT_PX,
        )
        .unwrap();
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
            DEFAULT_ROOT_PX,
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
