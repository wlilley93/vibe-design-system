//! `vds lock`: the enforcement lock.
//!
//! VDS S-8(5), stated plainly and not glossed: the lock CANNOT bind an author
//! with full write access who edits a gate and re-locks it in the same act. The
//! backstops for that residue are non-machine: the Principal's gate and the duty
//! of reasonable care. The lock makes the act visible in a diff. It does not
//! prevent it, and no VDS document may claim otherwise, including this one.

use clap::{Args as ClapArgs, Subcommand};
use vds_core::{
    Digest, EXIT_VIOLATION, FailingDirectionTest, Invocation, InvokedBy, LockEntry, LockKind,
    ProofKind, Result, Timestamp, VdsError, actor,
};
use vds_store::{Store, lock as locklib};

use crate::{Context, PASSED};

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Recompute every pinned digest and report what moved.
    Verify,
    /// Pin one gate.
    Add(AddArgs),
    /// Re-pin every gate whose bytes moved, recording what each superseded.
    Repin {
        /// Why. Required: re-locking without recording why is itself the breach
        /// the lock exists to make visible (VDS S-8(4)).
        #[arg(long)]
        rationale: Option<String>,
    },
}

pub fn run(ctx: &Context, args: &Args) -> Result<i32> {
    let project = ctx.project()?;
    let store = Store::new(&project);
    match &args.action {
        Action::Verify => verify(&store),
        Action::Add(a) => add(&store, a),
        Action::Repin { rationale } => repin(&store, rationale.as_deref()),
    }
}

