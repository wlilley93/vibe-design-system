//! The `contrast` proof. The gate the founding defect would have tripped.
//!
//! VDS S-7(5): "every registered component's boundaries clear their floors in
//! every theme". VDS S-1(4)(a) records what happens without it: a control
//! boundary was declared aligned in prose, shipped at 1.20:1 against a 3.0:1
//! requirement, and failed WCAG 2.2 SC 1.4.11 across five themes, worst at
//! 1.15:1, until a hand audit found it months later.
//!
//! The whole architecture of this kind is in one sentence of VDS S-2(4): a
//! [`ContrastFloor`] is a REQUIREMENT and holds no colour. It names a boundary,
//! the thing that boundary is measured against, and a minimum ratio drawn from
//! WCAG. The colours live in the record VDS S-2(3) makes the system of record for
//! what ships, and this proof DERIVES the ratio from that record at run time.
//! Nothing is stored, so nothing can go stale, and changing the stylesheet
//! changes the verdict without anyone editing `.vds/`.
//!
//! Eight rules. All fatal, because every one of them is a boundary this run could
//! not certify, and "I could not measure it" recorded as a pass is the failure
//! this whole system exists to prevent:
//!
//!   R1  a boundary that does not clear its floor in some theme. The finding
//!       carries the MEASURED ratio against the NAMED floor, so a reader can see
//!       the size of the gap rather than only its existence.
//!   R2  a floor whose property does not RESOLVE in some theme. A violation and
//!       never a skip: the reason is typed, and it is named in the finding.
//!   R3  a resolved value that is not a colour this instrument can read.
//!   R4  a backdrop that is itself translucent. Refused rather than composited
//!       over an assumed page colour, because the assumption would produce a
//!       confident wrong number and a number gets believed.
//!   R5  a floor property that a conditional at-rule gives ANOTHER value. The
//!       unconditional value was measured and the guarded one cannot be, so the
//!       boundary is not certified. See the note on this below.
//!   R6  a palette scope the theme discovery could not classify. A palette the
//!       gate never asks about is a palette that can ship below its floor with
//!       every proof green, which is exactly VDS S-1(4)(a).
//!   R7  a floor outside the range a WCAG 2.x ratio can take. A floor at or below
//!       1.0 cannot be failed by any pair of colours, so counting it as an
//!       enforced row would be the arithmetic half of the
//!       [2026] VJS-CC-OPBOX 3 D3 defect.
//!   R8  a floor whose boundary or backdrop does not name a custom property at
//!       all. Usually the storing form: a colour written where a name belongs.
//!
//! A row is one (component, floor, theme). Every theme the sheet declares is
//! measured, base included: a component that clears its floor in light and fails
//! in dark has FAILED, and a gate that measured only the base would have passed
//! the founding defect in four of its five themes.
//!
//! ## Two decisions that shape everything below
//!
//! **A finding carries the measured RATIO and never a resolved COLOUR.** A
//! captured proof record lands in `.vds/proofs/`, which is the tree
//! `no_stored_values` scans, so a finding that copied a resolved value would put
//! a realisation under the record permanently and both gates would then fail
//! forever on a file this one wrote. That is why every reason in here is reported
//! as a stable CLASS rather than by rendering the error the CSS layer returns:
//! [`ColourError`] names the offending input in several of its variants, and the
//! offending input is a colour.
//!
//! The ratio itself is not a realisation. Apply VDS S-2(5) to it: delete every
//! proof record and no shipped or decided value is lost, because the number is
//! recomputable by one command from the stylesheet (limb 1); change the sheet and
//! the next run says so rather than serving the old number (limb 2); no reader
//! can move a shipped pixel by editing a proof record (limb 3); and the figure is
//! reproducible from the named record by the named command (limb 4). VDS S-2(6)
//! settles the rest: a numeral is not automatically a value, and a measured ratio
//! is the same shape as the `minRatio` it is measured against, which that clause
//! declares lawful. The one place where the number does invert to a colour is the
//! top of the scale, and [`MAX_RECORDED_RATIO`] closes it.
//!
//! **This proof reads the SHIPPED record and nothing else.** It does not open the
//! decided-target Figma file, because that is a network read and VDS S-7(2)(1)
//! forbids one inside a proof. So a pass establishes that what ships clears the
//! floors, and never that what ships is what was decided; that agreement is the
//! `token_pin` kind's to establish (VDS S-7(5)).

use std::io::Write;

use vds_core::{ComponentRecord, ContrastFloor, ProofKind, Result, Status, VdsError, Violation};
use vds_css::colour::{self, Colour, ColourError};
use vds_css::sheet::{Sheet, Unresolvable};

use crate::ProofContext;
use crate::index::RegisterIndex;
use crate::run::{Outcome, ProofRun, Verdict};

pub const GATE: &str = "crates/vds-proof/src/contrast.rs";

/// The default shipped stylesheet, where `[surface] stylesheet` says nothing.
///
/// The key now exists and this constant is only its default, which is the path
/// VDS S-2(3) fixes as the system of record for what ships. It is deliberately
/// NOT mined out of `[governance] permit_required`: that list declares what a
/// permit covers, so adding a stylesheet to it would change what this proof
/// measures as a side effect, and a gate whose subject moves when an unrelated
/// list is edited is a gate nobody can reason about.
pub const SHIPPED_STYLESHEET: &str = "app/globals.css";

const RULE_BELOW_FLOOR: &str = "VDS S-7(5) contrast R1: every registered component's boundaries clear their floors in \
     every theme";
const RULE_UNRESOLVED: &str = "VDS S-7(5) contrast R2 / S-2(3): a floor names a custom property the shipped stylesheet \
     does not resolve in this theme";
const RULE_NOT_A_COLOUR: &str = "VDS S-7(5) contrast R3: a floor's property resolves to something this instrument cannot \
     read as a colour";
const RULE_TRANSLUCENT_BACKDROP: &str = "VDS S-7(5) contrast R4: the value a boundary is measured against is translucent, and no \
     backdrop is named to composite it over";
const RULE_CONDITIONAL: &str = "VDS S-7(5) contrast R5: a floor's property takes another value under a conditional \
     at-rule, and that value is not measured by this run";
const RULE_UNCLASSIFIED_SCOPE: &str = "VDS S-7(5) contrast R6: the stylesheet holds a palette scope the theme discovery did not \
     classify, so no floor was measured in it";
const RULE_UNMEASURABLE_FLOOR: &str = "VDS S-7(5) contrast R7 / S-2(6): a contrast floor sits outside the range a WCAG 2.x ratio \
     can take, so no pair of colours can fail it";
const RULE_NOT_A_PROPERTY: &str = "VDS S-7(5) contrast R8 / S-2(4): a contrast floor names a boundary that is not a custom \
     property, so there is nothing in the shipped record to resolve";
const RULE_MANY: &str =
    "VDS S-7(5) contrast: this run found more boundaries at fault than it lists individually";

/// What right would have looked like wherever a floor could not be measured.
///
/// One sentence, shared, because the answer is the same in every case and a
/// per-rule paraphrase would let the rules drift apart in wording while saying
/// the same thing.
const EXPECTED_MEASURABLE: &str = "the floor names two custom properties that the shipped stylesheet resolves, in this theme, \
     to colours this instrument can read, so that the boundary can be measured rather than \
     assumed. A floor VDS cannot measure is not a floor that passed.";

// Skip reasons. Stable machine keys and never sentences: each becomes a count in
// `rows_skipped_reasons`, and a per-row sentence would make every count one.
const SKIP_PROPOSED: &str = "proposed_nothing_shipped_by_construction";
const SKIP_RETIRED: &str = "retired_tombstone_vds_s9_6_3";
const SKIP_NO_FLOOR: &str = "record_declares_no_contrast_floor";
const SKIP_UNMEASURABLE_FLOOR: &str = "floor_outside_the_measurable_ratio_range";
const SKIP_NOT_A_PROPERTY: &str = "floor_does_not_name_a_custom_property";

/// The most findings one run lists individually.
///
/// A single undeclared property across sixty components and five themes is three
/// hundred true findings about one defect, and a record nobody reads is a
/// different way of hiding them. Nothing is dropped silently: the remainder is
/// counted in its own fatal finding.
const MAX_FINDINGS: usize = 100;

/// The largest ratio this proof records as a point value.
///
/// A WCAG 2.x ratio is bounded above by 21:1, and 21.00 is reached by exactly one
/// pair of colours: the two extreme greys. Recording it would therefore put two
/// realisations under `.vds/` in an invertible form, and VDS S-2(8) defines the
/// storing form by RECOVERABILITY rather than by spelling. Every other
/// two-decimal figure in the range has an enormous preimage set and recovers
/// nothing, so the cap costs one hundredth at one point of the scale and closes
/// the only place where a measurement is a value.
pub const MAX_RECORDED_RATIO: f64 = 20.99;

