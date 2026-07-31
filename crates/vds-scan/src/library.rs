//! Reading a component library: what it exports, and what those exports accept.
//!
//! This exists to answer one adoption problem. VDS S-14(1) is honest that the
//! register is the expensive part, and a project with ninety component files
//! faces ninety records before any proof can say anything. Nobody types that,
//! so the register never exists, and a register that never exists is exactly
//! the state that produced both defects at VDS S-1(4).
//!
//! What this module does is turn that from a week of typing into a review pass.
//! What it must NOT do is pretend the review already happened. Everything it
//! produces is a CANDIDATE, minted at `proposed`, and a candidate is not a
//! contract until someone reads it and advances it. VDS S-5(4) makes the
//! lifecycle a directed path for this reason: a record that arrived at
//! `registered` without anyone deciding it should be there is a register that
//! agrees with the code by construction and therefore checks nothing.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use vds_core::{Project, Result, VdsError};
use walkdir::WalkDir;

use crate::jsx;

/// One exported component found in a library file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryExport {
    /// Repository-relative path to the file that exports it.
    pub source_file: String,
    /// The exported name, or `default` for a default export.
    pub export_name: String,
    /// The identifier as written, where a default export has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_name: Option<String>,
    pub line: u32,
    /// Props extracted from a `Props` type or interface near the export.
    ///
    /// A best effort and labelled as one. This is not a TypeScript compiler,
    /// and a prop it did not find is a prop the candidate record will not carry.
    pub props: Vec<LibraryProp>,
    /// Why the prop list may be incomplete, where there is a reason to think so.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub props_incomplete_because: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryProp {
    pub name: String,
    pub type_expr: String,
    pub required: bool,
}

/// A file in a library directory that yielded no export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkippedFile {
    pub path: String,
    pub because: String,
}

#[derive(Debug, Default)]
pub struct LibraryScan {
    pub exports: Vec<LibraryExport>,
    pub skipped: Vec<SkippedFile>,
}

