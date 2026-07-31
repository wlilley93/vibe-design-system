//! Geometry: the SHAPE of a surface, and the bound on how many surfaces do not
//! comply.
//!
//! # The defect this closes, which was observed and not imagined
//!
//! VDS S-7A(1). A design system is adopted in two parts and only one of them is
//! easy to measure. The PAINT, which token a surface references, is a NAME, and
//! a name is trivially checkable: every proof kind before this one reads names.
//! The SHAPE - corner radius, boundary weight, control density, spacing step,
//! type scale - is what a person actually sees, and nothing read it.
//!
//! So swapping one dark-neutral palette for another dark-neutral palette is a
//! real change that is nearly invisible, while the geometry carrying the visual
//! identity stays exactly where it was. On the subscriber project every
//! instrument reported progress: 95.6% token adoption, 193 of 199 routes, 0 owed
//! column deviations. The product looked substantially unchanged. Both were
//! true, and there was no instrument that could hold both facts at once.
//!
//! # Why a bound is not a design realisation
//!
//! VDS S-2(4) admits a REQUIREMENT and refuses a REALISATION, and
//! `no_stored_values` re-checks that claim against the bytes on disk rather than
//! trusting this comment. Nothing here is a length. A radius of `8px` is a
//! realisation and has nowhere to live in this module; the COUNT of surfaces
//! whose radius does not comply is a fact about conformance, in the same shape
//! S-2(6) settles for a contrast ratio and [`super::ArrangementContract`] settles
//! for a column count. Deleting every record in this module loses no shipped
//! pixel, and no reader can recover a design value from one.
//!
//! # Why the current bound is DERIVED and never stored beside the history
//!
//! [`GeometryBound`] carries a history and no `bound` field. The current bound is
//! the last entry. A second copy of a number that is already present is a copy
//! that drifts, and the two would then disagree with nothing saying which
//! governs. The derive-don't-store ratio is on all fours and is not
//! re-litigable.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::Status;
use crate::error::{Result, VdsError};
use crate::ids::GeometryId;
use crate::project::Project;
use crate::timestamp::Timestamp;

/// The shapes a bound may be declared over. CLOSED, and closed on purpose.
///
/// VDS S-7A(3): the bound is per SURFACE KIND and never one number for the
/// estate. The subscriber project's instrument reported "561 hand-rolled
/// card-geometry containers, pin 561", and that number names no work: it cannot
/// be assigned to anybody, it cannot be finished, and it hides which shapes are
/// worst. An enum rather than a free string is the strongest available form of
/// that rule, because one undifferentiated bucket is then unrepresentable rather
/// than merely discouraged.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceKind {
    /// Corner radius.
    Radius,
    /// The weight of a surface's boundary: a border, a ring, a divider.
    BoundaryWeight,
    /// Control density: the spacing step a surface sets its own padding and gaps
    /// from.
    Density,
    /// Type scale: the size and leading step a surface sets its text from.
    TypeScale,
}

impl SurfaceKind {
    pub const ALL: [SurfaceKind; 4] = [
        SurfaceKind::Radius,
        SurfaceKind::BoundaryWeight,
        SurfaceKind::Density,
        SurfaceKind::TypeScale,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            SurfaceKind::Radius => "radius",
            SurfaceKind::BoundaryWeight => "boundary_weight",
            SurfaceKind::Density => "density",
            SurfaceKind::TypeScale => "type_scale",
        }
    }

    pub fn parse(raw: &str) -> Option<SurfaceKind> {
        SurfaceKind::ALL.into_iter().find(|k| k.as_str() == raw)
    }
}

impl std::fmt::Display for SurfaceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What a reading was taken FROM.
///
/// VDS S-7A(4): geometry is read from the SHIPPED artefact, never from a model of
/// it. A code model of the intended design is a legitimate design tool and is NOT
/// admissible as the subject of this proof, because it is a third artefact that
/// drifts: on the subscriber project a 17-page code model of the design drifted
/// so completely that it came to model the OUTGOING system it was built to
/// replace.
///
/// # Why [`ReadFrom::CodeModel`] is representable at all
///
/// It would be stronger, on the face of it, to leave it out of the enum so a
/// code-model reading could not be written down. It is weaker. A generator that
/// read a code model would then have no way to say so truthfully, and its only
/// route to a parseable ledger would be to label the reading `ShippedSource`. The
/// variant exists so that the honest recorder produces a finding that NAMES the
/// problem, rather than a validation error that teaches mislabelling. The proof
/// refuses it; the type does not hide it.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ReadFrom {
    /// The stylesheet that ships: the built, compiled CSS a browser receives.
    /// The strongest reading, because it is what the user's machine renders.
    ShippedStylesheet,
    /// The source that compiles into what ships. Admissible: it is the shipping
    /// code, not a description of it. Weaker than the stylesheet by exactly the
    /// distance a build step can introduce, and a reading must say which it took
    /// so nobody has to guess.
    ShippedSource,
    /// A hand-authored model of the intended design. REFUSED by the proof
    /// (S-7A(4)). Representable so it can be refused by name.
    CodeModel,
}

