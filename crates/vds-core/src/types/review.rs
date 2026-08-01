//! The visual review verdict: automated eyes, recorded, and ENUMERATED.
//!
//! Draft S-7D, ENACTMENT PENDING (SUBMISSION-VDS-016, and the route-scoping
//! amendment at SUBMISSION-VDS-018). The defect this closes is the founding one
//! of this whole lane: a migration shipped structurally-green pages that looked
//! nothing like their frames, under twenty-eight source-side gates of which none
//! read the ARTEFACT against the FRAME. This record is the verdict artefact of an
//! agent visual pass: what was looked at, WHICH PARTS of it were looked at, by
//! whom, what differed, and the hashes that make the verdict expire the moment
//! either side moves.
//!
//! # The second defect, and why this record grew two fields
//!
//! A family-level pass covered `/dashboards` and scored one route inside it
//! "capture debt, needs seeded data". The Principal then opened that one route
//! and found a counted facet row, a counted eyebrow, keyboard affordances,
//! date-group bands, rich row anatomy and a footer window statement in the
//! frame, none of them shipped, and none of them needing data to find. The
//! review was not wrong about the family; it never studied the route, and
//! nothing anywhere recorded that it had not. That is the same shape as a stale
//! hand census and as a depth-limited capture recording "no children" as fact:
//! ABSENCE OF STUDY IS INVISIBLE UNLESS THE INSTRUMENT ENUMERATES WHAT IT WAS
//! SUPPOSED TO STUDY.
//!
//! Two consequences, both structural rather than advisory:
//!
//!   1. [`VisualReviewRecord::routes`] is a LIST so that a grouped verdict is
//!      representable and can be refused BY NAME. A verdict naming more than one
//!      route confers coverage on none of them; it is recorded as evidence and
//!      establishes nothing. Modelling it as a single `String` would have made
//!      the family review unrepresentable in the record and perfectly invisible
//!      in the pipeline that produced it, which is how this defect happened.
//!   2. [`VisualReviewRecord::regions`] carries a per-region finding or an
//!      explicit "not examined". A review that examined two regions and one that
//!      examined seven are otherwise identical in the ledger, and triage that
//!      cannot be seen as triage is indistinguishable from completed work.
//!
//! # What the ENGINE owns, and what it does not
//!
//! The capture and review pipeline - rendering the route, screenshotting it,
//! exporting the frame image, running the reviewing agent - lives in the
//! consuming repo. VDS S-7(2)(1) forbids a network call (or a model call)
//! inside a proof, so the engine's whole jurisdiction is the RECORD: validate
//! it, store it, stale it, and ENUMERATE the estate it was supposed to cover.
//! A verdict is evidence only while:
//!
//!   - the shipped side's source hash still matches the screens ledger, and
//!   - the frame side's hash still matches the frames ledger, and
//!   - the frame carries authority: a sign-off row matching its CURRENT hash.
//!
//! Any of the three moving does not degrade the verdict, it ENDS it, visibly.
//! A silently green stale review is the exact instrument failure the geometry
//! amendment recorded as 561-pin-561.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::signoff::AuthorityVerdict;
use crate::digest::Digest;
use crate::error::{Result, VdsError};
use crate::ids::{ReviewId, SignoffId};
use crate::project::Project;
use crate::timestamp::Timestamp;

/// The bands of a screen a visual pass is expected to account for. CLOSED.
///
/// Closed for the reason [`super::SurfaceKind`] is: an open set of region names
/// lets one undifferentiated "the page" bucket back in, and "the page looks
/// fine" is exactly the finding that missed a counted facet row, a counted
/// eyebrow, date-group bands and a footer window statement on one route.
///
/// These are ANATOMY names and hold no design value (VDS S-2(4)): a region is a
/// place to look, never a size, a colour or a spacing.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ReviewRegion {
    /// The page header band: title, eyebrow, counts, primary actions.
    Header,
    /// The filter/facet band, and whatever counts it carries.
    Facets,
    /// The body rows or cards, and their internal anatomy.
    BodyRows,
    /// The side rail, where the frame draws one.
    Rail,
    /// The footer band, and any window or scope statement in it.
    Footer,
    /// The empty state, which is a drawn state and not an absence.
    EmptyState,
    /// Keyboard affordances drawn in the frame: focus rings, hints, shortcuts.
    Keyboard,
}