/// Walk the configured library directories and read what they export.
pub fn scan_library(project: &Project) -> Result<LibraryScan> {
    let mut out = LibraryScan::default();
    let extensions = &project.config.surface.component_extensions;

    for directory in &project.config.surface.library_dirs {
        let root = project.root.join(directory);
        if !root.is_dir() {
            return Err(VdsError::precondition(format!(
                "[surface] library_dirs names {directory:?}, which is not a directory in this \
                 project. Walking a directory that is not there yields no exports, and a scan \
                 that reports fewer components than exist is worse than one that refuses."
            )));
        }
        for entry in WalkDir::new(&root).sort_by_file_name() {
            let entry = entry
                .map_err(|e| VdsError::precondition(format!("could not walk {directory}: {e}")))?;
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !extensions.iter().any(|e| e == extension) {
                continue;
            }
            let relative = project.rel(path);

            if let Some(because) = not_a_component_file(path) {
                out.skipped.push(SkippedFile {
                    path: relative,
                    because,
                });
                continue;
            }

            let source = std::fs::read(path)
                .map(|b| String::from_utf8_lossy(&b).into_owned())
                .map_err(|e| VdsError::io(path.display(), e))?;

            let found = exports_in(&source, &relative);
            if found.is_empty() {
                out.skipped.push(SkippedFile {
                    path: relative,
                    because: "no exported component-shaped symbol: an export whose name starts \
                              with a capital, or a default export"
                        .into(),
                });
                continue;
            }
            out.exports.extend(found);
        }
    }
    out.exports
        .sort_by(|a, b| (&a.source_file, &a.export_name).cmp(&(&b.source_file, &b.export_name)));
    out.skipped.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

/// Files in a library directory that are not components.
///
/// Named rather than guessed at, and reported rather than silently dropped, so
/// the carve-out is a list a reader can disagree with.
fn not_a_component_file(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    let stem = path.file_stem()?.to_str()?;
    if stem == "index" {
        return Some("a barrel: it re-exports and defines nothing".into());
    }
    for marker in [".test.", ".spec.", ".stories.", ".d."] {
        if name.contains(marker) {
            return Some(format!(
                "a {} file, not a component",
                marker.trim_matches('.')
            ));
        }
    }
    None
}

/// Exported component-shaped symbols in one source file.
///
/// Component-shaped means the exported name begins with a capital, which is the
/// same rule JSX itself uses to tell a component from an element. A lowercase
/// export is a hook, a helper or a constant, and registering it would fill the
/// register with rows no screen can reference.
fn exports_in(source: &str, relative: &str) -> Vec<LibraryExport> {
    let code = jsx::blank_non_code(source);
    let mut types = prop_types_in(&code, source);
    // Appended, never merged: a named `interface FooProps` is the component's
    // declared contract and wins over the inline literal if a file carries both.
    for entry in inline_prop_types_in(&code, source) {
        if !types.iter().any(|t| t.0 == entry.0) {
            types.push(entry);
        }
    }
    let types = types;
    let mut out = Vec::new();

    for (index, line) in code.lines().enumerate() {
        let line_number = index as u32 + 1;
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix("export ") else {
            continue;
        };
        let rest = rest.trim_start();

        // `export default function Foo(...)` and `export default Foo`.
        if let Some(after) = rest.strip_prefix("default ") {
            let after = after.trim_start();
            let local = after
                .strip_prefix("function ")
                .unwrap_or(after)
                .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$'))
                .find(|s| !s.is_empty())
                .map(|s| s.to_owned());
            let found = local
                .as_deref()
                .and_then(|name| types.iter().find(|t| t.0 == format!("{name}Props")));
            let props = found.map(|t| t.1.clone()).unwrap_or_default();
            let inherited = found.and_then(|t| t.2.clone());
            out.push(LibraryExport {
                source_file: relative.to_owned(),
                export_name: "default".into(),
                local_name: local,
                line: line_number,
                props_incomplete_because: incomplete_reason(&props, inherited.clone()),
                props,
            });
            continue;
        }

        // `export function Foo`, `export const Foo =`, `export class Foo`.
        let after_keyword = ["function ", "const ", "let ", "var ", "class "]
            .iter()
            .find_map(|k| rest.strip_prefix(k));
        if let Some(after) = after_keyword {
            let name: String = after
                .trim_start()
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '$')
                .collect();
            if is_component_name(&name) {
                let found = types.iter().find(|t| t.0 == format!("{name}Props"));
                let props = found.map(|t| t.1.clone()).unwrap_or_default();
                let inherited = found.and_then(|t| t.2.clone());
                out.push(LibraryExport {
                    source_file: relative.to_owned(),
                    export_name: name,
                    local_name: None,
                    line: line_number,
                    props_incomplete_because: incomplete_reason(&props, inherited.clone()),
                    props,
                });
            }
            continue;
        }

        // `export { Foo, Bar as Baz }`.
        if let Some(open) = rest.find('{')
            && let Some(close) = rest.find('}')
            && close > open
        {
            for entry in rest[open + 1..close].split(',') {
                let entry = entry.trim();
                if entry.is_empty() || entry.starts_with("type ") {
                    continue;
                }
                let exported = match entry.split_once(" as ") {
                    Some((_, alias)) => alias.trim(),
                    None => entry,
                };
                if is_component_name(exported) {
                    let found = types.iter().find(|t| t.0 == format!("{exported}Props"));
                    let props = found.map(|t| t.1.clone()).unwrap_or_default();
                    let inherited = found.and_then(|t| t.2.clone());
                    out.push(LibraryExport {
                        source_file: relative.to_owned(),
                        export_name: exported.to_owned(),
                        local_name: None,
                        line: line_number,
                        props_incomplete_because: incomplete_reason(&props, inherited.clone()),
                        props,
                    });
                }
            }
        }
    }
    // The second export shape, and it is not a stylistic variant of the first.
    //
    // ESM-with-a-capital is the React file convention, and it is the ONLY shape this
    // scanner knew. site-factory's blocks are CommonJS registries of render functions
    // keyed by variant:
    //
    //     module.exports = { 'divider-1': dividerPlain, 'divider-2': dividerLabelled };
    //
    // Every key is lowercase and none is a declaration, so `exports_in` matched nothing.
    // Not an error - a SILENT ZERO. All 43 blocks scanned clean and yielded nothing, and
    // `vds register import` therefore could not read the library this repository ships,
    // which is why `vds-bridge.js` writes register records by hand instead. A scanner
    // that finds nothing and reports success is the failure mode this whole programme
    // exists to refuse, and it was sitting inside the importer.
    //
    // Only entered when the ESM pass found nothing, so a file that is both (a bundler
    // interop shim) is read as the ESM it primarily is.
    if out.is_empty() {
        out.extend(commonjs_registry_exports(&code, source, relative));
    }

    out.dedup_by(|a, b| a.export_name == b.export_name && a.source_file == b.source_file);
    out
}

/// `module.exports = { 'name-1': fn, 'name-2': fn }` - a registry, not a component.
///
/// Deliberately narrow. It reads ONE object literal assigned to `module.exports`, takes
/// its keys, and stops. It does not follow a spread, resolve a computed key, or read a
/// later `module.exports.x = y`, because each of those needs an evaluator and a
/// half-evaluated answer is confidently wrong - the same reason `prop_types_in` refuses
/// to follow an `extends`.
///
/// The keys are NOT filtered by `is_component_name`. That rule exists to keep hooks and
/// constants out of the register, and it earns its keep in a file where a component and a
/// helper are both top-level exports. In a registry every key is a variant by
/// construction: the object IS the component's variant list, so a capital-letter filter
/// would reject the whole shape rather than sift it.
///
/// No prop contract is attached. A registry's render functions take one argument and its
/// shape lives in no type this file declares, so every candidate carries
/// `props_incomplete_because` rather than an empty list that reads as "takes nothing".
fn commonjs_registry_exports(code: &str, source: &str, relative: &str) -> Vec<LibraryExport> {
    let mut out = Vec::new();
    let Some(assign) = code.find("module.exports") else {
        return out;
    };
    let after = &code[assign + "module.exports".len()..];
    // `module.exports.foo = ...` is a different shape and is not read here.
    let Some(eq) = after.find('=') else {
        return out;
    };
    if after[..eq].chars().any(|c| !c.is_whitespace()) {
        return out;
    }
    let rest = after[eq + 1..].trim_start();
    if !rest.starts_with('{') {
        return out;
    }
    let base = code.len() - rest.len();
    let line = code[..assign].lines().count().max(1) as u32;

    // STRUCTURE from the blanked copy, TEXT from the original, and the split is not a
    // stylistic preference - it is forced by what blanking does.
    //
    // `blank_non_code` replaces a string literal INCLUDING ITS QUOTES with spaces, so
    // `{ 'nav-1': navSimple }` arrives here as `{          : navSimple }`. The first
    // version of this function looked for quote characters at depth 1 and found none, so
    // it walked all 43 blocks correctly and returned nothing - a silent zero inside the
    // fix for a silent zero.
    //
    // So the brace walk runs on the blanked text, where a brace inside a comment or a
    // string cannot mislead it, and records the OFFSET of each `:` at the literal's own
    // depth. The key is then read backwards from that offset in the ORIGINAL, where it
    // still has its characters. `prop_types_in` carries the same warning about union
    // types, which is the tell that this is a property of the blanking pass rather than a
    // mistake either function made.
    let blanked: Vec<char> = rest.chars().collect();
    let original: Vec<char> = source.chars().collect();
    let mut depth = 0usize;
    let mut i = 0usize;
    while i < blanked.len() {
        match blanked[i] {
            '{' | '[' | '(' => depth += 1,
            '}' | ']' | ')' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            ':' if depth == 1 => {
                if let Some(name) = key_before(&original, base + i) {
                    out.push(LibraryExport {
                        source_file: relative.to_owned(),
                        export_name: name,
                        local_name: None,
                        line,
                        props: Vec::new(),
                        props_incomplete_because: Some(
                            "a CommonJS registry entry: its render function takes one argument \
                             whose shape no type in this file declares, so no prop contract \
                             could be read. Add what it accepts before advancing it past \
                             `proposed`."
                                .to_owned(),
                        ),
                    });
                }
            }
            _ => {}
        }
        i += 1;
    }
    out
}

