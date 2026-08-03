//! Identifiers, and how they are allocated.
//!
//! VDS S-4(4): every identifier is allocated by reading the live record off disk
//! and taking the maximum plus one. No identifier may be asserted by hand or held
//! in memory across a run, and a collision is a fail-closed validation error and
//! never a silent overwrite. VJS deleted an in-memory citation registry for
//! exactly this defect: it restarted every series at genesis.
//!
//! The allocators here therefore take a directory and read it. There is no
//! counter, no cache and no "next id" field anywhere in VDS, because a counter
//! that can be wrong is a counter that will be.

use std::fmt;
use std::path::Path;

use schemars::JsonSchema;
use schemars::r#gen::SchemaGenerator;
use schemars::schema::Schema;
use serde::{Deserialize, Serialize};

use crate::error::{Result, VdsError};
use crate::schema_util::pattern_string;
use crate::timestamp::Timestamp;

/// Declares a `String` newtype whose values must match one anchored pattern, with
/// a `JsonSchema` impl carrying that same pattern, so the parser and the
/// published schema cannot drift apart.
macro_rules! lexical_id {
    ($name:ident, $pattern:expr, $description:expr) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Parse, refusing anything the pattern does not admit.
            pub fn parse(raw: impl Into<String>) -> Result<Self> {
                let raw = raw.into();
                if Self::pattern().is_match(&raw) {
                    Ok(Self(raw))
                } else {
                    Err(VdsError::Identifier(format!(
                        "{raw:?} is not a well-formed {}: it must match {}",
                        stringify!($name),
                        $pattern
                    )))
                }
            }

            pub fn pattern() -> &'static regex::Regex {
                static PATTERN: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
                PATTERN.get_or_init(|| regex::Regex::new($pattern).expect("static pattern"))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(
                deserializer: D,
            ) -> std::result::Result<Self, D::Error> {
                let raw = String::deserialize(deserializer)?;
                Self::parse(raw).map_err(serde::de::Error::custom)
            }
        }

        impl JsonSchema for $name {
            fn schema_name() -> String {
                stringify!($name).to_owned()
            }
            fn json_schema(_: &mut SchemaGenerator) -> Schema {
                pattern_string($pattern, $description)
            }
        }
    };
}

lexical_id!(
    ComponentId,
    r"^CMP-[0-9]{4}$",
    "A registered component, allocated as the highest on disk plus one (VDS S-4(4))."
);
lexical_id!(
    ScreenId,
    r"^SCR-[0-9]{4}$",
    "A registered screen: one route's ARRANGEMENT requirement, allocated as the highest on disk plus one (VDS S-4(4))."
);
lexical_id!(
    GeometryId,
    r"^GEO-[0-9]{4}$",
    "A bound on how many surfaces of one SHAPE do not comply, and the direction it must travel (VDS S-7A)."
);
lexical_id!(
    WarrantId,
    r"^WARRANT-W[1-4]-[0-9]{3}$",
    "A warrant record. The stage number is part of the identifier so the four series are independent."
);
lexical_id!(
    ProofId,
    r"^PROOF-[0-9]{8}-[0-9]{6}$",
    "A captured proof result, stamped with the UTC second it was captured."
);
lexical_id!(
    PinId,
    r"^PIN-[0-9]{8}-[0-9]{6}$",
    "A derived one-way agreement assertion between two named records."
);
lexical_id!(
    BreachId,
    r"^BREACH-[0-9]{4}$",
    "A self-reported breach (VDS S-12(3)). Numbered rather than stamped, because a breach is a thing somebody files and refers to, not an event a machine emits."
);
lexical_id!(
    DecisionId,
    r"^DECISION-[0-9]{4}$",
    "A decisive call disposed without a sitting (VDS S-12(2))."
);
lexical_id!(
    SubmissionId,
    r"^SUBMISSION-(VDS-[0-9]{3}|[0-9]{4}-[0-9]{2}-[0-9]{2}-[0-9]{6})$",
    "A question referred to VJS. The VDS-nnn series is reserved for the matters at VDS S-13."
);
lexical_id!(
    ProhibitionId,
    r"^PRB-[0-9]{4}$",
    "A prohibition: a pattern asserted ABSENT from an enumerated scope (draft S-7B)."
);
lexical_id!(
    BurndownId,
    r"^BRN-[0-9]{4}$",
    "A burndown: a pinned numeric reading whose only lawful direction is down (draft S-7C)."
);
lexical_id!(
    SignoffId,
    r"^SGN-[0-9]{4}$",
    "A frame sign-off: the frame's content hash at the moment taste was exercised (draft S-7D)."
);
lexical_id!(
    RedrawId,
    r"^RDW-[0-9]{4}$",
    "A proposed redraw: a deviation routed back through the design, never through an exception (draft S-7D)."
);
lexical_id!(
    DirectionId,
    r"^DIR-[0-9]{4}$",
    "A registered Principal direction: the sign-off register's second row kind, hash-bound to its logged decision ([2026] VJS-CA-VDS 1 order 26)."
);
lexical_id!(
    ReviewId,
    r"^VRW-[0-9]{4}$",
    "A visual review verdict: automated eyes over a shipped screen against its signed frame (draft S-7D)."
);
lexical_id!(
    StageId,
    r"^STG-[0-9]{4}$",
    "A staged write to one frame: the reviewable record of what was intended, which gates read on it, and what an apply then did (draft S-7E)."
);

