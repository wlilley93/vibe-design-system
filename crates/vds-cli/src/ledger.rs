//! `vds ledger`: regenerate a generated inventory.

use std::collections::{BTreeMap, BTreeSet};

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

            // A count is a number; a name repeated across screens is a work
            // list. VDS exists to confine design to the library, and a component
            // defined inline in a page is the most ordinary way that fails: it
            // is outside every proof, because no register coordinate can name
            // it, and nothing has ever said which ones they are.
            let mut inline: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
            for (screen, reference) in ledger.component_references() {
                if reference.import_path.is_none() {
                    inline
                        .entry(reference.root.as_str())
                        .or_default()
                        .insert(screen.route.as_str());
                }
            }
            if !inline.is_empty() {
                let mut ranked: Vec<(&&str, &BTreeSet<&str>)> = inline.iter().collect();
                ranked.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(b.0)));
                println!();
                println!(
                    "{} component names are defined inline rather than imported, across {} \
                     references.",
                    inline.len(),
                    unresolved
                );
                println!(
                    "  Each is outside every proof: no register coordinate can name a component \
                     that no module exports. The ones appearing on several screens are the \
                     candidates for the library."
                );
                for (name, routes) in ranked.iter().take(12) {
                    println!("    {:28} on {} screens", name, routes.len());
                }
                if ranked.len() > 12 {
                    println!("    ... and {} more", ranked.len() - 12);
                }
            }
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