/// The object key immediately before `colon`, read out of the ORIGINAL characters.
///
/// Returns `None` rather than a guess wherever the text is not a plain key: an empty
/// span, an unterminated quote, or an identifier with a character no key may contain.
/// A registry whose keys this cannot read yields no candidates, which is the honest
/// outcome - importing a row named by a fragment of someone else's expression would put
/// a name in the register that names nothing.
fn key_before(original: &[char], colon: usize) -> Option<String> {
    let mut end = colon;
    while end > 0 && original.get(end - 1).is_some_and(|c| c.is_whitespace()) {
        end -= 1;
    }
    if end == 0 {
        return None;
    }
    let quote = original[end - 1];
    if quote == '\'' || quote == '"' || quote == '`' {
        let mut start = end - 1;
        while start > 0 && original[start - 1] != quote {
            start -= 1;
        }
        // `start` is now just after the opening quote, or 0 if there was none.
        if start == 0 && original.first() != Some(&quote) {
            return None;
        }
        let name: String = original[start..end - 1].iter().collect();
        return (!name.is_empty()).then_some(name);
    }
    // A bare identifier key: `{ hero: fn }`.
    let mut start = end;
    while start > 0 && is_key_char(original[start - 1]) {
        start -= 1;
    }
    let name: String = original[start..end].iter().collect();
    (!name.is_empty() && !name.chars().next().is_some_and(|c| c.is_ascii_digit())).then_some(name)
}

fn is_key_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '$' || c == '-'
}

/// Why this export's prop list is not the whole set, or `None`.
///
/// Two reasons, and the SECOND is the one that was missing. An empty list is
/// obviously incomplete and always was reported. A NON-empty list drawn from a
/// declaration that `extends` another type, or intersects one, looks complete
/// and is not, and nothing said so: `parity` R6, the direction its own header
/// calls the load-bearing half, then fires on every inherited prop as though the
/// code had invented it, or credits the row as ENFORCED over a comparison that
/// could never have been complete. A subset presented as a contract is worse
/// than an absent one.
fn incomplete_reason(props: &[LibraryProp], inherited: Option<String>) -> Option<String> {
    if let Some(reason) = inherited {
        return Some(format!(
            "{reason}. Resolving it needs a TypeScript compiler, and a half-resolved answer \
             would be confidently wrong."
        ));
    }
    props.is_empty().then(|| {
        "no `<Name>Props` type or interface was found in this file, so the candidate carries \
         no prop contract. Read the component and add what it accepts before advancing it \
         past `proposed`."
            .to_owned()
    })
}

/// PascalCase, not merely capitalised.
///
/// `export const CONSTANT = 1` starts with a capital and is not a component.
/// Registering it would fill the register with rows no screen can reference,
/// and a register full of rows nobody uses is one nobody reads.
fn is_component_name(name: &str) -> bool {
    name.chars().next().is_some_and(|c| c.is_ascii_uppercase())
        && name.chars().any(|c| c.is_ascii_lowercase())
}

