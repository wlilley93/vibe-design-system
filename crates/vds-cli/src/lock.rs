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

/// A `ci_workflow` reference, split into the file, job and step it names.
///
/// The lock spells these `<file> job:<job> step:<Step Name>`, which is a string
/// nothing has ever parsed. That is the defect: a reference can name a step that
/// was renamed, moved to another job, or deleted, and the lock keeps reporting
/// the gate as CI-invoked because it only ever compared the string to itself.
/// Same class as BREACH-0004 and the same class as BREACH-0011, where D4 read a
/// workflow FILE and never the RUN.
#[derive(Debug, PartialEq)]
struct CiReference<'a> {
    file: &'a str,
    job: Option<&'a str>,
    step: Option<&'a str>,
}

fn parse_ci_reference(reference: &str) -> CiReference<'_> {
    let mut file = reference;
    let mut job = None;
    let mut step = None;
    if let Some(at) = reference.find(" job:") {
        file = reference[..at].trim();
        let rest = &reference[at + " job:".len()..];
        match rest.find(" step:") {
            Some(s) => {
                job = Some(rest[..s].trim());
                step = Some(rest[s + " step:".len()..].trim());
            }
            None => job = Some(rest.trim()),
        }
    }
    CiReference { file, job, step }
}

/// Whether a step's `run:` block would actually reach `path`.
///
/// COARSE ON PURPOSE, and the coarseness is stated rather than hidden. It answers
/// one question - could this command have executed the file at all - and it
/// answers it from the command text, because resolving what `cargo test
/// --workspace` really runs needs cargo.
///
/// A `None` means "this checker has no opinion", which is NOT the same as "no".
/// A checker that guessed `no` would turn every unfamiliar command into a
/// finding, and a wall of false findings is how a gate stops being read.
fn command_reaches(run: &str, path: &str) -> Option<bool> {
    let run = run.to_lowercase();
    // The literal path, or the file's own name, appearing in the command.
    if run.contains(&path.to_lowercase()) {
        return Some(true);
    }
    // A Rust gate is reached by any workspace-wide cargo invocation, because
    // the failing-direction test lives in that file and `--workspace` runs it.
    if path.ends_with(".rs")
        && run.contains("cargo")
        && (run.contains("--workspace") || run.contains("--all"))
    {
        return Some(true);
    }
    // A JS gate under site-factory is reached by its own gate runner.
    if path.ends_with(".js") && run.contains("site-factory/tests/gate.js") {
        return Some(true);
    }
    // A Rust gate inside this workspace is reached by any step that RUNS THE
    // BUILT BINARY, because the binary is compiled from those crates. Without
    // this rule the checker had no opinion on eighteen of nineteen references -
    // every `vds proof`, `vds doctor` and `vds lock verify` step - which is a
    // check that almost never fires, dressed as a check that passed.
    if path.starts_with("crates/") && path.ends_with(".rs") && run.contains("/vds ") {
        return Some(true);
    }
    // A shell gate is reached by a step that invokes it by name; that is the
    // literal-path case above. Anything else, no opinion, and `None` is not
    // `false`: guessing `no` would turn every unfamiliar command into a finding
    // and a wall of false findings is how a gate stops being read.
    None
}