/// A ratio a WCAG 2.x measurement can actually take, exclusive of the bottom.
///
/// The bottom is exclusive on purpose: at 1.0 every pair of colours meets the
/// floor, including a boundary painted in its own background, so a floor there is
/// a requirement in name only and an enforced row that cannot fail.
///
/// This gate is TIGHTER than the front door, and the difference is deliberate
/// rather than an oversight to be reconciled quietly. `vds register` accepts a
/// floor of exactly 1.0 and puts no ceiling on one at all, because its question
/// is whether the author typed a ratio; the question here is whether the row can
/// fail. Two front doors and one wall (VDS S-11(5)) means the wall decides, and
/// the lawful fix for a floor this refuses is to raise it, which VDS S-9(5)
/// permits any project to do at any time.
const LOWEST_MEANINGFUL_FLOOR: f64 = 1.0;
const HIGHEST_POSSIBLE_RATIO: f64 = 21.0;

pub const REACH_NOTE: &str = "[reach] this run measures the stylesheet VDS S-2(3) fixes as the system of record for what \
     SHIPS, and nothing else. It does not open the decided-target Figma file, because resolving \
     a node is a network read and VDS S-7(2)(1) forbids one inside a proof. A pass therefore \
     establishes that what ships clears the floors in every theme the sheet declares, and never \
     that what ships is what was decided; the agreement between the two records is the \
     `token_pin` kind's to establish (VDS S-7(5)).";

pub const REDACTION_NOTE: &str = "[redaction] a finding names the component, the floor, its two custom properties, the theme \
     selector and the measured ratio, and never a resolved value, an alpha channel or a colour \
     channel. A captured proof record lands under the tree `no_stored_values` scans, so a \
     finding that copied a resolved value would put a realisation under the record permanently \
     and this gate would then fail forever on a file it wrote itself (VDS S-2(2)). The reason a \
     value could not be read is reported as a stable class rather than as the CSS layer's own \
     message, because several of those messages quote the value they refused. Open the named \
     property in the named scope to read it.";

pub const RATIO_NOTE: &str = "[ratio] a measured ratio is recorded as a number against a named floor, and is a \
     measurement over the record VDS S-2(3) names rather than a realisation the record holds: \
     deleting every proof record loses no shipped value, changing the sheet moves the next \
     reading, and no reader can move a shipped pixel by editing one (VDS S-2(5)). VDS S-2(6) \
     settles the shape: a numeral is not automatically a value, and a measured ratio is the same \
     shape as the minimum it is measured against. The single exception is the top of the scale, \
     which is reached by exactly one pair of colours and is therefore recorded as a bound rather \
     than as a point.";

pub const THEME_NOTE: &str = "[themes] every theme scope the stylesheet declares is measured, the base included, and the \
     set is discovered from the sheet rather than read from a list of theme names. A component \
     that clears its floor in one theme and fails in another has failed. A palette scope the \
     discovery could not classify is a fatal finding and never a silent omission, because a \
     palette the gate does not ask about is a palette that can ship below its floor with every \
     proof green.";

pub const ALPHA_NOTE: &str = "[alpha] a translucent boundary is composited over its backdrop before being measured, \
     source-over and in gamma-encoded sRGB, which is what a browser paints. A translucent \
     BACKDROP is refused instead, because compositing it over an assumed page colour would be a \
     guess and a guessed backdrop yields a confident wrong reading. A floor names two \
     properties, so the stack this run paints is exactly one layer over one backdrop; naming a \
     middle layer would need a field the component record does not have.";

pub const SCOPE_NOTE: &str = "[scope] a floor's declared scope is reported in a finding and never decides whether the \
     floor is enforced. A scope that switched enforcement off would be the quietly lowered floor \
     VDS S-9(5) forbids wearing a different hat: the lawful move there is to record the \
     component as decoration WITH its basis, in the register, where a reviewer can contest the \
     claim.";

pub const NO_SKIPPED_MEASUREMENT_NOTE: &str = "[unmeasured] a floor whose properties do not resolve is a VIOLATION and never a skip. The \
     only rows this run skips are the five named reasons counted above, and the last two of \
     those carry a fatal finding of their own, so no row anywhere in this proof is both \
     unmeasured and unreported.";

pub const EMPTY_REGISTER_NOTE: &str = "[register] no register record declares a contrast floor in an enforceable status, so no \
     boundary could be measured and this run is vacuous. It establishes nothing about any \
     component (VDS S-7(2)(4)), and it did not read the stylesheet at all: a stylesheet is a \
     precondition for a project that has something to measure against it, and this project has \
     nothing.";

pub fn run(ctx: &ProofContext, out: &mut dyn Write) -> Result<Outcome> {
    let project = ctx.project;
    let mut run = ctx.new_run(ProofKind::Contrast, GATE);
    run.input_file(&project.config_path)?;

    // Read through the same index every other proof reads through. An ambiguous
    // register, two records on one identifier or one code coordinate, is refused
    // as a precondition rather than half-proven (VDS S-4(4)), so two proofs never
    // disagree about what the register contains.
    let index = RegisterIndex::build(&ctx.store())?;
    for record in index.records() {
        run.input_file(&record.path)?;
    }

    run.note(REACH_NOTE);
    run.note(REDACTION_NOTE);
    run.note(RATIO_NOTE);
    run.note(THEME_NOTE);
    run.note(ALPHA_NOTE);
    run.note(SCOPE_NOTE);
    run.note(NO_SKIPPED_MEASUREMENT_NOTE);

    let to_measure: u64 = index
        .records()
        .iter()
        .map(|located| floors_to_measure(&located.value).len() as u64)
        .sum();

    // The stylesheet is a precondition ONLY for a project that has something to
    // measure against it. Demanding it unconditionally would make this kind exit
    // 2 on every project that has not registered a floor yet, including VDS
    // itself, and an exit 2 that means "nothing to do" teaches a reader to ignore
    // the one that means "the record was never opened".
    let sheet = if to_measure == 0 {
        run.note(EMPTY_REGISTER_NOTE);
        None
    } else {
        let path = project.root.join(&project.config.surface.stylesheet);
        let sheet = read_sheet(project, &path, to_measure)?;
        run.input_file(&path)?;
        run.note(format!(
            "[record] this run measured {}, which is `[surface] stylesheet` in the project \
             configuration{}. It is the ONE record measured: a project that ships some of its \
             tokens from another file is NOT covered by this run, and pointing this gate \
             elsewhere is a configuration change recorded in a diff, never a flag on the \
             command.",
            project.rel(&path),
            if project.config.surface.stylesheet == std::path::Path::new(SHIPPED_STYLESHEET) {
                format!(", left at the default VDS S-2(3) names ({SHIPPED_STYLESHEET})")
            } else {
                format!(
                    ", moved off the default VDS S-2(3) names ({SHIPPED_STYLESHEET}). A warrant \
                     citing this run is bounded by that choice"
                )
            }
        ));
        Some(sheet)
    };

    // Non-empty wherever `sheet` is `Some`: [`read_sheet`] refuses a stylesheet
    // that declares no theme scope, so a floor can never be silently measured
    // against nothing.
    let themes: Vec<String> = sheet
        .as_ref()
        .map(|sheet| {
            sheet
                .theme_selectors()
                .into_iter()
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();

    let mut findings = Findings::default();

    if let Some(sheet) = &sheet {
        run.note(format!(
            "[themes-measured] the theme scopes this run measured, in the order the sheet \
             declares them, base first: {}. A warrant citing this run covers those scopes and no \
             others.",
            themes
                .iter()
                .map(|theme| redacted(theme))
                .collect::<Vec<String>>()
                .join(", ")
        ));
        report_unclassified_scopes(&mut run, &mut findings, project, sheet);
    }

    // Pass one: the status of every record, and whether it declares a floor at
    // all. These rows are counted here and never again, so a record that reaches
    // pass two contributes exactly its (floor, theme) rows.
    for located in index.records() {
        let record = &located.value;
        if let Some(reason) = status_skip(record.status) {
            run.row(Verdict::Skipped(reason));
            continue;
        }
        if record.a11y.contrast_floors.is_empty() {
            // A row that cannot fail is not a row that was checked. Counting it
            // as enforced is the arithmetic half of the [2026] VJS-CC-OPBOX 3 D3
            // defect: `rows_enforced` rises and nothing was established.
            run.row(Verdict::Skipped(SKIP_NO_FLOOR));
        }
    }

    // Pass two: one row per (component, floor, theme). Guarded by the sheet
    // rather than by re-deriving the condition, so the two passes cannot drift
    // into a state where a floor row is emitted with nothing to measure it
    // against.
    if let Some(sheet) = &sheet {
        for located in index.records() {
            let record = &located.value;
            let at_record = project.rel(&located.path);
            for floor in floors_to_measure(record) {
                measure_floor(
                    &mut run,
                    &mut findings,
                    &Subject {
                        sheet,
                        themes: &themes,
                        record,
                        at_record: &at_record,
                        floor,
                    },
                );
            }
        }
    }

    findings.close(&mut run);
    if let Some(narrowest) = findings.narrowest.take() {
        run.note(narrowest.note());
    }

    run.finish(&ctx.capture_options()?, out)
}

/// Everything one floor's rows need, in one place.
///
/// A struct rather than six arguments: the two property names and the theme list
/// travel together everywhere, and a six-argument call is where two of them get
/// swapped.
struct Subject<'a> {
    sheet: &'a Sheet,
    themes: &'a [String],
    record: &'a ComponentRecord,
    at_record: &'a str,
    floor: &'a ContrastFloor,
}

/// The floors of one record that are this proof's to measure.
///
/// Written as one function and called from both passes, so the two can never
/// disagree about which records have something to check.
fn floors_to_measure(record: &ComponentRecord) -> &[ContrastFloor] {
    match status_skip(record.status) {
        Some(_) => &[],
        None => &record.a11y.contrast_floors,
    }
}

/// Why a record's floors are counted and never enforced, or `None`.
///
/// Written out rather than matched with a wildcard: the lifecycle is closed by
/// VDS S-5(4), and a wildcard would silently decide whatever an eighth status
/// turned out to mean. A `proposed` record has nothing shipped by construction,
/// so enforcing it would fail every new registration and teach an author to skip
/// the stage VDS S-5(4) makes mandatory. A `retired` record is a tombstone kept
/// forever (VDS S-9(6)(3)); VDS S-9(8) inverts the test after retirement, and the
/// gate for its code still being there is `reconciliation`, not this one. A
/// `deprecated` component is still on screen until it drains (VDS S-9(6)(2)), so
/// its boundaries are still measured.
fn status_skip(status: Status) -> Option<&'static str> {
    match status {
        Status::Proposed => Some(SKIP_PROPOSED),
        Status::Retired => Some(SKIP_RETIRED),
        Status::Designed
        | Status::Registered
        | Status::Built
        | Status::Verified
        | Status::Deprecated => None,
    }
}

