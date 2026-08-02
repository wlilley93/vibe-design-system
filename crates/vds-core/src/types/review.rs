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
//! # The third defect: a conform verdict earned nothing
//!
//! The record above could be MINTED. Three of its four hashes are fields a
//! recorder types, and the proof's entire conform branch was one line marking
//! the row enforced, so a passing record with no screenshot on disk, no served
//! build and a checklist in which every row said `examined: false` conjured a
//! route at parity out of nothing. Each of those alone was enough, and three of
//! the four were demonstrated on the downstream estate by writing the record.
//! So [`VisualReviewRecord::unearned_conformance`] holds a conform verdict to
//! the evidence a conform verdict implies: a screenshot that still re-hashes,
//! a named build the picture was taken of, and at least one band actually
//! studied. It holds NO OTHER VERDICT to that standard, deliberately: a
//! deviation claims no parity, and making it expensive to record one is how an
//! estate stops recording them.
//!
//! # The fourth defect: every difference read as the same kind of thing
//!
//! [`VisualReviewRecord::deltas`] was a list of bare strings, so a frame drawn
//! before a product decision, an annotation the exporter drew, a band with no
//! seeded data and a genuine code defect were one population with one queue.
//! [`ReviewDelta`] classifies each one and makes the classes that assert work
//! lives elsewhere CITE the row that carries it. What it deliberately cannot do
//! is close anything: see [`DeltaDisposition`].
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

/// What a recorded difference IS, in the reviewer's own analysis. CLOSED.
///
/// A bare delta list answered "what differs" and nothing else, so every
/// difference read as one kind of thing: work owed by the code. On the estate
/// this lane was written for, that is false of most of them. A frame drawn
/// before a product decision, an annotation layer the exporter drew into the
/// image, a shipped surface the design has since preferred, a band that needs
/// data nobody seeded, chrome that belongs to the shell and not the route, and
/// a difference a policy forbids closing are six different facts, and a list
/// that cannot tell them apart routes all six to the same queue.
///
/// # There is no `accepted` and no `wont_fix`, and there never will be
///
/// An acceptance state is TASTE, and taste is exercised once, at frame sign-off
/// ([2026] VJS-SC-OPBOX 1, and the repeal recorded in
/// [`crate::AuthorityVerdict`]). A fourth value here would be a fourth
/// [`AuthorityVerdict`] wearing a different field name, reachable by the
/// recorder rather than by the signer, and it would be the whole repeal undone
/// one enum away from the one that carries it.
///
/// So EVERY disposition leaves the verdict `deviate` and leaves the deviation
/// rule RED. A disposition CLASSIFIES and never DISPOSES: it says what kind of
/// work the difference is, and the three routes out of a deviation are
/// unchanged (a covering sign-off, a registered direction, or a deletion that
/// independently discharges [2026] VJS-CC-OPBOX 155 O7).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum DeltaDisposition {
    /// The code does not draw what the signed frame draws. The default kind,
    /// and the only one that needs no citation: it is a claim about the two
    /// artefacts the verdict already names.
    CodeDefect,
    /// The frame is behind the product: the shipped surface implements a later
    /// decision the drawing has not caught up with. Owed by a redraw.
    FrameBehindProduct,
    /// The difference is in the FRAME IMAGE and not in the design: a redline, a
    /// spec callout, a comment pin the export drew. Owed by a redraw, because
    /// the cure is a clean frame and not a code change.
    FrameDrawsAnnotation,
    /// The shipped surface is preferred to the drawing. Still a deviation, and
    /// still red: the resolution is a redraw adopting it, never this field.
    ShippedIsBetter,
    /// The band could not be compared because the capture had no data in it.
    /// Needs no citation: it is a fact about the capture the verdict names.
    NeedsAbsentData,
    /// The difference is drawn by the application shell rather than by this
    /// route, so the subject of the finding is another screen. Owed by the
    /// SCREEN record that does own it.
    BelongsToAppChrome,
    /// Closing the difference is forbidden: a registered prohibition or a
    /// Principal direction stands in the way. Owed by that row.
    ForbiddenByPolicy,
}