/// Every `ci_workflow` reference in the lock, checked against the workflow file.
///
/// Returns (findings, checked, unopinionated). Pure, so it is testable without a
/// repository: the defect this closes is precisely a check that was never run
/// against a real workflow.
fn ci_references_resolve(
    lock: &vds_core::EnforcementLock,
    workflows: &std::collections::BTreeMap<String, String>,
) -> (Vec<String>, usize, usize) {
    let mut findings = Vec::new();
    let mut checked = 0usize;
    let mut no_opinion = 0usize;

    for entry in &lock.entries {
        for invocation in &entry.invoked_by {
            if invocation.surface.to_string() != "ci_workflow" {
                continue;
            }
            let parsed = parse_ci_reference(&invocation.reference);
            checked += 1;
            let Some(text) = workflows.get(parsed.file) else {
                findings.push(format!(
                    "{}: names {} and that file is not in the repository. A lock entry \
                     invoked by a workflow that does not exist is not invoked at all.",
                    entry.path, parsed.file
                ));
                continue;
            };
            let Ok(doc) = serde_yaml::from_str::<serde_yaml::Value>(text) else {
                findings.push(format!(
                    "{}: {} does not parse as YAML, so nothing can be said about the step \
                     it names.",
                    entry.path, parsed.file
                ));
                continue;
            };
            let Some(job_name) = parsed.job else {
                // A file-only reference is legal and weaker. Not a finding, but
                // it cannot be checked past existence, and saying so is the point.
                no_opinion += 1;
                continue;
            };
            let job = doc.get("jobs").and_then(|j| j.get(job_name));
            let Some(job) = job else {
                findings.push(format!(
                    "{}: names job {:?} in {}, and that job does not exist. The reference has \
                     drifted from the workflow.",
                    entry.path, job_name, parsed.file
                ));
                continue;
            };
            let Some(step_name) = parsed.step else {
                no_opinion += 1;
                continue;
            };
            let steps = job.get("steps").and_then(|s| s.as_sequence());
            let found = steps.and_then(|steps| {
                steps.iter().find(|s| {
                    s.get("name").and_then(|n| n.as_str()).map(str::trim) == Some(step_name)
                })
            });
            let Some(step) = found else {
                let available: Vec<String> = steps
                    .map(|steps| {
                        steps
                            .iter()
                            .filter_map(|s| s.get("name").and_then(|n| n.as_str()))
                            .map(str::to_owned)
                            .collect()
                    })
                    .unwrap_or_default();
                findings.push(format!(
                    "{}: names step {:?} in job {:?}, and no such step exists. The job has: \
                     {}. A renamed step silently unbinds the gate from CI while the lock \
                     keeps reporting it as invoked.",
                    entry.path,
                    step_name,
                    job_name,
                    if available.is_empty() {
                        "no named steps".to_owned()
                    } else {
                        available.join(", ")
                    }
                ));
                continue;
            };
            // The step exists. Now the harder half: does what it RUNS reach the
            // pinned file at all?
            let run = step.get("run").and_then(|r| r.as_str()).unwrap_or("");
            match command_reaches(run, &entry.path) {
                Some(true) => {}
                Some(false) => findings.push(format!(
                    "{}: step {:?} exists and what it runs cannot reach that path.",
                    entry.path, step_name
                )),
                None => no_opinion += 1,
            }
        }
    }
    (findings, checked, no_opinion)
}