/// `interface FooProps { ... }` and `type FooProps = { ... }` blocks.
///
/// A shallow reader on purpose. It does not follow an `extends`, it does not
/// resolve a union or a mapped type, and it does not look in another file. Each
/// of those would need a TypeScript compiler, and a half-implemented one would
/// produce a prop list that is confidently wrong. What it misses is reported on
/// the candidate as `props_incomplete_because` rather than left to be assumed.
fn prop_types_in(code: &str, source: &str) -> Vec<(String, Vec<LibraryProp>, Option<String>)> {
    let chars: Vec<char> = code.chars().collect();
    // Offsets are preserved by blanking, so the block is LOCATED in the blanked
    // text (where a brace in a comment or a string cannot mislead) and READ from
    // the original (where a string literal union type still has its quotes).
    // Reading the blanked text turned `'primary' | 'ghost'` into `|`, which is a
    // prop contract that is confidently wrong rather than merely absent.
    let original: Vec<char> = source.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let remaining: String = chars[i..(i + 200).min(chars.len())].iter().collect();
        let trimmed = remaining.trim_start();
        let consumed = remaining.len() - trimmed.len();

        // Both keywords can introduce a `...Props` shape, and only the offset
        // differs.
        let name_and_offset = ["interface ", "type "].into_iter().find_map(|keyword| {
            trimmed
                .strip_prefix(keyword)
                .map(|rest| (rest.to_owned(), consumed + keyword.len()))
        });

        let Some((rest, offset)) = name_and_offset else {
            i += 1;
            continue;
        };
        let name: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '$')
            .collect();
        if !name.ends_with("Props") {
            i += 1;
            continue;
        }
        let Some(open) = chars[i + offset..].iter().position(|c| *c == '{') else {
            i += 1;
            continue;
        };
        let open = i + offset + open;
        let Some(close) = matching_brace(&chars, open) else {
            i = open + 1;
            continue;
        };
        let body: String = if close < original.len() {
            original[open + 1..close].iter().collect()
        } else {
            chars[open + 1..close].iter().collect()
        };
        // Everything between the type's name and its opening brace. This is
        // where `extends ButtonHTMLAttributes<...>` and `= Base & {` live, and
        // it is the difference between a prop list that is short and one that is
        // WRONG: a shallow reader that misses inherited members and says nothing
        // hands `parity` a complete-looking contract, and R6, the direction that
        // fails a prop nobody contracted, then fires on every inherited prop or
        // silently credits the row as enforced.
        let head: String = chars[i + offset..open].iter().collect();
        out.push((name, props_in_body(&body), inheritance_in(&head)));
        i = close + 1;
    }
    out
}

/// The THIRD prop-declaration shape: an anonymous type literal at the signature.
///
/// `prop_types_in` above reads `interface FooProps { ... }`, which is the shape
/// every React style guide teaches. Opbox's kit does not use it: nineteen of its
/// twenty-one components annotate the destructured parameter directly -
///
/// ```text
/// export function Field({ label, required, hint, children }: {
///   label: string; required?: boolean; hint?: string; children: React.ReactNode;
/// }) {
/// ```
///
/// - and the named reader finds nothing, so the import wrote nineteen records
/// carrying `props: []`. That is not a component without a contract; it is a
/// contract this reader could not see, and the two are indistinguishable in the
/// register, which is what makes it worth fixing rather than reporting.
///
/// Emitted as synthetic `{Name}Props` entries so the three lookup sites in
/// `exports_in` need no change, and appended AFTER the named ones so a real
/// `interface FooProps` always wins - a component with both is declaring the
/// named one as its contract.
fn inline_prop_types_in(code: &str, source: &str) -> Vec<(String, Vec<LibraryProp>, Option<String>)> {
    let chars: Vec<char> = code.chars().collect();
    let original: Vec<char> = source.chars().collect();
    let mut out = Vec::new();
    let mut search = 0usize;

    while let Some(found) = find_from(&chars, search, "export function ") {
        let after = found + "export function ".len();
        search = after;
        let name: String = chars[after..]
            .iter()
            .take_while(|c| c.is_ascii_alphanumeric() || **c == '_' || **c == '$')
            .collect();
        if name.is_empty() || !is_component_name(&name) {
            continue;
        }
        // A generic component (`function DataTable<T extends Row>({...})`) puts a
        // type parameter list between the name and the parameters, so find the
        // paren rather than assuming it sits against the name.
        let Some(open_paren) = chars[after + name.len()..]
            .iter()
            .position(|c| *c == '(')
            .map(|o| after + name.len() + o)
        else {
            continue;
        };
        let Some(close_paren) = matching_paren(&chars, open_paren) else {
            continue;
        };
        // The annotation colon sits at depth zero of the parameter list, AFTER
        // the destructuring block closes. A colon inside the destructuring is a
        // rename (`{ a: b }`) and a colon inside the type is a member, so depth
        // is the only thing that tells the three apart.
        let mut depth = 0i32;
        let mut colon = None;
        for idx in open_paren + 1..close_paren {
            match chars[idx] {
                '{' | '(' | '[' | '<' => depth += 1,
                '}' | ')' | ']' | '>' => depth -= 1,
                ':' if depth == 0 => {
                    colon = Some(idx);
                    break;
                }
                _ => {}
            }
        }
        let Some(colon) = colon else { continue };
        let Some(open) = chars[colon + 1..close_paren]
            .iter()
            .position(|c| !c.is_whitespace())
            .map(|o| colon + 1 + o)
        else {
            continue;
        };
        if chars[open] != '{' {
            // An annotation naming a type (`: ButtonProps`) is the first
            // reader's job, and re-reading it here would double-count.
            continue;
        }
        let Some(close) = matching_brace(&chars, open) else {
            continue;
        };
        // Read the members from the ORIGINAL for the reason the named reader
        // does: blanking strips a string-literal union down to its separators,
        // turning `'sm' | 'lg'` into `|`.
        let body: String = if close < original.len() {
            original[open + 1..close].iter().collect()
        } else {
            chars[open + 1..close].iter().collect()
        };
        // Everything after the literal and before the parameter list ends. This
        // is where `& React.HTMLAttributes<HTMLDivElement>` lives, and Panel and
        // Tab both have one, so a reader that ignored it would present a subset
        // as the whole contract.
        let tail: String = chars[close + 1..close_paren].iter().collect();
        out.push((format!("{name}Props"), props_in_body(&body), inheritance_in(&tail)));
        search = close_paren;
    }
    out
}

