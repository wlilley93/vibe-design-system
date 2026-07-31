//! The generated ledgers, and the staleness test that makes them worth reading.
//!
//! VDS S-4(2): ledgers are generated inventories, never hand-edited, and each
//! must have a staleness test that fails when its source changed and the
//! generator was not re-run.
//!
//! The staleness test here is stronger than comparing source digests, and the
//! difference matters. Comparing only the sources answers "have the screens
//! changed since this file was written". It does NOT answer "was this file
//! produced by the generator from those screens", so a hand-edited ledger with
//! an intact `source_digest` passes, and a proof reading it can be flipped from
//! failing to passing by editing the ledger rather than the code.
//!
//! [`check_fresh`] therefore REGENERATES the ledger in memory and compares the
//! content, which is the only test that answers the question actually being
//! asked. VDS S-2(5)(4) requires a ledger to be byte-reproducible by a named
//! command; this is that requirement, enforced rather than asserted.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use vds_core::{Digest, Project, Result, Timestamp, VdsError, digest_rows, write_text_atomically};

pub mod geometry;
pub mod glob;
pub mod jsx;
pub mod library;

pub const LEDGER_SCHEMA_VERSION: u32 = 1;
pub const GENERATOR_COMMAND: &str = "vds ledger screens";

/// One reference from a screen to something it renders.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reference {
    pub name: String,
    /// The identifier before the first dot, which is what an import binds.
    pub root: String,
    /// The name the module EXPORTS, which is half of the register's coordinate.
    ///
    /// Distinct from `root`, which is the local name at the use site. An alias
    /// (`import { Button as Btn }`) and a namespace member
    /// (`<Icons.Chevron />`) both make the two differ, and looking the local
    /// name up against a register that records the export name reports a
    /// registered component as unregistered, or matches the wrong record.
    #[serde(default)]
    pub export_name: Option<String>,
    /// Whether a dotted tag's member IS the export name, because the root was
    /// bound by `import * as ns`.
    ///
    /// False for a compound component (`<Card.Header />` from
    /// `import { Card }`), where the register coordinate names only the root and
    /// the member is unverified. A caller must say so rather than report the row
    /// as fully checked.
    #[serde(default)]
    pub namespace_member: bool,
    pub kind: ReferenceKind,
    /// The module the root name was imported from, AS WRITTEN. Null where the
    /// component is defined in the same file, or where the binding was
    /// ambiguous.
    pub import_path: Option<String>,
    /// A relative specifier resolved against this screen's own directory, as a
    /// repository-relative path.
    ///
    /// Without this a governed component imported by a relative path escapes
    /// enforcement entirely: `../../components/ui/button` does not start with
    /// `@/components/`, so the row is skipped as ungoverned and the anti-drift
    /// proof never asks about it. Rewriting one specifier is not a lot of work
    /// for someone trying to get a change past the gate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_path: Option<String>,
    pub line: u32,
    /// Why the import path is absent, where it is. A null with no explanation
    /// is a row nobody can act on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unresolved_because: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceKind {
    Component,
    /// A bare HTML element. VDS S-9(10) is RESERVED (SUBMISSION-VDS-005), so
    /// these are recorded and not enforced.
    Element,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Screen {
    /// Repository-relative path. The route a warrant is bounded by.
    pub route: String,
    pub digest: Digest,
    pub references: Vec<Reference>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreensLedger {
    pub schema_version: u32,
    /// When the generator last ran. Deliberately EXCLUDED from
    /// [`ScreensLedger::content_digest`], so re-running the generator over
    /// unchanged screens does not move any digest that a proof cites.
    pub generated_at: Timestamp,
    pub generated_by: String,
    pub source_globs: Vec<String>,
    /// The digest of the SOURCE FILES the ledger was generated from.
    pub source_digest: Digest,
    /// The digest of everything in this ledger except `generated_at`. What
    /// [`check_fresh`] compares, and what a proof records as an input.
    pub content_digest: Digest,
    pub screens: Vec<Screen>,
}

impl ScreensLedger {
    /// The digest of the ledger's CONTENT: everything a reader would call a
    /// fact, and nothing that merely records when the generator ran.
    pub fn compute_content_digest(&self) -> Result<Digest> {
        #[derive(Serialize)]
        struct Content<'a> {
            schema_version: u32,
            generated_by: &'a str,
            source_globs: &'a [String],
            source_digest: &'a Digest,
            screens: &'a [Screen],
        }
        Digest::of_value(&Content {
            schema_version: self.schema_version,
            generated_by: &self.generated_by,
            source_globs: &self.source_globs,
            source_digest: &self.source_digest,
            screens: &self.screens,
        })
    }

    pub fn component_references(&self) -> impl Iterator<Item = (&Screen, &Reference)> {
        self.screens.iter().flat_map(|screen| {
            screen
                .references
                .iter()
                .filter(|r| r.kind == ReferenceKind::Component)
                .map(move |r| (screen, r))
        })
    }

    /// Routes that render a given `(import path, export name)` pair.
    pub fn routes_consuming(&self, import_path: &str, export_name: &str) -> Vec<&str> {
        let mut routes: Vec<&str> = self
            .component_references()
            .filter(|(_, r)| {
                r.import_path.as_deref() == Some(import_path) && r.lookup_name() == export_name
            })
            .map(|(s, _)| s.route.as_str())
            .collect();
        routes.sort();
        routes.dedup();
        routes
    }
}

