//! `vds warrant`: report warrant status, or record a warrant granted elsewhere.
//!
//! **VDS grants nothing.** W1, W2 and W4 are VJS's on a referred submission and
//! W3 is the Principal's alone (VDS S-1(3), S-6(2), S-6(7)). `record` writes
//! down a grant that already happened and pins the evidence it was made on. If
//! no such grant happened, the file it writes is a false statement of the record
//! and not a warrant, and it says so on every run.
//!
//! Two things the retired tool asserted and did not do:
//!
//!   - **The stage ordering.** VDS S-6(2) calls it "the entire mechanism", and
//!     nothing checked it: a W3 recorded as granted with no W1 and no W2 on
//!     disk. [`check_stage_order`] is that check.
//!   - **The surface.** VDS S-6(4) spends a warrant when the surface it was
//!     granted over changes. The surface digest was read from the generated
//!     screens ledger, so a screen edited without regenerating left the warrant
//!     looking unspent at exactly the moment it was not. [`live_surface`]
//!     measures the screens themselves.

use clap::{Args as ClapArgs, Subcommand};
use vds_core::{
    AcceptanceEvent, AssentSource, Digest, EXIT_VIOLATION, EvidenceEntry, GrantedBy, ProofId,
    ProofStatus, Project, RECORDING_IS_NOT_GRANTING, Result, Stage, Surface, Timestamp, VdsError,
    Warrant, WarrantId, WarrantStatus,
};
use vds_store::Store;

use crate::{Context, PASSED};

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Report every stage, its evidence, and whether the surface has moved.
    Status,
    /// Write down a grant that already happened. This does NOT grant.
    Record(RecordArgs),
    /// Mark a warrant spent because its surface changed (VDS S-6(4)).
    Spend { id: String },
}

pub fn run(ctx: &Context, args: &Args) -> Result<i32> {
    let project = ctx.project()?;
    let store = Store::new(&project);
    match &args.action {
        Action::Status => status(&store),
        Action::Record(a) => record(&store, a),
        Action::Spend { id } => spend(&store, id),
    }
}

/// The surface a warrant is granted over, measured from live state.
///
/// Both halves are measured now: the screen files matching the declared globs,
/// and the register directory. Neither is read back from a generated artefact,
/// because a generated artefact goes stale exactly when the thing it describes
/// changes, which is the one moment this digest has to be right.
pub fn live_surface(project: &Project) -> Result<Surface> {
    let files = vds_scan::screen_files(project)?;
    Ok(Surface {
        screens_digest: vds_scan::source_digest(project, &files)?,
        register_digest: project.register_digest()?,
    })
}

fn status(store: &Store) -> Result<i32> {
    let warrants = store.read_warrants()?;
    let live = live_surface(store.project)?;
    let mut problems = 0usize;

    println!("VDS grants nothing. Granting W1, W2 and W4 is VJS's on a referred submission,");
    println!("and W3 is the Principal's alone (VDS S-1(3), S-6(2), S-6(7)). This is a report.");
    println!();
    println!("live surface  screens_digest:  {}", live.screens_digest);
    println!("              register_digest: {}", live.register_digest);
    println!();

    for stage in Stage::ALL {
        let held: Vec<_> = warrants.iter().filter(|w| w.value.stage == stage).collect();
        println!("{} {}", stage.short(), stage.as_str());

        if held.is_empty() {
            println!("   status: NOT GRANTED, no warrant record exists");
            problems += 1;
        }

        for warrant in &held {
            println!("   {}  status: {}", warrant.value.id, warrant.value.status);

            if let Some(reason) = warrant.value.void_on_its_face() {
                println!("     VOID ON ITS FACE: {reason}");
                problems += 1;
            }

            if warrant.value.status == WarrantStatus::Granted {
                if warrant.value.is_spent_by(&live) {
                    println!(
                        "     SPENT: the surface has changed since this warrant was granted \
                         (VDS S-6(4))."
                    );
                    if let Some(granted) = &warrant.value.surface {
                        if granted.screens_digest != live.screens_digest {
                            println!("       screens_digest granted-on: {}", granted.screens_digest);
                            println!("       screens_digest now:        {}", live.screens_digest);
                        }
                        if granted.register_digest != live.register_digest {
                            println!("       register_digest granted-on: {}", granted.register_digest);
                            println!("       register_digest now:        {}", live.register_digest);
                        }
                    }
                    println!("     Record it: vds warrant spend {}", warrant.value.id);
                    problems += 1;
                }
                if warrant.value.surface.is_none() {
                    println!(
                        "     NO SURFACE RECORDED: nothing can say whether this warrant is \
                         spent, so it can never be shown to still hold (VDS S-6(4))."
                    );
                    problems += 1;
                }

                for entry in &warrant.value.evidence {
                    match store.read_proof(&entry.proof_id) {
                        Err(_) => {
                            println!("     EVIDENCE MISSING: {} is not on disk", entry.proof_id);
                            problems += 1;
                        }
                        Ok(proof) => {
                            if proof.value.digest != entry.digest {
                                println!(
                                    "     EVIDENCE DIGEST MISMATCH: {} cites {} and the record \
                                     holds {}",
                                    entry.proof_id, entry.digest, proof.value.digest
                                );
                                problems += 1;
                            }
                            let defects = vds_proof::verify_record(&proof.value)?;
                            for defect in defects {
                                println!("     EVIDENCE UNSOUND: {} {defect}", entry.proof_id);
                                problems += 1;
                            }
                        }
                    }
                }
            }
        }

        for kind in stage.required_evidence() {
            match store.latest_citable_proof(*kind)? {
                Some(proof) => println!(
                    "   evidence {:22} {} {}",
                    kind.as_str(),
                    proof.value.id,
                    proof.value.digest
                ),
                None => {
                    let note = if kind.is_implemented() {
                        "no citable proof on disk"
                    } else {
                        "no citable proof on disk, and this kind is NOT IMPLEMENTED"
                    };
                    println!("   evidence {:22} {note}", kind.as_str());
                }
            }
        }
        if stage == Stage::W3PrincipalAccepted {
            println!("   evidence: an acceptance event, which no proof can substitute for");
        }
        if !held.iter().any(|w| w.value.status == WarrantStatus::Granted) {
            println!("   -> not granted");
        }
        println!();
    }

    let proofs = store.read_proofs()?.len();
    let granted = warrants
        .iter()
        .filter(|w| w.value.status == WarrantStatus::Granted)
        .count();
    println!(
        "{proofs} proof records against {granted} granted warrants (docs/GOAL.md D9: the proof \
         surface is the one that rots)."
    );

    Ok(if problems > 0 { EXIT_VIOLATION } else { PASSED })
}

