//! Colour values as a stylesheet writes them, and the WCAG 2.x contrast between
//! two of them.
//!
//! # Why this module refuses more than it guesses
//!
//! VDS exists because a control boundary was declared aligned between the decided
//! design and what shipped, and was in fact 1.20:1 against the 3.0:1 that WCAG 2.2
//! SC 1.4.11 requires, across five themes, worst at 1.15:1. Nothing caught it,
//! because the declaration was prose and prose is not re-computed. This module is
//! the half of the instrument that reads CSS and does the arithmetic.
//!
//! An instrument that reports a wrong number is worse than one that reports none.
//! A number gets believed; a refusal gets investigated. So every path in here that
//! cannot reach the right answer returns a typed [`ColourError`] naming the reason,
//! and no path returns a plausible substitute. Three places where that costs us a
//! reading and is still the right trade:
//!
//!   - An `oklch()` outside the sRGB gamut is refused, not clamped. Clamping
//!     answers a question about a different colour, and the browser would have
//!     gamut-mapped it by an algorithm this module does not implement.
//!   - A translucent colour with no backdrop is refused, not measured as if the
//!     alpha were 1. See the alpha section below: this is the single most likely
//!     way for a contrast gate to publish a confident wrong number.
//!   - A `color-mix()` in an interpolation space this module does not implement is
//!     refused by name, not approximated in sRGB.
//!
//! # Two arithmetic decisions that change verdicts
//!
//! **Compositing happens in gamma-encoded sRGB, not in linear light.** CSS composites
//! in the compositing colour space, which for an ordinary page is non-linear sRGB, and
//! that is what a browser paints. The difference is not academic: `rgba(0,0,0,0.18)`
//! over white composites to `#d1d1d1` and measures 1.53:1 against white. Compositing
//! the same layers in linear light gives an encoded `#eaeaea` and 1.21:1. Those are
//! different verdicts wherever a floor sits between them.
//!
//! **Values are not quantised to 8 bits.** The computed value of `oklch(0.5 0.02 85)`
//! is a triple of reals; rounding it to `#686357` before measuring is an assumption
//! about the framebuffer. Measured against the five planes of the real stylesheet's
//! `.sign-layout` scope, the unquantised readings are 5.75 / 5.50 / 5.19 / 4.88 / 5.50
//! and the 8-bit-quantised ones are 5.73 / 5.49 / 5.16 / 4.88 / 5.49. A caller that
//! must reproduce a pipeline which quantises can call [`Colour::quantise_8bit`], and
//! now knows the size of the disagreement rather than discovering it as a mystery.
//!
//! # Alpha is the trap
//!
//! A translucent colour has no luminance of its own. It has one against a specific
//! backdrop, and a different one against a different backdrop. The founding defect
//! was a boundary token, and boundary tokens are very often translucent, so this is
//! the failure mode most likely to be reached in practice. The type system carries
//! the rule: [`contrast_ratio`] takes two [`OpaqueColour`] values and cannot fail,
//! and the only way to obtain an `OpaqueColour` is [`Colour::require_opaque`], which
//! refuses, or [`Colour::composite_over`], which needs a backdrop.

use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// How many levels of `var()` substitution and nested colour functions are followed
/// before the value is treated as referring to itself.
///
/// A cycle (`--a: var(--b); --b: var(--a)`) is a real thing to find in a large
/// stylesheet, and without a limit the gate hangs instead of reporting. A hang in a
/// pre-push hook reads as a broken machine, not as a finding, so the loop is named
/// and returned.
const MAX_SUBSTITUTION_DEPTH: u32 = 32;

/// Below this encoded value the sRGB transfer function is linear.
///
/// This is the sRGB specification's constant. WCAG 2.x's relative luminance
/// definition writes 0.03928, which is the same joint carried at lower precision.
/// For any 8-bit channel the two agree exactly, because no value of `n / 255` falls
/// between them: 10/255 is 0.039216 and 11/255 is 0.043137. `no_eight_bit_channel_
/// falls_between_the_two_thresholds` asserts that by enumeration, so the choice is
/// recorded as measured rather than assumed.
const SRGB_LINEAR_JOINT: f64 = 0.04045;

/// The linear-light side of the same joint, used when encoding.
const LINEAR_SRGB_JOINT: f64 = 0.003_130_8;

/// How far outside `[0, 1]` a converted linear-light channel may sit and still be
/// snapped rather than refused as out of gamut.
///
/// Chosen so that snapping cannot change a painted pixel. The steepest part of the
/// encoding curve is the linear segment near black, where a slope of 12.92 turns this
/// tolerance into 0.0013 of encoded range, and one 8-bit step is 0.0039. Everywhere
/// else the curve is flatter and the margin is wider still.
///
/// A tighter tolerance is not free. The sRGB primaries written in OKLCh at ten
/// significant figures land about 1e-5 outside the gamut, purely from the rounding of
/// the published coordinates, and refusing pure green would make the module useless
/// on the values a design system moving to OKLCh actually writes. Anything beyond
/// this tolerance is a real excursion that the browser gamut-maps by an algorithm
/// not implemented here, and is refused.
const GAMUT_EPSILON: f64 = 1e-4;

// ---------------------------------------------------------------------------
// Colour
// ---------------------------------------------------------------------------

/// A colour resolved to sRGB with an alpha channel.
///
/// The three colour components are gamma-encoded (the numbers a browser puts in the
/// framebuffer), each in `[0, 1]` and finite. [`Colour::linear`] converts to the
/// linear-light representation that luminance is defined over. Both are kept because
/// compositing is defined on the encoded values and luminance on the linear ones, and
/// a module that held only one of them would have to convert in the wrong place.
///
/// The fields are private because the invariant (finite, in range) is what lets
/// [`relative_luminance`] be total. For the same reason the type is `Serialize` but
/// not `Deserialize`: a recorded proof should be able to print a colour, but a
/// hand-edited record must not be able to inject a NaN into a measurement.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Colour {
    r: f64,
    g: f64,
    b: f64,
    alpha: f64,
}

impl Colour {
    /// The CSS `transparent` keyword's value, `rgba(0, 0, 0, 0)`.
    ///
    /// [`parse`] deliberately refuses a bare `transparent` (see
    /// [`ColourError::TransparentKeyword`]); this constant is here so a caller that
    /// genuinely wants the spec value can name it, and so `color-mix()` can use it as
    /// an operand, which is where it actually appears in real stylesheets.
    pub const TRANSPARENT: Colour = Colour {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        alpha: 0.0,
    };

    /// A colour from gamma-encoded components in `[0, 1]`.
    ///
    /// Components outside the range are clamped, which is what CSS does to
    /// out-of-range `rgb()` channels at computed-value time. Non-finite components
    /// are refused rather than clamped, because there is no colour a NaN means and
    /// `f64::clamp` would propagate it into every later measurement.
    pub fn new(r: f64, g: f64, b: f64, alpha: f64) -> Result<Self, ColourError> {
        if !(r.is_finite() && g.is_finite() && b.is_finite() && alpha.is_finite()) {
            // Named for the constructor rather than for `rgb()`, because a caller
            // building a colour by hand needs to be told where the NaN was caught, not
            // pointed at a function it never called.
            return Err(ColourError::NonFiniteComponent {
                function: "Colour::new".to_string(),
            });
        }
        Ok(Colour {
            r: r.clamp(0.0, 1.0),
            g: g.clamp(0.0, 1.0),
            b: b.clamp(0.0, 1.0),
            alpha: alpha.clamp(0.0, 1.0),
        })
    }

    /// An opaque colour from 8-bit components, the form a hex literal carries.
    #[must_use]
    pub fn from_rgb8(r: u8, g: u8, b: u8) -> Self {
        Colour {
            r: f64::from(r) / 255.0,
            g: f64::from(g) / 255.0,
            b: f64::from(b) / 255.0,
            alpha: 1.0,
        }
    }

    /// A colour from 8-bit components including alpha, the form `#rrggbbaa` carries.
    #[must_use]
    pub fn from_rgba8(r: u8, g: u8, b: u8, alpha: u8) -> Self {
        Colour {
            alpha: f64::from(alpha) / 255.0,
            ..Colour::from_rgb8(r, g, b)
        }
    }

    /// The gamma-encoded red component, in `[0, 1]`.
    #[must_use]
    pub fn red(&self) -> f64 {
        self.r
    }

    /// The gamma-encoded green component, in `[0, 1]`.
    #[must_use]
    pub fn green(&self) -> f64 {
        self.g
    }

    /// The gamma-encoded blue component, in `[0, 1]`.
    #[must_use]
    pub fn blue(&self) -> f64 {
        self.b
    }

    /// The alpha channel, in `[0, 1]`. 1 is opaque.
    #[must_use]
    pub fn alpha(&self) -> f64 {
        self.alpha
    }

    /// The four channels in `[r, g, b, alpha]` order.
    #[must_use]
    pub fn components(&self) -> [f64; 4] {
        [self.r, self.g, self.b, self.alpha]
    }

    /// Whether the colour hides whatever is behind it.
    #[must_use]
    pub fn is_opaque(&self) -> bool {
        self.alpha >= 1.0
    }

    /// The same colour with each component rounded to the nearest 1/255.
    ///
    /// Provided so a caller can reproduce a measuring pipeline that goes through an
    /// 8-bit hex representation, which the incumbent gate on the real stylesheet does:
    /// its recorded readings for the `oklch()` scope match this rounding and not the
    /// unrounded value. Alpha is left alone, because it is composited, not painted.
    #[must_use]
    pub fn quantise_8bit(&self) -> Self {
        fn q(c: f64) -> f64 {
            (c * 255.0).round() / 255.0
        }
        Colour {
            r: q(self.r),
            g: q(self.g),
            b: q(self.b),
            alpha: self.alpha,
        }
    }

    /// The colour as an opaque one, or a refusal naming the alpha.
    ///
    /// This is the single gate between "a colour was parsed" and "a ratio may be
    /// computed". Everything downstream of it is total.
    pub fn require_opaque(&self) -> Result<OpaqueColour, ColourError> {
        if self.is_opaque() {
            Ok(OpaqueColour(*self))
        } else {
            Err(ColourError::TranslucentWithoutBackdrop { alpha: self.alpha })
        }
    }

    /// The linear-light form of the three colour components.
    ///
    /// Alpha is not carried, because a linear-light triple is what luminance is
    /// defined over and luminance is only defined once a colour is opaque.
    #[must_use]
    pub fn linear(&self) -> LinearRgb {
        LinearRgb {
            r: srgb_to_linear(self.r),
            g: srgb_to_linear(self.g),
            b: srgb_to_linear(self.b),
        }
    }

    /// This colour painted over an opaque backdrop, by source-over alpha compositing.
    ///
    /// The arithmetic is done on the gamma-encoded components, which is what a browser
    /// does and is not the same answer as compositing in linear light. See the module
    /// documentation for the size of the difference.
    #[must_use]
    pub fn composite_over(&self, backdrop: &OpaqueColour) -> OpaqueColour {
        let a = self.alpha;
        let b = backdrop.0;
        OpaqueColour(Colour {
            r: self.r * a + b.r * (1.0 - a),
            g: self.g * a + b.g * (1.0 - a),
            b: self.b * a + b.b * (1.0 - a),
            alpha: 1.0,
        })
    }

    /// The WCAG 2.x relative luminance, or a refusal if the colour is translucent.
    ///
    /// The refusal is the point. A translucent colour has a different luminance over
    /// every backdrop, so returning one number for it would be returning the number
    /// for a backdrop the caller never named.
    pub fn relative_luminance(&self) -> Result<f64, ColourError> {
        Ok(self.require_opaque()?.relative_luminance())
    }

    /// The colour as a CSS hex literal, six digits when opaque and eight when not.
    ///
    /// For display and for recording in a proof. Contrast is never computed from this
    /// string, because writing it rounds to 8 bits.
    #[must_use]
    pub fn to_css_hex(&self) -> String {
        fn byte(c: f64) -> u8 {
            (c * 255.0).round().clamp(0.0, 255.0) as u8
        }
        if self.is_opaque() {
            format!(
                "#{:02x}{:02x}{:02x}",
                byte(self.r),
                byte(self.g),
                byte(self.b)
            )
        } else {
            format!(
                "#{:02x}{:02x}{:02x}{:02x}",
                byte(self.r),
                byte(self.g),
                byte(self.b),
                byte(self.alpha)
            )
        }
    }
}

impl fmt::Display for Colour {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_css_hex())
    }
}

/// A colour known to be opaque, and therefore to have a luminance of its own.
///
/// The wrapper exists so that the refusal happens once, at a named boundary, instead
/// of at every arithmetic site. A function taking `OpaqueColour` cannot be handed a
/// translucent value by accident, which is the mistake this module is most concerned
/// with preventing.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct OpaqueColour(Colour);

impl OpaqueColour {
    /// Opaque white, `#ffffff`. The commonest backdrop in a light theme.
    pub const WHITE: OpaqueColour = OpaqueColour(Colour {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        alpha: 1.0,
    });

