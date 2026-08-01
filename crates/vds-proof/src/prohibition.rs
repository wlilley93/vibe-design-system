//! The `prohibition` proof. A pattern asserted ABSENT from an enumerated scope.
//!
//! Draft S-7B, ENACTMENT PENDING (SUBMISSION-VDS-013). The thirteenth kind, and
//! the instrument for the "no container radius in body regions" / "no dotfield
//! behind main areas" class of directive. On the migration this lane was drawn
//! from, every such directive lived as prose beside twenty-eight gates that
//! could not read it, and prose is not enforcement.
//!
//! # The two silent failures this kind refuses
//!
//! An absence check has exactly two ways to rot, and both are silent:
//!
//!   1. **The scope narrows.** A file renamed out of the glob takes its
//!      violations with it, and the pass over the smaller population reads
//!      exactly like a pass over the original. So the expansion at registration
//!      is RECORDED IN THE RECORD, and a recorded file the globs no longer
//!      reach is a fatal finding (R3), never a disappearance.
//!   2. **The pattern matches nothing anywhere.** A prohibition over an empty
//!      scope, or with an empty pattern, cannot fail, and a check that cannot
//!      fail is the defect this whole registry exists to prevent. Refused (R4,
//!      R5), and the row is NOT enforced.
//!
//! # The rules
//!
//! One row is one prohibition record.
//!
//!   R1  the pattern survives in scope. Fatal, and the finding names every
//!       surviving site as `file:line`, because "the pattern is present" names
//!       no work and a named site is a job.
//!   R2  a file in scope could not be read. Fatal: an unread file is exactly
//!       where a surviving site hides, and skipping it narrows the claim with
//!       nothing saying so.
//!   R3  the scope NARROWED: a file in the recorded expansion is no longer
//!       matched by the globs (renamed, deleted, or the glob edited). Fatal.
//!       Deliberate removals re-record the expansion through the front door.
//!   R4  the recorded expansion is empty. A prohibition over nothing cannot
//!       fail; fatal and not enforced.
//!   R5  the pattern is empty or whitespace. Matches everything or nothing
//!       depending on the reader's mood; fatal and not enforced.
//!   W1  the scope GREW: files now matched that the recorded expansion does
//!       not list. They are scanned (R1 covers them - growth must not create
//!       an unenforced shadow), and the growth is warned so the record gets
//!       re-expanded and the baseline stays honest.

use std::collections::BTreeSet;
use std::io::Write;

use vds_core::{ProofKind, Result, Violation};

use crate::ProofContext;
use crate::run::{Outcome, Verdict};

pub const GATE: &str = "crates/vds-proof/src/prohibition.rs";

const RULE_PRESENT: &str = "draft S-7B prohibition R1: the pattern must be absent from the scope";
const RULE_UNREADABLE: &str =
    "draft S-7B prohibition R2: an unread file is where a surviving site hides";
const RULE_NARROWED: &str =
    "draft S-7B prohibition R3: the scope cannot silently narrow; the expansion is recorded";
const RULE_EMPTY_SCOPE: &str = "VDS S-7(2)(4) prohibition R4: a prohibition over nothing";
const RULE_EMPTY_PATTERN: &str = "VDS S-7(2)(4) prohibition R5: an empty pattern";
const RULE_GREW: &str =
    "draft S-7B prohibition W1: the scope grew past its recorded expansion; re-record it";

/// How many surviving sites a finding names in full before summarising. Every
/// site is COUNTED either way; the cap keeps one thousand-hit pattern from
/// making the record unreadable.
const NAMED_SITES: usize = 12;