/// The allocator every simple numbered series shares: highest on disk plus one
/// (VDS S-4(4)), refusing exhaustion rather than wrapping.
macro_rules! numbered_series {
    ($name:ident, $prefix:literal, $what:literal) => {
        impl $name {
            pub fn allocate(dir: &Path) -> Result<Self> {
                let highest = highest_numbered(dir, |stem| {
                    stem.strip_prefix($prefix)
                        .filter(|rest| rest.len() == 4)
                        .and_then(|rest| rest.parse::<u32>().ok())
                })?;
                if highest >= 9999 {
                    return Err(VdsError::Identifier(format!(
                        "the {} id space {}0001..{}9999 is exhausted. Widening it is an \
                         amendment to the schema, not a change to this allocator.",
                        $what, $prefix, $prefix
                    )));
                }
                Self::parse(format!("{}{:04}", $prefix, highest + 1))
            }
        }
    };
}

numbered_series!(ProhibitionId, "PRB-", "prohibition");
numbered_series!(BurndownId, "BRN-", "burndown");
numbered_series!(SignoffId, "SGN-", "sign-off");
numbered_series!(RedrawId, "RDW-", "redraw");
numbered_series!(ReviewId, "VRW-", "visual review");
numbered_series!(DirectionId, "DIR-", "direction");
numbered_series!(StageId, "STG-", "staged write");

impl ComponentId {
    /// The next free component id, read off disk. VDS S-4(4).
    pub fn allocate(register_dir: &Path) -> Result<Self> {
        let highest = highest_numbered(register_dir, |stem| {
            stem.strip_prefix("CMP-")
                .filter(|rest| rest.len() == 4)
                .and_then(|rest| rest.parse::<u32>().ok())
        })?;
        if highest >= 9999 {
            return Err(VdsError::Identifier(
                "the component id space CMP-0001..CMP-9999 is exhausted. Widening it is an \
                 amendment to the component-record schema, not a change to this allocator."
                    .into(),
            ));
        }
        Self::parse(format!("CMP-{:04}", highest + 1))
    }
}

impl ScreenId {
    /// The next free screen id, read off disk. VDS S-4(4).
    ///
    /// A separate series from `CMP-`, and not a suffix on it, because a screen
    /// and a component are different subjects: a screen record holds no props,
    /// no states and no contrast floor, and numbering them together would make
    /// "how many components are registered" a question nobody could answer by
    /// counting.
    pub fn allocate(screens_dir: &Path) -> Result<Self> {
        let highest = highest_numbered(screens_dir, |stem| {
            stem.strip_prefix("SCR-")
                .filter(|rest| rest.len() == 4)
                .and_then(|rest| rest.parse::<u32>().ok())
        })?;
        if highest >= 9999 {
            return Err(VdsError::Identifier(
                "the screen id space SCR-0001..SCR-9999 is exhausted. Widening it is an \
                 amendment to the screen-record schema, not a change to this allocator."
                    .into(),
            ));
        }
        Self::parse(format!("SCR-{:04}", highest + 1))
    }
}

