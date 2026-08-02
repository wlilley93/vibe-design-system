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
    /// Register EVERY frame in the frame ledger that no screen record claims.
    Adopt(AdoptArgs),
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

/// Bulk adoption from the frame ledger.
///
/// # Why this exists, and why `add` alone was not enough
///
/// `add` registers ONE screen, by hand, with the route and column count typed
/// at the prompt. That is the right door for a screen somebody is deciding
/// about. It is the wrong door for adoption: the estate this was derived from
/// draws 192 screens, and a subscriber facing 192 invocations wrote zero. So
/// `screen_parity` ran for weeks reporting
///
///     rows_considered: 0
///     192 frame(s) in the capture are claimed by no screen record.
///     VACUOUS: this proof cannot currently fail.
///
/// while the same estate published parity numbers taken from its own private
/// instruments. A proof nobody can afford to populate is a proof that does not
/// run, and "the subscriber should have typed 192 commands" is a defect in the
/// tool and not in the subscriber.
///
/// # What it will not do
///
/// It will not invent authority. Every record lands at `proposed` unless the
/// caller passes `--status`, and the caller passing it is a human act. A frame
/// that DISCLAIMS ITSELF is skipped entirely rather than adopted at a lower
/// status, because a drawing that says it is not source-current states no
/// contract to register.
#[derive(ClapArgs)]
pub struct AdoptArgs {
    /// The lifecycle status to register at.
    ///
    /// `proposed` by default and deliberately. A bulk command that wrote an
    /// ENFORCEABLE status would let one invocation manufacture a contract for
    /// every screen in a file, which is the shape of act VDS S-5(4) makes a
    /// directed path precisely to prevent.
    #[arg(long, default_value = "proposed")]
    status: String,
    /// The authorities this registration rests on.
    #[arg(long, value_delimiter = ',', default_value = "ACT-VDS-001:s5a")]
    basis: Vec<String>,
    /// Report what would be written, and write nothing.
    #[arg(long)]
    dry_run: bool,
}

