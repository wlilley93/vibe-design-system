//! The geometry reading: what SHAPE the shipped stylesheet gives its surfaces.
//!
//! The generator behind `vds ledger geometry`, and the input the `geometry` proof
//! measures a bound against.
//!
//! # What it decides, and what it refuses to decide
//!
//! VDS does not hold the thresholds. Whether a compliant radius is 6px or 8px is
//! the subject's design system talking, and VDS taking a view would make it a
//! fourth design authority, which [2026] VJS-CC-OPBOX 3 forbids.
//!
//! What it does decide is structural and is the same move `no_stored_values`
//! makes: a declaration whose value REFERENCES the token layer takes its shape
//! from the system, and one that spells a length out does not. `border-radius:
//! var(--radius-md)` complies whatever `--radius-md` turns out to be;
//! `border-radius: 7px` does not comply with anything, because there is nothing
//! for it to comply WITH. No number in this module is a design value, and it
//! reads none.
//!
//! A subject wanting a stricter rule (say, that a radius must reference one of
//! three named tokens) writes its own generator and names it in `generatedBy`.
//! The reading is a schema, not a monopoly.
//!
//! # Why a reset counts as compliant
//!
//! `border-radius: 0`, `border-width: 0` and the keywords (`inherit`, `initial`,
//! `unset`, `revert`, `none`) are counted COMPLIANT, and the choice is stated
//! here rather than buried. S-7A(1) is about surfaces that carry a shape the
//! design system did not give them. A reset carries no shape at all: it is a
//! deliberate absence, not a hand-rolled value, and there is no smaller number to
//! drive it towards. Counting resets as violations would put a floor under every
//! bound made of them, and a bound that cannot reach zero is the ratchet S-7A(2)
//! refuses wearing a different hat.
//!
//! # What is UNDECIDED, and why it is counted rather than assumed away
//!
//! A value this reader cannot classify is `undecided`, never folded into
//! `compliant`. `geometry` R7 then treats a bound the undecided could carry it
//! past as UNDECIDED rather than met. Folding them into compliant turns a census
//! into flattery, which is the direction this programme has already paid for.

use std::collections::BTreeMap;

use vds_core::{
    GeometryReading, KindReading, Project, READING_SCHEMA_VERSION, ReadFrom, Result, SurfaceKind,
    Timestamp, VdsError,
};

pub const GENERATOR_COMMAND: &str = "vds ledger geometry";

/// The default shipped stylesheet, matching `contrast`'s.
///
/// The same constant and the same reason: this is the file VDS S-2(3) fixes as
/// the system of record for what the product actually looks like, and a geometry
/// reading taken from a different file than the contrast proof measures would
/// let the two report on two different products.
pub const SHIPPED_STYLESHEET: &str = "app/globals.css";

/// Which CSS properties carry each shape.
///
/// Matched on the property name with its longhands, so `border-radius` and
/// `border-top-left-radius` both count. A property absent from this table is not
/// read AT ALL, which is why [`GeometryReading::does_not_cover`] names the
/// omission rather than leaving a reader to infer full coverage.
fn kind_of(property: &str) -> Option<SurfaceKind> {
    let p = property.to_ascii_lowercase();
    if p.ends_with("radius") {
        return Some(SurfaceKind::Radius);
    }
    if p == "border-width"
        || (p.starts_with("border-") && p.ends_with("-width"))
        || p == "outline-width"
    {
        return Some(SurfaceKind::BoundaryWeight);
    }
    // `margin` is deliberately absent. It positions a surface relative to its
    // siblings, which is layout; density is the space a surface keeps INSIDE
    // itself, and conflating them makes a bound that two unrelated pieces of
    // work both move.
    if p == "padding"
        || p.starts_with("padding-")
        || p == "gap"
        || p == "row-gap"
        || p == "column-gap"
    {
        return Some(SurfaceKind::Density);
    }
    if p == "font-size" || p == "line-height" || p == "letter-spacing" {
        return Some(SurfaceKind::TypeScale);
    }
    None
}

/// A value that asserts no shape: a reset, not a hand-rolled number.
fn is_reset(value: &str) -> bool {
    let v = value.trim().to_ascii_lowercase();
    matches!(
        v.as_str(),
        "0" | "0px"
            | "0rem"
            | "0em"
            | "none"
            | "inherit"
            | "initial"
            | "unset"
            | "revert"
            | "auto"
            | "normal"
    )
}