impl ReviewRegion {
    pub const ALL: [ReviewRegion; 7] = [
        ReviewRegion::Header,
        ReviewRegion::Facets,
        ReviewRegion::BodyRows,
        ReviewRegion::Rail,
        ReviewRegion::Footer,
        ReviewRegion::EmptyState,
        ReviewRegion::Keyboard,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ReviewRegion::Header => "header",
            ReviewRegion::Facets => "facets",
            ReviewRegion::BodyRows => "body_rows",
            ReviewRegion::Rail => "rail",
            ReviewRegion::Footer => "footer",
            ReviewRegion::EmptyState => "empty_state",
            ReviewRegion::Keyboard => "keyboard",
        }
    }

    pub fn parse(raw: &str) -> Option<ReviewRegion> {
        ReviewRegion::ALL.into_iter().find(|r| r.as_str() == raw)
    }
}

impl std::fmt::Display for ReviewRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What the reviewer found in one region, or that they did not look.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RegionFinding {
    pub region: ReviewRegion,
    /// Whether this region of the PAIR was actually compared.
    ///
    /// `false` is a first-class, recordable answer and is the whole point:
    /// "capture debt, needs seeded data" is a legitimate thing to conclude
    /// about a region, and it has to be readable as a region that was not
    /// studied rather than disappearing into a verdict about the page.
    pub examined: bool,
    /// The finding, or the reason the region was not examined. Required either
    /// way: a region row carrying neither is a row that says nothing.
    pub finding: String,
}