    /// Opaque black, `#000000`.
    pub const BLACK: OpaqueColour = OpaqueColour(Colour {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        alpha: 1.0,
    });

    /// An opaque colour from 8-bit components.
    #[must_use]
    pub fn from_rgb8(r: u8, g: u8, b: u8) -> Self {
        OpaqueColour(Colour::from_rgb8(r, g, b))
    }

    /// The underlying colour, whose alpha is 1.
    #[must_use]
    pub fn colour(&self) -> Colour {
        self.0
    }

    /// The linear-light form of the colour.
    #[must_use]
    pub fn linear(&self) -> LinearRgb {
        self.0.linear()
    }

    /// The WCAG 2.x relative luminance, in `[0, 1]`.
    ///
    /// Total, because the type says the colour is opaque.
    #[must_use]
    pub fn relative_luminance(&self) -> f64 {
        self.linear().relative_luminance()
    }

    /// The colour as a six-digit CSS hex literal.
    #[must_use]
    pub fn to_css_hex(&self) -> String {
        self.0.to_css_hex()
    }
}

impl fmt::Display for OpaqueColour {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.to_css_hex())
    }
}

/// Linear-light sRGB components, the space relative luminance is defined over.
///
/// Obtained from a [`Colour`], so the components are finite and in `[0, 1]`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct LinearRgb {
    r: f64,
    g: f64,
    b: f64,
}

impl LinearRgb {
    /// The linear-light red component.
    #[must_use]
    pub fn red(&self) -> f64 {
        self.r
    }

    /// The linear-light green component.
    #[must_use]
    pub fn green(&self) -> f64 {
        self.g
    }

    /// The linear-light blue component.
    #[must_use]
    pub fn blue(&self) -> f64 {
        self.b
    }

    /// The WCAG 2.x relative luminance: `0.2126 R + 0.7152 G + 0.0722 B`.
    ///
    /// Source: WCAG 2.2, the definition of relative luminance, which is the sRGB
    /// luminance row of the sRGB-to-XYZ matrix at the precision WCAG states it.
    #[must_use]
    pub fn relative_luminance(&self) -> f64 {
        0.2126 * self.r + 0.7152 * self.g + 0.0722 * self.b
    }
}

/// The sRGB electro-optical transfer function, encoded to linear light.
///
/// The 0.04045 / 12.92 / 0.055 / 2.4 form, not a gamma-2.2 approximation. The
/// approximation is wrong by enough to move a reading across a floor: a mid grey
/// `#808080` on white is 3.95:1 by this function and 4.02:1 under gamma 2.2, so a
/// 4.0:1 floor would change verdict on the same pixel.
fn srgb_to_linear(c: f64) -> f64 {
    if c <= SRGB_LINEAR_JOINT {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// The inverse of [`srgb_to_linear`], linear light to an encoded component.
fn linear_to_srgb(c: f64) -> f64 {
    if c <= LINEAR_SRGB_JOINT {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Why a colour could not be resolved, or a ratio could not be computed.
///
/// Every variant names a distinct thing the caller can act on, because a gate that
/// reports "could not parse" tells a reader nothing about whether the stylesheet is
/// wrong or the instrument is incomplete. `Serialize` and `Deserialize` are derived
/// so a proof record can carry the reason rather than a rendered sentence.
#[derive(Debug, Clone, PartialEq, Error, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum ColourError {
    /// The value was empty or entirely whitespace.
    #[error("there is no value here to read as a colour")]
    Empty,

    /// The value is not in any syntax this module recognises.
    #[error("`{input}` is not a colour value in a syntax this module recognises")]
    UnrecognisedSyntax {
        /// The value as it was given.
        input: String,
    },

    /// A bare identifier that is not a colour keyword this module knows.
    #[error("`{keyword}` is not a colour keyword this module knows")]
    UnknownKeyword {
        /// The identifier, lowercased.
        keyword: String,
    },

    /// `currentColor`, which resolves to the inherited `color` property.
    ///
    /// It is not black. Treating it as black is how a gate reports a passing ratio
    /// for a boundary that is in fact painted in whatever the element inherited.
    #[error("`currentColor` is the inherited `color`, which is not carried in this value")]
    CurrentColor,

    /// A bare `transparent`, which has no luminance of its own.
    ///
    /// It is `rgba(0, 0, 0, 0)`, and specifically it is not black: composited over
    /// any backdrop it leaves the backdrop untouched. Refused at the top level so it
    /// can never be measured as black; resolved to its spec value inside
    /// `color-mix()`, where it is how a stylesheet says "this tint, at this alpha".
    #[error("`transparent` has no luminance of its own; it is rgba(0, 0, 0, 0), not black")]
    TransparentKeyword,

    /// A `var()` whose custom property was not among those supplied.
    #[error("custom property `{name}` was not supplied, so this value cannot be resolved")]
    UnresolvedCustomProperty {
        /// The property name, including the leading two hyphens.
        name: String,
    },

    /// A colour function this module has not implemented.
    #[error("`{function}()` is a colour function this module has not implemented")]
    UnimplementedFunction {
        /// The function name, lowercased.
        function: String,
    },

    /// A colour function whose arguments do not fit its grammar.
    #[error("`{function}()` is malformed: {detail}")]
    MalformedFunction {
        /// The function name, lowercased.
        function: String,
        /// What was wrong, in a sentence.
        detail: String,
    },

    /// Legacy comma syntax mixing numbers and percentages, which CSS forbids.
    ///
    /// The browser drops such a declaration entirely, so measuring it would be
    /// measuring a colour that is never painted.
    #[error("`{function}()` mixes numbers and percentages in the comma syntax, which CSS rejects")]
    LegacyComponentTypeMismatch {
        /// The function name, lowercased.
        function: String,
    },

    /// A `none` component, which carries no value to measure.
    #[error("`none` as a component of `{function}()` has no value to measure")]
    NoneComponent {
        /// The function name, lowercased.
        function: String,
    },

    /// A `color-mix()` interpolation space this module has not implemented.
    #[error("color-mix() in `{space}` is an interpolation space this module has not implemented")]
    UnimplementedInterpolationSpace {
        /// The space as written, including any hue interpolation method.
        space: String,
    },

    /// `color-mix()` percentages that sum to zero, which CSS makes invalid.
    #[error("color-mix() percentages sum to zero, which CSS makes invalid")]
    ZeroPercentageSum,

    /// An angle with a unit that is not one of `deg`, `grad`, `rad` or `turn`.
    #[error("`{unit}` is not an angle unit")]
    UnknownAngleUnit {
        /// The unit as written.
        unit: String,
    },

    /// A colour outside the sRGB gamut.
    ///
    /// Refused rather than clamped. The browser gamut-maps it by an algorithm this
    /// module does not implement, and a clamped value is a different colour with a
    /// different luminance.
    #[error(
        "`{input}` is outside the sRGB gamut ({channel} is {value}), and this module will not guess how it is gamut-mapped"
    )]
    OutOfSrgbGamut {
        /// The value as written.
        input: String,
        /// Which channel left the range: `red`, `green` or `blue`.
        channel: String,
        /// The linear-light value it reached.
        value: f64,
    },

    /// A luminance or a ratio was asked for on a translucent colour.
    #[error(
        "the colour is translucent (alpha {alpha}) and no backdrop was given, so it has no luminance of its own"
    )]
    TranslucentWithoutBackdrop {
        /// The alpha channel of the colour that was refused.
        alpha: f64,
    },

    /// A component that parsed as a number but is not finite.
    #[error("a component of `{function}()` is not a finite number")]
    NonFiniteComponent {
        /// The colour function it appeared in, lowercased, or the constructor that
        /// caught it.
        function: String,
    },

    /// Substitution ran deeper than [`MAX_SUBSTITUTION_DEPTH`].
    #[error("the value substitutes more than {limit} levels deep, so it refers to itself")]
    SubstitutionLoop {
        /// The depth limit that was reached.
        limit: u32,
    },
}

// ---------------------------------------------------------------------------
// Named colours
// ---------------------------------------------------------------------------

/// The sixteen HTML basic colour keywords, plus the `grey` spelling of `gray`.
///
/// Deliberately not the full 148-name CSS list. A hand-entered table of 148 triples
/// is a place for one wrong digit to hide, and a wrong digit here produces a
/// confident wrong ratio, which is the failure this module exists to avoid. An
/// unknown keyword is refused by name, so the gap is visible in a report rather than
/// silently mis-measured, and the table can grow against a cited source when a real
/// stylesheet needs it.
///
/// Source: CSS Color 3, the basic colour keywords, which are the HTML 4 sixteen.
const NAMED_COLOURS: [(&str, u8, u8, u8); 17] = [
    ("aqua", 0x00, 0xff, 0xff),
    ("black", 0x00, 0x00, 0x00),
    ("blue", 0x00, 0x00, 0xff),
    ("fuchsia", 0xff, 0x00, 0xff),
    ("gray", 0x80, 0x80, 0x80),
    ("green", 0x00, 0x80, 0x00),
    ("grey", 0x80, 0x80, 0x80),
    ("lime", 0x00, 0xff, 0x00),
    ("maroon", 0x80, 0x00, 0x00),
    ("navy", 0x00, 0x00, 0x80),
    ("olive", 0x80, 0x80, 0x00),
    ("purple", 0x80, 0x00, 0x80),
    ("red", 0xff, 0x00, 0x00),
    ("silver", 0xc0, 0xc0, 0xc0),
    ("teal", 0x00, 0x80, 0x80),
    ("white", 0xff, 0xff, 0xff),
    ("yellow", 0xff, 0xff, 0x00),
];

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Whether a bare `transparent` keyword is a value or a refusal in this position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransparentPolicy {
    /// Top level: refuse, so it can never be measured as if it were black.
    Refuse,
    /// Inside `color-mix()`: resolve to `rgba(0, 0, 0, 0)`, which is what CSS mixes.
    Resolve,
}

/// Parse a CSS colour value.
///
/// Recognises `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, `rgb()` and `rgba()` in both
/// the comma and the space syntax, `hsl()` and `hsla()` likewise, `oklch()`,
/// `color-mix()` in `srgb`, `srgb-linear` and `oklab`, and the sixteen basic colour
/// keywords.
///
/// A `var()` is refused by name, because this function is given no custom properties
/// to resolve it against. Use [`parse_with`] where the cascade is known.
///
/// # Errors
///
/// Returns the [`ColourError`] naming what could not be resolved. `currentColor` and
/// a bare `transparent` are refusals, not colours.
pub fn parse(input: &str) -> Result<Colour, ColourError> {
    parse_with(input, &|_| None)
}

/// Parse a CSS colour value, resolving `var()` against supplied custom properties.
///
/// `lookup` is given a property name including its leading two hyphens, and returns
/// its declared value if the cascade defines one. Substitution is textual and happens
/// before parsing, which is how CSS itself works, so `rgb(var(--channels) / 0.5)`
/// resolves when `--channels` is `0 135 234`. A `var()` with a fallback uses the
/// fallback only when `lookup` returns `None`, which is the CSS rule: a fallback is
/// for an undefined property, not for a property that resolves to something unusable.
///
/// # Errors
///
/// Returns the [`ColourError`] naming what could not be resolved, including
/// [`ColourError::SubstitutionLoop`] when the properties refer to each other.
pub fn parse_with(
    input: &str,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Result<Colour, ColourError> {
    let substituted = substitute_vars(input, lookup, 0)?;
    parse_resolved(&substituted, TransparentPolicy::Refuse, 0)
}

/// Replace every `var()` in the value with the text it resolves to.
///
/// Textual, and applied at any nesting depth, because that is what CSS does: `var()`
/// substitution happens on the token stream before the value is interpreted. Doing it
/// only at the top level would fail on `rgb(var(--channels) / 0.5)`, which is a common
/// way to write a translucent brand colour and therefore a common way for a boundary
/// token to be translucent.
fn substitute_vars(
    input: &str,
    lookup: &dyn Fn(&str) -> Option<String>,
    depth: u32,
) -> Result<String, ColourError> {
    if depth > MAX_SUBSTITUTION_DEPTH {
        return Err(ColourError::SubstitutionLoop {
            limit: MAX_SUBSTITUTION_DEPTH,
        });
    }
    if !contains_var(input) {
        return Ok(input.to_string());
    }

    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut i = 0usize;
    while i < chars.len() {
        if starts_var_at(&chars, i) {
            let Some(close) = matching_paren(&chars, i + 3) else {
                return Err(ColourError::MalformedFunction {
                    function: "var".to_string(),
                    detail: "the argument list is not closed".to_string(),
                });
            };
            let inner: String = chars[i + 4..close].iter().collect();
            out.push_str(&resolve_var(&inner, lookup, depth)?);
            i = close + 1;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    Ok(out)
}

/// Whether the value mentions `var(` at all, so the common case skips the scan.
fn contains_var(input: &str) -> bool {
    let lowered = input.to_ascii_lowercase();
    lowered.contains("var(")
}

/// Whether a `var(` token starts at `i`, and is a token rather than the tail of a name.
fn starts_var_at(chars: &[char], i: usize) -> bool {
    if i + 4 > chars.len() {
        return false;
    }
    let is_var = chars[i].eq_ignore_ascii_case(&'v')
        && chars[i + 1].eq_ignore_ascii_case(&'a')
        && chars[i + 2].eq_ignore_ascii_case(&'r')
        && chars[i + 3] == '(';
    if !is_var {
        return false;
    }
    // `--myvar(` is not a var() call. Requiring the preceding character to be a
    // non-identifier one keeps the substitution from eating part of another token.
    i == 0 || !(chars[i - 1].is_ascii_alphanumeric() || chars[i - 1] == '-' || chars[i - 1] == '_')
}

/// The index of the `)` closing the `(` at `open`.
fn matching_paren(chars: &[char], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (offset, ch) in chars[open..].iter().enumerate() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + offset);
                }
            }
            _ => {}
        }
    }
    None
}