// --------------------------------------------------------------------- record

#[derive(ClapArgs)]
pub struct RecordArgs {
    /// `W1`, `W2`, `W3` or `W4`.
    #[arg(long)]
    stage: String,
    #[arg(long)]
    issue: String,
    #[arg(long)]
    holding: String,
    #[arg(long)]
    runtime_summary: String,
    /// A proof id to cite, repeatable. Its digest is taken from the record on
    /// disk and never from the caller.
    #[arg(long)]
    evidence: Vec<String>,
    /// The VJS order that granted it. Required for W1, W2 and W4.
    #[arg(long)]
    grantor_citation: Option<String>,
    /// A member of the bench, repeatable. Required for W1, W2 and W4.
    #[arg(long)]
    bench: Vec<String>,
    #[arg(long, default_value = "sovereign_assent")]
    assent_source: String,
    /// The Principal's acceptance event. Required for W3, and forbidden
    /// elsewhere.
    #[arg(long)]
    acceptance_event: Option<String>,
    #[arg(long)]
    accepted_by: Option<String>,
    #[arg(long)]
    accepted_at: Option<String>,
    /// The case file the grant was made on. Its digest is repeated verbatim
    /// from the convening record (VDS S-10(5)).
    #[arg(long, conflicts_with = "case_file_digest")]
    case_file: Option<String>,
    #[arg(long)]
    case_file_digest: Option<String>,
    #[arg(long)]
    granted_at: Option<String>,
    #[arg(long, default_value = "granted")]
    status: String,
    #[arg(long)]
    forbidden: Vec<String>,
    #[arg(long)]
    supersedes: Vec<String>,
    #[arg(long)]
    reserved: Vec<String>,
    /// Record a warrant whose predecessor stage is not granted.
    ///
    /// There is no such flag. The ordering is not a preference.
    #[arg(long, hide = true, default_value_t = false)]
    #[allow(dead_code)]
    ignore_stage_order: bool,
}

/// VDS S-6(2): a stage may not be entered before the preceding warrant is
/// granted, and "the ordering is the entire mechanism".
fn check_stage_order(store: &Store, stage: Stage) -> Result<()> {
    let Some(predecessor) = stage.predecessor() else {
        return Ok(());
    };
    match store.granted_warrant(predecessor)? {
        Some(warrant) => {
            let live = live_surface(store.project)?;
            if warrant.value.is_spent_by(&live) {
                return Err(VdsError::precondition(format!(
                    "{} is granted but SPENT: the surface has changed since it was granted, so \
                     {} may not be entered on it (VDS S-6(4)).\n  \
                     Re-run the proofs, have {} re-granted over the current surface, and record \
                     the spend: vds warrant spend {}",
                    predecessor.short(),
                    stage.short(),
                    predecessor.short(),
                    warrant.value.id
                )));
            }
            Ok(())
        }
        None => Err(VdsError::precondition(format!(
            "{} cannot be recorded: {} is not granted.\n  \
             VDS S-6(2): a stage may not be entered before the preceding warrant is granted, \
             and the ordering is the entire mechanism. Every drift defect measured in the \
             motivating project was authored before anyone asked whether the thing being used \
             was registered.\n  \
             Run `vds warrant status` to see where the chain stops.",
            stage.short(),
            predecessor.short()
        ))),
    }
}