/// The screen files on the declared surface, sorted.
pub fn screen_files(project: &Project) -> Result<Vec<PathBuf>> {
    let globs = &project.config.surface.screen_globs;
    let mut found = glob::match_globs(&project.root, globs)?;
    found.sort();
    found.dedup();

    // A screen outside the root cannot be recorded as a repository-relative
    // route, and a ledger holding an absolute path stops being reproducible the
    // moment the checkout moves. Refuse rather than record one.
    let root = project
        .root
        .canonicalize()
        .unwrap_or_else(|_| project.root.clone());
    for path in &found {
        let resolved = path.canonicalize().unwrap_or_else(|_| path.clone());
        if resolved.strip_prefix(&root).is_err() {
            return Err(VdsError::precondition(format!(
                "{} matches a screen glob and lies outside the project root. A ledger row \
                 holding an absolute path is not byte-reproducible by a named command \
                 (VDS S-2(5)(4)), so this is refused rather than recorded.",
                resolved.display()
            )));
        }
    }
    Ok(found)
}

pub fn source_digest(project: &Project, files: &[PathBuf]) -> Result<Digest> {
    let mut rows = Vec::with_capacity(files.len());
    for file in files {
        rows.push((project.rel(file), Digest::of_file(file)?));
    }
    digest_rows(&rows)
}

/// Generate the screens ledger from the declared surface.
pub fn generate(project: &Project) -> Result<ScreensLedger> {
    let files = screen_files(project)?;
    let mut screens = Vec::with_capacity(files.len());

    for path in &files {
        let source = read_source(path)?;
        let scanned = jsx::scan(&source);
        if let Some(reason) = &scanned.unbalanced {
            // The one failure mode that is genuinely dangerous: a reference the
            // scanner did not SEE is not skipped, not counted and not reported.
            // It does not exist, and every proof downstream passes over a file
            // it never read while its skip counts look healthy. Refuse.
            return Err(VdsError::precondition(format!(
                "{} could not be scanned completely: {reason}.\n  \
                 A reference the scanner did not see is not counted anywhere, so a ledger \
                 built from this file would make every proof narrower than it looks and \
                 nothing would say so. This is refused rather than recorded.",
                project.rel(path)
            )));
        }
        let mut references = Vec::new();
        let screen_dir = path.parent().map(|p| p.to_path_buf());
        for tag in &scanned.tags {
            let (import_path, unresolved_because) = if tag.is_component {
                match scanned.module_for(&tag.root) {
                    Some(module) => (Some(module.to_owned()), None),
                    None if scanned.ambiguous_bindings.contains(&tag.root) => (
                        None,
                        Some(format!(
                            "{:?} is bound by more than one import in this file, so which \
                             module it names is ambiguous",
                            tag.root
                        )),
                    ),
                    None => (
                        None,
                        Some(format!(
                            "{:?} is not imported in this file, so it is defined locally or \
                             comes from a global",
                            tag.root
                        )),
                    ),
                }
            } else {
                (None, None)
            };
            let resolved_path = match (&import_path, &screen_dir) {
                (Some(specifier), Some(dir)) => resolve_relative(project, dir, specifier),
                _ => None,
            };
            references.push(Reference {
                name: tag.name.clone(),
                root: tag.root.clone(),
                export_name: tag
                    .is_component
                    .then(|| scanned.export_name_for(tag))
                    .flatten(),
                namespace_member: tag.is_component && scanned.is_namespace_member(tag),
                kind: if tag.is_component {
                    ReferenceKind::Component
                } else {
                    ReferenceKind::Element
                },
                import_path,
                resolved_path,
                line: tag.line,
                unresolved_because,
            });
        }
        // Sort so the ledger's bytes depend on the file's content and not on
        // the order the scanner happened to walk it.
        references.sort_by(|a, b| {
            a.line
                .cmp(&b.line)
                .then_with(|| a.name.cmp(&b.name))
                .then_with(|| a.kind.cmp_key().cmp(&b.kind.cmp_key()))
        });
        screens.push(Screen {
            route: project.rel(path),
            digest: Digest::of_file(path)?,
            references,
        });
    }
    screens.sort_by(|a, b| a.route.cmp(&b.route));

    let mut ledger = ScreensLedger {
        schema_version: LEDGER_SCHEMA_VERSION,
        generated_at: Timestamp::now(),
        generated_by: GENERATOR_COMMAND.to_owned(),
        source_globs: project.config.surface.screen_globs.clone(),
        source_digest: source_digest(project, &files)?,
        content_digest: Digest::of_text("placeholder"),
        screens,
    };
    ledger.content_digest = ledger.compute_content_digest()?;
    Ok(ledger)
}