/// Resolve the inside of one `var()`, following its fallback where there is one.
fn resolve_var(
    inner: &str,
    lookup: &dyn Fn(&str) -> Option<String>,
    depth: u32,
) -> Result<String, ColourError> {
    let parts = split_top_level(inner, ',');
    let name = parts[0].trim();
    if !name.starts_with("--") {
        return Err(ColourError::MalformedFunction {
            function: "var".to_string(),
            detail: format!("`{name}` is not a custom property name"),
        });
    }
    if let Some(value) = lookup(name) {
        return substitute_vars(&value, lookup, depth + 1);
    }
    if parts.len() > 1 {
        let fallback = parts[1..].join(",");
        return substitute_vars(fallback.trim(), lookup, depth + 1);
    }
    Err(ColourError::UnresolvedCustomProperty {
        name: name.to_string(),
    })
}

/// Parse a value in which every `var()` has already been substituted.
fn parse_resolved(
    input: &str,
    transparent: TransparentPolicy,
    depth: u32,
) -> Result<Colour, ColourError> {
    if depth > MAX_SUBSTITUTION_DEPTH {
        return Err(ColourError::SubstitutionLoop {
            limit: MAX_SUBSTITUTION_DEPTH,
        });
    }
    let value = input.trim();
    if value.is_empty() {
        return Err(ColourError::Empty);
    }
    if let Some(hex) = value.strip_prefix('#') {
        return parse_hex(hex, value);
    }
    if let Some((name, inner)) = as_function(value) {
        return match name.as_str() {
            "rgb" | "rgba" => parse_rgb(inner, &name),
            "hsl" | "hsla" => parse_hsl(inner, &name),
            "oklch" => parse_oklch(inner, value),
            "color-mix" => parse_color_mix(inner, depth),
            "var" => Err(ColourError::UnresolvedCustomProperty {
                name: split_top_level(inner, ',')[0].trim().to_string(),
            }),
            _ => Err(ColourError::UnimplementedFunction { function: name }),
        };
    }
    if value.contains('(') || value.contains(')') {
        return Err(ColourError::UnrecognisedSyntax {
            input: value.to_string(),
        });
    }
    parse_keyword(value, transparent)
}

/// Resolve a bare identifier.
fn parse_keyword(value: &str, transparent: TransparentPolicy) -> Result<Colour, ColourError> {
    let keyword = value.to_ascii_lowercase();
    if keyword.contains(char::is_whitespace) {
        return Err(ColourError::UnrecognisedSyntax {
            input: value.to_string(),
        });
    }
    if keyword == "currentcolor" {
        return Err(ColourError::CurrentColor);
    }
    if keyword == "transparent" {
        return match transparent {
            TransparentPolicy::Refuse => Err(ColourError::TransparentKeyword),
            TransparentPolicy::Resolve => Ok(Colour::TRANSPARENT),
        };
    }
    NAMED_COLOURS
        .iter()
        .find(|(name, _, _, _)| *name == keyword)
        .map(|&(_, r, g, b)| Colour::from_rgb8(r, g, b))
        .ok_or(ColourError::UnknownKeyword { keyword })
}

/// Parse the digits after a `#`.
fn parse_hex(digits: &str, whole: &str) -> Result<Colour, ColourError> {
    let malformed = || ColourError::UnrecognisedSyntax {
        input: whole.to_string(),
    };
    if !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(malformed());
    }
    let nibble = |c: char| -> u8 { c.to_digit(16).unwrap_or(0) as u8 };
    let d: Vec<char> = digits.chars().collect();
    match d.len() {
        // Each digit is doubled, not shifted: `#abc` is `#aabbcc`, so `#fff` is white
        // and not `#f0f0f0`. Shifting would darken every short-form colour slightly.
        3 => Ok(Colour::from_rgb8(
            nibble(d[0]) * 17,
            nibble(d[1]) * 17,
            nibble(d[2]) * 17,
        )),
        4 => Ok(Colour::from_rgba8(
            nibble(d[0]) * 17,
            nibble(d[1]) * 17,
            nibble(d[2]) * 17,
            nibble(d[3]) * 17,
        )),
        6 => Ok(Colour::from_rgb8(
            nibble(d[0]) * 16 + nibble(d[1]),
            nibble(d[2]) * 16 + nibble(d[3]),
            nibble(d[4]) * 16 + nibble(d[5]),
        )),
        8 => Ok(Colour::from_rgba8(
            nibble(d[0]) * 16 + nibble(d[1]),
            nibble(d[2]) * 16 + nibble(d[3]),
            nibble(d[4]) * 16 + nibble(d[5]),
            nibble(d[6]) * 16 + nibble(d[7]),
        )),
        _ => Err(malformed()),
    }
}

/// A component that may have been written as a number or as a percentage.
#[derive(Debug, Clone, Copy)]
enum Component {
    Number(f64),
    Percentage(f64),
}

/// Split a function's arguments into colour components and an optional alpha.
struct Arguments {
    components: Vec<String>,
    alpha: Option<String>,
    /// Set when the comma syntax was used, which restricts component types.
    legacy: bool,
}

/// Read a function's argument list in either the comma or the space syntax.
fn read_arguments(inner: &str, function: &str, arity: usize) -> Result<Arguments, ColourError> {
    let malformed = |detail: String| ColourError::MalformedFunction {
        function: function.to_string(),
        detail,
    };
    let commas = split_top_level(inner, ',');
    if commas.len() > 1 {
        if inner.contains('/') {
            return Err(malformed(
                "the comma syntax does not take a slash before alpha".to_string(),
            ));
        }
        if commas.len() != arity && commas.len() != arity + 1 {
            return Err(malformed(format!(
                "expected {arity} components and an optional alpha, found {}",
                commas.len()
            )));
        }
        return Ok(Arguments {
            components: commas[..arity]
                .iter()
                .map(|s| s.trim().to_string())
                .collect(),
            alpha: commas.get(arity).map(|s| s.trim().to_string()),
            legacy: true,
        });
    }

    let slashed = split_top_level(inner, '/');
    if slashed.len() > 2 {
        return Err(malformed("more than one slash".to_string()));
    }
    let components: Vec<String> = split_top_level_whitespace(slashed[0])
        .into_iter()
        .map(str::to_string)
        .collect();
    if components.len() != arity {
        return Err(malformed(format!(
            "expected {arity} components, found {}",
            components.len()
        )));
    }
    let alpha = match slashed.get(1) {
        Some(a) if a.trim().is_empty() => {
            return Err(malformed(
                "the slash is not followed by an alpha".to_string(),
            ));
        }
        Some(a) => Some(a.trim().to_string()),
        None => None,
    };
    Ok(Arguments {
        components,
        alpha,
        legacy: false,
    })
}

/// Read one number or percentage, refusing `none` and anything non-finite.
fn read_component(token: &str, function: &str) -> Result<Component, ColourError> {
    let text = token.trim();
    if text.eq_ignore_ascii_case("none") {
        return Err(ColourError::NoneComponent {
            function: function.to_string(),
        });
    }
    let (body, is_percentage) = match text.strip_suffix('%') {
        Some(rest) => (rest, true),
        None => (text, false),
    };
    let value: f64 = body.parse().map_err(|_| ColourError::MalformedFunction {
        function: function.to_string(),
        detail: format!("`{text}` is not a number"),
    })?;
    // `"inf"` and `"NaN"` parse as f64. Left unchecked they would travel all the way
    // into a luminance and come back out as a NaN ratio, which compares false against
    // every floor and so reports a failing boundary as passing.
    if !value.is_finite() {
        return Err(ColourError::NonFiniteComponent {
            function: function.to_string(),
        });
    }
    Ok(if is_percentage {
        Component::Percentage(value)
    } else {
        Component::Number(value)
    })
}

/// Read an alpha, which may be a number in `[0, 1]` or a percentage.
fn read_alpha(token: Option<&String>, function: &str) -> Result<f64, ColourError> {
    let Some(token) = token else {
        return Ok(1.0);
    };
    Ok(match read_component(token, function)? {
        Component::Number(n) => n.clamp(0.0, 1.0),
        Component::Percentage(p) => (p / 100.0).clamp(0.0, 1.0),
    })
}

/// `rgb()` and `rgba()`, in both syntaxes.
fn parse_rgb(inner: &str, function: &str) -> Result<Colour, ColourError> {
    let args = read_arguments(inner, function, 3)?;
    let mut channels = [0.0f64; 3];
    let mut percentages = 0usize;
    for (index, token) in args.components.iter().enumerate() {
        channels[index] = match read_component(token, function)? {
            Component::Number(n) => (n / 255.0).clamp(0.0, 1.0),
            Component::Percentage(p) => {
                percentages += 1;
                (p / 100.0).clamp(0.0, 1.0)
            }
        };
    }
    // CSS allows the space syntax to mix the two types and the comma syntax not to.
    // A mixed comma form makes the whole declaration invalid, so the browser paints
    // something else entirely and a ratio computed here would describe a colour that
    // never reaches a screen.
    if args.legacy && percentages != 0 && percentages != 3 {
        return Err(ColourError::LegacyComponentTypeMismatch {
            function: function.to_string(),
        });
    }
    let alpha = read_alpha(args.alpha.as_ref(), function)?;
    Colour::new(channels[0], channels[1], channels[2], alpha)
}

/// `hsl()` and `hsla()`, in both syntaxes.
fn parse_hsl(inner: &str, function: &str) -> Result<Colour, ColourError> {
    let args = read_arguments(inner, function, 3)?;
    let hue = read_angle_degrees(&args.components[0], function)?;
    let saturation = match read_component(&args.components[1], function)? {
        Component::Number(n) => n / 100.0,
        Component::Percentage(p) => p / 100.0,
    }
    .clamp(0.0, 1.0);
    let lightness = match read_component(&args.components[2], function)? {
        Component::Number(n) => n / 100.0,
        Component::Percentage(p) => p / 100.0,
    }
    .clamp(0.0, 1.0);
    let alpha = read_alpha(args.alpha.as_ref(), function)?;
    let (r, g, b) = hsl_to_srgb(hue, saturation, lightness);
    Colour::new(r, g, b, alpha)
}

/// The CSS Color 4 reference conversion from HSL to gamma-encoded sRGB.
fn hsl_to_srgb(hue: f64, saturation: f64, lightness: f64) -> (f64, f64, f64) {
    let h = hue.rem_euclid(360.0);
    let a = saturation * lightness.min(1.0 - lightness);
    let f = |n: f64| -> f64 {
        let k = (n + h / 30.0).rem_euclid(12.0);
        lightness - a * (k - 3.0).min(9.0 - k).clamp(-1.0, 1.0)
    };
    (f(0.0), f(8.0), f(4.0))
}

/// Read an angle, in degrees, gradians, radians or turns.
fn read_angle_degrees(token: &str, function: &str) -> Result<f64, ColourError> {
    let text = token.trim();
    if text.eq_ignore_ascii_case("none") {
        return Err(ColourError::NoneComponent {
            function: function.to_string(),
        });
    }
    let split = text
        .find(|c: char| c.is_ascii_alphabetic())
        .unwrap_or(text.len());
    let (number, unit) = text.split_at(split);
    let value: f64 = number.parse().map_err(|_| ColourError::MalformedFunction {
        function: function.to_string(),
        detail: format!("`{text}` is not an angle"),
    })?;
    if !value.is_finite() {
        return Err(ColourError::NonFiniteComponent {
            function: function.to_string(),
        });
    }
    match unit.to_ascii_lowercase().as_str() {
        "" | "deg" => Ok(value),
        "grad" => Ok(value * 360.0 / 400.0),
        "rad" => Ok(value.to_degrees()),
        "turn" => Ok(value * 360.0),
        other => Err(ColourError::UnknownAngleUnit {
            unit: other.to_string(),
        }),
    }
}

