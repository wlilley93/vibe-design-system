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
    // ORDER 31: only a CURRENT SOURCE frame is registrable. The ledger already
    // carries what this needs (`disclaimed`, `authority_by`) and the door never
    // looked. A register entry over a frame that says in its own authoritative
    // layer that it is NOT source-current is a signed contradiction, and the
    // register is the condition precedent to the whole regime: poisoning it at
    // genesis poisons everything downstream ([2026] VJS-SC-OPBOX 1 orders 23
    // and 25). Front door only - this creates no proof rule and reddens
    // nothing already recorded.
    if row.disclaimed {
        return Err(VdsError::precondition(format!(
            "{}/{} DISCLAIMS ITSELF: its authoritative layer {:?} says in its own name that \
             it is not source-current, or was never built. Such a frame states no contract, \
             so signing it would enter a contradiction as the register's own authority. \
             Resolve the label - redraw and re-capture - before signing.",
            args.file_key, row.node_id, row.authority_layer
        )));
    }
    if row.authority_by == vds_figma::frames::AuthorityBy::Unlabelled {
        return Err(VdsError::precondition(format!(
            "{}/{} carries NO AUTHORITY MARKER at all: {:?} was taken as the authority by \
             default, which is a default and not a declaration. Only frames labelled \
             CURRENT SOURCE are registrable ([2026] VJS-SC-OPBOX 1 order 25); if this file \
             uses different words for that, they belong in `[screens] authority_markers`.",
            args.file_key, row.node_id, row.authority_layer
        )));
    }
    if vds_figma::frames::authority_of(&row.authority_layer, &project.config.screens)
        != Some(vds_figma::frames::Authority::Current)
    {
        return Err(VdsError::precondition(format!(
            "{}/{} resolves its authority from {:?}, which is not a CURRENT SOURCE label \
             under `[screens] authority_markers`. LEGACY/REFERENCE and TARGET/proposal \
             frames are no_authority per se and are registrable only after redraw \
             ([2026] VJS-SC-OPBOX 1 order 25).",
            args.file_key, row.node_id, row.authority_layer
        )));
    }

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

// ---------------------------------------------------------------- directions

#[derive(ClapArgs)]
pub struct DirectionArgs {
    #[command(subcommand)]
    action: DirectionAction,
}

#[derive(Subcommand)]
enum DirectionAction {
    /// Register a Principal direction, hash-bound to its logged decision.
    Record(DirectionRecordArgs),
    /// Every direction, and whether its decision still digests to what was
    /// registered.
    List,
}

#[derive(ClapArgs)]
pub struct DirectionRecordArgs {
    /// The decision-log entry the direction was given in: a repository-relative
    /// path, or a decision id under `[paths] logs`.
    #[arg(long)]
    log_id: String,
    /// The route the direction touches, where it names no frame.
    #[arg(long, conflicts_with_all = ["file_key", "node_id"])]
    route: Option<String>,
    #[arg(long, requires = "node_id")]
    file_key: Option<String>,
    #[arg(long, requires = "file_key")]
    node_id: Option<String>,
    /// WHAT was directed, in the [2026] VJS-CC-OPBOX 155 O2 form.
    #[arg(long)]
    direction: String,
    /// HOW MUCH, in the same form. Preserved rather than summarised: it
    /// becomes the redraw brief.
    #[arg(long)]
    magnitude: String,
    #[arg(long)]
    notes: Option<String>,
}

pub fn run_direction(ctx: &Context, args: &DirectionArgs) -> Result<i32> {
    let project = ctx.project()?;
    let store = Store::new(&project);
    match &args.action {
        DirectionAction::Record(a) => direction_record(&store, a),
        DirectionAction::List => direction_list(&store),
    }
}

