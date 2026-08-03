//! `vds stage`: stage a write to a frame, review it, apply it, verify it.
//!
//! # What this door is, and the one thing it is not
//!
//! It is not a control that stops anyone writing to Figma directly, and no help
//! text, note or refusal in this file may say that it is. The REST API cannot
//! write document nodes, so VDS holds no privileged channel it could withhold;
//! VDS needs a token to READ and the plugin bridge needs none to WRITE, so
//! credential custody is the inverse of a control here; and the writer lock
//! `apply` takes says on its own face that it is ADVISORY and cannot stop a
//! writer that does not ask for it.
//!
//! What it is: a CLOSED operation vocabulary, an operation list on disk BEFORE
//! anything reaches the canvas, and a bypass rule that makes a write nobody
//! staged visible after the fact.
//!
//! # `add` RUNS NO GATE, and `plan` refuses to emit against a refusal
//!
//! The door is not the wall (VDS S-11(5); `crates/vds-cli/src/screen.rs` says
//! so on its own face). `add` records what somebody intends and the readings it
//! measured at that moment; `vds proof staged_write` re-derives every gate from
//! the intent rather than believing the record, so a record whose verdicts were
//! typed is a finding rather than a bypass. What `plan` will not do is EMIT
//! against a refusal: a plan is the thing an apply reads, and emitting one over
//! a refused gate would put the refusal downstream of the act it exists to stop.
//!
//! # `apply` is not a proof and can never be one
//!
//! It re-captures the live frame, which is a network act, and VDS S-7(2)(1)
//! forbids a network call inside a proof. So the four gates read local files
//! only and `apply` sits outside the proof path entirely.

use std::collections::BTreeMap;
use std::path::PathBuf;

use clap::{Args as ClapArgs, Subcommand};
use vds_core::{
    ApplyOutcome, Digest, EXIT_VIOLATION, GateReading, Result, StageId, StageInput, StageRecord,
    StageTarget, Timestamp, VdsError, Verification,
};
use vds_css::sheet::Sheet;
use vds_figma::stage::{DiffInputs, property_key};
use vds_proof::staged_write::{GateInputs, read_gates};
use vds_store::Store;

use crate::{Context, PASSED};

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Record a staged write and the gate readings its intent produces.
    ///
    /// RUNS NO GATE AS A CONDITION OF WRITING THE RECORD. It measures all four
    /// and records what they read, including a refusal, because a refusal that
    /// cannot be written down is a refusal nobody can look at. `plan` is the
    /// verb that refuses.
    Add(AddArgs),
    /// Emit the operation list to a file BEFORE anything reaches the canvas.
    ///
    /// Refuses to emit if any gate reads REFUSED.
    Plan(PlanArgs),
    /// Take the advisory writer lock and hand the plan's chunks over, in order.
    Apply(ApplyArgs),
    /// Re-capture, recompute the delta, and assert it is EMPTY.
    ///
    /// The only verb that declares success. Idempotence is MEASURED at the
    /// destination and never asserted in a comment.
    Verify(VerifyArgs),
    /// Every staged write, and what its gates read.
    List,
}

#[derive(ClapArgs)]
pub struct AddArgs {
    /// The intent file, project-relative. It carries boxes and paints, so it
    /// lives in the subscriber tree and never under `.vds/`.
    #[arg(long, value_name = "PATH")]
    intent: PathBuf,
    /// Who is staging this.
    #[arg(long, default_value = "")]
    by: String,
    /// The authorities this staging rests on.
    #[arg(long, value_delimiter = ',', default_value = "draft S-7E")]
    basis: Vec<String>,
}

#[derive(ClapArgs)]
pub struct PlanArgs {
    #[arg(long)]
    id: String,
    /// A saved `GET /v1/files/:key/nodes` capture carrying the target frame.
    ///
    /// The diff is taken against THIS and never against the network: the plan
    /// is read by the gates, and a proof may not call a network.
    #[arg(long, value_name = "PATH")]
    from: PathBuf,
}