/// `oklch()`.
///
/// Lightness is a number in `[0, 1]` or a percentage of it; chroma is a number, or a
/// percentage of the 0.4 reference the specification names; hue is an angle.
fn parse_oklch(inner: &str, whole: &str) -> Result<Colour, ColourError> {
    let function = "oklch";
    let args = read_arguments(inner, function, 3)?;
    let lightness = match read_component(&args.components[0], function)? {
        Component::Number(n) => n,
        Component::Percentage(p) => p / 100.0,
    }
    .clamp(0.0, 1.0);
    let chroma = match read_component(&args.components[1], function)? {
        Component::Number(n) => n,
        // CSS Color 4 fixes 100% of oklch chroma at 0.4.
        Component::Percentage(p) => p / 100.0 * 0.4,
    }
    .max(0.0);
    let hue = read_angle_degrees(&args.components[2], function)?;
    let alpha = read_alpha(args.alpha.as_ref(), function)?;

    let (a, b) = (
        chroma * hue.to_radians().cos(),
        chroma * hue.to_radians().sin(),
    );
    let linear = oklab_to_linear_srgb(lightness, a, b);
    let encoded = encode_in_gamut(linear, whole)?;
    Colour::new(encoded[0], encoded[1], encoded[2], alpha)
}

/// OKLab to linear-light sRGB.
///
/// The two published matrices, applied in order: OKLab to the cube roots of the LMS
/// cone responses, then the cubed responses to linear sRGB.
///
/// Source: CSS Color 4, the OKLab to sRGB conversion, which carries Björn Ottosson's
/// coefficients. Cross-checked against the subject stylesheet, whose own comment
/// records `oklch(0.82 0.008 85)` as `#c6c4be`; this implementation agrees to the
/// byte, which is asserted in `oklch_matches_the_hex_the_real_stylesheet_records`.
fn oklab_to_linear_srgb(l_star: f64, a: f64, b: f64) -> [f64; 3] {
    let l_ = l_star + 0.3963377774 * a + 0.2158037573 * b;
    let m_ = l_star - 0.1055613458 * a - 0.0638541728 * b;
    let s_ = l_star - 0.0894841775 * a - 1.2914855480 * b;

    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    [
        4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
        -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
        -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s,
    ]
}

/// Linear-light sRGB to OKLab, the inverse of [`oklab_to_linear_srgb`].
fn linear_srgb_to_oklab(rgb: [f64; 3]) -> [f64; 3] {
    let l = 0.4122214708 * rgb[0] + 0.5363325363 * rgb[1] + 0.0514459929 * rgb[2];
    let m = 0.2119034982 * rgb[0] + 0.6806995451 * rgb[1] + 0.1073969566 * rgb[2];
    let s = 0.0883024619 * rgb[0] + 0.2817188376 * rgb[1] + 0.6299787005 * rgb[2];

    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();

    [
        0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_,
        1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_,
        0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_,
    ]
}

/// Encode linear-light components, refusing anything outside the sRGB gamut.
///
/// Noise of a part in a million is snapped, because `oklch(1 0 0)` lands a hair above
/// 1.0 and refusing pure white would make the module useless. Anything further out is
/// a real out-of-gamut colour and is named, because the browser will gamut-map it by
/// an algorithm not implemented here and the mapped colour has a different luminance.
fn encode_in_gamut(linear: [f64; 3], whole: &str) -> Result<[f64; 3], ColourError> {
    const CHANNELS: [&str; 3] = ["red", "green", "blue"];
    let mut encoded = [0.0f64; 3];
    for (index, value) in linear.iter().enumerate() {
        if !value.is_finite() || *value < -GAMUT_EPSILON || *value > 1.0 + GAMUT_EPSILON {
            return Err(ColourError::OutOfSrgbGamut {
                input: whole.to_string(),
                channel: CHANNELS[index].to_string(),
                value: *value,
            });
        }
        encoded[index] = linear_to_srgb(value.clamp(0.0, 1.0));
    }
    Ok(encoded)
}

/// An interpolation space `color-mix()` may be asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MixSpace {
    /// Gamma-encoded sRGB, which is what all 59 uses in the subject stylesheet ask for.
    Srgb,
    /// Linear-light sRGB.
    SrgbLinear,
    /// OKLab, the rectangular form of OKLCh.
    Oklab,
}

/// `color-mix()`.
///
/// # Percentage normalisation
///
/// Implemented from CSS Color 5, the `color-mix()` percentage rules, in this order:
///
///   1. Both percentages omitted: both are 50%.
///   2. One omitted: it becomes 100% minus the other.
///   3. Each is clamped to `[0, 100]`.
///   4. A sum of zero makes the function invalid.
///   5. A sum other than 100% is scaled to 100%, and where the sum was **less** than
///      100% the result's alpha is multiplied by that sum as a fraction. That last
///      clause is the one an implementation invents wrongly if it guesses:
///      `color-mix(in srgb, red 30%, blue 30%)` is a half-and-half mix at alpha 0.6,
///      not an opaque one.
///
/// # Interpolation
///
/// Colours are interpolated with **premultiplied** alpha, per CSS Color 4. This is
/// not a detail. `color-mix(in srgb, red 6%, transparent)` is red at alpha 0.06;
/// interpolated without premultiplying it would be a near-black at alpha 0.06, and
/// over white those two composite to `#fff0f0` and `#f1f1f1`, which are a different
/// hue and a different luminance. Forty-five of the subject stylesheet's fifty-nine
/// mixes have `transparent` as an operand, so this is the common case, not the corner.
fn parse_color_mix(inner: &str, depth: u32) -> Result<Colour, ColourError> {
    let function = "color-mix";
    let malformed = |detail: String| ColourError::MalformedFunction {
        function: function.to_string(),
        detail,
    };
    let parts = split_top_level(inner, ',');
    if parts.len() != 3 {
        return Err(malformed(format!(
            "expected an interpolation space and two colours, found {} arguments",
            parts.len()
        )));
    }

    // Split rather than strip a `"in "` prefix, because CSS keywords are ASCII
    // case-insensitive and `In srgb` is as valid as `in srgb`.
    let space_text = parts[0].trim();
    let space_tokens = split_top_level_whitespace(space_text);
    let names_a_space = space_tokens
        .first()
        .is_some_and(|first| first.eq_ignore_ascii_case("in"));
    if !names_a_space || space_tokens.len() < 2 {
        return Err(malformed(format!(
            "`{space_text}` does not name an interpolation space"
        )));
    }
    let space_name = space_tokens[1..].join(" ").to_ascii_lowercase();
    // A hue interpolation method (`in oklch shorter hue`) only applies to polar
    // spaces, none of which are implemented, so any extra token lands in the same
    // refusal and is named in full rather than silently ignored.
    let space = match space_name.as_str() {
        "srgb" => MixSpace::Srgb,
        "srgb-linear" => MixSpace::SrgbLinear,
        "oklab" => MixSpace::Oklab,
        _ => {
            return Err(ColourError::UnimplementedInterpolationSpace { space: space_name });
        }
    };

    let (colour1, given1) = read_mix_operand(parts[1], depth)?;
    let (colour2, given2) = read_mix_operand(parts[2], depth)?;

    let (mut p1, mut p2) = match (given1, given2) {
        (None, None) => (50.0, 50.0),
        (Some(p), None) => (p.clamp(0.0, 100.0), (100.0 - p.clamp(0.0, 100.0)).max(0.0)),
        (None, Some(p)) => ((100.0 - p.clamp(0.0, 100.0)).max(0.0), p.clamp(0.0, 100.0)),
        (Some(a), Some(b)) => (a.clamp(0.0, 100.0), b.clamp(0.0, 100.0)),
    };
    let sum = p1 + p2;
    if sum <= 0.0 {
        return Err(ColourError::ZeroPercentageSum);
    }
    let alpha_multiplier = if sum < 100.0 { sum / 100.0 } else { 1.0 };
    p1 /= sum;
    p2 /= sum;

    mix(space, colour1, p1, colour2, p2, alpha_multiplier, inner)
}

/// Read one `color-mix()` operand, which is a colour with an optional percentage.
///
/// CSS Color 5 allows the percentage on either side of the colour, so both orders are
/// accepted rather than assuming the common one.
fn read_mix_operand(part: &str, depth: u32) -> Result<(Colour, Option<f64>), ColourError> {
    let function = "color-mix";
    let tokens = split_top_level_whitespace(part);
    if tokens.is_empty() {
        return Err(ColourError::MalformedFunction {
            function: function.to_string(),
            detail: "an operand is empty".to_string(),
        });
    }
    let mut percentage: Option<f64> = None;
    let mut colour_tokens: Vec<&str> = Vec::with_capacity(tokens.len());
    for token in tokens {
        if let Some(body) = token.strip_suffix('%')
            && let Ok(value) = body.parse::<f64>()
        {
            if !value.is_finite() {
                return Err(ColourError::NonFiniteComponent {
                    function: function.to_string(),
                });
            }
            if percentage.is_some() {
                return Err(ColourError::MalformedFunction {
                    function: function.to_string(),
                    detail: format!("`{}` carries two percentages", part.trim()),
                });
            }
            percentage = Some(value);
            continue;
        }
        colour_tokens.push(token);
    }
    if colour_tokens.is_empty() {
        return Err(ColourError::MalformedFunction {
            function: function.to_string(),
            detail: format!("`{}` has a percentage but no colour", part.trim()),
        });
    }
    let colour = parse_resolved(
        &colour_tokens.join(" "),
        TransparentPolicy::Resolve,
        depth + 1,
    )?;
    Ok((colour, percentage))
}

/// Interpolate two colours in a space, with premultiplied alpha.
fn mix(
    space: MixSpace,
    c1: Colour,
    p1: f64,
    c2: Colour,
    p2: f64,
    alpha_multiplier: f64,
    whole: &str,
) -> Result<Colour, ColourError> {
    let coords1 = coordinates(space, &c1);
    let coords2 = coordinates(space, &c2);

    let mut premultiplied = [0.0f64; 3];
    for index in 0..3 {
        premultiplied[index] = coords1[index] * c1.alpha * p1 + coords2[index] * c2.alpha * p2;
    }
    let alpha = c1.alpha * p1 + c2.alpha * p2;

    let mixed = if alpha > 0.0 {
        [
            premultiplied[0] / alpha,
            premultiplied[1] / alpha,
            premultiplied[2] / alpha,
        ]
    } else {
        // A fully transparent result has no colour to un-premultiply. Whatever is put
        // here contributes nothing to any composite, because the alpha is zero.
        [0.0, 0.0, 0.0]
    };

    let encoded = match space {
        MixSpace::Srgb => mixed,
        MixSpace::SrgbLinear => [
            linear_to_srgb(mixed[0].clamp(0.0, 1.0)),
            linear_to_srgb(mixed[1].clamp(0.0, 1.0)),
            linear_to_srgb(mixed[2].clamp(0.0, 1.0)),
        ],
        // The sRGB gamut is not convex in OKLab, so a mix of two in-gamut colours can
        // land outside it. That is refused by name rather than clamped, for the same
        // reason an out-of-gamut oklch() is.
        MixSpace::Oklab => encode_in_gamut(
            oklab_to_linear_srgb(mixed[0], mixed[1], mixed[2]),
            &format!("color-mix({whole})"),
        )?,
    };

    Colour::new(encoded[0], encoded[1], encoded[2], alpha * alpha_multiplier)
}

/// A colour's coordinates in an interpolation space.
fn coordinates(space: MixSpace, colour: &Colour) -> [f64; 3] {
    match space {
        MixSpace::Srgb => [colour.r, colour.g, colour.b],
        MixSpace::SrgbLinear => {
            let linear = colour.linear();
            [linear.r, linear.g, linear.b]
        }
        MixSpace::Oklab => {
            let linear = colour.linear();
            linear_srgb_to_oklab([linear.r, linear.g, linear.b])
        }
    }
}

/// Split on a separator that is not inside parentheses.
fn split_top_level(input: &str, separator: char) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (index, ch) in input.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            c if c == separator && depth == 0 => {
                out.push(&input[start..index]);
                start = index + c.len_utf8();
            }
            _ => {}
        }
    }
    out.push(&input[start..]);
    out
}

/// Split on whitespace that is not inside parentheses.
///
/// Nested functions stay whole, so `rgb(0 1 2)` is one token inside a `color-mix()`
/// operand rather than three.
fn split_top_level_whitespace(input: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start: Option<usize> = None;
    for (index, ch) in input.char_indices() {
        match ch {
            '(' => {
                start.get_or_insert(index);
                depth += 1;
            }
            ')' => {
                start.get_or_insert(index);
                depth = depth.saturating_sub(1);
            }
            c if c.is_whitespace() && depth == 0 => {
                if let Some(from) = start.take() {
                    out.push(&input[from..index]);
                }
            }
            _ => {
                start.get_or_insert(index);
            }
        }
    }
    if let Some(from) = start {
        out.push(&input[from..]);
    }
    out
}

/// Read `name(inner)`, where the closing parenthesis is the one that opened it.
fn as_function(input: &str) -> Option<(String, &str)> {
    let trimmed = input.trim();
    if !trimmed.ends_with(')') {
        return None;
    }
    let open = trimmed.find('(')?;
    let name = &trimmed[..open];
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return None;
    }
    let inner = &trimmed[open + 1..trimmed.len() - 1];
    let mut depth = 0i32;
    for ch in inner.chars() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return None;
                }
            }
            _ => {}
        }
    }
    if depth != 0 {
        return None;
    }
    Some((name.to_ascii_lowercase(), inner))
}