/// The index of `needle` in `chars` at or after `from`.
fn find_from(chars: &[char], from: usize, needle: &str) -> Option<usize> {
    let pat: Vec<char> = needle.chars().collect();
    if from >= chars.len() || pat.is_empty() {
        return None;
    }
    (from..=chars.len().saturating_sub(pat.len())).find(|&i| chars[i..i + pat.len()] == pat[..])
}

fn matching_paren(chars: &[char], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (offset, ch) in chars.iter().enumerate().skip(open) {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(offset);
                }
            }
            _ => {}
        }
    }
    None
}

/// What this declaration inherits that the shallow reader did not resolve.
///
/// `Some` means the prop list below is a SUBSET of what the component accepts,
/// and saying so is the whole point: a subset presented as a contract makes
/// `parity` R6 fire on every inherited prop, or worse, credits the row as
/// enforced over a comparison that could never have been complete.
fn inheritance_in(head: &str) -> Option<String> {
    let head = head.trim();
    if head.contains("extends") {
        return Some(
            "the declaration `extends` another type, and this reader does not follow an              extends clause, so the props below are a SUBSET of what the component accepts"
                .to_owned(),
        );
    }
    if head.contains('&') {
        return Some(
            "the declaration is an intersection (`&`), and this reader reads only the inline              member block, so the props below are a SUBSET of what the component accepts"
                .to_owned(),
        );
    }
    // A utility type wrapping the inline block: `Omit<Base, 'x'> & {...}` is
    // caught above, but `Partial<{...}>` and `Pick<...>` reach here.
    for utility in [
        "Omit<",
        "Pick<",
        "Partial<",
        "Required<",
        "Readonly<",
        "Record<",
    ] {
        if head.contains(utility) {
            return Some(format!(
                "the declaration applies the utility type `{}`, which this reader does not                  resolve, so the props below may not be the set the component accepts",
                utility.trim_end_matches('<')
            ));
        }
    }
    None
}

fn matching_brace(chars: &[char], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (offset, ch) in chars.iter().enumerate().skip(open) {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(offset);
                }
            }
            _ => {}
        }
    }
    None
}

/// `name?: type;` members at the top level of a type body.
fn props_in_body(body: &str) -> Vec<LibraryProp> {
    let mut out = Vec::new();
    let chars: Vec<char> = body.chars().collect();
    let mut depth = 0usize;
    let mut member = String::new();

    for ch in chars {
        match ch {
            '{' | '(' | '[' | '<' => {
                depth += 1;
                member.push(ch);
            }
            '}' | ')' | ']' | '>' => {
                depth = depth.saturating_sub(1);
                member.push(ch);
            }
            ';' | ',' | '\n' if depth == 0 => {
                if let Some(prop) = parse_member(&member) {
                    out.push(prop);
                }
                member.clear();
            }
            _ => member.push(ch),
        }
    }
    if let Some(prop) = parse_member(&member) {
        out.push(prop);
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out.dedup_by(|a, b| a.name == b.name);
    out
}

fn parse_member(raw: &str) -> Option<LibraryProp> {
    // The body is read from the ORIGINAL source, so a comment inside a type
    // survives to here and has to be dropped by hand.
    let member: String = raw
        .lines()
        .map(|line| match line.find("//") {
            Some(at) => &line[..at],
            None => line,
        })
        .collect::<Vec<_>>()
        .join(" ");
    let member = member.trim();
    if member.is_empty() || member.starts_with('*') || member.starts_with("/*") {
        return None;
    }
    let (head, type_expr) = member.split_once(':')?;
    let head = head.trim();
    let optional = head.ends_with('?');
    let name = head.trim_end_matches('?').trim();
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
    {
        return None;
    }
    let type_expr = type_expr.trim();
    if type_expr.is_empty() {
        return None;
    }
    Some(LibraryProp {
        name: name.to_owned(),
        // Collapse whitespace, so a multi-line union does not become a
        // multi-line field in a YAML record.
        type_expr: type_expr.split_whitespace().collect::<Vec<_>>().join(" "),
        required: !optional,
    })
}

/// Where a candidate's import path should come from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportPathSource {
    /// MEASURED: a screen on the declared surface imports this export from this
    /// module. The best available answer, because it is what the code does.
    Observed { specifier: String, route: String },
    /// DERIVED from a governed prefix and the file's path. A guess, and labelled
    /// as one on the candidate.
    Derived { specifier: String },
    /// Nothing imports it and no prefix could be applied.
    Unknown,
}

impl ImportPathSource {
    pub fn specifier(&self) -> Option<&str> {
        match self {
            ImportPathSource::Observed { specifier, .. }
            | ImportPathSource::Derived { specifier } => Some(specifier),
            ImportPathSource::Unknown => None,
        }
    }
}

