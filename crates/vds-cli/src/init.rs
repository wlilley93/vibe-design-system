//! `vds init`: scaffold a project's `.vds/`.

use clap::Args as ClapArgs;
use vds_core::{Result, VdsError, actor, default_config, write_text_atomically};

use crate::{Context, PASSED};

#[derive(ClapArgs)]
pub struct Args {
    /// The jurisdiction this project is. Defaults to the directory name.
    #[arg(long)]
    jurisdiction: Option<String>,
    /// The short code used in identifiers. Defaults to the directory name.
    #[arg(long)]
    repo_code: Option<String>,
    /// Overwrite an existing `.vds/config.toml`.
    #[arg(long)]
    force: bool,
}

/// The record directories `init` creates.
const RECORD_DIRS: &[&str] = &[
    "register",
    "warrants",
    "proofs",
    "pins",
    "ledgers",
    "submissions/draft",
    "submissions/filed",
    "submissions/docket",
    "court/convenings",
    "logs/decisions",
    "logs/breaches",
    "permits",
];

/// A jurisdiction id or repo code that is safe to interpolate into TOML.
///
/// The retired tool interpolated the argument raw, so
/// `init --jurisdiction 'acme "web"'` exited 0 and wrote a `config.toml` no
/// later command could parse. A malformed anchor is worse than a refused one:
/// every subsequent command fails with a parse error pointing at a file the
/// author did not write by hand.
fn check_identifier(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(VdsError::precondition(format!(
            "--{field} is empty. The anchor names this project in every record it writes."
        )));
    }
    let ok = value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.');
    if !ok {
        return Err(VdsError::precondition(format!(
            "--{field} {value:?} contains a character outside [A-Za-z0-9._-]. It is written \
             into .vds/config.toml, which is the one fixed anchor (VDS S-3(7)), and a quote \
             or a newline there produces a config no later command can parse."
        )));
    }
    Ok(())
}

pub fn run(ctx: &Context, args: &Args) -> Result<i32> {
    // Canonicalise before reading the directory name. `--root .` has no file
    // name, and the retired behaviour was to fall back to a placeholder, so a
    // project initialised from its own directory was called "project" in every
    // record it ever wrote.
    let root = ctx.init_root()?;
    let root = root.canonicalize().unwrap_or(root);
    let vds_dir = root.join(vds_core::project::VDS_DIR);
    let config_path = vds_dir.join(vds_core::project::CONFIG_FILE);

    if config_path.exists() && !args.force {
        return Err(VdsError::precondition(format!(
            "{} already exists. Pass --force to overwrite it.\n  \
             Overwriting an anchor silently would re-point every path role in a project \
             that already holds records under the old ones.",
            config_path.display()
        )));
    }

    let directory_name = root
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| {
            VdsError::precondition(
                "could not read a directory name from the project root, so there is nothing to \
                 name the jurisdiction after. Pass --jurisdiction and --repo-code explicitly.",
            )
        })?
        .to_owned();
    let jurisdiction = args
        .jurisdiction
        .clone()
        .unwrap_or_else(|| directory_name.clone());
    let repo_code = args
        .repo_code
        .clone()
        .unwrap_or_else(|| directory_name.to_uppercase().replace('-', "_"));

    check_identifier("jurisdiction", &jurisdiction)?;
    check_identifier("repo-code", &repo_code)?;

    // Everything is validated before the first write, so a refusal really did
    // nothing. Confirm the config parses BEFORE putting it on disk: writing an
    // anchor this build cannot read would be the defect this command exists to
    // avoid.
    let config_text = default_config(&jurisdiction, &repo_code);
    vds_core::Config::parse(&config_text, "<the config about to be written>")?;

    for relative in RECORD_DIRS {
        let path = vds_dir.join(relative);
        std::fs::create_dir_all(&path).map_err(|e| VdsError::io(path.display(), e))?;
    }
    write_text_atomically(&config_path, &config_text)?;

    // VDS S-3(9): the record is committed, not scratch. Only cache/ and
    // private/ are ignored, because a governance record that is gitignored is
    // not a record.
    write_text_atomically(
        &vds_dir.join(".gitignore"),
        "# VDS S-3(9): the record is committed, not scratch. These two are the only\n\
         # ignored paths, because a governance record that is gitignored is not a record.\n\
         cache/\nprivate/\n",
    )?;

    let project = vds_core::Project::discover(Some(&root))?;
    let lock = vds_designpack::pin(&project, &actor())?;
    vds_designpack::write_lock(&project, &lock)?;

    println!("initialised {}", project.rel(&vds_dir));
    println!("  config.toml, designpack.lock, .gitignore and the record directories");
    if lock.is_absent() {
        println!();
        println!("NOTE: no designpack/ is vendored, so designpack.lock pins the ABSENCE of one.");
        println!("      VDS S-15(1): the specification commences on a dated, digest-pinned");
        println!("      assent event in designpack/v1/provenance/assent/. Until then no warrant");
        println!("      may be granted, because there is nothing to grant one under.");
    }
    println!();
    println!("Next:");
    println!("  vds ledger screens     generate the declared surface");
    println!("  vds proof --all        run every implemented proof");
    println!("  vds doctor             measure this project against the done criteria");
    Ok(PASSED)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_identifier_with_a_quote_is_refused_before_anything_is_written() {
        assert!(check_identifier("jurisdiction", "acme \"web\"").is_err());
        assert!(check_identifier("jurisdiction", "acme\nweb").is_err());
        assert!(check_identifier("jurisdiction", "").is_err());
        assert!(check_identifier("jurisdiction", "acme-web_2.0").is_ok());
    }
}
