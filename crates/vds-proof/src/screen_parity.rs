//! The `screen_parity` proof. The only kind whose subject is a SCREEN.
//!
//! VDS S-7(5): "each registered screen's required arrangement is the arrangement
//! its authoritative frame draws". It is the eleventh kind and it exists because
//! the first ten all read a COMPONENT. Two of them say the word "screen"
//! (`register_completeness`, `composition`), but what they read is a screen's
//! REFERENCES: the ledger at `crates/vds-scan/src/lib.rs:88` holds a route, a
//! digest and a list of component names with line numbers. So a page could
//! render every registered component, each in an enforceable status, arranged in
//! a way its frame does not draw, and all ten kinds stayed green. Parity is a
//! claim about screens and there was no screen to make it about.
//!
//! # The rules
//!
//! One row is one registered screen.
//!
//!   R1  the record's arrangement contract can be met by nothing, or failed by
//!       nothing. The no-op guard, and the twin of `contrast` R7. Fatal, and the
//!       row is NOT enforced, because a row that cannot fail is not a row that
//!       was checked.
//!   R2  a binding screen record names no frame, so its requirement is measured
//!       against nothing.
//!   R3  the record names a frame the capture has no row for.
//!   R4  the frame draws a different number of content columns than the record
//!       requires.
//!   R5  the record requires a region the frame's authoritative layer does not
//!       draw.
//!   R6  the reading came from a subtree that reaches the capture boundary, so
//!       it states an absence nothing observed.
//!   R7  a node on the path stated no readable column contribution, so the
//!       total is UNKNOWN rather than smaller. See [`Contribution`].
//!   W1  a frame in the capture that no screen record claims. A screen drawn in
//!       the decided-target file and absent from the register is one design has
//!       committed to and governance has never seen. Informational, and NOT a
//!       row: the row unit is a registered screen, and counting an unclaimed
//!       frame as one would inflate `rows_enforced` with something nothing was
//!       enforced against.
//!
//! # Coverage is part of the result
//!
//! Every row lands in exactly one of [`Coverage`]'s three classes, and the
//! tally is checked against the row count before anything is printed. SCORED is
//! the only class a pass is about. UNSCORED means a requirement exists and this
//! run could not measure it, and every one of those is a fatal finding: R1, R2,
//! R3, R6 and R7 are all unscored. EXCLUDED means the DESIGN states nothing to
//! measure against, which is a different fact and never a failure: a screen the
//! register has not put in an enforceable status, and a frame that DISCLAIMS
//! ITSELF.
//!
//! The reason the three are separated rather than counted together is the prior
//! art's own number: its gate scored 32% of routes and would have reported zero
//! deviations, while 75 routes with a real multi-column contract were never
//! compared once.
//!
//! # The seam
//!
//! [`ArrangementSource`] is where a SUBJECT plugs in its own reader. The rules
//! above are stated against an [`Arrangement`], not against a Figma payload, so
//! the same rules run over any source that can produce one. VDS ships exactly
//! ONE implementation, [`FrameArrangements`], which reads the frame ledger. The
//! other side, what the CODE renders, is deliberately not here: counting a
//! route's columns from its source requires parsing the subject's own layout
//! components, and there is no general answer to "how many columns does this
//! render". In the subject this was derived from it took a TypeScript AST pass
//! over one component's three pane props plus the rails contributed by every
//! enclosing layout, and both halves were defects before they were features.
//! Writing a guess at that into VDS would make VDS an authority on what a screen
//! is made of.
//!
//! So [`REACH_NOTE`] says, on every captured record, that a pass here
//! establishes the requirement against the FRAME and never against the code.
//! That limit is stated rather than left to be assumed, because three separate
//! numbers in the subject were called parity and none of them was.
//!
//! This proof reads counts, layer NAMES and region NAMES. It reads no design
//! value (VDS S-2(2)).

use std::collections::BTreeSet;
use std::io::Write;

use vds_core::{FigmaFrame, ProofKind, Result, Status, VdsError, Violation};
use vds_figma::frames::{self, FrameLedger};

use crate::ProofContext;
use crate::run::{Outcome, ProofRun, Verdict};

pub const GATE: &str = "crates/vds-proof/src/screen_parity.rs";

const RULE_UNENFORCEABLE: &str = "VDS S-7(5) screen_parity R1 / S-7(2)(4): an arrangement contract that no measurement \
     could fail, or that none could satisfy";
const RULE_NO_FRAME: &str =
    "VDS S-7(5) screen_parity R2: a binding screen record names the frame that draws it";
const RULE_NO_ROW: &str =
    "VDS S-7(5) screen_parity R3: the capture covers every frame the register names";
const RULE_COLUMNS: &str = "VDS S-7(5) screen_parity R4: a screen's required column count is the one its \
     authoritative frame draws";
const RULE_REGION: &str = "VDS S-7(5) screen_parity R5: a screen's required regions are the ones its authoritative \
     frame draws";
const RULE_TRUNCATED: &str = "VDS S-7(5) screen_parity R6: a screen is measured against a reading taken from a subtree \
     that was captured in full";
const RULE_UNREADABLE: &str = "VDS S-7(5) screen_parity R7: every node on the path to a screen states its column \
     contribution, so an unreadable one makes the total unknown rather than smaller";
const RULE_UNCLAIMED: &str = "VDS S-5(6) screen_parity W1: a frame drawn in the decided-target file and claimed by no \
     screen record";

// Skip reasons. Stable machine keys and never sentences: each becomes a count in
// `rows_skipped_reasons`, and a per-row sentence would make every count one.
const SKIP_NOT_ENFORCEABLE: &str = "screen_status_is_not_registered_built_or_verified";
const SKIP_UNENFORCEABLE_CONTRACT: &str = "arrangement_contract_cannot_be_failed_or_cannot_be_met";
const SKIP_NO_FRAME: &str = "screen_record_names_no_frame";
const SKIP_NO_ROW: &str = "no_row_in_the_capture_for_the_named_frame";
const SKIP_DISCLAIMED: &str = "frame_disclaims_itself_and_states_no_contract";
const SKIP_TRUNCATED: &str = "column_derivation_read_the_capture_boundary";
const SKIP_UNREADABLE_CONTRIBUTION: &str = "a_node_on_the_path_stated_no_readable_contribution";

/// The most unclaimed frames one run names individually.
///
/// A first capture of a file nobody has registered yet is several hundred true
/// findings about one fact, and a record nobody reads is a different way of
/// hiding them. Nothing is dropped silently: the remainder is counted in the
/// note.
const MAX_UNCLAIMED_NAMED: usize = 40;

pub const REACH_NOTE: &str = "[reach] this run establishes that each registered screen's required arrangement is the one \
     its AUTHORITATIVE FRAME draws, in the decided-target file, as of the capture the frame \
     ledger was derived from. It does NOT establish what the code renders. Counting a route's \
     columns from its source means parsing the subject's own layout components, and there is no \
     general answer to that: in the subject this was derived from it took an AST pass over one \
     component's three pane props PLUS the rails contributed by every enclosing layout, and both \
     halves were defects before they were features. A subject that can read its own arrangement \
     supplies an `ArrangementSource` and the same rules run over it. Until it does, a pass here \
     is a pass about the FRAME, and saying so is the point: three separate numbers in the \
     subject were reported as parity and none of them was.";