fn verify(store: &Store) -> Result<i32> {
    let gates: Vec<String> = vds_proof::GATE_PATHS
        .iter()
        .map(|g| (*g).to_owned())
        .collect();
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

    // THE SECOND HALF OF THE LOCK'S CLAIM, and until now nobody checked it.
    // The digest half asks "is this gate the one that was pinned". This half
    // asks "is it still wired to what the lock says invokes it" - a step can be
    // renamed, moved or deleted and every digest still match perfectly.
    let mut ci_findings = Vec::new();
    if let Some(lock) = store.read_lock()? {
        let mut workflows = std::collections::BTreeMap::new();
        let dir = store.project.root.join(".github/workflows");
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.extension().is_some_and(|e| e == "yml" || e == "yaml") {
                    continue;
                }
                if let Ok(text) = std::fs::read_to_string(&path) {
                    workflows.insert(store.project.rel(&path), text);
                }
            }
        }
        let (findings, checked, no_opinion) = ci_references_resolve(&lock, &workflows);
        println!();
        println!(
            "{checked} ci_workflow references checked against {} workflow file(s); \
             {no_opinion} could not be judged past existence.",
            workflows.len()
        );
        ci_findings = findings;
        for finding in &ci_findings {
            println!("  {finding}");
        }
    }

    if !ci_findings.is_empty() {
        println!();
        println!(
            "A lock entry naming a CI step that does not exist is the same defect as \
             BREACH-0011 one level up: the lock reported seventeen gates as CI-invoked \
             while the job had never once started. A reference nobody parses is a \
             declaration, not a binding."
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
    let kind = LockKind::parse(&args.kind).ok_or_else(|| {
        VdsError::precondition(format!(
            "--kind {:?} is not a lock kind. The {} are: {}",
            args.kind,
            LockKind::ALL.len(),
            LockKind::ALL
                .iter()
                .map(|k| k.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))
    })?;

    proves_matches_kind(kind, args.proves.len()).map_err(VdsError::precondition)?;

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
        Some((head, "blocking" | "true" | "1" | "yes")) => (head, true),
        Some((head, "reporting" | "false" | "0" | "no")) => (head, false),
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

/// Whether `--proves` agrees with the kind, in BOTH directions.
///
/// This used to be an unconditional "proves must be non-empty", and that refusal caused a
/// breach rather than preventing one. A `criteria_grader` has no ProofKind by construction -
/// `LockKind::CriteriaGrader`'s own doc comment says "`proves` is empty for this kind, and
/// that emptiness is correct rather than missing" - and a `hook` INVOKES gates rather than
/// being one. Neither could be expressed through the CLI, so the grader entry was appended
/// to `.vds/enforcement.lock` with a shell heredoc instead. The heredoc was unquoted, bash
/// executed the backticked commands inside the rationale, and `cargo test` wrote about a
/// hundred lines of its own stdout into the enforcement lock. That is BREACH-0009.
///
/// The lesson is narrower than "be careful with heredocs": A TOOL THAT CANNOT EXPRESS A
/// LAWFUL STATE PUSHES ITS USER OUTSIDE THE TOOL, and here the outside was a shell writing
/// to the single most load-bearing artefact in the repository.
///
/// Both directions are checked, because the loose one is also a defect: a `hook` entry
/// carrying `proves: [contrast]` would claim the hook establishes a contrast result, which
/// it does not, and VDS S-8(5) forbids overclaiming an enforcement surface.
fn proves_matches_kind(kind: LockKind, proves: usize) -> std::result::Result<(), String> {
    let proves_nothing = matches!(kind, LockKind::CriteriaGrader | LockKind::Hook);
    match (proves == 0, proves_nothing) {
        (true, false) => Err(format!(
            "pass --proves at least once, naming a kind from the closed registry. A {} gate \
             that proves nothing is not a gate, and a lock entry claiming otherwise is a pin \
             on a file rather than on a check. (`criteria_grader` and `hook` are the only \
             kinds that may omit it: one GRADES proofs, the other INVOKES them.)",
            kind.as_str()
        )),
        (false, true) => Err(format!(
            "--kind {} must NOT pass --proves. It does not establish a proof kind: it {}. \
             Listing one would put a claim on the entry that the file cannot support, which \
             is the overclaim VDS S-8(5) exists to refuse.",
            kind.as_str(),
            match kind {
                LockKind::CriteriaGrader => "reads the proof records and grades them",
                _ => "invokes other gates",
            }
        )),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The failing-direction test for `proves_matches_kind`, seeded in both directions.
    ///
    /// The positive arms come first and are not decoration: a predicate that refused
    /// everything would satisfy every negative arm below, which is the check-that-cannot-pass
    /// shape this repository has shipped twice.
    #[test]
    fn proves_must_agree_with_the_kind_in_both_directions() {
        // A proving gate with a kind it proves: fine.
        assert!(proves_matches_kind(LockKind::ProofScript, 1).is_ok());
        // The two kinds that prove nothing, with nothing claimed: fine. This is the state
        // the CLI could not express, which is why BREACH-0009 happened in a shell.
        assert!(proves_matches_kind(LockKind::CriteriaGrader, 0).is_ok());
        assert!(proves_matches_kind(LockKind::Hook, 0).is_ok());

        // SEED 1: a proving gate that claims nothing is a pin on a file, not on a check.
        let e = proves_matches_kind(LockKind::ProofScript, 0).unwrap_err();
        assert!(
            e.contains("proof_script") && e.contains("criteria_grader") && e.contains("hook"),
            "the refusal must name the kind AND the two exemptions, or the author's next \
             move is to edit the lock by hand - which is exactly what went wrong: {e}"
        );

        // SEED 2: the loose direction, which the old unconditional check could not catch at
        // all. A hook claiming to prove a kind is an overclaim under VDS S-8(5).
        let e = proves_matches_kind(LockKind::Hook, 1).unwrap_err();
        assert!(
            e.contains("must NOT pass --proves") && e.contains("invokes other gates"),
            "a hook claiming a proof kind must be refused, and told why: {e}"
        );
        let e = proves_matches_kind(LockKind::CriteriaGrader, 2).unwrap_err();
        assert!(e.contains("grades them"), "{e}");
    }

    /// The committed hook must run the WHOLE check set, not a subset.
    ///
    /// Tested against the file that actually ships rather than a fixture, because a guard
    /// verified against a copy is a guard that passes while the real artefact drifts. A hook
    /// narrowed to `make test` would still exit non-zero on a failing test and would silently
    /// stop running every gate - which is the failure this asserts against.
    #[test]
    fn the_committed_pre_push_hook_runs_the_full_check_set() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../scripts/githooks/pre-push");
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "the pre-push hook must exist at {}: {e}. It is the only surface that runs \
                 the gates on this repository - see BREACH-0011",
                path.display()
            )
        });
        assert!(
            text.lines().any(|l| l.trim() == "make check"),
            "the hook must invoke `make check` as its own line. A hook that runs some of \
             the gates teaches the reader that the others are optional."
        );
        assert!(
            text.contains("not CI") || text.contains("NOT CI"),
            "the hook must say on its face that it is not CI. VDS S-7(3) holds that a hook \
             is not CI, and a green hook read as D4 met is the whole defect of BREACH-0011."
        );
    }

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
        assert!(
            error.to_string().contains("not an invocation surface"),
            "{error}"
        );
    }

    #[test]
    fn an_empty_reference_is_refused() {
        assert!(parse_invocation("ci_workflow=").is_err());
    }
}