impl ReferenceKind {
    fn cmp_key(self) -> u8 {
        match self {
            ReferenceKind::Component => 0,
            ReferenceKind::Element => 1,
        }
    }
}

/// Resolve a relative module specifier against the importing file's directory.
///
/// Returns a repository-relative path with no extension, so a caller can compare
/// it against a governed library directory. A specifier that is not relative, or
/// that escapes the project root, resolves to nothing: an absolute path in a
/// ledger is not reproducible after a move, and a bare specifier is a package.
fn resolve_relative(project: &Project, screen_dir: &Path, specifier: &str) -> Option<String> {
    if !specifier.starts_with("./") && !specifier.starts_with("../") {
        return None;
    }
    let mut resolved = screen_dir.to_path_buf();
    for segment in specifier.split('/') {
        match segment {
            "." => {}
            ".." => {
                resolved.pop();
            }
            other => resolved.push(other),
        }
    }
    let root = project
        .root
        .canonicalize()
        .unwrap_or_else(|_| project.root.clone());
    let relative = resolved.strip_prefix(&root).ok()?;
    Some(relative.to_string_lossy().replace('\\', "/"))
}

impl Reference {
    /// Whether this reference is inside the governed surface.
    ///
    /// Two ways in: the specifier as written starts with a governed prefix, or
    /// it resolves to a file inside a governed library directory. The second is
    /// what closes the relative-path escape.
    pub fn is_governed(&self, prefixes: &[String], library_dirs: &[String]) -> bool {
        if let Some(specifier) = &self.import_path
            && prefixes.iter().any(|p| specifier.starts_with(p))
        {
            return true;
        }
        if let Some(resolved) = &self.resolved_path
            && library_dirs
                .iter()
                .any(|dir| resolved.starts_with(dir.trim_end_matches('/')))
        {
            return true;
        }
        false
    }

    /// The export name to look up, falling back to the local root name.
    pub fn lookup_name(&self) -> &str {
        self.export_name.as_deref().unwrap_or(&self.root)
    }

    /// Whether a dotted tag's member is left unverified by a lookup on the root.
    ///
    /// True for a compound component and false for a namespace member, because
    /// only the second resolves the member to a real export.
    pub fn member_is_unverified(&self) -> bool {
        self.name != self.root && !self.namespace_member
    }
}

fn read_source(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path).map_err(|e| VdsError::io(path.display(), e))?;
    // Lossy on purpose: a screen with one invalid byte should be scanned, not
    // skipped. A skipped screen is a screen outside every proof, and nothing
    // would say so.
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

pub fn write(project: &Project) -> Result<(PathBuf, ScreensLedger)> {
    let ledger = generate(project)?;
    let path = project.screens_ledger_path();
    let text = serde_yaml::to_string(&ledger).map_err(|e| VdsError::Serialize {
        what: project.rel(&path),
        message: e.to_string(),
    })?;
    write_text_atomically(&path, &text)?;
    Ok((path, ledger))
}