pub const EXCLUSION_NOTE: &str = "[exclusions] a screen is EXCLUDED only where the DESIGN states nothing to measure against, \
     and an exclusion is never a pass. Two things qualify: a screen the register has not put in \
     an enforceable status, and a frame that DISCLAIMS ITSELF, meaning its authoritative layer \
     says in its own name that it is not source-current or was never built (25 of 188 frames in \
     the subject did this). Everything else this run could not measure is UNSCORED and carries a \
     fatal finding, including a reading taken at the capture boundary: bytes that cannot \
     distinguish \"draws nothing here\" from \"we did not look\" are not a reading, the prior \
     art recorded the first as a fact about one of the two busiest routes in the product, and \
     the truth was the second.";

// ------------------------------------------------------------------ the seam

/// What ONE node on the path to a screen contributes to its arrangement.
///
/// # Why a contribution and not a count
///
/// A route's column count is not one component's property, and a seam that
/// asked "how many columns does this page render" would make every subject
/// reproduce two defects that were both real in the subject this was derived
/// from:
///
///   - the columns contributed by an ENCLOSING layout are part of the page.
///     Twelve routes rendered a two-pane inner component while their frame drew
///     three, and the third was real: it came from a layout further up the tree
///     that mounts a shell with a rail. Counting only the innermost component
///     is counting part of the page and calling the remainder a defect.
///   - some shells fix their count IN THE COMPONENT rather than in a prop.
///     A shell that takes no pane prop at all and always renders exactly one
///     content column has stated the answer plainly, and a reader that only
///     understood props left 24 such routes unscored.
///
/// So the seam asks for the contribution of each node on the path from the
/// route entry to the leaf, and VDS sums them. A subject that gets its own
/// reader wrong is then wrong about ONE node rather than about a whole route,
/// and [`Contribution::Unreadable`] is how it says so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Contribution {
    /// This node contributes `columns` panes, and here is why it says so.
    Columns {
        /// `file:line`, a node id, a component name: whatever a reader opens.
        locator: String,
        columns: u32,
        /// How the source knows. Printed in a finding, because "code renders 3"
        /// is not something a reader can check and "3 = 2 panes at
        /// Workbench.tsx:41 + 1 rail at settings/layout.tsx:12" is.
        basis: String,
    },
    /// This node is on the path and MAY contribute, and this source cannot say
    /// how much.
    ///
    /// The variant that stops the second defect above from recurring silently.
    /// A reader that does not understand a shell has two options: report it, or
    /// contribute zero. Contributing zero is indistinguishable from a shell
    /// that genuinely adds no column, so the route is scored against a total
    /// that is quietly too small and the gate passes it. Here it cannot: a
    /// single unreadable node makes the whole route UNSCORED, which is a
    /// finding.
    Unreadable { locator: String, why: String },
}

impl Contribution {
    pub fn locator(&self) -> &str {
        match self {
            Contribution::Columns { locator, .. } => locator,
            Contribution::Unreadable { locator, .. } => locator,
        }
    }
}

/// One screen's arrangement, as some source reads it.
///
/// The proof's rules are stated against THIS and never against a Figma payload,
/// which is what makes the source substitutable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arrangement {
    /// Every node on the path that contributes, in the order the source walked
    /// them. Summed by [`Arrangement::columns`].
    pub contributions: Vec<Contribution>,
    /// The named regions this source found, unioned across the path. A region
    /// contributed by an enclosing layout is a region the screen has.
    pub regions: Vec<String>,
    /// This drawing says in its own name that it is not a contract.
    pub disclaimed: bool,
    /// This reading came from a subtree that reaches the limit of what was
    /// captured, so it states an absence it could not observe.
    pub truncated: bool,
    /// What a finding should name so a reader can go and look.
    pub locator: String,
}

impl Arrangement {
    /// One contribution, for a source that reads the whole arrangement in one
    /// place. The frame ledger is such a source: a frame draws its columns in
    /// one drawing.
    pub fn single(locator: impl Into<String>, columns: u32, basis: impl Into<String>) -> Self {
        let locator = locator.into();
        Arrangement {
            contributions: vec![Contribution::Columns {
                locator: locator.clone(),
                columns,
                basis: basis.into(),
            }],
            regions: Vec::new(),
            disclaimed: false,
            truncated: false,
            locator,
        }
    }

    pub fn with_regions(mut self, regions: Vec<String>) -> Self {
        self.regions = regions;
        self
    }

    /// The total column count, or the nodes that made it unknowable.
    ///
    /// `Err` and not a saturating sum. A source that could not read one node on
    /// the path has not measured a smaller screen, it has not measured the
    /// screen, and the difference is the whole of FINDING 6's second half.
    pub fn columns(&self) -> std::result::Result<u32, Vec<&Contribution>> {
        let unreadable: Vec<&Contribution> = self
            .contributions
            .iter()
            .filter(|c| matches!(c, Contribution::Unreadable { .. }))
            .collect();
        if !unreadable.is_empty() {
            return Err(unreadable);
        }
        Ok(self
            .contributions
            .iter()
            .map(|c| match c {
                Contribution::Columns { columns, .. } => *columns,
                Contribution::Unreadable { .. } => 0,
            })
            .sum())
    }

    /// How the total was arrived at, for a finding a reader can check.
    pub fn basis(&self) -> String {
        self.contributions
            .iter()
            .map(|c| match c {
                Contribution::Columns {
                    locator,
                    columns,
                    basis,
                } => format!("{columns} at {locator} ({basis})"),
                Contribution::Unreadable { locator, why } => {
                    format!("unreadable at {locator} ({why})")
                }
            })
            .collect::<Vec<String>>()
            .join(" + ")
    }
}

/// WHERE A SUBJECT PLUGS IN ITS OWN LAYOUT READER.
///
/// VDS ships one implementation, [`FrameArrangements`], over the frame ledger.
/// The subject-specific half, reading what the CODE renders, is out of VDS's
/// reach on purpose (see the module note and [`REACH_NOTE`]), and this trait is
/// the whole of what such a reader has to satisfy.
///
/// The shape follows [`vds_figma::FigmaSource`] deliberately
/// (`crates/vds-figma/src/pull.rs:28`): a trait, a description recorded on the
/// output, and no transport visible from the proof path, so nothing here can
/// acquire a network dependency VDS S-7(2)(1) forbids.
pub trait ArrangementSource {
    /// The arrangement for one frame, or `None` where this source has no
    /// reading for it.
    ///
    /// `None` is NOT "no columns". A source that cannot see a screen has to say
    /// so, and the proof reports it as R3 rather than scoring it clean, because
    /// a route quietly dropped from a gate is a route scored clean by silence.
    fn arrangement(&self, frame: &FigmaFrame) -> Option<Arrangement>;

    /// Every reading this source holds, for reporting the ones nobody claims.
    fn all_locators(&self) -> BTreeSet<String>;

