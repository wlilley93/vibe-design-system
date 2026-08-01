//! `vds signoff`: record a frame sign-off, and `vds redraw`: route a deviation
//! back through the design.
//!
//! Draft S-7D, ENACTMENT PENDING. Two doors in one module because they are two
//! ends of one loop: taste is exercised ONCE, at frame sign-off; a deviation
//! found downstream comes back as a proposed redraw, and the redraw is
//! resolvable only by a new sign-off row whose hash covers the change.
//!
//! `signoff record` reads the frame's CURRENT content hash out of the frames
//! ledger rather than taking one on the command line. A hash an author types
//! is a hash an author can type wrongly, and the whole mechanism is that the
//! recorded hash IS the frame as the signer saw it. The signer's name, by
//! contrast, is taken as given: VDS records who signed and never decides who
//! MAY sign - that is the Principal's own act, and this door only writes it
//! down (the same posture as `vds warrant record`).

use clap::{Args as ClapArgs, Subcommand};
use vds_core::{
    RedrawId, RedrawRecord, RedrawStatus, Result, SignOff, SignoffId, Timestamp, VdsError,
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
    /// Record a sign-off at the frame's CURRENT content hash.
    Record(RecordArgs),
    /// Every sign-off, and whether each still matches its frame.
    List,
}

#[derive(ClapArgs)]
pub struct RecordArgs {
    #[arg(long)]
    file_key: String,
    /// The frame's node id, either Figma spelling.
    #[arg(long)]
    node_id: String,
    /// Who exercised the taste. Written down, never validated: granting is not
    /// this tool's to do.
    #[arg(long)]
    signed_by: String,
    #[arg(long)]
    notes: Option<String>,
}

pub fn run(ctx: &Context, args: &Args) -> Result<i32> {
    let project = ctx.project()?;
    let store = Store::new(&project);
    match &args.action {
        Action::Record(a) => record(&store, a),
        Action::List => list(&store),
    }
}

fn record(store: &Store, args: &RecordArgs) -> Result<i32> {
    let project = store.project;
    let ledger = vds_figma::frames::read(project)?.ok_or_else(|| {
        VdsError::precondition(
            "no frames ledger exists, so the frame's current content hash cannot be read and \
             there is nothing to sign. Capture the frame and run `vds figma frames --from \
             <capture>` first: a sign-off is a hash, not a mood.",
        )
    })?;
    if ledger.file_key != args.file_key {
        return Err(VdsError::precondition(format!(
            "the frames ledger was captured from file {:?}, not {:?}. Signing across files \
             would bind a hash from one file to a claim about another.",
            ledger.file_key, args.file_key
        )));
    }
    let row = ledger.row(&args.node_id).ok_or_else(|| {
        VdsError::precondition(format!(
            "the frames ledger carries no frame {}. Re-capture with it included; a sign-off \
             for a frame nothing measured would be authority over nothing.",
            args.node_id
        ))
    })?;
    let frame_digest = row.content_digest.clone().ok_or_else(|| {
        VdsError::precondition(
            "this frame row predates per-frame content digests, so the current hash is \
             unknown. Regenerate the frames ledger from its capture; signing without the \
             hash would be authority by trust, which draft S-7D refuses.",
        )
    })?;
    let id = SignoffId::allocate(&store.signoffs_dir())?;
    let record = SignOff {
        id: id.clone(),
        file_key: args.file_key.clone(),
        node_id: row.node_id.clone(),
        frame_digest,
        signed_by: args.signed_by.clone(),
        signed_at: Timestamp::now(),
        notes: args.notes.clone(),
    };
    let path = store.signoff_path(&id);
    store.create(&path, &record)?;
    println!(
        "recorded {id}: {} signed {}/{}",
        record.signed_by, record.file_key, record.node_id
    );
    println!("  hash: {}", record.frame_digest);
    println!();
    println!(
        "Authority holds while the frame's current hash equals this one, and not a moment \
         longer: the frame changing reverts it to UNSIGNED until re-signed (draft S-7D)."
    );
    Ok(PASSED)
}

fn list(store: &Store) -> Result<i32> {
    let records = store.read_signoffs()?;
    if records.is_empty() {
        println!("no sign-off is recorded. Every frame-bound verdict is no_authority.");
        return Ok(PASSED);
    }
    let frames = vds_figma::frames::read(store.project)?;
    println!("{} sign-off(s):", records.len());
    for located in &records {
        let s = &located.value;
        let standing = match frames
            .as_ref()
            .filter(|l| l.file_key == s.file_key)
            .and_then(|l| l.row(&s.node_id))
            .and_then(|r| r.content_digest.as_ref())
        {
            Some(current) if current == &s.frame_digest => "CURRENT",
            Some(_) => "STALE: the frame changed after sign-off",
            None => "UNKNOWN: the frame has no current hash in the ledger",
        };
        println!(
            "  {:10} {}/{}  by {:20} at {}  {}",
            s.id.as_str(),
            s.file_key,
            s.node_id,
            s.signed_by,
            s.signed_at.as_str(),
            standing
        );
    }
    Ok(PASSED)
}

// ------------------------------------------------------------------- redraws