/// Why a ledger cannot be relied on.
#[derive(Debug)]
pub enum Staleness {
    GlobsChanged {
        recorded: Vec<String>,
        configured: Vec<String>,
    },
    SourcesChanged {
        added: Vec<String>,
        removed: Vec<String>,
        changed: Vec<String>,
    },
    /// The sources are unchanged and the ledger still does not match what the
    /// generator produces from them. Someone edited the ledger.
    NotGeneratedFromItsSources,
}

impl std::fmt::Display for Staleness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Staleness::GlobsChanged {
                recorded,
                configured,
            } => write!(
                f,
                "the screens ledger is STALE: [surface] screen_globs changed since it was \
                 generated.\n  ledger: {recorded:?}\n  config: {configured:?}\n  \
                 Regenerate with: {GENERATOR_COMMAND}"
            ),
            Staleness::SourcesChanged {
                added,
                removed,
                changed,
            } => {
                writeln!(
                    f,
                    "the screens ledger is STALE, so no result from it can be trusted \
                     (VDS S-4(2))."
                )?;
                if !added.is_empty() {
                    writeln!(f, "  added since generation:   {}", added.join(", "))?;
                }
                if !removed.is_empty() {
                    writeln!(f, "  removed since generation: {}", removed.join(", "))?;
                }
                if !changed.is_empty() {
                    writeln!(f, "  changed since generation: {}", changed.join(", "))?;
                }
                write!(f, "  Regenerate with: {GENERATOR_COMMAND}")
            }
            Staleness::NotGeneratedFromItsSources => write!(
                f,
                "the screens ledger does not match what the generator produces from the very \
                 screens it names. The sources are unchanged, so the ledger itself was \
                 edited.\n  A ledger is a generated inventory and never hand-edited \
                 (VDS S-4(2)), and it must be byte-reproducible by a named command \
                 (VDS S-2(5)(4)). Editing it moves what every proof reads without moving \
                 any screen.\n  Regenerate with: {GENERATOR_COMMAND}"
            ),
        }
    }
}

/// Refuse to proceed on a stale ledger, and say what moved.
pub fn check_fresh(project: &Project, ledger: &ScreensLedger) -> Result<()> {
    let configured = &project.config.surface.screen_globs;
    if &ledger.source_globs != configured {
        return Err(VdsError::precondition(
            Staleness::GlobsChanged {
                recorded: ledger.source_globs.clone(),
                configured: configured.clone(),
            }
            .to_string(),
        ));
    }

    let files = screen_files(project)?;
    let live_digest = source_digest(project, &files)?;
    if live_digest != ledger.source_digest {
        let recorded: BTreeMap<&str, &Digest> = ledger
            .screens
            .iter()
            .map(|s| (s.route.as_str(), &s.digest))
            .collect();
        let mut live: BTreeMap<String, Digest> = BTreeMap::new();
        for file in &files {
            live.insert(project.rel(file), Digest::of_file(file)?);
        }
        let added: Vec<String> = live
            .keys()
            .filter(|k| !recorded.contains_key(k.as_str()))
            .cloned()
            .collect();
        let removed: Vec<String> = recorded
            .keys()
            .filter(|k| !live.contains_key(**k))
            .map(|k| (*k).to_owned())
            .collect();
        let changed: Vec<String> = live
            .iter()
            .filter(|(k, v)| recorded.get(k.as_str()).is_some_and(|r| *r != *v))
            .map(|(k, _)| k.clone())
            .collect();
        return Err(VdsError::precondition(
            Staleness::SourcesChanged {
                added,
                removed,
                changed,
            }
            .to_string(),
        ));
    }

    // The sources are unchanged. Now the harder question: was this ledger
    // actually produced from them?
    let regenerated = generate(project)?;
    if regenerated.content_digest != ledger.content_digest
        || ledger.content_digest != ledger.compute_content_digest()?
    {
        return Err(VdsError::precondition(
            Staleness::NotGeneratedFromItsSources.to_string(),
        ));
    }
    Ok(())
}

