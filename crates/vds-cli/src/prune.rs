//! `vds prune`: bound the proof working set without losing a fact.
//!
//! `.vds/proofs/` grows by one record per kind per run. Nine kinds and a `make
//! check` a dozen times a day is a hundred records a week, and a record
//! directory nobody can read is a record directory nobody reads. The pile is
//! also actively harmful: it buries the ONE record that matters, which is the
//! most recent one for each kind, under ninety identical passes.
//!
//! # Why deleting evidence is lawful here, and where the line is
//!
//! Applying the four-limb test at VDS S-2(5) to a proof record settles it.
//!
//! **Deletion.** What is lost when a passing, uncited, superseded record goes?
//! The ability to re-read a run that agreed with a later run over the same
//! subject. Nothing downstream resolves against it.
//!
//! **Regeneration.** A proof record is produced by a named command over a named
//! subject (VDS S-7(2)(1)). It is regenerable by definition, which is the whole
//! point of the re-runnable limb.
//!
//! And the fact itself is not lost, because `.vds/` is COMMITTED. Git is the
//! append-only store; this directory is a working set over it. A record pruned
//! at HEAD is still at the commit that captured it, still reachable by
//! `git log -- .vds/proofs/PROOF-...`, and its id, kind, status and digest are
//! written into a retention log that is itself committed. Nothing becomes
//! unknowable; a working set becomes readable.
//!
//! # The four rules, in order, each of which can only keep
//!
//! 1. **Cited by a warrant, always kept.** A warrant's evidence entry names a
//!    `proof_id` (VDS S-6(3)), and a warrant naming a record that is not there
//!    is a signature on nothing. This rule is checked against every warrant on
//!    disk whatever its status, because a spent or superseded warrant is still a
//!    historical claim whose basis has to remain readable.
//! 2. **`failed`, always kept.** A failure is a thing that HAPPENED, and pruning
//!    failures would turn this command into a way of tidying away the history of
//!    a gate that used to be red. A pass merely agrees with the pass that
//!    superseded it.
//!
//!    A `vacuous` record is NOT covered by this rule, and the distinction earns
//!    its place. Against VDS itself five kinds are vacuous on every run because
//!    the project has no screens, so treating vacuity as interesting kept 54 of
//!    90 records and the command pruned almost nothing. A repeated vacuity is
//!    not an event; it is a standing condition, and the standing condition is
//!    carried by the most recent record, which rule 3 keeps. A vacuity that
//!    appeared and then went is inside the window while it is recent and in git
//!    forever after.
//! 3. **The most recent `--keep` of each kind, always kept.** Per KIND and not
//!    overall, so a kind that runs rarely is not evicted by a kind that runs on
//!    every commit.
//! 4. **Everything else is removed**, and named individually in the retention
//!    log with its digest.
//!
//! Nothing here decides anything a proof or a warrant decided. It moves files,
//! and it writes down which ones and why.

use std::collections::{BTreeMap, BTreeSet};

use clap::Args as ClapArgs;
use serde::Serialize;
use vds_core::{ProofId, ProofKind, ProofStatus, Result, Timestamp, VdsError, actor};
use vds_store::Store;

use crate::{Context, PASSED};

/// How many passing records of each kind survive by default.
///
/// Ten rather than one. One would mean the previous run is gone the moment a new
/// one lands, and the ordinary question a reader asks is "when did this last
/// change", which needs more than the current answer. Ten is roughly a day of
/// work at this repository's rate and fits on a screen.
const DEFAULT_KEEP: usize = 10;

#[derive(ClapArgs)]
pub struct Args {
    /// How many passing records of each kind to keep. The most recent win.
    #[arg(long, default_value_t = DEFAULT_KEEP)]
    keep: usize,
    /// Actually remove the files.
    ///
    /// The default is to report and remove nothing, which INVERTS the
    /// convention `vds register import` uses. That command's default is to
    /// write, and `--dry-run` opts out; here the default is to do nothing, and
    /// `--apply` opts in. The asymmetry is deliberate: a command that creates
    /// something is recoverable by deleting it, and a command that deletes is
    /// recoverable only from a place the author may not have looked.
    #[arg(long)]
    apply: bool,
}

/// One record's fate, and the rule that decided it.
struct Fate {
    id: ProofId,
    kind: ProofKind,
    status: ProofStatus,
    captured_at: Timestamp,
    digest: String,
    /// `None` to remove; `Some(rule)` to keep, naming the rule that kept it.
    kept_because: Option<&'static str>,
}

