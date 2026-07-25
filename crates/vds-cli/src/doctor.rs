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
    // PARSED, not counted. This used to list the directory, which counts a file
    // that does not parse as a decision log the same as one that does, and
    // reports a well-formed log with an empty `why` as though a fork had been
    // recorded. A count taken without opening anything is a count of files.
    let decisions = store.read_decisions()?;
    let breaches = store.read_breaches()?;
    let defective: Vec<String> = decisions
        .iter()
        .flat_map(|d| {
            d.value
                .defects()
                .into_iter()
                .map(move |defect| format!("{}: {defect}", d.value.id))
        })
        .chain(breaches.iter().flat_map(|b| {
            b.value
                .defects()
                .into_iter()
                .map(move |defect| format!("{}: {defect}", b.value.id))
        }))
        .collect();

    // VDS S-12(4): the two defects at S-1(4) ARE the founding breach entries and
    // are filed as breaches rather than described as background, "because a
    // system whose first act is to excuse the failures that motivated it has
    // taught itself the wrong lesson". Only the jurisdiction that ships VDS.md
    // owns them; a subject project files its own breaches or none.
    let owns_the_specification = matches!(
        reserved_clause_owner(store),
        ReservedClauseOwner::ThisProject
    );
    let founding_unfiled = owns_the_specification && breaches.len() < 2;
    Ok(Row {
        id: "D9 ",
        title: "proof records keep pace with decisions",
        verdict: if proofs >= granted && defective.is_empty() && !founding_unfiled {
            Verdict::Met
        } else {
            Verdict::Unmet
        },
        detail: {
            let mut detail = vec![
                format!("{proofs} proof records against {granted} granted warrants"),
                format!(
                    "{} decision logs and {} breach reports, all parsed and checked. Measured \
                     in VJS at drafting: 173 decision logs against 3 proof records. The proof \
                     surface is the one that rots.",
                    decisions.len(),
                    breaches.len()
                ),
            ];
            if founding_unfiled {
                detail.push(
                    "VDS S-12(4) makes the two defects at S-1(4) the FOUNDING breach entries, \
                     filed as breaches rather than described as background. Fewer than two are \
                     on file, so that clause is unmet. File them: vds log breach"
                        .to_owned(),
                );
            }
            detail.extend(defective.iter().map(|d| format!("DEFECTIVE {d}")));
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
        settled_by: "the proof, warrant, decision and breach records, each opened and checked",
    })
}

/// Who owns the specification's reserved clauses, which decides what D10 asks.
///
/// The clauses live in VDS.md, so the project that SHIPS VDS.md is the project
/// that answers them, and every other project receives the answers through a
/// vendored designpack (VDS S-11(3): no doctrine flows downstream by silence).
///
/// Before this distinction existed, D10 demanded that every project hold
/// `SUBMISSION-VDS-001` through `-006` in its own `.vds/submissions/`, and
/// running `vds doctor` on `examples/storefront` printed six MISSING lines about
/// questions that subject has no standing to answer and no business re-filing.
/// A criterion that reports a project as failing for not duplicating another
/// project's paperwork is a criterion that teaches a reader to skip it.
enum ReservedClauseOwner {
    /// The project holds VDS.md: it IS the jurisdiction.
    ThisProject,
    /// A designpack is vendored, so the answers arrive pinned from upstream.
    Upstream { pack: String },
    /// Neither. The project cannot see the reserved clauses at all.
    Nowhere,
}

fn reserved_clause_owner(store: &Store) -> ReservedClauseOwner {
    let project = store.project;
    if project.root.join("VDS.md").is_file() {
        return ReservedClauseOwner::ThisProject;
    }
    let (id, version) = project.config.designpack_parts();
    if id == "none" {
        return ReservedClauseOwner::Nowhere;
    }
    ReservedClauseOwner::Upstream {
        pack: format!("{id}@{version}"),
    }
}

fn d10(store: &Store) -> Result<Row> {
    match reserved_clause_owner(store) {
        ReservedClauseOwner::ThisProject => d10_as_the_jurisdiction(store),
        ReservedClauseOwner::Upstream { pack } => Ok(Row {
            id: "D10",
            title: "every RESERVED clause resolves to an open or answered submission",
            // The pack digest is checked by `vds pack verify`, which is D5's
            // business. Here the question is only whether the project is
            // receiving doctrine at all, and it is.
            verdict: Verdict::Met,
            detail: vec![
                format!(
                    "this project vendors {pack}, so the specification's reserved clauses are \
                     answered upstream and arrive pinned (VDS S-11(3))"
                ),
                "A subscriber does not re-file another jurisdiction's submissions. Run `vds \
                 pack verify` for whether what arrived is what was pinned."
                    .to_owned(),
            ],
            settled_by: "the designpack pin in .vds/config.toml",
        }),
        ReservedClauseOwner::Nowhere => Ok(Row {
            id: "D10",
            title: "every RESERVED clause resolves to an open or answered submission",
            verdict: Verdict::Unmet,
            detail: vec![
                "this project holds no VDS.md and vendors no designpack, so it cannot see the \
                 specification's reserved clauses at all, answered or open."
                    .to_owned(),
                "That is not a missing file in this repository. VDS S-15(1): the specification \
                 commences on a dated, digest-pinned assent event, and until a pack is vendored \
                 there is nothing downstream for a reserved clause to resolve against."
                    .to_owned(),
            ],
            settled_by: "the absence of both VDS.md and a vendored designpack",
        }),
    }
}