#[cfg(test)]
mod ci_reference_tests {
    use super::*;

    #[test]
    fn a_reference_splits_into_file_job_and_step() {
        let r = parse_ci_reference(".github/workflows/vds-enforce.yml job:enforce step:Test");
        assert_eq!(r.file, ".github/workflows/vds-enforce.yml");
        assert_eq!(r.job, Some("enforce"));
        assert_eq!(r.step, Some("Test"));

        // A step name containing a colon or a comma must survive, because the
        // real workflow has one: "The worked example, with no exemption".
        let r = parse_ci_reference("w.yml job:enforce step:The worked example, with no exemption");
        assert_eq!(r.step, Some("The worked example, with no exemption"));

        // A file-only reference stays legal and is simply weaker.
        let r = parse_ci_reference(".github/workflows/vds-enforce.yml");
        assert_eq!(r.job, None);
        assert_eq!(r.step, None);
    }

    #[test]
    fn no_opinion_is_not_the_same_as_unreachable() {
        // The three rules that DO have an opinion.
        assert_eq!(
            command_reaches("cargo test --workspace", "crates/x/src/a.rs"),
            Some(true)
        );
        assert_eq!(
            command_reaches("node site-factory/tests/gate.js", "site-factory/x.js"),
            Some(true)
        );
        assert_eq!(
            command_reaches(
                "./target/release/vds proof --all",
                "crates/vds-proof/src/x.rs"
            ),
            Some(true)
        );
        assert_eq!(
            command_reaches(
                "bash scripts/githooks/pre-push",
                "scripts/githooks/pre-push"
            ),
            Some(true)
        );

        // And the honest gap. `None` must never be reported as a finding: a
        // checker that guessed `false` on every command it did not recognise
        // would produce a wall of false findings, and a gate nobody reads is
        // worse than no gate.
        assert_eq!(command_reaches("echo hello", "crates/x/src/a.rs"), None);
        assert_eq!(command_reaches("npm run lint", "site-factory/x.js"), None);
    }

