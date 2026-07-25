//! A lexer for the parts of TypeScript/JSX a ledger needs: import bindings and
//! opened JSX tags.
//!
//! This is not a TypeScript parser and does not pretend to be one. It is a
//! scanner that knows where a string, a template literal, a comment and a regular
//! expression begin and end, so that it never reads code out of a place where
//! there is no code.
//!
//! That distinction is the whole reason this module exists rather than a handful
//! of regular expressions. A regex-based scanner mis-reads three things, and each
//! one silently narrows every proof built on the ledger:
//!
//!   - A `/*` inside a string literal starts a comment that never ends, and every
//!     JSX reference after it disappears. The proof then passes over a file it
//!     never read.
//!   - A commented-out `import` still registers its binding, and if it names the
//!     same local as a live import, the governed import path recorded for that
//!     component is the dead one.
//!   - `<Foo>` inside a string or a comment counts as a rendered component.
//!
//! A false NEGATIVE here is the dangerous direction: a component that is used and
//! not detected is a component the anti-drift proof will never ask about.

/// How a name was brought into the file.
///
/// The distinction matters because the register's coordinate is
/// `(import path, EXPORT name)`, and the tag in the source carries the LOCAL
/// name. `import { Button as Btn }` renders as `<Btn />`, and looking `Btn` up
/// against a register that knows `Button` misses: it reports a registered
/// component as unregistered, which is a false alarm, and the mirror case
/// `import { Card as Button }` matches the WRONG record, which is worse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingKind {
    /// `import { Button }` or `import { Button as Btn }`.
    Named,
    /// `import Button from "..."`. The export name is literally `default`.
    Default,
    /// `import * as Icons from "..."`. The export name comes from the member
    /// expression at the use site: `<Icons.Chevron />` exports `Chevron`.
    Namespace,
}

/// One import binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    /// The name as used in this file.
    pub local: String,
    /// The name the module exports, which is what the register records.
    pub exported: String,
    pub kind: BindingKind,
    pub module: String,
    pub line: u32,
}

/// One opened JSX tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagUse {
    /// The full tag as written, e.g. `Foo.Bar`.
    pub name: String,
    /// The identifier before the first dot, which is what an import binds.
    pub root: String,
    /// The segment after the first dot, where there is one. For a namespace
    /// import this is the export name.
    pub member: Option<String>,
    pub line: u32,
    /// Whether this is a component reference or a bare HTML element.
    pub is_component: bool,
}

/// What the scanner found in one file.
#[derive(Debug, Default)]
pub struct Scanned {
    pub bindings: Vec<Binding>,
    pub tags: Vec<TagUse>,
    /// A local name bound more than once, which makes the import path for that
    /// name ambiguous. Reported rather than resolved by "last one wins".
    pub ambiguous_bindings: Vec<String>,
    /// Set where the scan ended somewhere other than code, meaning a quote,
    /// backtick or block comment was opened and never closed.
    ///
    /// This exists because of the one failure mode that is genuinely dangerous
    /// here: a reference the scanner did not SEE is not skipped, not counted and
    /// not reported. It simply does not exist, and every proof downstream passes
    /// over a file it never read while its skip counts look perfectly healthy.
    ///
    /// A caller must treat this as fatal. Converting a silent narrowing into a
    /// loud one is the whole of what VDS is for (VDS S-1(4)).
    pub unbalanced: Option<String>,
}

impl Scanned {
    fn binding_for(&self, local: &str) -> Option<&Binding> {
        if self.ambiguous_bindings.iter().any(|a| a == local) {
            return None;
        }
        self.bindings.iter().find(|b| b.local == local)
    }

    /// The module a local name was imported from, where exactly one import
    /// bound it.
    pub fn module_for(&self, local: &str) -> Option<&str> {
        self.binding_for(local).map(|b| b.module.as_str())
    }

