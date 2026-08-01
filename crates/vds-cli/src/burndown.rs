//! `vds burndown`: pin a metric and lower the pin.
//!
//! The front door for the draft S-7C artefact, and a convenience door only
//! (VDS S-11(5)). The verb is `pin` and it only lowers, for the reason
//! `vds geometry lower` refuses to raise: the instrument this kind replaces
//! was a family of bespoke ratchets whose ceilings never came down. A genuine
//! re-baseline is a NEW record with the reason on it, after `deprecate`.

use clap::{Args as ClapArgs, Subcommand};
use vds_core::{BurndownId, BurndownRecord, PinnedValue, Result, Status, Timestamp, VdsError};
use vds_store::Store;

use crate::{Context, PASSED};

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Register a metric with its measured BASELINE as the first pin.
    Add(AddArgs),
    /// Lower an existing pin onto a new, lower measured value.
    Pin(PinArgs),
    /// Deprecate a record, so a new baseline may be registered for its metric.
    Deprecate(DeprecateArgs),
    /// Every burndown, its pin, and its deadline.
    List,
}

#[derive(ClapArgs)]
pub struct AddArgs {
    /// The machine key the reading reports this metric under.
    #[arg(long)]
    metric: String,
    /// Today's MEASURED value, which becomes the first pin.
    #[arg(long)]
    value: u64,
    /// By when the metric must be zero, as an RFC 3339 UTC timestamp.
    #[arg(long)]
    deadline: Option<String>,
    /// Why this baseline, in one line.
    #[arg(long)]
    because: Option<String>,
    #[arg(long, default_value = "registered")]
    status: String,
    #[arg(long, value_delimiter = ',', default_value = "draft S-7C")]
    basis: Vec<String>,
}

#[derive(ClapArgs)]
pub struct PinArgs {
    #[arg(long)]
    id: String,
    /// The new, LOWER pin: the value the reading now measures.
    #[arg(long)]
    to: u64,
    /// What fell, in one line.
    #[arg(long)]
    because: String,
}

#[derive(ClapArgs)]
pub struct DeprecateArgs {
    #[arg(long)]
    id: String,
    #[arg(long)]
    because: String,
}

pub fn run(ctx: &Context, args: &Args) -> Result<i32> {
    let project = ctx.project()?;
    let store = Store::new(&project);
    match &args.action {
        Action::Add(a) => add(&store, a),
        Action::Pin(a) => pin(&store, a),
        Action::Deprecate(a) => deprecate(&store, a),
        Action::List => list(&store),
    }
}

fn add(store: &Store, args: &AddArgs) -> Result<i32> {
    let status = Status::parse(&args.status).ok_or_else(|| {
        VdsError::precondition(format!("{:?} is not a lifecycle status", args.status))
    })?;
    let existing = store.read_burndowns()?;
    if let Some(clash) = existing
        .iter()
        .find(|r| r.value.metric == args.metric && r.value.status.is_enforceable())
        && status.is_enforceable()
    {
        return Err(VdsError::precondition(format!(
            "{} already pins {:?}, and two pins for one metric means nothing says which \
             governs (draft S-7C R7). Lower it with `vds burndown pin --id {} --to <n>`, or \
             deprecate it first for a genuine re-baseline.",
            clash.value.id, args.metric, clash.value.id
        )));
    }
    let deadline = args.deadline.as_deref().map(Timestamp::parse).transpose()?;
    let id = BurndownId::allocate(&store.burndowns_dir())?;
    let record = BurndownRecord {
        id: id.clone(),
        status,
        metric: args.metric.clone(),
        deadline,
        history: vec![PinnedValue {
            at: Timestamp::now(),
            value: args.value,
            because: args.because.clone(),
        }],
        basis: args.basis.clone(),
        notes: None,
    };
    let path = store.burndown_path(&id);
    store.create(&path, &record)?;
    println!("registered {id} [{}]", record.metric);
    println!("  pin:      {} (the measured baseline)", args.value);
    if let Some(d) = &record.deadline {
        println!("  deadline: {}", d.as_str());
    }
    println!();
    println!(
        "The pin must SIT ON the measured value: the proof is red on any increase AND on a \
         decrease that was not re-pinned (draft S-7C R1, R2). Green means the pin is the truth."
    );
    Ok(PASSED)
}

fn pin(store: &Store, args: &PinArgs) -> Result<i32> {
    let id = BurndownId::parse(&args.id)?;
    let path = store.burndown_path(&id);
    if !path.is_file() {
        return Err(VdsError::precondition(format!(
            "no burndown at {}",
            store.project.rel(&path)
        )));
    }
    let mut record: BurndownRecord = store.read(&path)?;
    let Some(current) = record.current().cloned() else {
        return Err(VdsError::precondition(format!(
            "{id} has an empty history, so there is no pin to lower."
        )));
    };
    if args.to >= current.value {
        return Err(VdsError::precondition(format!(
            "{id} is pinned at {} and --to is {}, which is not lower.\n  There is no way to \
             raise a pin: a pin that goes up is not a pin (draft S-7C R4). If the population \
             genuinely grew, deprecate this record and register a new baseline with the \
             reason on it.",
            current.value, args.to
        )));
    }
    record.history.push(PinnedValue {
        at: Timestamp::now(),
        value: args.to,
        because: Some(args.because.clone()),
    });
    store.replace(&path, &record)?;
    println!(
        "{id} [{}] pinned {} -> {}",
        record.metric, current.value, args.to
    );
    println!("  because: {}", args.because);
    Ok(PASSED)
}

fn deprecate(store: &Store, args: &DeprecateArgs) -> Result<i32> {
    let id = BurndownId::parse(&args.id)?;
    let path = store.burndown_path(&id);
    if !path.is_file() {
        return Err(VdsError::precondition(format!(
            "no burndown at {}",
            store.project.rel(&path)
        )));
    }
    let mut record: BurndownRecord = store.read(&path)?;
    if record.status == Status::Deprecated {
        return Err(VdsError::precondition(format!(
            "{id} is already deprecated"
        )));
    }
    record.status = Status::Deprecated;
    record.notes = Some(match record.notes.take() {
        Some(existing) => format!("{existing}\n\nDEPRECATED: {}", args.because),
        None => format!("DEPRECATED: {}", args.because),
    });
    store.replace(&path, &record)?;
    println!(
        "{id} [{}] deprecated. The record is KEPT: its history is the evidence the number ever fell.",
        record.metric
    );
    Ok(PASSED)
}

fn list(store: &Store) -> Result<i32> {
    let records = store.read_burndowns()?;
    if records.is_empty() {
        println!("no burndown is registered.");
        return Ok(PASSED);
    }
    let reading = vds_core::read_burndown_reading(store.project)?;
    println!("{} burndown(s):", records.len());
    for located in &records {
        let r = &located.value;
        let pin = r
            .current()
            .map_or("none".to_owned(), |p| p.value.to_string());
        let measured = reading
            .as_ref()
            .and_then(|reading| reading.row(&r.metric))
            .map_or("not measured".to_owned(), |row| row.value.to_string());
        println!(
            "  {:10} {:12} {:32} pin {:>8}   measured {:>12}   deadline {}",
            r.id.as_str(),
            r.status.as_str(),
            r.metric,
            pin,
            measured,
            r.deadline.as_ref().map_or("none", |d| d.as_str())
        );
    }
    Ok(PASSED)
}
