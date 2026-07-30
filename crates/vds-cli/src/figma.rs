//! `vds figma`, `vds brief` and `vds impl`: the design round trip.
//!
//! ```text
//!   vds brief            ->  what an agent may draw into Figma
//!   vds figma pull       ->  what the Figma file actually contains
//!   vds impl <CMP-id>    ->  what that drawing must become in code
//! ```
//!
//! All three are projections of the same register, which is what keeps the round
//! trip from drifting at the join. None of them carries a design value: the brief
//! and the contract state requirements, and the ledger records names and node
//! ids. What things look like stays in the Figma file and in `app/globals.css`,
//! which is where [2026] VJS-CC-OPBOX 3 D1 put them.

use std::path::PathBuf;

use clap::{Args as ClapArgs, Subcommand, ValueEnum};
use vds_core::{ComponentId, EXIT_VIOLATION, Result, Stage, VdsError};
use vds_figma::{FigmaSource, ledger as figma_ledger, pull};
use vds_store::Store;

use crate::{Context, PASSED};

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum Format {
    /// Prose, for pasting into a prompt or reading in a terminal.
    Markdown,
    /// Structured, for a tool that consumes it.
    Yaml,
    Json,
}

// ------------------------------------------------------------------ vds figma

#[derive(ClapArgs)]
pub struct FigmaArgs {
    #[command(subcommand)]
    action: FigmaAction,
}

#[derive(Subcommand)]
enum FigmaAction {
    /// Read the decided-target file into `.vds/ledgers/figma.yaml`.
    ///
    /// A LEDGER GENERATOR, not a proof: VDS S-7(2)(1) forbids a network call
    /// inside a proof, so this runs out of band and the proofs read what it
    /// wrote.
    Pull(PullArgs),
    /// Derive the SCREEN FRAME ledger from saved `nodes` captures.
    ///
    /// The other half of the Figma seam, and the one `screen_parity` reads.
    /// `pull` records the file's COMPONENT sets; this records what its SCREEN
    /// frames draw. Both are ledger generators run out of band, because
    /// VDS S-7(2)(1) forbids a network call inside a proof.
    ///
    /// There is deliberately no API transport here. A screen file's frames come
    /// in batches with an ids list in the query string, and building that
    /// batching into VDS would put a retry loop, a resume and a rate limit
    /// inside a governance tool. The capture is the subject's to run, with its
    /// own token, and this reads what it wrote.
    Frames(FramesArgs),
    /// Report what the ledger says, and what it cannot say.
    Status,
}

#[derive(ClapArgs)]
pub struct FramesArgs {
    /// The decided-target file. Defaults to the one every SCREEN record names.
    #[arg(long)]
    file_key: Option<String>,
    /// One or more saved `GET /v1/files/:key/nodes` responses.
    #[arg(long, value_name = "PATH", required = true, num_args = 1..)]
    from: Vec<PathBuf>,
}

#[derive(ClapArgs)]
pub struct PinArgs {
    #[command(subcommand)]
    action: PinAction,
}

#[derive(Subcommand)]
enum PinAction {
    /// Derive a pin: compare the shipped stylesheet against the decided-target
    /// file's variables, and record the verdicts.
    Generate(PinGenerateArgs),
}

#[derive(ClapArgs)]
pub struct PinGenerateArgs {
    /// The decided-target file. Defaults to the one every record names.
    #[arg(long)]
    file_key: Option<String>,
    /// Compare against a saved `variables/local` response instead of calling the
    /// API. Needs no token and no network, and is byte-reproducible.
    #[arg(long, value_name = "PATH")]
    from: Option<PathBuf>,
    /// The prefix this project puts on its custom properties, e.g. `sf-`.
    ///
    /// A Figma variable `control/border` becomes `--<prefix>control-border`.
    /// That correspondence is a convention nobody has written down, so it is
    /// applied, labelled as DERIVED in the report, and any variable it cannot
    /// place becomes a row the pin declines to enforce with the reason on it.
    #[arg(long, default_value = "")]
    prefix: String,
    /// What this pin is about, recorded on it.
    #[arg(
        long,
        default_value = "the shipped stylesheet against the decided-target variables"
    )]
    subject: String,
    /// The root font size in pixels, for resolving a `rem` against a Figma FLOAT.
    ///
    /// A REQUIREMENT and not a realisation: 16 is the CSS initial value, from the
    /// specification rather than from the design. A project that changes it says
    /// so here, because a comparison made against the wrong root would be
    /// confidently wrong about every `rem` in the sheet.
    #[arg(long, default_value_t = vds_figma::pin::DEFAULT_ROOT_PX)]
    root_px: f64,
}

