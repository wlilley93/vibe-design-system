//! `.vds/config.toml`: the one fixed anchor (VDS S-3(7)).
//!
//! Every other path is configurable from here by role. This file holds NO design
//! value (VDS S-2(2)): paths, globs and governance only. There is no field on
//! [`Config`] that could hold a colour, a length, a font, a duration or an easing
//! curve, and `no_stored_values` re-checks the bytes rather than trusting the
//! type.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Result, VdsError};

pub const CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub version: u32,
    pub jurisdiction_id: String,
    pub repo_code: String,
    /// `<id>@<version>` of the designpack this project subscribes to.
    pub designpack: String,
    #[serde(default)]
    pub paths: Paths,
    #[serde(default)]
    pub surface: SurfaceConfig,
    #[serde(default)]
    pub screens: ScreensConfig,
    #[serde(default)]
    pub geometry: GeometryConfig,
    #[serde(default)]
    pub governance: Governance,
}

/// Where each kind of record lives, by role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Paths {
    pub register: PathBuf,
    /// The screen register: one record per governed screen (VDS S-5A).
    ///
    /// `#[serde(default)]` and not a required field, because every config
    /// written before the screen record existed omits it and refusing those
    /// would make an amendment that adds an artefact kind a flag day for every
    /// adopting project.
    #[serde(default = "default_screens_dir")]
    pub screens: PathBuf,
    /// The geometry bounds: one record per SURFACE KIND (VDS S-7A(3)).
    ///
    /// Defaulted for the same reason `screens` is: a config written before the
    /// twelfth proof kind existed omits it, and refusing those would make an
    /// amendment a flag day for every adopting project.
    #[serde(default = "default_geometry_dir")]
    pub geometry: PathBuf,
    pub warrants: PathBuf,
    pub proofs: PathBuf,
    pub pins: PathBuf,
    pub ledgers: PathBuf,
    pub submissions: PathBuf,
    pub logs: PathBuf,
    pub permits: PathBuf,
}

fn default_screens_dir() -> PathBuf {
    PathBuf::from(".vds/screens")
}

fn default_geometry_dir() -> PathBuf {
    PathBuf::from(".vds/geometry")
}

impl Default for Paths {
    fn default() -> Self {
        Self {
            register: ".vds/register".into(),
            screens: default_screens_dir(),
            geometry: default_geometry_dir(),
            warrants: ".vds/warrants".into(),
            proofs: ".vds/proofs".into(),
            pins: ".vds/pins".into(),
            ledgers: ".vds/ledgers".into(),
            submissions: ".vds/submissions".into(),
            logs: ".vds/logs".into(),
            permits: ".vds/permits".into(),
        }
    }
}

/// The DECLARED SURFACE. Every VDS claim is bounded by it, and a screen outside
/// these globs is outside every proof.
///
/// docs/GOAL.md is explicit that "no unregistered component anywhere" is not
/// provable: a finite check proves the modelled paths and never the absence of
/// an unmodelled one. This struct is where that boundary is drawn, and every
/// warrant names it by digest so the boundary is provable after the fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceConfig {
    pub screen_globs: Vec<String>,
    /// A component reference whose import path starts with one of these is IN
    /// SCOPE for enforcement. Anything else is counted, not enforced, and the
    /// count is printed, so the carve-out is visible rather than assumed.
    pub governed_import_prefixes: Vec<String>,
    /// Directories the register is expected to cover, used by reconciliation.
    pub library_dirs: Vec<String>,
    pub screens_ledger: PathBuf,
    /// File extensions the library scan treats as a component module.
    #[serde(default = "default_component_extensions")]
    pub component_extensions: Vec<String>,
    /// Bare elements ABOVE the primitive floor this project nonetheless uses
    /// directly, each one a named exception ([2026] VJS-CC-VIBE-DESIGN-SYSTEM 5).
    /// Every use is counted and reported by site, mirroring
    /// `governed_import_prefixes`: a carve-out working as intended and one being
    /// used as an escape hatch produce the same count, and only visibility tells
    /// them apart.
    #[serde(default)]
    pub element_carveouts: Vec<String>,
    /// The shipped stylesheet: the record VDS S-2(3) fixes as the system of
    /// record for what a token resolves to. The `contrast` proof measures every
    /// floor against it.
    ///
    /// Configurable, and `#[serde(default)]` so an existing config keeps working.
    /// The default is the path S-2(3) names, so a project that follows the
    /// specification writes nothing and a project that ships its tokens from
    /// somewhere else says so ONCE, here, rather than being silently unmeasured.
    ///
    /// It is deliberately not mined out of `[governance] permit_required`. That
    /// list declares what a permit covers, so adding a stylesheet to it would
    /// change what `contrast` measures as a side effect, and a gate whose subject
    /// moves when an unrelated list is edited is a gate nobody can reason about.
    #[serde(default = "default_stylesheet")]
    pub stylesheet: PathBuf,
}

