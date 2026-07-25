//! `vds doctor`: measure this project against the done criteria.
//!
//! docs/GOAL.md says "A criterion with no settling command is not a criterion,
//! it is a hope." This is that command, for the criteria a command can settle.
//!
//! Everything printed here is MEASURED. AGENTS.md: "If you assert a number,
//! produce it with a command and name the command. An unmeasured number is an
//! opinion." So each row names how it was settled, and a criterion this build
//! cannot measure says so rather than being quietly omitted, because a report
//! that lists only what it can check reads as a clean bill of health.

use clap::Args as ClapArgs;
use vds_core::{EXIT_VIOLATION, ProofKind, ProofStatus, Result, Stage, WarrantStatus};
use vds_store::{Store, lock as locklib};

use crate::{Context, PASSED};

#[derive(ClapArgs)]
pub struct Args {
    /// Exit 0 even where criteria are unmet. For reading, not for gating.
    #[arg(long)]
    report_only: bool,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Verdict {
    Met,
    Unmet,
    /// This build cannot settle it, and says why rather than omitting it.
    Unmeasurable,
}

impl Verdict {
    fn mark(self) -> &'static str {
        match self {
            Verdict::Met => "MET       ",
            Verdict::Unmet => "UNMET     ",
            Verdict::Unmeasurable => "NOT CHECKED",
        }
    }
}

struct Row {
    id: &'static str,
    title: &'static str,
    verdict: Verdict,
    detail: Vec<String>,
    settled_by: &'static str,
}

pub fn run(ctx: &Context, args: &Args) -> Result<i32> {
    let project = ctx.project()?;
    let store = Store::new(&project);
    let rows = vec![
        d1(&store)?,
        d2(&store)?,
        d3(&store)?,
        d4(&store)?,
        d5(&store)?,
        d6(&store)?,
        d7(&store)?,
        d8(&store)?,
        d9(&store)?,
        d10(&store)?,
    ];

    println!(
        "VDS doctor: {} ({})",
        project.config.jurisdiction_id,
        project.root.display()
    );
    println!("Measured, not asserted. Each row names the command that settled it, and a criterion");
    println!("this build cannot settle says so rather than being left out.");
    println!();

    for row in &rows {
        println!("{}  {}  {}", row.verdict.mark(), row.id, row.title);
        for line in &row.detail {
            println!("               {line}");
        }
        println!("               settled by: {}", row.settled_by);
        println!();
    }

    let met = rows.iter().filter(|r| r.verdict == Verdict::Met).count();
    let unmet = rows.iter().filter(|r| r.verdict == Verdict::Unmet).count();
    let unmeasured = rows
        .iter()
        .filter(|r| r.verdict == Verdict::Unmeasurable)
        .count();

    println!(
        "{met} met, {unmet} unmet, {unmeasured} not checked, of {} criteria.",
        rows.len()
    );
    if unmeasured > 0 {
        println!(
            "The {unmeasured} not checked are NOT passes. A report that counted them as passes \
             would be the defect VDS exists to prevent."
        );
    }

    if args.report_only || unmet == 0 {
        Ok(PASSED)
    } else {
        Ok(EXIT_VIOLATION)
    }
}

fn d1(store: &Store) -> Result<Row> {
    let last = store.latest_proof(ProofKind::Reconciliation)?;
    let (verdict, detail) = match &last {
        None => (
            Verdict::Unmet,
            vec!["no reconciliation proof has ever run".to_owned()],
        ),
        Some(proof) => {
            let citable = proof.value.is_citable_evidence();
            (
                if citable {
                    Verdict::Met
                } else {
                    Verdict::Unmet
                },
                vec![format!(
                    "last run {} status {} rows_enforced {}",
                    proof.value.id, proof.value.status, proof.value.rows_enforced
                )],
            )
        }
    };
    Ok(Row {
        id: "D1 ",
        title: "the register reconciles, in both directions",
        verdict,
        detail,
        settled_by: "vds proof reconciliation",
    })
}

fn d2(store: &Store) -> Result<Row> {
    let census = store.proof_census()?;
    let lock = store.read_lock()?;
    let mut satisfied = Vec::new();
    let mut short = Vec::new();

    for kind in ProofKind::ALL {
        let (_, last) = census.get(&kind).cloned().unwrap_or((0, None));
        let has_run = last.as_ref().is_some_and(|r| r.rows_enforced > 0);
        let entry = lock
            .as_ref()
            .and_then(|l| l.entries.iter().find(|e| e.proves.contains(&kind)));
        let named_test = entry.is_some();
        let invoked = entry.is_some_and(|e| !e.invoked_by.is_empty());
        let automatic = last
            .as_ref()
            .is_some_and(|r| r.capture_mode == vds_core::CaptureMode::Automatic);

        if kind.is_implemented() && has_run && named_test && invoked && automatic {
            satisfied.push(kind);
        } else {
            let mut why = Vec::new();
            if !kind.is_implemented() {
                why.push("not implemented");
            }
            if !named_test {
                why.push("no lock entry naming a failing-direction test");
            }
            if !invoked {
                why.push("no invocation");
            }
            if !has_run {
                why.push("no run with rows_enforced > 0");
            }
            if !automatic {
                why.push("no automatic capture");
            }
            short.push(format!("{kind}: {}", why.join(", ")));
        }
    }

    Ok(Row {
        id: "D2 ",
        title: "every proof kind is valid on all five limbs of VDS S-7(2)",
        verdict: if satisfied.len() == ProofKind::ALL.len() {
            Verdict::Met
        } else {
            Verdict::Unmet
        },
        detail: std::iter::once(format!(
            "{} of {} kinds satisfy all five limbs",
            satisfied.len(),
            ProofKind::ALL.len()
        ))
        .chain(short)
        .collect(),
        settled_by: "a cross-check of .vds/proofs/ against .vds/enforcement.lock",
    })
}

