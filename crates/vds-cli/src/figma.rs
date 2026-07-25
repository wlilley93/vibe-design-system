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
    /// Report what the ledger says, and what it cannot say.
    Status,
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

pub fn run_figma(ctx: &Context, args: &FigmaArgs) -> Result<i32> {
    let project = ctx.project()?;
    let store = Store::new(&project);
    match &args.action {
        FigmaAction::Pull(pull_args) => pull_command(&store, pull_args),
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