#[derive(ClapArgs)]
pub struct PullArgs {
    /// The decided-target file. Defaults to the one every record names.
    #[arg(long)]
    file_key: Option<String>,
    /// Derive the ledger from a saved `GET /v1/files/:key` response instead of
    /// calling the API. Needs no token and no network, and is byte-reproducible.
    #[arg(long, value_name = "PATH")]
    from: Option<PathBuf>,
}

pub fn run_pin(ctx: &Context, args: &PinArgs) -> Result<i32> {
    let project = ctx.project()?;
    let store = Store::new(&project);
    match &args.action {
        PinAction::Generate(a) => pin_generate(&store, a),
    }
}

/// Fetch or read the decided-target variables, CACHE them, and compare.
///
/// The cache is the whole reason this is lawful. Comparing two realisations
/// needs both in hand, and VDS S-2(2) forbids `.vds/` holding either. S-3(9)
/// names `.vds/cache/` as one of two ignored directories, gitignored and skipped
/// by name by `no_stored_values`, so the raw response lives there and only the
/// verdicts reach a record. Nothing about S-2 needed amending: the cache is the
/// escape hatch the specification already provided, and this is what it is for.
fn pin_generate(store: &Store, args: &PinGenerateArgs) -> Result<i32> {
    let project = store.project;
    let file_key = resolve_file_key(store, &args.file_key)?;
    let cached = vds_figma::pin::cached_variables_path(project, &file_key);
    std::fs::create_dir_all(vds_figma::pin::cache_dir(project))
        .map_err(|e| VdsError::io(project.rel(&vds_figma::pin::cache_dir(project)), e))?;

    let source = match &args.from {
        Some(path) => vds_figma::pin::Source::Saved(path.as_path()),
        None => vds_figma::pin::Source::Network,
    };
    let body = match &args.from {
        Some(path) => {
            if !path.is_file() {
                return Err(VdsError::precondition(format!(
                    "--from {} does not exist",
                    path.display()
                )));
            }
            std::fs::read_to_string(path).map_err(|e| VdsError::io(path.display(), e))?
        }
        None => {
            let api = pull::FigmaApi::from_env()?;
            println!("reading the decided-target variables over the network...");
            pull::FigmaSource::fetch_variables(&api, &file_key)?
        }
    };
    std::fs::write(&cached, &body).map_err(|e| VdsError::io(project.rel(&cached), e))?;

    let generated = vds_figma::pin::generate(
        store,
        &file_key,
        &cached,
        source,
        &args.prefix,
        &args.subject,
        args.root_px,
    )?;

    println!(
        "cached the decided-target response at {}",
        project.rel(&cached)
    );
    println!(
        "  That directory is one of the two VDS S-3(9) ignores: gitignored, skipped by name by"
    );
    println!(
        "  `no_stored_values`, and counted. It is the only place in VDS a design value may sit,"
    );
    println!("  and it is why this comparison is lawful without amending VDS S-2.");
    println!();
    for line in &generated.report {
        println!("{line}");
    }
    println!();
    println!("wrote {}", project.rel(&generated.path));
    println!(
        "  {} rows, {} enforced. Every row carries a NAME and an AGREEMENT and nothing a brute",
        generated.pin.rows_considered, generated.pin.rows_enforced
    );
    println!("  force could invert (VDS S-2(7), SUBMISSION-VDS-006).");
    println!();
    println!("Now check it, which is a different act from generating it:");
    println!("  vds proof token_pin");
    Ok(PASSED)
}

pub fn run_figma(ctx: &Context, args: &FigmaArgs) -> Result<i32> {
    let project = ctx.project()?;
    let store = Store::new(&project);
    match &args.action {
        FigmaAction::Pull(pull_args) => pull_command(&store, pull_args),
        FigmaAction::Frames(frames_args) => frames_command(&project, &store, frames_args),
        FigmaAction::Status => status(&store),
    }
}

fn resolve_file_key(store: &Store, explicit: &Option<String>) -> Result<String> {
    if let Some(key) = explicit {
        return Ok(key.clone());
    }
    pull::declared_file_key(store)?.ok_or_else(|| {
        VdsError::precondition(
            "no register record names a Figma node, so there is no decided-target file to \
             read.\n  Record one with: vds register amend <CMP-id> --kind non_breaking \
             --what \"record the figma node\" --figma FILEKEY#12:34\n  Or name the file \
             explicitly with --file-key.",
        )
    })
}