// ---------------------------------------------------------------------------
// Contrast
// ---------------------------------------------------------------------------

/// The WCAG 2.x relative luminance of an opaque colour.
///
/// Source: WCAG 2.2, the definition of relative luminance.
#[must_use]
pub fn relative_luminance(colour: &OpaqueColour) -> f64 {
    colour.relative_luminance()
}

/// The WCAG 2.x contrast ratio between two opaque colours: `(L1 + 0.05) / (L2 + 0.05)`
/// with `L1` the lighter.
///
/// Total, and deliberately so: both arguments are opaque by construction, so there is
/// no failure left to report and no temptation to invent one.
///
/// Source: WCAG 2.2, the definition of contrast ratio.
#[must_use]
pub fn contrast_ratio(a: &OpaqueColour, b: &OpaqueColour) -> f64 {
    let la = a.relative_luminance();
    let lb = b.relative_luminance();
    let (lighter, darker) = if la >= lb { (la, lb) } else { (lb, la) };
    (lighter + 0.05) / (darker + 0.05)
}

/// The contrast ratio between two colours, refusing if either is translucent.
///
/// # Errors
///
/// [`ColourError::TranslucentWithoutBackdrop`], naming the alpha of the colour that
/// has no luminance of its own. Use [`contrast_ratio_over`] when the backdrop is known.
pub fn contrast_ratio_of(a: &Colour, b: &Colour) -> Result<f64, ColourError> {
    Ok(contrast_ratio(&a.require_opaque()?, &b.require_opaque()?))
}

/// The contrast ratio between two colours once both are painted over a backdrop.
///
/// This is the measurement a translucent boundary token actually needs: a 12%-alpha
/// rule over a card over the page has a ratio, but only once the stack is named.
#[must_use]
pub fn contrast_ratio_over(
    foreground: &Colour,
    background: &Colour,
    backdrop: &OpaqueColour,
) -> f64 {
    let painted_background = background.composite_over(backdrop);
    let painted_foreground = foreground.composite_over(&painted_background);
    contrast_ratio(&painted_foreground, &painted_background)
}

/// Paint a stack of layers over a backdrop, bottom layer first.
///
/// Folding [`Colour::composite_over`] by hand is easy to get backwards, and getting it
/// backwards changes the answer whenever two layers have different alphas.
#[must_use]
pub fn composite_stack(layers: &[Colour], backdrop: &OpaqueColour) -> OpaqueColour {
    layers
        .iter()
        .fold(*backdrop, |beneath, layer| layer.composite_over(&beneath))
}

/// Whether a ratio meets a floor.
///
/// Compares the unrounded value, and with no tolerance. Both halves of that matter.
/// A gate that rounds first passes 2.996:1 against a 3.0:1 floor, which is exactly the
/// class of near-miss the floor exists to catch. A gate with a tolerance passes it on
/// purpose. Floating-point noise can therefore fail a boundary that is exactly on the
/// floor; that direction is a false alarm, which is investigated, rather than a false
/// clearance, which is not.
#[must_use]
pub fn meets_floor(ratio: f64, floor: f64) -> bool {
    ratio >= floor
}