    /// A sentence naming the source, recorded on the proof record.
    fn describe(&self) -> String;
}

/// The one implementation VDS ships: the frame ledger.
pub struct FrameArrangements<'a> {
    pub ledger: &'a FrameLedger,
}

impl ArrangementSource for FrameArrangements<'_> {
    fn arrangement(&self, frame: &FigmaFrame) -> Option<Arrangement> {
        // The file key is checked by the caller against the register's own
        // answer, not silently here. A source that quietly returned `None` for
        // a frame in another file would report "the capture does not cover it",
        // which is true and is not the finding: the finding is that two
        // decided-target files is two opinions about what is decided.
        let row = self.ledger.row(&frame.node_id)?;
        // ONE contribution, because a frame draws its whole arrangement in one
        // place. The multi-node path is the CODE side's shape, not the
        // drawing's, and pretending the frame had one too would be a fiction
        // that made the seam look symmetrical when it is not.
        let mut arrangement = Arrangement::single(
            format!("{} {:?}", row.node_id, row.authority_layer),
            row.columns,
            format!(
                "clustered by x-interval under the authority resolved by {}",
                row.authority_by.as_str()
            ),
        )
        .with_regions(row.regions.clone());
        arrangement.disclaimed = row.disclaimed;
        arrangement.truncated = row.truncated;
        Some(arrangement)
    }

    fn all_locators(&self) -> BTreeSet<String> {
        frames::node_ids(self.ledger)
    }

    fn describe(&self) -> String {
        format!(
            "the frame ledger, captured from file {} at depth {}",
            self.ledger.file_key, self.ledger.capture_depth
        )
    }
}

// ---------------------------------------------------------------- coverage

/// SCORED, UNSCORED, EXCLUDED. Part of the result, never a footnote.
///
/// A screen proof that measures the routes it happens to understand and prints
/// a clean pass is the exact failure this capability exists to prevent. The
/// prior art's own gate scored 32% of routes and would have reported "zero
/// deviations" while 75 routes with a real multi-column contract were never
/// compared once. The number that makes that visible has to be part of the
/// result and not a sentence somebody may or may not read.
///
/// The three are genuinely different facts and are kept apart for that reason:
///
///   - SCORED: compared against a reading. The only class a pass is about.
///   - UNSCORED: there is a requirement and VDS could not measure it. Always a
///     fatal finding, never a quiet skip. This is the class the whole type
///     exists to make impossible to hide.
///   - EXCLUDED: there is nothing to measure it against, because the DESIGN
///     says so. A frame that disclaims itself, or a screen the register has
///     not put in an enforceable status. "The design is missing" is a different
///     fact from "the code is wrong", and a subject has to be able to say it.
///
/// The arithmetic is checked rather than trusted: [`Coverage::check_against`]
/// refuses a tally that does not add up to the rows considered, on the same
/// principle as `contrast` R7 refusing a floor no measurement can fail
/// (`crates/vds-proof/src/contrast.rs:155`). A run that cannot express its own
/// coverage has not established its own scope, and a proof that does not know
/// what it covered proves nothing about what it did not.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Coverage {
    pub scored: u64,
    pub unscored: u64,
    pub excluded: u64,
}

impl Coverage {
    pub fn total(&self) -> u64 {
        self.scored + self.unscored + self.excluded
    }

    /// Refuse a tally that does not account for every row considered.
    pub fn check_against(&self, rows_considered: u64) -> Result<()> {
        if self.total() != rows_considered {
            return Err(VdsError::precondition(format!(
                "this run considered {rows_considered} screen(s) and its coverage accounts for \
                 {} of them ({} scored, {} unscored, {} excluded). A proof that cannot say what \
                 it covered cannot say what it did not, and a pass over an unstated scope is \
                 the defect this kind exists to prevent, one level up. Refusing rather than \
                 printing a number that does not add up.",
                self.total(),
                self.scored,
                self.unscored,
                self.excluded
            )));
        }
        Ok(())
    }

    /// The line every run prints and every record carries.
    pub fn line(&self) -> String {
        self.line_for("registered screen(s)")
    }

    /// The same line, about a different subject.
    ///
    /// One arithmetic and one sentence, borrowed rather than reimplemented by
    /// the sixteenth kind. Two coverage types would be two chances to disagree
    /// about what "scored" counts, which is the failure this type exists to
    /// make visible one level down.
    pub fn line_for(&self, subject: &str) -> String {
        let total = self.total();
        // A run over zero rows reports 0%, not a division by zero and not
        // 100%. "Everything was scored" and "nothing was there to score" are
        // different facts, and the second one is what an empty register means.
        let percent = (self.scored * 100).checked_div(total).unwrap_or(0);
        format!(
            "[coverage] {} of {} {subject} were SCORED ({percent}%). {} UNSCORED: a \
             requirement exists and this run could not measure it, which is a finding and never \
             a pass. {} EXCLUDED: the design states nothing to measure against, which is a \
             different fact from the code being wrong.",
            self.scored, total, self.unscored, self.excluded
        )
    }
}

