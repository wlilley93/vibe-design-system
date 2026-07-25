//! `vds`: the front door.
//!
//! VDS S-11(5): two front doors, exactly one wall. This binary is a convenience
//! door. The wall is the proof engine, which runs whether or not this door was
//! used, and "the author used the tool" is never proof of conformance.
//!
//! What this tool will NOT do, by design:
//!
//!   - It does not GRANT a warrant. Granting W1, W2 and W4 is VJS's on a
//!     referred submission and W3 is the Principal's alone (VDS S-1(3), S-6(7)).
//!     `warrant record` writes down a grant that already happened, and refuses
//!     to write one that carries no grantor.
//!   - It does not decide anything contested. Every judgement call leaves by a
//!     submission under VDS S-10.
//!   - It does not read or write a design VALUE. Values live in the project's
//!     own systems of record (VDS S-2(2), S-2(3)).

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use vds_core::{EXIT_PASSED, EXIT_PRECONDITION, Result, VdsError};

mod doctor;
mod init;
mod ledger;
mod lock;
mod pack;
mod proof;
mod register;
mod schema;
mod warrant;

#[derive(Parser)]
#[command(
    name = "vds",
    about = "VDS: a design-artefact store and a proof producer. It decides nothing.",
    version
)]
struct Cli {
    /// The project root holding `.vds/config.toml`.
    ///
    /// Global, so it parses before OR after the subcommand. The retired tool
    /// accepted it only before the subcommand, which made its own printed
    /// advice ("Run: vds init --root <project>") an argparse error.
    #[arg(long, global = true, value_name = "PATH")]
    root: Option<PathBuf>,

    /// Emit machine-readable JSON instead of prose, where a command has both.
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scaffold a project's `.vds/`.
    Init(init::Args),
    /// Regenerate a generated inventory.
    Ledger(ledger::Args),
    /// Add, amend, deprecate or retire a component record.
    Register(register::Args),
    /// Run one proof kind, or every implemented kind.
    Proof(proof::Args),
    /// Report warrant status, or record a warrant granted elsewhere.
    Warrant(warrant::Args),
    /// Verify, pin or re-pin the enforcement lock.
    Lock(lock::Args),
    /// Pin or verify the vendored designpack.
    Pack(pack::Args),
    /// Emit the artefact JSON Schemas, or check the committed ones for drift.
    Schema(schema::Args),
    /// Measure this project against the done criteria, without flattering it.
    Doctor(doctor::Args),
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let code = dispatch(&cli).unwrap_or_else(|error| {
        // The banner promises the command did nothing. Every write in VDS goes
        // through an atomic rename and every refusal happens before the first
        // one, so that promise is kept rather than hoped for.
        eprintln!("VDS REFUSED, and did nothing:");
        for line in error.to_string().lines() {
            eprintln!("  {line}");
        }
        error.exit_code()
    });
    ExitCode::from(u8::try_from(code).unwrap_or(EXIT_PRECONDITION as u8))
}

fn dispatch(cli: &Cli) -> Result<i32> {
    let ctx = Context {
        root: cli.root.clone(),
        json: cli.json,
    };
    match &cli.command {
        Command::Init(args) => init::run(&ctx, args),
        Command::Ledger(args) => ledger::run(&ctx, args),
        Command::Register(args) => register::run(&ctx, args),
        Command::Proof(args) => proof::run(&ctx, args),
        Command::Warrant(args) => warrant::run(&ctx, args),
        Command::Lock(args) => lock::run(&ctx, args),
        Command::Pack(args) => pack::run(&ctx, args),
        Command::Schema(args) => schema::run(&ctx, args),
        Command::Doctor(args) => doctor::run(&ctx, args),
    }
}

/// What every command needs from the invocation.
pub struct Context {
    pub root: Option<PathBuf>,
    pub json: bool,
}

impl Context {
    /// Discover the project, refusing a `--root` that does not exist.
    ///
    /// The retired tool treated `--root` as a place to START walking upward, so
    /// a mistyped root silently found an ANCESTOR project and wrote into it. A
    /// root the caller named and that does not exist is a refusal.
    pub fn project(&self) -> Result<vds_core::Project> {
        if let Some(root) = &self.root
            && !root.is_dir()
        {
            return Err(VdsError::precondition(format!(
                "--root {} does not exist. Refusing rather than searching upward from it: a \
                 mistyped root that silently finds an ancestor project writes the record into \
                 the wrong repository.",
                root.display()
            )));
        }
        vds_core::Project::discover(self.root.as_deref())
    }

    /// The directory `init` should scaffold into.
    pub fn init_root(&self) -> Result<PathBuf> {
        match &self.root {
            Some(root) => {
                if !root.is_dir() {
                    return Err(VdsError::precondition(format!(
                        "--root {} does not exist. `vds init` scaffolds into an existing \
                         directory; creating one silently would put a governance record \
                         somewhere nobody chose.",
                        root.display()
                    )));
                }
                Ok(root.clone())
            }
            None => std::env::current_dir().map_err(|e| VdsError::io("the current directory", e)),
        }
    }
}

pub const PASSED: i32 = EXIT_PASSED;