#[derive(ClapArgs)]
pub struct ApplyArgs {
    #[arg(long)]
    id: String,
    /// The lock holder's name, as the estate's writer lock spells one.
    #[arg(long)]
    holder: String,
    /// Where the advisory lock lives. The SAME file and the SAME holder
    /// protocol the estate's `scripts/figma-writer-lock.py` uses, so a
    /// co-operating VDS apply and a co-operating agent now collide loudly
    /// instead of not colliding at all.
    #[arg(long, default_value = ".figma-locks")]
    locks: PathBuf,
}

#[derive(ClapArgs)]
pub struct VerifyArgs {
    #[arg(long)]
    id: String,
    /// A FRESH capture of the target frame, taken after the apply.
    #[arg(long, value_name = "PATH")]
    from: PathBuf,
}

pub fn run(ctx: &Context, args: &Args) -> Result<i32> {
    let project = ctx.project()?;
    let store = Store::new(&project);
    match &args.action {
        Action::Add(a) => add(&project, &store, a),
        Action::Plan(a) => plan(&project, &store, a),
        Action::Apply(a) => apply(&project, &store, a),
        Action::Verify(a) => verify(&project, &store, a),
        Action::List => list(&project, &store),
    }
}

/// Everything the gates read, assembled once so `add` and `plan` cannot differ
/// about what they were measured over.
struct Assembled {
    intent: vds_core::StageIntent,
    intent_digest: Digest,
    sheet: Option<Sheet>,
    stylesheet_rel: String,
    screens: Vec<vds_core::ScreenRecord>,
    bindings: Option<vds_core::RouteBindingLedger>,
    inputs: Vec<StageInput>,
}

fn assemble(
    project: &vds_core::Project,
    store: &Store,
    intent_path: &std::path::Path,
) -> Result<Assembled> {
    if let Some(why) = vds_core::intent_root_defect(project) {
        return Err(VdsError::precondition(format!("[stage] intent_root {why}")));
    }
    let intent = vds_core::read_intent(project, intent_path)?;
    let intent_digest = Digest::of_file(intent_path)?;

    let stylesheet = project.root.join(&project.config.surface.stylesheet);
    let stylesheet_rel = project.rel(&stylesheet);
    let mut inputs = vec![StageInput {
        name: project.rel(intent_path),
        digest: intent_digest.clone(),
    }];
    let sheet = if stylesheet.is_file() {
        let text =
            std::fs::read_to_string(&stylesheet).map_err(|e| VdsError::io(&stylesheet_rel, e))?;
        inputs.push(StageInput {
            name: stylesheet_rel.clone(),
            digest: Digest::of_file(&stylesheet)?,
        });
        let parsed = Sheet::parse(&text);
        (parsed.malformed().is_none()).then_some(parsed)
    } else {
        None
    };

    let located = store.read_screens()?;
    for row in &located {
        inputs.push(StageInput {
            name: project.rel(&row.path),
            digest: Digest::of_file(&row.path)?,
        });
    }
    let screens: Vec<vds_core::ScreenRecord> = located.into_iter().map(|l| l.value).collect();

    let bindings = vds_core::read_route_bindings(project)?;
    if bindings.is_some() {
        let path = vds_core::route_bindings_path(project);
        inputs.push(StageInput {
            name: project.rel(&path),
            digest: Digest::of_file(&path)?,
        });
    }

    Ok(Assembled {
        intent,
        intent_digest,
        sheet,
        stylesheet_rel,
        screens,
        bindings,
        inputs,
    })
}

fn gates_of(assembled: &Assembled, reserved: &[String]) -> Vec<vds_core::GateVerdict> {
    read_gates(&GateInputs {
        intent: &assembled.intent,
        sheet: assembled.sheet.as_ref(),
        stylesheet_path: &assembled.stylesheet_rel,
        screens: &assembled.screens,
        bindings: assembled.bindings.as_ref(),
        reserved_properties: reserved,
    })
}