/// Work out how a screen would import this export.
///
/// Preference order is deliberate. The screens ledger is EVIDENCE: if a screen
/// already imports this export, that specifier is what the code does, and any
/// rule VDS invented would be a second opinion about a fact. Only where nothing
/// imports it does this fall back to deriving one, and it says which it did, so
/// a reviewer knows which rows to check.
pub fn import_path_for(
    project: &Project,
    export: &LibraryExport,
    ledger: Option<&crate::ScreensLedger>,
) -> ImportPathSource {
    if let Some(ledger) = ledger {
        for (screen, reference) in ledger.component_references() {
            if reference.lookup_name() == export.export_name
                && let Some(specifier) = &reference.import_path
            {
                return ImportPathSource::Observed {
                    specifier: specifier.clone(),
                    route: screen.route.clone(),
                };
            }
        }
    }

    let prefix = project.config.surface.governed_import_prefixes.first();
    let library = project
        .config
        .surface
        .library_dirs
        .iter()
        .find(|dir| export.source_file.starts_with(dir.as_str()));

    match (prefix, library) {
        (Some(prefix), Some(library)) => {
            // `src/components/ui/button.tsx` under prefix `@/components/` and
            // library `src/components/ui` derives `@/components/ui/button`.
            let tail = library.rsplit('/').next().unwrap_or(library);
            let stem = Path::new(&export.source_file)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            ImportPathSource::Derived {
                specifier: format!(
                    "{}{tail}/{stem}",
                    prefix.trim_end_matches('/').to_owned() + "/"
                ),
            }
        }
        _ => ImportPathSource::Unknown,
    }
}