/// One recorded visual pass, against one frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct VisualReviewRecord {
    pub id: ReviewId,
    /// The route(s) this pass covered, as the screens ledger spells them.
    ///
    /// A LIST so a grouped verdict is representable and refusable by name. Read
    /// the module note: a single-route field would have made the family review
    /// that motivated this amendment impossible to write down and impossible to
    /// catch. Exactly one route confers coverage; more confers none.
    pub routes: Vec<String>,
    pub file_key: String,
    /// The frame reviewed against, in the `12:34` spelling.
    pub node_id: String,
    /// Hash of the shipped screenshot artefact itself.
    pub shipped_screenshot_digest: Digest,
    /// The screens-ledger digest of the route's SOURCE at review time. The
    /// shipped side's input hash: when the route's source moves, this no
    /// longer matches the ledger and the verdict is STALE.
    pub shipped_source_digest: Digest,
    /// Hash of the exported frame image the reviewer saw.
    pub frame_image_digest: Digest,
    /// The frame's content digest at review time, as the frames ledger
    /// computes it. The frame side's input hash.
    pub frame_digest: Digest,
    /// THE STAGE-1 CONTRACT VERSION this verdict was taken against: the
    /// sign-off row whose hash is the signed frame the reviewer compared to.
    ///
    /// The pipeline is four stages in fixed order - contract JSON, then Figma
    /// AND code built to it, then source-side gates, then this artefact-side
    /// visual check - and the stages must be LINKED, not parallel. A verdict
    /// that cannot name its contract version is how "we checked it" survives a
    /// contract change with nobody noticing: the frame is redrawn, the
    /// contract moves, and a verdict rendered against the old one keeps
    /// reading as coverage.
    ///
    /// `Option` because a `no_authority` verdict is precisely the state of
    /// having NO contract to cite, and forcing a citation there would teach a
    /// pipeline to invent one. Any other verdict without it is invalid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_signoff: Option<SignoffId>,
    /// The reviewer's verdict. `no_authority` is recordable - a pipeline that
    /// found the frame unsigned says so honestly - and the proof also DERIVES
    /// authority itself; a recorded `conform` against a frame the register
    /// shows unsigned is refused, not trusted.
    pub verdict: AuthorityVerdict,
    /// Which bands of the pair were examined, and what was found in each.
    ///
    /// Required non-empty. A verdict with no region checklist is a verdict
    /// about "the page", and a finding about the page cannot be told apart
    /// from a glance at the page.
    #[serde(default)]
    pub regions: Vec<RegionFinding>,
    /// What differed, one delta per line. Required non-empty for `deviate`
    /// and required EMPTY for `conform`: a verdict and its evidence may not
    /// disagree.
    #[serde(default)]
    pub deltas: Vec<String>,
    /// The reviewing agent's identity: model, harness, prompt version.
    pub reviewed_by: String,
    pub reviewed_at: Timestamp,
    #[serde(default)]
    pub basis: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl VisualReviewRecord {
    /// The single route this verdict confers coverage on, or `None`.
    ///
    /// `None` for zero routes and for MORE THAN ONE. A grouped verdict is not a
    /// weaker route verdict, it is a verdict about a different subject, and the
    /// caller must not be able to reach a route out of one by taking the first
    /// element.
    pub fn covered_route(&self) -> Option<&str> {
        match self.routes.as_slice() {
            [only] => Some(only.as_str()),
            _ => None,
        }
    }

    /// Why this record is invalid, or an empty list. Draft S-7D(7)-(8).
    ///
    /// Validation and not prose: the family review that motivated this
    /// amendment was a perfectly well-formed record under the previous shape,
    /// and prose saying "reviews should be per route" would have been just as
    /// true and just as silent.
    pub fn defects(&self) -> Vec<String> {
        let mut out = Vec::new();
        match self.routes.len() {
            0 => out.push(
                "names no route at all, so there is no subject: a verdict about nothing \
                 cannot be stale, cannot conform, and cannot be owed."
                    .to_owned(),
            ),
            1 => {}
            n => out.push(format!(
                "names {n} routes ({}). A verdict covering more than one route confers \
                 coverage on NONE of them: it is recorded as evidence and establishes \
                 nothing. This is the defect the amendment was written for - a family-level \
                 pass scored a route it never studied, and nothing recorded that it had not. \
                 Record one verdict per route.",
                self.routes.join(", ")
            )),
        }
        if self.contract_signoff.is_none() && !matches!(self.verdict, AuthorityVerdict::NoAuthority)
        {
            out.push(format!(
                "returns {} and names no contract version (contractSignoff is absent). A \
                 stage-4 verdict cites the stage-1 contract it was taken against, or it \
                 cannot be told apart from a verdict rendered against a contract that has \
                 since changed. Only a no_authority verdict may omit it, because that is \
                 the state of having no contract to cite.",
                self.verdict
            ));
        }
        if self.regions.is_empty() {
            out.push(
                "carries an empty region checklist. A verdict about \"the page\" cannot be \
                 told apart from a glance at the page, and a review that examined two bands \
                 must not read identically to one that examined seven. List every region \
                 looked at, and every region NOT looked at with the reason."
                    .to_owned(),
            );
        }
        for finding in &self.regions {
            if finding.finding.trim().is_empty() {
                out.push(format!(
                    "region {} carries an empty finding. An examined region with nothing \
                     recorded is a claim with no content; an unexamined one with no reason \
                     hides why it was skipped.",
                    finding.region
                ));
            }
        }
        let mut seen: Vec<ReviewRegion> = self.regions.iter().map(|r| r.region).collect();
        seen.sort();
        let before = seen.len();
        seen.dedup();
        if seen.len() != before {
            out.push(
                "lists a region twice. Two findings for one band is two answers, and \
                 nothing says which governs."
                    .to_owned(),
            );
        }
        out
    }

    /// The regions of the closed set this record accounts for in neither
    /// direction: not examined, and not recorded as unexamined.
    ///
    /// Warned rather than refused. An estate whose frames genuinely have no
    /// rail should not be forced to write a row denying one; what must not
    /// happen is that the omission is invisible.
    pub fn unaccounted_regions(&self) -> Vec<ReviewRegion> {
        ReviewRegion::ALL
            .into_iter()
            .filter(|region| !self.regions.iter().any(|r| &r.region == region))
            .collect()
    }

    pub fn examined_count(&self) -> usize {
        self.regions.iter().filter(|r| r.examined).count()
    }
}

// ------------------------------------------------------------ the manifest

pub const ROUTE_MANIFEST_SCHEMA_VERSION: u32 = 1;

