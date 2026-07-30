//! `vds screen`: register or amend a screen's ARRANGEMENT requirement.
//!
//! The front door to the ninth artefact kind (VDS S-4(1)). It is a convenience
//! door and not the wall: `screen_parity` runs whether or not this was used, and
//! "the author used the tool" is never proof of conformance (VDS S-11(5)).
//!
//! Two refusals happen HERE as well as at the wall, and the asymmetry is
//! deliberate rather than an oversight to be reconciled quietly. This door asks
//! whether the author typed something a screen could have; the proof asks
//! whether the row can fail. Where they differ the wall decides, and a floor
//! this door lets through and the proof refuses is a finding, not a bypass. See
//! `crates/vds-proof/src/contrast.rs:155`, which settles the same question the
//! same way for a contrast floor.
//!
//! This command reads and writes no design value (VDS S-2(2)). A column COUNT
//! is a requirement's shape; a column WIDTH is the design's own answer and has
//! nowhere to live.

use clap::{Args as ClapArgs, Subcommand};
use vds_core::{
    ArrangementContract, EXIT_VIOLATION, FigmaFrame, Result, ScreenId, ScreenRecord, Status,
    Timestamp, VdsError,
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
    /// Register a screen and the arrangement it requires.
    Add(AddArgs),
    /// Every registered screen, and what each requires.
    List,
}

#[derive(ClapArgs)]
pub struct AddArgs {
    /// What the screen IS, in this project's vocabulary: a route, a path, a
    /// name. Printed in every finding, because a finding that names only
    /// `SCR-0007` makes the reader go looking.
    #[arg(long)]
    route: String,
    /// How many side-by-side content PANES this screen requires.
    ///
    /// A count and never a width. A surface with no split requires 1, not 0:
    /// a screen with no split still has a content pane, and 0 is a requirement
    /// nothing can fail, which the proof refuses.
    #[arg(long)]
    columns: u32,
    /// The named regions this screen requires, in this project's vocabulary.
    ///
    /// Checked against what the frame generator found, and the generator is
    /// told which names to look for by `[screens] region_names`, so both halves
    /// read one list.
    #[arg(long, value_delimiter = ',')]
    regions: Vec<String>,
    /// The decided-target file holding the frame that draws this screen.
    #[arg(long)]
    file_key: Option<String>,
    /// The node id of that frame. Either of Figma's two spellings.
    #[arg(long)]
    node_id: Option<String>,
    /// The lifecycle status to register at.
    #[arg(long, default_value = "registered")]
    status: String,
    /// The authorities this registration rests on.
    #[arg(long, value_delimiter = ',', default_value = "ACT-VDS-001:s5a")]
    basis: Vec<String>,
}

pub fn run(ctx: &Context, args: &Args) -> Result<i32> {
    let project = ctx.project()?;
    let store = Store::new(&project);
    match &args.action {
        Action::Add(a) => add(&store, a),
        Action::List => list(&store),
    }
}

fn add(store: &Store, args: &AddArgs) -> Result<i32> {
    let status = Status::parse(&args.status).ok_or_else(|| {
        VdsError::precondition(format!(
            "{:?} is not one of the seven lifecycle statuses (VDS S-5(4)): {}",
            args.status,
            Status::PATH
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<&str>>()
                .join(", ")
        ))
    })?;

    let arrangement = ArrangementContract {
        columns: args.columns,
        regions: args.regions.clone(),
    };
    // Refused at the DOOR as well as at the wall. A record written here and
    // refused by every run of the proof is a record whose author was told at
    // the wrong end of the loop; the refusal names what to write instead,
    // because a refusal that does not is just a wall.
    if let Some(why) = arrangement.unenforceable_because() {
        return Err(VdsError::precondition(format!(
            "this screen would state a requirement no measurement could settle.\n  {why}"
        )));
    }

    // Both or neither. A node id with no file key names a node in a file
    // nobody stated, and three files were in play in the subject this was
    // derived from, where a bare node id resolved against the wrong one and
    // read as a deleted frame.
    let frame = match (&args.file_key, &args.node_id) {
        (Some(file_key), Some(node_id)) => Some(FigmaFrame {
            file_key: file_key.clone(),
            node_id: node_id.clone(),
            captured_at: Timestamp::now(),
        }),
        (None, None) => None,
        _ => {
            return Err(VdsError::precondition(
                "--file-key and --node-id go together. A node id with no file key names a node \
                 in a file nobody stated, and a bare node id resolved against the wrong file \
                 returns \"not found\", which reads as a deleted frame and is really a \
                 wrong-file error.",
            ));
        }
    };
    if frame.is_none() && status.is_enforceable() {
        return Err(VdsError::precondition(format!(
            "a screen at status {status} states a binding contract (VDS S-5(4)) and this one \
             names no frame, so `screen_parity` would report it as a requirement measured \
             against nothing.\n  Pass --file-key and --node-id, or register at --status \
             proposed until the screen is drawn."
        )));
    }

    let id = ScreenId::allocate(&store.screens_dir())?;
    let record = ScreenRecord {
        id: id.clone(),
        route: args.route.clone(),
        status,
        contract_version: 1,
        frame,
        arrangement,
        basis: args.basis.clone(),
        notes: None,
    };
    let path = store.screen_path(&id);
    store.create(&path, &record)?;

    println!("registered {id} {:?}", record.route);
    println!("  path:    {}", store.project.rel(&path));
    println!("  status:  {}", record.status);
    println!(
        "  columns: {}  (a COUNT of content panes; a width is a realisation and has no field \
         here, VDS S-2(4))",
        record.arrangement.columns
    );
    if !record.arrangement.regions.is_empty() {
        println!("  regions: {}", record.arrangement.regions.join(", "));
    }
    match &record.frame {
        Some(frame) => println!("  frame:   {} in file {}", frame.node_id, frame.file_key),
        None => println!(
            "  frame:   none. Nothing measures this screen until one is named and captured."
        ),
    }
    Ok(PASSED)
}

fn list(store: &Store) -> Result<i32> {
    let records = store.read_screens()?;
    if records.is_empty() {
        println!("no screen is registered.");
        println!(
            "  `screen_parity` is the only proof kind whose subject is a screen, and with an \
             empty screen register its runs are VACUOUS and prove nothing (VDS S-7(2)(4))."
        );
        return Ok(PASSED);
    }

    println!("{} registered screen(s):", records.len());
    let mut unmeasurable = 0;
    for located in &records {
        let record = &located.value;
        println!(
            "  {:10} {:32} {} column(s) {:10} {}",
            record.id.as_str(),
            record.route,
            record.arrangement.columns,
            record.status.as_str(),
            match &record.frame {
                Some(frame) => frame.node_id.clone(),
                None => "no frame".to_owned(),
            }
        );
        if record.status.is_enforceable() && record.frame.is_none() {
            unmeasurable += 1;
        }
    }
    if unmeasurable > 0 {
        println!();
        println!(
            "{unmeasurable} of them are in a binding status and name no frame, so their \
             requirement is measured against nothing and `screen_parity` reports each as a \
             finding rather than scoring it clean."
        );
        return Ok(EXIT_VIOLATION);
    }
    Ok(PASSED)
}
