//! VDS core: the artefact types, digests, identifiers and project discovery.
//!
//! **VDS decides nothing** (VDS S-1(2)). Nothing in this crate resolves a
//! contested question. It reads registrations, ledgers, locks and proof records,
//! it digests bytes, and it refuses what it cannot represent.
//!
//! **`.vds/` stores no design value** (VDS S-2(2)). There is no field anywhere
//! in [`types`] that can hold a colour, a length, a radius, a spacing step, a
//! font, a duration, an easing curve or a shadow. A REQUIREMENT is lawful
//! (`min_ratio: 3.0`, drawn from WCAG 2.2 SC 1.4.11); a REALISATION is not, and
//! has nowhere to live. The `no_stored_values` proof re-checks the bytes on disk
//! rather than trusting this paragraph, because a claim stated in prose and
//! enforced by discipline is precisely the defect class VDS exists to convert
//! into a failed proof (VDS S-1(4)).

pub mod config;
pub mod digest;
pub mod error;
pub mod ids;
pub mod project;
pub mod schema_util;
pub mod timestamp;
pub mod types;

pub use config::{
    Config, Governance, PathRole, Paths, ScreensConfig, SurfaceConfig, default_config,
};
pub use digest::{Digest, canonical_json, digest_rows};
pub use error::{EXIT_PASSED, EXIT_PRECONDITION, EXIT_VACUOUS, EXIT_VIOLATION, Result, VdsError};
pub use ids::{
    BreachId, ComponentId, DecisionId, PinId, ProofId, ScreenId, SubmissionId, WarrantId,
};
pub use project::{Project, write_atomically, write_text_atomically, yaml_files};
pub use timestamp::Timestamp;
pub use types::*;

/// Who is acting, for the `by` and `pinned_by` fields.
///
/// Read from the environment rather than inferred. An actor VDS guessed is an
/// actor nobody can be held to.
pub fn actor() -> String {
    std::env::var("VDS_ACTOR")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| std::env::var("USER").ok().filter(|s| !s.trim().is_empty()))
        .unwrap_or_else(|| "unknown".into())
}

/// The standing note carried on every self-issued permit, adopted from VJS
/// unchanged in form and meaning (VDS S-4(3)).
pub const SELF_ISSUE_NOTE: &str = "Self-issue proves the actor took the front door. It is not an \
external authority's approval, and it cannot satisfy a check reserved to the Sovereign or to a \
constituted bench.";

/// The note every recording of a warrant carries. VDS S-1(3): VDS may not grant
/// itself a warrant, and `warrant record` writes down a grant that happened
/// elsewhere.
pub const RECORDING_IS_NOT_GRANTING: &str = "STANDING NOTE: recording is not granting. This file \
asserts that a grant happened elsewhere and pins the evidence it was made on. If no such grant \
happened, this record is a false statement of the record, not a warrant.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actor_prefers_the_explicit_variable() {
        // SAFETY: single-threaded test, and the variable is read only here.
        unsafe {
            std::env::set_var("VDS_ACTOR", "a-named-actor");
        }
        assert_eq!(actor(), "a-named-actor");
        unsafe {
            std::env::remove_var("VDS_ACTOR");
        }
    }

    #[test]
    fn no_artefact_type_declares_a_property_that_is_a_realisation() {
        // A structural guard, not a style check. If someone adds a `colour`,
        // `fontFamily`, `duration`, `radius`, `shadow` or `easing` property to an
        // artefact type, `.vds/` has become a store and VDS S-2(2) is broken at
        // the type level, before any file is written. Checking the GENERATED
        // schema rather than the source means a property added through a nested
        // struct is caught too.
        let mut generator = schemars::r#gen::SchemaSettings::draft2019_09().into_generator();
        let schemas = [
            (
                "component-record",
                serde_json::to_string(&generator.root_schema_for::<ComponentRecord>()).unwrap(),
            ),
            (
                "warrant",
                serde_json::to_string(&generator.root_schema_for::<Warrant>()).unwrap(),
            ),
            (
                "proof-result",
                serde_json::to_string(&generator.root_schema_for::<ProofResult>()).unwrap(),
            ),
            (
                "pin",
                serde_json::to_string(&generator.root_schema_for::<Pin>()).unwrap(),
            ),
            (
                "submission",
                serde_json::to_string(&generator.root_schema_for::<Submission>()).unwrap(),
            ),
            (
                "enforcement-lock-entry",
                serde_json::to_string(&generator.root_schema_for::<LockEntry>()).unwrap(),
            ),
            // The screen record is the artefact most at risk of this, because
            // the arrangement it describes is naturally spoken about in widths:
            // the prior art it derives from records a frame's columns as
            // `[924, 420]`. It holds a COUNT.
            (
                "screen-record",
                serde_json::to_string(&generator.root_schema_for::<ScreenRecord>()).unwrap(),
            ),
        ];
        // Property NAMES only. A description may legitimately use the word
        // "colour" while explaining why there is no colour field.
        let forbidden = [
            "colour",
            "color",
            "hex",
            "rgb",
            "hsl",
            "oklch",
            "fontFamily",
            "fontSize",
            "lineHeight",
            "letterSpacing",
            "radius",
            "borderRadius",
            "shadow",
            "boxShadow",
            "duration",
            "easing",
            "cubicBezier",
            "spacing",
            "px",
            "rem",
            "opacity",
        ];
        for (name, schema) in &schemas {
            for property in &forbidden {
                assert!(
                    !schema.contains(&format!("\"{property}\":{{")),
                    "{name} declares a property named {property:?}, which is a realisation \
                     and forbidden by VDS S-2(4)"
                );
            }
        }
    }
}