fn d3(store: &Store) -> Result<Row> {
    let census = store.proof_census()?;
    let vacuous: Vec<String> = census
        .iter()
        .filter_map(|(kind, (_, last))| {
            last.as_ref()
                .filter(|r| r.status == ProofStatus::Vacuous)
                .map(|r| format!("{kind}: {} is vacuous", r.id))
        })
        .collect();
    Ok(Row {
        id: "D3 ",
        title: "no vacuous passes",
        verdict: if vacuous.is_empty() {
            Verdict::Met
        } else {
            Verdict::Unmet
        },
        detail: if vacuous.is_empty() {
            vec!["no proof kind's most recent result is vacuous".to_owned()]
        } else {
            vacuous
        },
        settled_by: "a scan over .vds/proofs/",
    })
}

fn d4(store: &Store) -> Result<Row> {
    let Some(lock) = store.read_lock()? else {
        return Ok(Row {
            id: "D4 ",
            title: "every gate is invoked by CI, not only by a hook",
            verdict: Verdict::Unmet,
            detail: vec![
                "no enforcement.lock, so no gate is pinned and none is invoked".to_owned(),
            ],
            settled_by: "a scan over .vds/enforcement.lock",
        });
    };
    let hook_only: Vec<String> = lock
        .entries
        .iter()
        .filter(|e| !e.has_blocking_ci())
        .map(|e| format!("{}: no blocking ci_workflow invocation", e.path))
        .collect();
    Ok(Row {
        id: "D4 ",
        title: "every gate is invoked by CI, not only by a hook",
        verdict: if hook_only.is_empty() && !lock.entries.is_empty() {
            Verdict::Met
        } else {
            Verdict::Unmet
        },
        detail: if hook_only.is_empty() {
            vec![format!(
                "{} pinned gates, every one invoked by a blocking ci_workflow",
                lock.entries.len()
            )]
        } else {
            hook_only
        },
        settled_by: "a scan over .vds/enforcement.lock",
    })
}

fn d5(store: &Store) -> Result<Row> {
    let gates: Vec<String> = vds_proof::GATE_PATHS
        .iter()
        .map(|g| (*g).to_owned())
        .collect();
    let verdict = locklib::verify_lock(store, &gates)?;
    Ok(Row {
        id: "D5 ",
        title: "zero enforcement-surface drift",
        verdict: if verdict.is_clean() {
            Verdict::Met
        } else {
            Verdict::Unmet
        },
        detail: if verdict.is_clean() {
            vec!["every pinned path matches its digest".to_owned()]
        } else {
            verdict.findings.iter().map(|f| f.to_string()).collect()
        },
        settled_by: "vds lock verify",
    })
}

fn d6(store: &Store) -> Result<Row> {
    let mut detail = Vec::new();
    let mut complete = true;
    for stage in Stage::ALL {
        match store.granted_warrant(stage)? {
            None => {
                detail.push(format!("{}: not granted", stage.short()));
                complete = false;
            }
            Some(warrant) => {
                let mut problems = Vec::new();
                if let Some(reason) = warrant.value.void_on_its_face() {
                    problems.push(reason.to_owned());
                }
                for entry in &warrant.value.evidence {
                    match store.read_proof(&entry.proof_id) {
                        Err(_) => problems.push(format!("{} is not on disk", entry.proof_id)),
                        Ok(proof) if proof.value.digest != entry.digest => {
                            problems.push(format!("{} digest mismatch", entry.proof_id))
                        }
                        Ok(_) => {}
                    }
                }
                if problems.is_empty() {
                    detail.push(format!("{}: {} granted", stage.short(), warrant.value.id));
                } else {
                    detail.push(format!("{}: {}", stage.short(), problems.join("; ")));
                    complete = false;
                }
            }
        }
    }
    Ok(Row {
        id: "D6 ",
        title: "the warrant chain is complete for the declared surface",
        verdict: if complete {
            Verdict::Met
        } else {
            Verdict::Unmet
        },
        detail,
        settled_by: "resolving every id and digest in the four warrants",
    })
}