fn default_stylesheet() -> PathBuf {
    PathBuf::from("app/globals.css")
}

fn default_component_extensions() -> Vec<String> {
    vec!["tsx".into(), "jsx".into()]
}

impl Default for SurfaceConfig {
    fn default() -> Self {
        Self {
            screen_globs: vec!["app/**/page.tsx".into()],
            governed_import_prefixes: vec!["@/components/".into()],
            library_dirs: vec!["src/components/ui".into()],
            screens_ledger: ".vds/ledgers/screens.yaml".into(),
            component_extensions: default_component_extensions(),
            element_carveouts: Vec::new(),
            stylesheet: default_stylesheet(),
        }
    }
}

/// THE SCREEN SEAM. Everything about a screen frame that is the SUBJECT's
/// vocabulary rather than VDS's.
///
/// A screen frame is not one drawing. It carries several and says in a layer
/// NAME which one governs, and the words it uses to say so belong to the project
/// that drew it. Hard-coding them here would make VDS an authority on what a
/// design file may call its own layers, which is the fourth authority
/// [2026] VJS-CC-OPBOX 3 forbids. So they are configured, and the defaults are
/// the ones actually observed in the subject this capability was derived from
/// rather than words invented for a specification.
///
/// What is NOT here, and deliberately: the geometry thresholds the column
/// derivation uses. Those are lengths, and a length under `.vds/**` is the
/// storing form VDS S-2(2) prohibits and `no_stored_values` R3 would fail on
/// forever. They live as constants in the generator
/// (`crates/vds-figma/src/frames.rs`), which is code and not a record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScreensConfig {
    /// A layer whose name carries one of these GOVERNS its frame, in this order
    /// of precedence: a frame carrying both a current-source layer and a legacy
    /// underlay must resolve to the former.
    ///
    /// Matched ANYWHERE in the name and case-insensitively. That asymmetry with
    /// `quarantine_markers` is not a compromise, it is what a real file says:
    /// one authoritative layer in the subject is named
    /// `/dashboards - current source matter master-detail`, with its marker
    /// mid-name and in lower case, and anchoring the match to the start
    /// resolved the very route the whole workstream began from to the wrong
    /// layer.
    pub authority_markers: Vec<String>,
    /// A layer whose name LEADS with one of these is never a build base.
    ///
    /// Matched against the LEADING SEGMENT only (see `name_separator`), and this
    /// is the subtle half. Matching anywhere produced nine false exclusions in
    /// the subject, because a hybrid name like
    /// `Screen - /matters/[id] - Profile - source contract + target recovery`
    /// is a CURRENT screen that merely mentions a target, and excluding it took
    /// two of the busiest surfaces in the product out of the contract on a word
    /// in a sentence.
    pub quarantine_markers: Vec<String>,
    /// What separates the segments of a layer name, for the leading-segment
    /// test above. A single character in every file seen; configured because
    /// the middot is a convention and not a law.
    pub name_separator: String,
    /// The shell regions the generator looks for by name, in the subject's own
    /// vocabulary.
    ///
    /// A screen record's `arrangement.regions` is checked against what the
    /// generator found, so the two halves must read ONE list, and this is it.
    pub region_names: Vec<String>,
    /// Where the frame ledger is written.
    pub frames_ledger: PathBuf,
}

impl Default for ScreensConfig {
    fn default() -> Self {
        Self {
            authority_markers: vec![
                "CURRENT SOURCE".into(),
                "SOURCE AUTHORITY".into(),
                "CURRENT CODE".into(),
            ],
            quarantine_markers: vec![
                "LEGACY UNDERLAY".into(),
                "REFERENCE".into(),
                "TARGET".into(),
            ],
            name_separator: "·".into(),
            region_names: vec![
                "rail".into(),
                "cmdbar".into(),
                "body".into(),
                "statusbar".into(),
            ],
            frames_ledger: ".vds/ledgers/frames.yaml".into(),
        }
    }
}

