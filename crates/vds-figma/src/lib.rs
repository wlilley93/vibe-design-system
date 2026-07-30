//! The Figma seam: the decided-target ledger, and the two projections of the
//! register that make the round trip governable.
//!
//! ```text
//!            vds brief                          vds impl <CMP-id>
//!   register --------->  a generating agent      register + figma ledger
//!                        draws into Figma   ---------------------------->
//!                              |                 an implementing agent
//!                              |                 writes the code
//!                              v                          |
//!                        vds figma pull                   v
//!                              |                    vds proof --all
//!                              v
//!                        .vds/ledgers/figma.yaml
//! ```
//!
//! Both directions are governed by the SAME register, which is the point. A
//! design brief and an implementation contract that came from different sources
//! would be two contracts, and the round trip would drift at the join.
//!
//! # Why the read is a ledger and not a proof
//!
//! VDS S-7(2)(1) requires a proof to be re-runnable and deterministic with no
//! network call. Reading Figma is a network call. So `vds figma pull` is a LEDGER
//! GENERATOR, run out of band, and the proofs read what it wrote and refuse it
//! when it is stale. That is the same arrangement the screens ledger uses, and it
//! is the only arrangement under which a proof can reach Figma at all.
//!
//! # What this crate will not do
//!
//! It will not read a design VALUE. No colour, length, radius, font, duration,
//! easing curve or shadow is read from the Figma file into any artefact here.
//! [2026] VJS-CC-OPBOX 3 D1 makes the Figma file the system of record for what is
//! decided; VDS reads it, records that a node EXISTS and what it DECLARES, and
//! never copies what it says. A brief that carried values would be VDS handing
//! design values down, which makes VDS the fourth authority CC-OPBOX 3 forbids.

pub mod brief;
pub mod contract;
pub mod frames;
pub mod ledger;
pub mod pin;
pub mod pull;

#[cfg(test)]
pub mod testing;

pub use brief::{DrawnSource, GenerationBrief, build as build_brief};
pub use contract::{ImplementationContract, build as build_contract};
pub use frames::{AuthorityBy, FrameLedger, FrameRow};
pub use ledger::{FigmaLedger, FigmaNodeRow, check_fresh};
pub use pull::{FigmaApi, FigmaSource, SavedResponse, build_ledger, declared_file_key, from_saved};