pub fn run(ctx: &Context, args: &Args) -> Result<i32> {
    let project = ctx.project()?;
    let store = Store::new(&project);
    match &args.action {
        Action::Add(a) => add(&store, a),
        Action::Adopt(a) => adopt(&project, &store, a),
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

/// Register every ledger frame that no screen record already claims.
///
/// ROUTE COMES FROM THE FRAME'S OWN NAME, and that is the one guess here. A
/// screen frame is named `Screen · /finance/invoices · list current` and the
/// route is the first middot-separated segment beginning with a slash. NOT the
/// second segment: several frames carry a leading qualifier, and taking
/// position rather than shape mislabels them. A frame whose name states no
/// route is REPORTED and skipped, never adopted under a guessed name, because a
/// record filed against the wrong route is worse than no record: it makes a
/// proof compare two screens that have nothing to do with each other.
fn adopt(project: &vds_core::Project, store: &Store, args: &AdoptArgs) -> Result<i32> {
    let status = Status::parse(&args.status).ok_or_else(|| {
        VdsError::precondition(format!(
            "{:?} is not one of the seven lifecycle statuses (VDS S-5(4))",
            args.status
        ))
    })?;

    let Some(ledger) = vds_figma::frames::read(project)? else {
        return Err(VdsError::precondition(
            "there is no frame ledger to adopt from.\n  Derive one first: vds figma frames              --from <saved nodes capture> --file-key <key>",
        ));
    };

    // A route already registered is left ALONE, so a re-run is safe and does
    // not renumber anything a reader may have cited.
    let existing: std::collections::BTreeSet<String> = store
        .read_screens()?
        .into_iter()
        .map(|l| l.value.route)
        .collect();

    let mut adopted = Vec::new();
    let mut skipped_disclaimed = 0usize;
    let mut skipped_claimed = 0usize;
    let mut no_route = Vec::new();

    for row in &ledger.frames {
        let Some(route) = route_in(&row.frame_name) else {
            no_route.push(row.frame_name.clone());
            continue;
        };
        if row.disclaimed {
            skipped_disclaimed += 1;
            continue;
        }
        if existing.contains(&route) {
            skipped_claimed += 1;
            continue;
        }
        adopted.push((route, row));
    }

    if args.dry_run {
        println!(
            "{} frame(s) would be adopted at status {status}.",
            adopted.len()
        );
    }
    for (route, row) in &adopted {
        let record = ScreenRecord {
            id: ScreenId::allocate(&store.screens_dir())?,
            route: route.clone(),
            status,
            contract_version: 1,
            frame: Some(FigmaFrame {
                file_key: ledger.file_key.clone(),
                node_id: row.node_id.clone(),
                captured_at: ledger.generated_at.clone(),
            }),
            arrangement: ArrangementContract {
                columns: row.columns,
                regions: row.regions.clone(),
            },
            basis: args.basis.clone(),
            // The reading's own caveats travel WITH the record. A truncated
            // capture's empty region list means we did not look, and a reader
            // holding the record without that sentence cannot tell it from a
            // frame that draws nothing.
            notes: Some(adoption_note(row)),
        };
        if args.dry_run {
            println!(
                "  {:38} {} column(s)  {}",
                record.route, record.arrangement.columns, row.node_id
            );
            continue;
        }
        let path = store.screen_path(&record.id);
        store.create(&path, &record)?;
        println!(
            "  {} {:38} {} column(s)  {}",
            record.id.as_str(),
            record.route,
            record.arrangement.columns,
            row.node_id
        );
    }

    println!();
    println!(
        "{} adopted at status {status}; {skipped_claimed} route(s) already registered and left \
         untouched.",
        adopted.len()
    );
    if skipped_disclaimed > 0 {
        println!(
            "{skipped_disclaimed} frame(s) DISCLAIM THEMSELVES and were not adopted: a drawing \
             whose own authority layer says it is not source-current states no contract to \
             register, and adopting one would file a requirement against a screen the designer \
             has already withdrawn."
        );
    }
    if !no_route.is_empty() {
        println!(
            "{} frame(s) name no route and were NOT adopted under a guessed one:",
            no_route.len()
        );
        for name in no_route.iter().take(10) {
            println!("    {name}");
        }
        if no_route.len() > 10 {
            println!("    ... and {} more", no_route.len() - 10);
        }
    }
    if !status.is_enforceable() {
        println!();
        println!(
            "NOTHING IS ENFORCEABLE YET. These sit at {status}, so `screen_parity` EXCLUDES \
             them and an exclusion is never a pass. Move a screen on when somebody has \
             ratified its drawing; a bulk command must not manufacture that act."
        );
    }
    Ok(PASSED)
}

/// The route a frame name states, or `None` where it states none.
fn route_in(frame_name: &str) -> Option<String> {
    frame_name
        .split('·')
        .map(str::trim)
        .find(|seg| seg.starts_with('/'))
        // Trailing prose after the path ("/finance list current") is not part
        // of it.
        .and_then(|seg| seg.split_whitespace().next())
        .map(str::to_owned)
}

/// What a reader of this record needs that the fields alone do not carry.
fn adoption_note(row: &vds_figma::frames::FrameRow) -> String {
    let mut note = format!(
        "Adopted from the frame ledger by `vds screen adopt`. Authority layer: {} (by {}).",
        row.authority_layer,
        row.authority_by.as_str()
    );
    if !row.quarantined.is_empty() {
        note.push_str(&format!(
            " Layers nobody may build from: {}.",
            row.quarantined.join(", ")
        ));
    }
    if row.truncated {
        note.push_str(
            " THIS FRAME'S CAPTURE IS TRUNCATED at the depth boundary, so an empty region list \
             here means WE DID NOT LOOK and not that the frame draws nothing. Recapture deeper \
             before relying on the arrangement below.",
        );
    }
    note
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

#[cfg(test)]
mod tests {
    use super::route_in;

    /// The one guess `adopt` makes, and the shape of it is the whole point.
    ///
    /// A route is the first middot segment beginning with a slash, NOT the
    /// second segment. Several frames in the subject carry a leading qualifier,
    /// and taking position rather than shape files a record against the wrong
    /// route, which makes a proof compare two screens that have nothing to do
    /// with each other. That is worse than no record.
    #[test]
    fn a_route_is_found_by_shape_and_not_by_position() {
        assert_eq!(
            route_in("Screen · /finance/invoices · list current").as_deref(),
            Some("/finance/invoices")
        );
        // Leading qualifier: the route is the THIRD segment here.
        assert_eq!(
            route_in("CURRENT SOURCE · Screen · /matters · board").as_deref(),
            Some("/matters")
        );
        // Trailing prose after the path is not part of it.
        assert_eq!(
            route_in("Screen · /finance list current").as_deref(),
            Some("/finance")
        );
        assert_eq!(route_in("Screen · /").as_deref(), Some("/"));
    }

    /// A frame naming no route is REPORTED and skipped, never adopted under a
    /// guessed name. `None` here is what makes that possible.
    #[test]
    fn a_frame_naming_no_route_yields_none() {
        assert_eq!(route_in("Components · Buttons"), None);
        assert_eq!(route_in("Frame"), None);
        assert_eq!(route_in(""), None);
    }
}