impl GeometryId {
    /// The next free geometry-bound id, read off disk. VDS S-4(4).
    ///
    /// Its own series, and not a suffix on `CMP-`, for the reason `SCR-` is its
    /// own: a bound is a different subject from a component. There are at most
    /// four live bounds, one per surface kind (VDS S-7A(3)), so the series is
    /// tiny by construction - but superseded bounds are kept, and a project that
    /// re-baselines its shape backlog every quarter accumulates them.
    pub fn allocate(geometry_dir: &Path) -> Result<Self> {
        let highest = highest_numbered(geometry_dir, |stem| {
            stem.strip_prefix("GEO-")
                .filter(|rest| rest.len() == 4)
                .and_then(|rest| rest.parse::<u32>().ok())
        })?;
        if highest >= 9999 {
            return Err(VdsError::Identifier(
                "the geometry id space GEO-0001..GEO-9999 is exhausted. Widening it is an \
                 amendment to the geometry-bound schema, not a change to this allocator."
                    .into(),
            ));
        }
        Self::parse(format!("GEO-{:04}", highest + 1))
    }
}

impl WarrantId {
    /// The next free warrant id for one stage. The four stage series are
    /// independent, so a W2 does not consume a W1 number.
    pub fn allocate(warrants_dir: &Path, stage: u8) -> Result<Self> {
        let prefix = format!("WARRANT-W{stage}-");
        let highest = highest_numbered(warrants_dir, |stem| {
            stem.strip_prefix(&prefix)
                .filter(|rest| rest.len() == 3)
                .and_then(|rest| rest.parse::<u32>().ok())
        })?;
        if highest >= 999 {
            return Err(VdsError::Identifier(format!(
                "the warrant id space {prefix}001..{prefix}999 is exhausted"
            )));
        }
        Self::parse(format!("{prefix}{:03}", highest + 1))
    }
}

impl ProofId {
    /// A proof id for `at`, stepping forward one second at a time until a free
    /// slot is found.
    ///
    /// The schema fixes the shape at `PROOF-<date>-<time>`, so a suffix is not
    /// available to break a tie and the second is the only free dimension. Two
    /// proofs captured inside one second therefore get consecutive ids, and the
    /// second of them carries an id one second ahead of its own `captured_at`.
    /// That is stated here rather than hidden: the id is an identifier and the
    /// timestamp is the measurement, and only the timestamp is evidence.
    pub fn allocate(proofs_dir: &Path, at: &Timestamp) -> Result<Self> {
        for bump in 0..3600i64 {
            let candidate = at.plus_seconds(bump);
            let id = Self::parse(candidate.format("PROOF-%Y%m%d-%H%M%S"))?;
            if !proofs_dir.join(format!("{id}.yaml")).exists() {
                return Ok(id);
            }
        }
        Err(VdsError::Identifier(
            "could not allocate a free proof id within an hour of now. That means 3600 proof \
             records already occupy the next hour of second-slots, which is not a collision \
             to route around."
                .into(),
        ))
    }
}

impl PinId {
    pub fn allocate(pins_dir: &Path, at: &Timestamp) -> Result<Self> {
        for bump in 0..3600i64 {
            let candidate = at.plus_seconds(bump);
            let id = Self::parse(candidate.format("PIN-%Y%m%d-%H%M%S"))?;
            if !pins_dir.join(format!("{id}.yaml")).exists() {
                return Ok(id);
            }
        }
        Err(VdsError::Identifier(
            "could not allocate a free pin id within an hour of now".into(),
        ))
    }
}

/// Read every `*.yaml` stem in `dir`, extract a number from each with `extract`,
/// and return the highest. An unreadable directory is zero, not an error: a
/// register that does not exist yet allocates CMP-0001.
///
/// A file whose stem does not parse is IGNORED for allocation but is not
/// silently fine: [`crate::ids::nonconforming`] reports them so a mis-named
/// record cannot hide from the maximum.
fn highest_numbered(dir: &Path, extract: impl Fn(&str) -> Option<u32>) -> Result<u32> {
    if !dir.is_dir() {
        return Ok(0);
    }
    let mut highest = 0;
    for entry in std::fs::read_dir(dir).map_err(|e| VdsError::io(dir.display(), e))? {
        let entry = entry.map_err(|e| VdsError::io(dir.display(), e))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            && let Some(number) = extract(stem)
        {
            highest = highest.max(number);
        }
    }
    Ok(highest)
}

