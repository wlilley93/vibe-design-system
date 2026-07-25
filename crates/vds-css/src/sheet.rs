//! Custom properties extracted from a stylesheet, and resolved per theme.
//!
//! VDS exists because a control boundary was declared aligned in prose and
//! shipped at 1.20:1 against a 3.0:1 requirement, across five themes. The
//! `contrast` proof is the gate that would have caught it, and this module is
//! the half of that gate that reads the stylesheet. Nothing here computes a
//! ratio, parses a colour or decides whether a value IS a colour: the output is
//! a value string per (theme, property), or a typed statement of why there is
//! none.
//!
//! Three properties of this module are load-bearing, and each of them is a
//! defect that a smaller implementation would ship.
//!
//! **It is a scanner, not a set of regular expressions.** A `{` inside a string
//! or a comment closes a block early, and every declaration after it is filed
//! under the wrong selector. On the subject stylesheet that is not theoretical:
//! `app/globals.css` line 3954 carries `onClick={handleAddPage}` inside a
//! comment, and a brace-counting scanner that does not know what a comment is
//! loses block-nesting synchronisation there and mis-files the rest of the file.
//! The failure is silent, and its shape is that a theme is credited with another
//! scope's values. That is the same defect class `vds-scan`'s `jsx` module was
//! rewritten to remove.
//!
//! **Substitution happens in the theme's context, at every level.** `--paper` is
//! declared once, in `:root`, as `var(--bg-primary)`. Resolving it once and
//! reusing the answer gives every theme `#ffffff`, because that is what
//! `--bg-primary` is in `:root`. In `.dark` the same declaration resolves to
//! `#1a1a1a`. A gate that got this wrong would measure every dark theme against
//! a white background and report the dark themes as the safest in the system.
//!
//! **A refusal is a result.** Where the sheet cannot be resolved without
//! guessing, the answer is an [`Unresolvable`] naming the reason, never a
//! plausible string. A gate that reports a ratio it computed wrongly is worse
//! than one that says it could not compute it, because the first gets believed.
//!
//! ## Where this deliberately diverges from the CSS specification
//!
//! CSS Variables Level 1 §3.1 makes every custom property in a dependency cycle
//! compute to the guaranteed-invalid value, which means a `var()` referencing
//! one falls back to its second argument. This module REFUSES a cycle instead,
//! and returns the cycle path. Taking the fallback would hand the gate a colour
//! to measure while the sheet contains a defect that nothing else reports, and
//! the measured colour would be the one shown only by accident. A cycle is a
//! fault in the stylesheet, and a governance gate exists to say so.
//!
//! Cascade layers are modelled only as far as they can be modelled exactly. See
//! [`Unresolvable::LayerConflict`].

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// How many custom-property references a single resolution may follow before it
/// gives up.
///
/// Cycle detection already terminates every cycle, so this cap exists for the
/// acyclic-but-absurd chain, and to keep a hostile or generated stylesheet from
/// exhausting the stack inside a CI gate. A gate that crashes is a gate that
/// gets switched off.
pub const MAX_RESOLUTION_DEPTH: usize = 32;

/// How many characters one resolution may produce before it gives up.
///
/// Substitution is not linear. `--a: var(--b) var(--b)` repeated down a chain of
/// n properties expands to 2^n, which is acyclic, within the depth cap, and
/// still hangs the process. The depth cap alone does not bound the output.
pub const MAX_EXPANSION_CHARS: usize = 64 * 1024;

/// One `--name: value` declaration, and the context that decides whether it
/// applies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Declaration {
    /// The custom property name, including the leading `--`.
    pub property: String,
    /// The value as written, with comments removed and whitespace outside
    /// strings collapsed. `!important` is not part of it.
    pub value: String,
    /// Whether the declaration carried `!important`, which changes which
    /// declaration wins and which is therefore not merely cosmetic.
    pub important: bool,
    /// The single selector this declaration applies to. A selector LIST is
    /// recorded once per member, because `:root, [data-theme='light'] { ... }`
    /// establishes the value for both scopes and recording it under the joined
    /// text would establish it for neither.
    pub selector: String,
    /// The dotted cascade-layer path, or `None` for an unlayered declaration.
    pub layer: Option<String>,
    /// The `@media`, `@supports` and `@container` preludes guarding this
    /// declaration, outermost first. Empty means the declaration always applies.
    ///
    /// A guarded declaration is not applied by [`Sheet::resolve`], because there
    /// is no single viewport or feature state to resolve against and picking one
    /// would be a guess. It is reported instead, on
    /// [`Resolution::conditional`], so the caller can see that the resolved
    /// value is not the only value the property ever takes.
    pub conditions: Vec<String>,
    /// 1-based line in the source, which survives comment blanking.
    pub line: u32,
    /// Position in source order across the whole sheet, which is what decides a
    /// tie within one scope and one layer.
    pub order: usize,
}

/// A scope that redefines the token palette.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Theme {
    /// The selector, exactly as it appears in the sheet.
    pub selector: String,
    /// True for `:root`, whose declarations every other theme falls back to.
    pub is_base: bool,
}

/// A declaration a conditional at-rule guards, reported rather than applied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionalValue {
    /// The scope the declaration sits in, which may be the base rather than the
    /// theme asked about.
    pub selector: String,
    /// The guarding preludes, outermost first.
    pub conditions: Vec<String>,
    /// The value AS WRITTEN. It is deliberately not substituted: the properties
    /// it references may themselves be conditional, and resolving it against the
    /// unconditional palette would produce a value that is never displayed.
    pub value: String,
    pub line: u32,
}

/// Why a (theme, property) pair has no resolved value.
///
/// Every variant names the thing that could not be determined. A caller that
/// prints these is telling the truth about the sheet; a caller that substitutes
/// a default for any of them has reintroduced the guess this module exists to
/// remove.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum Unresolvable {
    /// The selector asked about declares nothing anywhere in the sheet, so there
    /// is no scope to resolve against. Distinct from a theme that exists and is
    /// silent about this property.
    #[error(
        "no rule in this stylesheet has the selector {selector}, so there is no scope to resolve against"
    )]
    UnknownTheme { selector: String },

    /// Neither the theme nor the base declares the property unconditionally.
    #[error(
        "{property} is not declared in {selector} or in the base scope, so the sheet gives it no value here"
    )]
    NotDeclared { selector: String, property: String },

    /// A `var()` named a custom property that no scope in reach declares, and
    /// there was no fallback to take.
    #[error(
        "{name} is referenced but declared in no scope reachable from {selector}, and the reference has no fallback"
    )]
    UndefinedVariable { selector: String, name: String },

    /// The property references itself, directly or through a chain.
    ///
    /// CSS makes the whole cycle guaranteed-invalid and then takes fallbacks.
    /// This module refuses, so that the defect is reported rather than papered
    /// over with whichever fallback happens to be nearest.
    #[error("{} is a dependency cycle, so no value can be determined without guessing", .path.join(" -> "))]
    Cycle { path: Vec<String> },

    /// The chain of references was longer than [`MAX_RESOLUTION_DEPTH`].
    #[error(
        "resolving {property} followed more than {limit} custom-property references without terminating"
    )]
    DepthExceeded { property: String, limit: usize },

    /// Substitution produced more than [`MAX_EXPANSION_CHARS`] characters.
    #[error(
        "resolving {property} expanded past {limit} characters, which a stylesheet value does not legitimately do"
    )]
    ExpansionTooLarge { property: String, limit: usize },

    /// One scope declares the property in two different cascade layers, and
    /// which one wins depends on layer ORDER.
    ///
    /// Two parts of the cascade-layer rules are exact and are applied:
    /// an unlayered normal declaration beats every layered one, and an
    /// `!important` layered declaration beats an unlayered one. The remaining
    /// part is not: ordering between two NAMED layers depends on the order they
    /// were first declared and on any `@layer a, b;` statement, and this module
    /// does not model either. Rather than fall back to source order, which is
    /// the wrong answer whenever an earlier layer is declared later in the file,
    /// it refuses.
    #[error(
        "{property} is declared in {selector} in more than one cascade layer ({}), and which layer wins depends on layer order, which this module does not model",
        .layers.join(", ")
    )]
    LayerConflict {
        selector: String,
        property: String,
        layers: Vec<String>,
    },

    /// The value could not be read as CSS at all.
    #[error("the value of {property} could not be read: {detail}")]
    MalformedValue { property: String, detail: String },
}

/// The result of resolving one (theme, property) pair: a value, or a reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// The fully substituted value string.
    Value(String),
    /// Why there is no value.
    Unresolvable(Unresolvable),
}

/// What the sheet says about one property in one theme.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resolution {
    /// The theme selector asked about.
    pub theme: String,
    /// The property asked about.
    pub property: String,
    /// The value, or the reason there is none.
    pub outcome: Outcome,
    /// Declarations for this property that a conditional at-rule guards, and
    /// that were therefore NOT applied.
    ///
    /// A non-empty list means the resolved value is the unconditional one and
    /// the property takes a different value under some condition. A gate that
    /// ignores this checks a viewport the user may never be in.
    pub conditional: Vec<ConditionalValue>,
}

impl Resolution {
    /// The resolved value, or `None` where the pair was unresolvable.
    pub fn value(&self) -> Option<&str> {
        match &self.outcome {
            Outcome::Value(value) => Some(value),
            Outcome::Unresolvable(_) => None,
        }
    }

    /// The reason there is no value, or `None` where there is one.
    pub fn reason(&self) -> Option<&Unresolvable> {
        match &self.outcome {
            Outcome::Value(_) => None,
            Outcome::Unresolvable(reason) => Some(reason),
        }
    }

    /// Whether a value was determined.
    pub fn is_resolved(&self) -> bool {
        matches!(self.outcome, Outcome::Value(_))
    }
}

/// Every custom property a stylesheet declares, filed by scope, with the theme
/// scopes identified.
#[derive(Debug, Clone)]
pub struct Sheet {
    declarations: Vec<Declaration>,
    /// Selector to the indices in `declarations` it owns, in source order.
    scopes: BTreeMap<String, Vec<usize>>,
    themes: Vec<Theme>,
    base: Option<String>,
    malformed: Option<String>,
}