/// The path a candidate's source file should carry, repository-relative.
pub fn source_file_of(export: &LibraryExport) -> PathBuf {
    PathBuf::from(&export.source_file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vds_core::default_config;

    fn project_with(files: &[(&str, &str)]) -> (tempfile::TempDir, Project) {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".vds")).unwrap();
        std::fs::write(
            tmp.path().join(".vds/config.toml"),
            default_config("demo", "DEMO"),
        )
        .unwrap();
        std::fs::create_dir_all(tmp.path().join("src/components/ui")).unwrap();
        for (rel, contents) in files {
            let path = tmp.path().join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, contents).unwrap();
        }
        let project = Project::discover(Some(tmp.path())).unwrap();
        (tmp, project)
    }

    #[test]
    fn a_named_function_export_is_a_candidate() {
        let (_t, project) = project_with(&[(
            "src/components/ui/button.tsx",
            "export function Button() { return <button />; }\n",
        )]);
        let scan = scan_library(&project).unwrap();
        assert_eq!(scan.exports.len(), 1);
        assert_eq!(scan.exports[0].export_name, "Button");
        assert_eq!(scan.exports[0].source_file, "src/components/ui/button.tsx");
    }

    #[test]
    fn const_class_and_brace_exports_are_all_found() {
        let (_t, project) = project_with(&[(
            "src/components/ui/many.tsx",
            "export const Card = () => <div />;\n\
             export class Modal {}\n\
             function Inner() { return <i />; }\n\
             export { Inner as Drawer };\n",
        )]);
        let scan = scan_library(&project).unwrap();
        let mut names: Vec<&str> = scan
            .exports
            .iter()
            .map(|e| e.export_name.as_str())
            .collect();
        names.sort();
        assert_eq!(names, vec!["Card", "Drawer", "Modal"]);
    }

    #[test]
    fn a_default_export_records_its_local_name() {
        let (_t, project) = project_with(&[(
            "src/components/ui/sheet.tsx",
            "export default function Sheet() { return <div />; }\n",
        )]);
        let scan = scan_library(&project).unwrap();
        assert_eq!(scan.exports[0].export_name, "default");
        assert_eq!(scan.exports[0].local_name.as_deref(), Some("Sheet"));
    }

    #[test]
    fn a_lowercase_or_screaming_case_export_is_not_a_component() {
        let (_t, project) = project_with(&[(
            "src/components/ui/use-toast.tsx",
            "export function useToast() { return null; }\n\
             export const CONSTANT = 1;\n\
             export const MAX_WIDTH = 2;\n",
        )]);
        let scan = scan_library(&project).unwrap();
        assert!(scan.exports.is_empty(), "{:?}", scan.exports);
        assert_eq!(scan.skipped.len(), 1);
    }

    #[test]
    fn barrels_tests_stories_and_declarations_are_skipped_by_name() {
        let (_t, project) = project_with(&[
            ("src/components/ui/index.tsx", "export * from './button';\n"),
            (
                "src/components/ui/button.test.tsx",
                "export function Button(){}\n",
            ),
            (
                "src/components/ui/button.stories.tsx",
                "export function Button(){}\n",
            ),
            (
                "src/components/ui/types.d.tsx",
                "export function Button(){}\n",
            ),
        ]);
        let scan = scan_library(&project).unwrap();
        assert!(scan.exports.is_empty());
        assert_eq!(scan.skipped.len(), 4);
        assert!(scan.skipped.iter().all(|s| !s.because.is_empty()));
    }

    #[test]
    fn props_are_read_from_an_interface() {
        let (_t, project) = project_with(&[(
            "src/components/ui/button.tsx",
            "interface ButtonProps {\n  variant?: 'primary' | 'ghost';\n  onClick: () => void;\n}\n\
             export function Button(p: ButtonProps) { return <button />; }\n",
        )]);
        let scan = scan_library(&project).unwrap();
        let props = &scan.exports[0].props;
        assert_eq!(props.len(), 2, "{props:?}");
        let variant = props.iter().find(|p| p.name == "variant").unwrap();
        assert!(!variant.required);
        assert_eq!(variant.type_expr, "'primary' | 'ghost'");
        let click = props.iter().find(|p| p.name == "onClick").unwrap();
        assert!(click.required);
    }

    #[test]
    fn props_are_read_from_a_type_alias() {
        let (_t, project) = project_with(&[(
            "src/components/ui/card.tsx",
            "type CardProps = { title: string; subtitle?: string };\n\
             export function Card(p: CardProps) { return <div />; }\n",
        )]);
        assert_eq!(scan_library(&project).unwrap().exports[0].props.len(), 2);
    }

    #[test]
    fn a_string_literal_union_survives_comment_blanking() {
        let (_t, project) = project_with(&[(
            "src/components/ui/badge.tsx",
            "interface BadgeProps {\n  \
               // the visual weight\n  \
               tone: 'info' | 'warn' | 'danger';\n\
             }\n\
             export function Badge(p: BadgeProps) { return <span />; }\n",
        )]);
        let scan = scan_library(&project).unwrap();
        let props = &scan.exports[0].props;
        assert_eq!(props.len(), 1, "{props:?}");
        assert_eq!(
            props[0].type_expr, "'info' | 'warn' | 'danger'",
            "blanking the source to find the block must not blank the block's own contents"
        );
    }

    #[test]
    fn a_nested_type_does_not_end_the_member_early() {
        let (_t, project) = project_with(&[(
            "src/components/ui/table.tsx",
            "interface TableProps {\n  rows: Array<{ id: string; label: string }>;\n  dense?: boolean;\n}\n\
             export function Table(p: TableProps) { return <table />; }\n",
        )]);
        let props = &scan_library(&project).unwrap().exports[0].props;
        assert_eq!(props.len(), 2, "{props:?}");
        assert_eq!(
            props.iter().find(|p| p.name == "rows").unwrap().type_expr,
            "Array<{ id: string; label: string }>"
        );
    }

    #[test]
    fn a_component_with_no_props_type_says_why_its_contract_is_empty() {
        let (_t, project) = project_with(&[(
            "src/components/ui/divider.tsx",
            "export function Divider() { return <hr />; }\n",
        )]);
        let export = &scan_library(&project).unwrap().exports[0];
        assert!(export.props.is_empty());
        assert!(
            export
                .props_incomplete_because
                .as_deref()
                .unwrap()
                .contains("before advancing it past `proposed`"),
            "{:?}",
            export.props_incomplete_because
        );
    }

    #[test]
    fn an_export_inside_a_comment_is_not_an_export() {
        let (_t, project) = project_with(&[(
            "src/components/ui/button.tsx",
            "// export function Ghost() {}\n\
             /* export function Phantom() {} */\n\
             export function Button() { return <button />; }\n",
        )]);
        let scan = scan_library(&project).unwrap();
        let names: Vec<&str> = scan
            .exports
            .iter()
            .map(|e| e.export_name.as_str())
            .collect();
        assert_eq!(names, vec!["Button"]);
    }

    #[test]
    fn a_library_directory_that_is_not_there_is_refused() {
        let (_t, project) = project_with(&[]);
        std::fs::remove_dir_all(project.root.join("src/components/ui")).unwrap();
        let error = scan_library(&project).unwrap_err();
        assert!(
            error.to_string().contains("worse than one that refuses"),
            "{error}"
        );
    }

    #[test]
    fn an_import_path_is_taken_from_a_screen_that_already_imports_it() {
        let (_t, project) = project_with(&[
            (
                "src/components/ui/button.tsx",
                "export function Button() { return <button />; }\n",
            ),
            (
                "app/dash/page.tsx",
                "import { Button } from \"@/components/ui\";\n\
                 export default function P(){ return <Button />; }\n",
            ),
        ]);
        let ledger = crate::generate(&project).unwrap();
        let scan = scan_library(&project).unwrap();
        let source = import_path_for(&project, &scan.exports[0], Some(&ledger));
        assert_eq!(
            source,
            ImportPathSource::Observed {
                specifier: "@/components/ui".into(),
                route: "app/dash/page.tsx".into(),
            },
            "what the code does beats any rule VDS could invent"
        );
    }

    #[test]
    fn an_unimported_component_gets_a_derived_path_labelled_as_derived() {
        let (_t, project) = project_with(&[(
            "src/components/ui/button.tsx",
            "export function Button() { return <button />; }\n",
        )]);
        let scan = scan_library(&project).unwrap();
        let source = import_path_for(&project, &scan.exports[0], None);
        assert_eq!(
            source,
            ImportPathSource::Derived {
                specifier: "@/components/ui/button".into()
            }
        );
    }

    /// The failing-direction test for the CommonJS registry shape, with the negative
    /// control ASSERTED rather than assumed.
    ///
    /// The defect this closes was a SILENT ZERO. `exports_in` knew only the React
    /// convention - a capitalised named export or a default export - so site-factory's 43
    /// blocks, which export `module.exports = { 'divider-1': fn, 'divider-2': fn }`,
    /// scanned clean and yielded nothing. Not an error. A scan reporting success over an
    /// empty set, inside the importer that is supposed to be the on-ramp.
    ///
    /// The negative control is the whole point and it comes first: a file with no exports
    /// must yield ZERO. Without it, a reader that returned every colon in the file would
    /// satisfy every positive assertion below.
    #[test]
    fn a_commonjs_registry_yields_one_export_per_key_and_a_bare_file_yields_none() {
        // NEGATIVE CONTROL, asserted before anything is read from it.
        let helper = "'use strict';\nfunction esc(s) { return s; }\nconst X = { a: 1 };\n";
        assert_eq!(
            exports_in(helper, "blocks/helper.js"),
            Vec::new(),
            "a file that exports nothing must yield nothing. If this ever passes trivially, \
             every assertion below is meaningless."
        );

        // The real shape, quoted from site-factory/blocks/divider.js.
        let block = "'use strict';\n\
                     function dividerPlain(content) { return '<hr>'; }\n\
                     function dividerLabelled(content) { return '<div></div>'; }\n\
                     module.exports = {\n  'divider-1': dividerPlain,\n  \
                     'divider-2': dividerLabelled,\n};\n";
        let found = exports_in(block, "blocks/divider.js");
        let names: Vec<&str> = found.iter().map(|e| e.export_name.as_str()).collect();
        assert_eq!(
            names,
            vec!["divider-1", "divider-2"],
            "each registry key is one export, and the key is the variant"
        );
        assert!(
            found
                .iter()
                .all(|e| e.props_incomplete_because.is_some() && e.props.is_empty()),
            "a registry entry declares no prop type, so the candidate must SAY its contract \
             is unread rather than carry an empty list that reads as `takes nothing`"
        );

        // Keys survive blanking. `blank_non_code` replaces a string literal INCLUDING its
        // quotes with spaces, so a reader looking for quote characters finds none - which
        // is exactly how the first version of this returned 43 empty names.
        assert!(
            names.iter().all(|n| !n.trim().is_empty()),
            "keys must be read from the ORIGINAL source, not the blanked copy"
        );

        // A bare identifier key is a registry too.
        let bare = "module.exports = {\n  hero: h,\n  footer: f,\n};\n";
        assert_eq!(
            exports_in(bare, "blocks/x.js")
                .iter()
                .map(|e| e.export_name.as_str())
                .collect::<Vec<_>>(),
            vec!["hero", "footer"]
        );

        // A nested object must not contribute keys: only the literal's own depth counts,
        // or a render function's inline config would arrive in the register as components.
        let nested = "module.exports = {\n  'a-1': make({ inner: 1, deeper: { x: 2 } }),\n};\n";
        assert_eq!(
            exports_in(nested, "blocks/n.js")
                .iter()
                .map(|e| e.export_name.as_str())
                .collect::<Vec<_>>(),
            vec!["a-1"],
            "keys inside a nested object are not exports"
        );

        // `module.exports.foo = bar` is a DIFFERENT shape and is deliberately not read.
        // Reading half of it would produce a partial export list presented as a whole one.
        assert_eq!(
            exports_in("module.exports.hero = h;\n", "blocks/p.js"),
            Vec::new()
        );

        // ESM wins where a file has both, so a bundler interop shim is read as the ESM it
        // primarily is rather than twice.
        let both = "export function Hero() {}\nmodule.exports = { 'hero-1': Hero };\n";
        assert_eq!(
            exports_in(both, "blocks/b.js")
                .iter()
                .map(|e| e.export_name.as_str())
                .collect::<Vec<_>>(),
            vec!["Hero"]
        );
    }
}