/// Where the geometry reading is written, and nothing else.
///
/// Deliberately thin. The reader's THRESHOLDS - which radii are the system's,
/// which boundary weights, which spacing steps - are not here and must not come
/// here. That is the subject's design system talking, and VDS deciding what
/// counts as a compliant radius would be VDS becoming a fourth design authority,
/// which [2026] VJS-CC-OPBOX 3 forbids. VDS holds the BOUND and the DIRECTION;
/// the subject's generator holds what compliance means.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeometryConfig {
    /// Where the generated geometry reading is written (VDS S-7A(4)).
    pub reading_ledger: PathBuf,
}

impl Default for GeometryConfig {
    fn default() -> Self {
        Self {
            reading_ledger: ".vds/ledgers/geometry.yaml".into(),
        }
    }
}

/// VDS S-3(8): the enforcement machinery must not be editable without a permit,
/// or the gate can be removed by the same hand it constrains.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Governance {
    pub permit_required: Vec<String>,
    /// The append-only record directories.
    pub permit_exempt: Vec<String>,
}

impl Default for Governance {
    fn default() -> Self {
        Self {
            permit_required: vec![
                "app/globals.css".into(),
                "src/components/**".into(),
                "designpack/v1/**".into(),
                ".vds/register/**".into(),
                ".vds/config.toml".into(),
            ],
            permit_exempt: vec![
                ".vds/logs/**".into(),
                ".vds/permits/**".into(),
                ".vds/proofs/**".into(),
            ],
        }
    }
}

/// Which role a path belongs to. An enum rather than a string key, so a typo in
/// a role name is a compile error and not a runtime `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathRole {
    Register,
    Screens,
    Geometry,
    Warrants,
    Proofs,
    Pins,
    Ledgers,
    Submissions,
    Logs,
    Permits,
}

impl PathRole {
    pub fn as_str(self) -> &'static str {
        match self {
            PathRole::Register => "register",
            PathRole::Screens => "screens",
            PathRole::Geometry => "geometry",
            PathRole::Warrants => "warrants",
            PathRole::Proofs => "proofs",
            PathRole::Pins => "pins",
            PathRole::Ledgers => "ledgers",
            PathRole::Submissions => "submissions",
            PathRole::Logs => "logs",
            PathRole::Permits => "permits",
        }
    }
}

impl Config {
    pub fn role(&self, role: PathRole) -> &Path {
        match role {
            PathRole::Register => &self.paths.register,
            PathRole::Screens => &self.paths.screens,
            PathRole::Geometry => &self.paths.geometry,
            PathRole::Warrants => &self.paths.warrants,
            PathRole::Proofs => &self.paths.proofs,
            PathRole::Pins => &self.paths.pins,
            PathRole::Ledgers => &self.paths.ledgers,
            PathRole::Submissions => &self.paths.submissions,
            PathRole::Logs => &self.paths.logs,
            PathRole::Permits => &self.paths.permits,
        }
    }

    pub fn parse(text: &str, where_from: &str) -> Result<Config> {
        let config: Config =
            toml::from_str(text).map_err(|e| VdsError::parse(where_from, "TOML", e.message()))?;
        if config.version > CONFIG_VERSION {
            return Err(VdsError::SchemaVersionAhead {
                path: where_from.to_owned(),
                kind: "config",
                found: config.version,
                understood: CONFIG_VERSION,
            });
        }
        config.check(where_from)?;
        Ok(config)
    }