fn add(project: &vds_core::Project, store: &Store, args: &AddArgs) -> Result<i32> {
    let intent_path = project.root.join(&args.intent);
    let assembled = assemble(project, store, &intent_path)?;
    let gates = gates_of(&assembled, &project.config.stage.reserved_paint_properties);

    let id = StageId::allocate(&store.stages_dir())?;
    let record = StageRecord {
        id: id.clone(),
        route: assembled.intent.route.clone(),
        target: StageTarget {
            file_key: assembled.intent.file_key.clone(),
            node_id: assembled.intent.node_id.clone(),
        },
        intent_path: project.rel(&intent_path),
        intent_digest: assembled.intent_digest.clone(),
        inputs: assembled.inputs.clone(),
        gates: gates.clone(),
        apply: None,
        staged_by: if args.by.trim().is_empty() {
            vds_core::actor()
        } else {
            args.by.clone()
        },
        staged_at: Timestamp::now(),
        basis: args.basis.clone(),
        notes: None,
    };
    let defects = record.defects();
    if !defects.is_empty() {
        return Err(VdsError::precondition(format!(
            "this staged write would not validate, so nothing was written:\n  {}",
            defects.join("\n  ")
        )));
    }
    let path = store.stage_path(&id);
    store.create(&path, &record)?;

    println!("staged {id} {:?}", record.route);
    println!("  path:   {}", project.rel(&path));
    println!(
        "  target: {} in file {}",
        record.target.node_id, record.target.file_key
    );
    println!("  intent: {}", record.intent_path);
    println!();
    print_gates(&gates);
    println!();
    println!(
        "THIS COMMAND RAN NO GATE AS A CONDITION OF WRITING. The door is not the wall \
         (VDS S-11(5)):\n  it measured all four and wrote down what they read, refusals \
         included, because a refusal\n  nobody can write down is a refusal nobody can look at. \
         `vds proof staged_write` re-derives\n  every one of them rather than believing this \
         record."
    );
    if gates.iter().any(|g| g.reading == GateReading::Refused) {
        println!();
        println!("A gate REFUSES. `vds stage plan` will not emit an operation list against it.");
        return Ok(EXIT_VIOLATION);
    }
    println!();
    println!("Next: vds stage plan --id {id} --from <a saved capture of the target frame>");
    Ok(PASSED)
}

fn print_gates(gates: &[vds_core::GateVerdict]) {
    println!("gates:");
    for verdict in gates {
        let label = match verdict.reading {
            GateReading::Cleared => "cleared      ",
            GateReading::Refused => "REFUSED      ",
            GateReading::CouldNotRun => "could_not_run",
        };
        println!(
            "  {label} {:20} {}",
            verdict.gate.as_str(),
            verdict.gate.limb()
        );
        for line in wrap(&verdict.because) {
            println!("      {line}");
        }
    }
}

/// Fold a sentence at a readable width, so a refusal is legible in a terminal.
fn wrap(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.len() + 1 + word.len() > 88 {
            out.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        out.push(line);
    }
    out
}