/// Every clause VDS.md marks RESERVED, read out of the specification itself.
///
/// The marker is `**S-<clause> RESERVED.**` at the head of a paragraph, which is
/// the form S-1 announces: "Clauses marked **RESERVED** depend on a point that is
/// not settled."
///
/// Derived rather than listed, which is the same ratio this repository applies to
/// the JSON schemas. The list used to be six hardcoded pairs of submission id and
/// question, and a hardcoded list is a second copy of the specification: add a
/// RESERVED clause to VDS.md and D10 would go on reporting MET while covering one
/// clause fewer than exists. Now adding one to the specification makes D10 unmet
/// until a submission names it, which is the behaviour the criterion claims.
fn reserved_clauses_in(specification: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in specification.lines() {
        let Some(rest) = line.trim_start().strip_prefix("**S-") else {
            continue;
        };
        let Some((clause, _)) = rest.split_once(" RESERVED.**") else {
            continue;
        };
        let clause = format!("S-{clause}");
        if !out.contains(&clause) {
            out.push(clause);
        }
    }
    out
}

fn d10_as_the_jurisdiction(store: &Store) -> Result<Row> {
    let path = store.project.root.join("VDS.md");
    let specification = std::fs::read_to_string(&path)
        .map_err(|e| vds_core::VdsError::io(store.project.rel(&path), e))?;
    let reserved = reserved_clauses_in(&specification);

    let filed = store.read_submissions()?;
    let mut detail = vec![format!(
        "{} clauses marked RESERVED in VDS.md, {} submissions on file",
        reserved.len(),
        filed.len()
    )];
    let mut all_filed = true;

    for clause in &reserved {
        match filed
            .iter()
            .find(|s| s.value.reserved_clause.as_deref() == Some(clause.as_str()))
        {
            Some(submission) => {
                let defects = submission.value.defects();
                if defects.is_empty() {
                    detail.push(format!(
                        "{clause} -> {} filed: {}",
                        submission.value.id, submission.value.question
                    ));
                } else {
                    detail.push(format!(
                        "{clause} -> {} filed and DEFECTIVE: {}",
                        submission.value.id,
                        defects.join("; ")
                    ));
                    all_filed = false;
                }
            }
            None => {
                detail.push(format!(
                    "{clause} MISSING: VDS.md reserves it and no submission names it, so the \
                     clause fails closed with nobody asked (VDS S-15(3))"
                ));
                all_filed = false;
            }
        }
    }

    // A submission that names no reserved clause is not a defect: a departure
    // from the drafted specification (SUBMISSION-VDS-006 on S-2(7)) is a question
    // in its own right. It is listed so the count above adds up.
    for submission in &filed {
        let names_reserved = submission
            .value
            .reserved_clause
            .as_deref()
            .is_some_and(|c| reserved.iter().any(|r| r == c));
        if !names_reserved {
            let defects = submission.value.defects();
            detail.push(format!(
                "{} filed on {}, which VDS.md does not mark RESERVED{}",
                submission.value.id,
                submission
                    .value
                    .reserved_clause
                    .as_deref()
                    .unwrap_or("no clause"),
                if defects.is_empty() {
                    String::new()
                } else {
                    format!(" and is DEFECTIVE: {}", defects.join("; "))
                }
            ));
            if !defects.is_empty() {
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
        settled_by: "the RESERVED markers read out of VDS.md, cross-checked against \
                     .vds/submissions/",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The list D10 checks is the specification's, not a copy of it.
    ///
    /// Before this was derived, six pairs of submission id and question were
    /// hardcoded here. Adding a RESERVED clause to VDS.md would have left D10
    /// reporting MET while covering one clause fewer than exists, which is the
    /// silent-narrowing failure the criterion is for.
    #[test]
    fn the_reserved_clauses_are_read_out_of_the_specification() {
        let specification = "\
**S-1** Ordinary text mentioning RESERVED in passing.\n\
\n\
**S-3(6) RESERVED.** Whether one designpack binds a single project.\n\
\n\
**S-6(5) RESERVED.** Whether W1 may be granted provisionally.\n\
\n\
| **W2** | something | RESERVED, S-6(6) | a table row, not a marker |\n\
\n\
**S-6(5) RESERVED.** A duplicate, which must not be counted twice.\n";
        assert_eq!(
            reserved_clauses_in(specification),
            vec!["S-3(6)".to_owned(), "S-6(5)".to_owned()],
            "the marker is a paragraph head, not any line containing the word"
        );
    }

    /// The real specification, so the parser cannot pass its own fixture and
    /// fail the file it exists to read.
    #[test]
    fn the_committed_specification_reserves_the_clauses_it_says_it_does() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("VDS.md");
        let specification = std::fs::read_to_string(path).expect("VDS.md");
        let reserved = reserved_clauses_in(&specification);
        for clause in ["S-3(6)", "S-6(5)", "S-6(6)", "S-9(9)", "S-9(10)"] {
            assert!(
                reserved.contains(&clause.to_owned()),
                "{clause} is marked RESERVED in VDS.md and the parser missed it: {reserved:?}"
            );
        }
        assert_eq!(
            reserved.len(),
            5,
            "the specification reserves {} clauses. If that is right, this number moves with \
             it deliberately; if it is not, a marker has been misread: {reserved:?}",
            reserved.len()
        );
    }
}