fn direction_record(store: &Store, args: &DirectionRecordArgs) -> Result<i32> {
    let project = store.project;
    // The digest is READ from the log entry, never taken from the caller: a
    // direction is bound to what was actually decided, and a hash somebody
    // types is a hash somebody can type wrongly.
    let decision_digest =
        vds_core::decision_log_digest(project, &args.log_id).ok_or_else(|| {
            VdsError::precondition(format!(
                "the decision log {:?} cannot be resolved, either as a repository-relative path \
             or as a decision record under [paths] logs. A direction is hash-bound to its \
             LOGGED DECISION ([2026] VJS-SC-OPBOX 1 order 30), and a direction with no \
             readable decision behind it is authority the instruments cannot read - which is \
             authority the estate does not have.",
                args.log_id
            ))
        })?;
    let surface = match (&args.route, &args.file_key, &args.node_id) {
        (Some(route), _, _) => vds_core::DirectedSurface::Route {
            route: route.clone(),
        },
        (None, Some(file_key), Some(node_id)) => vds_core::DirectedSurface::Frame {
            file_key: file_key.clone(),
            node_id: node_id.clone(),
        },
        _ => {
            return Err(VdsError::precondition(
                "pass --route, or --file-key with --node-id. A direction that names no \
                 surface disposes of nothing.",
            ));
        }
    };
    let id = vds_core::DirectionId::allocate(&store.directions_dir())?;
    let record = vds_core::DirectionRecord {
        id: id.clone(),
        log_id: args.log_id.clone(),
        decision_digest,
        surface,
        direction: args.direction.clone(),
        magnitude: args.magnitude.clone(),
        directed_at: Timestamp::now(),
        notes: args.notes.clone(),
    };
    let path = store.direction_path(&id);
    store.create(&path, &record)?;
    println!("recorded {id}: {}", record.surface.describe());
    println!("  direction: {}", record.direction);
    println!("  magnitude: {}", record.magnitude);
    println!(
        "  bound to:  {} at {}",
        record.log_id, record.decision_digest
    );
    println!();
    println!(
        "Authority holds while the logged decision still digests to that value: staleness by \
         hash, never by trust. A direction confers authority for its OWN TERMS only and \
         carries a live duty to redraw, so the frame record converges on the directed state."
    );
    Ok(PASSED)
}

fn direction_list(store: &Store) -> Result<i32> {
    let records = store.read_directions()?;
    if records.is_empty() {
        println!("no direction is registered.");
        println!(
            "  [2026] VJS-SC-OPBOX 1 order 31 makes the four directions of 2026-08-01 the \
             register's FOUNDING entries, backfilled before any frame-bound proof runs in \
             blocking mode."
        );
        return Ok(PASSED);
    }
    println!("{} direction(s):", records.len());
    for located in &records {
        let d = &located.value;
        let standing = match vds_core::decision_log_digest(store.project, &d.log_id) {
            Some(current) if current == d.decision_digest => "LIVE",
            Some(_) => "LAPSED: the logged decision changed after registration",
            None => "UNREADABLE: the logged decision cannot be resolved",
        };
        println!(
            "  {:10} {:28} {:24} {}",
            d.id.as_str(),
            d.surface.describe(),
            d.log_id,
            standing
        );
        println!("      {} / {}", d.direction, d.magnitude);
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
    /// proposed | drawn | signed | parked | withdrawn
    #[arg(long)]
    to: String,
    /// The sign-off row that covers the change. Required for `signed`.
    #[arg(long)]
    resolved_by: Option<String>,
    /// The direction row that parks it. Required for `parked`.
    #[arg(long)]
    directed_by: Option<String>,
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
        directed_by: None,
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
        "parked" => RedrawStatus::Parked,
        "withdrawn" => RedrawStatus::Withdrawn,
        other => {
            return Err(VdsError::precondition(format!(
                "{other:?} is not one of proposed | drawn | signed | parked | withdrawn. \
                 There is deliberately no `accepted`: an acceptance state is taste \
                 exercised downstream of sign-off, which S-7D(4) repeals. A direction the \
                 Principal gave is recorded as `parked` under a direction row, which is \
                 taste exercised AT the register."
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
    if to == RedrawStatus::Parked {
        let Some(raw) = &args.directed_by else {
            return Err(VdsError::precondition(
                "`parked` requires --directed-by <DIR-nnnn>: a park rests on a REGISTERED \
                 PRINCIPAL DIRECTION, and the word is not the row ([2026] VJS-CA-VDS 1 \
                 order 27).",
            ));
        };
        let direction_id = vds_core::DirectionId::parse(raw)?;
        let directions = store.read_directions()?;
        let Some(direction) = directions.iter().find(|d| d.value.id == direction_id) else {
            return Err(VdsError::precondition(format!(
                "{direction_id} does not exist in the direction register."
            )));
        };
        let current = vds_core::decision_log_digest(store.project, &direction.value.log_id);
        if !vds_core::direction_authority(&direction.value, current.as_ref()).is_signed() {
            return Err(VdsError::precondition(format!(
                "{direction_id} no longer carries authority: its logged decision does not \
                 digest to what was registered. Re-register the direction against the \
                 decision as it now stands."
            )));
        }
        record.directed_by = Some(direction_id);
    }
    record.status = to;
    store.replace(&path, &record)?;
    println!("{id} -> {}", record.status);
    if to == RedrawStatus::Signed {
        println!(
            "  the proof still verifies the covering hash against the frame's CURRENT one on \
             every run (S-7D R8); this door checked the row exists and names the frame."
        );
    }
    if to == RedrawStatus::Parked {
        println!(
            "  while the registered direction stands no gate may count this a violation, and \
             the subject keeps its render rights ([2026] VJS-SC-OPBOX 1 order 29). The \
             redraw DUTY stands: the frame record converges on the directed state."
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
