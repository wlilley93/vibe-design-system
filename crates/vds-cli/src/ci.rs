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

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use vds_core::{Result, Timestamp, VdsError};
use vds_store::Store;

pub const SCHEMA_VERSION: u32 = 1;

/// One workflow, and what its runs did.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRuns {
    /// The workflow's path as the lock names it, e.g. `.github/workflows/vds-enforce.yml`.
    pub file: String,
    /// How many runs the source response carried. A denominator, stated so a narrow
    /// window is visible as a narrow window.
    pub runs_considered: usize,
    /// How many of those runs actually reached a conclusion. This is a SEPARATE number
    /// from `runs_considered` because a run still in flight is not evidence either way,
    /// and conflating the two made `never_succeeded` fire on a window where nothing had
    /// finished yet - caught by its own test, which is why the field exists.
    pub runs_concluded: usize,
    pub successes: usize,
    /// The newest run's conclusion verbatim, whatever the forge called it.
    pub newest_conclusion: Option<String>,
    pub newest_at: Option<String>,
    pub oldest_at: Option<String>,
    /// The newest run's commit, so a reader can see WHICH tree passed. Nothing here
    /// compares it to HEAD.
    pub newest_head_sha: Option<String>,
    /// The `name:` declared inside the workflow file, which is what the forge reports per
    /// run. Recorded because the join between the lock's PATH and the forge's NAME is the
    /// one place this ledger could silently match the wrong workflow, and a join nobody
    /// can see is a join nobody can check.
    pub joined_on_name: Option<String>,
    /// Present only when it is true, and it is the whole point of the ledger. It means
    /// runs CONCLUDED and none of them passed - not merely that no success has been seen,
    /// which would also be true of a window where everything is still running.
    #[serde(skip_serializing_if = "std::ops::Not::not", default)]
    pub never_succeeded: bool,
    /// Every distinct conclusion with its count, because "53 failures" and "40 failures
    /// and 13 cancelled" want different answers.
    pub conclusions: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiLedger {
    pub schema_version: u32,
    pub generated_at: Timestamp,
    pub generated_by: String,
    /// The exact command that produced the source bytes, so the derivation is repeatable
    /// by a reader who has the credential.
    pub source: String,
    pub workflows: Vec<WorkflowRuns>,
    pub notes: Vec<String>,
}

impl CiLedger {
    pub fn row(&self, file: &str) -> Option<&WorkflowRuns> {
        self.workflows.iter().find(|w| w.file == file)
    }
}

/// One run as `gh run list --json` reports it.
///
/// `conclusion` is `Option` and an EMPTY STRING is normalised to absent, because a run
/// that is still in progress reports `""` rather than null, and counting that as a
/// distinct conclusion would put `""` in the histogram.
#[derive(Debug, Deserialize)]
struct GhRun {
    #[serde(default)]
    conclusion: Option<String>,
    #[serde(default, rename = "createdAt")]
    created_at: Option<String>,
    #[serde(default, rename = "headSha")]
    head_sha: Option<String>,
    /// The forge reports the workflow's DECLARED NAME per run, not its path. `gh run list`
    /// has no `path` field at all - asking for one is refused by name, which is how this
    /// was found. So the join is lock-path -> the `name:` inside that file -> this.
    #[serde(default, rename = "workflowName")]
    workflow_name: Option<String>,
}

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
    let ledger: CiLedger = serde_yaml::from_str(&text).map_err(|e| VdsError::Artefact {
        path: store.project.rel(&path),
        message: format!("is not a CI run ledger: {e}"),
    })?;
    if ledger.schema_version != SCHEMA_VERSION {
        return Err(VdsError::Artefact {
            path: store.project.rel(&path),
            message: format!(
                "is schema_version {} and this build reads {SCHEMA_VERSION}. Regenerate it \
                 rather than editing it: a ledger edited to parse is a ledger nobody measured.",
                ledger.schema_version
            ),
        });
    }
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