impl Sheet {
    /// Read a stylesheet.
    ///
    /// This never fails, because a parse error in one place must not be allowed
    /// to hide what the rest of the sheet says. It reports structural damage on
    /// [`Sheet::malformed`] instead, which a caller must treat as fatal for the
    /// same reason `vds-scan` does: a declaration the scanner did not SEE is not
    /// skipped and not counted, it simply does not exist, and every count
    /// downstream looks healthy while the gate reads a file it never read.
    pub fn parse(source: &str) -> Self {
        let original: Vec<char> = source.chars().collect();
        let (blank, mask, mut malformed) = blank_non_code(&original);
        let line_starts = line_starts(&blank);
        let scanned = Scanned {
            original: &original,
            mask: &mask,
            blank: &blank,
            line_starts: &line_starts,
        };

        let mut declarations: Vec<Declaration> = Vec::new();
        let mut stack: Vec<Frame> = Vec::new();
        let mut anonymous_layers = 0usize;
        // Depth of braces that belong to a custom property's VALUE rather than
        // to a nested rule. `--grid: { a: b };` is legal: a custom property
        // accepts almost any balanced token stream, braces included. Treating
        // that `{` as opening a rule pushes a phantom scope and files every
        // later declaration one level too deep.
        let mut value_braces = 0usize;
        let mut buf_start = 0usize;

        for (i, structural) in blank.iter().enumerate() {
            match *structural {
                '{' => {
                    if value_braces > 0 {
                        value_braces += 1;
                        continue;
                    }
                    if in_style_scope(&stack) && starts_custom_property(&scanned, buf_start, i) {
                        value_braces = 1;
                        continue;
                    }
                    let prelude = normalise(&original, &mask, buf_start, i);
                    stack.push(Frame::open(&prelude, &mut anonymous_layers));
                    buf_start = i + 1;
                }
                '}' => {
                    if value_braces > 0 {
                        value_braces -= 1;
                        continue;
                    }
                    flush(&scanned, &stack, buf_start, i, &mut declarations);
                    if stack.pop().is_none() && malformed.is_none() {
                        malformed = Some(format!(
                            "a closing brace at line {} has no matching open block, so every rule \
                             after it may be attributed to the wrong selector",
                            line_of(&line_starts, i)
                        ));
                    }
                    buf_start = i + 1;
                }
                ';' => {
                    if value_braces > 0 {
                        continue;
                    }
                    flush(&scanned, &stack, buf_start, i, &mut declarations);
                    buf_start = i + 1;
                }
                _ => {}
            }
        }

        if !stack.is_empty() && malformed.is_none() {
            malformed = Some(format!(
                "{} block(s) were opened and never closed, so the end of the sheet was read \
                 inside a rule and its declarations may be attributed to the wrong selector",
                stack.len()
            ));
        }

        let mut scopes: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        let mut first_seen: BTreeMap<String, usize> = BTreeMap::new();
        for (index, declaration) in declarations.iter().enumerate() {
            scopes
                .entry(declaration.selector.clone())
                .or_default()
                .push(index);
            first_seen
                .entry(declaration.selector.clone())
                .or_insert(index);
        }

        let base = scopes.contains_key(":root").then(|| ":root".to_owned());
        let themes = discover_themes(&declarations, &scopes, &first_seen, base.as_deref());

        Sheet {
            declarations,
            scopes,
            themes,
            base,
            malformed,
        }
    }

    /// Structural damage that makes the extraction untrustworthy, if any.
    ///
    /// A caller must treat this as fatal rather than proceeding on a partial
    /// read.
    pub fn malformed(&self) -> Option<&str> {
        self.malformed.as_deref()
    }

    /// Every custom-property declaration in the sheet, in source order.
    pub fn declarations(&self) -> &[Declaration] {
        &self.declarations
    }

    /// Every selector that declares at least one custom property.
    pub fn scope_selectors(&self) -> Vec<&str> {
        self.scopes.keys().map(String::as_str).collect()
    }

    /// The declarations one selector owns, in source order.
    pub fn declarations_in(&self, selector: &str) -> Vec<&Declaration> {
        self.scopes
            .get(selector)
            .map(|indices| indices.iter().map(|i| &self.declarations[*i]).collect())
            .unwrap_or_default()
    }

    /// The theme scopes found in this sheet, base first.
    ///
    /// Discovered from the sheet, never from a list of names. A scope is a theme
    /// when its selector is root-like (built only from `:root`, `html`, `body`,
    /// classes and attribute selectors, with no combinator, so it can match the
    /// document root or a wrapper rather than a component) AND it redeclares at
    /// least one property the base declares. The second half is what separates a
    /// theme from a component that happens to own custom properties:
    /// `.onyx-sidebar__row[data-module='matters']` is root-like by shape and
    /// declares `--module-tint`, which `:root` never mentions, so it is not a
    /// palette.
    ///
    /// Where the sheet has no `:root` there is no base to overlap with, so every
    /// root-like scope that declares a custom property is reported. With no
    /// base, each palette scope stands alone and there is nothing to inherit.
    pub fn themes(&self) -> &[Theme] {
        &self.themes
    }

    /// The theme selectors, base first.
    pub fn theme_selectors(&self) -> Vec<&str> {
        self.themes.iter().map(|t| t.selector.as_str()).collect()
    }

    /// The base scope's selector, where the sheet has one.
    pub fn base_selector(&self) -> Option<&str> {
        self.base.as_deref()
    }

    /// Every property name declared in any theme scope, sorted.
    ///
    /// This is the resolvable surface: the set a caller iterates to ask what
    /// each theme says.
    pub fn theme_properties(&self) -> Vec<&str> {
        let selectors: BTreeSet<&str> = self.theme_selectors().into_iter().collect();
        let mut names: BTreeSet<&str> = BTreeSet::new();
        for declaration in &self.declarations {
            if selectors.contains(declaration.selector.as_str()) {
                names.insert(declaration.property.as_str());
            }
        }
        names.into_iter().collect()
    }

    /// Scopes that redeclare part of the base palette but were NOT classed as
    /// themes.
    ///
    /// This exists because the dangerous direction is a theme that is not
    /// discovered: a palette the gate never asks about is a palette that can
    /// ship at 1.15:1 with every proof green. `:root:not(.compact)` and
    /// `.dark .panel` are both plausible in a real sheet and neither is
    /// root-like, so both would be dropped in silence. A caller should refuse to
    /// certify a sheet where this list is not empty, or widen the register to
    /// name them.
    pub fn unclassified_palette_scopes(&self) -> Vec<&str> {
        let Some(base) = self.base.as_deref() else {
            return Vec::new();
        };
        let base_properties: BTreeSet<&str> = self
            .declarations_in(base)
            .into_iter()
            .map(|d| d.property.as_str())
            .collect();
        let themes: BTreeSet<&str> = self.theme_selectors().into_iter().collect();
        let mut out: Vec<&str> = self
            .scopes
            .keys()
            .map(String::as_str)
            .filter(|selector| !themes.contains(selector))
            .filter(|selector| {
                self.declarations_in(selector)
                    .iter()
                    .any(|d| base_properties.contains(d.property.as_str()))
            })
            .collect();
        out.sort_unstable();
        out
    }

    /// What one theme says about one property.
    pub fn resolve(&self, theme: &str, property: &str) -> Resolution {
        let conditional = self.conditional_values(theme, property);
        let outcome = if !self.scopes.contains_key(theme) {
            Outcome::Unresolvable(Unresolvable::UnknownTheme {
                selector: theme.to_owned(),
            })
        } else {
            match self.winner(theme, property) {
                Err(reason) => Outcome::Unresolvable(reason),
                Ok(None) => Outcome::Unresolvable(Unresolvable::NotDeclared {
                    selector: theme.to_owned(),
                    property: property.to_owned(),
                }),
                Ok(Some(declaration)) => {
                    let mut context = Context {
                        sheet: self,
                        theme,
                        stack: vec![property.to_owned()],
                        produced: 0,
                        origin: property.to_owned(),
                    };
                    match context.substitute(&declaration.value, 1) {
                        Ok(value) => Outcome::Value(value),
                        Err(reason) => Outcome::Unresolvable(reason),
                    }
                }
            }
        };
        Resolution {
            theme: theme.to_owned(),
            property: property.to_owned(),
            outcome,
            conditional,
        }
    }

    /// Every property in the theme surface, resolved for one theme.
    pub fn resolve_theme(&self, theme: &str) -> BTreeMap<String, Resolution> {
        self.theme_properties()
            .into_iter()
            .map(|property| (property.to_owned(), self.resolve(theme, property)))
            .collect()
    }

    /// Resolve an arbitrary value string in a theme's context.
    ///
    /// The caller needs this because the interesting values are not only the
    /// custom properties themselves: `border: 1px dashed var(--border-control)`
    /// is where a control boundary is actually set, and the gate has to know
    /// what that `var()` becomes in each theme.
    pub fn resolve_value(&self, theme: &str, value: &str) -> Outcome {
        if !self.scopes.contains_key(theme) {
            return Outcome::Unresolvable(Unresolvable::UnknownTheme {
                selector: theme.to_owned(),
            });
        }
        let mut context = Context {
            sheet: self,
            theme,
            stack: Vec::new(),
            produced: 0,
            origin: "<value>".to_owned(),
        };
        match context.substitute(value, 0) {
            Ok(resolved) => Outcome::Value(resolved),
            Err(reason) => Outcome::Unresolvable(reason),
        }
    }

    /// The winning unconditional declaration for a property in a scope, falling
    /// back to the base scope.
    fn winner(&self, theme: &str, property: &str) -> Result<Option<&Declaration>, Unresolvable> {
        if let Some(found) = self.winner_in(theme, property)? {
            return Ok(Some(found));
        }
        match self.base.as_deref() {
            Some(base) if base != theme => self.winner_in(base, property),
            _ => Ok(None),
        }
    }

    /// The winning unconditional declaration for a property within ONE scope.
    ///
    /// Two parts of the cascade decide this, and both change the answer:
    /// `!important` beats normal, and an unlayered normal declaration beats
    /// every layered one however early it appears. Source order only settles
    /// what is left.
    fn winner_in(&self, scope: &str, property: &str) -> Result<Option<&Declaration>, Unresolvable> {
        let mut candidates: Vec<&Declaration> = self
            .declarations_in(scope)
            .into_iter()
            .filter(|d| d.property == property)
            .filter(|d| d.conditions.is_empty())
            .collect();
        if candidates.is_empty() {
            return Ok(None);
        }

        if candidates.iter().any(|d| d.important) {
            candidates.retain(|d| d.important);
            // Reversed for important declarations: a layered !important beats an
            // unlayered one (CSS Cascade 5, "Cascade Layers").
            if candidates.iter().any(|d| d.layer.is_some()) {
                candidates.retain(|d| d.layer.is_some());
            }
        } else if candidates.iter().any(|d| d.layer.is_none()) {
            candidates.retain(|d| d.layer.is_none());
        }

        let layers: BTreeSet<&str> = candidates
            .iter()
            .map(|d| d.layer.as_deref().unwrap_or(""))
            .collect();
        if layers.len() > 1 {
            return Err(Unresolvable::LayerConflict {
                selector: scope.to_owned(),
                property: property.to_owned(),
                layers: layers
                    .into_iter()
                    .map(|layer| {
                        if layer.is_empty() {
                            "<unlayered>".to_owned()
                        } else {
                            layer.to_owned()
                        }
                    })
                    .collect(),
            });
        }

        Ok(candidates.into_iter().max_by_key(|d| d.order))
    }