/// What one screen row turned out to be. Consumed by [`score`], which is the
/// only place a row is counted, so the coverage tally and `rows_considered`
/// cannot drift apart.
enum Scoring {
    Scored,
    /// A requirement VDS could not measure. Always fatal.
    Unscored(&'static str),
    /// The design states nothing to measure against.
    Excluded(&'static str),
}

/// Count one screen, in both registers at once.
///
/// The single call site is the point. `ProofRun::classify` already makes
/// `rows_considered` and the skip counts add up by consuming a token
/// (`crates/vds-proof/src/run.rs:47`); this does the same job for the coverage
/// tally, so a future edit cannot count a row in one and not the other.
fn score(run: &mut ProofRun, coverage: &mut Coverage, scoring: Scoring) {
    match scoring {
        Scoring::Scored => {
            coverage.scored += 1;
            run.row(Verdict::Enforced);
        }
        Scoring::Unscored(reason) => {
            coverage.unscored += 1;
            run.row(Verdict::Skipped(reason));
        }
        Scoring::Excluded(reason) => {
            coverage.excluded += 1;
            run.row(Verdict::Skipped(reason));
        }
    }
}

// ------------------------------------------------------------------ the proof

pub fn run(ctx: &ProofContext, out: &mut dyn Write) -> Result<Outcome> {
    let project = ctx.project;
    let store = ctx.store();
    let records = store.read_screens()?;

    let Some(ledger) = frames::read(project)? else {
        // An absent ledger with NO registered screen is not a fault: a project
        // that has registered no screen has nothing for this kind to be about,
        // and the run says so as a vacuity rather than as a refusal. An absent
        // ledger with registered screens IS a refusal, because there is then a
        // requirement and nothing to measure it against, and reporting that as
        // a vacuity would let a project register a hundred screens and never
        // capture a frame while this kind printed a tidy exit 3.
        if records.is_empty() {
            let mut run = ctx.new_run(ProofKind::ScreenParity, GATE);
            run.input_file(&project.config_path)?;
            run.note(REACH_NOTE);
            run.note(Coverage::default().line());
            run.note(format!(
                "no screen is registered and no frame ledger exists, so this run considered \
                 nothing. Register a screen with `vds screen add` and capture its frame with \
                 `{}`.",
                frames::GENERATOR_COMMAND
            ));
            return run.finish(&ctx.capture_options()?, out);
        }
        return Err(VdsError::precondition(format!(
            "{} registered screen(s) state an arrangement requirement and there is no frame \
             ledger at {}, so there is nothing to measure them against and this proof did not \
             run.\n  The frame ledger is a generated inventory (VDS S-4(2)) derived out of band \
             from a saved capture, because VDS S-7(2)(1) forbids a network call inside a \
             proof.\n  Run: {} --from <capture.json>",
            records.len(),
            project.rel(&frames::ledger_path(project)),
            frames::GENERATOR_COMMAND
        )));
    };

    // The register's own view of which file is decided, checked BEFORE any row
    // is measured. A ledger pulled from the wrong file answers every question
    // about the wrong document, and it answers them confidently.
    let declared = frames::declared_file_key(&store)?;
    frames::check_fresh(&ledger, declared.as_deref())?;

    let source = FrameArrangements { ledger: &ledger };
    run_against(ctx, &source, &records, out)
}

/// The rules, over any source. Split out so the seam is real at the call site
/// and not merely described in a comment.
pub fn run_against(
    ctx: &ProofContext,
    source: &dyn ArrangementSource,
    records: &[vds_store::Located<vds_core::ScreenRecord>],
    out: &mut dyn Write,
) -> Result<Outcome> {
    let project = ctx.project;
    let mut run = ctx.new_run(ProofKind::ScreenParity, GATE);
    run.input_file(&project.config_path)?;
    for record in records {
        run.input_file(&record.path)?;
    }
    run.note(REACH_NOTE);
    run.note(EXCLUSION_NOTE);
    run.note(format!("[source] read from {}", source.describe()));

    let mut claimed: BTreeSet<String> = BTreeSet::new();
    let mut coverage = Coverage::default();

    for located in records {
        let record = &located.value;
        let at = format!("{} {:?}", record.id, record.route);

        if !record.status.is_enforceable() {
            // `proposed` and `designed` have shipped nothing by construction,
            // and `deprecated`/`retired` are tombstones kept forever
            // (VDS S-9(6)(3)). Measuring a tombstone's arrangement is measuring
            // a screen that is on its way out. EXCLUDED and not unscored: the
            // register has said this screen is not in a state to be measured,
            // which is a decision and not a gap.
            score(
                &mut run,
                &mut coverage,
                Scoring::Excluded(SKIP_NOT_ENFORCEABLE),
            );
            continue;
        }

        // R1, the no-op guard, checked FIRST. Every rule below it is a
        // comparison, and comparing against a contract nothing can fail
        // produces a pass that means nothing. A component record was once
        // "fixed" by writing a contrast floor of 1.0, and the contrast proof
        // refuses exactly that at `crates/vds-proof/src/contrast.rs:155`.
        if let Some(why) = record.arrangement.unenforceable_because() {
            score(
                &mut run,
                &mut coverage,
                Scoring::Unscored(SKIP_UNENFORCEABLE_CONTRACT),
            );
            run.fail(Violation::fatal(
                at.clone(),
                RULE_UNENFORCEABLE,
                format!(
                    "an arrangement contract a measurement could fail: columns between 1 and {} \
                     (VDS S-7(2)(4))",
                    vds_core::MAX_COLUMNS
                ),
                why,
            ));
            continue;
        }

        let Some(frame) = &record.frame else {
            // A binding record (VDS S-5(4): from `registered` onward the
            // contract is complete and binding) whose requirement names nothing
            // to measure. Staging is what `proposed` and `designed` are for,
            // and both are skipped above.
            score(&mut run, &mut coverage, Scoring::Unscored(SKIP_NO_FRAME));
            run.fail(Violation::fatal(
                at.clone(),
                RULE_NO_FRAME,
                "a `frame` naming the node in the decided-target file that draws this screen",
                format!(
                    "{} is {} and names no frame, so its required arrangement of {} column(s) \
                     is measured against nothing",
                    record.id, record.status, record.arrangement.columns
                ),
            ));
            continue;
        };

        let Some(arrangement) = source.arrangement(frame) else {
            score(&mut run, &mut coverage, Scoring::Unscored(SKIP_NO_ROW));
            run.fail(Violation::fatal(
                at.clone(),
                RULE_NO_ROW,
                format!(
                    "a reading for node {} in {}",
                    frame.node_id,
                    source.describe()
                ),
                format!(
                    "no reading covers it, so this screen's requirement is unmeasured. A route \
                     the capture does not reach is not a route that passed; re-capture \
                     including {}.",
                    frame.node_id
                ),
            ));
            continue;
        };
        claimed.insert(frames::normalise_node_id(&frame.node_id));

        if arrangement.disclaimed {
            // EXCLUDED, and this is the one class that is deliberately not a
            // finding. The design says in its own name that it is not a
            // contract, so there is nothing to deviate from; 25 of 188 frames
            // in the subject did this. Reporting it as a violation would be the
            // gate crying wolf about a route the designer already marked
            // superseded, and reporting it as nothing would be the route
            // disappearing.
            score(&mut run, &mut coverage, Scoring::Excluded(SKIP_DISCLAIMED));
            // A WARNING and not an informational, and the difference is
            // whether a reader at a terminal ever sees which route this was.
            // `ProofRun::print` lists warnings and fatals and not
            // informationals (`crates/vds-proof/src/run.rs:310`), so an
            // exclusion filed as informational is named only inside the
            // captured record. `composition` reached the same conclusion for a
            // deprecated component under VDS S-9(6)(1): printed AND captured,
            // counted, and not blocking. An exclusion nobody can see is a route
            // scored clean by silence.
            run.warn(Violation::fatal(
                at.clone(),
                RULE_COLUMNS,
                "a frame that states a contract",
                format!(
                    "{} disclaims itself, so it states no arrangement to deviate from and this \
                     screen was EXCLUDED rather than scored. The design is missing, which is a \
                     different fact from the code being wrong. Repair the frame or retire the \
                     record.",
                    arrangement.locator
                ),
            ));
            continue;
        }

        if arrangement.truncated {
            // AN UNSEEN CHILD IS NOT AN ABSENT CHILD, and this is the branch
            // that keeps the two apart. Enforcing here would report a column
            // count derived from a subtree nobody fetched, which is what the
            // prior art did about one of the two busiest routes in the product.
            //
            // UNSCORED AND FATAL, not an informational exclusion. The capture
            // being too shallow is a defect in the capture, fixable by one
            // re-run at a greater depth, and a route nobody could measure must
            // never be one the gate passes over quietly: that is exactly how a
            // gate scores 32% of its subject and reports zero deviations.
            score(&mut run, &mut coverage, Scoring::Unscored(SKIP_TRUNCATED));
            run.fail(Violation::fatal(
                at.clone(),
                RULE_TRUNCATED,
                "a reading taken from a subtree that was captured in full",
                format!(
                    "{} was read from a subtree reaching the capture boundary, so its column \
                     count states an absence the capture could not observe. Re-capture deeper \
                     and regenerate: {} --from <capture.json>",
                    arrangement.locator,
                    frames::GENERATOR_COMMAND
                ),
            ));
            continue;
        }

        // FINDING 6. The total is SUMMED over every node the source walked, and
        // an unreadable node makes the total unknown rather than smaller.
        let columns = match arrangement.columns() {
            Ok(columns) => columns,
            Err(unreadable) => {
                score(
                    &mut run,
                    &mut coverage,
                    Scoring::Unscored(SKIP_UNREADABLE_CONTRIBUTION),
                );
                run.fail(Violation::fatal(
                    at.clone(),
                    RULE_UNREADABLE,
                    "a column contribution from every node on the path to this screen",
                    format!(
                        "{} node(s) on the path could not be read, so the total is unknown \
                         rather than small: {}. Contributing zero for a node nobody understood \
                         would score this screen against a total that is quietly too low, and \
                         the gate would pass it.",
                        unreadable.len(),
                        arrangement.basis()
                    ),
                ));
                continue;
            }
        };

        score(&mut run, &mut coverage, Scoring::Scored);

        if record.arrangement.columns != columns {
            run.fail(Violation::fatal(
                at.clone(),
                RULE_COLUMNS,
                format!(
                    "{} content column(s), as {} requires",
                    record.arrangement.columns, record.id
                ),
                format!(
                    "{} draws {columns} ({}). Either the screen was built to a different \
                     arrangement than the one registered, or the record is out of date with \
                     the drawing.",
                    arrangement.locator,
                    arrangement.basis()
                ),
            ));
        }

        let missing = record.required_regions_missing_from(&arrangement.regions);
        if !missing.is_empty() {
            run.fail(Violation::fatal(
                at,
                RULE_REGION,
                format!(
                    "the region(s) {} required by {}",
                    record.arrangement.regions.join(", "),
                    record.id
                ),
                format!(
                    "{} draws {}, so {} {} absent. A region the frame does not draw is one the \
                     screen cannot have.",
                    arrangement.locator,
                    if arrangement.regions.is_empty() {
                        "no named region".to_owned()
                    } else {
                        arrangement.regions.join(", ")
                    },
                    missing.join(", "),
                    if missing.len() == 1 { "is" } else { "are" }
                ),
            ));
        }
    }

    // W1, the other direction of VDS S-5(6). NOT counted as rows: the row unit
    // is a registered screen, and counting a frame nobody registered would
    // raise `rows_enforced` for something nothing was enforced against, which is
    // the arithmetic half of the [2026] VJS-CC-OPBOX 3 D3 defect.
    let unclaimed: Vec<String> = source
        .all_locators()
        .into_iter()
        .filter(|id| !claimed.contains(id))
        .collect();
    if !unclaimed.is_empty() {
        run.note(format!(
            "{} frame(s) in the capture are claimed by no screen record. A screen drawn in the \
             decided-target file and absent from the register is one design has committed to \
             and governance has never seen (VDS S-5(6)). They are NOT rows: nothing was \
             enforced against them, so counting them would raise rows_enforced for nothing.",
            unclaimed.len()
        ));
        for node_id in unclaimed.iter().take(MAX_UNCLAIMED_NAMED) {
            run.inform(Violation::fatal(
                node_id.clone(),
                RULE_UNCLAIMED,
                "a screen record claiming this frame, or a capture that does not include it",
                "no screen record names this node, so nothing VDS holds says what it requires"
                    .to_owned(),
            ));
        }
        if unclaimed.len() > MAX_UNCLAIMED_NAMED {
            run.note(format!(
                "{} further unclaimed frame(s) are counted above and not named individually; a \
                 record nobody reads is a different way of hiding them, and a first capture of \
                 an unregistered file is several hundred findings about one fact.",
                unclaimed.len() - MAX_UNCLAIMED_NAMED
            ));
        }
    }

    // FINDING 7. The coverage is checked BEFORE it is printed, so a tally that
    // does not account for every row considered is a refusal rather than a
    // number nobody adds up. `run.finish` has not been called yet, so nothing
    // has been captured and the refusal leaves no record behind claiming a
    // result.
    coverage.check_against(run.rows_considered())?;
    run.note(coverage.line());

    run.finish(&ctx.capture_options()?, out)
}

/// A convenience for a caller that already holds the records.
pub fn source_from(ledger: &FrameLedger) -> FrameArrangements<'_> {
    FrameArrangements { ledger }
}

/// The statuses this proof measures, for `vds screen` to print.
pub fn enforceable_statuses() -> [Status; 3] {
    Status::ENFORCEABLE
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{Harness, run_kind};
    use vds_core::{
        ArrangementContract, EXIT_PASSED, EXIT_VACUOUS, EXIT_VIOLATION, ProofStatus, ScreenId,
        Timestamp,
    };

    /// A screen registered at two columns, and a capture whose frame draws two.
    fn agreeing() -> Harness {
        let h = Harness::new();
        h.screen_record("SCR-0001", "/orders", 2, &["rail", "body"], Some("1:1"));
        h.frames(&[Harness::frame(
            "1:1",
            "Screen · /orders",
            &["rail", "body"],
            2,
        )]);
        h
    }

    #[test]
    fn a_screen_whose_frame_draws_what_it_requires_passes() {
        let h = agreeing();
        let (outcome, text) = run_kind(&h, ProofKind::ScreenParity);
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
        assert_eq!(outcome.exit_code, EXIT_PASSED);
        assert_eq!(outcome.rows_enforced, 1, "one registered screen is one row");
    }

    /// THE FAILING-DIRECTION TEST VDS S-7(2)(2) REQUIRES, and the one
    /// `.vds/enforcement.lock` names. It seeds the exact defect this whole
    /// capability exists for: a screen carrying one more column than its frame
    /// draws, with every component on it registered and every other kind green.
    #[test]
    fn screen_parity_fails_on_a_screen_with_a_column_its_frame_does_not_draw() {
        let h = Harness::new();
        h.screen_record("SCR-0001", "/dashboards", 3, &[], Some("1:1"));
        h.frames(&[Harness::frame("1:1", "Screen · /dashboards", &["body"], 2)]);

        let (outcome, text) = run_kind(&h, ProofKind::ScreenParity);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert_eq!(outcome.status, ProofStatus::Failed);
        assert!(text.contains("/dashboards"), "{text}");
        assert!(text.contains("draws 2"), "{text}");
        assert_eq!(
            outcome.rows_enforced, 1,
            "the row was enforced AND failed; a failure over zero enforced rows is a vacuity \
             wearing a violation's clothes: {text}"
        );
    }

    /// The mutation check for the same rule in the other direction: the record
    /// is left alone and the FRAME moves. A gate that only noticed the record
    /// changing would be a gate on the register, not on the agreement.
    #[test]
    fn screen_parity_fails_when_the_frame_moves_and_the_record_does_not() {
        let h = agreeing();
        let (before, _) = run_kind(&h, ProofKind::ScreenParity);
        assert_eq!(before.exit_code, EXIT_PASSED);

        h.frames(&[Harness::frame(
            "1:1",
            "Screen · /orders",
            &["rail", "body"],
            3,
        )]);
        let (after, text) = run_kind(&h, ProofKind::ScreenParity);
        assert_eq!(after.exit_code, EXIT_VIOLATION, "{text}");
    }

    #[test]
    fn screen_parity_fails_on_a_required_region_the_frame_does_not_draw() {
        let h = Harness::new();
        h.screen_record(
            "SCR-0001",
            "/settings",
            1,
            &["rail", "cmdbar", "body"],
            Some("1:1"),
        );
        h.frames(&[Harness::frame("1:1", "Screen · /settings", &["body"], 1)]);

        let (outcome, text) = run_kind(&h, ProofKind::ScreenParity);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("rail, cmdbar"), "{text}");
        assert!(text.contains("cannot have"), "{text}");
    }