    /// The failing direction, and the one that is the whole point of the check:
    /// a step renamed in the workflow while every digest still matches.
    #[test]
    fn a_renamed_step_unbinds_the_gate_and_is_reported() {
        let workflow = "name: e\njobs:\n  enforce:\n    steps:\n      - name: Tests\n        run: cargo test --workspace\n";
        let mut files = std::collections::BTreeMap::new();
        files.insert(".github/workflows/w.yml".to_owned(), workflow.to_owned());

        let lock: vds_core::EnforcementLock = serde_yaml::from_str(
            "schema_version: 1\ngenerated_at: 2026-07-31T00:00:00Z\nentries:\n\
             - path: crates/a/src/b.rs\n  digest: sha256:0\n  kind: proof_script\n\
               \x20 invoked_by:\n  - surface: ci_workflow\n    reference: '.github/workflows/w.yml job:enforce step:Test'\n\
               \x20   blocking: true\n  proves: []\n  failing_direction_test:\n    path: crates/a/src/b.rs\n\
               \x20   test_name: t\n  pinned_at: 2026-07-31T00:00:00Z\n  pinned_by: x\n",
        )
        .expect("fixture lock parses");

        let (findings, checked, no_opinion) = ci_references_resolve(&lock, &files);
        assert_eq!(checked, 1);
        assert_eq!(no_opinion, 0);
        assert_eq!(findings.len(), 1, "a renamed step must be a finding");
        assert!(
            findings[0].contains("no such step exists"),
            "{}",
            findings[0]
        );
        // The finding must LIST what the job does have, or it names a problem
        // and no way to fix it.
        assert!(findings[0].contains("Tests"), "{}", findings[0]);

        // The negative control: with the step named correctly, clean. Without
        // this the test above would pass on a checker that always finds fault.
        let mut fixed = lock;
        fixed.entries[0].invoked_by[0].reference =
            ".github/workflows/w.yml job:enforce step:Tests".to_owned();
        let (findings, _, no_opinion) = ci_references_resolve(&fixed, &files);
        assert_eq!(findings, Vec::<String>::new());
        assert_eq!(no_opinion, 0, "cargo test --workspace reaches a .rs gate");
    }

    #[test]
    fn a_missing_workflow_file_and_a_missing_job_are_both_reported() {
        let mut files = std::collections::BTreeMap::new();
        files.insert(
            ".github/workflows/w.yml".to_owned(),
            "name: e\njobs:\n  other:\n    steps: []\n".to_owned(),
        );
        let make = |reference: &str| -> vds_core::EnforcementLock {
            serde_yaml::from_str(&format!(
                "schema_version: 1\ngenerated_at: 2026-07-31T00:00:00Z\nentries:\n\
                 - path: crates/a/src/b.rs\n  digest: sha256:0\n  kind: proof_script\n\
                   \x20 invoked_by:\n  - surface: ci_workflow\n    reference: '{reference}'\n\
                   \x20   blocking: true\n  proves: []\n  failing_direction_test:\n\
                   \x20   path: crates/a/src/b.rs\n    test_name: t\n\
                   \x20 pinned_at: 2026-07-31T00:00:00Z\n  pinned_by: x\n"
            ))
            .expect("fixture parses")
        };

        let (gone, _, _) =
            ci_references_resolve(&make(".github/workflows/absent.yml job:e step:S"), &files);
        assert_eq!(gone.len(), 1);
        assert!(gone[0].contains("not in the repository"), "{}", gone[0]);

        let (job, _, _) =
            ci_references_resolve(&make(".github/workflows/w.yml job:enforce step:S"), &files);
        assert_eq!(job.len(), 1);
        assert!(job[0].contains("does not exist"), "{}", job[0]);
    }
}