    fn conditional_values(&self, theme: &str, property: &str) -> Vec<ConditionalValue> {
        let mut scopes: Vec<&str> = vec![theme];
        if let Some(base) = self.base.as_deref()
            && base != theme
        {
            scopes.push(base);
        }
        let mut out: Vec<ConditionalValue> = scopes
            .into_iter()
            .flat_map(|scope| self.declarations_in(scope))
            .filter(|d| d.property == property && !d.conditions.is_empty())
            .map(|d| ConditionalValue {
                selector: d.selector.clone(),
                conditions: d.conditions.clone(),
                value: d.value.clone(),
                line: d.line,
            })
            .collect();
        out.sort_by_key(|c| c.line);
        out
    }
}

/// One resolution in progress, carrying what stops it running away.
struct Context<'a> {
    sheet: &'a Sheet,
    theme: &'a str,
    /// The properties currently being resolved, innermost last. This IS the
    /// cycle detector, and it is also the cycle path that gets reported.
    stack: Vec<String>,
    produced: usize,
    /// The property the caller asked about, named in depth and budget refusals
    /// so the message points at something the caller recognises.
    origin: String,
}

impl Context<'_> {
    fn resolve_property(&mut self, property: &str, depth: usize) -> Result<String, Unresolvable> {
        if self.stack.iter().any(|seen| seen == property) {
            let mut path: Vec<String> = self
                .stack
                .iter()
                .skip_while(|seen| seen.as_str() != property)
                .cloned()
                .collect();
            path.push(property.to_owned());
            return Err(Unresolvable::Cycle { path });
        }
        if depth >= MAX_RESOLUTION_DEPTH {
            return Err(Unresolvable::DepthExceeded {
                property: self.origin.clone(),
                limit: MAX_RESOLUTION_DEPTH,
            });
        }
        let declaration = match self.sheet.winner(self.theme, property)? {
            Some(declaration) => declaration,
            None => {
                return Err(Unresolvable::UndefinedVariable {
                    selector: self.theme.to_owned(),
                    name: property.to_owned(),
                });
            }
        };
        // Cloned because the borrow of `self.sheet` cannot be held across the
        // recursive call that mutates `self.stack`.
        let value = declaration.value.clone();
        self.stack.push(property.to_owned());
        let resolved = self.substitute(&value, depth + 1);
        self.stack.pop();
        resolved
    }

    /// Replace every `var()` in a value, in this theme's context.
    fn substitute(&mut self, value: &str, depth: usize) -> Result<String, Unresolvable> {
        let chars: Vec<char> = value.chars().collect();
        let mut out = String::new();
        let mut i = 0usize;
        while i < chars.len() {
            let ch = chars[i];
            if ch == '"' || ch == '\'' {
                let end = end_of_string(&chars, i);
                out.extend(&chars[i..end]);
                i = end;
                continue;
            }
            if !opens_var(&chars, i) {
                out.push(ch);
                i += 1;
                continue;
            }
            let open = i + 3;
            let Some(close) = matching_paren(&chars, open) else {
                return Err(Unresolvable::MalformedValue {
                    property: self.origin.clone(),
                    detail: "a var( was opened and never closed".to_owned(),
                });
            };
            let (name, fallback) = split_var_arguments(&chars, open + 1, close);
            if !name.starts_with("--") {
                return Err(Unresolvable::MalformedValue {
                    property: self.origin.clone(),
                    detail: format!(
                        "var({name}) does not name a custom property; var()'s first argument must \
                         begin with two dashes"
                    ),
                });
            }
            let substituted = match self.resolve_property(&name, depth) {
                Ok(resolved) => resolved,
                // CSS Variables Level 1 §3.1: a custom property that is not
                // declared, or whose own value fails to substitute, holds the
                // guaranteed-invalid value, and a var() referencing it takes its
                // fallback. Anything else is this module refusing, and a refusal
                // must not be swallowed by a fallback: a cycle stays reported.
                Err(undefined @ Unresolvable::UndefinedVariable { .. }) => match fallback {
                    Some(fallback) => self.substitute(&fallback, depth + 1)?,
                    // Re-raised unchanged rather than rebuilt around `name`, so
                    // the reason names the property that is declared nowhere and
                    // not whichever property happened to reference it. In
                    // `--a: var(--b)` over `--b: var(--c)`, the defect is `--c`.
                    None => return Err(undefined),
                },
                Err(other) => return Err(other),
            };
            self.produced += substituted.chars().count();
            if self.produced > MAX_EXPANSION_CHARS {
                return Err(Unresolvable::ExpansionTooLarge {
                    property: self.origin.clone(),
                    limit: MAX_EXPANSION_CHARS,
                });
            }
            out.push_str(&substituted);
            i = close + 1;
        }
        Ok(collapse_outside_strings(&out))
    }
}

/// What kind of block a `{` opened.
enum Frame {
    Style {
        selectors: Vec<String>,
    },
    Layer {
        name: String,
    },
    Conditional {
        prelude: String,
    },
    /// `@keyframes`, `@font-face`, `@page`, `@property` and anything else. Its
    /// direct children are not a scope, so declarations sitting straight inside
    /// one are not recorded against any selector.
    Other,
}

impl Frame {
    fn open(prelude: &str, anonymous_layers: &mut usize) -> Frame {
        if !prelude.starts_with('@') {
            return Frame::Style {
                selectors: split_selector_list(prelude),
            };
        }
        let keyword: String = prelude
            .chars()
            .take_while(|c| !c.is_whitespace() && *c != '(')
            .collect::<String>()
            .to_ascii_lowercase();
        match keyword.as_str() {
            "@layer" => {
                let name = prelude["@layer".len()..].trim();
                if name.is_empty() {
                    // Two anonymous layers are two DIFFERENT layers, so they must
                    // not share a name, or a conflict between them would be
                    // mistaken for a conflict within one.
                    *anonymous_layers += 1;
                    Frame::Layer {
                        name: format!("<anonymous {anonymous_layers}>"),
                    }
                } else {
                    Frame::Layer {
                        name: name.to_owned(),
                    }
                }
            }
            "@media" | "@supports" | "@container" => Frame::Conditional {
                prelude: prelude.to_owned(),
            },
            _ => Frame::Other,
        }
    }
}

fn in_style_scope(stack: &[Frame]) -> bool {
    matches!(stack.last(), Some(Frame::Style { .. }))
}

/// The parallel views of the source that the structural walk needs at once: the
/// text as written, what each character IS, the text with comments and string
/// bodies blanked, and where the lines begin.
///
/// All four are indexed by the same character offset, which is the property that
/// lets a block boundary be found in the blanked text while the selector and the
/// value are read out of the original.
struct Scanned<'a> {
    original: &'a [char],
    mask: &'a [Mask],
    blank: &'a [char],
    line_starts: &'a [usize],
}

/// Record a declaration against every selector of the innermost style block.
fn flush(scanned: &Scanned, stack: &[Frame], start: usize, end: usize, out: &mut Vec<Declaration>) {
    let Some(Frame::Style { selectors }) = stack.last() else {
        return;
    };
    let Scanned {
        original,
        mask,
        blank,
        line_starts,
    } = *scanned;
    let (start, end) = trim(original, mask, start, end);
    if start == end {
        return;
    }
    let Some(colon) = find_top_level(blank, start, end, ':') else {
        return;
    };
    let (name_start, name_end) = trim(original, mask, start, colon);
    let name = normalise(original, mask, name_start, name_end);
    if !is_custom_property_name(&name) {
        return;
    }
    let (value, important) = split_important(&normalise(original, mask, colon + 1, end));

    let layer = layer_path(stack);
    let conditions = conditions(stack);
    let line = line_of(line_starts, name_start);
    for selector in selectors {
        out.push(Declaration {
            property: name.clone(),
            value: value.clone(),
            important,
            selector: selector.clone(),
            layer: layer.clone(),
            conditions: conditions.clone(),
            line,
            order: out.len(),
        });
    }
}