    /// The EXPORT name a tag resolves to, which is half of the register's
    /// coordinate.
    ///
    /// Returns `None` where the tag is not imported at all, which means it is
    /// defined locally or comes from a global, and the caller records that
    /// rather than guessing.
    pub fn export_name_for(&self, tag: &TagUse) -> Option<String> {
        let binding = self.binding_for(&tag.root)?;
        Some(match binding.kind {
            // `<Icons.Chevron />` against `import * as Icons`: the export is
            // the member, not the namespace.
            BindingKind::Namespace => match &tag.member {
                Some(member) => member.clone(),
                // A bare `<Icons />` against a namespace import renders a module
                // object, which is not a component. Fall back to the local name
                // so the row is reported rather than silently resolved.
                None => binding.local.clone(),
            },
            BindingKind::Named | BindingKind::Default => binding.exported.clone(),
        })
    }
}

/// Where the scanner currently is. Everything except `Code` is a region in which
/// no import and no tag may be recognised, except `JsxText`, which is where a
/// child tag legitimately appears.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Region {
    Code,
    /// Between a JSX opening tag's `>` and the next `<` or `{`. Content, not
    /// code: a quote here is an apostrophe and a backtick is a backtick.
    JsxText,
    LineComment,
    BlockComment,
    SingleQuote,
    DoubleQuote,
    Template,
}

/// Blank out every non-code region, preserving byte offsets and line breaks.
///
/// Preserving offsets is what lets the line number of a match be computed from
/// the blanked text and still be the line number in the original file.
pub fn blank_non_code(source: &str) -> String {
    blank_non_code_checked(source).0
}