fn pull_command(store: &Store, args: &PullArgs) -> Result<i32> {
    let file_key = resolve_file_key(store, &args.file_key)?;

    let ledger = match &args.from {
        Some(path) => {
            if !path.is_file() {
                return Err(VdsError::precondition(format!(
                    "--from {} does not exist",
                    path.display()
                )));
            }
            pull::from_saved(store, &file_key, path)?
        }
        None => {
            let api = pull::FigmaApi::from_env()?;
            let body = api.fetch_file(&file_key)?;
            pull::build_ledger(store, &file_key, &body, &api.describe())?
        }
    };

    let path = pull::write(store, &ledger)?;
    let resolved = ledger.nodes.iter().filter(|n| n.resolved).count();
    let unresolved = ledger.nodes.len() - resolved;

    println!("wrote {}", store.project.rel(&path));
    println!(
        "  file:           {} ({})",
        ledger.file_name, ledger.file_key
    );
    println!("  file version:   {}", ledger.file_version);
    println!("  nodes resolved: {resolved} of {}", ledger.nodes.len());
    if unresolved > 0 {
        println!("  UNRESOLVED:     {unresolved}");
        for row in ledger.unresolved() {
            println!(
                "    {} node {}: {}",
                row.component_id,
                row.node_id,
                row.unresolved_because
                    .as_deref()
                    .unwrap_or("no reason recorded")
            );
        }
    }
    if !ledger.unclaimed.is_empty() {
        println!("  unclaimed sets: {}", ledger.unclaimed.len());
        for node in &ledger.unclaimed {
            println!("    {} \"{}\"", node.node_id, node.figma_name);
        }
    }
    println!("  content_digest: {}", ledger.content_digest);
    for note in &ledger.notes {
        println!();
        println!("note: {note}");
    }
    Ok(PASSED)
}

fn status(store: &Store) -> Result<i32> {
    let Some(ledger) = pull::read(store)? else {
        println!("no figma ledger.");
        println!();
        println!(
            "Without one, `states.drawn` in every record is the register's own hand-maintained \
             claim rather than a measurement of the file that decides. VDS S-5(5): a \
             hand-maintained register decays, and this project's evidence for that is in \
             docs/GOAL.md."
        );
        println!();
        println!("Run: vds figma pull");
        return Ok(PASSED);
    };

    println!(
        "figma ledger for {} ({})",
        ledger.file_name, ledger.file_key
    );
    println!("  file version:  {}", ledger.file_version);
    println!("  generated at:  {}", ledger.generated_at);
    println!();

    let mut problems = 0;
    match figma_ledger::check_fresh(&ledger, pull::declared_file_key(store)?.as_deref()) {
        Ok(()) => println!("  the ledger is internally consistent and names the register's file"),
        Err(error) => {
            println!("  STALE OR EDITED:");
            for line in error.to_string().lines() {
                println!("    {line}");
            }
            problems += 1;
        }
    }

    for row in &ledger.nodes {
        if row.resolved {
            let drawn: Vec<&str> = row.states_drawn.iter().map(|s| s.as_str()).collect();
            println!(
                "  {} node {} resolves as {:?}, draws: {}",
                row.component_id,
                row.node_id,
                row.figma_name.as_deref().unwrap_or(""),
                if drawn.is_empty() {
                    "no recognised state".to_owned()
                } else {
                    drawn.join(", ")
                }
            );
        } else {
            println!(
                "  {} node {} DOES NOT RESOLVE: {}",
                row.component_id,
                row.node_id,
                row.unresolved_because
                    .as_deref()
                    .unwrap_or("no reason recorded")
            );
            problems += 1;
        }
    }

    if !ledger.unclaimed.is_empty() {
        println!();
        println!(
            "  {} component sets in the file are claimed by no register record:",
            ledger.unclaimed.len()
        );
        for node in &ledger.unclaimed {
            println!("    {} \"{}\"", node.node_id, node.figma_name);
        }
        problems += 1;
    }

    println!();
    println!("What this ledger cannot say:");
    println!(
        "  Whether any node LOOKS right. It records names, node ids and variant values, and no \
         colour, length, font, duration or easing curve. Those live in the Figma file, which \
         [2026] VJS-CC-OPBOX 3 D1 makes the system of record for what is decided, and VDS reads \
         it and never overrules it."
    );

    Ok(if problems > 0 { EXIT_VIOLATION } else { PASSED })
}

// ------------------------------------------------------------------ vds brief

#[derive(ClapArgs)]
pub struct BriefArgs {
    #[arg(long, value_enum, default_value = "markdown")]
    format: Format,
    /// Write to a file instead of standard output.
    #[arg(long, value_name = "PATH")]
    out: Option<PathBuf>,
}

pub fn run_brief(ctx: &Context, args: &BriefArgs) -> Result<i32> {
    let project = ctx.project()?;
    let store = Store::new(&project);
    let ledger = pull::read(&store)?;
    if let Some(ledger) = &ledger {
        figma_ledger::check_fresh(ledger, pull::declared_file_key(&store)?.as_deref())?;
    }
    let w1 = store.granted_warrant(Stage::W1RegisterComplete)?;
    let brief = vds_figma::build_brief(&store, ledger.as_ref(), w1.as_ref().map(|w| &w.value))?;

    let text = render(&brief, args.format)?;
    emit(&text, &args.out)?;
    Ok(PASSED)
}