fn plan(project: &vds_core::Project, store: &Store, args: &PlanArgs) -> Result<i32> {
    let id = StageId::parse(&args.id)?;
    let record = store.read_stage(&id)?.value;
    if record.apply.is_some() {
        return Err(VdsError::precondition(format!(
            "{id} already records an apply. Re-planning over one would emit an operation list \
             against a frame the recorded apply has already changed, and the two would disagree \
             about what the canvas holds. Stage a new write instead."
        )));
    }
    let intent_path = project.root.join(&record.intent_path);
    let assembled = assemble(project, store, &intent_path)?;
    let gates = gates_of(&assembled, &project.config.stage.reserved_paint_properties);

    let refusals: Vec<&vds_core::GateVerdict> = gates
        .iter()
        .filter(|g| g.reading == GateReading::Refused)
        .collect();
    if !refusals.is_empty() {
        println!("REFUSED, and nothing was emitted:");
        for verdict in &refusals {
            println!("  {} ({})", verdict.gate, verdict.gate.limb());
            for line in wrap(&verdict.because) {
                println!("      {line}");
            }
        }
        println!();
        println!(
            "A plan is what an apply reads, so emitting one over a refused gate would put the \
             refusal\n  downstream of the act it exists to stop."
        );
        return Ok(EXIT_VIOLATION);
    }

    if !args.from.is_file() {
        return Err(VdsError::precondition(format!(
            "--from {} does not exist. The diff is taken against a SAVED capture and never \
             against the network, because the gates that read this plan are a proof and a proof \
             may not call one (VDS S-7(2)(1)).",
            args.from.display()
        )));
    }
    let body =
        std::fs::read_to_string(&args.from).map_err(|e| VdsError::io(args.from.display(), e))?;
    // The project's OWN vocabulary, passed through, because the words a file uses
    // for "this layer is the current source" live in `[screens] authority_markers`.
    // Defaulting them here would resolve the authority layer only for projects
    // using the shipped spellings and would silently create a second set of bands
    // for every project that does not.
    let Some(reading) =
        vds_figma::read_frame(&body, &assembled.intent.node_id, &project.config.screens)?
    else {
        return Err(VdsError::precondition(format!(
            "the capture does not carry node {}. A diff taken against a frame nobody captured \
             would emit a create for every band and rebuild a frame that may already be right, \
             so this refuses rather than guessing. Re-capture including that node.",
            assembled.intent.node_id
        )));
    };

    // THE DECLARED EXTENT, AGAINST THE CAPTURE. G3 read it from the intent because
    // it had no capture in front of it, so on its own the declaration is a claim -
    // and a claim believed is a claim that can be minted, which is the defect R7
    // exists for one artefact along. This is where the claim meets the drawing.
    if let Some(why) = vds_figma::stage::extent_disagreement(&assembled.intent, &reading) {
        println!("REFUSED, and nothing was emitted:");
        for line in wrap(&why) {
            println!("      {line}");
        }
        println!();
        println!(
            "G3 measured the canonical shell against the extent this intent DECLARES, because a \
             proof may\n  not fetch a capture (VDS S-7(2)(1)). This command has one, so the \
             declaration is checked\n  against it here rather than believed."
        );
        return Ok(EXIT_VIOLATION);
    }
    if let Some(under) = &reading.bands_under {
        println!(
            "bands read from the authority layer {under:?}, not from the frame's own children."
        );
        println!(
            "  This reader used to take the direct children, which on a frame like this one saw \
             ZERO\n  bands, reported every declared band missing, and drew a SECOND full set \
             beside the first."
        );
        println!();
    }

    // The paints, resolved ONCE, in the base scope, from the same sheet the
    // contrast gate measured. Resolving them twice would give the gate and the
    // write two opinions about one token.
    let mut paints: BTreeMap<String, vds_css::colour::Colour> = BTreeMap::new();
    if let Some(sheet) = &assembled.sheet {
        let base = sheet
            .base_selector()
            .map(str::to_owned)
            .or_else(|| sheet.theme_selectors().first().map(|s| (*s).to_owned()));
        if let Some(base) = base {
            for band in &assembled.intent.bands {
                let Some(paint) = &band.paint else { continue };
                if let Some(value) = sheet.resolve(&base, paint.property.trim()).value()
                    && let Ok(colour) = vds_css::colour::parse(value)
                {
                    paints.insert(property_key(&paint.property), colour);
                }
            }
        }
    }

    let operations = vds_figma::diff(&DiffInputs {
        intent: &assembled.intent,
        reading: &reading,
        paints: &paints,
    });
    let emitted = vds_figma::emit_plan(
        &id,
        &assembled.intent,
        assembled.intent_digest.clone(),
        &reading,
        &project.rel(&args.from),
        Digest::of_file(&args.from)?,
        operations,
        // THE READINGS THIS PLAN IS EMITTED UNDER, onto the face of the artefact.
        // The plan is the thing this capability calls REVIEWABLE and it carried no
        // reading from any gate at all, so a reviewer holding one could not see
        // whether a single gate had run. Every reading here is `cleared` or
        // `could_not_run`, because the refusal branch above returned already.
        gates.clone(),
        Timestamp::now(),
    )?;
    let path = vds_core::plan_path(project, &id);
    vds_core::write_plan(project, &path, &emitted)?;

    println!("wrote {}", project.rel(&path));
    println!("  operations: {}", emitted.operation_count());
    println!("  chunks:     {}", emitted.chunks.len());
    println!("  digest:     {}", emitted.content_digest);
    println!(
        "  scope:      {} ({})",
        emitted.container.name, emitted.container.node_id
    );
    println!();
    for chunk in &emitted.chunks {
        println!(
            "  chunk {} ({} operation(s)):",
            chunk.ordinal,
            chunk.operations.len()
        );
        for operation in &chunk.operations {
            println!("    {operation}");
        }
    }
    if !emitted.untouched.is_empty() {
        println!();
        println!(
            "  {} layer(s) in this frame are NOT bands and no operation above can reach them:",
            emitted.untouched.len()
        );
        for name in emitted.untouched.iter().take(12) {
            println!("    {name}");
        }
    }

    // THE BANDS LEFT ALONE. Not operations and not findings: the bands the OLD
    // diff deleted on the strength of the intent's silence. Published so a
    // reviewer can see that this plan deliberately does nothing to them.
    let left_alone = reading.undeclared_bands(&assembled.intent);
    if !left_alone.is_empty() {
        println!();
        println!(
            "  {} band(s) in this frame the intent neither declares nor deletes, and they are \
             LEFT ALONE:",
            left_alone.len()
        );
        for band in &left_alone {
            println!("    {band}");
        }
        println!(
            "    Each of these used to be DELETED, purely because the intent did not mention it. \
             To\n    remove one, name it in the intent's `deletes` list and re-plan."
        );
    }

    println!();
    println!("  {}", emitted.coverage);

    println!();
    println!(
        "THIS IS THE REVIEWABLE ARTEFACT. Nothing has reached the canvas. There is no \
         page-level and\n  no frame-level delete in the vocabulary above, and a delete reaches \
         one band only where its\n  name is in the closed review set AND the intent EXPLICITLY \
         LISTS it in `deletes`. Silence is\n  not permission to delete."
    );

    // THE DESTRUCTIVE OPERATIONS, ON THEIR OWN AND LOUDLY. The one verb that can
    // lose a designer's work must not be found by reading a list of six.
    let destructive = emitted.destructive();
    if !destructive.is_empty() {
        println!();
        println!(
            "!! DESTRUCTIVE: {} of these {} operation(s) DELETE a band, and a delete is the one \
             act on",
            destructive.len(),
            emitted.operation_count()
        );
        println!(
            "!! this path that re-running cannot undo. Whatever a designer put inside these bands \
             goes"
        );
        println!("!! with them:");
        for operation in &destructive {
            println!("!!     {operation}");
        }
        println!(
            "!! Each one is here because THIS INTENT NAMES IT in `deletes`. If that is not what \
             was\n!! meant, amend the intent and re-plan: nothing has reached the canvas yet."
        );
    }
    if emitted.operation_count() == 0 {
        println!();
        println!(
            "ZERO OPERATIONS: the frame already is the intent. That is what an idempotent second \
             run\n  looks like, and it is MEASURED here rather than assumed."
        );
    }
    Ok(PASSED)
}

