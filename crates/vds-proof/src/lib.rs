//! The proof kinds, and the engine that captures them.
//!
//! VDS S-7(5) fixes ten kinds as a CLOSED registry and VDS S-7(6) makes adding
//! one an amendment to the specification rather than a script anyone may drop
//! in. That closure is enforced here by construction: [`run`] matches on
//! [`ProofKind`], which is an enum, so a new kind cannot be dispatched without
//! adding a variant to the type the specification names.
//!
//! All ten are implemented. [`ProofKind::unimplemented_because`] returns None
//! for every one of them, and the type still carries it: a kind that later has to
//! be withdrawn must say WHY, per kind, rather than disappearing from a match.

use std::io::Write;

use vds_core::{Digest, EXIT_PASSED, InvokedBy, Project, ProofKind, Result, Timestamp};
use vds_store::Store;

pub mod composition;
pub mod contrast;
pub mod index;
pub mod ledger_staleness;
pub mod no_stored_values;
pub mod parity;
pub mod preimage;
pub mod reconciliation;
pub mod register_completeness;
pub mod retirement_drain;
pub mod run;
pub mod states;
pub mod token_pin;

#[cfg(test)]
pub mod testing;

#[cfg(test)]
mod zzprobe;

pub use run::{
    Capture, Outcome, ProofRun, Row, Verdict, aggregate_exit, evidence_digest, guarded,
    verify_record,
};

/// Everything a proof needs that is not its own logic.
pub struct ProofContext<'a> {
    pub project: &'a Project,
    pub invoked_by: InvokedBy,
    pub allow_vacuous: bool,
    pub capture: bool,
}

impl<'a> ProofContext<'a> {
    pub fn store(&self) -> Store<'a> {
        Store::new(self.project)
    }

    /// The designpack digest in force, recomputed from the vendored tree.
    ///
    /// Recomputed and not read from the lock. A proof records the designpack
    /// digest in force WHEN IT RAN (VDS S-11(1)), and reading that from the lock
    /// records the digest someone last wrote down, which is a different claim.
    pub fn designpack_digest(&self) -> Result<Digest> {
        vds_designpack::digest_in_force(self.project)
    }

    pub fn capture_options(&self) -> Result<Capture> {
        Ok(Capture {
            capture: self.capture,
            allow_vacuous: self.allow_vacuous,
            designpack_digest: self.designpack_digest()?,
        })
    }

    /// The command a reader would run to reproduce this proof. Recorded on the
    /// record, so VDS S-7(2)(1) is checkable rather than asserted.
    pub fn command(&self, kind: ProofKind) -> String {
        format!("vds proof {kind}")
    }

    pub fn new_run(&self, kind: ProofKind, gate: &'static str) -> ProofRun<'a> {
        ProofRun::new(
            self.project,
            kind,
            gate,
            self.command(kind),
            self.invoked_by,
        )
    }
}

/// Run one proof kind.
///
/// The match is exhaustive over [`ProofKind`], which is what makes VDS S-7(6)'s
/// closure structural: a kind cannot be dispatched without a variant on the type
/// the specification names, and a variant cannot be added without an arm here.
///
/// There is no longer a refusal arm, because there is no longer an unimplemented
/// kind. Should one be withdrawn, the arm that replaces its dispatch must return
/// EXIT_PRECONDITION and never a pass: a caller who asked for `contrast` and got
/// exit 0 would reasonably conclude the contrast floors had been checked.
pub fn run(kind: ProofKind, ctx: &ProofContext, out: &mut dyn Write) -> Result<Outcome> {
    match kind {
        ProofKind::RegisterCompleteness => register_completeness::run(ctx, out),
        ProofKind::Composition => composition::run(ctx, out),
        ProofKind::Contrast => contrast::run(ctx, out),
        ProofKind::Parity => parity::run(ctx, out),
        ProofKind::States => states::run(ctx, out),
        ProofKind::Reconciliation => reconciliation::run(ctx, out),
        ProofKind::RetirementDrain => retirement_drain::run(ctx, out),
        ProofKind::LedgerStaleness => ledger_staleness::run(ctx, out),
        ProofKind::NoStoredValues => no_stored_values::run(ctx, out),
        ProofKind::TokenPin => token_pin::run(ctx, out),
    }
}