impl DeltaDisposition {
    pub const ALL: [DeltaDisposition; 7] = [
        DeltaDisposition::CodeDefect,
        DeltaDisposition::FrameBehindProduct,
        DeltaDisposition::FrameDrawsAnnotation,
        DeltaDisposition::ShippedIsBetter,
        DeltaDisposition::NeedsAbsentData,
        DeltaDisposition::BelongsToAppChrome,
        DeltaDisposition::ForbiddenByPolicy,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            DeltaDisposition::CodeDefect => "code_defect",
            DeltaDisposition::FrameBehindProduct => "frame_behind_product",
            DeltaDisposition::FrameDrawsAnnotation => "frame_draws_annotation",
            DeltaDisposition::ShippedIsBetter => "shipped_is_better",
            DeltaDisposition::NeedsAbsentData => "needs_absent_data",
            DeltaDisposition::BelongsToAppChrome => "belongs_to_app_chrome",
            DeltaDisposition::ForbiddenByPolicy => "forbidden_by_policy",
        }
    }

    pub fn parse(raw: &str) -> Option<DeltaDisposition> {
        DeltaDisposition::ALL
            .into_iter()
            .find(|d| d.as_str() == raw)
    }

    /// The record series a delta of this kind must cite, or an empty slice.
    ///
    /// The citation is what makes the taxonomy load-bearing. Without it, seven
    /// dispositions are seven synonyms for "not now": a recorder types
    /// `frame_behind_product` and the difference leaves the queue with nothing
    /// anywhere naming the redraw that is supposed to bring it back. With it,
    /// every disposition that ASSERTS SOMETHING ELSE EXISTS has to name the
    /// something else, and the citation is checked against the register by the
    /// proof that reads it.
    ///
    /// Two kinds require nothing, and both for the same reason: they are claims
    /// about the artefacts the verdict already names, so there is no other row
    /// to point at.
    pub fn owed_by_prefixes(self) -> &'static [&'static str] {
        match self {
            DeltaDisposition::CodeDefect | DeltaDisposition::NeedsAbsentData => &[],
            DeltaDisposition::FrameBehindProduct
            | DeltaDisposition::FrameDrawsAnnotation
            | DeltaDisposition::ShippedIsBetter => &["RDW-"],
            DeltaDisposition::BelongsToAppChrome => &["SCR-"],
            DeltaDisposition::ForbiddenByPolicy => &["PRB-", "DIR-"],
        }
    }
}

impl std::fmt::Display for DeltaDisposition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One recorded difference: what it is, what KIND of thing it is, and the row
/// that carries the work where the kind asserts one exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ReviewDelta {
    /// The difference, in the reviewer's own words.
    ///
    /// Prose, and NOT exempt from VDS S-2(2): `.vds/**` is scanned in full by
    /// `no_stored_values` and the two ignored directories are closed at two
    /// (VDS S-3(9)), so a delta that spells a length, a duration or a colour
    /// literal puts that value under the record permanently. Name the CLASS -
    /// "the row gutter is tighter than the frame draws", "the border reads at
    /// a lower contrast than the floor" - exactly as every note this engine
    /// writes does.
    pub describes: String,
    /// What kind of difference this is. Required, and a non-`Option` field
    /// under `deny_unknown_fields`, so a record that omits it is refused by the
    /// DESERIALISER rather than by a rule: an unclassified delta is not a
    /// weaker delta, it is the old bare string back.
    pub disposition: DeltaDisposition,
    /// The row that carries the work, where the disposition asserts one exists.
    /// See [`DeltaDisposition::owed_by_prefixes`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owed_by: Option<String>,
}

impl ReviewDelta {
    /// Why this delta is invalid, or `None`.
    pub fn defect(&self) -> Option<String> {
        if self.describes.trim().is_empty() {
            return Some(format!(
                "carries a {} disposition and describes nothing. A classification with no \
                 finding under it classifies nothing.",
                self.disposition
            ));
        }
        let required = self.disposition.owed_by_prefixes();
        if required.is_empty() {
            return None;
        }
        let expected = required.join(" or ");
        match self.owed_by.as_deref().map(str::trim) {
            None | Some("") => Some(format!(
                "is dispositioned {} and cites no row (ownedBy is absent). That disposition \
                 asserts the work lives somewhere else, and a taxonomy whose kinds cite \
                 nothing is seven synonyms for \"not now\": cite the {expected} row that \
                 carries it. Delta: {}",
                self.disposition, self.describes
            )),
            Some(cited) if !required.iter().any(|p| cited.starts_with(p)) => Some(format!(
                "is dispositioned {} and cites {cited}, which is not a {expected} row. The \
                 citation names the record series that carries the work, and a citation \
                 pointing at the wrong series resolves to nothing on every run. Delta: {}",
                self.disposition, self.describes
            )),
            Some(_) => None,
        }
    }
}

