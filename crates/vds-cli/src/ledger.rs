//! `vds ledger`: regenerate a generated inventory.

use clap::{Args as ClapArgs, Subcommand};
use vds_core::Result;

use crate::{Context, PASSED};

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    which: Which,
}

#[derive(Subcommand)]
enum Which {
    /// The declared surface: every screen matching `[surface] screen_globs`.
    Screens,
}

pub fn run(ctx: &Context, args: &Args) -> Result<i32> {
    let project = ctx.project()?;
    match args.which {
        Which::Screens => {
            let (path, ledger) = vds_scan::write(&project)?;
            let components = ledger.component_references().count();
            let elements: usize = ledger
                .screens
                .iter()
                .flat_map(|s| &s.references)
                .filter(|r| r.kind == vds_scan::ReferenceKind::Element)
                .count();
            let unresolved = ledger
                .component_references()
                .filter(|(_, r)| r.import_path.is_none())
                .count();

            println!("wrote {}", project.rel(&path));
            println!("  screens:                 {}", ledger.screens.len());
            println!("  component references:    {components}");
            println!(
                "  bare element references: {elements}  (informational only, \
                 VDS S-9(10) RESERVED)"
            );
            if unresolved > 0 {
                println!(
                    "  unresolved imports:      {unresolved}  (defined locally, or bound by \
                     more than one import)"
                );
            }
            println!("  source_digest:           {}", ledger.source_digest);
            println!("  content_digest:          {}", ledger.content_digest);
            if ledger.screens.is_empty() {
                println!();
                println!(
                    "NOTE: the declared surface matched no file. Every VDS claim is bounded by \
                     this surface, so a surface of nothing makes every proof vacuous. Check \
                     [surface] screen_globs in .vds/config.toml."
                );
            }
            Ok(PASSED)
        }
    }
}