impl ReadFrom {
    pub fn as_str(self) -> &'static str {
        match self {
            ReadFrom::ShippedStylesheet => "shipped_stylesheet",
            ReadFrom::ShippedSource => "shipped_source",
            ReadFrom::CodeModel => "code_model",
        }
    }

    /// Whether a reading taken this way is admissible as the subject of the
    /// geometry proof.
    pub fn is_shipped(self) -> bool {
        matches!(self, ReadFrom::ShippedStylesheet | ReadFrom::ShippedSource)
    }
}

impl std::fmt::Display for ReadFrom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One bound, as declared at one moment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BoundEntry {
    pub at: Timestamp,
    /// The largest number of non-compliant surfaces admitted from this moment.
    pub bound: u32,
    /// What lowered it, in one line. Printed in the finding when a bound has not
    /// moved, because "declare a lower bound" is advice and "the last three
    /// reductions came from these three pieces of work" is a plan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub because: Option<String>,
}

/// A bound on how many surfaces of one kind may fail to comply, and the
/// direction it must travel.
///
/// VDS S-7A(2) is the operative clause and it is why this type is not a ratchet.
/// The subscriber project HAD a shape instrument: a ratchet holding the count of
/// non-compliant containers at its current value so it could not rise. That is a
/// FLOOR, and a floor is a different instrument from a target. A number that may
/// only be held can never fall, and this one did not: it moved from 667 to 561
/// through work done for other reasons, then stopped. A ratchet that never
/// tightens is a record of a defect, presented as a control.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct GeometryBound {
    pub id: GeometryId,
    /// Which shape. One record per kind: S-7A(3).
    pub surface_kind: SurfaceKind,
    pub status: Status,
    /// How long the bound may stand without falling, in days.
    ///
    /// Declared by the project rather than fixed by VDS, because the rate a
    /// backlog can be worked down is a fact about the subject and not about the
    /// specification. What VDS fixes is that the window EXISTS and that expiry
    /// is fatal.
    pub declared_window_days: u32,
    /// Every bound ever declared for this kind, OLDEST FIRST.
    ///
    /// The current bound is the last entry, derived and never stored beside it.
    /// The history is the whole point: a direction is a claim about time, and a
    /// record holding only today's number cannot answer whether it fell.
    pub history: Vec<BoundEntry>,
    /// The authorities this bound rests on.
    #[serde(default)]
    pub basis: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl GeometryBound {
    /// The bound in force, or `None` where none was ever declared.
    pub fn current(&self) -> Option<&BoundEntry> {
        self.history.last()
    }

    /// Whether the history is in chronological order.
    ///
    /// Checked rather than assumed. Every question this type answers - did it
    /// fall, when did it last fall, what is in force - reads the LAST entry, so
    /// a history written out of order silently answers all three about the wrong
    /// moment.
    pub fn is_chronological(&self) -> bool {
        self.history
            .windows(2)
            .all(|w| w[0].at.as_str() <= w[1].at.as_str())
    }

    /// The most recent entry that was LOWER than the one before it, if any.
    ///
    /// The first entry is not a reduction. Declaring a bound for the first time
    /// is establishing the baseline, and counting it as a fall would let a
    /// project satisfy the direction rule by doing nothing but registering.
    pub fn last_reduction(&self) -> Option<&BoundEntry> {
        self.history
            .windows(2)
            .rev()
            .find(|w| w[1].bound < w[0].bound)
            .map(|w| &w[1])
    }
}

// ---------------------------------------------------------------- the reading