#[derive(ClapArgs)]
pub struct RedrawArgs {
    #[command(subcommand)]
    action: RedrawAction,
}

#[derive(Subcommand)]
enum RedrawAction {
    /// Open a proposed redraw for a recorded deviation.
    Add(RedrawAddArgs),
    /// Move a redraw along its path: drawn, signed (needs --resolved-by), or
    /// withdrawn.
    SetStatus(RedrawSetStatusArgs),
    /// Every redraw and where it stands.
    List,
}

#[derive(ClapArgs)]
pub struct RedrawAddArgs {
    /// The deviation this resolves: the review record and the delta it named.
    #[arg(long)]
    deviation: String,
    /// The proposed design change, described.
    #[arg(long)]
    proposed: String,
    #[arg(long)]
    file_key: String,
    #[arg(long)]
    node_id: String,
}

#[derive(ClapArgs)]
pub struct RedrawSetStatusArgs {
    #[arg(long)]
    id: String,
    /// proposed | drawn | signed | withdrawn
    #[arg(long)]
    to: String,
    /// The sign-off row that covers the change. Required for `signed`.
    #[arg(long)]
    resolved_by: Option<String>,
}

pub fn run_redraw(ctx: &Context, args: &RedrawArgs) -> Result<i32> {
    let project = ctx.project()?;
    let store = Store::new(&project);
    match &args.action {
        RedrawAction::Add(a) => redraw_add(&store, a),
        RedrawAction::SetStatus(a) => redraw_set_status(&store, a),
        RedrawAction::List => redraw_list(&store),
    }
}

fn redraw_add(store: &Store, args: &RedrawAddArgs) -> Result<i32> {
    let id = RedrawId::allocate(&store.redraws_dir())?;
    let record = RedrawRecord {
        id: id.clone(),
        deviation: args.deviation.clone(),
        review_id: None,
        proposed: args.proposed.clone(),
        status: RedrawStatus::Proposed,
        file_key: args.file_key.clone(),
        node_id: args.node_id.clone(),
        resolved_by: None,
        opened_at: Timestamp::now(),
        basis: vec!["draft S-7D".into()],
        notes: None,
    };
    let path = store.redraw_path(&id);
    store.create(&path, &record)?;
    println!("opened {id} (proposed): {}", record.proposed);
    println!("  the band comes back through the design, never through an exception.");
    Ok(PASSED)
}

fn redraw_set_status(store: &Store, args: &RedrawSetStatusArgs) -> Result<i32> {
    let id = RedrawId::parse(&args.id)?;
    let path = store.redraw_path(&id);
    if !path.is_file() {
        return Err(VdsError::precondition(format!(
            "no redraw at {}",
            store.project.rel(&path)
        )));
    }
    let to = match args.to.as_str() {
        "proposed" => RedrawStatus::Proposed,
        "drawn" => RedrawStatus::Drawn,
        "signed" => RedrawStatus::Signed,
        "withdrawn" => RedrawStatus::Withdrawn,
        other => {
            return Err(VdsError::precondition(format!(
                "{other:?} is not one of proposed | drawn | signed | withdrawn. There is \
                 deliberately no `accepted`: an acceptance state is taste exercised \
                 downstream of sign-off, which draft S-7D forbids."
            )));
        }
    };
    let mut record: RedrawRecord = store.read(&path)?;
    if to == RedrawStatus::Signed {
        let Some(raw) = &args.resolved_by else {
            return Err(VdsError::precondition(
                "`signed` requires --resolved-by <SGN-nnnn>: a redraw is resolvable ONLY by a \
                 sign-off row whose hash covers the change. The word is not the row.",
            ));
        };
        let signoff_id = SignoffId::parse(raw)?;
        let signoffs = store.read_signoffs()?;
        let Some(signoff) = signoffs.iter().find(|s| s.value.id == signoff_id) else {
            return Err(VdsError::precondition(format!(
                "{signoff_id} does not exist in the sign-off register."
            )));
        };
        if signoff.value.file_key != record.file_key || signoff.value.node_id != record.node_id {
            return Err(VdsError::precondition(format!(
                "{signoff_id} signs {}/{}, not this redraw's frame {}/{}.",
                signoff.value.file_key, signoff.value.node_id, record.file_key, record.node_id
            )));
        }
        record.resolved_by = Some(signoff_id);
    }
    record.status = to;
    store.replace(&path, &record)?;
    println!("{id} -> {}", record.status);
    if to == RedrawStatus::Signed {
        println!(
            "  the proof still verifies the covering hash against the frame's CURRENT one on \
             every run (draft S-7D R8); this door checked the row exists and names the frame."
        );
    }
    Ok(PASSED)
}

fn redraw_list(store: &Store) -> Result<i32> {
    let records = store.read_redraws()?;
    if records.is_empty() {
        println!("no redraw is open.");
        return Ok(PASSED);
    }
    println!("{} redraw(s):", records.len());
    for located in &records {
        let r = &located.value;
        println!(
            "  {:10} {:10} {}/{}  {}",
            r.id.as_str(),
            r.status.as_str(),
            r.file_key,
            r.node_id,
            r.proposed
        );
    }
    Ok(PASSED)
}