/// A ratio rendered for a human, to two decimal places.
///
/// Truncated rather than rounded, so a displayed `3.00` is never a 2.999 that was
/// dressed up. Display only: [`meets_floor`] never sees this string.
#[must_use]
pub fn format_ratio(ratio: f64) -> String {
    let truncated = (ratio * 100.0).floor() / 100.0;
    format!("{truncated:.2}")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Ratios are compared to two decimal places unless a test says otherwise,
    /// because that is the precision every published reading in this domain is
    /// quoted at.
    fn assert_close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "expected {expected}, got {actual}"
        );
    }

    fn parsed(input: &str) -> Colour {
        parse(input).unwrap_or_else(|e| panic!("`{input}` should parse, got {e}"))
    }

    fn opaque(input: &str) -> OpaqueColour {
        parsed(input).require_opaque().expect("should be opaque")
    }

    fn ratio(a: &str, b: &str) -> f64 {
        contrast_ratio(&opaque(a), &opaque(b))
    }

    /// A ratio rounded to two places, for comparing against a reading somebody else
    /// published.
    ///
    /// Deliberately not [`format_ratio`], which truncates: a gate must not round a
    /// near-miss up to its floor, but a published reading was rounded by whoever
    /// published it, and comparing a truncation against a rounding manufactures a
    /// disagreement in the last digit that is not a disagreement about the colour.
    fn rounded(ratio: f64) -> String {
        format!("{ratio:.2}")
    }

    // -- WCAG known answers -------------------------------------------------

    /// WCAG 2.2, the definition of contrast ratio: the range is 1 to 21, and 21 is
    /// black against white. Asserted exactly, not approximately, because every
    /// implementation of this formula is expected to reproduce the endpoint.
    #[test]
    fn black_on_white_is_exactly_twenty_one_to_one() {
        assert_eq!(ratio("#000000", "#ffffff"), 21.0);
        assert_eq!(ratio("black", "white"), 21.0);
    }

    #[test]
    fn a_colour_against_itself_is_exactly_one_to_one() {
        assert_eq!(ratio("#ffffff", "#ffffff"), 1.0);
        assert_eq!(ratio("#0f172a", "#0f172a"), 1.0);
    }

    /// Worked values every contrast tool in the field agrees on, and which a wrong
    /// transfer function misses: `#767676` is the classic grey that just clears the
    /// 4.5:1 floor on white, `#777777` the one just above it, `#808080` the mid grey
    /// that clears 3:1 and not 4.5:1.
    #[test]
    fn published_grey_on_white_readings() {
        assert_close(ratio("#767676", "#ffffff"), 4.5422, 0.0001);
        assert_close(ratio("#777777", "#ffffff"), 4.4781, 0.0001);
        assert_close(ratio("#808080", "#ffffff"), 3.9494, 0.0001);
        assert!(meets_floor(ratio("#767676", "#ffffff"), 4.5));
        assert!(!meets_floor(ratio("#777777", "#ffffff"), 4.5));
    }

    /// The 0.04045 the sRGB specification states and the 0.03928 WCAG 2.x writes are
    /// the same joint at different precision. For 8-bit input they cannot disagree,
    /// because no channel value lands between them. Asserted by enumeration rather
    /// than by eye, since the whole point is that nobody has to trust the claim.
    #[test]
    fn no_eight_bit_channel_falls_between_the_two_thresholds() {
        for value in 0u16..=255 {
            let c = f64::from(value) / 255.0;
            assert_eq!(
                c <= 0.03928,
                c <= SRGB_LINEAR_JOINT,
                "channel {value} straddles the two thresholds"
            );
        }
    }

    /// A gamma-2.2 approximation is close enough to look right and far enough to
    /// change a verdict, on a real token, at the real floor.
    ///
    /// `--dashboard-select: #0087ea` on `--soft-3: #e2e8f0` is the tightest reading in
    /// the subject stylesheet's light theme: its own comment records 3.01:1 and says
    /// the value was moved from `#0d99ff` specifically to reach it. By the piecewise
    /// sRGB function it is 3.0130 and clears the 3.0 floor. By gamma 2.2 it is 2.9940
    /// and fails. Same pixels, opposite verdicts, on the exact boundary the founding
    /// defect was about.
    #[test]
    fn the_piecewise_function_is_not_gamma_two_point_two() {
        fn naive_luminance(hex: &str) -> f64 {
            let c = parsed(hex);
            0.2126 * c.red().powf(2.2) + 0.7152 * c.green().powf(2.2) + 0.0722 * c.blue().powf(2.2)
        }
        let piecewise = ratio("#0087ea", "#e2e8f0");
        let naive = (naive_luminance("#e2e8f0") + 0.05) / (naive_luminance("#0087ea") + 0.05);

        assert_close(piecewise, 3.0130, 0.0001);
        assert_close(naive, 2.9940, 0.0001);
        assert!(meets_floor(piecewise, 3.0));
        assert!(!meets_floor(naive, 3.0));
    }

    // -- Hex ----------------------------------------------------------------

    #[test]
    fn short_hex_doubles_each_digit() {
        assert_eq!(parsed("#fff"), parsed("#ffffff"));
        assert_eq!(parsed("#abc"), parsed("#aabbcc"));
        assert_eq!(parsed("#000"), parsed("#000000"));
    }

    #[test]
    fn hex_carries_alpha_in_four_and_eight_digit_forms() {
        assert_close(parsed("#ffffff80").alpha(), 128.0 / 255.0, 1e-12);
        assert_eq!(parsed("#fff8"), parsed("#ffffff88"));
        assert!(parsed("#ffffffff").is_opaque());
    }

    #[test]
    fn hex_is_case_insensitive() {
        assert_eq!(parsed("#AABBCC"), parsed("#aabbcc"));
    }

    #[test]
    fn a_hex_of_the_wrong_length_is_refused_with_the_value() {
        let error = parse("#ffff0").unwrap_err();
        assert_eq!(
            error,
            ColourError::UnrecognisedSyntax {
                input: "#ffff0".to_string()
            }
        );
    }

    #[test]
    fn a_hex_with_a_non_hex_digit_is_refused() {
        assert!(matches!(
            parse("#gggggg"),
            Err(ColourError::UnrecognisedSyntax { .. })
        ));
    }

    // -- Keywords -----------------------------------------------------------

    #[test]
    fn the_sixteen_basic_keywords_carry_their_specified_values() {
        assert_eq!(parsed("black"), Colour::from_rgb8(0x00, 0x00, 0x00));
        assert_eq!(parsed("silver"), Colour::from_rgb8(0xc0, 0xc0, 0xc0));
        assert_eq!(parsed("gray"), Colour::from_rgb8(0x80, 0x80, 0x80));
        assert_eq!(parsed("white"), Colour::from_rgb8(0xff, 0xff, 0xff));
        assert_eq!(parsed("maroon"), Colour::from_rgb8(0x80, 0x00, 0x00));
        assert_eq!(parsed("red"), Colour::from_rgb8(0xff, 0x00, 0x00));
        assert_eq!(parsed("purple"), Colour::from_rgb8(0x80, 0x00, 0x80));
        assert_eq!(parsed("fuchsia"), Colour::from_rgb8(0xff, 0x00, 0xff));
        assert_eq!(parsed("green"), Colour::from_rgb8(0x00, 0x80, 0x00));
        assert_eq!(parsed("lime"), Colour::from_rgb8(0x00, 0xff, 0x00));
        assert_eq!(parsed("olive"), Colour::from_rgb8(0x80, 0x80, 0x00));
        assert_eq!(parsed("yellow"), Colour::from_rgb8(0xff, 0xff, 0x00));
        assert_eq!(parsed("navy"), Colour::from_rgb8(0x00, 0x00, 0x80));
        assert_eq!(parsed("blue"), Colour::from_rgb8(0x00, 0x00, 0xff));
        assert_eq!(parsed("teal"), Colour::from_rgb8(0x00, 0x80, 0x80));
        assert_eq!(parsed("aqua"), Colour::from_rgb8(0x00, 0xff, 0xff));
        assert_eq!(parsed("grey"), parsed("gray"));
        assert_eq!(parsed("WHITE"), parsed("white"));
    }

    /// The defect: `currentColor` resolved to black would report a passing ratio for
    /// a boundary painted in whatever the element inherited.
    #[test]
    fn current_color_is_a_refusal_and_not_black() {
        assert_eq!(
            parse("currentColor").unwrap_err(),
            ColourError::CurrentColor
        );
        assert_eq!(
            parse("currentcolor").unwrap_err(),
            ColourError::CurrentColor
        );
        assert_ne!(parse("currentColor").ok(), Some(parsed("black")));
    }

    /// The same defect in its more dangerous form, because `transparent` really does
    /// carry black components and a naive parser produces a 21:1 reading from it.
    #[test]
    fn a_bare_transparent_is_a_refusal_and_not_black() {
        assert_eq!(
            parse("transparent").unwrap_err(),
            ColourError::TransparentKeyword
        );
        assert_eq!(Colour::TRANSPARENT.alpha(), 0.0);
        assert!(matches!(
            Colour::TRANSPARENT.relative_luminance(),
            Err(ColourError::TranslucentWithoutBackdrop { .. })
        ));
    }

    #[test]
    fn an_unknown_keyword_is_refused_by_name() {
        assert_eq!(
            parse("rebeccapurple").unwrap_err(),
            ColourError::UnknownKeyword {
                keyword: "rebeccapurple".to_string()
            }
        );
    }

    #[test]
    fn an_empty_value_is_refused_as_empty() {
        assert_eq!(parse("").unwrap_err(), ColourError::Empty);
        assert_eq!(parse("   ").unwrap_err(), ColourError::Empty);
    }

    // -- rgb() --------------------------------------------------------------

    #[test]
    fn rgb_in_both_syntaxes_agrees() {
        let expected = Colour::from_rgb8(0, 135, 234);
        assert_eq!(parsed("rgb(0, 135, 234)"), expected);
        assert_eq!(parsed("rgb(0 135 234)"), expected);
        assert_eq!(parsed("RGB(0 135 234)"), expected);
    }

    #[test]
    fn rgb_alpha_in_both_syntaxes_agrees() {
        let comma = parsed("rgba(0, 0, 0, 0.18)");
        let modern = parsed("rgb(0 0 0 / 0.18)");
        let percentage = parsed("rgb(0 0 0 / 18%)");
        assert_close(comma.alpha(), 0.18, 1e-12);
        assert_eq!(comma, modern);
        assert_close(percentage.alpha(), 0.18, 1e-12);
    }

    #[test]
    fn rgb_percentages_resolve_against_one_hundred() {
        assert_eq!(parsed("rgb(100%, 0%, 0%)"), parsed("#ff0000"));
        assert_eq!(parsed("rgb(0% 50% 100%)").blue(), 1.0);
    }

    #[test]
    fn out_of_range_rgb_channels_are_clamped_as_css_clamps_them() {
        assert_eq!(parsed("rgb(300 -20 0)"), parsed("#ff0000"));
        assert_eq!(parsed("rgb(0 0 0 / 5)").alpha(), 1.0);
    }

    /// Mixing the two component types is invalid in the comma syntax, so the browser
    /// drops the declaration. A ratio computed here would describe a colour nothing
    /// paints, which is a wrong answer wearing the clothes of a right one.
    #[test]
    fn the_comma_syntax_refuses_mixed_component_types() {
        assert_eq!(
            parse("rgb(255, 50%, 0)").unwrap_err(),
            ColourError::LegacyComponentTypeMismatch {
                function: "rgb".to_string()
            }
        );
        // The space syntax does allow it, so this must still parse.
        assert!(parse("rgb(255 50% 0)").is_ok());
    }

    #[test]
    fn a_slash_in_the_comma_syntax_is_refused() {
        assert!(matches!(
            parse("rgb(1, 2, 3 / 0.5)"),
            Err(ColourError::MalformedFunction { .. })
        ));
    }

    #[test]
    fn the_wrong_number_of_components_is_refused_with_the_count() {
        let error = parse("rgb(1 2)").unwrap_err();
        match error {
            ColourError::MalformedFunction { function, detail } => {
                assert_eq!(function, "rgb");
                assert!(detail.contains("found 2"), "detail was `{detail}`");
            }
            other => panic!("expected a malformed function, got {other}"),
        }
    }

    /// `"inf"` and `"NaN"` parse as f64. Unchecked they reach the luminance and come
    /// back as a NaN ratio, and a NaN compares false against every floor, so a failing
    /// boundary is reported as one the gate could not fault.
    #[test]
    fn non_finite_components_are_refused() {
        assert_eq!(
            parse("rgb(inf 0 0)").unwrap_err(),
            ColourError::NonFiniteComponent {
                function: "rgb".to_string()
            }
        );
        assert_eq!(
            parse("rgb(NaN 0 0)").unwrap_err(),
            ColourError::NonFiniteComponent {
                function: "rgb".to_string()
            }
        );
    }

    #[test]
    fn a_none_component_is_refused_by_name() {
        assert_eq!(
            parse("rgb(none 0 0)").unwrap_err(),
            ColourError::NoneComponent {
                function: "rgb".to_string()
            }
        );
    }

    // -- hsl() --------------------------------------------------------------

    /// Compared as hex, not as components, and that is not laziness. `hsl(120 100% 25%)`
    /// is exactly `rgb(0, 127.5, 0)`, and `#008000` is `rgb(0, 128, 0)`. The two are
    /// the same painted pixel and different real numbers, so asserting component
    /// equality would assert that this module quantises, which it does not.
    #[test]
    fn hsl_known_answers() {
        assert_eq!(parsed("hsl(0, 100%, 50%)").to_css_hex(), "#ff0000");
        assert_eq!(parsed("hsl(120 100% 25%)").to_css_hex(), "#008000");
        assert_eq!(parsed("hsl(240 100% 50%)").to_css_hex(), "#0000ff");
        assert_eq!(parsed("hsl(0 0% 50%)").to_css_hex(), "#808080");
        assert_eq!(parsed("hsl(0 0% 100%)").to_css_hex(), "#ffffff");
        assert_eq!(parsed("hsl(210 50% 50%)").to_css_hex(), "#4080bf");
    }

    #[test]
    fn hsl_hue_units_all_reach_the_same_colour() {
        let red = parsed("#ff0000");
        assert_eq!(parsed("hsl(0deg 100% 50%)"), red);
        assert_eq!(parsed("hsl(360deg 100% 50%)"), red);
        assert_eq!(parsed("hsl(0turn 100% 50%)"), red);
        assert_eq!(parsed("hsl(0rad 100% 50%)"), red);
        assert_eq!(parsed("hsl(0grad 100% 50%)"), red);
        assert_eq!(parsed("hsl(-360 100% 50%)"), red);
        assert_eq!(parsed("hsl(0.5turn 100% 50%)"), parsed("hsl(180 100% 50%)"));
    }

    #[test]
    fn an_unknown_angle_unit_is_refused_by_name() {
        assert_eq!(
            parse("hsl(1foo 100% 50%)").unwrap_err(),
            ColourError::UnknownAngleUnit {
                unit: "foo".to_string()
            }
        );
    }

    #[test]
    fn hsla_carries_alpha() {
        assert_close(parsed("hsla(0, 100%, 50%, 0.4)").alpha(), 0.4, 1e-12);
    }

    // -- oklch() ------------------------------------------------------------

    /// The endpoints land one unit in the last place away from 1.0, because
    /// `1.055 * 1 - 0.055` is 0.9999999999999999 in binary floating point and the
    /// transfer function's fixed point is therefore not exact. That is 4e-15 on a
    /// ratio of 21, which is why this module compares colours by their hex rather than
    /// snapping the arithmetic to make an assertion prettier.
    #[test]
    fn oklch_endpoints() {
        assert_eq!(parsed("oklch(1 0 0)").to_css_hex(), "#ffffff");
        assert_eq!(parsed("oklch(0 0 0)").to_css_hex(), "#000000");
        assert_eq!(parsed("oklch(100% 0 0)").to_css_hex(), "#ffffff");
        assert_close(parsed("oklch(1 0 0)").red(), 1.0, 1e-12);
        assert_close(
            contrast_ratio(&opaque("oklch(1 0 0)"), &OpaqueColour::BLACK),
            21.0,
            1e-12,
        );
    }

    /// The subject stylesheet records its own conversion in a comment: the token
    /// `--border-strong` in the `.sign-layout` scope "resolves here to
    /// `oklch(0.82 0.008 85)` = `#c6c4be`". Reproducing that byte for byte is the
    /// cross-check that the two published matrices were entered correctly, which is
    /// the failure this test exists to catch: a transposed digit gives a plausible
    /// colour and therefore a plausible ratio that is not the ratio.
    #[test]
    fn oklch_matches_the_hex_the_real_stylesheet_records() {
        assert_eq!(parsed("oklch(0.82 0.008 85)").to_css_hex(), "#c6c4be");
    }

    /// The sRGB primaries have known OKLCh coordinates. Round-tripping them is the
    /// check on hue and chroma, which the achromatic endpoints above cannot exercise.
    #[test]
    fn oklch_reaches_the_srgb_primaries() {
        assert_eq!(
            parsed("oklch(0.62795550 0.25768330 29.23388deg)").to_css_hex(),
            "#ff0000"
        );
        assert_eq!(
            parsed("oklch(0.86643965 0.29483287 142.49535)").to_css_hex(),
            "#00ff00"
        );
        assert_eq!(
            parsed("oklch(0.45201370 0.31321437 264.05202)").to_css_hex(),
            "#0000ff"
        );
    }

    /// The forward and inverse matrices are published as separately rounded ten-digit
    /// values, so their product is the identity only to about one part in ten million.
    /// The tolerance here records that, because it is the accuracy ceiling on any
    /// `color-mix(in oklab, ...)` and someone should know the number rather than
    /// discover it. One part in ten million is a thousandth of an 8-bit step.
    #[test]
    fn oklch_round_trips_through_oklab_and_back() {
        for input in [
            "oklch(0.985 0.004 85)",
            "oklch(0.5 0.02 85)",
            "oklch(0.18 0.01 80)",
        ] {
            let colour = parsed(input);
            let linear = colour.linear();
            let lab = linear_srgb_to_oklab([linear.r, linear.g, linear.b]);
            let back = oklab_to_linear_srgb(lab[0], lab[1], lab[2]);
            assert_close(back[0], linear.r, 1e-6);
            assert_close(back[1], linear.g, 1e-6);
            assert_close(back[2], linear.b, 1e-6);
        }
    }

    /// Clamping an out-of-gamut colour would answer a question about a different
    /// colour, and the browser reaches its own answer by a gamut-mapping algorithm
    /// this module does not implement. So it refuses, and names the channel.
    #[test]
    fn an_out_of_gamut_oklch_is_refused_and_names_the_channel() {
        let error = parse("oklch(0.7 0.4 30)").unwrap_err();
        match error {
            ColourError::OutOfSrgbGamut {
                input,
                channel,
                value,
            } => {
                assert_eq!(input, "oklch(0.7 0.4 30)");
                assert_eq!(channel, "red");
                assert!(value > 1.0, "expected a channel above one, got {value}");
            }
            other => panic!("expected an out-of-gamut refusal, got {other}"),
        }
        // The other direction, a channel below zero, is refused the same way.
        assert!(matches!(
            parse("oklch(0.9 0.35 140)"),
            Err(ColourError::OutOfSrgbGamut { .. })
        ));
        // And a colour a hair outside, which is only the rounding of its own
        // coordinates, is snapped rather than refused. See GAMUT_EPSILON.
        assert!(parse("oklch(0.86643965 0.29483287 142.49535)").is_ok());
    }

    #[test]
    fn oklch_alpha_is_read() {
        assert_close(parsed("oklch(0.5 0.02 85 / 0.25)").alpha(), 0.25, 1e-12);
    }

    // -- color-mix() --------------------------------------------------------

    #[test]
    fn color_mix_defaults_to_half_and_half() {
        assert_eq!(
            parsed("color-mix(in srgb, red, blue)").to_css_hex(),
            "#800080"
        );
    }

    #[test]
    fn a_single_percentage_implies_the_complement() {
        assert_eq!(
            parsed("color-mix(in srgb, red 25%, blue)"),
            parsed("color-mix(in srgb, red 25%, blue 75%)")
        );
        assert_eq!(
            parsed("color-mix(in srgb, red, blue 25%)"),
            parsed("color-mix(in srgb, red 75%, blue 25%)")
        );
    }

    /// CSS Color 5: percentages that do not sum to 100 are scaled to 100, and where
    /// they summed to less than 100 the result's alpha is multiplied by that sum.
    /// Both halves are asserted, because an implementation that scales and forgets the
    /// alpha multiplier produces an opaque colour where the browser paints a
    /// translucent one, which changes every ratio measured from it.
    #[test]
    fn percentages_are_scaled_and_a_short_sum_multiplies_alpha() {
        let scaled_up = parsed("color-mix(in srgb, red 60%, blue 60%)");
        assert_eq!(scaled_up.to_css_hex(), parsed("#800080").to_css_hex());
        assert!(scaled_up.is_opaque());

        let short = parsed("color-mix(in srgb, red 30%, blue 30%)");
        assert_close(short.alpha(), 0.6, 1e-12);
        assert_close(short.red(), 0.5, 1e-12);
        assert_close(short.blue(), 0.5, 1e-12);
    }

    #[test]
    fn percentages_summing_to_zero_are_refused() {
        assert_eq!(
            parse("color-mix(in srgb, red 0%, blue 0%)").unwrap_err(),
            ColourError::ZeroPercentageSum
        );
    }

    /// The premultiplied rule, on the shape that actually appears in the subject
    /// stylesheet thirty-odd times. Without premultiplying, mixing 6% of a colour with
    /// `transparent` drags the result towards black, because `transparent` is
    /// `rgba(0, 0, 0, 0)` and its zeroes get averaged in. With it, the result is the
    /// colour at 6% alpha, which is what the design means and what the browser paints.
    #[test]
    fn mixing_with_transparent_keeps_the_colour_and_takes_the_alpha() {
        let mixed = parsed("color-mix(in srgb, #ff0000 6%, transparent)");
        assert_close(mixed.alpha(), 0.06, 1e-12);
        assert_close(mixed.red(), 1.0, 1e-12);
        assert_close(mixed.green(), 0.0, 1e-12);

        // The un-premultiplied answer, for the record: a near-black at the same alpha,
        // which over white composites to a grey rather than to a pink.
        let over_white = mixed.composite_over(&OpaqueColour::WHITE);
        assert_eq!(over_white.to_css_hex(), "#fff0f0");
    }

    #[test]
    fn transparent_is_resolvable_inside_a_mix_but_not_alone() {
        assert!(parse("color-mix(in srgb, black 6%, transparent)").is_ok());
        assert_eq!(
            parse("transparent").unwrap_err(),
            ColourError::TransparentKeyword
        );
    }

    #[test]
    fn a_percentage_may_precede_its_colour() {
        assert_eq!(
            parsed("color-mix(in srgb, 25% red, blue)"),
            parsed("color-mix(in srgb, red 25%, blue)")
        );
    }

    #[test]
    fn mixing_in_oklab_is_not_the_same_as_mixing_in_srgb() {
        let in_oklab = parsed("color-mix(in oklab, white, black)");
        let in_srgb = parsed("color-mix(in srgb, white, black)");
        assert_eq!(in_oklab.to_css_hex(), "#636363");
        assert_eq!(in_srgb.to_css_hex(), "#808080");
        assert_ne!(in_oklab, in_srgb);
    }

    #[test]
    fn mixing_in_linear_srgb_is_its_own_answer() {
        let linear = parsed("color-mix(in srgb-linear, white, black)");
        assert_eq!(linear.to_css_hex(), "#bcbcbc");
    }

    /// An unimplemented space is named, not approximated in sRGB. Approximating would
    /// put a number in the report that no browser will reproduce.
    #[test]
    fn an_unimplemented_interpolation_space_is_refused_by_name() {
        assert_eq!(
            parse("color-mix(in oklch, red, blue)").unwrap_err(),
            ColourError::UnimplementedInterpolationSpace {
                space: "oklch".to_string()
            }
        );
        assert_eq!(
            parse("color-mix(in hsl longer hue, red, blue)").unwrap_err(),
            ColourError::UnimplementedInterpolationSpace {
                space: "hsl longer hue".to_string()
            }
        );
    }

    /// CSS keywords are ASCII case-insensitive, and a stylesheet that writes `In sRGB`
    /// paints the same pixels as one that writes `in srgb`.
    #[test]
    fn the_interpolation_space_keyword_is_case_insensitive() {
        assert_eq!(
            parsed("color-mix(In sRGB, white, black)").to_css_hex(),
            "#808080"
        );
    }

    #[test]
    fn a_mix_without_an_interpolation_space_is_refused() {
        assert!(matches!(
            parse("color-mix(red, blue)"),
            Err(ColourError::MalformedFunction { .. })
        ));
        assert!(matches!(
            parse("color-mix(in, red, blue)"),
            Err(ColourError::MalformedFunction { .. })
        ));
    }

    #[test]
    fn a_mix_of_mixes_resolves() {
        let nested = parsed("color-mix(in srgb, color-mix(in srgb, white, black) 50%, white)");
        assert_eq!(nested.to_css_hex(), "#bfbfbf");
    }

    #[test]
    fn currentcolor_inside_a_mix_propagates_its_refusal() {
        assert_eq!(
            parse("color-mix(in srgb, currentColor 50%, white)").unwrap_err(),
            ColourError::CurrentColor
        );
    }

    // -- var() --------------------------------------------------------------

    #[test]
    fn an_unresolved_var_is_refused_by_property_name() {
        assert_eq!(
            parse("var(--border-control)").unwrap_err(),
            ColourError::UnresolvedCustomProperty {
                name: "--border-control".to_string()
            }
        );
    }

    #[test]
    fn a_var_resolves_through_the_supplied_properties() {
        let lookup = |name: &str| match name {
            "--accent" => Some("#0066ff".to_string()),
            "--primary" => Some("var(--accent)".to_string()),
            _ => None,
        };
        assert_eq!(
            parse_with("var(--primary)", &lookup).unwrap(),
            parsed("#0066ff")
        );
    }

    /// CSS substitutes `var()` on the token stream, so a custom property can supply
    /// part of a function's arguments. `rgb(var(--channels) / 0.5)` is a common way to
    /// write a translucent brand colour, and a parser that only handles a whole-value
    /// `var()` reports it as unrecognised syntax.
    #[test]
    fn a_var_may_supply_part_of_a_function() {
        let lookup = |name: &str| match name {
            "--dashboard-select-rgb" => Some("0 135 234".to_string()),
            _ => None,
        };
        let colour = parse_with("rgb(var(--dashboard-select-rgb) / 0.2)", &lookup).unwrap();
        assert_eq!(colour.to_css_hex(), "#0087ea33");
        assert_close(colour.alpha(), 0.2, 1e-12);
    }

    /// A fallback is for a property the cascade never defined. Using it when the
    /// property IS defined would measure a colour the page does not paint.
    #[test]
    fn a_var_fallback_is_used_only_when_the_property_is_undefined() {
        let empty = |_: &str| None;
        assert_eq!(
            parse_with("var(--missing, #ff0000)", &empty).unwrap(),
            parsed("#ff0000")
        );
        let defined = |name: &str| (name == "--defined").then(|| "#00ff00".to_string());
        assert_eq!(
            parse_with("var(--defined, #ff0000)", &defined).unwrap(),
            parsed("#00ff00")
        );
    }

    /// Two properties that name each other are a real thing to find in a large
    /// stylesheet. Without a limit the gate hangs, and a hung pre-push hook reads as a
    /// broken machine rather than as a finding.
    #[test]
    fn a_var_cycle_is_reported_rather_than_followed_forever() {
        let cyclic = |name: &str| match name {
            "--a" => Some("var(--b)".to_string()),
            "--b" => Some("var(--a)".to_string()),
            _ => None,
        };
        assert_eq!(
            parse_with("var(--a)", &cyclic).unwrap_err(),
            ColourError::SubstitutionLoop {
                limit: MAX_SUBSTITUTION_DEPTH
            }
        );
    }

    #[test]
    fn a_var_resolving_to_an_unparsable_value_reports_that_and_not_the_fallback() {
        let broken = |name: &str| (name == "--x").then(|| "not-a-colour".to_string());
        assert_eq!(
            parse_with("var(--x, #ffffff)", &broken).unwrap_err(),
            ColourError::UnknownKeyword {
                keyword: "not-a-colour".to_string()
            }
        );
    }

    // -- Alpha, compositing and refusals ------------------------------------

    /// The central refusal. A translucent colour has a different luminance over every
    /// backdrop, so one number for it is one backdrop's number, presented as if it
    /// were the colour's own.
    #[test]
    fn a_translucent_colour_has_no_ratio_of_its_own() {
        let glass = parsed("rgba(255, 255, 255, 0.82)");
        let error = contrast_ratio_of(&glass, &parsed("#0f172a")).unwrap_err();
        match error {
            ColourError::TranslucentWithoutBackdrop { alpha } => assert_close(alpha, 0.82, 1e-12),
            other => panic!("expected a translucency refusal, got {other}"),
        }
        assert!(glass.require_opaque().is_err());
    }

    #[test]
    fn a_translucent_colour_measured_over_a_named_backdrop_does_have_one() {
        let rule = parsed("rgba(0, 0, 0, 0.18)");
        let painted = rule.composite_over(&OpaqueColour::WHITE);
        assert_eq!(painted.to_css_hex(), "#d1d1d1");
        assert_close(
            contrast_ratio(&painted, &OpaqueColour::WHITE),
            1.5255,
            0.0001,
        );
    }

    /// Compositing in linear light instead of in the encoded space gives a different
    /// pixel and a different verdict. Recorded as a number so the choice cannot be
    /// quietly reversed.
    #[test]
    fn compositing_happens_in_the_encoded_space_not_in_linear_light() {
        let encoded_space = parsed("rgba(0, 0, 0, 0.18)")
            .composite_over(&OpaqueColour::WHITE)
            .relative_luminance();
        // What the same layers would give if the arithmetic were done on linear light.
        let linear_space_luminance = 1.0f64 * (1.0 - 0.18);
        assert_close(encoded_space, 0.6383, 0.0001);
        assert_close(linear_space_luminance, 0.82, 1e-12);
        assert_close(1.05 / (encoded_space + 0.05), 1.5255, 0.0001);
        assert_close(1.05 / (linear_space_luminance + 0.05), 1.2069, 0.0001);
    }

    #[test]
    fn a_stack_of_layers_composites_bottom_first() {
        let backdrop = OpaqueColour::WHITE;
        let bottom = parsed("rgba(0, 0, 0, 0.5)");
        let top = parsed("rgba(255, 255, 255, 0.5)");
        let stacked = composite_stack(&[bottom, top], &backdrop);
        // Bottom first: white -> 0.5 grey -> half way back to white.
        assert_eq!(stacked.to_css_hex(), "#bfbfbf");
        // The other order is a different pixel, which is why the helper exists.
        let reversed = composite_stack(&[top, bottom], &backdrop);
        assert_eq!(reversed.to_css_hex(), "#808080");
    }

    #[test]
    fn contrast_over_a_backdrop_paints_both_layers_first() {
        let translucent_rule = parsed("rgba(0, 0, 0, 0.25)");
        let card = parsed("#ffffff");
        let measured = contrast_ratio_over(&translucent_rule, &card, &OpaqueColour::WHITE);
        let by_hand = contrast_ratio(
            &translucent_rule.composite_over(&OpaqueColour::WHITE),
            &OpaqueColour::WHITE,
        );
        assert_close(measured, by_hand, 1e-12);
    }

    // -- Reporting ----------------------------------------------------------

    /// A gate that rounds before it compares passes 2.996:1 against a 3.0:1 floor,
    /// which is precisely the near-miss the floor exists to catch.
    #[test]
    fn a_floor_is_compared_against_the_unrounded_ratio() {
        assert!(!meets_floor(2.996, 3.0));
        assert!(meets_floor(3.0, 3.0));
        assert_eq!(format_ratio(2.996), "2.99");
        assert_eq!(format_ratio(3.004), "3.00");
    }

    #[test]
    fn a_displayed_ratio_truncates_so_it_never_flatters() {
        assert_eq!(format_ratio(21.0), "21.00");
        assert_eq!(format_ratio(1.2043515089087955), "1.20");
        assert_eq!(format_ratio(2.9999), "2.99");
    }

    // -- The real stylesheet ------------------------------------------------

    /// An excerpt of the real subject, `opbox-frontend/app/globals.css`, at the
    /// commit read on 2026-07-25.
    ///
    /// Committed here as a string rather than as a separate fixture file for two
    /// reasons: it keeps this module's whole test corpus in the one file that owns it,
    /// and it cannot go missing from a clean checkout the way a path can. The
    /// declarations are verbatim; the long provenance comments that follow several of
    /// them in the original are elided, and every value is unchanged. The full file is
    /// exercised by `the_whole_real_stylesheet_resolves`, which is `#[ignore]`d
    /// because it reads an absolute path outside this repository.
    const GLOBALS_EXCERPT: &str = r"
