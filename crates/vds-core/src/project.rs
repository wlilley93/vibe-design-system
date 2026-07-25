//! Project discovery and the atomic write.
//!
//! Two things live here that every other crate depends on. The first is finding
//! the project: `.vds/config.toml` is the one fixed anchor (VDS S-3(7)), and
//! discovery walks up from a starting directory looking for it.
//!
//! The second is [`write_atomically`]. The CLI's refusal banner says "VDS
//! REFUSED, and did nothing", and that sentence has to be true. A write that
//! truncates a file and then fails leaves a governance record half-written,
//! which is worse than either outcome the caller was choosing between. Every
//! write in VDS therefore goes to a temporary file in the destination directory
//! and is renamed into place, which is atomic within a filesystem.

use std::path::{Path, PathBuf};

use crate::config::{Config, PathRole};
use crate::digest::Digest;
use crate::error::{Result, VdsError};

pub const VDS_DIR: &str = ".vds";
pub const CONFIG_FILE: &str = "config.toml";

/// A project VDS can operate on: a root, and the config found at its anchor.
#[derive(Debug, Clone)]
pub struct Project {
    pub root: PathBuf,
    pub config: Config,
    pub config_path: PathBuf,
}

impl Project {
    /// Walk up from `start` looking for `.vds/config.toml`.
    pub fn discover(start: Option<&Path>) -> Result<Project> {
        let here = match start {
            Some(path) => path.to_path_buf(),
            None => std::env::current_dir().map_err(|e| VdsError::io("the current directory", e))?,
        };
        let here = here
            .canonicalize()
            .unwrap_or_else(|_| here.clone());

        for candidate in here.ancestors() {
            let config_path = candidate.join(VDS_DIR).join(CONFIG_FILE);
            if config_path.is_file() {
                let text = std::fs::read_to_string(&config_path)
                    .map_err(|e| VdsError::io(config_path.display(), e))?;
                let config = Config::parse(&text, &config_path.display().to_string())?;
                return Ok(Project {
                    root: candidate.to_path_buf(),
                    config,
                    config_path,
                });
            }
        }
        Err(VdsError::NoProject(here.display().to_string()))
    }

    pub fn vds_dir(&self) -> PathBuf {
        self.root.join(VDS_DIR)
    }

    pub fn path(&self, role: PathRole) -> PathBuf {
        self.root.join(self.config.role(role))
    }

    pub fn screens_ledger_path(&self) -> PathBuf {
        self.root.join(&self.config.surface.screens_ledger)
    }

    pub fn enforcement_lock_path(&self) -> PathBuf {
        self.vds_dir().join(crate::types::LOCK_FILE_NAME)
    }

    pub fn designpack_lock_path(&self) -> PathBuf {
        self.vds_dir().join("designpack.lock")
    }

    /// A path relative to the project root, for printing.
    ///
    /// Falls back to the absolute path where the target is genuinely outside the
    /// root. Printing a misleading relative path would be worse than printing a
    /// long one.
    pub fn rel(&self, path: &Path) -> String {
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };
        let root = self.root.canonicalize().unwrap_or_else(|_| self.root.clone());
        let target = absolute.canonicalize().unwrap_or(absolute);
        match target.strip_prefix(&root) {
            Ok(rest) => rest.to_string_lossy().replace('\\', "/"),
            Err(_) => target.to_string_lossy().into_owned(),
        }
    }

    /// The digest of the register, as a set of `(relative path, file digest)`
    /// rows. Half of the surface a warrant is granted over (VDS S-6(4)).
    pub fn register_digest(&self) -> Result<Digest> {
        let directory = self.path(PathRole::Register);
        let mut rows = Vec::new();
        if directory.is_dir() {
            let mut files: Vec<PathBuf> = std::fs::read_dir(&directory)
                .map_err(|e| VdsError::io(directory.display(), e))?
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("yaml"))
                .collect();
            files.sort();
            for file in files {
                rows.push((self.rel(&file), Digest::of_file(&file)?));
            }
        }
        crate::digest::digest_rows(&rows)
    }
}