/// The advisory writer lock, in the estate's own file and holder protocol.
///
/// ADVISORY, and the word is not decoration. It cannot stop a writer that does
/// not ask for it, because the plugin bridge has no admission control this
/// could sit in front of. What it converts is a silent collision into a loud
/// refusal for every writer that DOES ask, and until now VDS was not one of
/// them at all: a co-operating VDS apply and a co-operating agent would not
/// even have collided.
fn lock_path(project: &vds_core::Project, locks: &std::path::Path, file_key: &str) -> PathBuf {
    let safe: String = file_key
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    project.root.join(locks).join(format!("{safe}.json"))
}

fn apply(project: &vds_core::Project, store: &Store, args: &ApplyArgs) -> Result<i32> {
    let id = StageId::parse(&args.id)?;
    let mut record = store.read_stage(&id)?.value;
    if record.apply.is_some() {
        return Err(VdsError::precondition(format!(
            "{id} already records an apply. A second apply over one record would leave two \
             attempts under one identifier and nothing saying which the frame reflects."
        )));
    }
    let plan_path = vds_core::plan_path(project, &id);
    let Some(plan) = vds_core::read_plan(project, &plan_path)? else {
        return Err(VdsError::precondition(format!(
            "{id} has no plan at {}. The operation list exists BEFORE anything reaches the \
             canvas, and that is the whole of what this capability buys.\n  Run: vds stage plan \
             --id {id} --from <a saved capture>",
            project.rel(&plan_path)
        )));
    };
    if let Some(why) = plan.untrustworthy_because()? {
        return Err(VdsError::precondition(format!(
            "the plan cannot be relied on, so nothing was applied: {why}"
        )));
    }
    if vds_figma::frames::authority_of(&plan.container.name, &project.config.screens)
        != Some(vds_figma::frames::Authority::Current)
    {
        return Err(VdsError::precondition(format!(
            "the plan's authority container {:?} is not a named CURRENT SOURCE layer under \
             [screens] authority_markers. Apply refuses rather than handing a bridge an \
             unresolvable or ambiguous parent.",
            plan.container.name
        )));
    }
    if plan.intent_digest != record.intent_digest {
        return Err(VdsError::precondition(format!(
            "the plan was emitted from an intent digesting to {} and the record pins {}. An \
             apply that read a plan computed from another intent would write something nobody \
             reviewed.",
            plan.intent_digest, record.intent_digest
        )));
    }

    // TAKE THE LOCK. Same file, same holder protocol.
    let path = lock_path(project, &args.locks, &record.target.file_key);
    if let Ok(text) = std::fs::read_to_string(&path) {
        let held: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|_| {
            // A corrupt lock is treated as HELD and never as absent. Failing
            // closed is the whole point: an unreadable lock must never read as
            // "free to write".
            serde_json::json!({"holder": "(an unreadable lock file)", "task": "unknown"})
        });
        let holder = held.get("holder").and_then(|v| v.as_str()).unwrap_or("");
        if holder != args.holder {
            println!(
                "REFUSED. The Figma writer lock on {} is held.",
                record.target.file_key
            );
            println!("  holder: {holder}");
            println!(
                "  task:   {}",
                held.get("task")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
            );
            println!();
            println!(
                "Do not write to this file. Two writers is how work was destroyed here on \
                 2026-07-25:\n  a build step whose documented behaviour is to delete a page by \
                 name and recreate it ran\n  while another writer had landed parts of the same \
                 family."
            );
            return Ok(EXIT_VIOLATION);
        }
    }
    std::fs::create_dir_all(path.parent().expect("a parent"))
        .map_err(|e| VdsError::io(path.display(), e))?;
    let taken = Timestamp::now();
    vds_core::write_text_atomically(
        &path,
        &format!(
            "{}\n",
            serde_json::to_string_pretty(&serde_json::json!({
                "holder": args.holder,
                "task": format!("vds stage apply {id} on {}", record.target.node_id),
                "acquired": taken.as_str(),
            }))
            .map_err(|e| VdsError::Serialize {
                what: "the writer lock".into(),
                message: e.to_string(),
            })?
        ),
    )?;

    record.apply = Some(ApplyOutcome {
        applied_at: taken,
        applied_by: vds_core::actor(),
        lock_holder: args.holder.clone(),
        chunks: plan.chunks.len() as u32,
        operations: plan.operation_count() as u32,
        plan_digest: plan.content_digest.clone(),
        verification: None,
    });
    store.replace(&store.stage_path(&id), &record)?;

    println!(
        "ACQUIRED the advisory writer lock on {}.",
        record.target.file_key
    );
    println!("  lock:   {}", project.rel(&path));
    println!("  holder: {}", args.holder);
    println!();
    println!(
        "ADVISORY, and that is not a hedge. This lock cannot stop a writer that does not ask for \
         it,\n  because the plugin bridge has no admission control it could sit in front of. \
         What it does\n  is make a co-operating VDS apply and a co-operating agent collide \
         loudly, which they could\n  not do before, because VDS was not a lock participant at \
         all."
    );
    println!();
    println!(
        "Hand these {} chunk(s) to the plugin bridge IN ORDER. There is no transaction and no",
        plan.chunks.len()
    );
    println!(
        "  operation scope: {} ({})",
        plan.container.name, plan.container.node_id
    );
    println!(
        "  Every create, edit and delete below MUST resolve its band under this container; an \
         existing sibling is outside the plan."
    );
    println!(
        "  atomicity: chunk {} can fail after the earlier ones have landed, and nothing rolls",
        plan.chunks.len()
    );
    println!("  them back. That is why the next command is the one that declares success.");
    for chunk in &plan.chunks {
        println!();
        println!("  --- chunk {} of {} ---", chunk.ordinal, plan.chunks.len());
        println!("      digest: {}", chunk.digest);
        for operation in &chunk.operations {
            println!("      {operation}");
        }
    }
    println!();
    println!("Then, and only then:");
    println!("  vds stage verify --id {id} --from <a FRESH capture of the frame>");
    println!(
        "  python3 scripts/figma-writer-lock.py release --file {} --holder {}",
        record.target.file_key, args.holder
    );
    Ok(PASSED)
}