fn record(store: &Store, args: &RecordArgs) -> Result<i32> {
    let stage = Stage::parse(&args.stage).ok_or_else(|| {
        VdsError::precondition(format!(
            "--stage {:?} is not a stage. The four are W1, W2, W3, W4.",
            args.stage
        ))
    })?;
    let status = parse_warrant_status(&args.status)?;

    // Only a GRANT enters a stage. Recording a refusal is a record of the
    // chain not advancing, and refusing to record one would erase the refusal.
    if status == WarrantStatus::Granted {
        check_stage_order(store, stage)?;
    }

    let mut evidence = Vec::new();
    for raw in &args.evidence {
        let proof_id = ProofId::parse(raw)?;
        let proof = store.read_proof(&proof_id)?;
        let defects = vds_proof::verify_record(&proof.value)?;
        if !defects.is_empty() {
            return Err(VdsError::precondition(format!(
                "{proof_id} may not be cited as evidence:\n  {}",
                defects.join("\n  ")
            )));
        }
        evidence.push(EvidenceEntry {
            proof_id,
            kind: proof.value.kind,
            // Taken from the record on disk, never from the caller: a warrant
            // that cites a digest the caller supplied proves the caller.
            digest: proof.value.digest.clone(),
            status: ProofStatus::Passed,
        });
    }

    let required = stage.required_evidence();
    let missing: Vec<&str> = required
        .iter()
        .filter(|k| !evidence.iter().any(|e| &e.kind == *k))
        .map(|k| k.as_str())
        .collect();
    if !missing.is_empty() {
        return Err(VdsError::precondition(format!(
            "{} requires evidence of kind {} (VDS S-6(2)); missing: {}",
            stage.short(),
            required.iter().map(|k| k.as_str()).collect::<Vec<_>>().join(", "),
            missing.join(", ")
        )));
    }

    let case_file_digest = match (&args.case_file, &args.case_file_digest) {
        (Some(path), _) => {
            let path = std::path::Path::new(path);
            if !path.is_file() {
                return Err(VdsError::precondition(format!(
                    "--case-file {} does not exist",
                    path.display()
                )));
            }
            Digest::of_file(path)?
        }
        (None, Some(raw)) => {
            let digest: Digest = serde_json::from_value(serde_json::Value::String(raw.clone()))
                .map_err(|_| VdsError::precondition(format!("{raw:?} is not a sha256 digest")))?;
            if !digest.is_well_formed() {
                return Err(VdsError::precondition(format!(
                    "{raw:?} is not a sha256:<64 hex> digest"
                )));
            }
            digest
        }
        (None, None) => {
            return Err(VdsError::precondition(
                "pass --case-file PATH or --case-file-digest. A warrant repeats the convening \
                 record's case_file_digest verbatim, so what was decided on is provable after \
                 the fact (VDS S-10(5)).",
            ));
        }
    };

    let live = live_surface(store.project)?;
    let (granted_by, assent_source, acceptance_event, bench, citation) = if stage
        == Stage::W3PrincipalAccepted
    {
        let Some(path) = &args.acceptance_event else {
            return Err(VdsError::precondition(
                "W3 needs --acceptance-event PATH. Acceptance is reserved to the Sovereign \
                 under ACT-001:s2: no proof substitutes for it, no bench may grant it, and VDS \
                 may never infer it from silence (VDS S-6(7)).",
            ));
        };
        let path = std::path::Path::new(path);
        if !path.is_file() {
            return Err(VdsError::precondition(format!(
                "--acceptance-event {} does not exist",
                path.display()
            )));
        }
        let accepted_at = match &args.accepted_at {
            Some(raw) => Timestamp::parse(raw)?,
            None => Timestamp::now(),
        };
        (
            GrantedBy::Principal,
            AssentSource::PrincipalAcceptance,
            Some(AcceptanceEvent {
                path: store.project.rel(path),
                digest: Digest::of_file(path)?,
                accepted_at,
                accepted_by: args
                    .accepted_by
                    .clone()
                    .unwrap_or_else(|| "the Principal".to_owned()),
                surface_digest: Digest::of_value(&live)?,
            }),
            Vec::new(),
            None,
        )
    } else {
        let Some(citation) = &args.grantor_citation else {
            return Err(VdsError::precondition(format!(
                "{} needs --grantor-citation, the VJS order that granted it. VDS grants nothing \
                 and may not grant itself a warrant (VDS S-1(3)). This command RECORDS a grant \
                 that already happened; it does not make one.",
                stage.short()
            )));
        };
        if args.bench.is_empty() {
            return Err(VdsError::precondition(format!(
                "{} needs --bench at least once. A warrant names its bench, or nobody can be \
                 asked what they decided.",
                stage.short()
            )));
        }
        if args.acceptance_event.is_some() {
            return Err(VdsError::precondition(format!(
                "--acceptance-event belongs to W3 alone. {} is granted by a bench, and an \
                 acceptance event on it would claim the Principal accepted something they \
                 were never shown (VDS S-6(7)).",
                stage.short()
            )));
        }
        (
            GrantedBy::VjsCourt,
            parse_assent_source(&args.assent_source)?,
            None,
            args.bench.clone(),
            Some(citation.clone()),
        )
    };

    let mut reserved = args.reserved.clone();
    if matches!(stage, Stage::W1RegisterComplete | Stage::W2DesignComplete) {
        reserved.push(
            "VDS S-9(10) RESERVED (SUBMISSION-VDS-005): the composition and \
             register_completeness proofs treat bare HTML elements as informational rows, so \
             this warrant does not reach the primitive layer."
                .to_owned(),
        );
    }
    if stage == Stage::W2DesignComplete {
        reserved.push(
            "VDS S-6(6) RESERVED (SUBMISSION-VDS-002): who may grant W2 is unsettled. Until \
             answered W2 is referred to VJS like W1 and W4, and a proof-only candidate may be \
             recorded but never treated as granted."
                .to_owned(),
        );
    }

    let id = WarrantId::allocate(&store.warrants_dir(), stage.number())?;
    let now = Timestamp::now();
    let warrant = Warrant {
        id: id.clone(),
        stage,
        project: store.project.config.jurisdiction_id.clone(),
        status,
        issue: args.issue.clone(),
        holding: args.holding.clone(),
        granted_by,
        grantor_citation: citation,
        assent_source,
        acceptance_event,
        evidence,
        case_file_digest,
        directives: vec![],
        forbidden: args.forbidden.clone(),
        exceptions: None,
        supersedes: args
            .supersedes
            .iter()
            .map(WarrantId::parse)
            .collect::<Result<_>>()?,
        unlocks: vec![stage.unlocks().to_owned()],
        surface: Some(live),
        runtime_summary: args.runtime_summary.clone(),
        created_at: now.clone(),
        granted_at: Some(match &args.granted_at {
            Some(raw) => Timestamp::parse(raw)?,
            None => now,
        }),
        bench,
        vote: None,
        source_opinion: None,
        appealable: true,
        reserved,
    };

    if let Some(reason) = warrant.void_on_its_face() {
        return Err(VdsError::precondition(format!(
            "refusing to write a warrant that is void on its face: {reason}"
        )));
    }

    let path = store.warrant_path(&id);
    store.create(&path, &warrant)?;
    println!("recorded {id} at {}", store.project.rel(&path));
    println!();
    for line in RECORDING_IS_NOT_GRANTING.split(". ") {
        println!("{}", line.trim_end_matches('.').to_owned() + ".");
    }
    Ok(PASSED)
}