/// The estate's own list of routes a visual pass is expected to cover.
///
/// Supplied by the subject and never derived here, for the reason the geometry
/// reading is supplied: WHICH routes are in the programme is the estate's
/// question (in the motivating project it is a route tracker), and VDS deciding
/// it would make VDS the authority on the estate's own scope. What VDS owns is
/// that the enumeration EXISTS and that the proof reports every one of its
/// entries in one of three populations.
///
/// A ledger under VDS S-4(2): generated, digest-witnessed, byte-reproducible by
/// the named command. Without the digest, a route that is going unreviewed can
/// be cured by deleting its line, and the coverage report shrinks to fit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RouteManifest {
    pub schema_version: u32,
    pub generated_by: String,
    pub taken_at: Timestamp,
    /// Where the estate's list came from, in the estate's own words: a tracker,
    /// a query, a file. Named so a reader can tell a full estate from a slice.
    pub source: String,
    /// Every route in the programme, as the screens ledger spells them.
    pub routes: Vec<String>,
    /// What the estate knows this manifest does NOT cover, in its own words.
    #[serde(default)]
    pub does_not_cover: Vec<String>,
    pub content_digest: Digest,
}

impl RouteManifest {
    pub fn compute_content_digest(&self) -> Result<Digest> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Content<'a> {
            schema_version: u32,
            generated_by: &'a str,
            taken_at: &'a Timestamp,
            source: &'a str,
            routes: &'a [String],
            does_not_cover: &'a [String],
        }
        Digest::of_value(&Content {
            schema_version: self.schema_version,
            generated_by: &self.generated_by,
            taken_at: &self.taken_at,
            source: &self.source,
            routes: &self.routes,
            does_not_cover: &self.does_not_cover,
        })
    }

    pub fn untrustworthy_because(&self) -> Result<Option<String>> {
        let recomputed = self.compute_content_digest()?;
        Ok((recomputed != self.content_digest).then(|| {
            format!(
                "the manifest's contentDigest is {} and its content digests to {recomputed}. \
                 It was edited after it was generated. A route quietly removed from the \
                 manifest is a route that stops being reported as owed, which is the \
                 narrowing this ledger exists to prevent. Regenerate it rather than \
                 correcting the digest by hand.",
                self.content_digest
            )
        }))
    }
}

pub fn route_manifest_path(project: &Project) -> std::path::PathBuf {
    project.root.join(&project.config.review.route_manifest)
}

pub fn write_route_manifest(
    project: &Project,
    manifest: &RouteManifest,
) -> Result<std::path::PathBuf> {
    let path = route_manifest_path(project);
    let text = serde_yaml::to_string(manifest).map_err(|e| VdsError::Serialize {
        what: "the route manifest".into(),
        message: e.to_string(),
    })?;
    crate::write_text_atomically(&path, &text)?;
    Ok(path)
}