/// Blank out every non-code region, and report the region the scan ended in.
///
/// The `JsxText` region is the part that took two attempts to get right. A
/// quote in JSX CONTENT is a character, not a delimiter: `<p>it's fine</p>` and
/// ``<p>press `Enter`</p>`` are ordinary text. Treating the apostrophe as
/// opening a string blanks the rest of the line, and treating the backtick as
/// opening a template literal blanks an unbounded run of the file, taking every
/// component reference in it with it. A look-behind at the previous character is
/// not enough, because the quote is usually several characters into the text.
/// Content has to be a MODE.
pub fn blank_non_code_checked(source: &str) -> (String, Option<String>) {
    let chars: Vec<char> = source.chars().collect();
    let mut out: Vec<char> = Vec::with_capacity(chars.len());
    let mut region = Region::Code;
    let mut i = 0;

    // Depth of `${ ... }` interpolations inside template literals.
    let mut template_stack: Vec<u32> = Vec::new();
    // Brace depth of each `{ ... }` expression container entered FROM JsxText,
    // so `{cond ? <A/> : <B/>}` is code and the `}` that closes it returns to
    // content rather than leaving the scanner in code for the rest of the file.
    let mut jsx_expression_depth: Vec<u32> = Vec::new();
    // Whether we are between a `<` that opened a tag and its `>`.
    let mut in_tag = false;

    while i < chars.len() {
        let ch = chars[i];
        let next = chars.get(i + 1).copied();

        match region {
            Region::Code => {
                if ch == '/' && next == Some('/') {
                    region = Region::LineComment;
                    out.push(' ');
                    out.push(' ');
                    i += 2;
                    continue;
                }
                if ch == '/' && next == Some('*') {
                    region = Region::BlockComment;
                    out.push(' ');
                    out.push(' ');
                    i += 2;
                    continue;
                }
                if ch == '\'' {
                    region = Region::SingleQuote;
                    out.push(' ');
                    i += 1;
                    continue;
                }
                if ch == '"' {
                    region = Region::DoubleQuote;
                    out.push(' ');
                    i += 1;
                    continue;
                }
                if ch == '`' {
                    region = Region::Template;
                    template_stack.push(0);
                    out.push(' ');
                    i += 1;
                    continue;
                }

                // A `<` that begins a tag, a closing tag or a fragment.
                if ch == '<'
                    && matches!(next, Some(c) if is_ident_start(c) || c == '/' || c == '>')
                {
                    in_tag = true;
                }
                // The `>` that ends it. `/>` self-closes and still returns to
                // the parent's children, so both spellings land in JsxText.
                if ch == '>' && in_tag {
                    in_tag = false;
                    region = Region::JsxText;
                    out.push(ch);
                    i += 1;
                    continue;
                }

                if let Some(depth) = jsx_expression_depth.last_mut() {
                    if ch == '{' {
                        *depth += 1;
                    } else if ch == '}' {
                        *depth -= 1;
                        if *depth == 0 {
                            jsx_expression_depth.pop();
                            region = Region::JsxText;
                            out.push(ch);
                            i += 1;
                            continue;
                        }
                    }
                }
                if ch == '}'
                    && jsx_expression_depth.is_empty()
                    && let Some(depth) = template_stack.last_mut()
                    && *depth > 0
                {
                    *depth -= 1;
                    if *depth == 0 {
                        region = Region::Template;
                        out.push(' ');
                        i += 1;
                        continue;
                    }
                }
                out.push(ch);
                i += 1;
            }

            Region::JsxText => {
                if ch == '<' {
                    region = Region::Code;
                    // Do not consume: the Code arm decides whether this opens a
                    // tag, so `a < b` written in content is handled once.
                    continue;
                }
                if ch == '{' {
                    jsx_expression_depth.push(1);
                    region = Region::Code;
                    out.push(ch);
                    i += 1;
                    continue;
                }
                // Everything else is content. Emitted verbatim so offsets and
                // line numbers hold, and so a `<` is still found.
                out.push(ch);
                i += 1;
            }

            Region::LineComment => {
                if ch == '\n' {
                    region = Region::Code;
                    out.push('\n');
                } else {
                    out.push(' ');
                }
                i += 1;
            }
            Region::BlockComment => {
                if ch == '*' && next == Some('/') {
                    region = Region::Code;
                    out.push(' ');
                    out.push(' ');
                    i += 2;
                    continue;
                }
                out.push(if ch == '\n' { '\n' } else { ' ' });
                i += 1;
            }
            Region::SingleQuote | Region::DoubleQuote => {
                let closer = if region == Region::SingleQuote { '\'' } else { '"' };
                if ch == '\\' {
                    out.push(' ');
                    out.push(' ');
                    i += 2;
                    continue;
                }
                if ch == closer {
                    region = Region::Code;
                    out.push(' ');
                    i += 1;
                    continue;
                }
                // A newline inside a quoted string is a syntax error in real
                // TypeScript. Treating it as a terminator keeps one malformed
                // string from blanking the rest of the file.
                if ch == '\n' {
                    region = Region::Code;
                    out.push('\n');
                    i += 1;
                    continue;
                }
                out.push(' ');
                i += 1;
            }
            Region::Template => {
                if ch == '\\' {
                    out.push(' ');
                    out.push(' ');
                    i += 2;
                    continue;
                }
                if ch == '$' && next == Some('{') {
                    region = Region::Code;
                    if let Some(depth) = template_stack.last_mut() {
                        *depth = 1;
                    }
                    out.push(' ');
                    out.push(' ');
                    i += 2;
                    continue;
                }
                if ch == '`' {
                    template_stack.pop();
                    region = Region::Code;
                    out.push(' ');
                    i += 1;
                    continue;
                }
                out.push(if ch == '\n' { '\n' } else { ' ' });
                i += 1;
            }
        }
    }

    let unbalanced = match region {
        // Code and JsxText are both complete endings, and a quoted string is
        // terminated at a newline above, so none of them can hide anything. A
        // file may legitimately end inside a line comment.
        Region::Code | Region::JsxText | Region::LineComment => None,
        Region::BlockComment => Some(
            "a block comment was opened and never closed, so everything after it was blanked \
             and any component reference in it was not seen at all"
                .to_owned(),
        ),
        Region::SingleQuote | Region::DoubleQuote => {
            Some("a quoted string was opened and never closed".to_owned())
        }
        Region::Template => Some(
            "a template literal (a backtick) was opened and never closed, so everything after \
             it was blanked and any component reference in it was not seen at all"
                .to_owned(),
        ),
    };
    (out.into_iter().collect(), unbalanced)
}