pub fn run(ctx: &ProofContext, out: &mut dyn Write) -> Result<Outcome> {
    let project = ctx.project;
    let store = ctx.store();
    let mut run = ctx.new_run(ProofKind::Prohibition, GATE);

    let records = store.read_prohibitions()?;
    for record in &records {
        run.input_file(&record.path)?;
    }

    for record in &records {
        let prohibition = &record.value;
        let location = format!("{} [{:?}]", prohibition.id, prohibition.pattern);

        if !prohibition.status.is_enforceable() {
            run.row(Verdict::Skipped("prohibition_not_in_an_enforceable_status"));
            continue;
        }

        if prohibition.pattern.trim().is_empty() {
            run.row(Verdict::Skipped("empty_pattern"));
            run.fail(Violation::fatal(
                location.clone(),
                RULE_EMPTY_PATTERN,
                "a non-empty literal pattern",
                "the pattern is empty or whitespace, so this row matches everything or \
                 nothing and cannot state a checkable absence."
                    .to_owned(),
            ));
            continue;
        }

        if prohibition.expansion.is_empty() {
            run.row(Verdict::Skipped("empty_recorded_expansion"));
            run.fail(Violation::fatal(
                location.clone(),
                RULE_EMPTY_SCOPE,
                "a recorded expansion naming at least one file",
                "the recorded expansion is empty, so no file is in scope and nothing the \
                 codebase could contain would fail this row. A prohibition over nothing is \
                 not a control, it is the appearance of one."
                    .to_owned(),
            ));
            continue;
        }

        // ONE expansion of the declared globs, reused by R3 and W1, so the two
        // rules cannot disagree about what the scope currently is.
        let matched = vds_scan::glob::match_globs(&project.root, &prohibition.scope)?;
        let current: BTreeSet<String> = matched.iter().map(|p| project.rel(p)).collect();
        let recorded: BTreeSet<String> = prohibition.expansion.iter().cloned().collect();

        // R3 before R1: a narrowed scope makes the absence claim smaller than
        // the record states, so the row must fail even where every file still
        // matched is clean.
        let lost: Vec<&String> = recorded.difference(&current).collect();
        if !lost.is_empty() {
            run.row(Verdict::Enforced);
            run.fail(Violation::fatal(
                location.clone(),
                RULE_NARROWED,
                format!(
                    "every file in the recorded expansion ({}) still matched by the scope",
                    recorded.len()
                ),
                format!(
                    "{} recorded file(s) are no longer matched: {}. Renamed, deleted, or \
                     the glob was edited; any of the three shrinks the population and a \
                     pass over the smaller scope reads exactly like a pass over the \
                     original. If the removal is deliberate, re-record the expansion \
                     through `vds prohibition add` so the baseline says what the scope is.",
                    lost.len(),
                    lost.iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
            continue;
        }

        run.row(Verdict::Enforced);

        // W1: growth. Scanned below regardless - the union is what R1 reads -
        // so growth cannot open an unenforced shadow while it waits to be
        // re-recorded.
        let grown: Vec<&String> = current.difference(&recorded).collect();
        if !grown.is_empty() {
            run.warn(Violation::fatal(
                location.clone(),
                RULE_GREW,
                "the recorded expansion to equal the current one",
                format!(
                    "{} file(s) now match the scope and are not in the recorded expansion: \
                     {}. They ARE scanned by this run; re-record the expansion so the \
                     baseline stays honest.",
                    grown.len(),
                    grown
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
        }

        // R1 and R2 over the union of current matches. The recorded set's
        // survivors were handled by R3 above, so `current` is the whole scope.
        let mut surviving: Vec<String> = Vec::new();
        for path in &matched {
            let rel = project.rel(path);
            let text = match std::fs::read_to_string(path) {
                Ok(text) => text,
                Err(error) => {
                    run.fail(Violation::fatal(
                        rel.clone(),
                        RULE_UNREADABLE,
                        "every file in scope to be readable",
                        format!(
                            "could not be read ({error}), and an unread file is exactly \
                             where a surviving site hides."
                        ),
                    ));
                    continue;
                }
            };
            for (index, line) in text.lines().enumerate() {
                if line.contains(&prohibition.pattern) {
                    surviving.push(format!("{rel}:{}", index + 1));
                }
            }
        }

        if !surviving.is_empty() {
            let shown = surviving
                .iter()
                .take(NAMED_SITES)
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(", ");
            let tail = if surviving.len() > NAMED_SITES {
                format!(", and {} more", surviving.len() - NAMED_SITES)
            } else {
                String::new()
            };
            run.fail(Violation::fatal(
                location.clone(),
                RULE_PRESENT,
                format!(
                    "{:?} absent from all {} file(s) in scope{}",
                    prohibition.pattern,
                    current.len(),
                    prohibition
                        .because
                        .as_deref()
                        .map(|b| format!(" ({b})"))
                        .unwrap_or_default()
                ),
                format!(
                    "{} surviving site(s): {shown}{tail}. Each is a job; the directive is \
                     not met while any survives.",
                    surviving.len()
                ),
            ));
        }
    }

    if records.is_empty() {
        run.note(
            "[scope] no prohibition is registered, so every row is skipped and this run is \
             vacuous. That is the honest state of a project with no absence directives, and \
             it is NOT evidence (VDS S-7(2)(4)).",
        );
    }

    run.finish(&ctx.capture_options()?, out)
}

#[cfg(test)]
mod proof_tests {
    use crate::testing::{Harness, run_kind};
    use vds_core::{EXIT_PASSED, EXIT_VACUOUS, EXIT_VIOLATION, ProofKind, ProofStatus};

    #[test]
    fn a_pattern_absent_from_its_scope_passes() {
        let h = Harness::new();
        h.write("src/components/body/panel.tsx", "export const p = 1\n");
        h.prohibition("rounded-", &["src/components/body/**/*.tsx"]);
        let (outcome, text) = run_kind(&h, ProofKind::Prohibition);
        assert_eq!(outcome.status, ProofStatus::Passed, "{text}");
        assert_eq!(outcome.exit_code, EXIT_PASSED, "{text}");
        assert_eq!(outcome.rows_enforced, 1, "{text}");
    }

    /// THE failing-direction test VDS S-7(2)(2) requires, and the one the
    /// enforcement lock names: the pattern SURVIVES, and the finding names the
    /// surviving sites rather than announcing a count.
    #[test]
    fn a_surviving_site_fails_and_is_named_by_file_and_line() {
        let h = Harness::new();
        h.write(
            "src/components/body/panel.tsx",
            "export const p = 1\nconst cls = \"rounded-xl p-4\"\n",
        );
        h.prohibition("rounded-", &["src/components/body/**/*.tsx"]);
        let (outcome, text) = run_kind(&h, ProofKind::Prohibition);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert_eq!(outcome.status, ProofStatus::Failed);
        assert!(
            text.contains("src/components/body/panel.tsx:2"),
            "the surviving site must be named by file and line: {text}"
        );
        assert_eq!(outcome.rows_enforced, 1, "{text}");
    }

    /// The anti-narrowing rule, seeded the way narrowing actually happens: the
    /// file is renamed out of the glob and every remaining file is clean.
    #[test]
    fn a_scope_that_narrowed_fails_even_though_every_remaining_file_is_clean() {
        let h = Harness::new();
        h.write(
            "src/components/body/panel.tsx",
            "const cls = \"rounded-xl\"\n",
        );
        h.write("src/components/body/other.tsx", "export const ok = 1\n");
        h.prohibition("rounded-", &["src/components/body/**/*.tsx"]);
        // The rename: the offending file leaves the glob, violations and all.
        std::fs::rename(
            h.root().join("src/components/body/panel.tsx"),
            h.root().join("src/components/panel.tsx"),
        )
        .unwrap();
        let (outcome, text) = run_kind(&h, ProofKind::Prohibition);
        assert_eq!(
            outcome.exit_code, EXIT_VIOLATION,
            "a silently narrowed scope must not pass: {text}"
        );
        assert!(text.contains("no longer matched"), "{text}");
        assert!(text.contains("panel.tsx"), "{text}");
    }

    #[test]
    fn a_prohibition_over_an_empty_expansion_cannot_fail_and_is_refused() {
        let h = Harness::new();
        h.prohibition("rounded-", &["src/components/nowhere/**/*.tsx"]);
        let (outcome, text) = run_kind(&h, ProofKind::Prohibition);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("prohibition over nothing"), "{text}");
        assert_eq!(
            outcome.rows_enforced, 0,
            "a row that cannot fail was not checked: {text}"
        );
    }

    #[test]
    fn an_empty_pattern_is_refused_rather_than_matched() {
        let h = Harness::new();
        h.write("src/components/body/panel.tsx", "export const p = 1\n");
        h.prohibition("  ", &["src/components/body/**/*.tsx"]);
        let (outcome, text) = run_kind(&h, ProofKind::Prohibition);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION, "{text}");
        assert!(text.contains("empty or whitespace"), "{text}");
        assert_eq!(outcome.rows_enforced, 0, "{text}");
    }

    /// Growth is scanned AND warned: a new file in scope must not be an
    /// unenforced shadow while it waits to be re-recorded, and the warning is
    /// what keeps the recorded baseline honest.
    #[test]
    fn a_grown_scope_is_scanned_and_warned_not_silently_absorbed() {
        let h = Harness::new();
        h.write("src/components/body/panel.tsx", "export const p = 1\n");
        h.prohibition("rounded-", &["src/components/body/**/*.tsx"]);
        // A NEW offending file appears after registration.
        h.write(
            "src/components/body/late.tsx",
            "const cls = \"rounded-md\"\n",
        );
        let (outcome, text) = run_kind(&h, ProofKind::Prohibition);
        assert_eq!(
            outcome.exit_code, EXIT_VIOLATION,
            "the new file's violation must be enforced: {text}"
        );
        assert!(text.contains("late.tsx:1"), "{text}");
        assert!(
            text.contains("re-record the expansion"),
            "growth must be warned so the baseline is re-recorded: {text}"
        );
    }

    #[test]
    fn a_project_with_no_prohibition_is_vacuous_and_says_it_is_not_evidence() {
        let h = Harness::new();
        let (outcome, text) = run_kind(&h, ProofKind::Prohibition);
        assert_eq!(outcome.status, ProofStatus::Vacuous, "{text}");
        assert_eq!(outcome.exit_code, EXIT_VACUOUS, "{text}");
        assert!(text.contains("NOT evidence"), "{text}");
    }

    #[test]
    fn a_non_enforceable_record_is_skipped_with_the_reason_counted() {
        let h = Harness::new();
        h.write("src/components/body/panel.tsx", "rounded-\n");
        h.prohibition_with_status("rounded-", &["src/components/body/**/*.tsx"], "proposed");
        let (outcome, text) = run_kind(&h, ProofKind::Prohibition);
        assert_eq!(outcome.status, ProofStatus::Vacuous, "{text}");
        assert!(
            text.contains("prohibition_not_in_an_enforceable_status"),
            "{text}"
        );
    }
}