fn d7(store: &Store) -> Result<Row> {
    let last = store.latest_proof(ProofKind::NoStoredValues)?;
    let (verdict, detail) = match &last {
        None => (
            Verdict::Unmet,
            vec!["no no_stored_values proof has ever run".to_owned()],
        ),
        Some(proof) => (
            if proof.value.is_citable_evidence() {
                Verdict::Met
            } else {
                Verdict::Unmet
            },
            vec![format!(
                "last run {} status {} with {} findings over {} files",
                proof.value.id,
                proof.value.status,
                proof.value.violations.len(),
                proof.value.rows_enforced
            )],
        ),
    };
    Ok(Row {
        id: "D7 ",
        title: ".vds/ holds no design value",
        verdict,
        detail,
        settled_by: "vds proof no_stored_values",
    })
}

fn d8(store: &Store) -> Result<Row> {
    let last = store.latest_proof(ProofKind::LedgerStaleness)?;
    let (verdict, detail) = match &last {
        None => (
            Verdict::Unmet,
            vec!["no ledger_staleness proof has ever run".to_owned()],
        ),
        Some(proof) => (
            if proof.value.is_citable_evidence() {
                Verdict::Met
            } else {
                Verdict::Unmet
            },
            vec![format!(
                "last run {} status {} over {} ledgers",
                proof.value.id, proof.value.status, proof.value.rows_enforced
            )],
        ),
    };
    Ok(Row {
        id: "D8 ",
        title: "every ledger is current with its source",
        verdict,
        detail,
        settled_by: "vds proof ledger_staleness",
    })
}

fn d9(store: &Store) -> Result<Row> {
    let proofs = store.read_proofs()?.len();
    let granted = store
        .read_warrants()?
        .iter()
        .filter(|w| w.value.status == WarrantStatus::Granted)
        .count();
    let decisions = count_files(
        &store
            .project
            .path(vds_core::PathRole::Logs)
            .join("decisions"),
    );
    Ok(Row {
        id: "D9 ",
        title: "proof records keep pace with decisions",
        verdict: if proofs >= granted {
            Verdict::Met
        } else {
            Verdict::Unmet
        },
        detail: {
            let mut detail = vec![
                format!("{proofs} proof records against {granted} granted warrants"),
                format!(
                    "{decisions} decision logs. Measured in VJS at drafting: 173 decision logs \
                     against 3 proof records. The proof surface is the one that rots."
                ),
            ];
            // The criterion is met by a LARGE number and undermined by an
            // enormous one. Past a few hundred, the pile buries the one record
            // per kind that four other criteria are settled by reading, and
            // nobody opens the directory again. Said here rather than made a
            // failure, because the count is not itself a defect.
            if proofs > 200 {
                detail.push(format!(
                    "{proofs} is past the point where the directory is readable, and the \
                     record each of D2, D3, D7 and D8 is settled by is the most recent one of \
                     its kind. `vds prune` keeps that one, keeps every failure, keeps anything \
                     a warrant cites, and logs what it removed."
                ));
            }
            detail
        },
        settled_by: "two directory counts",
    })
}

fn d10(store: &Store) -> Result<Row> {
    // The five reserved matters at VDS S-13, plus SUBMISSION-VDS-006 which this
    // port opened by departing from drafted S-2(7).
    const RESERVED: &[(&str, &str)] = &[
        (
            "SUBMISSION-VDS-001",
            "S-6(5) may W1 be granted provisionally",
        ),
        ("SUBMISSION-VDS-002", "S-6(6) who may grant W2"),
        ("SUBMISSION-VDS-003", "S-3(6) what a designpack binds"),
        ("SUBMISSION-VDS-004", "S-9(9) forced-drain retirement"),
        (
            "SUBMISSION-VDS-005",
            "S-9(10) where the primitive floor sits",
        ),
        ("SUBMISSION-VDS-006", "S-2(7) the pin's per-value digests"),
    ];
    let filed = store.read_submissions()?;
    let mut detail = Vec::new();
    let mut all_filed = true;
    for (id, question) in RESERVED {
        match filed.iter().find(|s| s.value.id.as_str() == *id) {
            Some(submission) => {
                let defects = submission.value.defects();
                if defects.is_empty() {
                    detail.push(format!("{id} filed: {question}"));
                } else {
                    detail.push(format!("{id} filed and DEFECTIVE: {}", defects.join("; ")));
                    all_filed = false;
                }
            }
            None => {
                detail.push(format!("{id} MISSING: {question}"));
                all_filed = false;
            }
        }
    }
    Ok(Row {
        id: "D10",
        title: "every RESERVED clause resolves to an open or answered submission",
        verdict: if all_filed {
            Verdict::Met
        } else {
            Verdict::Unmet
        },
        detail,
        settled_by: "a cross-check between VDS S-13 and .vds/submissions/",
    })
}

fn count_files(dir: &std::path::Path) -> usize {
    std::fs::read_dir(dir)
        .map(|entries| entries.flatten().filter(|e| e.path().is_file()).count())
        .unwrap_or(0)
}