    /// Refusals that must happen at LOAD, not at first use.
    ///
    /// A configuration that cannot bound anything is not a configuration with a
    /// small problem, it is a configuration that will make every proof vacuous
    /// and every vacuity look like the project's fault.
    fn check(&self, where_from: &str) -> Result<()> {
        if self.screen_globs_are_empty() {
            return Err(VdsError::precondition(format!(
                "{where_from}: [surface] screen_globs is empty. A declared surface of nothing \
                 proves nothing, so this is refused at load rather than producing a vacuous \
                 pass at every proof."
            )));
        }
        for role in [
            PathRole::Register,
            PathRole::Screens,
            PathRole::Geometry,
            PathRole::Warrants,
            PathRole::Proofs,
            PathRole::Pins,
            PathRole::Ledgers,
            PathRole::Submissions,
            PathRole::Logs,
            PathRole::Permits,
        ] {
            let path = self.role(role);
            if path.is_absolute() || path.components().any(|c| c.as_os_str() == "..") {
                return Err(VdsError::precondition(format!(
                    "{where_from}: [paths] {} is {}, which escapes the project root. Every \
                     record path is repository-relative.",
                    role.as_str(),
                    path.display()
                )));
            }
        }
        Ok(())
    }

    fn screen_globs_are_empty(&self) -> bool {
        self.surface
            .screen_globs
            .iter()
            .all(|glob| glob.trim().is_empty())
    }

    /// The designpack id and version, split from `<id>@<version>`.
    pub fn designpack_parts(&self) -> (&str, &str) {
        match self.designpack.split_once('@') {
            Some((id, version)) => (id, version),
            None => (self.designpack.as_str(), ""),
        }
    }
}

/// The template `vds init` writes.
pub const DEFAULT_CONFIG_TEMPLATE: &str = r#"# VDS project configuration. The one fixed anchor (VDS S-3(7)).
# This file holds NO design value (VDS S-2(2)). Paths, globs and governance only.
version = 1
jurisdiction_id = "{jurisdiction_id}"
repo_code = "{repo_code}"
designpack = "none@0"

[paths]
register = ".vds/register"
# The screen register (VDS S-5A): one record per governed screen, holding the
# ARRANGEMENT it requires. A count of content panes and a list of region names;
# never a width, because a width is a realisation (VDS S-2(4)).
screens = ".vds/screens"
warrants = ".vds/warrants"
proofs = ".vds/proofs"
pins = ".vds/pins"
ledgers = ".vds/ledgers"
submissions = ".vds/submissions"
logs = ".vds/logs"
permits = ".vds/permits"

[surface]
# The DECLARED SURFACE. Every VDS claim is bounded by it, and a screen outside
# these globs is outside every proof.
screen_globs = ["app/**/page.tsx"]
# A component reference whose import path starts with one of these is IN SCOPE
# for enforcement. Anything else is counted and not enforced, and the count is
# printed, so the carve-out is visible rather than assumed.
governed_import_prefixes = ["@/components/"]
# Directories the register is expected to cover, used by reconciliation.
library_dirs = ["src/components/ui"]
screens_ledger = ".vds/ledgers/screens.yaml"
component_extensions = ["tsx", "jsx"]
# The shipped stylesheet: the system of record for what a token RESOLVES to
# (VDS S-2(3)). `contrast` measures every floor in the register against it, in
# every theme it declares. The value here is the path the specification names;
# change it only if your project genuinely ships its tokens elsewhere, and know
# that nothing else is measured.
stylesheet = "app/globals.css"

[screens]
# THE SCREEN SEAM: everything about a screen frame that is THIS PROJECT's
# vocabulary rather than VDS's. A screen frame carries several drawings and says
# in a layer NAME which one governs.
#
# An authority marker is matched ANYWHERE in the name and case-insensitively; a
# quarantine marker is matched against the LEADING SEGMENT only. That asymmetry
# is load-bearing and is not a compromise: an authoritative layer in the subject
# this was derived from carries its marker mid-name and lower case, while a
# hybrid name that merely MENTIONS a target is a current screen, and matching
# quarantine anywhere excluded nine of them.
authority_markers = ["CURRENT SOURCE", "SOURCE AUTHORITY", "CURRENT CODE"]
quarantine_markers = ["LEGACY UNDERLAY", "REFERENCE", "TARGET"]
name_separator = "\u00b7"
# The shell regions the generator looks for by name. A screen record's required
# regions are checked against what it found, so both halves read THIS list.
region_names = ["rail", "cmdbar", "body", "statusbar"]
frames_ledger = ".vds/ledgers/frames.yaml"

[governance]
# VDS S-3(8): the enforcement machinery must not be editable without a permit.
permit_required = [
  "app/globals.css",
  "src/components/**",
  "designpack/v1/**",
  ".vds/register/**",
  ".vds/config.toml",
]
permit_exempt = [".vds/logs/**", ".vds/permits/**", ".vds/proofs/**"]
"#;

