//! `vds log`: the two governance logs, as commands rather than as advice.
//!
//! Both directories existed. Both were hand-written YAML with no type, no schema
//! and no command, and `vds doctor` counted the decision logs by listing a
//! directory without ever opening one.
//!
//! The sharper reason this exists: `vds lock repin` ends by printing
//!
//! > Self-file this under VDS S-12(3). A re-pin with a rationale and no breach
//! > report is a rationale nobody has to answer for.
//!
//! and there was no command to do it. That is the same defect adoption found in
//! `vds register import`, whose printed advice could not be followed either, and
//! it is worse here: the instruction is about the enforcement surface, so the one
//! place the tool asks to be held to account was the one place it made that
//! impossible.
//!
//! # Why filing is refused rather than warned about
//!
//! Both types carry `defects()`, and this command refuses to write a record that
//! has any. A governance log's failure mode is not being malformed, it is being
//! well-formed and empty of content: a `why` that says "for clarity" records a
//! fork nobody can reconstruct, and a breach citing no instrument is an apology.
//! Writing it with a warning would put it on disk, where `doctor` counts it, and
//! the count is what a reader takes for the state of the record.

use clap::{Args as ClapArgs, Subcommand};
use vds_core::{
    BreachId, BreachReport, DecisionId, DecisionLog, Result, SubmissionId, Timestamp, VdsError,
    actor,
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
    /// Record a fork disposed without a sitting (VDS S-12(2)).
    Decision(DecisionArgs),
    /// Self-report a breach (VDS S-12(3)). Restorative, never punitive.
    Breach(BreachArgs),
    /// Every log on disk, with any defect named.
    List,
}

#[derive(ClapArgs)]
pub struct DecisionArgs {
    /// What was decided, in one sentence.
    #[arg(long)]
    decision: String,
    /// Why the call was disposable without a sitting.
    ///
    /// This is the record that a fork was CONSIDERED, so it has to carry the
    /// argument a reviewer would disagree with, not a restatement of the
    /// decision.
    #[arg(long)]
    why: String,
    /// A clause or a ruling this rests on. Repeatable, and at least one.
    #[arg(long)]
    basis: Vec<String>,
    /// The fork DID need the court, and this submission carries it.
    #[arg(long)]
    submission_id: Option<String>,
    /// For a decision about re-pinning a gate: the digest the re-pin superseded.
    #[arg(long)]
    supersedes_digest: Option<String>,
}

#[derive(ClapArgs)]
pub struct BreachArgs {
    /// What happened, in enough detail to be checked by somebody who was not
    /// there.
    #[arg(long)]
    what_happened: String,
    /// The instrument fallen below. Repeatable, and at least one: a breach of
    /// nothing in particular is an apology rather than a record (VDS S-12(3)).
    #[arg(long)]
    law_breached: Vec<String>,
    /// How it was found. This decides whether the same class gets found again.
    #[arg(long)]
    discovered_by: String,
    /// What stopped it getting worse, before the remedy.
    #[arg(long)]
    containment: String,
    /// What was done to make the work good. Repeatable, and at least one.
    #[arg(long)]
    remedy: Vec<String>,
    /// What stops it recurring, or an honest statement that nothing does.
    #[arg(long)]
    prevention: Option<String>,
    /// The decision log this breach arose from, where one exists.
    #[arg(long)]
    decision_log_id: Option<String>,
}

pub fn run(ctx: &Context, args: &Args) -> Result<i32> {
    let project = ctx.project()?;
    let store = Store::new(&project);
    match &args.action {
        Action::Decision(a) => decision(&store, a),
        Action::Breach(a) => breach(&store, a),
        Action::List => list(&store),
    }
}

fn decision(store: &Store, args: &DecisionArgs) -> Result<i32> {
    let dir = store.decisions_dir();
    std::fs::create_dir_all(&dir).map_err(|e| VdsError::io(store.project.rel(&dir), e))?;

    let submission_id = args
        .submission_id
        .as_deref()
        .map(SubmissionId::parse)
        .transpose()?;
    let record = DecisionLog {
        id: DecisionId::parse(next_id(&dir, "DECISION")?)?,
        at: Timestamp::now(),
        by: actor(),
        decision: args.decision.clone(),
        court_required: submission_id.is_some(),
        why: args.why.clone(),
        basis: args.basis.clone(),
        submission_id,
        supersedes_digest: args
            .supersedes_digest
            .as_deref()
            .map(vds_core::Digest::parse)
            .transpose()?,
    };
    refuse_if_defective(&record.defects(), "decision log")?;

    let path = dir.join(format!("{}.yaml", record.id));
    store.create(&path, &record)?;
    println!("wrote {}", store.project.rel(&path));
    if record.court_required {
        println!(
            "  court_required is true because a submission was named. VDS S-12(2) makes the log \
             the ALTERNATIVE to a referral, so this one records that a referral happened rather \
             than that one was avoided."
        );
    } else {
        println!(
            "  court_required is false. That is a CLAIM: the call was reversible and its blast \
             radius low. `why` is the argument for it, and a reviewer who disagrees has \
             something concrete to disagree with."
        );
    }
    Ok(PASSED)
}