/// Read and parse the stylesheet, or refuse in a way that says the proof did not
/// run.
fn read_sheet(
    project: &vds_core::Project,
    path: &std::path::Path,
    to_measure: u64,
) -> Result<Sheet> {
    let at = project.rel(path);
    if !path.is_file() {
        return Err(VdsError::precondition(format!(
            "{at} is not a file, and {to_measure} contrast floors in the register have to be \
             measured against it. VDS S-2(3) makes it the system of record for what ships, so a \
             run that could not open it establishes nothing about any boundary: a caller told \
             that every boundary clears its floor, about a stylesheet that was never opened, has \
             been told nothing. This proof did not run."
        )));
    }
    let text = std::fs::read_to_string(path).map_err(|e| VdsError::io(&at, e))?;
    let sheet = Sheet::parse(&text);

    if let Some(damage) = sheet.malformed() {
        return Err(VdsError::precondition(format!(
            "{at}: {damage}. A declaration the scanner did not see is not skipped and not \
             counted, it simply does not exist, so a run over a partial read would credit a \
             theme with another scope's values and report the result as clean. This proof did \
             not run."
        )));
    }
    if sheet.theme_selectors().is_empty() {
        return Err(VdsError::precondition(format!(
            "{at} declares no theme scope at all, and {to_measure} contrast floors have to be \
             measured in every theme. Either this is not the record the floors are written \
             against, or this project's palette does not live in CSS custom properties; either \
             way nothing here can be measured, and a vacuous run would read as a clean sheet. \
             This proof did not run."
        )));
    }
    Ok(sheet)
}

/// VDS S-7(5) contrast R6.
///
/// `vds_css` classifies a scope as a theme only where it is root-like AND
/// redeclares part of the base palette, and reports the scopes that redeclare the
/// palette and are not root-like separately. Those are the dangerous ones:
/// `:root:not(.compact)` and a nested `.dark .panel` both redefine tokens and
/// neither is a theme, so a gate that ignored them would measure a palette that
/// is not the one the user sees.
fn report_unclassified_scopes(
    run: &mut ProofRun,
    findings: &mut Findings,
    project: &vds_core::Project,
    sheet: &Sheet,
) {
    let at = project.rel(&project.root.join(SHIPPED_STYLESHEET));
    for selector in sheet.unclassified_palette_scopes() {
        findings.fail(
            run,
            Violation::fatal(
                format!("{at} [{}]", redacted(selector)),
                RULE_UNCLASSIFIED_SCOPE,
                "every scope that redeclares part of the base palette is a theme this run can \
                 ask about, so that no palette ships unmeasured. A scope that is not root-like \
                 redefines tokens somewhere inside the document, and this run cannot tell which \
                 elements it covers.",
                format!(
                    "the scope {} redeclares part of the base palette and was not classified as \
                     a theme, so not one floor was measured in it",
                    redacted(selector)
                ),
            ),
        );
    }
}

/// One floor, in every theme.
fn measure_floor(run: &mut ProofRun, findings: &mut Findings, subject: &Subject) {
    let floor = subject.floor;
    let record = subject.record;
    let themes = subject.themes;

    // Two checks that are theme-independent, so they are made once. Reporting
    // either of them per theme would make one defect look like as many defects as
    // the project has palettes.
    let (boundary, against) = match floor_properties(floor) {
        Ok(pair) => pair,
        Err(which) => {
            for _ in themes {
                run.row(Verdict::Skipped(SKIP_NOT_A_PROPERTY));
            }
            findings.fail(
                run,
                Violation::fatal(
                    format!("{} <{}>", subject.at_record, record.id),
                    RULE_NOT_A_PROPERTY,
                    "boundary and against each name a custom property, with or without its \
                     leading dashes, so that the shipped record can be asked what they resolve \
                     to. A floor is a requirement and holds no value (VDS S-2(4)).",
                    format!(
                        "a floor of {}:1 whose {which} a custom property name. The text is not \
                         repeated here: where it is a colour, this floor is the storing form \
                         VDS S-2(2) forbids and `no_stored_values` is the gate for it.",
                        colour::format_ratio(floor.min_ratio)
                    ),
                ),
            );
            return;
        }
    };

    if !(floor.min_ratio.is_finite()
        && floor.min_ratio > LOWEST_MEANINGFUL_FLOOR
        && floor.min_ratio <= HIGHEST_POSSIBLE_RATIO)
    {
        for _ in themes {
            run.row(Verdict::Skipped(SKIP_UNMEASURABLE_FLOOR));
        }
        findings.fail(
            run,
            Violation::fatal(
                format!("{} <{}>", subject.at_record, record.id),
                RULE_UNMEASURABLE_FLOOR,
                format!(
                    "a floor above {LOWEST_MEANINGFUL_FLOOR} and at or below \
                     {HIGHEST_POSSIBLE_RATIO}, which is the range a WCAG 2.x contrast ratio can \
                     take. A floor at or below the bottom is met by every pair of colours, \
                     including a boundary painted in its own background; one above the top is \
                     met by none."
                ),
                format!(
                    "{} against {}: a floor of {} that no measurement can fail or no design can \
                     satisfy, so its rows were counted and not enforced",
                    redacted(&boundary),
                    redacted(&against),
                    floor.min_ratio
                ),
            ),
        );
        return;
    }

    for theme in themes {
        run.row(Verdict::Enforced);
        let at = format!(
            "{} [{}] <{} {} against {}>",
            subject.at_record,
            redacted(theme),
            record.id,
            redacted(&boundary),
            redacted(&against)
        );
        match read_boundary(subject.sheet, theme, &boundary, &against) {
            Reading::Refused { rule, class, why } => findings.fail(
                run,
                Violation::fatal(at, rule, EXPECTED_MEASURABLE, format!("{class}: {why}")),
            ),
            Reading::Ratio(ratio) => {
                if colour::meets_floor(ratio, floor.min_ratio) {
                    findings.observe_clearance(ratio, floor.min_ratio, &at);
                    continue;
                }
                findings.fail(
                    run,
                    Violation::fatal(
                        at,
                        RULE_BELOW_FLOOR,
                        format!(
                            "{} against {} at {}:1 or more in {}, the floor {} declares on the \
                             basis it records, for a boundary in scope {}",
                            redacted(&boundary),
                            redacted(&against),
                            colour::format_ratio(floor.min_ratio),
                            redacted(theme),
                            record.id,
                            floor
                                .scope
                                .map(|scope| scope.as_str())
                                .unwrap_or("unstated")
                        ),
                        format!(
                            "{}:1, measured from what the shipped record resolves those two \
                             properties to in {}. Neither resolved value is repeated here; see \
                             the redaction note.",
                            recorded_ratio(ratio),
                            redacted(theme)
                        ),
                    ),
                );
            }
        }
    }
}