fn spend(store: &Store, id: &str) -> Result<i32> {
    let id = WarrantId::parse(id)?;
    let located = store.read_warrant(&id)?;
    if located.value.status != WarrantStatus::Granted {
        return Err(VdsError::precondition(format!(
            "{id} has status {} and only a granted warrant can be spent.",
            located.value.status
        )));
    }
    let mut warrant = located.value;
    warrant.status = WarrantStatus::Spent;
    store.replace(&located.path, &warrant)?;
    println!("{id} marked spent. The record is never deleted (VDS S-6(4)).");
    Ok(PASSED)
}

fn parse_warrant_status(raw: &str) -> Result<WarrantStatus> {
    Ok(match raw {
        "granted" => WarrantStatus::Granted,
        "refused" => WarrantStatus::Refused,
        "spent" => WarrantStatus::Spent,
        "superseded" => WarrantStatus::Superseded,
        "revoked" => WarrantStatus::Revoked,
        other => {
            return Err(VdsError::precondition(format!(
                "{other:?} is not a warrant status. The five are: granted, refused, spent, \
                 superseded, revoked"
            )));
        }
    })
}

fn parse_assent_source(raw: &str) -> Result<AssentSource> {
    Ok(match raw {
        "sovereign_assent" => AssentSource::SovereignAssent,
        "standing_bounded_assent" => AssentSource::StandingBoundedAssent,
        "principal_acceptance" => AssentSource::PrincipalAcceptance,
        other => {
            return Err(VdsError::precondition(format!(
                "{other:?} is not an assent source. The three are: sovereign_assent, \
                 standing_bounded_assent, principal_acceptance"
            )));
        }
    })
}
