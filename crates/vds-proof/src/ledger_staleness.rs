//! TODO: the `ledger_staleness` proof.
use std::io::Write;
use vds_core::{Result, VdsError};
use crate::{Outcome, ProofContext};
pub const GATE: &str = "crates/vds-proof/src/ledger_staleness.rs";
pub fn run(_ctx: &ProofContext, _out: &mut dyn Write) -> Result<Outcome> {
    Err(VdsError::precondition("not yet implemented"))
}