fn breach(store: &Store, args: &BreachArgs) -> Result<i32> {
    let dir = store.breaches_dir();
    std::fs::create_dir_all(&dir).map_err(|e| VdsError::io(store.project.rel(&dir), e))?;

    let record = BreachReport {
        id: BreachId::parse(next_id(&dir, "BREACH")?)?,
        at: Timestamp::now(),
        by: actor(),
        what_happened: args.what_happened.clone(),
        law_breached: args.law_breached.clone(),
        discovered_by: args.discovered_by.clone(),
        containment: args.containment.clone(),
        remedy: args.remedy.clone(),
        prevention: args.prevention.clone(),
        decision_log_id: args
            .decision_log_id
            .as_deref()
            .map(DecisionId::parse)
            .transpose()?,
    };
    refuse_if_defective(&record.defects(), "breach report")?;

    let path = dir.join(format!("{}.yaml", record.id));
    store.create(&path, &record)?;
    println!("wrote {}", store.project.rel(&path));
    println!(
        "  Filed, not charged. VDS S-12(3): remedy is restorative, the work is made good and \
         the lawful route resumed. There is no field here for blame, and that is deliberate: a \
         system that punishes self-reporting stops receiving self-reports."
    );
    Ok(PASSED)
}

fn list(store: &Store) -> Result<i32> {
    let decisions = store.read_decisions()?;
    let breaches = store.read_breaches()?;
    let mut defective = 0usize;

    println!(
        "{} decision logs in {}",
        decisions.len(),
        store.project.rel(&store.decisions_dir())
    );
    for located in &decisions {
        let record = &located.value;
        println!(
            "  {}  {}  court_required={}",
            record.id, record.at, record.court_required
        );
        println!("    {}", record.decision);
        for defect in record.defects() {
            defective += 1;
            println!("    DEFECTIVE: {defect}");
        }
    }

    println!();
    println!(
        "{} breach reports in {}",
        breaches.len(),
        store.project.rel(&store.breaches_dir())
    );
    for located in &breaches {
        let record = &located.value;
        println!("  {}  {}", record.id, record.at);
        println!("    {}", record.what_happened.lines().next().unwrap_or(""));
        println!("    law breached: {}", record.law_breached.join("; "));
        for defect in record.defects() {
            defective += 1;
            println!("    DEFECTIVE: {defect}");
        }
    }

    if breaches.is_empty() {
        println!();
        println!(
            "NOTE: VDS S-12(4) makes the two defects at S-1(4) the FOUNDING breach entries, \
             filed as breaches rather than described as background, because a system whose \
             first act is to excuse the failures that motivated it has taught itself the wrong \
             lesson. An empty breach directory in the VDS jurisdiction is that clause unmet."
        );
    }

    if defective > 0 {
        println!();
        println!("{defective} defect(s) across the logs above.");
        return Ok(vds_core::EXIT_VIOLATION);
    }
    Ok(PASSED)
}

fn refuse_if_defective(defects: &[String], what: &str) -> Result<()> {
    if defects.is_empty() {
        return Ok(());
    }
    Err(VdsError::precondition(format!(
        "this {what} would be filed with {} defect(s), so nothing was written:\n  {}\n  A \
         governance log's failure mode is being well-formed and EMPTY OF CONTENT, not being \
         malformed. Writing it with a warning would put it on disk where `vds doctor` counts \
         it, and the count is what a reader takes for the state of the record.",
        defects.len(),
        defects.join("\n  ")
    )))
}

/// The next identifier, read from the live directory rather than a counter.
///
/// VDS S-4(4): a counter and a directory disagree the first time a write fails
/// between them, and the directory is the one that is true.
fn next_id(dir: &std::path::Path, prefix: &str) -> Result<String> {
    let mut highest = 0u32;
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir).map_err(|e| VdsError::io(dir.display(), e))? {
            let entry = entry.map_err(|e| VdsError::io(dir.display(), e))?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            let Some(rest) = name.strip_prefix(&format!("{prefix}-")) else {
                continue;
            };
            if let Some(number) = rest.strip_suffix(".yaml")
                && let Ok(parsed) = number.parse::<u32>()
            {
                highest = highest.max(parsed);
            }
        }
    }
    Ok(format!("{prefix}-{:04}", highest + 1))
}