/// Derive the ledger from a `gh run list --json ...` response.
///
/// The row is keyed on `workflow_file`, the path the LOCK uses, because that is what D4
/// looks up. The forge does not report a path - `gh run list` has no such field - so the
/// filter is `workflow_name`, read out of the workflow file's own `name:` by the caller
/// rather than guessed from the filename.
///
/// `workflow_name` is `Option` for one honest reason: a workflow file with no `name:` key
/// is legal, and in that case the forge names runs after the path. Passing `None` means
/// "do not filter", and the ledger says so in a note - because a filter that silently
/// matches everything is worse than no filter, and this is exactly the shape that let a
/// success from another workflow get borrowed in the first place.
pub fn derive(
    workflow_file: &str,
    workflow_name: Option<&str>,
    source: &str,
    raw: &str,
    generated_by: &str,
) -> Result<CiLedger> {
    let runs: Vec<GhRun> = serde_json::from_str(raw).map_err(|e| VdsError::Artefact {
        path: source.to_owned(),
        message: format!(
            "is not a `gh run list --json` array: {e}. Expected a JSON array of objects \
             carrying at least `conclusion` and `createdAt`."
        ),
    })?;

    let mut notes = Vec::new();
    let total_in_response = runs.len();
    let mut mismatched = 0usize;
    let mut kept: Vec<&GhRun> = Vec::new();
    for run in &runs {
        match (workflow_name, run.workflow_name.as_deref()) {
            (Some(want), Some(got)) if got != want => mismatched += 1,
            _ => kept.push(run),
        }
    }
    if mismatched > 0 {
        notes.push(format!(
            "{mismatched} of {total_in_response} runs in the response named a different \
             workflow and were EXCLUDED. A response for another workflow is not evidence \
             about this one."
        ));
    }
    match workflow_name {
        Some(name) => notes.push(format!(
            "joined lock path {workflow_file} to forge runs by the workflow's declared \
             name {name:?}. `gh run list` reports no path, so this join is the one place \
             the wrong workflow could be measured."
        )),
        None => notes.push(format!(
            "NO NAME FILTER WAS APPLIED: {workflow_file} declares no `name:`, so every run \
             in the response was counted. If the response covers more than one workflow, \
             this row is not about {workflow_file} alone."
        )),
    }

    // Newest first is what `gh run list` returns, and the ledger does not re-sort: it
    // records `newest_at` and `oldest_at` from the ends so a reader can see the window
    // and check the assumption rather than take it.
    let mut conclusions: BTreeMap<String, usize> = BTreeMap::new();
    let mut successes = 0usize;
    let mut in_progress = 0usize;
    for run in &kept {
        match run
            .conclusion
            .as_deref()
            .map(str::trim)
            .filter(|c| !c.is_empty())
        {
            Some("success") => {
                successes += 1;
                *conclusions.entry("success".to_owned()).or_default() += 1;
            }
            Some(other) => *conclusions.entry(other.to_owned()).or_default() += 1,
            None => in_progress += 1,
        }
    }
    if in_progress > 0 {
        notes.push(format!(
            "{in_progress} runs had no conclusion yet and are counted in \
             runs_considered but in no conclusion bucket."
        ));
    }

    let newest = kept.first();
    let oldest = kept.last();
    let runs_concluded = kept.len().saturating_sub(in_progress);
    let row = WorkflowRuns {
        file: workflow_file.to_owned(),
        runs_considered: kept.len(),
        runs_concluded,
        successes,
        newest_conclusion: newest
            .and_then(|r| r.conclusion.clone())
            .map(|c| c.trim().to_owned())
            .filter(|c| !c.is_empty()),
        newest_at: newest.and_then(|r| r.created_at.clone()),
        oldest_at: oldest.and_then(|r| r.created_at.clone()),
        newest_head_sha: newest.and_then(|r| r.head_sha.clone()),
        joined_on_name: workflow_name.map(str::to_owned),
        never_succeeded: successes == 0 && runs_concluded > 0,
        conclusions,
    };

    if row.never_succeeded {
        notes.push(format!(
            "NOT ONE of the {} runs that CONCLUDED in this window succeeded. Every gate \
             whose lock entry names this workflow is declared and unrun.",
            row.runs_concluded
        ));
    }
    if kept.is_empty() {
        notes.push(
            "the response carried no runs for this workflow, so this ledger establishes \
             nothing about it. An empty window is not a passing window."
                .to_owned(),
        );
    }
    notes.push(
        "This ledger records CONCLUSIONS. It does not establish that a successful run \
         executed any particular gate, nor that it ran over the current tree."
            .to_owned(),
    );

    Ok(CiLedger {
        schema_version: SCHEMA_VERSION,
        generated_at: Timestamp::now(),
        generated_by: generated_by.to_owned(),
        source: source.to_owned(),
        workflows: vec![row],
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUCCESS_RUN: &str = r#"[
      {"conclusion":"success","createdAt":"2026-07-31T07:38:23Z","headSha":"abc","workflowName":"vds-enforce"},
      {"conclusion":"failure","createdAt":"2026-07-30T21:14:23Z","headSha":"def","workflowName":"vds-enforce"}
    ]"#;

    /// The real, measured shape: every run failed because the job never started.
    const NEVER_STARTED: &str = r#"[
      {"conclusion":"failure","createdAt":"2026-07-31T07:38:23Z","headSha":"abc","workflowName":"vds-enforce"},
      {"conclusion":"failure","createdAt":"2026-07-25T14:06:34Z","headSha":"def","workflowName":"vds-enforce"}
    ]"#;

    #[test]
    fn a_workflow_that_never_succeeded_is_recorded_as_never_succeeded() {
        let l = derive(
            ".github/workflows/vds-enforce.yml",
            Some("vds-enforce"),
            "test",
            NEVER_STARTED,
            "test",
        )
        .unwrap();
        let row = l.row(".github/workflows/vds-enforce.yml").unwrap();
        assert!(row.never_succeeded, "two failures and no success");
        assert_eq!(row.successes, 0);
        assert_eq!(row.runs_considered, 2);
        assert_eq!(row.conclusions.get("failure"), Some(&2));
        assert_eq!(row.newest_at.as_deref(), Some("2026-07-31T07:38:23Z"));
        assert_eq!(row.oldest_at.as_deref(), Some("2026-07-25T14:06:34Z"));
        assert!(
            l.notes.iter().any(|n| n.contains("NOT ONE")),
            "the ledger must say so in words, not only in a boolean: {:?}",
            l.notes
        );
    }

    #[test]
    fn one_success_clears_never_succeeded() {
        let l = derive(
            ".github/workflows/vds-enforce.yml",
            Some("vds-enforce"),
            "test",
            SUCCESS_RUN,
            "test",
        )
        .unwrap();
        let row = l.row(".github/workflows/vds-enforce.yml").unwrap();
        assert!(!row.never_succeeded);
        assert_eq!(row.successes, 1);
        assert_eq!(row.newest_conclusion.as_deref(), Some("success"));
    }

    /// An empty window must not read as a clean one. This is the vacuity rule applied to
    /// a ledger: nothing observed is not nothing wrong.
    #[test]
    fn an_empty_window_establishes_nothing_and_says_so() {
        let l = derive(
            ".github/workflows/vds-enforce.yml",
            Some("vds-enforce"),
            "test",
            "[]",
            "test",
        )
        .unwrap();
        let row = l.row(".github/workflows/vds-enforce.yml").unwrap();
        assert_eq!(row.runs_considered, 0);
        assert!(
            !row.never_succeeded,
            "never_succeeded means runs happened and none passed; zero runs is a \
             different statement and must not borrow this one"
        );
        assert!(l.notes.iter().any(|n| n.contains("not a passing window")));
    }

    /// A response for the wrong workflow must not be counted as evidence about this one.
    #[test]
    fn runs_naming_another_workflow_are_excluded_and_counted() {
        let raw = r#"[
          {"conclusion":"success","createdAt":"2026-07-31T07:38:23Z","workflowName":"other"},
          {"conclusion":"failure","createdAt":"2026-07-30T21:14:23Z","workflowName":"vds-enforce"}
        ]"#;
        let l = derive(
            ".github/workflows/vds-enforce.yml",
            Some("vds-enforce"),
            "test",
            raw,
            "test",
        )
        .unwrap();
        let row = l.row(".github/workflows/vds-enforce.yml").unwrap();
        assert_eq!(
            row.runs_considered, 1,
            "the other workflow's run is excluded"
        );
        assert_eq!(
            row.successes, 0,
            "the success belonged to another workflow and must not be borrowed"
        );
        assert!(l.notes.iter().any(|n| n.contains("EXCLUDED")));
    }

    /// A run still in flight reports `""`, not null. It must not become a conclusion.
    #[test]
    fn an_in_flight_run_is_not_a_conclusion() {
        let raw = r#"[{"conclusion":"","createdAt":"2026-07-31T09:00:00Z"}]"#;
        let l = derive(
            ".github/workflows/vds-enforce.yml",
            Some("vds-enforce"),
            "test",
            raw,
            "test",
        )
        .unwrap();
        let row = l.row(".github/workflows/vds-enforce.yml").unwrap();
        assert!(
            row.conclusions.is_empty(),
            "an empty-string conclusion must not appear in the histogram: {:?}",
            row.conclusions
        );
        assert_eq!(row.runs_considered, 1);
        assert!(!row.never_succeeded, "nothing has concluded yet");
    }
}