fn line_index(text: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    for (offset, ch) in text.char_indices() {
        if ch == '\n' {
            starts.push(offset + 1);
        }
    }
    starts
}

fn line_of(starts: &[usize], offset: usize) -> u32 {
    match starts.binary_search(&offset) {
        Ok(index) => index as u32 + 1,
        Err(index) => index as u32,
    }
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_' || ch == '$'
}

fn is_ident_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'
}

fn starts_keyword(chars: &[char], at: usize, keyword: &str) -> bool {
    let keyword: Vec<char> = keyword.chars().collect();
    if at + keyword.len() > chars.len() {
        return false;
    }
    if chars[at..at + keyword.len()] != keyword[..] {
        return false;
    }
    if at > 0 && is_ident_char(chars[at - 1]) {
        return false;
    }
    match chars.get(at + keyword.len()) {
        None => true,
        Some(next) => !is_ident_char(*next),
    }
}

/// Every name a clause binds, as `(local, exported, kind)`.
///
/// Handles `Default`, `* as ns`, `{ a, b as c }`, `Default, { a }` and the
/// `type` modifier in both positions.
fn clause_locals(clause: &str) -> Vec<(String, String, BindingKind)> {
    let mut out = Vec::new();
    let clause = clause.trim();
    if clause.is_empty() {
        return out;
    }
    // `import type { X } from` binds no value.
    if clause.trim_start().starts_with("type ")
        && !clause.trim_start().trim_start_matches("type ").starts_with('{')
    {
        // `import type Foo from "..."`: a type-only default import.
        return out;
    }
    if clause.trim_start().starts_with("type ") {
        return out;
    }

    let (outside, inside) = match (clause.find('{'), clause.find('}')) {
        (Some(open), Some(close)) if close > open => {
            (clause[..open].to_string(), Some(clause[open + 1..close].to_string()))
        }
        _ => (clause.to_string(), None),
    };

    for part in outside.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some(rest) = part.strip_prefix("* as ") {
            let local = rest.trim();
            if is_identifier(local) {
                out.push((local.to_owned(), local.to_owned(), BindingKind::Namespace));
            }
        } else if part.starts_with('*') {
            continue;
        } else if is_identifier(part) {
            // A default import's export name is literally `default`. Recording
            // the local name here would let `import Anything from "..."` match
            // a register record called `Anything` that the module does not
            // export.
            out.push((part.to_owned(), "default".to_owned(), BindingKind::Default));
        }
    }

    if let Some(inside) = inside {
        for entry in inside.split(',') {
            let entry = entry.trim();
            if entry.is_empty() || entry.starts_with("type ") {
                continue;
            }
            let (exported, local) = match entry.split_once(" as ") {
                Some((original, alias)) => (original.trim(), alias.trim()),
                None => (entry, entry),
            };
            if is_identifier(local) && is_identifier(exported) {
                out.push((local.to_owned(), exported.to_owned(), BindingKind::Named));
            }
        }
    }
    out
}

fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if is_ident_start(first) => chars.all(is_ident_char),
        _ => false,
    }
}

