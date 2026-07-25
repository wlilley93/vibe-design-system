//! `vds proof`: run one proof kind, or every implemented kind.
//!
//! The retired tool's `--all` re-parsed the CLI's own argv inside each proof
//! module, so `vds proof --all` (the command its own `init` printed as the next
//! step) died in argparse with exit 2 and ran nothing. Dispatch here is a
//! function call, so there is no argv to re-parse.

use clap::Args as ClapArgs;
use vds_core::{InvokedBy, ProofKind, Result, VdsError};

use crate::{Context, PASSED};

#[derive(ClapArgs)]
pub struct Args {
    /// The kind to run. Omit with `--all` or `--list`.
    kind: Option<String>,
    /// Run every implemented kind.
    #[arg(long, conflicts_with_all = ["kind", "list"])]
    all: bool,
    /// List the closed registry and say which kinds are implemented.
    #[arg(long, conflicts_with_all = ["kind", "all"])]
    list: bool,
    /// What invoked this run. Recorded honestly on the record; `manual` does
    /// not satisfy VDS S-7(2)(3).
    #[arg(long, default_value = "manual")]
    invoked_by: String,
    /// Exit 0 instead of 3 when no row is in an enforceable state. The vacuity
    /// is still recorded as `vacuous` and still says so in the output.
    #[arg(long)]
    allow_vacuous: bool,
    /// Write no proof record. For local inspection only: a run made this way
    /// can never be cited as evidence.
    #[arg(long)]
    no_capture: bool,
}

pub fn run(ctx: &Context, args: &Args) -> Result<i32> {
    if args.list {
        return list();
    }

    let project = ctx.project()?;
    let invoked_by = InvokedBy::parse(&args.invoked_by).ok_or_else(|| {
        VdsError::precondition(format!(
            "--invoked-by {:?} is not an invocation surface. The six are: {}",
            args.invoked_by,
            InvokedBy::ALL
                .iter()
                .map(|i| i.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))
    })?;

    let proof_ctx = vds_proof::ProofContext {
        project: &project,
        invoked_by,
        allow_vacuous: args.allow_vacuous,
        capture: !args.no_capture,
    };

    let mut out = std::io::stdout();
    if args.all {
        let outcomes = vds_proof::run_all(&proof_ctx, &mut out)?;
        let code = vds_proof::print_summary(&outcomes, &mut out);
        if invoked_by == InvokedBy::Manual {
            println!();
            println!(
                "NOTE: invoked_by is `manual`, which is the author choosing to run it. \
                 VDS S-7(2)(3) requires something OTHER than the author to invoke a gate, so \
                 these records do not satisfy the invocation limb."
            );
        }
        return Ok(code);
    }

    let Some(raw) = &args.kind else {
        return Err(VdsError::precondition(
            "name a proof kind, or pass --all, or pass --list.",
        ));
    };
    let kind = ProofKind::parse(raw).ok_or_else(|| {
        VdsError::precondition(format!(
            "{raw:?} is not a proof kind. The registry is CLOSED at ten (VDS S-7(5)):\n  {}\n  \
             Adding a kind amends the specification and the invariant registry; it is not a \
             script anyone may drop in (VDS S-7(6)).",
            ProofKind::ALL
                .iter()
                .map(|k| k.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))
    })?;

    let outcome = vds_proof::run(kind, &proof_ctx, &mut out)?;
    Ok(outcome.exit_code)
}

fn list() -> Result<i32> {
    println!("proof kinds (VDS S-7(5), a CLOSED registry of {}):", ProofKind::ALL.len());
    println!();
    for kind in ProofKind::ALL {
        match kind.unimplemented_because() {
            None => println!("  {:22} implemented      {}", kind.as_str(), kind.establishes()),
            Some(_) => println!("  {:22} NOT IMPLEMENTED  {}", kind.as_str(), kind.establishes()),
        }
    }
    let missing: Vec<ProofKind> = ProofKind::ALL
        .into_iter()
        .filter(|k| !k.is_implemented())
        .collect();
    if !missing.is_empty() {
        println!();
        println!("why the {} unimplemented kinds are unimplemented:", missing.len());
        for kind in missing {
            println!("  {}", kind.as_str());
            for line in wrap(kind.unimplemented_because().unwrap_or("unstated"), 70) {
                println!("    {line}");
            }
        }
    }
    Ok(PASSED)
}

/// Wrap text at a column, for the one place a paragraph is printed under a
/// heading. Not a general utility: it exists so the "why unimplemented" lines
/// read as prose rather than as one long line.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if !current.is_empty() && current.len() + 1 + word.len() > width {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapping_keeps_every_word_and_respects_the_width() {
        let text = "a bb ccc dddd eeeee ffffff";
        let lines = wrap(text, 10);
        assert_eq!(lines.join(" "), text);
        assert!(lines.iter().all(|l| l.len() <= 10), "{lines:?}");
    }

    #[test]
    fn wrapping_does_not_break_a_word_longer_than_the_width() {
        assert_eq!(wrap("supercalifragilistic", 5), vec!["supercalifragilistic"]);
    }
}