/// How one declaration's value classifies.
enum Verdict {
    /// References the token layer, so it takes its shape from the system.
    Complies,
    /// Spells a shape out, so it complies with nothing.
    DoesNot,
    /// The reader cannot tell.
    Undecided,
}

fn classify(value: &str) -> Verdict {
    let v = value.trim();
    if v.is_empty() {
        return Verdict::Undecided;
    }
    // A `var()` ANYWHERE in the value, including inside a `calc()` or a
    // multi-value shorthand, means the shape came from the token layer.
    // `calc(var(--radius-md) - 1px)` is a system radius adjusted for a border,
    // which is a legitimate derivation and not a hand-rolled number.
    if v.contains("var(") {
        return Verdict::Complies;
    }
    if is_reset(v) {
        return Verdict::Complies;
    }
    // A shorthand of several resets: `padding: 0 0`, `border-radius: 0 0 0 0`.
    if v.split_whitespace().all(is_reset) {
        return Verdict::Complies;
    }
    // A bare length or percentage, alone or in a shorthand. This is the shape
    // the bound is about.
    let looks_like_length = |t: &str| {
        let t = t.trim_end_matches(|c: char| c.is_ascii_alphabetic() || c == '%');
        !t.is_empty()
            && t.chars()
                .all(|c| c.is_ascii_digit() || c == '.' || c == '-')
    };
    if v.split_whitespace()
        .all(|t| looks_like_length(t) || is_reset(t))
    {
        return Verdict::DoesNot;
    }
    // Anything else: a keyword this reader does not know, an `env()`, a
    // `clamp()` with no var, a value spanning a construct it cannot parse. It
    // says so rather than guessing, and `geometry` R7 makes the caller account
    // for it.
    Verdict::Undecided
}

/// Which stylesheet to read, per `[surface] stylesheet` or the default.
pub fn stylesheet_path(project: &Project) -> std::path::PathBuf {
    project.root.join(SHIPPED_STYLESHEET)
}