/// Every opened JSX tag.
///
/// A tag containing a dot is ALWAYS a component reference, whatever the case of
/// its root. `<ui.Button />` is a member expression in JSX and resolves to a
/// value, not to an HTML element named "ui.Button", so classifying it by the case
/// of `ui` would take a governed component out of enforcement entirely.
fn scan_tags(code: &str, starts: &[usize], found: &mut Scanned) {
    let chars: Vec<char> = code.chars().collect();
    let mut byte_of_char: Vec<usize> = Vec::with_capacity(chars.len() + 1);
    {
        let mut offset = 0;
        for ch in &chars {
            byte_of_char.push(offset);
            offset += ch.len_utf8();
        }
        byte_of_char.push(offset);
    }

    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] != '<' {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        if !chars.get(j).is_some_and(|c| is_ident_start(*c)) {
            i += 1;
            continue;
        }
        let name_start = j;
        while j < chars.len() && (is_ident_char(chars[j]) || chars[j] == '.' || chars[j] == '-') {
            j += 1;
        }
        let name: String = chars[name_start..j].iter().collect();
        if name.is_empty() || name.ends_with('.') {
            i = j;
            continue;
        }
        // A `<` that is not opening a tag: `a < b`, or a generic parameter list
        // `useState<Foo>(...)`. A real JSX tag is followed by whitespace, `/`,
        // `>` or an attribute name.
        let follows = chars.get(j).copied();
        let opens_a_tag = matches!(follows, Some(c) if c.is_whitespace() || c == '>' || c == '/');
        if !opens_a_tag {
            i = j;
            continue;
        }
        // `<T,>` and `<T extends X>` in a .tsx generic. A generic parameter is
        // followed by `>(`, `,` or ` extends`.
        if follows == Some('>')
            && chars.get(j + 1) == Some(&'(')
        {
            i = j;
            continue;
        }

        let mut segments = name.split('.');
        let root = segments.next().unwrap_or(&name).to_owned();
        let member = segments.next().map(|s| s.to_owned());
        let is_component = name.contains('.')
            || root.chars().next().is_some_and(|c| c.is_ascii_uppercase());
        found.tags.push(TagUse {
            line: line_of(starts, byte_of_char[i]),
            name,
            root,
            member,
            is_component,
        });
        i = j;
    }
}