impl std::fmt::Display for ReviewDelta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} [{}", self.describes, self.disposition)?;
        if let Some(owed_by) = &self.owed_by {
            write!(f, ", owed by {owed_by}")?;
        }
        f.write_str("]")
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
    /// Where the screenshot IS, project-relative, so the hash above can be
    /// RE-COMPUTED rather than believed.
    ///
    /// A conform verdict with no path rests on two hashes of three, and the
    /// missing one is the picture of the product: nothing in the record can be
    /// re-derived from the artefact, so the whole parity claim is a field
    /// somebody typed. Three of the four earning conditions were found on the
    /// downstream estate by MINTING a passing record, and each one alone
    /// conjured a route at parity out of nothing.
    ///
    /// `Option` because a `deviate` verdict claims no parity and must not be
    /// held to a parity standard: hold it there and people stop recording
    /// deviations, which costs more than the field buys.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screenshot_path: Option<String>,
    /// The build the screenshot was taken OF, in the estate's own spelling: a
    /// commit, a tag, an image digest.
    ///
    /// [`Self::shipped_source_digest`] is computed from the LOCAL TREE, so on
    /// its own it can describe code the screenshot never ran: a reviewer
    /// captures a deployed page, digests the working copy, and the record links
    /// a picture of one build to the hash of another. Naming the served build
    /// is the only thing in the record that says which one was on the screen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub served_build: Option<String>,
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
    /// What differed, one row per difference. Required non-empty for `deviate`
    /// and required EMPTY for `conform`: a verdict and its evidence may not
    /// disagree.
    ///
    /// Rows and not strings. A bare list answered "what differs" and left every
    /// difference reading as work owed by the code, which is false of most of
    /// them; see [`DeltaDisposition`] for what the classification buys and for
    /// why it can never dispose of anything.
    #[serde(default)]
    pub deltas: Vec<ReviewDelta>,
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
        // An undispositioned or uncited delta REFUSES THE RECORD, and the
        // consequence is the one that matters: an invalid record confers no
        // coverage, so its route goes back into the never-reviewed population
        // by name. A delta rule that only warned would leave the route reading
        // as covered by a verdict nobody can act on.
        for delta in &self.deltas {
            if let Some(defect) = delta.defect() {
                out.push(format!("a recorded delta {defect}"));
            }
        }
        out
    }

    /// The bands this record says it actually STUDIED.
    ///
    /// Study and not paperwork: a row with `examined: false` is a lawful and
    /// useful answer about a band and it is not a look at it.
    pub fn examined_regions(&self) -> Vec<ReviewRegion> {
        self.regions
            .iter()
            .filter(|r| r.examined)
            .map(|r| r.region)
            .collect()
    }

    /// Why this CONFORM verdict has not earned its parity claim, or an empty
    /// list. Draft S-7D(12).
    ///
    /// Empty for every verdict that is not `conform`, and that is deliberate: a
    /// `deviate` or `no_authority` verdict claims no parity, so holding it to a
    /// parity standard would make recording a deviation more expensive than
    /// recording nothing, and people stop recording deviations.
    ///
    /// `screenshot_rehashed` is the caller's answer to the one question this
    /// type cannot ask: does the file at [`Self::screenshot_path`] still digest
    /// to [`Self::shipped_screenshot_digest`]? A type that read the filesystem
    /// would be an artefact type doing IO, and a proof that trusted the field
    /// would be the whole defect again.
    pub fn unearned_conformance(&self, screenshot_rehashed: bool) -> Vec<String> {
        if !matches!(self.verdict, AuthorityVerdict::Conform) {
            return Vec::new();
        }
        let mut out = Vec::new();
        match &self.screenshot_path {
            None => out.push(
                "claims conformance and names no screenshot (screenshotPath is absent), so \
                 nothing in this record can be re-hashed. The verdict then rests on two hashes \
                 of three, and the missing one is the picture of the product: a record like \
                 this can be MINTED, and minting one is enough to conjure a route at parity \
                 out of nothing."
                    .to_owned(),
            ),
            Some(path) if !screenshot_rehashed => out.push(format!(
                "claims conformance and names {path}, which does not re-hash to \
                 {}. Either the file is missing or its bytes are not the ones reviewed, and \
                 in both cases the verdict describes a picture nobody can now produce.",
                self.shipped_screenshot_digest
            )),
            Some(_) => {}
        }
        if self.served_build.is_none() {
            out.push(
                "claims conformance and names no served build (servedBuild is absent). The \
                 shipped source digest is computed from the LOCAL TREE, so on its own it can \
                 describe code the screenshot never ran: a capture of a deployed page paired \
                 with the hash of a working copy is two builds in one record."
                    .to_owned(),
            );
        }
        if self.examined_count() == 0 {
            out.push(
                "claims conformance and examined NO band: every region row says examined: \
                 false. That record is well formed - it has a full checklist and a reason on \
                 every row - and it says nobody looked. A conform verdict over zero studied \
                 bands is the word \"green\" with a checklist stapled to it."
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
            screenshot_path: Some("design/captures/dash.png".into()),
            served_build: Some("build 3a4a316d".into()),
            contract_signoff: Some(SignoffId::parse("SGN-0001").unwrap()),
            verdict: AuthorityVerdict::Deviate,
            regions,
            deltas: vec![ReviewDelta {
                describes: "the counted facet row is not shipped".into(),
                disposition: DeltaDisposition::CodeDefect,
                owed_by: None,
            }],
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

    // -- draft S-7D(13): a delta is classified, and the class cites its row ----

    fn delta(disposition: DeltaDisposition, owed_by: Option<&str>) -> ReviewDelta {
        ReviewDelta {
            describes: "the facet row is drawn in the frame and not shipped".into(),
            disposition,
            owed_by: owed_by.map(str::to_owned),
        }
    }

    #[test]
    fn the_disposition_vocabulary_is_closed_at_seven_and_round_trips() {
        assert_eq!(
            DeltaDisposition::ALL.len(),
            7,
            "the vocabulary is CLOSED: a new kind is an amendment, not an edit"
        );
        for disposition in DeltaDisposition::ALL {
            assert_eq!(
                DeltaDisposition::parse(disposition.as_str()),
                Some(disposition),
                "{disposition} does not survive its own spelling"
            );
        }
        assert_eq!(DeltaDisposition::parse("accepted"), None);
        assert_eq!(
            DeltaDisposition::parse("wont_fix"),
            None,
            "an acceptance state is taste exercised after sign-off, and taste is exercised \
             once, at sign-off"
        );
    }

    /// THE SEED FOR THE CITATION LIMB: a disposition that asserts the work
    /// lives elsewhere, and nothing anywhere naming the elsewhere.
    #[test]
    fn a_disposition_that_asserts_another_row_must_cite_one() {
        let uncited = delta(DeltaDisposition::FrameBehindProduct, None);
        let defect = uncited.defect().expect("a missing citation is a defect");
        assert!(defect.contains("cites no row"), "{defect}");
        assert!(defect.contains("synonyms for"), "{defect}");
        assert!(
            delta(DeltaDisposition::FrameBehindProduct, Some("RDW-0004"))
                .defect()
                .is_none()
        );
    }

    #[test]
    fn a_citation_in_the_wrong_series_resolves_to_nothing_and_is_refused() {
        // A redraw citation where a screen record is owed: the row exists, and
        // it is not the row that carries this work.
        let defect = delta(DeltaDisposition::BelongsToAppChrome, Some("RDW-0004"))
            .defect()
            .expect("a wrong-series citation is a defect");
        assert!(defect.contains("not a SCR- row"), "{defect}");
        assert!(
            delta(DeltaDisposition::BelongsToAppChrome, Some("SCR-0007"))
                .defect()
                .is_none()
        );
        // Two lawful series for one disposition: a prohibition or a direction.
        assert!(
            delta(DeltaDisposition::ForbiddenByPolicy, Some("DIR-0001"))
                .defect()
                .is_none()
        );
        assert!(
            delta(DeltaDisposition::ForbiddenByPolicy, Some("PRB-0002"))
                .defect()
                .is_none()
        );
        assert!(
            delta(DeltaDisposition::ForbiddenByPolicy, Some("SGN-0001"))
                .defect()
                .is_some()
        );
    }

    #[test]
    fn the_two_dispositions_about_the_named_artefacts_need_no_citation() {
        // Both are claims about the two artefacts the verdict already names, so
        // there is no other row to point at and demanding one would teach a
        // pipeline to invent one.
        assert!(delta(DeltaDisposition::CodeDefect, None).defect().is_none());
        assert!(
            delta(DeltaDisposition::NeedsAbsentData, None)
                .defect()
                .is_none()
        );
        assert!(DeltaDisposition::CodeDefect.owed_by_prefixes().is_empty());
        assert!(
            DeltaDisposition::NeedsAbsentData
                .owed_by_prefixes()
                .is_empty()
        );
    }

    #[test]
    fn a_delta_that_describes_nothing_classifies_nothing() {
        let mut d = delta(DeltaDisposition::CodeDefect, None);
        d.describes = "   ".into();
        assert!(d.defect().unwrap().contains("describes nothing"));
    }

    #[test]
    fn an_uncited_delta_refuses_the_whole_record() {
        let mut r = record(
            &["app/dash/page.tsx"],
            vec![region(ReviewRegion::Header, true)],
        );
        r.deltas = vec![delta(DeltaDisposition::ShippedIsBetter, None)];
        assert!(
            r.defects().iter().any(|d| d.contains("cites no row")),
            "{:?}",
            r.defects()
        );
    }

    /// A missing disposition is a DESERIALISATION error, not a rule.
    ///
    /// `deny_unknown_fields` plus a non-`Option` field makes an unclassified
    /// delta unrepresentable, which is the same move `routes: Vec<String>` made
    /// for the family verdict: the shape that caused the defect cannot be
    /// written down. A rule would have left the old bare string readable and
    /// merely disapproved of.
    #[test]
    fn a_delta_with_no_disposition_is_refused_by_the_type_and_not_by_a_rule() {
        let r = record(
            &["app/dash/page.tsx"],
            vec![region(ReviewRegion::Header, true)],
        );
        let text = serde_yaml::to_string(&r).unwrap();
        assert!(
            serde_yaml::from_str::<VisualReviewRecord>(&text).is_ok(),
            "the fixture must parse, or the negative below proves nothing"
        );
        let undispositioned: String = text
            .lines()
            .filter(|line| !line.trim_start().starts_with("disposition:"))
            .map(|line| format!("{line}\n"))
            .collect();
        assert_ne!(
            undispositioned, text,
            "the seed did not remove the disposition"
        );
        assert!(
            serde_yaml::from_str::<VisualReviewRecord>(&undispositioned).is_err(),
            "an unclassified delta must be unrepresentable, not merely invalid"
        );
    }

    // -- draft S-7D(12): a conform verdict earns its claim ---------------------

    #[test]
    fn a_deviate_verdict_is_held_to_no_parity_standard() {
        let mut r = record(
            &["app/dash/page.tsx"],
            vec![region(ReviewRegion::Header, true)],
        );
        r.screenshot_path = None;
        r.served_build = None;
        assert!(
            r.unearned_conformance(false).is_empty(),
            "a deviate verdict claims no parity, and holding it to a parity standard makes \
             recording a deviation more expensive than recording nothing"
        );
    }

    #[test]
    fn a_conform_verdict_must_earn_all_four_conditions() {
        let mut r = record(
            &["app/dash/page.tsx"],
            vec![region(ReviewRegion::Header, true)],
        );
        r.verdict = AuthorityVerdict::Conform;
        r.deltas = vec![];
        assert!(r.unearned_conformance(true).is_empty(), "the earning shape");

        let mut minted = r.clone();
        minted.screenshot_path = None;
        assert!(
            minted.unearned_conformance(false)[0].contains("MINTED"),
            "{:?}",
            minted.unearned_conformance(false)
        );

        let mut moved = r.clone();
        assert!(moved.unearned_conformance(false)[0].contains("does not re-hash"));
        moved.served_build = None;
        assert!(
            moved
                .unearned_conformance(true)
                .iter()
                .any(|d| d.contains("LOCAL TREE"))
        );

        let mut nobody_looked = r.clone();
        nobody_looked.regions = vec![region(ReviewRegion::Header, false)];
        assert!(
            nobody_looked.defects().is_empty(),
            "the record is WELL FORMED, which is the whole point: it passes validation and \
             says nobody looked"
        );
        assert_eq!(
            nobody_looked.examined_regions(),
            Vec::<ReviewRegion>::new(),
            "a row that says it was not examined is not a look at the band"
        );
        assert!(
            nobody_looked
                .unearned_conformance(true)
                .iter()
                .any(|d| d.contains("examined NO band"))
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