:root {
  --bg-primary: #ffffff;
  --bg-secondary: #f8fafc;
  --bg-tertiary: #f1f5f9;
  --app-canvas: #eef0f2;
  --soft-3: #e2e8f0;
  --border: #cbd5e1;
  --border-control: #748eaf;
  --border-strong: #94a3b8;
  --border-hover: #556680;
  --comment-rule: #b98200;
  --accent: #0066ff;
  --dashboard-select: #0087ea;
  --dashboard-select-rgb: 0 135 234;
  --dashboard-select-bg: rgba(0, 135, 234, 0.08);
  --spotlight-glass: rgba(255, 255, 255, 0.82);
  --status-success-bg: rgba(34, 197, 94, 0.1);
  --ink: #0f172a;
  --paper: var(--bg-primary);
  --soft: var(--bg-secondary);
  --dotfield-dot: var(--ink);
}

.ember {
  --bg-primary: #1c1208;
  --bg-secondary: #150d05;
  --bg-tertiary: #251a0c;
  --border: #2d1f0a;
  --border-control: #8b601f;
  --border-hover: #b47e2e;
}

.sign-layout {
  --bg-primary: oklch(0.985 0.004 85);
  --bg-secondary: oklch(0.97 0.005 85);
  --bg-tertiary: oklch(0.95 0.006 85);
  --soft-3: oklch(0.93 0.006 85);
  --border: oklch(0.9 0.006 85);
  --border-strong: oklch(0.82 0.008 85);
  --border-hover: oklch(0.5 0.02 85);
  --border-control: oklch(0.6 0.02 85);
  --comment-rule: #b37e00;
}

