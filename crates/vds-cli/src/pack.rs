//! `vds pack`: the vendored designpack.
//!
//! VDS S-11(1): adoption is vendored, read-only, digest-pinned and fail-closed,
//! and the runtime never fetches doctrine. VDS S-11(3): a digest bump is a
//! deliberate recorded act, and no doctrine flows downstream by silence.

use clap::{Args as ClapArgs, Subcommand};
use vds_core::{EXIT_VIOLATION, Result, actor};
use vds_designpack::PackVerdict;

use crate::{Context, PASSED};

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Recompute the vendored pack's digest and compare it to the lock.
    Verify,
    /// Pin whatever is vendored right now.
    Pin,
}

pub fn run(ctx: &Context, args: &Args) -> Result<i32> {
    let project = ctx.project()?;
    match args.action {
        Action::Verify => {
            let verdict = vds_designpack::verify(&project)?;
            if let Some(lock) = vds_designpack::read_lock(&project)? {
                println!("designpack: {}@{}", lock.designpack_id, lock.designpack_version);
                println!("  pinned:  {}", lock.digest);
                println!("  in force:{}", vds_designpack::digest_in_force(&project)?);
                println!("  locked by {} at {}", lock.locked_by, lock.generated_at);
                if lock.is_absent() {
                    println!();
                    println!(
                        "This project pins the ABSENCE of a designpack. VDS S-15(1): the \
                         specification commences on a dated, digest-pinned assent event in \
                         designpack/v1/provenance/assent/. Until then no warrant may be \
                         granted, because there is nothing to grant one under."
                    );
                }
            }
            println!();
            println!("{verdict}");
            Ok(match verdict {
                PackVerdict::Drifted { .. } => EXIT_VIOLATION,
                _ => PASSED,
            })
        }
        Action::Pin => {
            let previous = vds_designpack::read_lock(&project)?;
            let lock = vds_designpack::pin(&project, &actor())?;
            let path = vds_designpack::write_lock(&project, &lock)?;
            println!("pinned {}@{}", lock.designpack_id, lock.designpack_version);
            println!("  digest: {}", lock.digest);
            println!("  wrote {}", project.rel(&path));
            if let Some(previous) = previous
                && previous.digest != lock.digest
            {
                println!();
                println!("  superseded: {}", previous.digest);
                println!(
                    "  VDS S-11(3): a digest bump is a deliberate recorded act. Self-file the \
                     reason under VDS S-12(3), or the record says doctrine changed and not why."
                );
            }
            Ok(PASSED)
        }
    }
}