const KEPT_CITED: &str = "cited by a warrant (VDS S-6(3))";
const KEPT_FAILED: &str = "a failure, which is an event and not a standing condition";
const KEPT_RECENT: &str = "among the most recent of its kind";

#[derive(Serialize)]
struct RetentionLog {
    id: String,
    at: Timestamp,
    by: String,
    keep_per_kind: usize,
    /// Why this is lawful, written into the log rather than left in a comment,
    /// so a reader of the record does not have to read the source to know.
    basis: Vec<String>,
    rationale: String,
    removed: Vec<RemovedEntry>,
    kept_totals: BTreeMap<String, usize>,
}

#[derive(Serialize)]
struct RemovedEntry {
    proof_id: String,
    kind: String,
    status: String,
    captured_at: Timestamp,
    /// The record's own digest, so a later reader can confirm which bytes went.
    digest: String,
}

pub fn run(ctx: &Context, args: &Args) -> Result<i32> {
    if args.keep == 0 {
        return Err(VdsError::precondition(
            "--keep 0 would remove the most recent record of every kind, and `vds doctor` \
             settles four of its ten criteria by reading exactly that record. Keep at least \
             one.",
        ));
    }

    let project = ctx.project()?;
    let store = Store::new(&project);

    // Every proof id any warrant names, whatever the warrant's status. A spent
    // warrant is still a historical claim and its basis has to stay readable.
    let mut cited: BTreeSet<String> = BTreeSet::new();
    for warrant in store.read_warrants()? {
        for entry in &warrant.value.evidence {
            cited.insert(entry.proof_id.to_string());
        }
    }

    let proofs = store.read_proofs()?;
    if proofs.is_empty() {
        println!("no proof records, so there is nothing to prune.");
        return Ok(PASSED);
    }

    // Grouped by kind, newest first. `read_proofs` returns them in a stable
    // order, and the identifier carries the capture instant, so sorting by id
    // descending is sorting by time descending.
    let mut by_kind: BTreeMap<ProofKind, Vec<&vds_store::Located<vds_core::ProofResult>>> =
        BTreeMap::new();
    for located in &proofs {
        by_kind.entry(located.value.kind).or_default().push(located);
    }
    for group in by_kind.values_mut() {
        group.sort_by(|a, b| b.value.id.as_str().cmp(a.value.id.as_str()));
    }

    let mut fates: Vec<Fate> = Vec::new();
    for (kind, group) in &by_kind {
        let mut recent_kept = 0usize;
        for located in group {
            let record = &located.value;
            let kept_because = if cited.contains(record.id.as_str()) {
                Some(KEPT_CITED)
            } else if record.status == ProofStatus::Failed {
                Some(KEPT_FAILED)
            } else if recent_kept < args.keep {
                recent_kept += 1;
                Some(KEPT_RECENT)
            } else {
                None
            };
            fates.push(Fate {
                id: record.id.clone(),
                kind: *kind,
                status: record.status,
                captured_at: record.captured_at.clone(),
                digest: record.digest.to_string(),
                kept_because,
            });
        }
    }

    let removing: Vec<&Fate> = fates.iter().filter(|f| f.kept_because.is_none()).collect();

    println!(
        "{} proof records under {}",
        proofs.len(),
        project.rel(&store.proofs_dir())
    );
    for (kind, group) in &by_kind {
        let kept = fates
            .iter()
            .filter(|f| f.kind == *kind && f.kept_because.is_some())
            .count();
        println!(
            "  {:24} {:3} records, {kept} kept, {} to remove",
            kind.as_str(),
            group.len(),
            group.len() - kept
        );
    }

    let cited_count = fates
        .iter()
        .filter(|f| f.kept_because == Some(KEPT_CITED))
        .count();
    let failed = fates
        .iter()
        .filter(|f| f.kept_because == Some(KEPT_FAILED))
        .count();
    println!();
    println!("kept by rule:");
    println!("  {cited_count:3}  {KEPT_CITED}");
    println!("  {failed:3}  {KEPT_FAILED}");
    println!(
        "  {:3}  {KEPT_RECENT} (--keep {})",
        fates
            .iter()
            .filter(|f| f.kept_because == Some(KEPT_RECENT))
            .count(),
        args.keep
    );

    if removing.is_empty() {
        println!();
        println!("nothing to remove.");
        return Ok(PASSED);
    }

    println!();
    println!("{} to remove:", removing.len());
    for fate in removing.iter().take(30) {
        println!("  {} {} {}", fate.id, fate.kind.as_str(), fate.captured_at);
    }
    if removing.len() > 30 {
        println!("  ... and {} more", removing.len() - 30);
    }

    if !args.apply {
        println!();
        println!("Nothing was removed. This command reports by default and deletes only with");
        println!("--apply, because a create is undone by a delete and a delete is undone only");
        println!("from a place the author may not have looked.");
        println!();
        println!("  vds prune --keep {} --apply", args.keep);
        return Ok(PASSED);
    }

    // The log is written BEFORE the first unlink. A crash between them leaves a
    // log naming a record that is still there, which a reader can see is wrong.
    // The other order leaves a record gone and nothing saying so, which a reader
    // cannot.
    let log_dir = project.vds_dir().join("logs").join("retention");
    std::fs::create_dir_all(&log_dir).map_err(|e| VdsError::io(project.rel(&log_dir), e))?;
    let id = next_retention_id(&log_dir)?;
    let log = RetentionLog {
        id: id.clone(),
        at: Timestamp::now(),
        by: actor(),
        keep_per_kind: args.keep,
        basis: vec![
            "VDS S-2(5) deletion limb: nothing downstream resolves against a passing, \
             uncited, superseded proof record"
                .into(),
            "VDS S-2(5) regeneration limb: a proof record is produced by a named command \
             over a named subject and is regenerable by definition (VDS S-7(2)(1))"
                .into(),
            "VDS S-6(3): a record cited by any warrant is never removed, whatever that \
             warrant's status"
                .into(),
            "a failed record is never removed: a failure is an event, and a vacuity is a \
             standing condition carried by the most recent record"
                .into(),
        ],
        rationale: format!(
            "The proof directory is a WORKING SET over an append-only store. `.vds/` is \
             committed, so every record named below remains at the commit that captured it \
             and is reachable by `git log -- .vds/proofs/<id>.yaml`. Its id, kind, status and \
             digest are recorded here, so which bytes went is answerable without git as well. \
             {} removed, {} kept.",
            removing.len(),
            fates.len() - removing.len()
        ),
        removed: removing
            .iter()
            .map(|f| RemovedEntry {
                proof_id: f.id.to_string(),
                kind: f.kind.as_str().to_owned(),
                status: f.status.as_str().to_owned(),
                captured_at: f.captured_at.clone(),
                digest: f.digest.clone(),
            })
            .collect(),
        kept_totals: by_kind
            .keys()
            .map(|kind| {
                (
                    kind.as_str().to_owned(),
                    fates
                        .iter()
                        .filter(|f| f.kind == *kind && f.kept_because.is_some())
                        .count(),
                )
            })
            .collect(),
    };
    let log_path = log_dir.join(format!("{id}.yaml"));
    let text = serde_yaml::to_string(&log).map_err(|e| VdsError::Serialize {
        what: "the retention log".into(),
        message: e.to_string(),
    })?;
    std::fs::write(&log_path, text).map_err(|e| VdsError::io(project.rel(&log_path), e))?;

    for fate in &removing {
        let path = store.proof_path(&fate.id);
        std::fs::remove_file(&path).map_err(|e| VdsError::io(project.rel(&path), e))?;
    }

    println!();
    println!(
        "removed {}, wrote {}",
        removing.len(),
        project.rel(&log_path)
    );
    println!(
        "Every one of them is still at the commit that captured it. Nothing in this log is a \
         design value: an id, a kind, a status, an instant and a digest of a record."
    );
    Ok(PASSED)
}

/// The next identifier, read from the live directory rather than a counter.
///
/// VDS S-4(4): a counter and a directory disagree the first time a write fails
/// between them, and the directory is the one that is true.
fn next_retention_id(dir: &std::path::Path) -> Result<String> {
    let mut highest = 0u32;
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir).map_err(|e| VdsError::io(dir.display(), e))? {
            let entry = entry.map_err(|e| VdsError::io(dir.display(), e))?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            let Some(rest) = name.strip_prefix("RETENTION-") else {
                continue;
            };
            let Some(number) = rest.strip_suffix(".yaml") else {
                continue;
            };
            if let Ok(parsed) = number.parse::<u32>() {
                highest = highest.max(parsed);
            }
        }
    }
    Ok(format!("RETENTION-{:04}", highest + 1))
}