    // -- R1, the no-op guard, both ends --------------------------------------

    /// The [2026] VJS-CC-OPBOX 3 D3 defect in its screen form: a record "fixed"
    /// into something no measurement can fail. `contrast` refuses a floor of
    /// 1.0 for the same reason, and a component record was once repaired
    /// exactly that way.
    #[test]
    fn a_zero_column_contract_is_refused_rather_than_passed() {
        let h = Harness::new();
        h.screen_record("SCR-0001", "/x", 0, &[], Some("1:1"));
        h.frames(&[Harness::frame("1:1", "Screen · /x", &["body"], 1)]);

        let (outcome, text) = run_kind(&h, ProofKind::ScreenParity);
        assert_eq!(
            outcome.exit_code, EXIT_VIOLATION,
            "0 columns agrees with nothing and disagrees with nothing, and passing it would be \
             a green row that proves nothing: {text}"
        );
        assert_eq!(
            outcome.rows_enforced, 0,
            "a row that cannot fail is not a row that was checked: {text}"
        );
        assert!(
            text.contains(SKIP_UNENFORCEABLE_CONTRACT),
            "the skip has to be visible in the record, not just in the exit code: {text}"
        );
    }

    /// The twin, and the one people route around. A permanently red row teaches
    /// everyone to reach for the escape hatch, and then every other check goes
    /// off with it.
    #[test]
    fn a_contract_no_frame_could_ever_satisfy_is_refused_too() {
        let h = Harness::new();
        h.screen_record("SCR-0001", "/x", 9000, &[], Some("1:1"));
        h.frames(&[Harness::frame("1:1", "Screen · /x", &["body"], 2)]);

        let (outcome, text) = run_kind(&h, ProofKind::ScreenParity);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("permanently red"), "{text}");
        assert_eq!(outcome.rows_enforced, 0, "{text}");
    }

    // -- the exclusions ------------------------------------------------------

    /// A frame that says in its own name that it is not source-current states
    /// no contract. Measuring against it produces a difference that is real and
    /// means nothing, which is a gate crying wolf about a route the designer
    /// already marked superseded.
    #[test]
    fn a_self_disclaiming_frame_is_named_and_not_scored_either_way() {
        let h = Harness::new();
        h.screen_record("SCR-0001", "/sql", 3, &[], Some("1:1"));
        h.frames(&[Harness::frame(
            "1:1",
            "LEGACY / TARGET REFERENCE · /sql · NOT SOURCE CURRENT",
            &["body"],
            1,
        )]);

        let (outcome, text) = run_kind(&h, ProofKind::ScreenParity);
        assert_eq!(
            outcome.status,
            ProofStatus::Vacuous,
            "the only screen was excluded, so the run proves nothing and says so: {text}"
        );
        assert_eq!(outcome.exit_code, EXIT_VACUOUS);
        assert!(text.contains(SKIP_DISCLAIMED), "{text}");
        assert!(
            text.contains("disclaims itself"),
            "the exclusion has to be NAMED, or a route silently dropped from a gate is a route \
             scored clean by silence: {text}"
        );
        assert!(
            text.contains("1 EXCLUDED"),
            "\"the design is missing\" is a different fact from \"the code is wrong\", and the \
             coverage line is where a reader learns which one this was: {text}"
        );
    }

    /// AN UNSEEN CHILD IS NOT AN ABSENT CHILD. The count is real, the reading
    /// is not, and enforcing it would be the prior art's own mistake: an
    /// absence recorded as a fact about a route with four children nobody
    /// fetched.
    /// UNSCORED AND FATAL. A route nobody could measure must never be one the
    /// gate passes over quietly: that is exactly how the prior art's own gate
    /// scored 32% of its subject and would have reported zero deviations.
    #[test]
    fn a_reading_from_the_capture_boundary_is_unscored_and_fails() {
        let h = Harness::new();
        h.screen_record("SCR-0001", "/matters", 2, &[], Some("2:2"));
        h.frames(&[Harness::boundary_frame("2:2", "Screen · /matters")]);

        let (outcome, text) = run_kind(&h, ProofKind::ScreenParity);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert_eq!(outcome.rows_enforced, 0, "{text}");
        assert!(text.contains(SKIP_TRUNCATED), "{text}");
        assert!(text.contains("could not observe"), "{text}");
        assert!(
            text.contains("1 UNSCORED"),
            "the coverage line has to say a requirement went unmeasured: {text}"
        );
    }

    #[test]
    fn a_screen_that_is_only_proposed_is_counted_and_not_enforced() {
        let h = agreeing();
        h.amend_screen("SCR-0001", |record| record.status = Status::Proposed);
        let (outcome, text) = run_kind(&h, ProofKind::ScreenParity);
        assert_eq!(outcome.status, ProofStatus::Vacuous, "{text}");
        assert!(text.contains(SKIP_NOT_ENFORCEABLE), "{text}");
    }

    // -- R2, R3 --------------------------------------------------------------

    #[test]
    fn a_binding_record_with_no_frame_fails_rather_than_being_skipped_quietly() {
        let h = Harness::new();
        h.screen_record("SCR-0001", "/x", 2, &[], None);
        h.frames(&[Harness::frame("1:1", "Screen · /x", &["body"], 2)]);

        let (outcome, text) = run_kind(&h, ProofKind::ScreenParity);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("measured against nothing"), "{text}");
    }

    #[test]
    fn a_frame_the_capture_does_not_reach_fails_rather_than_passing() {
        let h = Harness::new();
        h.screen_record("SCR-0001", "/x", 2, &[], Some("9:9"));
        h.frames(&[Harness::frame("1:1", "Screen · /elsewhere", &["body"], 2)]);

        let (outcome, text) = run_kind(&h, ProofKind::ScreenParity);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(
            text.contains("is not a route that passed"),
            "an unmeasured route scored clean is the defect this whole kind exists for: {text}"
        );
    }

    // -- W1 ------------------------------------------------------------------

    #[test]
    fn a_frame_no_screen_record_claims_is_reported_and_is_not_a_row() {
        let h = agreeing();
        h.frames(&[
            Harness::frame("1:1", "Screen · /orders", &["rail", "body"], 2),
            Harness::frame("7:7", "Screen · /undeclared", &["body"], 1),
        ]);

        let (outcome, text) = run_kind(&h, ProofKind::ScreenParity);
        assert_eq!(outcome.exit_code, EXIT_PASSED, "{text}");
        assert_eq!(
            outcome.rows_enforced, 1,
            "an unclaimed frame is not a row, or rows_enforced rises for something nothing was \
             enforced against: {text}"
        );
        assert!(text.contains("governance has never seen"), "{text}");
    }

    // -- preconditions -------------------------------------------------------

    /// A project that has registered no screen has nothing for this kind to be
    /// about, and a vacuity is the honest report. Refusing here would make
    /// adopting the eleventh kind a precondition failure on every project that
    /// has not used it yet.
    #[test]
    fn a_project_with_no_screens_and_no_capture_is_vacuous_rather_than_refused() {
        let h = Harness::new();
        let (outcome, text) = run_kind(&h, ProofKind::ScreenParity);
        assert_eq!(outcome.status, ProofStatus::Vacuous, "{text}");
        assert_eq!(outcome.rows_considered, 0, "{text}");
    }

    /// The other half, and the one that stops the vacuity above from becoming a
    /// hiding place: register a hundred screens, capture nothing, and this must
    /// not print a tidy exit 3.
    #[test]
    fn a_registered_screen_with_no_capture_is_a_precondition_failure() {
        let h = Harness::new();
        h.screen_record("SCR-0001", "/x", 2, &[], Some("1:1"));
        let error = h.run_kind_err(ProofKind::ScreenParity);
        assert!(error.to_string().contains("did not run"), "{error}");
        assert!(error.to_string().contains("vds figma frames"), "{error}");
    }

    #[test]
    fn a_hand_edited_frame_ledger_is_refused_rather_than_read() {
        let h = agreeing();
        let path = h.root().join(".vds/ledgers/frames.yaml");
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, text.replace("columns: 2", "columns: 3")).unwrap();
        let error = h.run_kind_err(ProofKind::ScreenParity);
        assert!(error.to_string().contains("was edited"), "{error}");
    }

    /// Two decided-target files is two opinions about what is decided, and a
    /// ledger from the wrong one answers every question about the wrong
    /// document, confidently.
    #[test]
    fn a_ledger_from_a_file_the_register_does_not_name_is_refused() {
        let h = agreeing();
        h.amend_screen("SCR-0001", |record| {
            if let Some(frame) = record.frame.as_mut() {
                frame.file_key = "SOMEWHERE-ELSE".into();
            }
        });
        let error = h.run_kind_err(ProofKind::ScreenParity);
        assert!(error.to_string().contains("two opinions"), "{error}");
    }

    // -- determinism ---------------------------------------------------------

    /// VDS S-7(2)(1). The generator stamps `generated_at`, so a proof digesting
    /// the ledger FILE would move its evidence digest every time the generator
    /// ran over an unchanged capture, and every warrant citing it would look
    /// spent.
    #[test]
    fn two_runs_over_one_capture_cite_the_same_evidence_digest() {
        let h = agreeing();
        let (first, _) = run_kind(&h, ProofKind::ScreenParity);
        let (second, _) = run_kind(&h, ProofKind::ScreenParity);
        let store = h.store();
        assert_eq!(
            store
                .read_proof(&first.record_id.unwrap())
                .unwrap()
                .value
                .digest,
            store
                .read_proof(&second.record_id.unwrap())
                .unwrap()
                .value
                .digest
        );
    }

    #[test]
    fn the_run_records_what_it_does_not_reach() {
        let h = agreeing();
        run_kind(&h, ProofKind::ScreenParity);
        let record = h.last_proof(ProofKind::ScreenParity);
        let reach = record
            .notes
            .iter()
            .find(|n| n.starts_with("[reach]"))
            .unwrap_or_else(|| panic!("{:?}", record.notes));
        assert!(
            reach.contains("does NOT establish what the code renders"),
            "a reader who is not told the limit will assume the gate covers the code, which is \
             how three numbers in the subject were reported as parity: {reach}"
        );
        assert!(
            record.notes.iter().any(|n| n.starts_with("[exclusions]")),
            "{:?}",
            record.notes
        );
    }

    // -- the seam ------------------------------------------------------------

    /// THE SEAM, exercised. The rules are stated against an `Arrangement`, so a
    /// subject that can read what its CODE renders gets the same rules with no
    /// change to any of them. This stub stands in for that reader, and the fact
    /// that it fails the same way the frame source does is the whole claim the
    /// seam makes.
    /// A stub standing in for a subject's own code reader, shaped the way
    /// FINDING 6 says such a reader has to be: one contribution per node on the
    /// path from the route entry to the leaf, which VDS sums.
    struct StubCodeReader {
        /// (locator, columns) per node on the path. The innermost component
        /// first, then each enclosing layout.
        path: Vec<(&'static str, u32)>,
        /// A node the reader could not understand, if any.
        unreadable: Option<&'static str>,
    }

    impl StubCodeReader {
        fn innermost(columns: u32) -> Self {
            StubCodeReader {
                path: vec![("app/orders/page.tsx:12 <Workbench>", columns)],
                unreadable: None,
            }
        }
    }

    impl ArrangementSource for StubCodeReader {
        fn arrangement(&self, frame: &FigmaFrame) -> Option<Arrangement> {
            let mut contributions: Vec<Contribution> = self
                .path
                .iter()
                .map(|(locator, columns)| Contribution::Columns {
                    locator: (*locator).to_owned(),
                    columns: *columns,
                    basis: "a pane prop that is not null".to_owned(),
                })
                .collect();
            if let Some(locator) = self.unreadable {
                contributions.push(Contribution::Unreadable {
                    locator: locator.to_owned(),
                    why: "a shell that takes no pane prop, so the count is fixed in the \
                          component and this reader does not open it"
                        .to_owned(),
                });
            }
            Some(Arrangement {
                contributions,
                regions: vec!["body".into()],
                disclaimed: false,
                truncated: false,
                locator: format!("the code path for node {}", frame.node_id),
            })
        }
        fn all_locators(&self) -> BTreeSet<String> {
            BTreeSet::new()
        }
        fn describe(&self) -> String {
            "a stub layout reader standing in for a subject's own".to_owned()
        }
    }

    #[test]
    fn the_same_rules_run_over_a_substituted_source() {
        let h = Harness::new();
        h.screen_record("SCR-0001", "/orders", 2, &[], Some("1:1"));
        let records = h.store().read_screens().unwrap();
        let ctx = h.context();

        let mut out = Vec::new();
        let agreeing =
            run_against(&ctx, &StubCodeReader::innermost(2), &records, &mut out).unwrap();
        assert_eq!(agreeing.exit_code, EXIT_PASSED);
        assert_eq!(agreeing.rows_enforced, 1);

        let mut out = Vec::new();
        let disagreeing =
            run_against(&ctx, &StubCodeReader::innermost(3), &records, &mut out).unwrap();
        assert_eq!(disagreeing.exit_code, EXIT_VIOLATION);
        let text = String::from_utf8(out).unwrap();
        assert!(
            text.contains("app/orders/page.tsx:12"),
            "a finding from a substituted source names ITS locator, not a Figma node: {text}"
        );
    }

    /// FINDING 6, first half. A route's columns are the SUM over the path, and
    /// a seam that asked one component for the answer would make every subject
    /// reproduce the defect that failed twelve routes for a column they have:
    /// the third column came from an enclosing layout.
    #[test]
    fn a_column_contributed_by_an_enclosing_layout_counts_towards_the_total() {
        let h = Harness::new();
        h.screen_record("SCR-0001", "/settings/teams", 3, &[], Some("1:1"));
        let records = h.store().read_screens().unwrap();
        let ctx = h.context();

        // The innermost component alone renders two panes, and the record
        // requires three. Counting only it reports a defect that is not there.
        let mut out = Vec::new();
        let inner_only =
            run_against(&ctx, &StubCodeReader::innermost(2), &records, &mut out).unwrap();
        assert_eq!(
            inner_only.exit_code, EXIT_VIOLATION,
            "the fixture must actually be short by one, or the next assertion proves nothing"
        );

        let mut out = Vec::new();
        let whole_path = run_against(
            &ctx,
            &StubCodeReader {
                path: vec![
                    ("app/settings/teams/page.tsx:20 <Workbench>", 2),
                    ("app/settings/layout.tsx:12 <CanvasPage rightRail>", 1),
                ],
                unreadable: None,
            },
            &records,
            &mut out,
        )
        .unwrap();
        assert_eq!(
            whole_path.exit_code,
            EXIT_PASSED,
            "counting only the innermost component is counting part of the page and calling \
             the remainder a defect: {}",
            String::from_utf8(out).unwrap()
        );
    }

    /// FINDING 6, second half, and the one that has to be structural. A reader
    /// that does not understand a shell has two options: report it, or
    /// contribute zero. Contributing zero is indistinguishable from a shell
    /// that genuinely adds no column, so the route is scored against a total
    /// that is quietly too small and the gate passes it. This asserts it
    /// cannot.
    #[test]
    fn a_node_the_reader_cannot_understand_makes_the_route_unscored_and_not_short() {
        let h = Harness::new();
        h.screen_record("SCR-0001", "/settings", 2, &[], Some("1:1"));
        let records = h.store().read_screens().unwrap();
        let ctx = h.context();

        let mut out = Vec::new();
        let outcome = run_against(
            &ctx,
            &StubCodeReader {
                // One readable pane. Summed as-is this is 1 against a required
                // 2, which reads as a missing column; the truth is that the
                // shell adds one and this reader cannot see it.
                path: vec![("app/settings/page.tsx:8 <Workbench>", 1)],
                unreadable: Some("src/components/SettingsPageShell.tsx"),
            },
            &records,
            &mut out,
        )
        .unwrap();
        let text = String::from_utf8(out).unwrap();
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert_eq!(
            outcome.rows_enforced, 0,
            "an unreadable node makes the total UNKNOWN, so nothing was scored: {text}"
        );
        assert!(text.contains("SettingsPageShell.tsx"), "{text}");
        assert!(
            text.contains("unknown rather than small"),
            "the finding has to say WHY it is not a missing column, or the reader fixes the \
             wrong thing: {text}"
        );
        assert!(text.contains("1 UNSCORED"), "{text}");
    }

    /// FINDING 7. Every row lands in exactly one class and the tally is checked
    /// against the row count, so a run cannot report on a scope it did not
    /// establish. Seeded RED by hand here because the production path cannot
    /// produce a mismatch: `score` is the only place a row is counted, which is
    /// the point of it.
    #[test]
    fn a_coverage_tally_that_does_not_add_up_is_refused() {
        let honest = Coverage {
            scored: 2,
            unscored: 1,
            excluded: 1,
        };
        honest.check_against(4).unwrap();

        let error = honest.check_against(9).unwrap_err();
        assert!(
            error.to_string().contains("cannot say what it did not"),
            "{error}"
        );
        assert_eq!(error.exit_code(), vds_core::EXIT_PRECONDITION);
    }

    /// The coverage line is what a reader sees, so it has to say the thing the
    /// prior art's report did not: how much of the subject was measured.
    #[test]
    fn the_coverage_line_states_the_scored_fraction_and_lands_on_the_record() {
        let h = Harness::new();
        h.screen_record("SCR-0001", "/a", 2, &[], Some("1:1"));
        h.screen_record("SCR-0002", "/b", 2, &[], Some("2:2"));
        h.screen_record("SCR-0003", "/c", 2, &[], None);
        h.amend_screen("SCR-0002", |record| record.status = Status::Proposed);
        h.frames(&[Harness::frame("1:1", "Screen · /a", &["body"], 2)]);

        let (_, text) = run_kind(&h, ProofKind::ScreenParity);
        assert!(
            text.contains("1 of 3 registered screen(s) were SCORED (33%)"),
            "{text}"
        );

        let record = h.last_proof(ProofKind::ScreenParity);
        assert!(
            record.notes.iter().any(|n| n.starts_with("[coverage]")),
            "a coverage number printed and not captured is a coverage number nobody reads: \
             {:?}",
            record.notes
        );
    }

    #[test]
    fn a_source_with_no_reading_for_a_screen_says_so_rather_than_scoring_it_clean() {
        struct Blind;
        impl ArrangementSource for Blind {
            fn arrangement(&self, _: &FigmaFrame) -> Option<Arrangement> {
                None
            }

            fn all_locators(&self) -> BTreeSet<String> {
                BTreeSet::new()
            }
            fn describe(&self) -> String {
                "a source that reads nothing".to_owned()
            }
        }

        let h = Harness::new();
        h.screen_record("SCR-0001", "/orders", 2, &[], Some("1:1"));
        let records = h.store().read_screens().unwrap();
        let ctx = h.context();
        let mut out = Vec::new();
        let outcome = run_against(&ctx, &Blind, &records, &mut out).unwrap();
        assert_eq!(
            outcome.exit_code,
            EXIT_VIOLATION,
            "None must never read as \"no columns\": {}",
            String::from_utf8(out).unwrap()
        );
    }

    /// A helper the record type owns, checked here because the proof is what
    /// depends on it. Kept so a future edit that reorders the contract's
    /// regions cannot silently change which findings are produced.
    #[test]
    fn the_contract_type_and_the_proof_agree_about_an_empty_region_list() {
        let contract = ArrangementContract {
            columns: 1,
            regions: vec![],
            bands: vec![],
        };
        let record = vds_core::ScreenRecord {
            id: ScreenId::parse("SCR-0001").unwrap(),
            route: "/x".into(),
            status: Status::Registered,
            contract_version: 1,
            frame: None,
            arrangement: contract,
            basis: vec![],
            notes: None,
        };
        assert!(
            record.required_regions_missing_from(&[]).is_empty(),
            "a screen that requires no region is satisfied by a frame that draws none"
        );
        let _ = Timestamp::fixed(2026, 7, 30, 10, 0, 0);
    }
}