/// Write `bytes` to `path` through a temporary file in the same directory.
///
/// The rename is what makes the CLI's "and did nothing" banner honest: a reader
/// either sees the previous bytes or the new ones, and never a truncated file.
pub fn write_atomically(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().ok_or_else(|| {
        VdsError::precondition(format!("{} has no parent directory", path.display()))
    })?;
    std::fs::create_dir_all(parent).map_err(|e| VdsError::io(parent.display(), e))?;

    let mut temp = tempfile::NamedTempFile::new_in(parent)
        .map_err(|e| VdsError::io(parent.display(), e))?;
    {
        use std::io::Write;
        temp.write_all(bytes)
            .map_err(|e| VdsError::io(path.display(), e))?;
        temp.flush().map_err(|e| VdsError::io(path.display(), e))?;
    }
    temp.persist(path)
        .map_err(|e| VdsError::io(path.display(), e.error))?;
    Ok(())
}

pub fn write_text_atomically(path: &Path, text: &str) -> Result<()> {
    write_atomically(path, text.as_bytes())
}

/// Every `*.yaml` in a directory, sorted. An absent directory is empty, not an
/// error: a project with no warrants yet has no warrants, which is a fact and
/// not a fault.
pub fn yaml_files(directory: &Path) -> Result<Vec<PathBuf>> {
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    let mut files: Vec<PathBuf> = std::fs::read_dir(directory)
        .map_err(|e| VdsError::io(directory.display(), e))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("yaml"))
        .collect();
    files.sort();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::default_config;

    fn scaffold() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        let vds = tmp.path().join(".vds");
        std::fs::create_dir_all(&vds).unwrap();
        std::fs::write(vds.join("config.toml"), default_config("demo", "DEMO")).unwrap();
        tmp
    }

    #[test]
    fn discovery_walks_up_to_the_anchor() {
        let tmp = scaffold();
        let deep = tmp.path().join("a/b/c");
        std::fs::create_dir_all(&deep).unwrap();
        let project = Project::discover(Some(&deep)).unwrap();
        assert_eq!(
            project.root.canonicalize().unwrap(),
            tmp.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn discovery_says_what_to_run_when_there_is_no_project() {
        let tmp = tempfile::tempdir().unwrap();
        let err = Project::discover(Some(tmp.path())).unwrap_err();
        assert!(err.to_string().contains("vds init"), "{err}");
    }

    #[test]
    fn register_digest_of_an_absent_register_is_stable() {
        let tmp = scaffold();
        let project = Project::discover(Some(tmp.path())).unwrap();
        let empty = project.register_digest().unwrap();
        assert_eq!(empty, project.register_digest().unwrap());
    }

    #[test]
    fn register_digest_moves_when_a_record_moves() {
        let tmp = scaffold();
        let project = Project::discover(Some(tmp.path())).unwrap();
        let register = project.path(PathRole::Register);
        std::fs::create_dir_all(&register).unwrap();
        std::fs::write(register.join("CMP-0001.yaml"), "id: CMP-0001\n").unwrap();
        let before = project.register_digest().unwrap();
        std::fs::write(register.join("CMP-0001.yaml"), "id: CMP-0001\nname: x\n").unwrap();
        assert_ne!(before, project.register_digest().unwrap());
    }

    #[test]
    fn register_digest_ignores_a_non_yaml_file() {
        let tmp = scaffold();
        let project = Project::discover(Some(tmp.path())).unwrap();
        let register = project.path(PathRole::Register);
        std::fs::create_dir_all(&register).unwrap();
        std::fs::write(register.join("CMP-0001.yaml"), "id: CMP-0001\n").unwrap();
        let before = project.register_digest().unwrap();
        std::fs::write(register.join("notes.md"), "scratch").unwrap();
        assert_eq!(before, project.register_digest().unwrap());
    }

    #[test]
    fn an_atomic_write_replaces_the_bytes_whole() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("nested/deeply/file.yaml");
        write_text_atomically(&target, "first").unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "first");
        write_text_atomically(&target, "second").unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "second");
    }

    #[test]
    fn an_atomic_write_leaves_no_temporary_behind() {
        let tmp = tempfile::tempdir().unwrap();
        write_text_atomically(&tmp.path().join("a.yaml"), "x").unwrap();
        let names: Vec<String> = std::fs::read_dir(tmp.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["a.yaml".to_string()]);
    }

    #[test]
    fn rel_falls_back_to_an_absolute_path_outside_the_root() {
        let tmp = scaffold();
        let project = Project::discover(Some(tmp.path())).unwrap();
        let outside = Path::new("/etc/hostname");
        assert_eq!(project.rel(outside), "/etc/hostname");
    }
}