/// The two custom properties a floor names, or which of its two fields does not
/// name one.
///
/// One match over both fields, so the "which field" sentence in the finding
/// cannot disagree with the test that produced it, and so there is no fourth case
/// to answer with an unreachable arm.
fn floor_properties(floor: &ContrastFloor) -> std::result::Result<(String, String), &'static str> {
    match (
        custom_property(&floor.boundary),
        custom_property(&floor.against),
    ) {
        (Some(boundary), Some(against)) => Ok((boundary, against)),
        (None, None) => Err("boundary and against are neither"),
        (None, Some(_)) => Err("boundary is not"),
        (Some(_), None) => Err("against is not"),
    }
}

/// What one (component, floor, theme) row measured, or why it could not.
enum Reading {
    Ratio(f64),
    Refused {
        rule: &'static str,
        /// A stable machine key for the kind of refusal, so two runs describing
        /// the same defect describe it in the same words.
        class: &'static str,
        why: String,
    },
}

/// Resolve both properties in one theme and measure between them.
///
/// The order is deliberate. The boundary is the subject of the floor, so its
/// defect is the one a reader should see first, and where both properties are
/// unresolvable the second surfaces on the next run. Reporting both would double
/// the finding count for one broken row, and states.rs settles the house answer:
/// one row, one finding.
fn read_boundary(sheet: &Sheet, theme: &str, boundary: &str, against: &str) -> Reading {
    let foreground = match resolved_colour(sheet, theme, boundary) {
        Ok(colour) => colour,
        Err(refused) => return refused,
    };
    let background = match resolved_colour(sheet, theme, against) {
        Ok(colour) => colour,
        Err(refused) => return refused,
    };

    // The backdrop has to be opaque before anything can be painted over it, and
    // the type system is what enforces that: `require_opaque` is the only way to
    // reach a ratio, so a translucent backdrop cannot be measured by accident.
    let Ok(backdrop) = background.require_opaque() else {
        return Reading::Refused {
            rule: RULE_TRANSLUCENT_BACKDROP,
            class: "translucent_backdrop",
            why: format!(
                "{} resolves to a translucent value in this theme, so it has no luminance of \
                 its own and the boundary has no ratio until the surface behind it is named. \
                 The alpha is not repeated here: it is a component of a colour.",
                redacted(against)
            ),
        };
    };
    // Painted rather than measured directly. `composite_over` is a no-op for an
    // opaque foreground, so there is one path and not two, and alpha can never be
    // silently dropped by taking the wrong branch.
    let painted = foreground.composite_over(&backdrop);
    Reading::Ratio(colour::contrast_ratio(&painted, &backdrop))
}

/// What one property resolves to in one theme, as a colour.
fn resolved_colour(
    sheet: &Sheet,
    theme: &str,
    property: &str,
) -> std::result::Result<Colour, Reading> {
    let resolution = sheet.resolve(theme, property);

    // A conditional declaration is checked BEFORE the value, because the defect
    // is not that the unconditional value is wrong: it is that the property takes
    // a value this run cannot pin down. `vds_css` deliberately does not
    // substitute a guarded value, since the properties it references may
    // themselves be conditional, so there is nothing here to measure and a pass
    // would certify a palette the user may never be shown.
    if let Some(guarded) = resolution.conditional.first() {
        return Err(Reading::Refused {
            rule: RULE_CONDITIONAL,
            class: "conditional_declaration",
            why: format!(
                "{} is declared again in {} under {}, at line {}, so the value measured here is \
                 not the only value it takes. Hoist the guarded palette into a scope selector, \
                 where this gate can ask about it.",
                redacted(property),
                redacted(&guarded.selector),
                redacted(&guarded.conditions.join(" and ")),
                guarded.line
            ),
        });
    }

    let Some(value) = resolution.value() else {
        let reason = resolution
            .reason()
            .expect("a resolution is a value or a reason");
        return Err(Reading::Refused {
            rule: RULE_UNRESOLVED,
            class: unresolvable_class(reason),
            why: unresolvable_detail(reason),
        });
    };

    // `colour::parse` and never `parse_with`: the sheet has already substituted
    // every reference in the theme's own context, and handing the parser a second
    // lookup would resolve one that the cascade did not.
    colour::parse(value).map_err(|error| Reading::Refused {
        rule: RULE_NOT_A_COLOUR,
        class: colour_error_class(&error),
        why: format!(
            "{} resolves in this theme to something that is not a colour this instrument can \
             read. The value is not repeated here; open that property in that scope.",
            redacted(property)
        ),
    })
}

/// A stable machine key per reason a resolution failed.
///
/// The variants are matched exhaustively rather than rendered, so a ninth reason
/// added upstream stops the build here rather than arriving as an unclassified
/// sentence.
fn unresolvable_class(reason: &Unresolvable) -> &'static str {
    match reason {
        Unresolvable::UnknownTheme { .. } => "unknown_theme",
        Unresolvable::NotDeclared { .. } => "not_declared_in_this_theme_or_the_base",
        Unresolvable::UndefinedVariable { .. } => "reference_to_an_undeclared_property",
        Unresolvable::Cycle { .. } => "dependency_cycle",
        Unresolvable::DepthExceeded { .. } => "reference_chain_too_deep",
        Unresolvable::ExpansionTooLarge { .. } => "substitution_too_large",
        Unresolvable::LayerConflict { .. } => "cascade_layer_conflict",
        Unresolvable::MalformedValue { .. } => "malformed_value",
    }
}

/// The reason in a sentence, built from the typed fields and never from the
/// error's own rendering.
///
/// Every field used here is a NAME: a selector, a property, a layer, a limit.
/// They are still passed through [`redacted`], because a selector comes from the
/// shipped sheet and a stylesheet that names a class after an arbitrary colour
/// value is an ordinary thing for a utility framework to emit.
fn unresolvable_detail(reason: &Unresolvable) -> String {
    match reason {
        Unresolvable::UnknownTheme { selector } => format!(
            "the sheet has no scope {}, so there was nothing to resolve against",
            redacted(selector)
        ),
        Unresolvable::NotDeclared { selector, property } => format!(
            "neither {} nor the base scope declares {}, so the shipped record gives this \
             boundary no value at all here",
            redacted(selector),
            redacted(property)
        ),
        Unresolvable::UndefinedVariable { selector, name } => format!(
            "{} is referenced from {} and declared in no scope in reach, and the reference has \
             no fallback",
            redacted(name),
            redacted(selector)
        ),
        Unresolvable::Cycle { path } => format!(
            "the properties refer to each other: {}",
            path.iter()
                .map(|step| redacted(step))
                .collect::<Vec<String>>()
                .join(" then ")
        ),
        Unresolvable::DepthExceeded { property, limit } => format!(
            "resolving {} followed more than {limit} references without terminating",
            redacted(property)
        ),
        Unresolvable::ExpansionTooLarge { property, limit } => format!(
            "resolving {} expanded past {limit} characters, which a stylesheet value does not \
             legitimately do",
            redacted(property)
        ),
        Unresolvable::LayerConflict {
            selector,
            property,
            layers,
        } => format!(
            "{} is declared in {} in more than one cascade layer ({}), and which one wins \
             depends on an order this instrument does not model",
            redacted(property),
            redacted(selector),
            layers
                .iter()
                .map(|layer| redacted(layer))
                .collect::<Vec<String>>()
                .join(" and ")
        ),
        Unresolvable::MalformedValue { property, detail } => format!(
            "{} could not be read: {}",
            redacted(property),
            redacted(detail)
        ),
    }
}

/// A stable machine key per reason a value was not a colour.
///
/// The error's `Display` is deliberately never used. Several variants quote the
/// input they refused, and the input is a design value: rendering one into a
/// finding would write a realisation into a record under `.vds/`, which is the
/// tree `no_stored_values` scans. The two gates would then fight forever over a
/// file this one wrote.
fn colour_error_class(error: &ColourError) -> &'static str {
    match error {
        ColourError::Empty => "the_property_resolves_to_nothing",
        ColourError::UnrecognisedSyntax { .. } => "not_a_colour_syntax_this_instrument_reads",
        ColourError::UnknownKeyword { .. } => "an_unknown_colour_keyword",
        ColourError::CurrentColor => "the_inherited_colour_which_this_value_does_not_carry",
        ColourError::TransparentKeyword => "fully_transparent_and_therefore_no_luminance",
        ColourError::UnresolvedCustomProperty { .. } => "a_reference_the_cascade_did_not_resolve",
        ColourError::UnimplementedFunction { .. } => "a_colour_function_this_instrument_lacks",
        ColourError::MalformedFunction { .. } => "a_malformed_colour_function",
        ColourError::LegacyComponentTypeMismatch { .. } => "a_component_type_mismatch_css_rejects",
        ColourError::NoneComponent { .. } => "a_component_with_no_value_to_measure",
        ColourError::UnimplementedInterpolationSpace { .. } => {
            "an_interpolation_space_this_instrument_lacks"
        }
        ColourError::ZeroPercentageSum => "percentages_that_sum_to_zero",
        ColourError::UnknownAngleUnit { .. } => "an_angle_unit_that_is_not_one",
        ColourError::OutOfSrgbGamut { .. } => "outside_the_srgb_gamut_and_not_guessed_at",
        ColourError::TranslucentWithoutBackdrop { .. } => "translucent_with_no_backdrop_named",
        ColourError::NonFiniteComponent { .. } => "a_component_that_is_not_a_finite_number",
        ColourError::SubstitutionLoop { .. } => "a_substitution_loop",
    }
}

