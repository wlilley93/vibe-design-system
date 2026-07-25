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
    pub governance: Governance,
}

/// Where each kind of record lives, by role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Paths {
    pub register: PathBuf,
    pub warrants: PathBuf,
    pub proofs: PathBuf,
    pub pins: PathBuf,
    pub ledgers: PathBuf,
    pub submissions: PathBuf,
    pub logs: PathBuf,
    pub permits: PathBuf,
}

impl Default for Paths {
    fn default() -> Self {
        Self {
            register: ".vds/register".into(),
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
        assert_eq!(config.governance, Governance::default());
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