fn layer_path(stack: &[Frame]) -> Option<String> {
    let names: Vec<&str> = stack
        .iter()
        .filter_map(|frame| match frame {
            Frame::Layer { name } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    (!names.is_empty()).then(|| names.join("."))
}

fn conditions(stack: &[Frame]) -> Vec<String> {
    stack
        .iter()
        .filter_map(|frame| match frame {
            Frame::Conditional { prelude } => Some(prelude.clone()),
            _ => None,
        })
        .collect()
}

/// Whether the pending buffer is the start of `--name:`, which decides whether a
/// `{` belongs to a value or opens a rule.
fn starts_custom_property(scanned: &Scanned, start: usize, end: usize) -> bool {
    let Scanned {
        original,
        mask,
        blank,
        ..
    } = *scanned;
    let (start, end) = trim(original, mask, start, end);
    let Some(colon) = find_top_level(blank, start, end, ':') else {
        return false;
    };
    let (name_start, name_end) = trim(original, mask, start, colon);
    is_custom_property_name(&normalise(original, mask, name_start, name_end))
}

fn is_custom_property_name(name: &str) -> bool {
    // The bare `--` is a legal custom property name in the current
    // specification, but nothing in a design system declares it, and accepting
    // it would let a stray `--:` in a malformed value register as a property.
    name.len() > 2
        && name.starts_with("--")
        && name[2..]
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || !c.is_ascii())
}

/// Split a trailing `!important` off a value.
///
/// Not cosmetic: `--ag-cell-horizontal-padding: 0 !important` appears in the
/// subject stylesheet, and leaving the flag in the value hands a downstream
/// parser the string `0 !important` to make sense of, while dropping the flag
/// silently loses the fact that this declaration beats every normal one.
fn split_important(value: &str) -> (String, bool) {
    const KEYWORD: &str = "important";
    let trimmed = value.trim_end();
    // The keyword is ASCII case-insensitive in CSS, so the suffix is compared
    // that way rather than matched literally.
    if trimmed.len() < KEYWORD.len() || !trimmed.is_char_boundary(trimmed.len() - KEYWORD.len()) {
        return (value.to_owned(), false);
    }
    let (head, tail) = trimmed.split_at(trimmed.len() - KEYWORD.len());
    if !tail.eq_ignore_ascii_case(KEYWORD) {
        return (value.to_owned(), false);
    }
    match head.trim_end().strip_suffix('!') {
        Some(head) => (head.trim_end().to_owned(), true),
        // `very-important` ends in the keyword and is an ordinary value.
        None => (value.to_owned(), false),
    }
}

/// Split a selector list on its top-level commas.
///
/// `:root, [data-theme='light'] { --x: 1 }` establishes `--x` for two scopes.
/// Filing it under the joined text `:root, [data-theme='light']` files it under
/// neither, and both themes then resolve `--x` from somewhere else or not at
/// all.
fn split_selector_list(prelude: &str) -> Vec<String> {
    let chars: Vec<char> = prelude.chars().collect();
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (i, ch) in chars.iter().enumerate() {
        match ch {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(chars[start..i].iter().collect::<String>().trim().to_owned());
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(chars[start..].iter().collect::<String>().trim().to_owned());
    parts.retain(|part| !part.is_empty());
    parts
}

/// Whether a selector can match the document root or a theme wrapper.
///
/// Built only from `:root`, `html`, `body`, classes and attribute selectors,
/// with no combinator and no other pseudo-class. Any other element name is a
/// component, not a document root, and a combinator means the rule matches
/// something INSIDE a scope rather than being one.
fn is_root_like(selector: &str) -> bool {
    let chars: Vec<char> = selector.chars().collect();
    let mut i = 0usize;
    let mut tokens = 0usize;
    while i < chars.len() {
        match chars[i] {
            ':' => {
                let rest: String = chars[i..].iter().collect();
                if !rest.starts_with(":root") {
                    return false;
                }
                if chars
                    .get(i + 5)
                    .is_some_and(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                {
                    return false;
                }
                i += 5;
            }
            '.' => {
                let start = i + 1;
                let mut end = start;
                while end < chars.len()
                    && (chars[end].is_alphanumeric() || chars[end] == '-' || chars[end] == '_')
                {
                    end += 1;
                }
                if end == start {
                    return false;
                }
                i = end;
            }
            '[' => match chars[i..].iter().position(|c| *c == ']') {
                Some(offset) => i += offset + 1,
                None => return false,
            },
            c if c.is_ascii_alphabetic() => {
                let start = i;
                let mut end = i;
                while end < chars.len() && chars[end].is_ascii_alphabetic() {
                    end += 1;
                }
                let name: String = chars[start..end].iter().collect::<String>().to_lowercase();
                if name != "html" && name != "body" {
                    return false;
                }
                i = end;
            }
            _ => return false,
        }
        tokens += 1;
    }
    tokens > 0
}

fn discover_themes(
    declarations: &[Declaration],
    scopes: &BTreeMap<String, Vec<usize>>,
    first_seen: &BTreeMap<String, usize>,
    base: Option<&str>,
) -> Vec<Theme> {
    let base_properties: BTreeSet<&str> = match base {
        Some(base) => scopes
            .get(base)
            .map(|indices| {
                indices
                    .iter()
                    .map(|i| declarations[*i].property.as_str())
                    .collect()
            })
            .unwrap_or_default(),
        None => BTreeSet::new(),
    };

    let mut found: Vec<Theme> = scopes
        .keys()
        .filter(|selector| is_root_like(selector))
        .filter(|selector| {
            if Some(selector.as_str()) == base {
                return true;
            }
            if base.is_none() {
                return true;
            }
            scopes[*selector]
                .iter()
                .any(|i| base_properties.contains(declarations[*i].property.as_str()))
        })
        .map(|selector| Theme {
            selector: selector.clone(),
            is_base: Some(selector.as_str()) == base,
        })
        .collect();

    found.sort_by_key(|theme| {
        (
            !theme.is_base,
            first_seen
                .get(&theme.selector)
                .copied()
                .unwrap_or(usize::MAX),
        )
    });
    found
}

/// What each character of the source is, so a value can be reassembled from the
/// original text without reassembling the comments too.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mask {
    Code,
    Comment,
    /// A string delimiter or a character inside a string. Emitted verbatim,
    /// because a space inside `content: "a  b"` is content and not separation.
    StringPart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Region {
    Code,
    Comment,
    SingleQuote,
    DoubleQuote,
}

/// Blank every comment and string body, preserving character offsets.
///
/// Preserving offsets is what lets the structural walk run on the blanked text
/// while the recorded selector and value are sliced out of the ORIGINAL, so a
/// `{`, `;` or `:` that is really content never moves a block boundary and the
/// text that gets recorded is still the text the author wrote.
///
/// CSS has no `//` line comment. Treating one as a comment would blank the rest
/// of the line of `@import url('https://fonts.googleapis.com/...')`, which is
/// line 1 of the subject stylesheet.
fn blank_non_code(chars: &[char]) -> (Vec<char>, Vec<Mask>, Option<String>) {
    let mut blank: Vec<char> = Vec::with_capacity(chars.len());
    let mut mask: Vec<Mask> = Vec::with_capacity(chars.len());
    let mut region = Region::Code;
    let mut i = 0usize;

    let push = |blank: &mut Vec<char>, mask: &mut Vec<Mask>, ch: char, kind: Mask| {
        blank.push(ch);
        mask.push(kind);
    };

    while i < chars.len() {
        let ch = chars[i];
        let next = chars.get(i + 1).copied();
        match region {
            Region::Code => {
                if ch == '/' && next == Some('*') {
                    region = Region::Comment;
                    push(&mut blank, &mut mask, ' ', Mask::Comment);
                    push(&mut blank, &mut mask, ' ', Mask::Comment);
                    i += 2;
                    continue;
                }
                if ch == '\'' || ch == '"' {
                    region = if ch == '\'' {
                        Region::SingleQuote
                    } else {
                        Region::DoubleQuote
                    };
                    push(&mut blank, &mut mask, ' ', Mask::StringPart);
                    i += 1;
                    continue;
                }
                if ch == '\\' {
                    // A CSS escape hides the next character from the structural
                    // walk. `.print\:hidden` is a real selector in the subject
                    // stylesheet and its colon is part of a class name, not the
                    // separator of a declaration.
                    push(&mut blank, &mut mask, ' ', Mask::Code);
                    if let Some(escaped) = next {
                        push(
                            &mut blank,
                            &mut mask,
                            if escaped == '\n' { '\n' } else { ' ' },
                            Mask::Code,
                        );
                    }
                    i += 2;
                    continue;
                }
                push(&mut blank, &mut mask, ch, Mask::Code);
                i += 1;
            }
            Region::Comment => {
                if ch == '*' && next == Some('/') {
                    region = Region::Code;
                    push(&mut blank, &mut mask, ' ', Mask::Comment);
                    push(&mut blank, &mut mask, ' ', Mask::Comment);
                    i += 2;
                    continue;
                }
                push(
                    &mut blank,
                    &mut mask,
                    if ch == '\n' { '\n' } else { ' ' },
                    Mask::Comment,
                );
                i += 1;
            }
            Region::SingleQuote | Region::DoubleQuote => {
                let closer = if region == Region::SingleQuote {
                    '\''
                } else {
                    '"'
                };
                if ch == '\\' {
                    push(&mut blank, &mut mask, ' ', Mask::StringPart);
                    if let Some(escaped) = next {
                        push(
                            &mut blank,
                            &mut mask,
                            if escaped == '\n' { '\n' } else { ' ' },
                            Mask::StringPart,
                        );
                    }
                    i += 2;
                    continue;
                }
                if ch == closer {
                    region = Region::Code;
                    push(&mut blank, &mut mask, ' ', Mask::StringPart);
                    i += 1;
                    continue;
                }
                if ch == '\n' {
                    // A raw newline ends a CSS string (it produces a
                    // bad-string-token). Carrying the string on would blank the
                    // rest of the file, and every declaration in it would vanish
                    // rather than be reported.
                    region = Region::Code;
                    push(&mut blank, &mut mask, '\n', Mask::Code);
                    i += 1;
                    continue;
                }
                push(&mut blank, &mut mask, ' ', Mask::StringPart);
                i += 1;
            }
        }
    }

    let malformed = match region {
        Region::Code => None,
        Region::Comment => Some(
            "a comment was opened and never closed, so every declaration after it was blanked \
             and is missing from this extraction entirely"
                .to_owned(),
        ),
        Region::SingleQuote | Region::DoubleQuote => Some(
            "a string was opened and never closed at the end of the sheet, so the text after it \
             was blanked"
                .to_owned(),
        ),
    };
    (blank, mask, malformed)
}

/// Rebuild a span of the ORIGINAL text with comments treated as separation and
/// whitespace outside strings collapsed.
///
/// Collapsing is safe because whitespace between CSS tokens carries no meaning
/// beyond separation, and it is necessary because values in the subject
/// stylesheet run over several lines with indentation. The exception is a
/// string, whose interior is content, so strings are copied character for
/// character.
fn normalise(original: &[char], mask: &[Mask], start: usize, end: usize) -> String {
    let mut out = String::new();
    let mut pending = false;
    for (offset, kind) in mask[start..end].iter().enumerate() {
        let ch = original[start + offset];
        match kind {
            Mask::Comment => pending = true,
            Mask::StringPart => {
                if pending && !out.is_empty() {
                    out.push(' ');
                }
                pending = false;
                out.push(ch);
            }
            Mask::Code => {
                if ch.is_whitespace() {
                    pending = true;
                } else {
                    if pending && !out.is_empty() {
                        out.push(' ');
                    }
                    pending = false;
                    out.push(ch);
                }
            }
        }
    }
    out
}

/// Collapse whitespace runs outside strings, for text that has already been
/// through [`normalise`] once and then had values substituted into it.
fn collapse_outside_strings(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut pending = false;
    let mut i = 0usize;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '"' || ch == '\'' {
            if pending && !out.is_empty() {
                out.push(' ');
            }
            pending = false;
            let end = end_of_string(&chars, i);
            out.extend(&chars[i..end]);
            i = end;
            continue;
        }
        if ch.is_whitespace() {
            pending = true;
        } else {
            if pending && !out.is_empty() {
                out.push(' ');
            }
            pending = false;
            out.push(ch);
        }
        i += 1;
    }
    out
}

/// One past the closing quote of the string starting at `open`, or the end of
/// the input where it is unterminated.
fn end_of_string(chars: &[char], open: usize) -> usize {
    let quote = chars[open];
    let mut i = open + 1;
    while i < chars.len() {
        if chars[i] == '\\' {
            i += 2;
            continue;
        }
        if chars[i] == quote {
            return i + 1;
        }
        i += 1;
    }
    chars.len()
}

/// Whether a `var(` function starts at `i`.
///
/// The name is matched case-insensitively because CSS function names are ASCII
/// case-insensitive, and the preceding character is checked so that
/// `--my-var(x)` and `avar(` are not mistaken for one.
fn opens_var(chars: &[char], i: usize) -> bool {
    if i + 3 >= chars.len() {
        return false;
    }
    if !chars[i].eq_ignore_ascii_case(&'v')
        || !chars[i + 1].eq_ignore_ascii_case(&'a')
        || !chars[i + 2].eq_ignore_ascii_case(&'r')
        || chars[i + 3] != '('
    {
        return false;
    }
    if i > 0 {
        let previous = chars[i - 1];
        if previous.is_alphanumeric() || previous == '-' || previous == '_' {
            return false;
        }
    }
    true
}

/// The index of the `)` matching the `(` at `open`, respecting strings.
fn matching_paren(chars: &[char], open: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut i = open;
    while i < chars.len() {
        match chars[i] {
            '"' | '\'' => {
                i = end_of_string(chars, i);
                continue;
            }
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// The custom property name and the fallback of a `var()`, given the span
/// between its parentheses.
///
/// The fallback is everything after the FIRST top-level comma, taken verbatim.
/// CSS Variables Level 1 §3 is explicit about this:
/// `var(--my-var, --my-background, pink)` has the single fallback
/// `--my-background, pink`, not two arguments. `Some("")` is an empty fallback,
/// which is legal and is not the same thing as `None`.
fn split_var_arguments(chars: &[char], start: usize, end: usize) -> (String, Option<String>) {
    let mut depth = 0i32;
    let mut i = start;
    while i < end {
        match chars[i] {
            '"' | '\'' => {
                i = end_of_string(chars, i).min(end);
                continue;
            }
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            ',' if depth == 0 => {
                let name: String = chars[start..i].iter().collect::<String>().trim().to_owned();
                let fallback: String = chars[i + 1..end]
                    .iter()
                    .collect::<String>()
                    .trim()
                    .to_owned();
                return (name, Some(fallback));
            }
            _ => {}
        }
        i += 1;
    }
    (
        chars[start..end]
            .iter()
            .collect::<String>()
            .trim()
            .to_owned(),
        None,
    )
}

fn find_top_level(chars: &[char], start: usize, end: usize, needle: char) -> Option<usize> {
    let mut depth = 0i32;
    for (offset, ch) in chars[start..end].iter().enumerate() {
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            c if *c == needle && depth == 0 => return Some(start + offset),
            _ => {}
        }
    }
    None
}

/// Narrow a span past leading and trailing separation.
///
/// A comment is separation and a space between tokens is separation. A space
/// INSIDE a string is neither, and trimming it is not cosmetic: the blanked text
/// renders `--x: "two  spaces"` as a run of spaces, so a trim that read the
/// blanked text alone would trim the whole value away and report the property as
/// empty. The mask is what keeps the two apart.
fn trim(original: &[char], mask: &[Mask], mut start: usize, mut end: usize) -> (usize, usize) {
    let separation = |i: usize| match mask[i] {
        Mask::Comment => true,
        Mask::StringPart => false,
        Mask::Code => original[i].is_whitespace(),
    };
    while start < end && separation(start) {
        start += 1;
    }
    while end > start && separation(end - 1) {
        end -= 1;
    }
    (start, end)
}

fn line_starts(chars: &[char]) -> Vec<usize> {
    let mut starts = vec![0usize];
    for (index, ch) in chars.iter().enumerate() {
        if *ch == '\n' {
            starts.push(index + 1);
        }
    }
    starts
}

fn line_of(starts: &[usize], index: usize) -> u32 {
    match starts.binary_search(&index) {
        Ok(found) => found as u32 + 1,
        Err(found) => found as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A faithful excerpt of the subject stylesheet,
    /// `opbox-frontend/app/globals.css`, committed inline rather than as a
    /// fixture file.
    ///
    /// Inline because this module is the only file this work may add, and
    /// because an inline constant is committed with the test that reads it and
    /// therefore cannot go missing from a clean checkout the way a fixture path
    /// can. The 7,308-line original is exercised by
    /// [`the_real_subject_stylesheet`], which is `#[ignore]`d because it reads an
    /// absolute path outside this repository.
    ///
    /// Every construct here is copied from the original, not invented: the
    /// `@layer base` wrapper around the theme blocks, the five theme roots, the
    /// trailing comments carrying apostrophes and a `{`, the `var()` aliases,
    /// `oklch`, `rgba`, `color-mix`, the `!important` custom properties, the
    /// `@media` override of a `:root` property, and the unlayered `:root` block
    /// at the end of the file.
    const EXCERPT: &str = r#"
@import url('https://fonts.googleapis.com/css2?family=Fira+Code&family=Fraunces:opsz,wght@9..144,300;9..144,400&display=swap');

@tailwind base;

@layer base {
  :root {
    --font-body: "Bricolage Grotesque", var(--font-geist-sans);
    --font-geist-sans: ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif;

    --bg-primary: #ffffff;
    --bg-secondary: #f8fafc;
    --bg-tertiary: #f1f5f9;

    --fg-primary: #0f172a;

    --paper: var(--bg-primary);
    --soft: var(--bg-secondary);

    --border: #cbd5e1;
    --border-control: #748eaf;  /* [2026] VJS-CC-OPBOX 2 D6: 3.08:1 min across this theme's three planes (--bg-primary/secondary/tertiary, reached in markup via --paper/--soft/--soft-2); --border is 1.48:1 and fails SC 1.4.11. */
    --comment-rule: #b98200;  /* the 2px rule under mark.comment-highlight, which is CLICKABLE (useDocumentComments.ts closest('mark.comment-highlight')). Was rgba(255,180,0,0.5). */

    --accent: #0066ff;
    --accent-soft: #eef5ff;
    --spotlight-glass: rgba(255, 255, 255, 0.82);

    --app-page-gutter-x: 24px;
    --font-body: "Bricolage Grotesque", system-ui, -apple-system, "Segoe UI", sans-serif;
  }

  .dark {
    --bg-primary: #1a1a1a;
    --bg-secondary: #141414;
    --fg-primary: #ebebeb;
    --border: #2a2a2a;
    --border-control: #707070;  /* [2026] VJS-CC-OPBOX 2 D6: 3.06:1 min across this theme's planes; --border was 1.21:1 and fails SC 1.4.11 */
    --accent: #0066ff;
    --accent-soft: rgba(0, 102, 255, 0.18);
  }

  .neon {
    --bg-primary: #0f1419;
    --fg-primary: #e6edf3;
    --border-control: #4a709a;
    --accent: #00d4ff;
  }

  .ember {
    --bg-primary: #1c1208;
    --fg-primary: #f5e6d0;
    --border-control: #8b601f;
    --accent: #f59e0b;
  }

  .ocean {
    --bg-primary: #0d1b2a;
    --fg-primary: #e0ecf4;
    --border-control: #3d74b6;
    --accent: #06d6a0;
  }
}

/* Add page button */
.add-page-button {
  display: inline-flex;
  /* [2026] VJS-CC-OPBOX 2: CONTROL. A real <button type="button" onClick={handleAddPage}>
     at PagedDocumentEditor.tsx:216-222. background is transparent, so the dashed stroke is
     definitionally the entire boundary. --border measured 1.05:1 worst. */
  border: 1px dashed var(--border-control);
  color: var(--fg-tertiary);
}

/* AG Grid 32 native selection / controls column - center checkboxes */
.ag-controls-col-centered {
  --ag-cell-horizontal-padding: 0 !important;
  --ag-cell-widget-spacing: 0 !important;
}

.onyx-sidebar__row[data-module="matters"] {
  --module-tint: #0066ff;
}

@media (max-width: 767px) {
  :root {
    --app-page-gutter-x: 8px;
  }
}

.sign-layout {
  --bg-primary: oklch(0.985 0.004 85);
  --paper: var(--bg-primary);
  --border-control: oklch(0.6 0.02 85);
  --accent-soft: color-mix(in srgb, var(--accent) 12%, transparent);
}

:root {
  --ink: #0f172a;
  --ink-hair: var(--border);
}
"#;

    fn parse(source: &str) -> Sheet {
        let sheet = Sheet::parse(source);
        assert_eq!(
            sheet.malformed(),
            None,
            "the fixture must be structurally sound or every other assertion is about the wrong text"
        );
        sheet
    }

    fn value(sheet: &Sheet, theme: &str, property: &str) -> String {
        let resolution = sheet.resolve(theme, property);
        match resolution.outcome {
            Outcome::Value(value) => value,
            Outcome::Unresolvable(reason) => {
                panic!("expected {property} to resolve in {theme}, got: {reason}")
            }
        }
    }

    fn reason(sheet: &Sheet, theme: &str, property: &str) -> Unresolvable {
        let resolution = sheet.resolve(theme, property);
        match resolution.outcome {
            Outcome::Value(value) => {
                panic!("expected {property} to be unresolvable in {theme}, got {value:?}")
            }
            Outcome::Unresolvable(reason) => reason,
        }
    }

    // ---------------------------------------------------------------- extraction

    #[test]
    fn a_declaration_is_filed_under_the_selector_that_encloses_it() {
        let sheet = parse(":root { --a: 1px; }\n.dark { --a: 2px; }\n");
        assert_eq!(value(&sheet, ":root", "--a"), "1px");
        assert_eq!(value(&sheet, ".dark", "--a"), "2px");
    }

    #[test]
    fn nesting_through_at_rules_does_not_change_the_selector() {
        let sheet = parse("@layer base { @media screen { .dark { --a: 1px; } } }\n");
        let declaration = &sheet.declarations()[0];
        assert_eq!(declaration.selector, ".dark");
        assert_eq!(declaration.layer.as_deref(), Some("base"));
        assert_eq!(declaration.conditions, vec!["@media screen".to_string()]);
    }

    /// The named defect. `app/globals.css` line 3954 carries
    /// `onClick={handleAddPage}` inside a comment. A brace counter that does not
    /// know what a comment is closes `.add-page-button` there, and every rule
    /// after it in the file is filed one level out.
    #[test]
    fn a_brace_inside_a_comment_does_not_close_the_block() {
        let sheet = parse(
            ".a {\n  /* a real <button onClick={handleAddPage}> at Editor.tsx:216 */\n  --x: red;\n}\n.b { --x: blue; }\n",
        );
        assert_eq!(value(&sheet, ".a", "--x"), "red");
        assert_eq!(value(&sheet, ".b", "--x"), "blue");
        assert_eq!(sheet.declarations().len(), 2);
    }

    #[test]
    fn an_unbalanced_brace_inside_a_comment_does_not_close_the_block() {
        let sheet =
            parse(".a {\n  /* opens { and never closes it */\n  --x: red;\n}\n.b { --x: blue; }\n");
        assert_eq!(value(&sheet, ".a", "--x"), "red");
        assert_eq!(value(&sheet, ".b", "--x"), "blue");
    }

    #[test]
    fn a_brace_inside_a_string_does_not_close_the_block() {
        let sheet = parse(".a { --x: \"}{\"; --y: red; }\n.b { --y: blue; }\n");
        assert_eq!(value(&sheet, ".a", "--y"), "red");
        assert_eq!(value(&sheet, ".b", "--y"), "blue");
    }

    /// Line 1 of the subject stylesheet is an `@import` whose URL contains
    /// `9..144,300;9..144,400`. Semicolons inside a string do not end a
    /// statement, and reading them as separators desynchronises everything that
    /// follows.
    #[test]
    fn a_semicolon_inside_a_string_does_not_end_a_statement() {
        let sheet = parse(
            "@import url('https://x/css2?family=Fraunces:opsz,wght@9..144,300;9..144,400&display=swap');\n:root { --a: 1px; }\n",
        );
        assert_eq!(value(&sheet, ":root", "--a"), "1px");
        assert_eq!(sheet.declarations().len(), 1);
    }

    #[test]
    fn a_comment_opener_inside_a_string_does_not_start_a_comment() {
        let sheet = parse(".a { --glob: \"/* not a comment\"; --x: red; }\n");
        assert_eq!(value(&sheet, ".a", "--x"), "red");
    }

    /// The subject stylesheet's comments are full of prose: `this theme's
    /// planes`, `closest('mark.comment-highlight')`, `they're`. An apostrophe in
    /// a comment that opened a string would blank the rest of the sheet.
    #[test]
    fn an_apostrophe_inside_a_comment_does_not_open_a_string() {
        let sheet = parse(
            ":root {\n  /* 3.06:1 min across this theme's planes, closest('mark.comment-highlight') */\n  --x: red;\n}\n.b { --y: blue; }\n",
        );
        assert_eq!(value(&sheet, ":root", "--x"), "red");
        assert_eq!(value(&sheet, ".b", "--y"), "blue");
    }

    /// CSS has no `//` comment. Treating one as a comment blanks the rest of the
    /// line, and line 1 of the subject stylesheet is an `@import` of an https
    /// URL.
    #[test]
    fn a_double_slash_is_not_a_comment() {
        let sheet = parse(".a { --u: url(https://x/y); --x: red; }\n");
        assert_eq!(value(&sheet, ".a", "--u"), "url(https://x/y)");
        assert_eq!(value(&sheet, ".a", "--x"), "red");
    }

    #[test]
    fn a_trailing_declaration_without_a_semicolon_is_recorded() {
        let sheet = parse(".a { --x: red }\n");
        assert_eq!(value(&sheet, ".a", "--x"), "red");
    }

    #[test]
    fn a_selector_list_records_the_declaration_under_every_member() {
        let sheet = parse(":root, [data-theme='light'] { --x: red; }\n");
        assert_eq!(value(&sheet, ":root", "--x"), "red");
        assert_eq!(value(&sheet, "[data-theme='light']", "--x"), "red");
    }

    /// A custom property accepts a balanced brace run as its value. Reading that
    /// `{` as the start of a rule pushes a phantom scope and files everything
    /// after it one level too deep.
    #[test]
    fn a_brace_inside_a_custom_property_value_does_not_open_a_rule() {
        let sheet = parse(".a { --grid: { rows: 2 }; --x: red; }\n.b { --x: blue; }\n");
        assert_eq!(value(&sheet, ".a", "--x"), "red");
        assert_eq!(value(&sheet, ".b", "--x"), "blue");
    }

    #[test]
    fn important_is_recorded_and_removed_from_the_value() {
        let sheet =
            parse(".ag-controls-col-centered { --ag-cell-horizontal-padding: 0 !important; }\n");
        let declaration = &sheet.declarations()[0];
        assert_eq!(declaration.value, "0");
        assert!(declaration.important);
    }

    #[test]
    fn important_is_matched_case_insensitively_and_a_value_ending_in_the_word_is_not() {
        let sheet = parse(".a { --x: 0 !IMPORTANT; --y: very-important; }\n");
        assert!(sheet.declarations_in(".a")[0].important);
        assert!(!sheet.declarations_in(".a")[1].important);
        assert_eq!(sheet.declarations_in(".a")[1].value, "very-important");
    }

    #[test]
    fn a_multi_line_value_is_collapsed_to_one_line() {
        let sheet = parse(
            ".a {\n  --shadow: 0 1px 3px rgba(0,0,0,0.12),\n            0 4px 12px rgba(0,0,0,0.08);\n}\n",
        );
        assert_eq!(
            value(&sheet, ".a", "--shadow"),
            "0 1px 3px rgba(0,0,0,0.12), 0 4px 12px rgba(0,0,0,0.08)"
        );
    }

    #[test]
    fn a_comment_inside_a_value_separates_tokens_rather_than_joining_them() {
        let sheet = parse(".a { --x: 1px/* c */solid; }\n");
        assert_eq!(value(&sheet, ".a", "--x"), "1px solid");
    }

    #[test]
    fn whitespace_inside_a_string_in_a_value_is_preserved() {
        let sheet = parse(".a { --x: \"two  spaces\"; }\n");
        assert_eq!(value(&sheet, ".a", "--x"), "\"two  spaces\"");
    }

    #[test]
    fn a_line_number_survives_comment_blanking() {
        let sheet = parse(":root {\n  /* one\n     two\n     three */\n  --x: red;\n}\n");
        assert_eq!(sheet.declarations()[0].line, 5);
    }

    #[test]
    fn a_declaration_directly_inside_a_font_face_belongs_to_no_scope() {
        let sheet = parse("@font-face { --x: red; font-family: A; }\n");
        assert!(
            sheet.declarations().is_empty(),
            "an at-rule that is not a style rule has no selector to file a declaration under"
        );
    }

    #[test]
    fn a_property_name_that_is_not_a_custom_property_is_not_recorded() {
        let sheet = parse(".a { color: red; -webkit-mask: none; --x: blue; }\n");
        assert_eq!(sheet.declarations().len(), 1);
        assert_eq!(sheet.declarations()[0].property, "--x");
    }

    #[test]
    fn an_escaped_colon_in_a_selector_is_not_a_declaration_separator() {
        let sheet = parse(".dashboard .print\\:hidden { --x: red; }\n");
        assert_eq!(sheet.declarations().len(), 1);
        assert_eq!(sheet.declarations()[0].property, "--x");
    }

    // ---------------------------------------------------- malformed input, reported

    #[test]
    fn an_unterminated_comment_is_reported_rather_than_silently_swallowing_the_sheet() {
        let sheet = Sheet::parse(":root { --a: 1px; }\n/* oops\n.dark { --a: 2px; }\n");
        let malformed = sheet.malformed().expect("must be reported");
        assert!(malformed.contains("never closed"), "{malformed}");
        assert!(
            malformed.contains("missing from this extraction"),
            "{malformed}"
        );
    }

    #[test]
    fn an_unclosed_block_is_reported() {
        let sheet = Sheet::parse(":root { --a: 1px;\n");
        let malformed = sheet.malformed().expect("must be reported");
        assert!(malformed.contains("never closed"), "{malformed}");
    }

    #[test]
    fn a_stray_closing_brace_is_reported() {
        let sheet = Sheet::parse(":root { --a: 1px; } }\n.dark { --a: 2px; }\n");
        let malformed = sheet.malformed().expect("must be reported");
        assert!(malformed.contains("no matching open block"), "{malformed}");
    }

    #[test]
    fn a_sound_sheet_reports_nothing_malformed() {
        assert_eq!(Sheet::parse(EXCERPT).malformed(), None);
    }

    // -------------------------------------------------------------------- themes

    #[test]
    fn the_theme_list_is_discovered_from_the_sheet_and_not_from_a_list_of_names() {
        // Deliberately none of dark/ember/neon/ocean: a hardcoded five would
        // report nothing here, and a hardcoded five would also miss a sixth.
        let sheet =
            parse(":root { --bg: #fff; }\n.midnight { --bg: #000; }\n.sepia { --bg: #f4ecd8; }\n");
        assert_eq!(
            sheet.theme_selectors(),
            vec![":root", ".midnight", ".sepia"]
        );
        assert_eq!(sheet.base_selector(), Some(":root"));
        assert!(sheet.themes()[0].is_base);
        assert!(!sheet.themes()[1].is_base);
    }

    #[test]
    fn a_data_theme_attribute_selector_is_a_theme() {
        let sheet = parse(":root { --bg: #fff; }\n[data-theme='dusk'] { --bg: #223; }\n");
        assert!(sheet.theme_selectors().contains(&"[data-theme='dusk']"));
        assert_eq!(value(&sheet, "[data-theme='dusk']", "--bg"), "#223");
    }

    #[test]
    fn html_and_body_carriers_are_themes() {
        let sheet = parse(
            ":root { --bg: #fff; }\nhtml.dark { --bg: #000; }\nbody.high-contrast { --bg: #111; }\n",
        );
        assert!(sheet.theme_selectors().contains(&"html.dark"));
        assert!(sheet.theme_selectors().contains(&"body.high-contrast"));
    }

    /// A component that owns custom properties is not a palette.
    /// `.onyx-sidebar__row[data-module='matters']` is root-like by SHAPE and
    /// declares `--module-tint`, which `:root` never mentions. Reporting it as a
    /// theme would put a per-component tint into the contrast surface and make
    /// the theme count wrong.
    #[test]
    fn a_component_scope_owning_custom_properties_is_not_a_theme() {
        let sheet = parse(EXCERPT);
        assert!(
            !sheet
                .theme_selectors()
                .contains(&".ag-controls-col-centered")
        );
        assert!(
            !sheet
                .theme_selectors()
                .contains(&".onyx-sidebar__row[data-module=\"matters\"]")
        );
    }

    #[test]
    fn a_descendant_selector_is_never_a_theme() {
        let sheet = parse(":root { --bg: #fff; }\n.dark .panel { --bg: #000; }\n");
        assert_eq!(sheet.theme_selectors(), vec![":root"]);
    }

    /// The dangerous direction. `.dark .panel` redeclares a base palette
    /// property and is not root-like, so it is not a theme, and a caller that
    /// simply iterated `themes()` would never ask about it. Silence there is the
    /// exact failure VDS exists to catch, so it is reported instead.
    #[test]
    fn a_non_root_scope_that_redeclares_the_base_palette_is_reported_not_dropped() {
        let sheet = parse(":root { --bg: #fff; }\n.dark .panel { --bg: #000; }\n");
        assert_eq!(sheet.unclassified_palette_scopes(), vec![".dark .panel"]);
    }

    #[test]
    fn a_component_scope_owning_only_its_own_properties_is_not_reported_as_a_missed_palette() {
        let sheet = parse(EXCERPT);
        assert!(
            sheet.unclassified_palette_scopes().is_empty(),
            "got {:?}",
            sheet.unclassified_palette_scopes()
        );
    }

    #[test]
    fn a_sheet_with_no_root_has_no_base_and_every_root_like_palette_stands_alone() {
        let sheet = parse(".dark { --bg: #000; }\n.light { --bg: #fff; }\n");
        assert_eq!(sheet.base_selector(), None);
        assert_eq!(sheet.theme_selectors(), vec![".dark", ".light"]);
        assert_eq!(value(&sheet, ".dark", "--bg"), "#000");
        assert!(matches!(
            reason(&sheet, ".dark", "--missing"),
            Unresolvable::NotDeclared { .. }
        ));
    }

    #[test]
    fn the_theme_property_surface_is_the_union_over_theme_scopes_only() {
        let sheet = parse(EXCERPT);
        let properties = sheet.theme_properties();
        assert!(properties.contains(&"--border-control"));
        assert!(
            !properties.contains(&"--module-tint"),
            "a component-scoped property is not part of the theme surface"
        );
        assert!(
            !properties.contains(&"--ag-cell-widget-spacing"),
            "a component-scoped property is not part of the theme surface"
        );
    }

    // ---------------------------------------------------------------- resolution

    #[test]
    fn a_theme_declaration_beats_the_base_and_the_base_fills_the_gaps() {
        let sheet = parse(EXCERPT);
        assert_eq!(value(&sheet, ".dark", "--bg-primary"), "#1a1a1a");
        assert_eq!(value(&sheet, ".dark", "--bg-tertiary"), "#f1f5f9");
    }

    /// The defect this module exists for. `--paper: var(--bg-primary)` is
    /// declared once, in `:root`. Resolving it once and reusing the answer gives
    /// every theme `#ffffff`, and a contrast gate then measures every dark theme
    /// against white and calls the dark themes the safest in the system.
    /// Substitution has to happen in the asking theme's context at every level.
    #[test]
    fn a_base_declaration_resolves_through_the_asking_themes_overrides() {
        let sheet = parse(EXCERPT);
        assert_eq!(value(&sheet, ":root", "--paper"), "#ffffff");
        assert_eq!(value(&sheet, ".dark", "--paper"), "#1a1a1a");
        assert_eq!(value(&sheet, ".neon", "--paper"), "#0f1419");
        assert_eq!(value(&sheet, ".ember", "--paper"), "#1c1208");
        assert_eq!(value(&sheet, ".ocean", "--paper"), "#0d1b2a");
    }

    #[test]
    fn a_var_inside_a_function_resolves_in_the_asking_themes_context() {
        let sheet = parse(EXCERPT);
        assert_eq!(
            value(&sheet, ".sign-layout", "--accent-soft"),
            "color-mix(in srgb, #0066ff 12%, transparent)"
        );
    }

    #[test]
    fn a_later_declaration_in_the_same_scope_and_layer_wins() {
        let sheet = parse(EXCERPT);
        assert_eq!(
            value(&sheet, ":root", "--font-body"),
            "\"Bricolage Grotesque\", system-ui, -apple-system, \"Segoe UI\", sans-serif",
            "the excerpt declares --font-body twice in :root, exactly as the original does"
        );
    }

    #[test]
    fn several_vars_in_one_value_are_all_substituted() {
        let sheet = parse(
            ":root { --a: 1px; --b: solid; --c: red; --border: var(--a) var(--b) var(--c); }\n",
        );
        assert_eq!(value(&sheet, ":root", "--border"), "1px solid red");
    }

    #[test]
    fn a_chain_of_vars_resolves_to_the_end() {
        let sheet = parse(":root { --a: var(--b); --b: var(--c); --c: var(--d); --d: #060; }\n");
        assert_eq!(value(&sheet, ":root", "--a"), "#060");
    }

    /// CSS Variables Level 1 §3, the specification's own worked example:
    /// `.two { color: var(--my-var, red); }` falls back to `red` when
    /// `--my-var` is not defined.
    #[test]
    fn a_fallback_is_used_when_the_variable_is_not_defined() {
        let sheet = parse(":root { --two: var(--my-var, red); }\n");
        assert_eq!(value(&sheet, ":root", "--two"), "red");
    }

    /// CSS Variables Level 1 §3, the next line of the same example:
    /// `var(--my-var, var(--my-background, pink))` is `pink` when neither is
    /// defined.
    #[test]
    fn a_fallback_may_itself_be_a_var_with_a_fallback() {
        let sheet = parse(":root { --three: var(--my-var, var(--my-background, pink)); }\n");
        assert_eq!(value(&sheet, ":root", "--three"), "pink");
    }

    /// CSS Variables Level 1 §3 again, the case the specification calls out as
    /// the trap: in `var(--my-var, --my-background, pink)` the fallback is the
    /// single token stream `--my-background, pink`, not a second variable
    /// reference. A parser that split on every comma would resolve
    /// `--my-background` and produce a colour the browser never shows.
    #[test]
    fn the_fallback_is_everything_after_the_first_comma_and_is_not_a_variable_reference() {
        let sheet =
            parse(":root { --my-background: #0f0; --x: var(--my-var, --my-background, pink); }\n");
        assert_eq!(value(&sheet, ":root", "--x"), "--my-background, pink");
    }

    #[test]
    fn a_fallback_containing_a_function_with_commas_survives_intact() {
        let sheet = parse(":root { --x: var(--nope, rgba(1, 2, 3, 0.4)); }\n");
        assert_eq!(value(&sheet, ":root", "--x"), "rgba(1, 2, 3, 0.4)");
    }

    /// An empty fallback is legal and is not the same thing as no fallback:
    /// `var(--a,)` resolves to nothing, where `var(--a)` is invalid.
    #[test]
    fn an_empty_fallback_resolves_to_empty_and_is_not_an_undefined_variable() {
        let sheet = parse(":root { --x: var(--nope,); }\n");
        assert_eq!(value(&sheet, ":root", "--x"), "");
    }

    /// A custom property declared with an empty value holds the empty token
    /// stream, not the guaranteed-invalid value, so the fallback is NOT taken
    /// (CSS Variables Level 1 §2). Taking it would give the gate a colour that
    /// the browser does not use.
    #[test]
    fn a_declared_but_empty_property_does_not_trigger_the_fallback() {
        let sheet = parse(":root { --empty: ; --x: var(--empty, red); }\n");
        assert_eq!(value(&sheet, ":root", "--x"), "");
    }

    #[test]
    fn a_theme_may_blank_a_base_value_by_redeclaring_it_empty() {
        let sheet = parse(
            ":root { --shadow: 0 1px 2px #000; }\n.flat { --shadow: ; --bg: #fff; }\n:root { --bg: #eee; }\n",
        );
        assert_eq!(value(&sheet, ".flat", "--shadow"), "");
    }

    #[test]
    fn an_undefined_variable_with_no_fallback_names_the_variable_in_the_reason() {
        let sheet = parse(":root { --x: var(--never-declared); }\n");
        match reason(&sheet, ":root", "--x") {
            Unresolvable::UndefinedVariable { name, selector } => {
                assert_eq!(name, "--never-declared");
                assert_eq!(selector, ":root");
            }
            other => panic!("wrong reason: {other}"),
        }
    }

    #[test]
    fn an_undefined_variable_deep_in_a_chain_names_the_undefined_one() {
        let sheet = parse(":root { --a: var(--b); --b: var(--c); }\n");
        match reason(&sheet, ":root", "--a") {
            Unresolvable::UndefinedVariable { name, .. } => assert_eq!(name, "--c"),
            other => panic!("wrong reason: {other}"),
        }
    }

    /// CSS Variables Level 1 §3.1: a custom property whose own value fails to
    /// substitute holds the guaranteed-invalid value, so a `var()` referencing
    /// IT takes its own fallback. `--broken` is declared, but it is invalid, so
    /// `var(--broken, red)` is `red`.
    #[test]
    fn a_reference_to_an_invalid_property_takes_the_outer_fallback() {
        let sheet = parse(":root { --broken: var(--nope); --x: var(--broken, red); }\n");
        assert_eq!(value(&sheet, ":root", "--x"), "red");
    }

    #[test]
    fn a_property_declared_in_no_scope_is_not_declared_rather_than_undefined() {
        let sheet = parse(":root { --a: 1px; }\n");
        match reason(&sheet, ":root", "--nothing") {
            Unresolvable::NotDeclared { property, selector } => {
                assert_eq!(property, "--nothing");
                assert_eq!(selector, ":root");
            }
            other => panic!("wrong reason: {other}"),
        }
    }

    #[test]
    fn an_unknown_theme_is_refused_rather_than_resolved_against_the_base() {
        let sheet = parse(":root { --a: 1px; }\n");
        match reason(&sheet, ".not-in-this-sheet", "--a") {
            Unresolvable::UnknownTheme { selector } => assert_eq!(selector, ".not-in-this-sheet"),
            other => panic!("wrong reason: {other}"),
        }
    }

    /// CSS Variables Level 1 §2.1's worked example, with the arithmetic dropped:
    /// `--one: var(--two); --two: var(--one);` is a dependency cycle.
    #[test]
    fn a_two_step_cycle_is_refused_and_the_path_is_returned() {
        let sheet = parse(":root { --one: var(--two); --two: var(--one); }\n");
        match reason(&sheet, ":root", "--one") {
            Unresolvable::Cycle { path } => {
                assert_eq!(path, vec!["--one", "--two", "--one"]);
            }
            other => panic!("wrong reason: {other}"),
        }
    }

    /// CSS Variables Level 1 §2.1: `--a: var(--a)` is a cycle of one.
    #[test]
    fn a_self_reference_is_refused() {
        let sheet = parse(":root { --a: var(--a); }\n");
        match reason(&sheet, ":root", "--a") {
            Unresolvable::Cycle { path } => assert_eq!(path, vec!["--a", "--a"]),
            other => panic!("wrong reason: {other}"),
        }
    }

    #[test]
    fn a_cycle_reached_only_through_a_theme_override_is_still_refused() {
        let sheet = parse(":root { --a: #fff; --b: var(--a); }\n.dark { --a: var(--b); }\n");
        assert_eq!(value(&sheet, ":root", "--b"), "#fff");
        assert!(matches!(
            reason(&sheet, ".dark", "--b"),
            Unresolvable::Cycle { .. }
        ));
    }

    /// The deliberate divergence from CSS, asserted so it cannot drift. CSS
    /// would make the cycle guaranteed-invalid and hand back `red`. This module
    /// refuses, because a gate that reports `red` has quietly measured a colour
    /// nobody chose while a real defect in the sheet goes unreported.
    #[test]
    fn a_fallback_does_not_rescue_a_cycle() {
        let sheet = parse(":root { --a: var(--b); --b: var(--a, red); }\n");
        assert!(matches!(
            reason(&sheet, ":root", "--a"),
            Unresolvable::Cycle { .. }
        ));
    }

    #[test]
    fn a_chain_longer_than_the_depth_cap_is_refused_and_says_so() {
        let mut source = String::from(":root {\n");
        for i in 0..(MAX_RESOLUTION_DEPTH + 4) {
            source.push_str(&format!("  --p{i}: var(--p{});\n", i + 1));
        }
        source.push_str(&format!("  --p{}: #000;\n}}\n", MAX_RESOLUTION_DEPTH + 4));
        let sheet = parse(&source);
        match reason(&sheet, ":root", "--p0") {
            Unresolvable::DepthExceeded { property, limit } => {
                assert_eq!(property, "--p0");
                assert_eq!(limit, MAX_RESOLUTION_DEPTH);
            }
            other => panic!("wrong reason: {other}"),
        }
    }

    /// Doubling every level is acyclic and inside the depth cap, and still
    /// produces 2^n characters. A gate that hangs is a gate that gets switched
    /// off, so the budget is a refusal rather than a wait.
    #[test]
    fn an_exponential_expansion_is_refused_rather_than_run() {
        let mut source = String::from(":root {\n  --p0: 0123456789;\n");
        for i in 1..24 {
            source.push_str(&format!("  --p{i}: var(--p{}) var(--p{});\n", i - 1, i - 1));
        }
        source.push_str("}\n");
        let sheet = parse(&source);
        match reason(&sheet, ":root", "--p23") {
            Unresolvable::ExpansionTooLarge { property, limit } => {
                assert_eq!(property, "--p23");
                assert_eq!(limit, MAX_EXPANSION_CHARS);
            }
            other => panic!("wrong reason: {other}"),
        }
    }

    #[test]
    fn an_unterminated_var_is_refused_and_says_so() {
        let sheet = parse(":root { --a: 1px; --x: var(--a; }\n");
        match reason(&sheet, ":root", "--x") {
            Unresolvable::MalformedValue { detail, .. } => {
                assert!(detail.contains("never closed"), "{detail}")
            }
            other => panic!("wrong reason: {other}"),
        }
    }

    #[test]
    fn a_var_whose_first_argument_is_not_a_custom_property_is_refused() {
        let sheet = parse(":root { --x: var(bg-primary); }\n");
        match reason(&sheet, ":root", "--x") {
            Unresolvable::MalformedValue { detail, .. } => {
                assert!(detail.contains("two dashes"), "{detail}")
            }
            other => panic!("wrong reason: {other}"),
        }
    }

    #[test]
    fn a_var_inside_a_string_is_text_and_is_not_substituted() {
        let sheet = parse(":root { --a: red; --x: \"var(--a)\"; }\n");
        assert_eq!(value(&sheet, ":root", "--x"), "\"var(--a)\"");
    }

    #[test]
    fn an_identifier_ending_in_var_is_not_a_var_function() {
        let sheet = parse(":root { --x: myvar(1); }\n");
        assert_eq!(value(&sheet, ":root", "--x"), "myvar(1)");
    }

    #[test]
    fn var_is_recognised_whatever_its_case() {
        let sheet = parse(":root { --a: red; --x: VAR(--a); }\n");
        assert_eq!(value(&sheet, ":root", "--x"), "red");
    }

    #[test]
    fn an_arbitrary_value_can_be_resolved_in_a_themes_context() {
        let sheet = parse(EXCERPT);
        assert_eq!(
            sheet.resolve_value(".dark", "1px dashed var(--border-control)"),
            Outcome::Value("1px dashed #707070".to_owned())
        );
        assert_eq!(
            sheet.resolve_value(":root", "1px dashed var(--border-control)"),
            Outcome::Value("1px dashed #748eaf".to_owned())
        );
    }

    // ------------------------------------------------- conditions and cascade layers

    /// `--app-page-gutter-x` is 24px in `@layer base`'s `:root` and 8px in an
    /// unlayered `@media (max-width: 767px)` `:root`. There is no single
    /// viewport to resolve against, so the guarded declaration is not applied
    /// and IS reported. A gate that ignored the report would certify a desktop
    /// the user may never see.
    #[test]
    fn a_conditional_declaration_is_not_applied_but_is_reported() {
        let sheet = parse(EXCERPT);
        let resolution = sheet.resolve(":root", "--app-page-gutter-x");
        assert_eq!(resolution.value(), Some("24px"));
        assert_eq!(resolution.conditional.len(), 1);
        assert_eq!(resolution.conditional[0].value, "8px");
        assert_eq!(
            resolution.conditional[0].conditions,
            vec!["@media (max-width: 767px)".to_string()]
        );
    }

    #[test]
    fn a_property_declared_only_conditionally_falls_through_to_the_base() {
        let sheet = parse(
            ":root { --gap: 24px; }\n@media (max-width: 767px) { .dark { --gap: 8px; } }\n.dark { --bg: #000; }\n:root { --bg: #fff; }\n",
        );
        let resolution = sheet.resolve(".dark", "--gap");
        assert_eq!(resolution.value(), Some("24px"));
        assert_eq!(resolution.conditional.len(), 1);
    }

    /// CSS Cascade 5: unlayered normal declarations win over every layered one,
    /// whatever the source order. Reading source order alone gives `#000` here,
    /// which is not what a browser paints.
    #[test]
    fn an_unlayered_declaration_beats_a_layered_one_declared_after_it() {
        let sheet = parse(":root { --bg: #fff; }\n@layer base { :root { --bg: #000; } }\n");
        assert_eq!(value(&sheet, ":root", "--bg"), "#fff");
    }

    /// The mirror rule: an `!important` layered declaration beats an unlayered
    /// one.
    #[test]
    fn an_important_layered_declaration_beats_an_unlayered_important_one() {
        let sheet = parse(
            "@layer base { :root { --bg: #000 !important; } }\n:root { --bg: #fff !important; }\n",
        );
        assert_eq!(value(&sheet, ":root", "--bg"), "#000");
    }

    #[test]
    fn important_beats_normal_in_the_same_scope() {
        let sheet = parse(":root { --bg: #000 !important; }\n:root { --bg: #fff; }\n");
        assert_eq!(value(&sheet, ":root", "--bg"), "#000");
    }

    /// Ordering between two NAMED layers depends on where each layer was first
    /// declared and on any `@layer a, b;` statement, neither of which this
    /// module models. Source order is the wrong answer whenever an earlier layer
    /// appears later in the file, so it refuses and names the layers.
    #[test]
    fn a_property_split_across_two_named_layers_is_refused_and_names_them() {
        let sheet = parse(
            "@layer components { :root { --bg: #000; } }\n@layer base { :root { --bg: #fff; } }\n",
        );
        match reason(&sheet, ":root", "--bg") {
            Unresolvable::LayerConflict {
                property, layers, ..
            } => {
                assert_eq!(property, "--bg");
                assert_eq!(layers, vec!["base".to_string(), "components".to_string()]);
            }
            other => panic!("wrong reason: {other}"),
        }
    }

    #[test]
    fn two_declarations_in_the_same_named_layer_are_settled_by_source_order() {
        let sheet =
            parse("@layer base { :root { --bg: #000; } }\n@layer base { :root { --bg: #fff; } }\n");
        assert_eq!(value(&sheet, ":root", "--bg"), "#fff");
    }

    #[test]
    fn two_anonymous_layers_are_two_layers_and_are_not_merged() {
        let sheet = parse("@layer { :root { --bg: #000; } }\n@layer { :root { --bg: #fff; } }\n");
        assert!(matches!(
            reason(&sheet, ":root", "--bg"),
            Unresolvable::LayerConflict { .. }
        ));
    }

    #[test]
    fn a_nested_layer_records_its_full_path() {
        let sheet = parse("@layer a { @layer b { :root { --bg: #000; } } }\n");
        assert_eq!(sheet.declarations()[0].layer.as_deref(), Some("a.b"));
    }

    // ----------------------------------------------------------- the real subject

    #[test]
    fn the_committed_excerpt_of_the_subject_stylesheet_resolves_end_to_end() {
        let sheet = parse(EXCERPT);

        assert_eq!(
            sheet.theme_selectors(),
            vec![
                ":root",
                ".dark",
                ".neon",
                ".ember",
                ".ocean",
                ".sign-layout"
            ],
            "the original names .sign-layout 'a sixth token scope' in its own comment, so six is \
             the right answer and five would be a silent narrowing"
        );

        // The control boundary the production defect was measured on, in every
        // theme, resolved through each theme's own overrides.
        let control: Vec<String> = sheet
            .theme_selectors()
            .into_iter()
            .map(|theme| {
                format!(
                    "{theme}={}",
                    sheet
                        .resolve(theme, "--border-control")
                        .value()
                        .expect("declared in every theme")
                )
            })
            .collect();
        assert_eq!(
            control,
            vec![
                ":root=#748eaf",
                ".dark=#707070",
                ".neon=#4a709a",
                ".ember=#8b601f",
                ".ocean=#3d74b6",
                ".sign-layout=oklch(0.6 0.02 85)",
            ]
        );

        // Every theme resolves every property of the shared surface, or says why
        // not. Nothing may be silently absent.
        for theme in sheet.theme_selectors() {
            let resolved = sheet.resolve_theme(theme);
            assert_eq!(resolved.len(), sheet.theme_properties().len());
            for (property, resolution) in &resolved {
                assert!(
                    resolution.is_resolved(),
                    "{theme} {property}: {}",
                    resolution.reason().expect("unresolved has a reason")
                );
            }
        }

        assert!(sheet.unclassified_palette_scopes().is_empty());
    }

    /// The 7,308-line original. Ignored because it reads an absolute path
    /// outside this repository, which a clean checkout elsewhere does not have:
    /// a test that passed by finding no file would be vacuous, and VDS S-7(2)(2)
    /// treats a vacuous check as no check at all. Run it with
    /// `cargo test -p vds-css -- --ignored --nocapture`.
    #[test]
    #[ignore = "reads /home/jellytot/Projects/opbox-prod/opbox-frontend/app/globals.css, which is outside this repository"]
    fn the_real_subject_stylesheet() {
        const PATH: &str = "/home/jellytot/Projects/opbox-prod/opbox-frontend/app/globals.css";
        let source = std::fs::read_to_string(PATH).expect("the subject stylesheet must be present");
        let sheet = Sheet::parse(&source);
        assert_eq!(sheet.malformed(), None);

        println!("declarations: {}", sheet.declarations().len());
        println!("scopes: {}", sheet.scope_selectors().len());
        println!("themes: {:?}", sheet.theme_selectors());
        println!("theme properties: {}", sheet.theme_properties().len());
        println!(
            "unclassified palette scopes: {:?}",
            sheet.unclassified_palette_scopes()
        );

        for theme in sheet.theme_selectors() {
            let resolved = sheet.resolve_theme(theme);
            let ok = resolved.values().filter(|r| r.is_resolved()).count();
            let conditional = resolved
                .values()
                .filter(|r| !r.conditional.is_empty())
                .count();
            println!(
                "{theme}: {ok}/{} resolved, {conditional} with a conditional override",
                resolved.len()
            );
            for (property, resolution) in &resolved {
                if let Some(reason) = resolution.reason() {
                    println!("    {property}: {reason}");
                }
            }
        }

        // Spot checks a reader can verify against the source by eye. Every one
        // of these is an ALIAS, declared once in `:root` and never redeclared by
        // the overlays, so each answer is only correct if substitution ran in
        // the asking theme's context.
        for property in [
            "--paper",
            "--soft",
            "--ink-hair",
            "--dotfield-dot",
            "--border-control",
        ] {
            let row: Vec<String> = sheet
                .theme_selectors()
                .into_iter()
                .map(|theme| {
                    format!(
                        "{theme}={}",
                        sheet
                            .resolve(theme, property)
                            .value()
                            .unwrap_or("<unresolved>")
                    )
                })
                .collect();
            println!("{property}: {}", row.join("  "));
        }

        // The five theme roots the original declares, plus the sixth its own
        // comment names. A regression that dropped one would be the silent
        // narrowing this module exists to prevent.
        assert_eq!(
            sheet.theme_selectors(),
            vec![
                ":root",
                ".dark",
                ".neon",
                ".ember",
                ".ocean",
                ".sign-layout"
            ]
        );
        // The control boundary that shipped at 1.20:1 must have a value in every
        // theme, because that is the measurement the gate cannot skip.
        for theme in sheet.theme_selectors() {
            assert!(
                sheet.resolve(theme, "--border-control").is_resolved(),
                "{theme} must resolve --border-control"
            );
        }
    }
}