/// The custom property a floor names, with its leading dashes, or `None` where
/// the field does not name one.
///
/// Both spellings are accepted, because the register is authored by hand and
/// `control-border` and `--control-border` are the same intention. What is NOT
/// accepted is anything outside a custom property's shape, and the refusal is the
/// point: falling back to reading the field as a colour would MEASURE a value
/// stored in the register, which is to reward the storing form VDS S-2(2)
/// forbids. The register names things; the stylesheet holds them.
fn custom_property(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    let name = trimmed.strip_prefix("--").unwrap_or(trimmed);
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return None;
    }
    Some(format!("--{name}"))
}

/// A ratio as it is recorded: truncated to two places, and bounded at the top.
///
/// See [`MAX_RECORDED_RATIO`] for why the top of the scale is a bound.
fn recorded_ratio(ratio: f64) -> String {
    if ratio > MAX_RECORDED_RATIO {
        return format!("{} or more", colour::format_ratio(MAX_RECORDED_RATIO));
    }
    colour::format_ratio(ratio)
}

/// Text from a named record, with the realisation shapes taken out.
///
/// Only three shapes appear here, and each of them plausibly appears in a
/// selector or a property name: a colour literal in the hash-sigil form, a CSS
/// colour function, and a number carrying a length or time unit. A utility
/// framework emits a class named after an arbitrary colour, and a design system
/// names a property after a step in its scale, so both reach a finding by an
/// ordinary route rather than an adversarial one. What comes out is not the
/// text a reader would grep for, and the redaction note says so.
///
/// This is a guard on THIS proof's output and not a second `no_stored_values`. A
/// realisation in the register is that proof's to find, at its source; the job
/// here is only to avoid copying one into a record that is never deleted.
fn redacted(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        if let Some(width) = colour_literal_at(&chars, i) {
            out.push_str(REDACTED);
            i += width;
            continue;
        }
        if let Some(width) = unit_number_at(&chars, i) {
            out.push_str(REDACTED);
            i += width;
            continue;
        }
        if let Some(width) = colour_function_at(&chars, i) {
            out.push_str(REDACTED);
            i += width;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

const REDACTED: &str = "<a realisation, redacted>";

/// The CSS length and time units `no_stored_values` enforces over the record.
///
/// The length half is held in step by a test, because that list is public there.
/// The two time units are held in step by reading, because they are not. The
/// asymmetry is worth stating rather than glossing: a unit this redactor misses
/// is a unit that reaches a proof record and fails that gate on a file this one
/// wrote.
const UNITS: &[&str] = &[
    "px", "rem", "em", "ex", "ch", "vh", "vw", "vmin", "vmax", "pt", "pc", "cm", "mm", "in", "ms",
    "s",
];

fn wordish(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The length of a colour literal starting here, if there is one.
///
/// Three, four, six and eight hexadecimal digits after the sigil are the four
/// sRGB spellings; any other run is hexadecimal that happens to follow a hash,
/// and an id selector is the ordinary case of that.
fn colour_literal_at(chars: &[char], start: usize) -> Option<usize> {
    if chars[start] != '#' {
        return None;
    }
    let mut end = start + 1;
    while end < chars.len() && chars[end].is_ascii_hexdigit() {
        end += 1;
    }
    let digits = end - start - 1;
    if !matches!(digits, 3 | 4 | 6 | 8) || chars.get(end).copied().is_some_and(wordish) {
        return None;
    }
    Some(end - start)
}

/// The length of a number carrying a CSS unit starting here, if there is one.
fn unit_number_at(chars: &[char], start: usize) -> Option<usize> {
    if !chars[start].is_ascii_digit() {
        return None;
    }
    if start > 0 && wordish(chars[start - 1]) {
        return None;
    }
    let mut end = start;
    while end < chars.len() && (chars[end].is_ascii_digit() || chars[end] == '.') {
        end += 1;
    }
    let letters_from = end;
    while end < chars.len() && chars[end].is_ascii_alphabetic() {
        end += 1;
    }
    if chars.get(end).copied().is_some_and(wordish) {
        return None;
    }
    let unit: String = chars[letters_from..end]
        .iter()
        .flat_map(|c| c.to_lowercase())
        .collect();
    if !UNITS.contains(&unit.as_str()) {
        return None;
    }
    Some(end - start)
}

/// The length of a CSS colour function's name and opening parenthesis starting
/// here, if there is one. Only the head is redacted, which is enough: what
/// `no_stored_values` matches is the name and the parenthesis together.
fn colour_function_at(chars: &[char], start: usize) -> Option<usize> {
    if start > 0 && wordish(chars[start - 1]) {
        return None;
    }
    // Only the head is examined, and the window is the longest name plus its
    // parenthesis. Lowercasing the whole remaining text at every position would be
    // quadratic and would reach nothing a fixed window does not.
    let window: String = chars[start..]
        .iter()
        .take(FUNCTION_WINDOW)
        .flat_map(|c| c.to_lowercase())
        .collect();
    // The parenthesis is checked separately rather than concatenated, so the order
    // of the list cannot matter: `color` cannot swallow `color-mix`, because the
    // character after the shorter name is a hyphen and not an open parenthesis.
    COLOUR_FUNCTIONS
        .iter()
        .find(|name| window.starts_with(**name) && window.as_bytes().get(name.len()) == Some(&b'('))
        .map(|name| name.len() + 1)
}

/// The CSS colour functions.
///
/// This mirrors the alternation `no_stored_values` compiles into its
/// colour-function pattern, which is private to that module and therefore not
/// comparable by a test. Said plainly rather than glossed: the two are kept in
/// step by reading, and a name missing from here is a redaction that does not
/// happen rather than a wrong one.
const COLOUR_FUNCTIONS: &[&str] = &[
    "rgba",
    "rgb",
    "hsla",
    "hsl",
    "hwb",
    "oklch",
    "oklab",
    "lch",
    "lab",
    "color-mix",
    "color",
];

/// The longest name in [`COLOUR_FUNCTIONS`] plus its parenthesis, held in place
/// by a test rather than by arithmetic a reader has to redo.
const FUNCTION_WINDOW: usize = 10;

/// Findings, capped, and the narrowest clearance seen.
///
/// The two live together because both are per-run accumulators that the row loop
/// hands facts to, and keeping them apart would mean threading two mutable
/// references through every call.
#[derive(Default)]
struct Findings {
    emitted: usize,
    suppressed: usize,
    narrowest: Option<Clearance>,
}

/// The tightest a passing boundary came to its floor.
///
/// Recorded because a pass with no margin cannot tell a boundary that clears by a
/// hair from one that clears comfortably, and the founding defect was a boundary
/// whose ratio nobody recomputed. Ties keep the first row encountered, and the
/// row order is fixed (register by path, floors as declared, themes as the sheet
/// declares them), so two runs over one tree name the same row.
struct Clearance {
    ratio: f64,
    floor: f64,
    at: String,
}

impl Clearance {
    fn note(&self) -> String {
        format!(
            "[margin] the narrowest clearance measured in this run was {}:1 against a floor of \
             {}:1, at {}. A pass that records no margin cannot tell a boundary that clears by a \
             hair from one that clears comfortably, and VDS S-1(4)(a) is a boundary whose ratio \
             nobody recomputed.",
            recorded_ratio(self.ratio),
            colour::format_ratio(self.floor),
            self.at
        )
    }
}

impl Findings {
    fn fail(&mut self, run: &mut ProofRun, violation: Violation) {
        if self.emitted >= MAX_FINDINGS {
            self.suppressed += 1;
            return;
        }
        self.emitted += 1;
        run.fail(violation);
    }

    fn observe_clearance(&mut self, ratio: f64, floor: f64, at: &str) {
        let clearance = ratio - floor;
        let tighter = match &self.narrowest {
            Some(current) => clearance < current.ratio - current.floor,
            None => true,
        };
        if tighter {
            self.narrowest = Some(Clearance {
                ratio,
                floor,
                at: at.to_owned(),
            });
        }
    }

    fn close(&mut self, run: &mut ProofRun) {
        if self.suppressed == 0 {
            return;
        }
        let total = self.emitted + self.suppressed;
        run.fail(Violation::fatal(
            SHIPPED_STYLESHEET.to_owned(),
            RULE_MANY,
            EXPECTED_MEASURABLE,
            format!(
                "{total} boundaries at fault. The first {MAX_FINDINGS} are listed individually \
                 and the remaining {} are counted here, so that one undeclared property across \
                 every component and every theme does not become a record nobody reads.",
                self.suppressed
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{Harness, run_kind};
    use vds_core::{
        ComponentId, EXIT_PASSED, EXIT_PRECONDITION, EXIT_VACUOUS, EXIT_VIOLATION, FloorScope,
        ProofKind, ProofStatus, Status,
    };

    /// A light theme where the boundary clears 3.0:1 and a dark theme where it
    /// does not. The dark pair is the founding defect's shape: two greys a
    /// designer would call "obviously different" and a measurement calls 1.21:1.
    const TWO_THEMES: &str = "\
:root { --surface: #ffffff; --control-border: #767676; }
.dark { --surface: #1a1a1a; --control-border: #2a2a2a; }
";

    /// The same two themes, both clearing the floor.
    const TWO_GOOD_THEMES: &str = "\
:root { --surface: #ffffff; --control-border: #767676; }
.dark { --surface: #1a1a1a; --control-border: #9a9a9a; }
";

    fn sheet(h: &Harness, css: &str) {
        h.write(SHIPPED_STYLESHEET, css);
    }

    /// One registered record whose single floor is `boundary` against `against`.
    fn with_floor(
        h: &Harness,
        name: &str,
        status: Status,
        boundary: &str,
        against: &str,
        min_ratio: f64,
    ) -> ComponentId {
        let id = h.register(name, status);
        h.amend(&id, |record| {
            record.a11y.contrast_floors = vec![ContrastFloor {
                boundary: boundary.into(),
                against: against.into(),
                min_ratio,
                basis: "WCAG 2.2 SC 1.4.11".into(),
                scope: Some(FloorScope::ControlBoundary),
            }];
        });
        id
    }

    /// The default subject: one control boundary against one surface, floor 3.0.
    fn subject(h: &Harness, status: Status) -> ComponentId {
        with_floor(h, "Button", status, "control-border", "surface", 3.0)
    }

    #[test]
    fn a_boundary_that_clears_its_floor_in_every_theme_passes() {
        let h = Harness::new();
        sheet(&h, TWO_GOOD_THEMES);
        subject(&h, Status::Registered);
        let (outcome, text) = run_kind(&h, ProofKind::Contrast);
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
        assert_eq!(outcome.exit_code, EXIT_PASSED);
        assert_eq!(
            outcome.rows_enforced, 2,
            "one row per (component, floor, theme): {text}"
        );
    }

    /// The failing-direction test VDS S-7(2)(2) requires, and the one the
    /// enforcement lock names. It seeds a boundary that clears its floor in the
    /// base theme and fails it in the dark one, and asserts the non-zero exit.
    ///
    /// This is the founding defect (VDS S-1(4)(a)) in miniature: a gate that
    /// measured only the base scope would pass this stylesheet.
    #[test]
    fn contrast_fails_on_a_boundary_below_its_floor_in_one_theme() {
        let h = Harness::new();
        sheet(&h, TWO_THEMES);
        subject(&h, Status::Registered);
        let (outcome, text) = run_kind(&h, ProofKind::Contrast);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert_eq!(outcome.status, ProofStatus::Failed);
        assert_eq!(outcome.rows_enforced, 2, "the base theme was measured too");
        assert!(text.contains(".dark"), "{text}");
        assert!(
            text.contains("1.21:1"),
            "the finding must carry the measured ratio, not only the fact of a failure: {text}"
        );
        assert!(text.contains("3.00:1 or more"), "{text}");
    }

    #[test]
    fn a_deprecated_record_is_still_measured_because_it_is_still_shipped() {
        let h = Harness::new();
        sheet(&h, TWO_THEMES);
        subject(&h, Status::Deprecated);
        let (outcome, text) = run_kind(&h, ProofKind::Contrast);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert_eq!(outcome.rows_enforced, 2);
    }

    // -- alpha ---------------------------------------------------------------

    /// The failure mode the CSS layer calls the most likely way to publish a
    /// confident wrong number. Treating this boundary as opaque reads 21:1 and
    /// passes; compositing it, which is what a browser paints, reads 1.53:1.
    #[test]
    fn a_translucent_boundary_is_composited_over_its_backdrop_and_not_read_as_opaque() {
        let h = Harness::new();
        sheet(
            &h,
            ":root { --surface: #ffffff; --control-border: rgba(0, 0, 0, 0.18); }\n",
        );
        subject(&h, Status::Registered);
        let (outcome, text) = run_kind(&h, ProofKind::Contrast);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(
            // 1.52 and not the 1.53 the CSS layer's documentation quotes for the
            // same pair: a recorded ratio is TRUNCATED and never rounded, so a
            // displayed figure is never a smaller one dressed up.
            text.contains("1.52:1"),
            "an alpha read as opaque would have measured the boundary as black on white and \
             passed at the top of the scale: {text}"
        );
    }

    #[test]
    fn a_translucent_backdrop_is_refused_rather_than_composited_over_a_guess() {
        let h = Harness::new();
        sheet(
            &h,
            ":root { --surface: rgba(255, 255, 255, 0.5); --control-border: #767676; }\n",
        );
        subject(&h, Status::Registered);
        let (outcome, text) = run_kind(&h, ProofKind::Contrast);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("translucent_backdrop"), "{text}");
        assert_eq!(
            outcome.rows_enforced, 1,
            "the row was measured and refused, not skipped: {text}"
        );
    }

    // -- unresolvable is a violation, never a skip ---------------------------

    #[test]
    fn a_property_undeclared_in_one_theme_is_a_violation_and_never_a_skip() {
        let h = Harness::new();
        sheet(
            &h,
            "\
:root { --surface: #ffffff; --control-border: #767676; }
.dark { --surface: #1a1a1a; --other: #2a2a2a; }
",
        );
        subject(&h, Status::Registered);
        let (outcome, text) = run_kind(&h, ProofKind::Contrast);
        assert_eq!(
            outcome.exit_code, EXIT_PASSED,
            "the dark scope inherits the base declaration, which is what a browser does: {text}"
        );

        // Now remove the base declaration too, so nothing in reach declares it.
        sheet(
            &h,
            ".dark { --surface: #1a1a1a; --control-border: #2a2a2a; }\n:root { --surface: #ffffff; }\n",
        );
        let (outcome, text) = run_kind(&h, ProofKind::Contrast);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(
            text.contains("not_declared_in_this_theme_or_the_base"),
            "{text}"
        );
        let record = h.last_proof(ProofKind::Contrast);
        assert!(
            record.rows_skipped_reasons.is_empty(),
            "an unmeasurable boundary is a violation and never a skip: {:?}",
            record.rows_skipped_reasons
        );
    }

    #[test]
    fn a_property_that_resolves_to_something_that_is_not_a_colour_is_a_violation() {
        let h = Harness::new();
        sheet(
            &h,
            ":root { --surface: #ffffff; --control-border: 1px solid #767676; }\n",
        );
        subject(&h, Status::Registered);
        let (outcome, text) = run_kind(&h, ProofKind::Contrast);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(
            text.contains("not_a_colour_syntax_this_instrument_reads"),
            "{text}"
        );
    }

    #[test]
    fn a_dependency_cycle_is_reported_by_name_rather_than_resolved_to_a_fallback() {
        let h = Harness::new();
        sheet(
            &h,
            ":root { --surface: #ffffff; --control-border: var(--a); --a: var(--control-border); }\n",
        );
        subject(&h, Status::Registered);
        let (outcome, text) = run_kind(&h, ProofKind::Contrast);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("dependency_cycle"), "{text}");
    }

    /// The dark-mode shape of the founding defect. A palette declared inside a
    /// conditional at-rule is a palette this run cannot measure, and passing the
    /// unconditional value would certify a theme the user may never see.
    #[test]
    fn a_conditional_declaration_of_a_floor_property_is_a_violation() {
        let h = Harness::new();
        sheet(
            &h,
            "\
:root { --surface: #ffffff; --control-border: #767676; }
.dark { --surface: #1a1a1a; --control-border: #9a9a9a; }
@media (prefers-contrast: more) { :root { --control-border: #000000; } }
",
        );
        subject(&h, Status::Registered);
        let (outcome, text) = run_kind(&h, ProofKind::Contrast);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("conditional_declaration"), "{text}");
    }

    /// A palette the theme discovery cannot classify is a palette the gate never
    /// asks about, which is how a boundary ships below its floor with every proof
    /// green.
    #[test]
    fn an_unclassified_palette_scope_fails_the_run() {
        let h = Harness::new();
        sheet(
            &h,
            "\
:root { --surface: #ffffff; --control-border: #767676; }
:root:not(.compact) { --control-border: #eeeeee; }
",
        );
        subject(&h, Status::Registered);
        let (outcome, text) = run_kind(&h, ProofKind::Contrast);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("was not classified as a theme"), "{text}");
    }

    // -- floors that cannot be measured --------------------------------------

    #[test]
    fn a_floor_no_pair_of_colours_can_fail_is_reported_and_never_counted_as_enforced() {
        for unmeasurable in [1.0, 0.0, -3.0, 21.5] {
            let h = Harness::new();
            sheet(&h, TWO_GOOD_THEMES);
            with_floor(
                &h,
                "Button",
                Status::Registered,
                "control-border",
                "surface",
                unmeasurable,
            );
            let (outcome, text) = run_kind(&h, ProofKind::Contrast);
            assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{unmeasurable}: {text}");
            assert_eq!(outcome.rows_enforced, 0);
            assert_eq!(outcome.rows_considered, 2);
            assert!(text.contains(SKIP_UNMEASURABLE_FLOOR), "{text}");
        }
    }

    /// A floor holding a colour instead of a property name is the storing form,
    /// and the finding must say so WITHOUT copying the value into the record.
    #[test]
    fn a_floor_that_names_no_custom_property_is_reported_without_quoting_it() {
        let h = Harness::new();
        sheet(&h, TWO_GOOD_THEMES);
        with_floor(&h, "Button", Status::Registered, "#ebebeb", "surface", 3.0);
        let (outcome, text) = run_kind(&h, ProofKind::Contrast);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("boundary is not"), "{text}");
        assert!(text.contains(SKIP_NOT_A_PROPERTY), "{text}");

        let record = h.last_proof(ProofKind::Contrast);
        let rendered = format!("{:?}", record.violations);
        assert!(
            !rendered.contains("ebebeb"),
            "the finding copied the stored value into a record that is never deleted: {rendered}"
        );
    }

    // -- the register side ---------------------------------------------------

    #[test]
    fn a_proposed_record_is_counted_and_never_enforced() {
        let h = Harness::new();
        sheet(&h, TWO_THEMES);
        subject(&h, Status::Proposed);
        let (outcome, text) = run_kind(&h, ProofKind::Contrast);
        assert_eq!(
            outcome.status,
            ProofStatus::Vacuous,
            "a proposed record would fail R1 if it were enforced, which is what makes the \
             carve-out real rather than decorative: {text}"
        );
        assert_eq!(outcome.rows_considered, 1);
        assert_eq!(outcome.rows_enforced, 0);
        assert!(text.contains(SKIP_PROPOSED), "{text}");
    }

    #[test]
    fn a_retired_tombstone_is_counted_and_never_enforced() {
        let h = Harness::new();
        sheet(&h, TWO_THEMES);
        subject(&h, Status::Retired);
        let (outcome, text) = run_kind(&h, ProofKind::Contrast);
        assert_eq!(outcome.status, ProofStatus::Vacuous, "{text}");
        assert!(text.contains(SKIP_RETIRED), "{text}");
    }

    #[test]
    fn a_record_declaring_no_contrast_floor_is_counted_and_never_enforced() {
        let h = Harness::new();
        sheet(&h, TWO_THEMES);
        let id = h.register("Divider", Status::Registered);
        h.amend(&id, |record| record.a11y.contrast_floors.clear());
        let (outcome, text) = run_kind(&h, ProofKind::Contrast);
        assert_eq!(outcome.status, ProofStatus::Vacuous, "{text}");
        assert_eq!(outcome.rows_considered, 1);
        assert!(text.contains(SKIP_NO_FLOOR), "{text}");
    }

    #[test]
    fn an_empty_register_is_vacuous_and_never_passed() {
        let h = Harness::new();
        sheet(&h, TWO_THEMES);
        let (outcome, text) = run_kind(&h, ProofKind::Contrast);
        assert_eq!(outcome.status, ProofStatus::Vacuous, "{text}");
        assert_eq!(outcome.exit_code, EXIT_VACUOUS);
        assert!(!text.contains("PASS:"), "no PASS beside a vacuity: {text}");
        assert!(text.contains("[register]"), "{text}");
    }

    #[test]
    fn every_theme_is_one_row_and_the_counts_add_up() {
        let h = Harness::new();
        sheet(
            &h,
            "\
:root { --surface: #ffffff; --control-border: #767676; }
.dark { --surface: #1a1a1a; --control-border: #9a9a9a; }
[data-theme='ember'] { --surface: #ffffff; --control-border: #6a6a6a; }
",
        );
        subject(&h, Status::Registered);
        with_floor(
            &h,
            "Card",
            Status::Proposed,
            "control-border",
            "surface",
            3.0,
        );
        let (outcome, text) = run_kind(&h, ProofKind::Contrast);
        assert_eq!(outcome.rows_enforced, 3, "three themes, one floor: {text}");
        assert_eq!(
            outcome.rows_considered, 4,
            "and the proposed record: {text}"
        );

        let record = h.last_proof(ProofKind::Contrast);
        let skipped: u64 = record.rows_skipped_reasons.values().sum();
        assert_eq!(record.rows_considered, record.rows_enforced + skipped);
    }

    // -- preconditions -------------------------------------------------------

    #[test]
    fn a_missing_stylesheet_is_a_precondition_failure_and_not_a_pass() {
        let h = Harness::new();
        subject(&h, Status::Registered);
        let error = h.run_kind_err(ProofKind::Contrast);
        assert_eq!(error.exit_code(), EXIT_PRECONDITION);
        assert!(
            error.to_string().contains("has been told nothing"),
            "{error}"
        );
    }

    #[test]
    fn a_missing_stylesheet_is_not_a_precondition_failure_when_there_is_nothing_to_measure() {
        let h = Harness::new();
        let (outcome, text) = run_kind(&h, ProofKind::Contrast);
        assert_eq!(
            outcome.status,
            ProofStatus::Vacuous,
            "an exit 2 that means `nothing to do` teaches a reader to ignore the one that means \
             `the record was never opened`: {text}"
        );
    }

    #[test]
    fn a_malformed_stylesheet_is_a_precondition_failure_and_not_a_pass() {
        let h = Harness::new();
        sheet(
            &h,
            ":root { --surface: #ffffff; --control-border: #767676; }\n}\n",
        );
        subject(&h, Status::Registered);
        let error = h.run_kind_err(ProofKind::Contrast);
        assert_eq!(error.exit_code(), EXIT_PRECONDITION);
        assert!(error.to_string().contains("did not run"), "{error}");
    }

    #[test]
    fn a_stylesheet_with_no_theme_scope_is_a_precondition_failure() {
        let h = Harness::new();
        sheet(&h, ".button { color: #767676; }\n");
        subject(&h, Status::Registered);
        let error = h.run_kind_err(ProofKind::Contrast);
        assert_eq!(error.exit_code(), EXIT_PRECONDITION);
        assert!(error.to_string().contains("no theme scope"), "{error}");
    }

    // -- what lands in the record --------------------------------------------

    #[test]
    fn the_run_records_what_it_reaches_and_what_it_does_not() {
        let h = Harness::new();
        sheet(&h, TWO_GOOD_THEMES);
        subject(&h, Status::Registered);
        run_kind(&h, ProofKind::Contrast);
        let record = h.last_proof(ProofKind::Contrast);
        for marker in [
            "[reach]",
            "[redaction]",
            "[ratio]",
            "[themes]",
            "[alpha]",
            "[scope]",
            "[unmeasured]",
            "[record]",
            "[themes-measured]",
            "[margin]",
        ] {
            assert!(
                record.notes.iter().any(|note| note.contains(marker)),
                "{marker} is missing from {:?}",
                record.notes
            );
        }
        assert!(
            record
                .notes
                .iter()
                .any(|note| note.contains("never that what ships is what was decided")),
            "{:?}",
            record.notes
        );
    }

    #[test]
    fn the_narrowest_clearance_is_recorded_on_a_passing_run() {
        let h = Harness::new();
        sheet(&h, TWO_GOOD_THEMES);
        subject(&h, Status::Registered);
        let (outcome, text) = run_kind(&h, ProofKind::Contrast);
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
        let record = h.last_proof(ProofKind::Contrast);
        let margin = record
            .notes
            .iter()
            .find(|note| note.starts_with("[margin]"))
            .unwrap_or_else(|| panic!("no margin note: {:?}", record.notes));
        assert!(
            margin.contains("against a floor of 3.00:1"),
            "the margin is only meaningful beside the floor it cleared: {margin}"
        );
    }

    /// The whole point of the redaction rule, tested against the gate that would
    /// fight this one. A captured contrast record lands under `.vds/`, which is
    /// the tree `no_stored_values` scans, so a finding carrying a resolved colour
    /// would make that proof fail forever on a file this one wrote, with no
    /// lawful way back: a record is never deleted.
    ///
    /// Every failing rule is exercised, one stylesheet at a time, because each
    /// one writes a different sentence into the record and the dangerous one is
    /// whichever is written last.
    #[test]
    fn a_captured_contrast_record_of_every_finding_does_not_fail_no_stored_values() {
        let h = Harness::new();
        subject(&h, Status::Registered);

        for css in [
            // R1, below the floor in the second theme.
            TWO_THEMES,
            // R2, declared in no scope in reach.
            ":root { --surface: #ffffff; }\n.dark { --surface: #1a1a1a; }\n",
            // R3, a shorthand rather than a colour.
            ":root { --surface: #ffffff; --control-border: 1px solid #767676; }\n",
            // R4, a translucent backdrop.
            ":root { --surface: rgba(255, 255, 255, 0.5); --control-border: #767676; }\n",
            // R5, a value a conditional at-rule guards.
            ":root { --surface: #ffffff; --control-border: #767676; }\n@media (min-width: 40rem) \
             { :root { --control-border: #eeeeee; } }\n",
            // R6, a palette scope the discovery could not classify.
            ":root { --surface: #ffffff; --control-border: #767676; }\n:root:not(.compact) { \
             --control-border: #eeeeee; }\n",
        ] {
            sheet(&h, css);
            let (contrast, text) = run_kind(&h, ProofKind::Contrast);
            assert_eq!(
                contrast.exit_code, EXIT_VIOLATION,
                "the failing direction is the one that writes into the record: {text}"
            );
        }

        // R7, a floor no measurement can fail. Seeded through the register, so
        // this one also covers what a floor figure looks like in a finding.
        sheet(&h, TWO_GOOD_THEMES);
        with_floor(
            &h,
            "Divider",
            Status::Registered,
            "control-border",
            "surface",
            0.5,
        );
        assert_eq!(
            run_kind(&h, ProofKind::Contrast).0.exit_code,
            EXIT_VIOLATION
        );

        let (guard, text) = run_kind(&h, ProofKind::NoStoredValues);
        assert_eq!(
            guard.exit_code, EXIT_PASSED,
            "a contrast record put a realisation under the record, and the two gates will now \
             fight forever over a file neither can delete:\n{text}"
        );
        assert!(guard.rows_enforced > 7, "every captured record was scanned");
    }

    #[test]
    fn a_finding_names_the_boundary_and_never_a_resolved_value() {
        let h = Harness::new();
        sheet(&h, TWO_THEMES);
        subject(&h, Status::Registered);
        run_kind(&h, ProofKind::Contrast);
        let record = h.last_proof(ProofKind::Contrast);
        let rendered = format!("{record:?}");
        for value in ["1a1a1a", "2a2a2a", "767676", "ffffff"] {
            assert!(
                !rendered.contains(value),
                "the record carries a resolved colour: {rendered}"
            );
        }
        assert!(rendered.contains("--control-border"), "{rendered}");
        assert!(rendered.contains(".dark"), "{rendered}");
    }

    // -- the pure functions --------------------------------------------------

    #[test]
    fn a_floor_may_name_its_property_with_or_without_the_leading_dashes() {
        assert_eq!(
            custom_property("control-border").as_deref(),
            Some("--control-border")
        );
        assert_eq!(
            custom_property("--control-border").as_deref(),
            Some("--control-border")
        );
        assert_eq!(custom_property("  surface  ").as_deref(), Some("--surface"));
    }

    /// Reading the field as a colour would MEASURE a value stored in the
    /// register, which rewards the storing form the whole system forbids.
    #[test]
    fn a_floor_field_that_holds_a_value_rather_than_a_name_is_refused() {
        for stored in ["#ebebeb", "rgb(1, 2, 3)", "var(--x)", "", "  ", "a b"] {
            assert_eq!(custom_property(stored), None, "{stored:?}");
        }
    }

    /// The top of the scale is reached by exactly one pair of colours, so
    /// recording it as a point value would record those two colours.
    #[test]
    fn the_top_of_the_scale_is_recorded_as_a_bound_and_never_as_a_point() {
        assert_eq!(recorded_ratio(21.0), "20.99 or more");
        assert_eq!(recorded_ratio(20.995), "20.99 or more");
        assert_eq!(recorded_ratio(20.99), "20.99");
        assert_eq!(recorded_ratio(4.5), "4.50");
        assert_eq!(
            recorded_ratio(2.999),
            "2.99",
            "truncated and never rounded, so a displayed 3.00 is never a 2.999 dressed up"
        );
    }

    #[test]
    fn the_redactor_takes_out_every_realisation_shape_a_selector_could_carry() {
        for text in [
            "#ebebeb",
            ".bg-[#ebebeb]",
            "--space-12px",
            "[data-x='0.5rem']",
            "rgb(1,2,3)",
            "--t-160ms",
        ] {
            let out = redacted(text);
            assert!(out.contains(REDACTED), "{text:?} was not redacted: {out}");
        }
    }

    /// The other direction, and the one that decides whether this is usable: a
    /// property name a reader has to be able to grep for must survive intact.
    #[test]
    fn the_redactor_leaves_an_ordinary_property_name_and_selector_alone() {
        for text in [
            "--control-border",
            "--surface",
            ".dark",
            ":root",
            "[data-theme='ember']",
            "#app",
            "--radius-2",
            "--scale-100",
        ] {
            assert_eq!(
                redacted(text),
                text,
                "a false redaction sends a reader looking in the wrong place"
            );
        }
    }

    /// A unit this redactor misses is a unit that reaches a proof record and
    /// fails `no_stored_values` on a file this proof wrote.
    #[test]
    fn the_redactor_covers_every_unit_the_record_scanner_enforces() {
        for unit in crate::no_stored_values::LENGTH_UNITS {
            assert!(
                UNITS.contains(unit),
                "{unit:?} is enforced under the record and is not redacted out of a finding"
            );
        }
    }

    /// The window is the only thing that decides how far the redactor looks, so
    /// a longer function name added to the list would be seen by nothing.
    #[test]
    fn every_colour_function_fits_in_the_window_the_redactor_examines() {
        for function in COLOUR_FUNCTIONS {
            assert!(
                function.len() < FUNCTION_WINDOW,
                "{function:?} and its parenthesis are longer than the window, so it is in the \
                 list and out of reach"
            );
        }
    }

    /// The stylesheet is an INPUT, and the evidence digest has to move when it
    /// does. A proof that forgot to record it would pass every other test in this
    /// module while citing evidence that outlived the thing it measured, and
    /// every warrant granted on it would keep looking current after the sheet
    /// changed underneath it (VDS S-6(4)).
    ///
    /// The edit is deliberately one no measurement can see: a rule that declares
    /// no custom property is not a scope, so the findings, the rows and the notes
    /// are identical and the file digest is the only thing left that could move.
    #[test]
    fn the_evidence_digest_moves_when_the_stylesheet_moves() {
        let h = Harness::new();
        sheet(&h, TWO_GOOD_THEMES);
        subject(&h, Status::Registered);

        run_kind(&h, ProofKind::Contrast);
        let before = h.last_proof(ProofKind::Contrast);

        sheet(
            &h,
            &format!("{TWO_GOOD_THEMES}.unrelated {{ padding: 0; }}\n"),
        );
        run_kind(&h, ProofKind::Contrast);
        let after = h.last_proof(ProofKind::Contrast);

        assert_eq!(before.violations, after.violations, "the same findings");
        assert_eq!(before.notes, after.notes, "and the same notes");
        assert_ne!(
            before.digest, after.digest,
            "the stylesheet is not among this run's inputs, so a warrant citing it would keep \
             looking current after the record it measured had changed"
        );
    }

    /// One undeclared property across every component and every theme is
    /// hundreds of true findings about one defect, and a record nobody reads is a
    /// different way of hiding them. Nothing is dropped in silence.
    #[test]
    fn many_findings_are_capped_and_the_remainder_is_counted() {
        let h = Harness::new();
        // Both themes below the floor, so every row fails.
        sheet(
            &h,
            "\
:root { --surface: #ffffff; --control-border: #f0f0f0; }
.dark { --surface: #1a1a1a; --control-border: #2a2a2a; }
",
        );
        let components = 55;
        for index in 0..components {
            with_floor(
                &h,
                &format!("Button{index}"),
                Status::Registered,
                "control-border",
                "surface",
                3.0,
            );
        }

        let (outcome, _) = run_kind(&h, ProofKind::Contrast);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION);
        assert_eq!(outcome.rows_enforced, components * 2);

        let record = h.last_proof(ProofKind::Contrast);
        assert_eq!(record.violations.len(), MAX_FINDINGS + 1);
        assert!(
            record
                .violations
                .iter()
                .any(|violation| violation.actual.contains("110 boundaries at fault")),
            "the remainder has to be counted, not dropped: {:?}",
            record.violations.last()
        );
    }

    /// The subject is the record VDS S-2(3) names, and a run says which file it
    /// read. A project whose tokens ship from somewhere else is not covered, and
    /// the record has to carry that rather than leave a reader to assume the
    /// gate found whatever it needed.
    #[test]
    fn the_run_names_the_stylesheet_it_measured() {
        let h = Harness::new();
        sheet(&h, TWO_GOOD_THEMES);
        subject(&h, Status::Registered);
        run_kind(&h, ProofKind::Contrast);
        let record = h.last_proof(ProofKind::Contrast);
        assert!(
            record
                .notes
                .iter()
                .any(|note| note.contains(SHIPPED_STYLESHEET)),
            "{:?}",
            record.notes
        );
    }
}