/// Load the screens ledger and refuse it if it is stale.
pub fn load_fresh(project: &Project) -> Result<ScreensLedger> {
    let path = project.screens_ledger_path();
    if !path.is_file() {
        return Err(VdsError::precondition(format!(
            "{} is absent. The declared surface is a generated ledger (VDS S-4(2)).\n  \
             Run: {GENERATOR_COMMAND}",
            project.rel(&path)
        )));
    }
    let text = std::fs::read_to_string(&path).map_err(|e| VdsError::io(path.display(), e))?;
    let raw: serde_yaml::Value = serde_yaml::from_str(&text).map_err(|e| VdsError::Artefact {
        path: project.rel(&path),
        message: format!("is not readable YAML: {e}"),
    })?;
    let found = raw
        .get("schema_version")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    if found > LEDGER_SCHEMA_VERSION {
        return Err(VdsError::SchemaVersionAhead {
            path: project.rel(&path),
            kind: "screens ledger",
            found,
            understood: LEDGER_SCHEMA_VERSION,
        });
    }
    let ledger: ScreensLedger = serde_yaml::from_value(raw).map_err(|e| VdsError::Artefact {
        path: project.rel(&path),
        message: format!("is not a screens ledger: {e}"),
    })?;
    check_fresh(project, &ledger)?;
    Ok(ledger)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vds_core::default_config;

    struct Fixture {
        _tmp: tempfile::TempDir,
        project: Project,
    }

    fn fixture(screens: &[(&str, &str)]) -> Fixture {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".vds")).unwrap();
        std::fs::write(
            tmp.path().join(".vds/config.toml"),
            default_config("demo", "DEMO"),
        )
        .unwrap();
        for (route, source) in screens {
            let path = tmp.path().join(format!("app/{route}/page.tsx"));
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, source).unwrap();
        }
        let project = Project::discover(Some(tmp.path())).unwrap();
        Fixture { _tmp: tmp, project }
    }

    fn f_write(f: &Fixture, rel: &str, contents: &str) {
        let path = f.project.root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    const DASH: &str = r#"
import { Button, Card } from "@/components/ui";
export default function Page() {
  return <div><Card><Button /></Card></div>;
}
"#;

    #[test]
    fn generation_records_components_and_elements_apart() {
        let f = fixture(&[("dash", DASH)]);
        let ledger = generate(&f.project).unwrap();
        assert_eq!(ledger.screens.len(), 1);
        let kinds: Vec<&ReferenceKind> = ledger.screens[0]
            .references
            .iter()
            .map(|r| &r.kind)
            .collect();
        assert_eq!(
            kinds
                .iter()
                .filter(|k| ***k == ReferenceKind::Component)
                .count(),
            2
        );
        assert_eq!(
            kinds
                .iter()
                .filter(|k| ***k == ReferenceKind::Element)
                .count(),
            1
        );
    }

    #[test]
    fn generation_resolves_the_import_path_of_each_component() {
        let f = fixture(&[("dash", DASH)]);
        let ledger = generate(&f.project).unwrap();
        for (_, reference) in ledger.component_references() {
            assert_eq!(reference.import_path.as_deref(), Some("@/components/ui"));
        }
    }

    #[test]
    fn a_locally_defined_component_says_why_it_has_no_import_path() {
        let f = fixture(&[(
            "local",
            "function Local() { return <span />; }\nexport default function P(){ return <Local />; }\n",
        )]);
        let ledger = generate(&f.project).unwrap();
        let local = ledger
            .component_references()
            .find(|(_, r)| r.root == "Local")
            .unwrap()
            .1;
        assert!(local.import_path.is_none());
        assert!(
            local
                .unresolved_because
                .as_deref()
                .unwrap()
                .contains("not imported"),
            "{:?}",
            local.unresolved_because
        );
    }

    /// VDS S-7(2)(1): same inputs, same output, same digest. Re-running the
    /// generator over unchanged screens must not move a digest a proof cites.
    #[test]
    fn regenerating_over_unchanged_screens_does_not_move_the_content_digest() {
        let f = fixture(&[("dash", DASH)]);
        let first = generate(&f.project).unwrap();
        let second = generate(&f.project).unwrap();
        assert_eq!(first.content_digest, second.content_digest);
        assert_eq!(first.source_digest, second.source_digest);
        assert_eq!(first.screens, second.screens);
    }

    #[test]
    fn the_content_digest_excludes_generated_at() {
        let f = fixture(&[("dash", DASH)]);
        let mut ledger = generate(&f.project).unwrap();
        let before = ledger.compute_content_digest().unwrap();
        ledger.generated_at = Timestamp::fixed(2000, 1, 1, 0, 0, 0);
        assert_eq!(before, ledger.compute_content_digest().unwrap());
    }

    #[test]
    fn a_fresh_ledger_passes_the_staleness_test() {
        let f = fixture(&[("dash", DASH)]);
        write(&f.project).unwrap();
        load_fresh(&f.project).unwrap();
    }

    #[test]
    fn a_changed_screen_makes_the_ledger_stale_and_names_the_file() {
        let f = fixture(&[("dash", DASH)]);
        write(&f.project).unwrap();
        std::fs::write(
            f.project.root.join("app/dash/page.tsx"),
            format!("{DASH}// edited\n"),
        )
        .unwrap();
        let err = load_fresh(&f.project).unwrap_err();
        assert!(
            err.to_string().contains("changed since generation"),
            "{err}"
        );
        assert!(err.to_string().contains("app/dash/page.tsx"), "{err}");
    }

    #[test]
    fn an_added_screen_makes_the_ledger_stale() {
        let f = fixture(&[("dash", DASH)]);
        write(&f.project).unwrap();
        let path = f.project.root.join("app/extra/page.tsx");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, DASH).unwrap();
        let err = load_fresh(&f.project).unwrap_err();
        assert!(err.to_string().contains("added since generation"), "{err}");
    }

    #[test]
    fn a_removed_screen_makes_the_ledger_stale() {
        let f = fixture(&[("dash", DASH), ("other", DASH)]);
        write(&f.project).unwrap();
        std::fs::remove_file(f.project.root.join("app/other/page.tsx")).unwrap();
        let err = load_fresh(&f.project).unwrap_err();
        assert!(
            err.to_string().contains("removed since generation"),
            "{err}"
        );
    }

    #[test]
    fn changing_the_globs_makes_the_ledger_stale() {
        let f = fixture(&[("dash", DASH)]);
        write(&f.project).unwrap();
        let config = f.project.root.join(".vds/config.toml");
        let text = std::fs::read_to_string(&config)
            .unwrap()
            .replace(r#"["app/**/page.tsx"]"#, r#"["app/**/route.tsx"]"#);
        std::fs::write(&config, text).unwrap();
        let reloaded = Project::discover(Some(&f.project.root)).unwrap();
        let err = load_fresh(&reloaded).unwrap_err();
        assert!(err.to_string().contains("screen_globs changed"), "{err}");
    }

    /// The defect that motivates `content_digest`. Comparing only source
    /// digests answers "have the screens changed", not "was this produced from
    /// them", so a hand-edited ledger passes and a proof reading it can be
    /// flipped from failing to passing without touching a screen.
    #[test]
    fn a_hand_edited_ledger_is_refused_even_though_its_sources_are_untouched() {
        let f = fixture(&[("dash", DASH)]);
        let (path, mut ledger) = write(&f.project).unwrap();

        // Delete a component reference, exactly as someone would to silence a
        // composition failure, and leave source_digest alone.
        ledger.screens[0]
            .references
            .retain(|r| r.kind != ReferenceKind::Component);
        std::fs::write(&path, serde_yaml::to_string(&ledger).unwrap()).unwrap();

        let err = load_fresh(&f.project).unwrap_err();
        assert!(err.to_string().contains("was edited"), "{err}");
    }

    #[test]
    fn a_hand_edited_ledger_with_a_recomputed_content_digest_is_still_refused() {
        let f = fixture(&[("dash", DASH)]);
        let (path, mut ledger) = write(&f.project).unwrap();
        ledger.screens[0]
            .references
            .retain(|r| r.kind != ReferenceKind::Component);
        // The forger recomputes the content digest so the file is internally
        // consistent. Regeneration still disagrees with it.
        ledger.content_digest = ledger.compute_content_digest().unwrap();
        std::fs::write(&path, serde_yaml::to_string(&ledger).unwrap()).unwrap();

        let err = load_fresh(&f.project).unwrap_err();
        assert!(err.to_string().contains("was edited"), "{err}");
    }

    #[test]
    fn an_absent_ledger_says_what_to_run() {
        let f = fixture(&[("dash", DASH)]);
        let err = load_fresh(&f.project).unwrap_err();
        assert!(err.to_string().contains(GENERATOR_COMMAND), "{err}");
    }

    #[test]
    fn a_ledger_from_the_future_is_refused_rather_than_partially_read() {
        let f = fixture(&[("dash", DASH)]);
        let (path, _) = write(&f.project).unwrap();
        let text = std::fs::read_to_string(&path)
            .unwrap()
            .replace("schema_version: 1", "schema_version: 99");
        std::fs::write(&path, text).unwrap();
        let err = load_fresh(&f.project).unwrap_err();
        assert!(err.to_string().contains("VDS S-11(2)"), "{err}");
    }

    /// Rewriting one import specifier to a relative path took a governed
    /// component out of enforcement entirely, because the specifier no longer
    /// started with a governed prefix and the row was skipped as ungoverned.
    #[test]
    fn a_relative_import_of_a_governed_component_is_still_governed() {
        let f = fixture(&[]);
        std::fs::create_dir_all(f.project.root.join("src/components/ui")).unwrap();
        f_write(
            &f,
            "app/dash/page.tsx",
            "import { Button } from \"../../src/components/ui/button\";\n\
             export default function P(){ return <Button />; }\n",
        );
        let ledger = generate(&f.project).unwrap();
        let reference = ledger.component_references().next().unwrap().1;

        assert_eq!(
            reference.resolved_path.as_deref(),
            Some("src/components/ui/button")
        );
        let prefixes = vec!["@/components/".to_string()];
        let dirs = vec!["src/components/ui".to_string()];
        assert!(
            !reference
                .import_path
                .as_deref()
                .unwrap()
                .starts_with(&prefixes[0]),
            "the specifier as written escapes the prefix check, which is the point"
        );
        assert!(
            reference.is_governed(&prefixes, &dirs),
            "and resolving it against the library directory brings it back in"
        );
    }

    #[test]
    fn a_relative_import_of_something_outside_the_library_is_not_governed() {
        let f = fixture(&[]);
        f_write(
            &f,
            "app/dash/page.tsx",
            "import { Chart } from \"../vendor/chart\";\n\
             export default function P(){ return <Chart />; }\n",
        );
        let ledger = generate(&f.project).unwrap();
        let reference = ledger.component_references().next().unwrap().1;
        assert!(!reference.is_governed(
            &["@/components/".to_string()],
            &["src/components/ui".to_string()]
        ));
    }

    #[test]
    fn a_bare_package_specifier_resolves_to_nothing() {
        let f = fixture(&[]);
        f_write(
            &f,
            "app/dash/page.tsx",
            "import { Chart } from \"third-party\";\n\
             export default function P(){ return <Chart />; }\n",
        );
        let ledger = generate(&f.project).unwrap();
        assert_eq!(
            ledger
                .component_references()
                .next()
                .unwrap()
                .1
                .resolved_path,
            None
        );
    }

    #[test]
    fn a_namespace_member_is_verified_and_a_compound_member_is_not() {
        let f = fixture(&[]);
        f_write(
            &f,
            "app/ns/page.tsx",
            "import * as Icons from \"@/components/ui\";\n\
             export default function P(){ return <Icons.Chevron />; }\n",
        );
        f_write(
            &f,
            "app/compound/page.tsx",
            "import { Card } from \"@/components/ui\";\n\
             export default function P(){ return <Card.Header />; }\n",
        );
        let ledger = generate(&f.project).unwrap();

        let namespaced = ledger
            .component_references()
            .find(|(_, r)| r.name == "Icons.Chevron")
            .unwrap()
            .1;
        assert!(namespaced.namespace_member);
        assert_eq!(namespaced.lookup_name(), "Chevron");
        assert!(
            !namespaced.member_is_unverified(),
            "a namespace member resolves to a real export and IS checked"
        );

        let compound = ledger
            .component_references()
            .find(|(_, r)| r.name == "Card.Header")
            .unwrap()
            .1;
        assert!(!compound.namespace_member);
        assert_eq!(compound.lookup_name(), "Card");
        assert!(
            compound.member_is_unverified(),
            "the register knows Card; Header is a property of it that no coordinate names"
        );
    }

    #[test]
    fn the_ledger_records_the_export_name_a_lookup_needs() {
        let f = fixture(&[]);
        f_write(
            &f,
            "app/dash/page.tsx",
            "import { Button as Btn } from \"@/components/ui\";\n\
             export default function P(){ return <Btn />; }\n",
        );
        let ledger = generate(&f.project).unwrap();
        let reference = ledger.component_references().next().unwrap().1;
        assert_eq!(reference.root, "Btn");
        assert_eq!(reference.lookup_name(), "Button");
    }

    /// The critical case an adversarial reviewer found: a stray backtick in JSX
    /// text opened a template literal that never closed, every reference after
    /// it vanished, and the ledger recorded no skip, no note and no finding.
    #[test]
    fn a_screen_the_scanner_cannot_read_completely_is_refused_not_recorded() {
        let f = fixture(&[]);
        f_write(
            &f,
            "app/dash/page.tsx",
            "import { Button } from \"@/components/ui\";\n\
             const broken = `unterminated;\n\
             export default function P(){ return <Button />; }\n",
        );
        let error = generate(&f.project).unwrap_err();
        assert!(
            error.to_string().contains("not counted anywhere"),
            "{error}"
        );
        assert!(error.to_string().contains("app/dash/page.tsx"), "{error}");
    }

    #[test]
    fn a_typescript_generic_is_not_read_as_an_opening_tag() {
        // The two lines below are a delta-debugged minimum from a real 544-line page
        // that this scanner REFUSED to read, reporting "a template literal was opened
        // and never closed" against valid TypeScript. It took out four proofs at once,
        // because all of them depend on a screens ledger the scan would not build.
        //
        // The mechanism: `useState<string | null>` looks exactly like an opening tag on
        // its first two characters, so `<s` set in_tag and the `>` moved the region to
        // JsxText. In JsxText a `//` is not a comment, so the `{` inside the comment's
        // backticks pushed a brace and the next backtick opened a template with nothing
        // left to close it.
        //
        // Asserting the ledger BUILDS is the point. A file that cannot be read is not a
        // file with zero references, and the difference is what the unbalanced check is
        // for.
        let f = fixture(&[]);
        f_write(
            &f,
            "app/dash/page.tsx",
            "import { Button } from \"@/components/ui\";\n\
             export default function P(){\n\
             const [e, setE] = useState<string | null>(null);\n\
             // holds (`a/page.tsx:1-2`, `loading || error ? null : {`).\n\
             return <div><Button /></div>; }\n",
        );
        let ledger = generate(&f.project).expect("a valid TS generic must not refuse the scan");
        let names: Vec<&str> = ledger
            .component_references()
            .map(|(_, r)| r.name.as_str())
            .collect();
        assert!(names.contains(&"Button"), "got {names:?}");
    }

    #[test]
    fn an_apostrophe_in_jsx_text_does_not_hide_the_rest_of_the_line() {
        let f = fixture(&[]);
        f_write(
            &f,
            "app/dash/page.tsx",
            "import { Button, Card } from \"@/components/ui\";\n\
             export default function P(){ return <div><p>it's fine</p><Button /><Card /></div>; }\n",
        );
        let ledger = generate(&f.project).unwrap();
        let names: Vec<&str> = ledger
            .component_references()
            .map(|(_, r)| r.name.as_str())
            .collect();
        assert_eq!(names, vec!["Button", "Card"]);
    }

    #[test]
    fn a_backtick_in_jsx_text_does_not_swallow_the_rest_of_the_file() {
        let f = fixture(&[]);
        f_write(
            &f,
            "app/dash/page.tsx",
            "import { Button, Card } from \"@/components/ui\";\n\
             export default function P(){ return <div><p>press `Enter`</p><Button /><Card /></div>; }\n",
        );
        let ledger = generate(&f.project).unwrap();
        assert_eq!(ledger.component_references().count(), 2);
    }

    #[test]
    fn a_real_template_literal_in_code_is_still_blanked() {
        let f = fixture(&[]);
        f_write(
            &f,
            "app/dash/page.tsx",
            "import { Button } from \"@/components/ui\";\n\
             const label = `see <Card /> here`;\n\
             export default function P(){ return <Button />; }\n",
        );
        let ledger = generate(&f.project).unwrap();
        let names: Vec<&str> = ledger
            .component_references()
            .map(|(_, r)| r.name.as_str())
            .collect();
        assert_eq!(names, vec!["Button"], "a template in CODE is still not JSX");
    }

    #[test]
    fn routes_consuming_counts_each_route_once() {
        let f = fixture(&[(
            "dash",
            "import { Button } from \"@/components/ui\";\nexport default function P(){ return <div><Button /><Button /></div>; }\n",
        )]);
        let ledger = generate(&f.project).unwrap();
        assert_eq!(
            ledger.routes_consuming("@/components/ui", "Button"),
            vec!["app/dash/page.tsx"]
        );
    }
}