.onyx-btn--default:hover:not(:disabled) { background: color-mix(in srgb, var(--ink) 4%, transparent); }
.onyx-tag--mute { background: color-mix(in srgb, var(--ink) 8%, transparent); color: var(--ink-sub); }
.ag-row-hover { background: color-mix(in srgb, var(--accent) 14%, var(--bg-primary)) !important; }
";

    /// Strip `/* ... */` so a comment containing a semicolon cannot end a declaration.
    fn strip_comments(css: &str) -> String {
        let mut out = String::with_capacity(css.len());
        let mut rest = css;
        while let Some(start) = rest.find("/*") {
            out.push_str(&rest[..start]);
            match rest[start + 2..].find("*/") {
                Some(end) => rest = &rest[start + 2 + end + 2..],
                None => return out,
            }
        }
        out.push_str(rest);
        out
    }

    /// The custom properties declared inside one selector's block.
    ///
    /// Enough of a CSS reader for a test, and no more: this module's job is colour,
    /// and reading a stylesheet properly belongs to its sibling.
    fn properties_in_block(css: &str, selector: &str) -> Vec<(String, String)> {
        let cleaned = strip_comments(css);
        let mut out = Vec::new();
        let mut search_from = 0usize;
        while let Some(found) = cleaned[search_from..].find(selector) {
            let at = search_from + found;
            search_from = at + selector.len();
            let before_is_boundary = at == 0
                || !cleaned[..at]
                    .chars()
                    .next_back()
                    .is_some_and(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.');
            let after = cleaned[search_from..].trim_start();
            if !before_is_boundary || !after.starts_with('{') {
                continue;
            }
            let open = cleaned[search_from..].find('{').unwrap() + search_from;
            let mut depth = 0usize;
            let mut close = open;
            for (index, ch) in cleaned[open..].char_indices() {
                match ch {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            close = open + index;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            for declaration in split_top_level(&cleaned[open + 1..close], ';') {
                let Some((name, value)) = declaration.split_once(':') else {
                    continue;
                };
                let name = name.trim();
                if name.starts_with("--") {
                    out.push((name.to_string(), value.trim().to_string()));
                }
            }
        }
        out
    }

    /// Look a property up in a theme block, falling back to `:root`, which is how the
    /// cascade resolves a token a theme does not override.
    fn theme_lookup<'a>(css: &'a str, theme: &'a str) -> impl Fn(&str) -> Option<String> + use<'a> {
        move |name: &str| {
            let in_theme = properties_in_block(css, theme);
            if let Some((_, value)) = in_theme.iter().rev().find(|(n, _)| n == name) {
                return Some(value.clone());
            }
            properties_in_block(css, ":root")
                .iter()
                .rev()
                .find(|(n, _)| n == name)
                .map(|(_, value)| value.clone())
        }
    }

    fn token_ratio(css: &str, theme: &str, token: &str, plane: &str) -> f64 {
        let lookup = theme_lookup(css, theme);
        let a = parse_with(&format!("var({token})"), &lookup)
            .unwrap_or_else(|e| panic!("{theme} {token}: {e}"))
            .require_opaque()
            .unwrap_or_else(|e| panic!("{theme} {token}: {e}"));
        let b = parse_with(&format!("var({plane})"), &lookup)
            .unwrap_or_else(|e| panic!("{theme} {plane}: {e}"))
            .require_opaque()
            .unwrap_or_else(|e| panic!("{theme} {plane}: {e}"));
        contrast_ratio(&a, &b)
    }

    /// The defect VDS was built for, measured.
    ///
    /// The subject stylesheet's own `.ember` block records, in prose, that `--border`
    /// "was 1.15:1 and fails SC 1.4.11". This module reads the same declarations and
    /// reaches 1.15 on `--bg-primary` and 1.20 on `--bg-secondary`, which are the two
    /// numbers the founding record quotes. A gate that could produce these from the
    /// stylesheet would have caught the defect at the time.
    #[test]
    fn the_founding_defect_reproduces_from_the_excerpt() {
        let css = GLOBALS_EXCERPT;
        assert_eq!(
            rounded(token_ratio(css, ".ember", "--border", "--bg-primary")),
            "1.15"
        );
        assert_eq!(
            rounded(token_ratio(css, ".ember", "--border", "--bg-secondary")),
            "1.20"
        );
        assert!(!meets_floor(
            token_ratio(css, ".ember", "--border", "--bg-primary"),
            3.0
        ));
        // And the token the theme replaced it with clears the floor on all three planes.
        for plane in ["--bg-primary", "--bg-secondary", "--bg-tertiary"] {
            assert!(meets_floor(
                token_ratio(css, ".ember", "--border-control", plane),
                3.0
            ));
        }
    }

    /// The subject stylesheet carries readings in its own comments, computed by a
    /// different tool. Reproducing them exactly, to the two decimals they are quoted
    /// at, is the strongest available check that this module agrees with a working
    /// implementation on real values.
    ///
    /// `--border-hover: #556680` is annotated "5.84, 5.58, 5.33" on the light theme's
    /// three planes; `--dashboard-select: #0087ea` is annotated "3.71, 3.55, 3.39";
    /// `--border-control: #748eaf` is annotated "3.08:1 min"; `--border: #cbd5e1` is
    /// annotated "1.48:1".
    #[test]
    fn the_stylesheets_own_recorded_readings_reproduce() {
        let css = GLOBALS_EXCERPT;
        let planes = ["--bg-primary", "--bg-secondary", "--bg-tertiary"];
        let expected: [(&str, [&str; 3]); 4] = [
            ("--border-hover", ["5.84", "5.58", "5.33"]),
            ("--dashboard-select", ["3.71", "3.55", "3.39"]),
            ("--border-control", ["3.37", "3.22", "3.08"]),
            ("--border", ["1.48", "1.42", "1.36"]),
        ];
        for (token, readings) in expected {
            for (plane, reading) in planes.iter().zip(readings) {
                assert_eq!(
                    rounded(token_ratio(css, ":root", token, plane)),
                    reading,
                    "{token} on {plane}"
                );
            }
        }
    }

    /// The same, through the `oklch()` path, which is where an arithmetic error would
    /// hide. The `.sign-layout` scope states its `--border-control` as
    /// `oklch(0.6 0.02 85)` and records "3.21:1 minimum" across its planes.
    ///
    /// The scope's other recorded readings are reproduced only after
    /// [`Colour::quantise_8bit`]: unquantised this module reads 5.75 / 5.50 / 5.19 for
    /// `--border-hover` where the comment says 5.73 / 5.49 / 5.16, and quantised it
    /// reads exactly those. That is the incumbent tool going through an 8-bit hex, and
    /// it is asserted here so the difference is a recorded property of the two
    /// pipelines rather than an unexplained disagreement.
    #[test]
    fn the_oklch_scope_reproduces_and_names_its_quantisation_difference() {
        let css = GLOBALS_EXCERPT;
        let lookup = theme_lookup(css, ".sign-layout");
        let quantised_ratio = |token: &str, plane: &str| {
            let a = parse_with(&format!("var({token})"), &lookup)
                .unwrap()
                .quantise_8bit()
                .require_opaque()
                .unwrap();
            let b = parse_with(&format!("var({plane})"), &lookup)
                .unwrap()
                .quantise_8bit()
                .require_opaque()
                .unwrap();
            contrast_ratio(&a, &b)
        };

        assert_eq!(
            rounded(token_ratio(
                css,
                ".sign-layout",
                "--border-control",
                "--soft-3"
            )),
            "3.21"
        );
        assert_eq!(
            rounded(token_ratio(
                css,
                ".sign-layout",
                "--border-hover",
                "--soft-3"
            )),
            "4.88"
        );
        assert_eq!(
            rounded(token_ratio(
                css,
                ".sign-layout",
                "--border-hover",
                "--bg-primary"
            )),
            "5.75"
        );
        assert_eq!(
            rounded(quantised_ratio("--border-hover", "--bg-primary")),
            "5.73"
        );
        assert_eq!(
            rounded(quantised_ratio("--border-hover", "--bg-tertiary")),
            "5.16"
        );
    }

    /// The `color-mix()` rules in the excerpt are the shape that dominates the real
    /// file: a token at a low percentage against `transparent`, which is a translucent
    /// result and therefore has no ratio until a backdrop is named.
    #[test]
    fn a_real_color_mix_declaration_resolves_to_a_translucent_tint() {
        let lookup = theme_lookup(GLOBALS_EXCERPT, ":root");
        let mixed = parse_with("color-mix(in srgb, var(--ink) 8%, transparent)", &lookup).unwrap();
        assert_close(mixed.alpha(), 0.08, 1e-12);
        assert_eq!(mixed.to_css_hex(), "#0f172a14");
        assert!(matches!(
            mixed.relative_luminance(),
            Err(ColourError::TranslucentWithoutBackdrop { .. })
        ));
        // Over the light theme's paper it is a very pale grey, and nowhere near a
        // boundary floor, which is the finding a gate would report.
        let painted = mixed.composite_over(&OpaqueColour::WHITE);
        assert_eq!(painted.to_css_hex(), "#ececee");
        assert!(!meets_floor(
            contrast_ratio(&painted, &OpaqueColour::WHITE),
            3.0
        ));
    }

    #[test]
    fn a_real_color_mix_over_an_opaque_plane_is_opaque() {
        let lookup = theme_lookup(GLOBALS_EXCERPT, ":root");
        let mixed = parse_with(
            "color-mix(in srgb, var(--accent) 14%, var(--bg-primary))",
            &lookup,
        )
        .unwrap();
        assert!(mixed.is_opaque());
        assert_eq!(mixed.to_css_hex(), "#dbeaff");
    }

    /// The whole real stylesheet, 7,308 lines of it, read from its absolute path.
    ///
    /// `#[ignore]`d because that path is outside this repository and a clean checkout
    /// elsewhere would fail on it. A test that silently passed when the file was
    /// missing would be vacuous, which is worse than one that says it did not run.
    ///
    /// Run it with:
    /// `cargo test -p vds-css -- --ignored --nocapture the_whole_real_stylesheet_resolves`
    #[test]
    #[ignore = "reads /home/jellytot/Projects/opbox-prod/opbox-frontend/app/globals.css, which is outside this repository"]
    fn the_whole_real_stylesheet_resolves() {
        const PATH: &str = "/home/jellytot/Projects/opbox-prod/opbox-frontend/app/globals.css";
        let css = std::fs::read_to_string(PATH).expect("the subject stylesheet should be readable");

        let themes = [
            ":root",
            ".dark",
            ".neon",
            ".ember",
            ".ocean",
            ".sign-layout",
        ];
        let mut resolved = 0usize;
        let mut translucent = 0usize;
        let mut deliberate: Vec<(String, String, String)> = Vec::new();
        let mut refused: Vec<(String, String, String)> = Vec::new();

        for theme in themes {
            let lookup = theme_lookup(&css, theme);
            for (name, value) in properties_in_block(&css, theme) {
                // A theme block is mostly not colours: it also carries lengths, font
                // stacks and shadow lists. Substitute first and judge the shape after,
                // so that an alias like `--paper: var(--bg-primary)` is measured and an
                // alias like `--font-body: var(--font-geist-sans)` is not.
                let Ok(substituted) = substitute_vars(&value, &lookup, 0) else {
                    continue;
                };
                if !looks_like_a_colour(&substituted) {
                    continue;
                }
                match parse_with(&value, &lookup) {
                    Ok(colour) => {
                        if colour.is_opaque() {
                            resolved += 1;
                        } else {
                            translucent += 1;
                        }
                    }
                    // `transparent` and `currentColor` are refused on purpose, and are
                    // separated here so they cannot be read as a gap in the parser.
                    Err(error @ (ColourError::TransparentKeyword | ColourError::CurrentColor)) => {
                        deliberate.push((theme.to_string(), name, error.to_string()));
                    }
                    Err(error) => refused.push((theme.to_string(), name, error.to_string())),
                }
            }
        }

        println!("resolved opaque: {resolved}");
        println!("resolved translucent: {translucent}");
        println!("refused on purpose: {}", deliberate.len());
        for (theme, name, error) in &deliberate {
            println!("  {theme} {name}: {error}");
        }
        println!("refused: {}", refused.len());
        for (theme, name, error) in &refused {
            println!("  {theme} {name}: {error}");
        }

        // The five themes' control tokens, against the three planes each theme
        // declares, are the reading the founding defect was about.
        for theme in [
            ":root",
            ".dark",
            ".neon",
            ".ember",
            ".ocean",
            ".sign-layout",
        ] {
            for plane in ["--bg-primary", "--bg-secondary", "--bg-tertiary"] {
                let control = token_ratio(&css, theme, "--border-control", plane);
                let border = token_ratio(&css, theme, "--border", plane);
                println!(
                    "{theme} --border-control on {plane}: {}  (--border: {})",
                    format_ratio(control),
                    format_ratio(border)
                );
                assert!(
                    meets_floor(control, 3.0),
                    "{theme} --border-control on {plane} is {} and the floor is 3.0",
                    format_ratio(control)
                );
            }
        }

        assert!(resolved > 200, "only {resolved} opaque colours resolved");
        assert!(
            refused.is_empty(),
            "{} colour-shaped declarations did not resolve",
            refused.len()
        );
    }

    /// Whether a declaration's value is worth handing to the colour parser.
    ///
    /// A theme block is mostly not colours. This keeps the sweep above honest: it
    /// counts refusals of things that really were colours, rather than of every
    /// `264px` and every font stack.
    fn looks_like_a_colour(value: &str) -> bool {
        let v = value.trim().to_ascii_lowercase();
        if v.is_empty() {
            return false;
        }
        let is_function = ["rgb(", "rgba(", "hsl(", "hsla(", "oklch(", "color-mix("]
            .iter()
            .any(|prefix| v.starts_with(prefix));
        let is_keyword = v == "transparent"
            || v == "currentcolor"
            || NAMED_COLOURS.iter().any(|(name, _, _, _)| *name == v);
        v.starts_with('#') || is_function || is_keyword
    }
}