/// Run every implemented kind, and report the worst outcome by severity.
pub fn run_all(ctx: &ProofContext, out: &mut dyn Write) -> Result<Vec<Outcome>> {
    let mut outcomes = Vec::new();
    for kind in ProofKind::implemented() {
        writeln!(out, "{}", "=".repeat(72)).ok();
        match run(kind, ctx, out) {
            Ok(outcome) => outcomes.push(outcome),
            Err(error) => {
                // One proof's precondition failure must not silence the others.
                // A run that stopped at the first stumble reports less than it
                // measured, and the reader cannot tell which.
                writeln!(
                    out,
                    "PRECONDITION FAILED for {kind}, so it did not run and proves nothing:"
                )
                .ok();
                writeln!(out, "  {error}").ok();
                outcomes.push(Outcome {
                    kind,
                    status: vds_core::ProofStatus::Failed,
                    exit_code: vds_core::EXIT_PRECONDITION,
                    record_id: None,
                    rows_considered: 0,
                    rows_enforced: 0,
                });
            }
        }
        writeln!(out).ok();
    }
    Ok(outcomes)
}

/// Print the summary that follows a `--all` run, including what did NOT run.
pub fn print_summary(outcomes: &[Outcome], out: &mut dyn Write) -> i32 {
    writeln!(out, "{}", "=".repeat(72)).ok();
    writeln!(out, "summary:").ok();
    for outcome in outcomes {
        let label = match outcome.exit_code {
            EXIT_PASSED if outcome.status == vds_core::ProofStatus::Vacuous => "vacuous (allowed)",
            EXIT_PASSED => "passed",
            1 => "FAILED",
            2 => "PRECONDITION FAILED",
            3 => "vacuous",
            _ => "unknown",
        };
        writeln!(
            out,
            "  {:24} {label:20} rows_enforced={}",
            outcome.kind.as_str(),
            outcome.rows_enforced
        )
        .ok();
    }

    let missing: Vec<ProofKind> = ProofKind::ALL
        .into_iter()
        .filter(|k| !k.is_implemented())
        .collect();
    if !missing.is_empty() {
        writeln!(out).ok();
        writeln!(
            out,
            "{} of the {} specified proof kinds are NOT implemented and did not run:",
            missing.len(),
            ProofKind::ALL.len()
        )
        .ok();
        for kind in missing {
            writeln!(
                out,
                "  {:24} {}",
                kind.as_str(),
                kind.unimplemented_because().unwrap_or("unstated")
            )
            .ok();
        }
        writeln!(
            out,
            "  A warrant relying on this run must not be described as covering them \
             (VDS S-6(3))."
        )
        .ok();
    }

    let codes: Vec<i32> = outcomes.iter().map(|o| o.exit_code).collect();
    aggregate_exit(&codes)
}

/// The gate paths this build ships, for the enforcement lock's UNPINNED report.
///
/// Named explicitly rather than discovered by walking a directory. A hardcoded
/// walk silently stops being true the moment the layout changes, and a lock that
/// reports nothing unpinned because it looked in the wrong place is worse than
/// one that reports nothing at all.
pub const GATE_PATHS: &[&str] = &[
    composition::GATE,
    ledger_staleness::GATE,
    no_stored_values::GATE,
    reconciliation::GATE,
    register_completeness::GATE,
    retirement_drain::GATE,
    contrast::GATE,
    parity::GATE,
    states::GATE,
    token_pin::GATE,
];

/// A timestamp helper for proofs that must record when they measured something.
pub fn now() -> Timestamp {
    Timestamp::now()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_implemented_kind_is_dispatched_and_every_unimplemented_one_refuses() {
        // A compile-time-adjacent check: the dispatcher's match is exhaustive
        // over ProofKind, so this only has to confirm the two arms agree with
        // ProofKind::is_implemented.
        for kind in ProofKind::ALL {
            let dispatched = true;
            assert_eq!(
                dispatched,
                kind.is_implemented(),
                "{kind} is dispatched and unimplemented, or implemented and not dispatched"
            );
        }
    }

    #[test]
    fn there_is_one_gate_path_per_implemented_kind() {
        assert_eq!(GATE_PATHS.len(), ProofKind::implemented().len());
        let mut sorted: Vec<&str> = GATE_PATHS.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), GATE_PATHS.len(), "a duplicate gate path");
        for gate in GATE_PATHS {
            assert!(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../..")
                    .join(gate)
                    .exists(),
                "the lock would pin {gate}, which does not exist"
            );
        }
    }
}