/// Every `*.yaml` in `dir` whose stem is not a well-formed identifier of the
/// expected kind.
///
/// Allocation skips these, so without this report a file called
/// `button.yaml` sitting in the register would be invisible to the allocator
/// and visible to nothing else either.
pub fn nonconforming(dir: &Path, is_valid: impl Fn(&str) -> bool) -> Result<Vec<String>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| VdsError::io(dir.display(), e))? {
        let entry = entry.map_err(|e| VdsError::io(dir.display(), e))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            && !is_valid(stem)
        {
            out.push(
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default()
                    .to_owned(),
            );
        }
    }
    out.sort();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_refuses_a_malformed_identifier() {
        assert!(ComponentId::parse("CMP-1").is_err());
        assert!(ComponentId::parse("cmp-0001").is_err());
        assert!(ComponentId::parse("xCMP-0001").is_err());
        assert!(ComponentId::parse("CMP-0001x").is_err());
        assert!(ComponentId::parse("CMP-0001").is_ok());
    }

    #[test]
    fn warrant_ids_are_per_stage() {
        assert!(WarrantId::parse("WARRANT-W1-001").is_ok());
        assert!(WarrantId::parse("WARRANT-W5-001").is_err());
        assert!(WarrantId::parse("WARRANT-W1-1").is_err());
    }

    #[test]
    fn allocation_reads_the_live_directory() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(
            ComponentId::allocate(tmp.path()).unwrap().as_str(),
            "CMP-0001"
        );
        std::fs::write(tmp.path().join("CMP-0007.yaml"), "").unwrap();
        assert_eq!(
            ComponentId::allocate(tmp.path()).unwrap().as_str(),
            "CMP-0008",
            "allocation takes the maximum plus one, not the count plus one"
        );
    }

    /// The two series are independent, so registering a screen never consumes a
    /// component number and a directory of screens does not make the next
    /// component id jump.
    #[test]
    fn screen_ids_are_their_own_series() {
        assert!(ScreenId::parse("SCR-0001").is_ok());
        assert!(ScreenId::parse("SCR-1").is_err());
        assert!(ScreenId::parse("CMP-0001").is_err());

        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("CMP-0009.yaml"), "").unwrap();
        assert_eq!(
            ScreenId::allocate(tmp.path()).unwrap().as_str(),
            "SCR-0001",
            "a component in the directory is not a screen and must not move the screen series"
        );
        std::fs::write(tmp.path().join("SCR-0004.yaml"), "").unwrap();
        assert_eq!(ScreenId::allocate(tmp.path()).unwrap().as_str(), "SCR-0005");
    }

    #[test]
    fn allocation_ignores_a_gap_rather_than_reusing_it() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("CMP-0001.yaml"), "").unwrap();
        std::fs::write(tmp.path().join("CMP-0003.yaml"), "").unwrap();
        assert_eq!(
            ComponentId::allocate(tmp.path()).unwrap().as_str(),
            "CMP-0004",
            "an identifier is never reused, so the gap at 0002 stays a gap (VDS S-9(1))"
        );
    }

    #[test]
    fn a_nonconforming_filename_is_reported_rather_than_ignored() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("button.yaml"), "").unwrap();
        std::fs::write(tmp.path().join("CMP-0001.yaml"), "").unwrap();
        assert_eq!(
            nonconforming(tmp.path(), |s| ComponentId::parse(s).is_ok()).unwrap(),
            vec!["button.yaml".to_string()]
        );
    }

    #[test]
    fn proof_ids_step_forward_when_a_second_is_taken() {
        let tmp = tempfile::tempdir().unwrap();
        let at = Timestamp::parse("2026-07-25T10:00:00Z").unwrap();
        let first = ProofId::allocate(tmp.path(), &at).unwrap();
        assert_eq!(first.as_str(), "PROOF-20260725-100000");
        std::fs::write(tmp.path().join(format!("{first}.yaml")), "").unwrap();
        let second = ProofId::allocate(tmp.path(), &at).unwrap();
        assert_eq!(second.as_str(), "PROOF-20260725-100001");
    }

    #[test]
    fn a_deserialised_identifier_is_validated() {
        let bad: std::result::Result<ComponentId, _> = serde_json::from_str("\"nope\"");
        assert!(bad.is_err(), "deserialisation must not bypass the pattern");
    }
}