/// Generate the reading from the shipped stylesheet.
///
/// `taken_at` is supplied by the caller rather than read from the clock here, so
/// a test can pin it. It lands INSIDE the content digest, because `geometry` R3
/// measures the bound's window from it and a digest that skipped it would leave
/// the field with the most leverage unwitnessed.
pub fn build(project: &Project, taken_at: Timestamp) -> Result<GeometryReading> {
    let path = stylesheet_path(project);
    let source = std::fs::read_to_string(&path).map_err(|e| VdsError::io(path.display(), e))?;
    let sheet = vds_css::sheet::Sheet::parse(&source);
    // Structural damage is FATAL and not a narrowing. A declaration the scanner
    // did not see is not skipped and not counted: it does not exist, and every
    // count below would look healthy while the reader read a file it never read.
    if let Some(malformed) = sheet.malformed() {
        return Err(VdsError::Artefact {
            path: project.rel(&path),
            message: format!(
                "is structurally damaged and was not fully read: {malformed}. A geometry \
                 reading taken from a partly-read stylesheet undercounts every kind, and an \
                 undercount reads exactly like progress."
            ),
        });
    }

    let mut tally: BTreeMap<SurfaceKind, (u32, u32, u32)> = BTreeMap::new();
    let mut samples: BTreeMap<SurfaceKind, Vec<String>> = BTreeMap::new();
    for declaration in sheet.properties() {
        let Some(kind) = kind_of(&declaration.property) else {
            continue;
        };
        let entry = tally.entry(kind).or_insert((0, 0, 0));
        entry.0 += 1;
        match classify(&declaration.value) {
            Verdict::Complies => {}
            Verdict::DoesNot => {
                entry.1 += 1;
                let sample = samples.entry(kind).or_default();
                if sample.len() < 8 {
                    sample.push(format!(
                        "{}:{} {} in {}",
                        project.rel(&path),
                        declaration.line,
                        declaration.property,
                        declaration.selector
                    ));
                }
            }
            Verdict::Undecided => entry.2 += 1,
        }
    }

    let kinds: Vec<KindReading> = SurfaceKind::ALL
        .into_iter()
        .filter_map(|kind| {
            tally.get(&kind).map(|(considered, non, und)| KindReading {
                surface_kind: kind,
                considered: *considered,
                non_compliant: *non,
                undecided: *und,
                sample: samples.get(&kind).cloned().unwrap_or_default(),
            })
        })
        .collect();

    let mut reading = GeometryReading {
        schema_version: READING_SCHEMA_VERSION,
        generated_by: GENERATOR_COMMAND.to_owned(),
        taken_at,
        // The SOURCE and not the built bundle. Weaker than reading compiled CSS
        // by exactly the distance a build step can introduce, and the field says
        // which was taken so nobody has to guess (VDS S-7A(4)).
        read_from: ReadFrom::ShippedSource,
        sources: vec![project.rel(&path)],
        kinds,
        does_not_cover: vec![
            "utility classes. A framework that composes shape from class names \
             (`rounded-lg`, `p-4`) puts it in the markup, and this reader looks only at the \
             stylesheet. On such a project the counts here are a floor and the reading needs \
             its own generator."
                .into(),
            "inline style attributes and any shape set from JavaScript.".into(),
            "`margin`, which positions a surface among its siblings rather than shaping it. \
             Density here is the space a surface keeps inside itself."
                .into(),
            "whether a referenced token holds a value the design system endorses. This reader \
             establishes that a surface takes its shape FROM the system, never that the \
             system's own value is right, which is not VDS's to say."
                .into(),
        ],
        content_digest: vds_core::Digest::of_text("placeholder"),
    };
    reading.content_digest = reading.compute_content_digest()?;
    Ok(reading)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(value: &str) -> &'static str {
        match classify(value) {
            Verdict::Complies => "complies",
            Verdict::DoesNot => "does_not",
            Verdict::Undecided => "undecided",
        }
    }

    #[test]
    fn a_token_reference_complies_however_it_is_wrapped() {
        assert_eq!(v("var(--radius-md)"), "complies");
        assert_eq!(
            v("calc(var(--radius-md) - 1px)"),
            "complies",
            "a system radius adjusted for a border is a derivation, not a hand-rolled number"
        );
        assert_eq!(v("var(--r) var(--r) 0 0"), "complies");
    }

    #[test]
    fn a_spelled_out_length_complies_with_nothing() {
        assert_eq!(v("7px"), "does_not");
        assert_eq!(v("0.5rem"), "does_not");
        assert_eq!(v("6px 6px 0 0"), "does_not");
        assert_eq!(v("50%"), "does_not");
    }

    #[test]
    fn a_reset_asserts_no_shape_and_is_not_a_violation() {
        // Stated in the module doc and tested here: counting resets as
        // violations puts a floor under every bound made of them, and a bound
        // that cannot reach zero is the ratchet S-7A(2) refuses.
        for reset in [
            "0", "0px", "none", "inherit", "initial", "unset", "auto", "normal",
        ] {
            assert_eq!(v(reset), "complies", "{reset}");
        }
        assert_eq!(v("0 0 0 0"), "complies");
    }

    #[test]
    fn a_value_the_reader_cannot_classify_says_so_rather_than_guessing() {
        assert_eq!(v("env(safe-area-inset-top)"), "undecided");
        assert_eq!(v("clamp(1rem, 2vw, 3rem)"), "undecided");
        assert_eq!(v(""), "undecided");
    }

    #[test]
    fn the_property_table_maps_longhands_and_excludes_margin() {
        assert_eq!(kind_of("border-radius"), Some(SurfaceKind::Radius));
        assert_eq!(kind_of("border-top-left-radius"), Some(SurfaceKind::Radius));
        assert_eq!(kind_of("border-width"), Some(SurfaceKind::BoundaryWeight));
        assert_eq!(
            kind_of("border-bottom-width"),
            Some(SurfaceKind::BoundaryWeight)
        );
        assert_eq!(kind_of("padding-inline"), Some(SurfaceKind::Density));
        assert_eq!(kind_of("gap"), Some(SurfaceKind::Density));
        assert_eq!(kind_of("font-size"), Some(SurfaceKind::TypeScale));
        // Layout, not shape. A bound that counted margin would be moved by two
        // unrelated pieces of work and would tell nobody which.
        assert_eq!(kind_of("margin"), None);
        assert_eq!(kind_of("margin-top"), None);
        assert_eq!(kind_of("color"), None);
        // `border-color` ends in neither "radius" nor "-width".
        assert_eq!(kind_of("border-color"), None);
    }
}