/// Scan a source file for import bindings and opened JSX tags.
///
/// [`blank_non_code`] necessarily erases the string that names the module, so
/// the specifier is read back from the ORIGINAL text at the same character
/// offsets. Two passes rather than one, and the alternative is a scanner that
/// keeps strings and then has to decide, per string, whether that particular
/// string is code. It is not, except this one.
pub fn scan(source: &str) -> Scanned {
    let (code, unbalanced) = blank_non_code_checked(source);
    let starts = line_index(&code);
    let mut found = Scanned {
        unbalanced,
        ..Default::default()
    };
    scan_tags(&code, &starts, &mut found);

    let code_chars: Vec<char> = code.chars().collect();
    let source_chars: Vec<char> = source.chars().collect();
    let mut byte_of_char: Vec<usize> = Vec::with_capacity(code_chars.len() + 1);
    {
        let mut offset = 0;
        for ch in &code_chars {
            byte_of_char.push(offset);
            offset += ch.len_utf8();
        }
        byte_of_char.push(offset);
    }

    let mut i = 0usize;
    while i < code_chars.len() {
        if !starts_keyword(&code_chars, i, "import") {
            i += 1;
            continue;
        }
        let line = line_of(&starts, byte_of_char[i]);
        let mut j = i + "import".len();
        while j < code_chars.len() && code_chars[j].is_whitespace() {
            j += 1;
        }
        if code_chars.get(j) == Some(&'(') {
            i = j;
            continue;
        }

        let clause_start = j;
        let mut clause_end = None;
        let mut k = j;
        while k < code_chars.len() && k < clause_start + 4096 {
            if code_chars[k] == ';' {
                break;
            }
            if starts_keyword(&code_chars, k, "from") {
                clause_end = Some(k);
                break;
            }
            k += 1;
        }
        let Some(clause_end) = clause_end else {
            i = j;
            continue;
        };
        let clause: String = code_chars[clause_start..clause_end].iter().collect();

        // The specifier: in the ORIGINAL text, the next quoted run after `from`.
        let mut m = clause_end + "from".len();
        while m < source_chars.len() && source_chars[m].is_whitespace() {
            m += 1;
        }
        let quote = source_chars.get(m).copied();
        if !matches!(quote, Some('"') | Some('\'')) {
            i = clause_end;
            continue;
        }
        let quote = quote.expect("checked above");
        let mut n = m + 1;
        let mut module = String::new();
        while n < source_chars.len() && source_chars[n] != quote {
            module.push(source_chars[n]);
            n += 1;
        }

        for (local, exported, kind) in clause_locals(&clause) {
            if found.bindings.iter().any(|b| b.local == local)
                && !found.ambiguous_bindings.contains(&local)
            {
                found.ambiguous_bindings.push(local.clone());
            }
            found.bindings.push(Binding {
                local,
                exported,
                kind,
                module: module.clone(),
                line,
            });
        }
        i = clause_end;
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn modules(source: &str) -> Vec<(String, String)> {
        let scanned = scan(source);
        scanned
            .bindings
            .iter()
            .map(|b| (b.local.clone(), b.module.clone()))
            .collect()
    }

    fn components(source: &str) -> Vec<String> {
        scan(source)
            .tags
            .into_iter()
            .filter(|t| t.is_component)
            .map(|t| t.name)
            .collect()
    }

    #[test]
    fn a_default_import_binds_its_local_name() {
        assert_eq!(
            modules("import Button from \"@/components/ui/button\";\n"),
            vec![("Button".into(), "@/components/ui/button".into())]
        );
    }

    #[test]
    fn named_aliased_and_namespace_imports_all_bind() {
        let source = r#"
import { Card, Table as Grid } from "@/components/ui";
import * as Icons from "@/components/icons";
import Default, { Extra } from "@/components/mixed";
"#;
        let bound: Vec<String> = modules(source).into_iter().map(|(l, _)| l).collect();
        assert_eq!(bound, vec!["Card", "Grid", "Icons", "Default", "Extra"]);
    }

    #[test]
    fn a_multi_line_import_clause_binds_every_name() {
        let source = "import {\n  Alpha,\n  Beta as Gamma,\n} from \"@/components/ui\";\n";
        let bound: Vec<String> = modules(source).into_iter().map(|(l, _)| l).collect();
        assert_eq!(bound, vec!["Alpha", "Gamma"]);
    }

    #[test]
    fn a_type_only_import_binds_nothing() {
        assert!(modules("import type { ButtonProps } from \"@/components/ui\";\n").is_empty());
        assert!(modules("import type Props from \"@/types\";\n").is_empty());
    }

    #[test]
    fn an_inline_type_specifier_is_excluded_and_its_siblings_are_not() {
        let bound: Vec<String> =
            modules("import { type Props, Button } from \"@/components/ui\";\n")
                .into_iter()
                .map(|(l, _)| l)
                .collect();
        assert_eq!(bound, vec!["Button"]);
    }

    #[test]
    fn a_dynamic_import_binds_nothing() {
        assert!(modules("const C = await import(\"@/components/ui\");\n").is_empty());
    }

    #[test]
    fn a_side_effect_import_binds_nothing() {
        assert!(modules("import \"@/styles/globals.css\";\n").is_empty());
    }

    /// The dangerous direction: a commented-out import that shadows a live one.
    /// Reading raw source and letting the last match win records the DEAD path.
    #[test]
    fn a_commented_out_import_binds_nothing() {
        let source = r#"
import { Button } from "@/components/ui";
// import { Button } from "@/legacy/ui";
/* import { Button } from "@/other/ui"; */
export default function P() { return <Button />; }
"#;
        assert_eq!(
            modules(source),
            vec![("Button".to_string(), "@/components/ui".to_string())]
        );
    }

    #[test]
    fn a_local_name_bound_twice_is_ambiguous_rather_than_last_wins() {
        let source = r#"
import { Button } from "@/components/ui";
import { Button } from "@/legacy/ui";
"#;
        let scanned = scan(source);
        assert_eq!(scanned.ambiguous_bindings, vec!["Button".to_string()]);
        assert_eq!(
            scanned.module_for("Button"),
            None,
            "an ambiguous binding resolves to nothing, so the row is reported and not guessed"
        );
    }

    /// A `/*` inside a string once started a comment that never ended, and every
    /// JSX reference after it vanished from the ledger.
    #[test]
    fn a_comment_opener_inside_a_string_does_not_start_a_comment() {
        let source = r#"
import { Button, Card } from "@/components/ui";
const glob = "src/**/*.tsx";
const tricky = "/* not a comment";
export default function P() {
  return <div><Button /><Card /></div>;
}
"#;
        assert_eq!(components(source), vec!["Button", "Card"]);
    }

    #[test]
    fn a_tag_inside_a_string_or_comment_is_not_a_reference() {
        let source = r#"
import { Button } from "@/components/ui";
const doc = "use <Button /> like this";
// <Card />
/* <Table /> */
const tpl = `<Modal />`;
export default function P() { return <Button />; }
"#;
        assert_eq!(components(source), vec!["Button"]);
    }

    #[test]
    fn a_tag_inside_a_template_interpolation_is_a_reference() {
        let source = "const x = `${<Button />}`;\n";
        assert_eq!(components(source), vec!["Button"]);
    }

    /// `<ui.Button />` is a member expression and resolves to a value. Reading
    /// the case of `ui` and calling it an HTML element takes a governed
    /// component out of enforcement entirely.
    #[test]
    fn a_dotted_tag_is_a_component_whatever_the_case_of_its_root() {
        assert_eq!(components("<ui.Button />"), vec!["ui.Button"]);
        assert_eq!(components("<Icons.Chevron />"), vec!["Icons.Chevron"]);
        assert_eq!(components("<a.b.C />"), vec!["a.b.C"]);
    }

    #[test]
    fn a_bare_element_is_not_a_component() {
        let scanned = scan("<div><span /></div>");
        assert!(scanned.tags.iter().all(|t| !t.is_component));
        assert_eq!(scanned.tags.len(), 2);
    }

    #[test]
    fn a_hyphenated_custom_element_is_not_a_component() {
        let scanned = scan("<my-widget />");
        assert_eq!(scanned.tags.len(), 1);
        assert!(!scanned.tags[0].is_component);
    }

    #[test]
    fn a_less_than_that_is_not_a_tag_is_not_counted() {
        let scanned = scan("const ok = a < b && c > d;\n");
        assert!(scanned.tags.is_empty(), "{:?}", scanned.tags);
    }

    #[test]
    fn a_generic_call_is_not_a_tag() {
        let scanned = scan("const [s] = useState<Foo>(null);\n");
        assert!(scanned.tags.is_empty(), "{:?}", scanned.tags);
    }

    #[test]
    fn a_fragment_is_not_a_component() {
        let scanned = scan("<><Button /></>");
        assert_eq!(
            scanned.tags.iter().filter(|t| t.is_component).count(),
            1
        );
    }

    /// The register's coordinate is (import path, EXPORT name), and an alias
    /// makes the local name differ from it. Looking up the local name reports a
    /// registered component as unregistered.
    #[test]
    fn an_alias_resolves_to_the_name_the_module_exports() {
        let scanned = scan(
            "import { Button as Btn } from \"@/components/ui\";\n<Btn />\n",
        );
        let tag = scanned.tags.iter().find(|t| t.root == "Btn").unwrap();
        assert_eq!(scanned.export_name_for(tag).as_deref(), Some("Button"));
    }

    /// The mirror case, which is worse: the local name matches a DIFFERENT
    /// record, so the gate checks the wrong contract and passes.
    #[test]
    fn an_alias_that_shadows_another_components_name_resolves_to_the_real_export() {
        let scanned = scan(
            "import { Card as Button } from \"@/components/ui\";\n<Button />\n",
        );
        let tag = scanned.tags.iter().find(|t| t.root == "Button").unwrap();
        assert_eq!(
            scanned.export_name_for(tag).as_deref(),
            Some("Card"),
            "the tag says Button and the module exports Card; the register knows Card"
        );
    }

    #[test]
    fn a_namespace_member_resolves_to_the_member_name() {
        let scanned = scan(
            "import * as Icons from \"@/components/ui\";\n<Icons.Chevron />\n",
        );
        let tag = scanned.tags.iter().find(|t| t.name == "Icons.Chevron").unwrap();
        assert_eq!(scanned.export_name_for(tag).as_deref(), Some("Chevron"));
    }

    #[test]
    fn a_default_import_exports_the_name_default() {
        let scanned = scan("import Button from \"@/components/ui/button\";\n<Button />\n");
        let tag = scanned.tags.iter().find(|t| t.root == "Button").unwrap();
        assert_eq!(
            scanned.export_name_for(tag).as_deref(),
            Some("default"),
            "recording the local name would let `import Anything from` match a register \
             record the module does not export"
        );
    }

    #[test]
    fn a_tag_that_is_not_imported_resolves_to_nothing() {
        let scanned = scan("function Local(){return <i/>}\n<Local />\n");
        let tag = scanned.tags.iter().find(|t| t.root == "Local").unwrap();
        assert_eq!(scanned.export_name_for(tag), None);
    }

    #[test]
    fn line_numbers_survive_blanking() {
        let source = "import { Button } from \"@/components/ui\";\n\n/* a\n   multi\n   line */\n<Button />\n";
        let scanned = scan(source);
        let tag = scanned.tags.iter().find(|t| t.name == "Button").unwrap();
        assert_eq!(tag.line, 6, "a blanked comment must not move a line number");
    }

    #[test]
    fn an_escaped_quote_does_not_end_a_string() {
        let source = r#"
import { Button } from "@/components/ui";
const s = "he said \"<Card />\" loudly";
export default function P() { return <Button />; }
"#;
        assert_eq!(components(source), vec!["Button"]);
    }

    #[test]
    fn an_unterminated_string_does_not_swallow_the_rest_of_the_file() {
        let source = "const broken = \"oops\nexport default function P() { return <Button />; }\n";
        assert_eq!(components(source), vec!["Button"]);
    }

    #[test]
    fn a_jsx_expression_container_is_code_and_its_close_returns_to_content() {
        let source = "import { A, B } from \"@/components/ui\";\n\
                      const x = <div>{cond ? <A /> : <B />}it's fine</div>;\n";
        assert_eq!(components(source), vec!["A", "B"]);
    }

    #[test]
    fn a_nested_brace_inside_a_jsx_expression_does_not_end_it_early() {
        let source = "import { A } from \"@/components/ui\";\n\
                      const x = <div>{items.map((i) => { return <A key={i} />; })}don't</div>;\n";
        assert_eq!(components(source), vec!["A"]);
    }

    #[test]
    fn a_greater_than_in_ordinary_code_does_not_open_a_content_region() {
        let source = "import { A } from \"@/components/ui\";\n\
                      const ok = a > b;\n\
                      const s = \"<Ghost />\";\n\
                      const x = <A />;\n";
        assert_eq!(
            components(source),
            vec!["A"],
            "a comparison must not put the scanner into content mode, or the string after \
             it stops being blanked"
        );
    }

    #[test]
    fn an_arrow_function_does_not_open_a_content_region() {
        let source = "import { A } from \"@/components/ui\";\n\
                      const f = () => \"<Ghost />\";\n\
                      const x = <A />;\n";
        assert_eq!(components(source), vec!["A"]);
    }

    #[test]
    fn an_unbalanced_template_is_reported_rather_than_swallowing_the_file() {
        let scanned = scan("const broken = `oops;\n<Button />\n");
        assert!(scanned.unbalanced.is_some(), "{scanned:?}");
        assert!(scanned.unbalanced.unwrap().contains("never closed"));
    }

    #[test]
    fn an_unbalanced_block_comment_is_reported() {
        let scanned = scan("/* oops\n<Button />\n");
        assert!(scanned.unbalanced.is_some());
    }

    #[test]
    fn a_well_formed_file_reports_nothing_unbalanced() {
        let source = "import { A } from \"@/components/ui\";\n\
                      const t = `fine`;\n/* fine */\nconst x = <A />;\n";
        assert_eq!(scan(source).unbalanced, None);
    }

    #[test]
    fn a_file_ending_in_jsx_content_is_balanced() {
        assert_eq!(scan("const x = <div>trailing text").unbalanced, None);
    }

    #[test]
    fn scanning_is_idempotent_on_already_blanked_source() {
        let source = "import { Button } from \"@/components/ui\";\n<Button />\n";
        assert_eq!(blank_non_code(&blank_non_code(source)), blank_non_code(source));
    }
}