pub fn default_config(jurisdiction_id: &str, repo_code: &str) -> String {
    DEFAULT_CONFIG_TEMPLATE
        .replace("{jurisdiction_id}", jurisdiction_id)
        .replace("{repo_code}", repo_code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_template_parses_into_the_defaults() {
        let config = Config::parse(&default_config("demo", "DEMO"), "<template>").unwrap();
        assert_eq!(config.version, CONFIG_VERSION);
        assert_eq!(config.jurisdiction_id, "demo");
        assert_eq!(config.repo_code, "DEMO");
        assert_eq!(config.paths, Paths::default());
        assert_eq!(config.surface, SurfaceConfig::default());
        assert_eq!(config.screens, ScreensConfig::default());
        assert_eq!(config.governance, Governance::default());
    }

    /// An amendment that adds an artefact kind must not be a flag day for every
    /// project already using VDS. A config written before the screen record
    /// existed carries neither `[paths] screens` nor a `[screens]` section, and
    /// it has to keep loading with the defaults rather than being refused.
    #[test]
    fn a_config_written_before_screens_existed_still_loads() {
        let text = r#"
version = 1
jurisdiction_id = "demo"
repo_code = "DEMO"
designpack = "none@0"

[paths]
register = ".vds/register"
warrants = ".vds/warrants"
proofs = ".vds/proofs"
pins = ".vds/pins"
ledgers = ".vds/ledgers"
submissions = ".vds/submissions"
logs = ".vds/logs"
permits = ".vds/permits"
"#;
        let config = Config::parse(text, "old.toml").expect("an older config still loads");
        assert_eq!(config.paths.screens, default_screens_dir());
        assert_eq!(config.screens, ScreensConfig::default());
    }

    /// The middot is a convention rather than a law, so it is configured. This
    /// test is here because the template writes it as a TOML unicode escape,
    /// and an escape that silently produced the two characters `\u` would make
    /// every leading-segment test match nothing.
    #[test]
    fn the_template_writes_a_real_separator_and_not_its_escape() {
        let config = Config::parse(&default_config("demo", "DEMO"), "<template>").unwrap();
        assert_eq!(config.screens.name_separator, "·");
        assert_eq!(config.screens.name_separator.chars().count(), 1);
    }

    #[test]
    fn a_screens_path_escaping_the_root_is_refused_at_load() {
        let text = default_config("demo", "DEMO")
            .replace(r#"screens = ".vds/screens""#, r#"screens = "/etc""#);
        let err = Config::parse(&text, "c.toml").unwrap_err();
        assert!(
            err.to_string().contains("escapes the project root"),
            "{err}"
        );
    }

    #[test]
    fn a_future_config_version_is_refused_at_load() {
        let text = default_config("demo", "DEMO").replace("version = 1", "version = 2");
        let err = Config::parse(&text, "c.toml").unwrap_err();
        assert!(
            matches!(err, VdsError::SchemaVersionAhead { .. }),
            "VDS S-11(2): refuse what you cannot read; got {err}"
        );
    }

    #[test]
    fn an_empty_declared_surface_is_refused_at_load() {
        let text = default_config("demo", "DEMO")
            .replace(r#"screen_globs = ["app/**/page.tsx"]"#, "screen_globs = []");
        let err = Config::parse(&text, "c.toml").unwrap_err();
        assert!(err.to_string().contains("proves nothing"), "{err}");
    }

    #[test]
    fn a_path_escaping_the_root_is_refused_at_load() {
        let text = default_config("demo", "DEMO").replace(
            r#"register = ".vds/register""#,
            r#"register = "../elsewhere""#,
        );
        assert!(Config::parse(&text, "c.toml").is_err());
    }

    #[test]
    fn an_unknown_config_key_is_refused_rather_than_ignored() {
        let text = format!("{}\nsurprise = 1\n", default_config("demo", "DEMO"));
        assert!(
            Config::parse(&text, "c.toml").is_err(),
            "a silently ignored key is a setting the author believes is in force"
        );
    }

    #[test]
    fn the_designpack_splits_into_id_and_version() {
        let config = Config::parse(&default_config("demo", "DEMO"), "c.toml").unwrap();
        assert_eq!(config.designpack_parts(), ("none", "0"));
    }
}