// ------------------------------------------------------------------- vds impl

#[derive(ClapArgs)]
pub struct ImplArgs {
    /// The component to implement.
    id: String,
    #[arg(long, value_enum, default_value = "markdown")]
    format: Format,
    #[arg(long, value_name = "PATH")]
    out: Option<PathBuf>,
}

pub fn run_impl(ctx: &Context, args: &ImplArgs) -> Result<i32> {
    let project = ctx.project()?;
    let store = Store::new(&project);
    let id = ComponentId::parse(&args.id)?;
    let ledger = pull::read(&store)?;
    if let Some(ledger) = &ledger {
        figma_ledger::check_fresh(ledger, pull::declared_file_key(&store)?.as_deref())?;
    }
    let contract = vds_figma::build_contract(&store, &id, ledger.as_ref())?;

    let text = match args.format {
        Format::Markdown => contract.to_markdown(),
        Format::Yaml => serde_yaml::to_string(&contract).map_err(|e| VdsError::Serialize {
            what: "the implementation contract".into(),
            message: e.to_string(),
        })?,
        Format::Json => {
            serde_json::to_string_pretty(&contract).map_err(|e| VdsError::Serialize {
                what: "the implementation contract".into(),
                message: e.to_string(),
            })?
        }
    };
    emit(&text, &args.out)?;
    Ok(PASSED)
}

fn render(brief: &vds_figma::GenerationBrief, format: Format) -> Result<String> {
    Ok(match format {
        Format::Markdown => brief.to_markdown(),
        Format::Yaml => serde_yaml::to_string(brief).map_err(|e| VdsError::Serialize {
            what: "the generation brief".into(),
            message: e.to_string(),
        })?,
        Format::Json => serde_json::to_string_pretty(brief).map_err(|e| VdsError::Serialize {
            what: "the generation brief".into(),
            message: e.to_string(),
        })?,
    })
}

fn emit(text: &str, out: &Option<PathBuf>) -> Result<()> {
    match out {
        Some(path) => {
            vds_core::write_text_atomically(path, text)?;
            println!("wrote {}", path.display());
            Ok(())
        }
        None => {
            print!("{text}");
            Ok(())
        }
    }
}

// ----------------------------------------------------------- vds figma frames

/// Derive the frame ledger from saved captures.
///
/// Every number in it is derived and nothing is asserted: re-running against a
/// fresh capture is the only way to change it, which is what makes it a ledger
/// (VDS S-4(2)) rather than a record somebody maintains.
fn frames_command(project: &vds_core::Project, store: &Store, args: &FramesArgs) -> Result<i32> {
    let file_key = match &args.file_key {
        Some(key) => key.clone(),
        None => vds_figma::frames::declared_file_key(store)?.ok_or_else(|| {
            VdsError::precondition(
                "no screen record names a Figma file, so there is no decided-target file to \
                 derive a frame ledger from.\n  Register a screen with: \
                 vds screen add --route <route> --columns <n> --file-key <key> --node-id <id>\n  \
                 Or name the file here with --file-key.",
            )
        })?,
    };

    let ledger = vds_figma::frames::from_saved(&file_key, &args.from, &project.config.screens)?;
    let path = vds_figma::frames::write(project, &ledger)?;

    let disclaimed = ledger.frames.iter().filter(|f| f.disclaimed).count();
    let truncated = ledger.frames.iter().filter(|f| f.truncated).count();
    let quarantined = ledger
        .frames
        .iter()
        .filter(|f| !f.quarantined.is_empty())
        .count();

    println!("wrote {}", project.rel(&path));
    println!("  frames:          {}", ledger.frames.len());
    println!("  capture depth:   {}", ledger.capture_depth);
    println!("  content_digest:  {}", ledger.content_digest);
    println!();
    println!(
        "  {quarantined} frame(s) carry a layer nobody may build from (a legacy underlay, a \
         reference, a target). They are RECORDED rather than dropped: \"this route has a \
         legacy underlay\" is exactly the fact a reader needs in order to not build from it."
    );
    println!(
        "  {disclaimed} frame(s) DISCLAIM THEMSELVES, so they state no contract and \
         screen_parity excludes them rather than measuring a difference that means nothing."
    );
    if truncated > 0 {
        println!();
        println!(
            "  {truncated} frame(s) derived their column count from a subtree that reaches the \
             CAPTURE BOUNDARY, and screen_parity will report each as a finding rather than \
             scoring it. A response carries no \"cut off here\" flag, so \"draws nothing\" \
             and \"we did not look\" are the same bytes and only the depth asked for knows \
             the difference. Re-capture deeper."
        );
    }
    Ok(PASSED)
}