fn verify(project: &vds_core::Project, store: &Store, args: &VerifyArgs) -> Result<i32> {
    let id = StageId::parse(&args.id)?;
    let mut record = store.read_stage(&id)?.value;
    let Some(apply) = record.apply.clone() else {
        return Err(VdsError::precondition(format!(
            "{id} records no apply, so there is nothing to verify. Verifying an unapplied stage \
             would measure the frame against an intent nothing wrote."
        )));
    };
    let intent_path = project.root.join(&record.intent_path);
    let assembled = assemble(project, store, &intent_path)?;

    let body =
        std::fs::read_to_string(&args.from).map_err(|e| VdsError::io(args.from.display(), e))?;
    // THROUGH THE AUTHORITY LAYER, with the project's own vocabulary, and this is
    // the call site the defect was worst at: the verification re-read the frame the
    // SAME WRONG WAY the diff had, found no residual, and declared success over a
    // frame that had just been given a second full set of bands.
    let Some(reading) =
        vds_figma::read_frame(&body, &assembled.intent.node_id, &project.config.screens)?
    else {
        return Err(VdsError::precondition(format!(
            "the capture does not carry node {}, so the delta cannot be recomputed and this \
             apply cannot be declared finished.",
            assembled.intent.node_id
        )));
    };

    let plan_path = vds_core::plan_path(project, &id);
    let Some(plan) = vds_core::read_plan(project, &plan_path)? else {
        return Err(VdsError::precondition(format!(
            "{id} records an apply but has no plan at {}. The verification cannot prove which \
             authority container the apply targeted.",
            project.rel(&plan_path)
        )));
    };
    if let Some(why) = plan.untrustworthy_because()? {
        return Err(VdsError::precondition(format!(
            "the plan cannot be used for verification: {why}"
        )));
    }
    if plan.container != reading.container {
        println!("REFUSED. The resolved authority container changed after the plan was emitted.");
        println!(
            "  planned: {} ({})",
            plan.container.name, plan.container.node_id
        );
        println!(
            "  current: {} ({})",
            reading.container.name, reading.container.node_id
        );
        println!(
            "  No residual is accepted from a different subtree: re-plan against a fresh capture \
             before verifying."
        );
        return Ok(EXIT_VIOLATION);
    }

    let mut paints: BTreeMap<String, vds_css::colour::Colour> = BTreeMap::new();
    if let Some(sheet) = &assembled.sheet {
        let base = sheet
            .base_selector()
            .map(str::to_owned)
            .or_else(|| sheet.theme_selectors().first().map(|s| (*s).to_owned()));
        if let Some(base) = base {
            for band in &assembled.intent.bands {
                let Some(paint) = &band.paint else { continue };
                if let Some(value) = sheet.resolve(&base, paint.property.trim()).value()
                    && let Ok(colour) = vds_css::colour::parse(value)
                {
                    paints.insert(property_key(&paint.property), colour);
                }
            }
        }
    }
    let residual = vds_figma::diff(&DiffInputs {
        intent: &assembled.intent,
        reading: &reading,
        paints: &paints,
    });

    // The frame's CURRENT content digest, over the captured subtree exactly as
    // the frame ledger computes one, so the bypass rule compares like with like.
    let payload: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| VdsError::parse(project.rel(&args.from), "JSON", e))?;
    let document = payload
        .get("nodes")
        .and_then(|v| v.as_object())
        .and_then(|nodes| {
            nodes.iter().find_map(|(node_id, wrapper)| {
                (vds_figma::frames::normalise_node_id(node_id)
                    == vds_figma::frames::normalise_node_id(&assembled.intent.node_id))
                .then(|| wrapper.get("document"))?
            })
        })
        .ok_or_else(|| {
            VdsError::precondition("the capture carries no document for the target node")
        })?;
    let frame_digest_after = Digest::of_value(document)?;

    let mut updated = apply;
    updated.verification = Some(Verification {
        verified_at: Timestamp::now(),
        frame_digest_after: frame_digest_after.clone(),
        residual_operations: residual.len() as u32,
    });
    record.apply = Some(updated);
    store.replace(&store.stage_path(&id), &record)?;

    println!("verified {id} against {}", project.rel(&args.from));
    println!("  residual operations: {}", residual.len());
    println!("  frame digest after:  {frame_digest_after}");
    if residual.is_empty() {
        println!();
        println!(
            "EMPTY. The frame IS the intent, measured at the destination rather than asserted. \
             That\n  digest now joins the set the bypass rule accepts for this frame, so a later \
             write that\n  does not come through VDS is visible after the fact."
        );
        return Ok(PASSED);
    }
    println!();
    println!("NOT FINISHED. The delta still emits:");
    for operation in &residual {
        println!("    {operation}");
    }
    println!();
    println!(
        "There is no atomicity on this path: the bridge caps one call and offers no transaction, \
         so a\n  chunk can fail after earlier chunks landed. Re-plan against a fresh capture and \
         apply again."
    );
    Ok(EXIT_VIOLATION)
}

