//! `vds prohibition`: register a pattern as ABSENT from an enumerated scope.
//!
//! The front door for the draft S-7B artefact. A convenience door and not the
//! wall (VDS S-11(5)): the `prohibition` proof runs whether or not this was
//! used. What the door owns is the RECORDED EXPANSION: `add` expands the scope
//! at registration and writes the file list into the record, which is the
//! baseline the proof's anti-narrowing rule measures against. `re-expand`
//! refreshes it deliberately, through the same door, so a scope change is an
//! auditable amendment rather than a silent shrink.

use clap::{Args as ClapArgs, Subcommand};
use vds_core::{ProhibitionId, ProhibitionRecord, Result, Status, Timestamp, VdsError};
use vds_store::Store;

use crate::{Context, PASSED};

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Register a prohibition, recording the scope's expansion as the baseline.
    Add(AddArgs),
    /// Re-expand an existing prohibition's scope, deliberately.
    ReExpand(ReExpandArgs),
    /// Every prohibition, its scope size, and its status.
    List,
}

#[derive(ClapArgs)]
pub struct AddArgs {
    /// The forbidden spelling, matched as a literal substring per line.
    #[arg(long)]
    pattern: String,
    /// The scope: an explicit path or glob, repeatable.
    #[arg(long = "scope", required = true)]
    scope: Vec<String>,
    /// Why this pattern is forbidden here, in one line.
    #[arg(long)]
    because: Option<String>,
    #[arg(long, default_value = "registered")]
    status: String,
    #[arg(long, value_delimiter = ',', default_value = "draft S-7B")]
    basis: Vec<String>,
}

#[derive(ClapArgs)]
pub struct ReExpandArgs {
    #[arg(long)]
    id: String,
    /// Why the scope legitimately changed.
    #[arg(long)]
    because: String,
}

pub fn run(ctx: &Context, args: &Args) -> Result<i32> {
    let project = ctx.project()?;
    let store = Store::new(&project);
    match &args.action {
        Action::Add(a) => add(&store, a),
        Action::ReExpand(a) => re_expand(&store, a),
        Action::List => list(&store),
    }
}

fn expand(store: &Store, scope: &[String]) -> Result<Vec<String>> {
    let mut expansion: Vec<String> = vds_scan::glob::match_globs(&store.project.root, scope)?
        .iter()
        .map(|p| store.project.rel(p))
        .collect();
    expansion.sort();
    expansion.dedup();
    Ok(expansion)
}

fn add(store: &Store, args: &AddArgs) -> Result<i32> {
    if args.pattern.trim().is_empty() {
        return Err(VdsError::precondition(
            "--pattern is empty or whitespace. A prohibition with an empty pattern matches \
             everything or nothing, and the proof refuses it; refusing it here saves writing \
             a record that can only fail.",
        ));
    }
    let status = Status::parse(&args.status).ok_or_else(|| {
        VdsError::precondition(format!("{:?} is not a lifecycle status", args.status))
    })?;
    let expansion = expand(store, &args.scope)?;
    if expansion.is_empty() {
        return Err(VdsError::precondition(format!(
            "the scope {:?} matches no file, so this prohibition would be a check that cannot \
             fail and the proof would refuse it (draft S-7B R4). Name a scope that exists.",
            args.scope
        )));
    }
    let id = ProhibitionId::allocate(&store.prohibitions_dir())?;
    let record = ProhibitionRecord {
        id: id.clone(),
        status,
        pattern: args.pattern.clone(),
        scope: args.scope.clone(),
        expansion,
        directed_at: Some(Timestamp::now()),
        because: args.because.clone(),
        basis: args.basis.clone(),
        notes: None,
    };
    let path = store.prohibition_path(&id);
    store.create(&path, &record)?;
    println!("registered {id} [{:?}]", record.pattern);
    println!("  path:      {}", store.project.rel(&path));
    println!("  scope:     {}", record.scope.join(", "));
    println!(
        "  expansion: {} file(s), RECORDED as the anti-narrowing baseline (draft S-7B R3)",
        record.expansion.len()
    );
    Ok(PASSED)
}

fn re_expand(store: &Store, args: &ReExpandArgs) -> Result<i32> {
    let id = ProhibitionId::parse(&args.id)?;
    let path = store.prohibition_path(&id);
    if !path.is_file() {
        return Err(VdsError::precondition(format!(
            "no prohibition at {}",
            store.project.rel(&path)
        )));
    }
    let mut record: ProhibitionRecord = store.read(&path)?;
    let before = record.expansion.len();
    record.expansion = expand(store, &record.scope)?;
    if record.expansion.is_empty() {
        return Err(VdsError::precondition(
            "the scope now matches no file. Re-expanding to nothing would leave a check that \
             cannot fail; deprecate the record instead if the scope is genuinely gone.",
        ));
    }
    record.notes = Some(match record.notes.take() {
        Some(existing) => format!(
            "{existing}\n\nRE-EXPANDED {} -> {} file(s): {}",
            before,
            record.expansion.len(),
            args.because
        ),
        None => format!(
            "RE-EXPANDED {} -> {} file(s): {}",
            before,
            record.expansion.len(),
            args.because
        ),
    });
    store.replace(&path, &record)?;
    println!(
        "{id} re-expanded: {before} -> {} file(s)",
        record.expansion.len()
    );
    println!("  because: {}", args.because);
    Ok(PASSED)
}

fn list(store: &Store) -> Result<i32> {
    let records = store.read_prohibitions()?;
    if records.is_empty() {
        println!("no prohibition is registered.");
        return Ok(PASSED);
    }
    println!("{} prohibition(s):", records.len());
    for located in &records {
        let r = &located.value;
        println!(
            "  {:10} {:12} {:40} scope {} file(s)",
            r.id.as_str(),
            r.status.as_str(),
            format!("{:?}", r.pattern),
            r.expansion.len()
        );
    }
    Ok(PASSED)
}