#[cfg(test)]
mod inline_props_tests {
    use super::*;

    /// The Opbox kit's shape, and the reason the inline reader exists: nineteen
    /// of its twenty-one components annotate the destructured parameter rather
    /// than declaring a named `<Name>Props`.
    #[test]
    fn reads_an_inline_annotation_and_flags_its_intersection() {
        let source = "export function Field({ label, required }: {\n  \
                      label: string; required?: boolean;\n}) { return <div />; }\n\
                      export function Panel({ interactive, ...rest }: {\n  \
                      interactive?: boolean;\n} & React.HTMLAttributes<HTMLDivElement>) \
                      { return <div />; }\n";
        let exports = exports_in(source, "ui.tsx");

        let field = exports.iter().find(|e| e.export_name == "Field").expect("Field");
        assert_eq!(
            field.props.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
            ["label", "required"],
            "an inline type literal is a prop contract and must be read as one"
        );
        assert!(
            field.props_incomplete_because.is_none(),
            "Field inherits nothing, so nothing may be withheld from it"
        );

        // THE HALF THAT MATTERS. A non-empty list that is still a SUBSET is
        // worse than an empty one, because it looks complete.
        let panel = exports.iter().find(|e| e.export_name == "Panel").expect("Panel");
        assert_eq!(panel.props.len(), 1);
        let because = panel
            .props_incomplete_because
            .as_deref()
            .expect("Panel intersects React.HTMLAttributes and must say so");
        assert!(
            because.contains("intersection"),
            "the reason must name the intersection, got: {because}"
        );
    }

    /// A named declaration still wins, so a file carrying both is not read twice.
    #[test]
    fn a_named_props_type_beats_the_inline_literal() {
        let source = "interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> \
                      {\n  variant?: 'a' | 'b';\n}\n\
                      export function Button({ variant }: ButtonProps) { return <button />; }\n";
        let exports = exports_in(source, "ui.tsx");
        let button = exports.iter().find(|e| e.export_name == "Button").expect("Button");
        assert_eq!(button.props.len(), 1);
        assert!(
            button.props_incomplete_because.as_deref().unwrap_or("").contains("extends"),
            "an `extends` clause must be declared as making the list a subset"
        );
    }
}