/// What one surface's shape turned out to be.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Compliance {
    /// The surface takes its shape from the design system.
    Complies,
    /// The surface sets its own shape.
    DoesNot,
    /// The reader could not resolve this surface's shape.
    ///
    /// The instrument saying "I DO NOT KNOW", and the reason this variant exists
    /// rather than the reader defaulting to one of the other two. Folding an
    /// unresolved surface into `Complies` turns a census into flattery; folding
    /// it into `DoesNot` cries wolf and gets the instrument ignored. Counted
    /// separately, and the proof treats a total that could cross the bound once
    /// the undecided are resolved as UNDECIDED rather than as a pass.
    Undecided,
}

/// One kind's reading: how many surfaces were looked at, and what they were.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct KindReading {
    pub surface_kind: SurfaceKind,
    /// Every surface the reader looked at, of this kind.
    pub considered: u32,
    /// Of those, how many do not comply.
    pub non_compliant: u32,
    /// Of those, how many the reader could not resolve.
    pub undecided: u32,
    /// Where the worst offenders are, so a row is a job somebody can pick up.
    /// Repository-relative, and a sample rather than the full list: a ledger
    /// carrying six hundred paths is a ledger nobody reads.
    #[serde(default)]
    pub sample: Vec<String>,
}

impl KindReading {
    /// How many surfaces are known to comply.
    ///
    /// Saturating, because the arithmetic is only sound if the three buckets
    /// partition `considered`. [`Self::buckets_partition`] is what checks that,
    /// and the proof refuses a reading where it does not hold rather than
    /// silently reporting a wrapped number.
    pub fn compliant(&self) -> u32 {
        self.considered
            .saturating_sub(self.non_compliant)
            .saturating_sub(self.undecided)
    }

    /// Whether the buckets add up.
    pub fn buckets_partition(&self) -> bool {
        self.non_compliant.saturating_add(self.undecided) <= self.considered
    }

    /// The largest the non-compliant count could turn out to be once every
    /// undecided surface is resolved. The number a bound must be compared
    /// against before a pass may be declared.
    pub fn worst_case(&self) -> u32 {
        self.non_compliant.saturating_add(self.undecided)
    }
}

pub const READING_SCHEMA_VERSION: u32 = 1;

/// A generated reading of the shipped artefact's geometry.
///
/// A ledger under VDS S-4(2): generated, never hand-edited, and byte-reproducible
/// by the named command. It is an INPUT to the proof and never a record of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct GeometryReading {
    pub schema_version: u32,
    /// The command that reproduces this file byte for byte.
    pub generated_by: String,
    /// When the reading was taken. The proof measures the bound's DIRECTION
    /// against this moment and never against the wall clock: a rule read from
    /// the system time would make a proof's findings change without any input
    /// changing, and VDS S-7(2)(1) requires an unchanged check to cite the same
    /// evidence.
    pub taken_at: Timestamp,
    /// What was read. S-7A(4).
    pub read_from: ReadFrom,
    /// The artefacts actually read, repository-relative. Named so a reader can
    /// tell a reading of the built bundle from a reading of one stylesheet that
    /// happens to sit beside it.
    #[serde(default)]
    pub sources: Vec<String>,
    pub kinds: Vec<KindReading>,
    /// What the reader knows it did not cover, in the reader's own words.
    ///
    /// Required to be stated, because a census that does not publish its own
    /// blind spots reads as complete.
    #[serde(default)]
    pub does_not_cover: Vec<String>,
}

impl GeometryReading {
    pub fn kind(&self, kind: SurfaceKind) -> Option<&KindReading> {
        self.kinds.iter().find(|k| k.surface_kind == kind)
    }
}

/// Where the geometry reading lives, per `[geometry] reading_ledger`.
pub fn reading_path(project: &Project) -> std::path::PathBuf {
    project.root.join(&project.config.geometry.reading_ledger)
}

pub fn write_reading(project: &Project, reading: &GeometryReading) -> Result<std::path::PathBuf> {
    let path = reading_path(project);
    let text = serde_yaml::to_string(reading).map_err(|e| VdsError::Serialize {
        what: "the geometry reading".into(),
        message: e.to_string(),
    })?;
    crate::write_text_atomically(&path, &text)?;
    Ok(path)
}

