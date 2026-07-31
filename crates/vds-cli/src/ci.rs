//! The CI run ledger: whether the workflow a gate names has ever actually concluded.
//!
//! This exists because of BREACH-0011, and the shape of that breach is the reason the
//! ledger is a ledger rather than a proof.
//!
//! D4's text is "every gate is invoked by CI, not only by a hook". It has been settled
//! twice and both settlings stopped short of the destination:
//!
//! 1. Originally by `LockEntry::has_blocking_ci`, which reads the lock's own declaration
//!    about itself. Renaming the step so ZERO steps matched left D4 reporting Met over
//!    sixteen gates. That is BREACH-0004.
//! 2. Then by `resolve_ci_reference`, which opens the workflow FILE and resolves the job
//!    and step. Better, and still not the thing: a step that exists in a file is not a
//!    step that ran.
//!
//! Measured 2026-07-31: fifty-three consecutive `vds-enforce` runs, **zero successes**,
//! every one ending in three to eleven seconds with the annotation `The job was not
//! started because recent account payments have failed or your spending limit needs to be
//! increased`. The job had never STARTED, in the entire life of the repository, while D4
//! reported Met on seventeen gates.
//!
//! So the missing limb is the run itself. A run is a network fact, and VDS S-7(2)(1)
//! forbids a network call inside a proof, so it cannot be asked at proof time. It is
//! recorded out of band into a ledger, exactly as `vds figma pull` records what a Figma
//! file draws, and `--from` makes the derivation reproducible from saved bytes with no
//! network and no credential.
//!
//! WHAT THIS LEDGER CANNOT SAY, stated here rather than discovered later:
//!
//! - whether the run that succeeded ran the gate. A conclusion is per-WORKFLOW; binding a
//!   conclusion to one step's `run:` body is the residue `resolve_ci_reference` already
//!   declines, and this ledger does not close it either.
//! - whether the successful run was over the current tree. `head_sha` is recorded so a
//!   reader can check, and nothing here compares it, because a gate that passed on an
//!   older commit is a different question from a gate that has never passed at all - and
//!   the second is the one that was true here.
//! - anything about a workflow it was not asked about. The row set is what the source
//!   response contained, and `runs_considered` is stated so a one-run window cannot be
//!   read as evidence about a year.

use vds_core::{Result, VdsError};
use vds_store::Store;

pub use vds_core::types::ci::{CiLedger, derive};

pub fn ledger_path(store: &Store) -> std::path::PathBuf {
    store
        .project
        .path(vds_core::PathRole::Ledgers)
        .join("ci.yaml")
}

pub fn read(store: &Store) -> Result<Option<CiLedger>> {
    let path = ledger_path(store);
    if !path.is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path).map_err(|e| VdsError::io(path.display(), e))?;
    // Parsing and the schema check live on the type (vds_core::types::ci), the one
    // implementation the staleness gate reads too.
    let ledger = CiLedger::parse(&text).map_err(|message| VdsError::Artefact {
        path: store.project.rel(&path),
        message,
    })?;
    Ok(Some(ledger))
}

pub fn write(store: &Store, ledger: &CiLedger) -> Result<std::path::PathBuf> {
    let path = ledger_path(store);
    let text = serde_yaml::to_string(ledger).map_err(|e| VdsError::Serialize {
        what: "the CI run ledger".into(),
        message: e.to_string(),
    })?;
    vds_core::write_text_atomically(&path, &text)?;
    Ok(path)
}