fn list(project: &vds_core::Project, store: &Store) -> Result<i32> {
    let records = store.read_stages()?;
    if records.is_empty() {
        println!("no staged write.");
        println!(
            "  `staged_write` reports VACUOUS over an empty stage register, which is not a pass \
             and\n  is not evidence for any warrant (VDS S-7(2)(4))."
        );
        return Ok(PASSED);
    }
    println!("{} staged write(s):", records.len());
    let mut refused = 0;
    for located in &records {
        let record = &located.value;
        let state = match &record.apply {
            None => "staged".to_owned(),
            Some(a) => match &a.verification {
                None => "applied, NOT VERIFIED".to_owned(),
                Some(v) if v.succeeded() => "applied and verified".to_owned(),
                Some(v) => format!("applied, {} residual", v.residual_operations),
            },
        };
        println!(
            "  {:10} {:34} {:12} {}",
            record.id.as_str(),
            record.route,
            record.target.node_id,
            state
        );
        for verdict in record.refusals() {
            refused += 1;
            println!("      REFUSED {} ({})", verdict.gate, verdict.gate.limb());
        }
        for gate in record.gates_not_asked() {
            println!(
                "      NOT ASKED {gate}: a gate absent from the record reads as green to anyone \
                 counting refusals"
            );
        }
    }
    println!();
    println!("  path: {}", project.rel(&store.stages_dir()));
    if refused > 0 {
        return Ok(EXIT_VIOLATION);
    }
    Ok(PASSED)
}