/// Read the geometry reading, or `None` where none has been generated.
///
/// The schema version is read from the RAW value before the typed parse, so a
/// ledger from a future build is refused rather than half-understood
/// (VDS S-11(2)). A loader that skipped the fields it could not parse would
/// compare a reading it only half read and call the difference conformance.
pub fn read_reading(project: &Project) -> Result<Option<GeometryReading>> {
    let path = reading_path(project);
    if !path.is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path).map_err(|e| VdsError::io(path.display(), e))?;
    let raw: serde_yaml::Value = serde_yaml::from_str(&text).map_err(|e| VdsError::Artefact {
        path: project.rel(&path),
        message: format!("is not readable YAML: {e}"),
    })?;
    let found = raw
        .get("schemaVersion")
        .or_else(|| raw.get("schema_version"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    if found > READING_SCHEMA_VERSION {
        return Err(VdsError::SchemaVersionAhead {
            path: project.rel(&path),
            kind: "geometry reading",
            found,
            understood: READING_SCHEMA_VERSION,
        });
    }
    let reading: GeometryReading =
        serde_yaml::from_value(raw).map_err(|e| VdsError::Artefact {
            path: project.rel(&path),
            message: format!("is not a geometry reading: {e}"),
        })?;
    Ok(Some(reading))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(at: &str, bound: u32) -> BoundEntry {
        BoundEntry {
            at: Timestamp::parse(at).expect("test timestamp"),
            bound,
            because: None,
        }
    }

    fn bound(history: Vec<BoundEntry>) -> GeometryBound {
        GeometryBound {
            id: GeometryId::parse("GEO-0001").expect("test id"),
            surface_kind: SurfaceKind::Radius,
            status: Status::Registered,
            declared_window_days: 30,
            history,
            basis: vec!["VDS S-7A(2)".into()],
            notes: None,
        }
    }

    #[test]
    fn the_current_bound_is_the_last_entry_and_is_not_stored_beside_it() {
        let b = bound(vec![
            entry("2026-07-01T00:00:00Z", 667),
            entry("2026-07-20T00:00:00Z", 561),
        ]);
        assert_eq!(b.current().map(|e| e.bound), Some(561));
    }

    #[test]
    fn a_first_declaration_is_a_baseline_and_never_a_reduction() {
        // Otherwise a project satisfies the direction rule by registering and
        // then doing nothing, which is the instrument the amendment refuses.
        let b = bound(vec![entry("2026-07-01T00:00:00Z", 667)]);
        assert!(b.last_reduction().is_none());
    }

    #[test]
    fn a_raise_after_a_fall_does_not_count_as_the_last_reduction() {
        let b = bound(vec![
            entry("2026-07-01T00:00:00Z", 667),
            entry("2026-07-10T00:00:00Z", 561),
            entry("2026-07-20T00:00:00Z", 600),
        ]);
        assert_eq!(
            b.last_reduction().map(|e| e.at.as_str().to_owned()),
            Some("2026-07-10T00:00:00Z".to_owned()),
            "the reduction is the entry that WENT DOWN, not the most recent entry"
        );
    }

    #[test]
    fn an_out_of_order_history_is_detectable() {
        let b = bound(vec![
            entry("2026-07-20T00:00:00Z", 561),
            entry("2026-07-01T00:00:00Z", 667),
        ]);
        assert!(!b.is_chronological());
    }

    #[test]
    fn the_worst_case_counts_every_undecided_surface_against_the_bound() {
        // The instrument saying "I do not know". 10 known bad and 5 unresolved
        // is not 10: it is anything up to 15, and a bound of 12 cannot be
        // declared met on this reading.
        let r = KindReading {
            surface_kind: SurfaceKind::Radius,
            considered: 100,
            non_compliant: 10,
            undecided: 5,
            sample: vec![],
        };
        assert_eq!(r.worst_case(), 15);
        assert_eq!(r.compliant(), 85);
        assert!(r.buckets_partition());
    }

    #[test]
    fn buckets_that_exceed_the_population_are_detectable_rather_than_wrapping() {
        let r = KindReading {
            surface_kind: SurfaceKind::Radius,
            considered: 10,
            non_compliant: 8,
            undecided: 5,
            sample: vec![],
        };
        assert!(!r.buckets_partition());
        assert_eq!(r.compliant(), 0, "saturating, never wrapped to 4294967293");
    }

    #[test]
    fn a_code_model_reading_is_representable_so_that_it_can_be_refused_by_name() {
        assert!(!ReadFrom::CodeModel.is_shipped());
        assert!(ReadFrom::ShippedStylesheet.is_shipped());
        assert!(ReadFrom::ShippedSource.is_shipped());
    }

    #[test]
    fn every_surface_kind_round_trips_through_its_wire_name() {
        for kind in SurfaceKind::ALL {
            assert_eq!(SurfaceKind::parse(kind.as_str()), Some(kind));
        }
    }
}