fn verify(store: &Store) -> Result<i32> {
    let gates: Vec<String> = vds_proof::GATE_PATHS.iter().map(|g| (*g).to_owned()).collect();
    let verdict = locklib::verify_lock(store, &gates)?;

    match store.read_lock()? {
        None => {}
        Some(lock) => {
            println!(
                "{} entries in {}",
                lock.entries.len(),
                store.project.rel(&store.lock_path())
            );
            for entry in &lock.entries {
                let surfaces: Vec<String> = entry
                    .invoked_by
                    .iter()
                    .map(|i| {
                        format!(
                            "{}({})",
                            i.surface,
                            if i.blocking { "blocking" } else { "reporting" }
                        )
                    })
                    .collect();
                println!("  {}", entry.path);
                println!("    digest:  {}", entry.digest);
                println!(
                    "    proves:  {}",
                    entry
                        .proves
                        .iter()
                        .map(|k| k.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                println!("    invoked: {}", surfaces.join(", "));
                println!(
                    "    failing-direction test: {}::{}",
                    entry.failing_direction_test.path, entry.failing_direction_test.test_name
                );
                if let Some(seeds) = &entry.failing_direction_test.seeds {
                    println!("      seeds: {seeds}");
                }
            }
        }
    }

    for note in &verdict.notes {
        println!();
        println!("{note}");
    }

    if !verdict.is_clean() {
        println!();
        println!(
            "ENFORCEMENT DRIFT, {} findings, each named in full:",
            verdict.findings.len()
        );
        for finding in &verdict.findings {
            println!("  {finding}");
        }
        println!();
        println!(
            "VDS S-8(5), stated plainly: the lock cannot bind an author with write access who \
             edits a gate and re-locks it in the same act. It makes the act visible in a diff. \
             It does not prevent it."
        );
        return Ok(EXIT_VIOLATION);
    }

    if store.read_lock()?.is_some() {
        println!();
        println!("no enforcement drift: every pinned path matches its digest.");
    }
    Ok(PASSED)
}

#[derive(ClapArgs)]
pub struct AddArgs {
    /// The gate, repository-relative.
    path: String,
    #[arg(long, default_value = "proof_script")]
    kind: String,
    /// `surface=reference[=blocking]`, repeatable. At least one: an uninvoked
    /// gate is not enforcement (VDS S-7(2)(3)).
    #[arg(long)]
    invoked_by: Vec<String>,
    /// A proof kind this gate produces, repeatable.
    #[arg(long)]
    proves: Vec<String>,
    /// The file holding the test that proves this gate's FAILING direction.
    #[arg(long)]
    test_path: Option<String>,
    /// The name of that test.
    #[arg(long)]
    test_name: Option<String>,
    /// What violation the test seeds, in one line, where a reviewer will see it.
    #[arg(long)]
    seeds: Option<String>,
    /// Required when re-pinning a path whose bytes have changed.
    #[arg(long)]
    rationale: Option<String>,
}

fn add(store: &Store, args: &AddArgs) -> Result<i32> {
    let target = store.project.root.join(&args.path);
    if !target.is_file() {
        return Err(VdsError::precondition(format!(
            "{} does not exist, so there is nothing to pin",
            args.path
        )));
    }
    if args.invoked_by.is_empty() {
        return Err(VdsError::precondition(
            "pass --invoked-by at least once, as 'surface=reference' or \
             'surface=reference=blocking'. An empty invocation list is not representable, \
             because an uninvoked gate is not enforcement (VDS S-7(2)(3)).",
        ));
    }
    let (Some(test_path), Some(test_name)) = (&args.test_path, &args.test_name) else {
        return Err(VdsError::precondition(
            "pass --test-path and --test-name. An entry cannot be written without naming the \
             test that proves the gate's FAILING direction, which is how VDS S-7(2)(2) is made \
             structural rather than aspirational: a check whose failing direction is asserted \
             nowhere has proven only its happy path.",
        ));
    };
    if !store.project.root.join(test_path).is_file() {
        return Err(VdsError::precondition(format!(
            "--test-path {test_path} does not exist"
        )));
    }
    if args.proves.is_empty() {
        return Err(VdsError::precondition(
            "pass --proves at least once, naming a kind from the closed registry. A gate that \
             proves nothing is not a gate, and a lock entry claiming otherwise is a pin on a \
             file rather than on a check.",
        ));
    }

    let kind = LockKind::parse(&args.kind).ok_or_else(|| {
        VdsError::precondition(format!(
            "--kind {:?} is not a lock kind. The five are: {}",
            args.kind,
            LockKind::ALL
                .iter()
                .map(|k| k.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))
    })?;

    let mut proves = Vec::new();
    for raw in &args.proves {
        proves.push(ProofKind::parse(raw).ok_or_else(|| {
            VdsError::precondition(format!(
                "--proves {raw:?} is not in the closed registry (VDS S-7(5))"
            ))
        })?);
    }

    let mut invocations = Vec::new();
    for spec in &args.invoked_by {
        invocations.push(parse_invocation(spec)?);
    }

    let existing = store.read_lock()?;
    let mut entries: Vec<LockEntry> = existing.map(|l| l.entries).unwrap_or_default();
    let previous = entries.iter().find(|e| e.path == args.path).cloned();
    entries.retain(|e| e.path != args.path);

    let digest = Digest::of_file(&target)?;
    let mut entry = LockEntry {
        path: args.path.clone(),
        digest: digest.clone(),
        kind,
        invoked_by: invocations,
        proves,
        failing_direction_test: FailingDirectionTest {
            path: test_path.clone(),
            test_name: test_name.clone(),
            seeds: args.seeds.clone(),
        },
        pinned_at: Timestamp::now(),
        pinned_by: actor(),
        supersedes_digest: None,
        relock_rationale: None,
    };

    if let Some(previous) = previous
        && previous.digest != digest
    {
        let Some(rationale) = &args.rationale else {
            return Err(VdsError::precondition(format!(
                "{} is already pinned at {} and the bytes have changed. Re-pinning is \
                 deliberate: pass --rationale, and self-file under VDS S-12(3). Re-locking \
                 without recording why is itself the breach the lock exists to make visible.",
                args.path, previous.digest
            )));
        };
        entry.supersedes_digest = Some(previous.digest);
        entry.relock_rationale = Some(rationale.clone());
    }

    entries.push(entry);
    let count = entries.len();
    let path = locklib::write_lock(store.project, entries)?;
    println!("pinned {} at {digest}", args.path);
    println!("  wrote {} ({count} entries)", store.project.rel(&path));
    Ok(PASSED)
}

/// `surface=reference` or `surface=reference=blocking`.
///
/// The reference may itself contain `=`, so the split is on the FIRST separator
/// for the surface and the LAST for the blocking flag, and a reference that
/// happens to end in something flag-like is treated as a reference unless the
/// tail is one of the recognised words.
fn parse_invocation(spec: &str) -> Result<Invocation> {
    let (surface_raw, rest) = spec.split_once('=').ok_or_else(|| {
        VdsError::precondition(format!(
            "--invoked-by {spec:?} must be 'surface=reference' or \
             'surface=reference=blocking'"
        ))
    })?;
    let surface = InvokedBy::parse(surface_raw).ok_or_else(|| {
        VdsError::precondition(format!(
            "--invoked-by {spec:?}: {surface_raw:?} is not an invocation surface. The six \
             are: {}",
            InvokedBy::ALL
                .iter()
                .map(|i| i.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))
    })?;

    let (reference, blocking) = match rest.rsplit_once('=') {
        Some((head, tail)) if matches!(tail, "blocking" | "true" | "1" | "yes") => (head, true),
        Some((head, tail)) if matches!(tail, "reporting" | "false" | "0" | "no") => (head, false),
        _ => (rest, true),
    };
    if reference.trim().is_empty() {
        return Err(VdsError::precondition(format!(
            "--invoked-by {spec:?}: the reference is empty. It must say WHERE the gate is \
             wired: a workflow file and job, a hook path and line, a package script name."
        )));
    }
    Ok(Invocation {
        surface,
        reference: reference.to_owned(),
        blocking,
    })
}

fn repin(store: &Store, rationale: Option<&str>) -> Result<i32> {
    let rationale = rationale.unwrap_or("");
    let changed = locklib::repin_lock(store, rationale)?;
    if changed.is_empty() {
        println!("nothing to re-pin: every pinned path already matches its digest");
        return Ok(PASSED);
    }
    println!(
        "re-pinned {} entries, each recording what it superseded:",
        changed.len()
    );
    for entry in &changed {
        println!("  {}", entry.path);
        println!("    was: {}", entry.was);
        println!("    now: {}", entry.now);
    }
    println!("  rationale: {rationale}");
    println!();
    println!(
        "Self-file this under VDS S-12(3). A re-pin with a rationale and no breach report is \
         a rationale nobody has to answer for."
    );
    Ok(PASSED)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_invocation_defaults_to_blocking() {
        let invocation = parse_invocation("ci_workflow=.github/workflows/vds.yml").unwrap();
        assert_eq!(invocation.surface, InvokedBy::CiWorkflow);
        assert_eq!(invocation.reference, ".github/workflows/vds.yml");
        assert!(invocation.blocking);
    }

    #[test]
    fn an_invocation_can_be_marked_reporting_only() {
        let invocation = parse_invocation("ci_workflow=w.yml=reporting").unwrap();
        assert!(
            !invocation.blocking,
            "a CI job that runs the gate and ignores its exit code is not enforcement"
        );
    }

    #[test]
    fn a_reference_containing_an_equals_sign_survives() {
        let invocation = parse_invocation("package_script=npm run check --flag=x").unwrap();
        assert_eq!(invocation.reference, "npm run check --flag=x");
        assert!(invocation.blocking);
    }

    #[test]
    fn an_unknown_surface_is_refused_by_name() {
        let error = parse_invocation("vibes=somewhere").unwrap_err();
        assert!(error.to_string().contains("not an invocation surface"), "{error}");
    }

    #[test]
    fn an_empty_reference_is_refused() {
        assert!(parse_invocation("ci_workflow=").is_err());
    }
}