/// Read the route manifest, or `None` where the estate has supplied none.
pub fn read_route_manifest(project: &Project) -> Result<Option<RouteManifest>> {
    let path = route_manifest_path(project);
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
    if found > ROUTE_MANIFEST_SCHEMA_VERSION {
        return Err(VdsError::SchemaVersionAhead {
            path: project.rel(&path),
            kind: "route manifest",
            found,
            understood: ROUTE_MANIFEST_SCHEMA_VERSION,
        });
    }
    let manifest: RouteManifest = serde_yaml::from_value(raw).map_err(|e| VdsError::Artefact {
        path: project.rel(&path),
        message: format!("is not a route manifest: {e}"),
    })?;
    Ok(Some(manifest))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(region: ReviewRegion, examined: bool) -> RegionFinding {
        RegionFinding {
            region,
            examined,
            finding: if examined {
                "matches the frame".to_owned()
            } else {
                "not examined: the capture had no seeded data".to_owned()
            },
        }
    }

    fn record(routes: &[&str], regions: Vec<RegionFinding>) -> VisualReviewRecord {
        VisualReviewRecord {
            id: ReviewId::parse("VRW-0001").unwrap(),
            routes: routes.iter().map(|r| (*r).to_owned()).collect(),
            file_key: "KEY".into(),
            node_id: "1:2".into(),
            shipped_screenshot_digest: Digest::of_text("png"),
            shipped_source_digest: Digest::of_text("row"),
            frame_image_digest: Digest::of_text("frame-png"),
            frame_digest: Digest::of_text("frame"),
            contract_signoff: Some(SignoffId::parse("SGN-0001").unwrap()),
            verdict: AuthorityVerdict::Deviate,
            regions,
            deltas: vec!["the counted facet row is not shipped".into()],
            reviewed_by: "claude-fable-5 visual pass v1".into(),
            reviewed_at: Timestamp::fixed(2026, 8, 1, 10, 0, 0),
            basis: vec!["draft S-7D".into()],
            notes: None,
        }
    }

    #[test]
    fn a_review_record_round_trips_and_refuses_unknown_fields() {
        let r = record(
            &["app/dash/page.tsx"],
            vec![region(ReviewRegion::Header, true)],
        );
        let text = serde_yaml::to_string(&r).unwrap();
        assert_eq!(
            serde_yaml::from_str::<VisualReviewRecord>(&text).unwrap(),
            r
        );
        let with_surprise = format!("{text}surprise: 1\n");
        assert!(serde_yaml::from_str::<VisualReviewRecord>(&with_surprise).is_err());
    }

    #[test]
    fn one_route_confers_coverage_and_more_than_one_confers_none() {
        let single = record(
            &["app/dash/page.tsx"],
            vec![region(ReviewRegion::Header, true)],
        );
        assert_eq!(single.covered_route(), Some("app/dash/page.tsx"));
        assert!(single.defects().is_empty());

        // The motivating record: a family pass naming several routes.
        let grouped = record(
            &["app/dash/inbox/page.tsx", "app/dash/outbox/page.tsx"],
            vec![region(ReviewRegion::Header, true)],
        );
        assert_eq!(
            grouped.covered_route(),
            None,
            "a grouped verdict must not be reachable as a route verdict by taking the first \
             element"
        );
        let defects = grouped.defects();
        assert!(
            defects
                .iter()
                .any(|d| d.contains("confers coverage on NONE")),
            "{defects:?}"
        );
    }

    #[test]
    fn an_empty_region_checklist_is_a_defect() {
        let r = record(&["app/dash/page.tsx"], vec![]);
        let defects = r.defects();
        assert!(
            defects.iter().any(|d| d.contains("empty region checklist")),
            "{defects:?}"
        );
    }

    #[test]
    fn an_unexamined_region_is_recordable_and_visible_as_triage() {
        let r = record(
            &["app/dash/page.tsx"],
            vec![
                region(ReviewRegion::Header, true),
                region(ReviewRegion::BodyRows, false),
            ],
        );
        assert!(r.defects().is_empty(), "not-examined is a lawful answer");
        assert_eq!(
            r.examined_count(),
            1,
            "a review that examined one band must not count as two"
        );
        assert!(r.unaccounted_regions().contains(&ReviewRegion::Facets));
    }

    #[test]
    fn a_region_row_with_no_finding_is_a_defect_in_both_directions() {
        let mut r = record(
            &["app/dash/page.tsx"],
            vec![region(ReviewRegion::Header, true)],
        );
        r.regions[0].finding = "   ".into();
        assert!(
            r.defects().iter().any(|d| d.contains("empty finding")),
            "{:?}",
            r.defects()
        );
    }

    #[test]
    fn a_repeated_region_is_two_answers_and_is_refused() {
        let r = record(
            &["app/dash/page.tsx"],
            vec![
                region(ReviewRegion::Header, true),
                region(ReviewRegion::Header, false),
            ],
        );
        assert!(
            r.defects()
                .iter()
                .any(|d| d.contains("lists a region twice")),
            "{:?}",
            r.defects()
        );
    }

    #[test]
    fn an_edited_manifest_is_untrustworthy_by_its_own_digest() {
        let mut manifest = RouteManifest {
            schema_version: ROUTE_MANIFEST_SCHEMA_VERSION,
            generated_by: "vds ledger routes --from -".into(),
            taken_at: Timestamp::fixed(2026, 8, 1, 10, 0, 0),
            source: "the estate's route tracker".into(),
            routes: vec!["app/dash/inbox/page.tsx".into(), "app/dash/page.tsx".into()],
            does_not_cover: vec![],
            content_digest: Digest::of_text("placeholder"),
        };
        manifest.content_digest = manifest.compute_content_digest().unwrap();
        assert!(manifest.untrustworthy_because().unwrap().is_none());
        // The realistic edit: a route going unreviewed is deleted from the list.
        manifest.routes.remove(0);
        assert!(manifest.untrustworthy_because().unwrap().is_some());
    }
}
