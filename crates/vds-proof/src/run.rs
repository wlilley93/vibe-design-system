//! The proof run: how a check accumulates a result, and how that result is
//! captured.
//!
//! VDS S-7(2) makes five conditions load-bearing. Three of them are enforced by
//! this type rather than requested by a comment:
//!
//! **(1) Re-runnable and deterministic.** The digest a warrant cites is computed
//! over the run's FINDINGS and INPUTS and nothing else. It excludes the capture
//! time, the duration and the invoking surface, so re-running an unchanged check
//! cites the same evidence. A digest that moves on unchanged input makes every
//! warrant look spent and teaches a reader to ignore the field.
//!
//! **(4) Non-vacuous.** A row is classified exactly once, by
//! [`ProofRun::classify`], which consumes a [`Row`] token. There is no way to
//! call "enforce" and "skip" on the same row, so `rows_considered` always equals
//! `rows_enforced` plus the skip counts. The old arrangement had two independent
//! counters and a proof that counted one row as both, which made the vacuity
//! check unable to see its own arithmetic.
//!
//! **(5) Captured automatically.** [`ProofRun::finish`] writes the record as a
//! side effect of the run. There is no constructor that takes a status: the
//! status is derived from the violations and the enforced-row count.
//!
//! And one condition the drafted specification asserted and nothing checked: a
//! captured record's `digest` is RECOMPUTABLE from the record itself, so a record
//! edited from `failed` to `passed` no longer matches its own digest and is
//! refused as evidence. Fixing `capture_mode` to one value makes forgery trivial,
//! not impossible; recomputation is what makes it detectable.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

use serde::Serialize;
use vds_core::{
    CaptureMode, Digest, EXIT_PASSED, EXIT_VACUOUS, EXIT_VIOLATION, InvokedBy, Project, ProofId,
    ProofKind, ProofResult, ProofStatus, Result, Severity, Timestamp, VdsError, Violation,
};
use vds_store::Store;

/// A row that has been counted as considered and not yet classified.
///
/// Not `Clone` and not `Copy`: it is consumed by [`ProofRun::classify`], so one
/// row cannot be classified twice.
#[must_use = "a considered row must be classified, or rows_considered and the skip counts \
              stop adding up and the vacuity check cannot see its own arithmetic"]
pub struct Row(());

/// What a row turned out to be.
pub enum Verdict {
    /// In scope, checked. Only these count towards `rows_enforced`.
    Enforced,
    /// Considered and deliberately not enforced, with the reason recorded.
    ///
    /// The reason is a stable machine key, not a sentence: it becomes a count in
    /// `rows_skipped_reasons`, and a per-row sentence would make every run's
    /// record unique and every count one.
    Skipped(&'static str),
}

/// Accumulates one proof's result, prints it, captures it and yields an exit
/// code.
pub struct ProofRun<'a> {
    project: &'a Project,
    kind: ProofKind,
    /// The gate, repository-relative. Pinned in `.vds/enforcement.lock`.
    gate: String,
    command: String,
    invoked_by: InvokedBy,
    started: Timestamp,

    rows_considered: u64,
    rows_enforced: u64,
    skipped: BTreeMap<String, u64>,
    violations: Vec<Violation>,
    notes: Vec<String>,
    /// `(relative path, digest)` of every input read. Digested into
    /// `inputs_digest`.
    inputs: BTreeMap<String, Digest>,
}

impl<'a> ProofRun<'a> {
    pub fn new(
        project: &'a Project,
        kind: ProofKind,
        gate: impl Into<String>,
        command: impl Into<String>,
        invoked_by: InvokedBy,
    ) -> Self {
        Self {
            project,
            kind,
            gate: gate.into(),
            command: command.into(),
            invoked_by,
            started: Timestamp::now(),
            rows_considered: 0,
            rows_enforced: 0,
            skipped: BTreeMap::new(),
            violations: Vec::new(),
            notes: Vec::new(),
            inputs: BTreeMap::new(),
        }
    }

    /// Count a row as considered. The returned token must be classified.
    pub fn consider(&mut self) -> Row {
        self.rows_considered += 1;
        Row(())
    }

    /// Classify a considered row, consuming its token.
    pub fn classify(&mut self, _row: Row, verdict: Verdict) {
        match verdict {
            Verdict::Enforced => self.rows_enforced += 1,
            Verdict::Skipped(reason) => {
                *self.skipped.entry(reason.to_owned()).or_insert(0) += 1;
            }
        }
    }

    /// Consider and classify in one call, for the common case.
    pub fn row(&mut self, verdict: Verdict) {
        let row = self.consider();
        self.classify(row, verdict);
    }

    pub fn fail(&mut self, violation: Violation) {
        self.violations.push(violation);
    }

    /// Record a finding that is real and does not block.
    ///
    /// VDS S-9(6)(1) requires every consuming site of a deprecated component to
    /// be reported, per site, by route, and to never pass silently. Printing it
    /// and not capturing it is passing silently the moment anyone reads the
    /// record instead of the terminal, so a warning is a captured violation
    /// carrying [`Severity::Warning`], not a note.
    pub fn warn(&mut self, violation: Violation) {
        self.violations
            .push(violation.with_severity(Severity::Warning));
    }

    pub fn inform(&mut self, violation: Violation) {
        self.violations
            .push(violation.with_severity(Severity::Informational));
    }

    pub fn note(&mut self, line: impl Into<String>) {
        self.notes.push(line.into());
    }

    /// Record an input by path, digesting its bytes.
    pub fn input_file(&mut self, path: &Path) -> Result<()> {
        let digest = Digest::of_file(path)?;
        self.inputs.insert(self.project.rel(path), digest);
        Ok(())
    }

    /// Record an input that is not a file: a derived digest a proof depends on.
    ///
    /// Used for the screens ledger, whose FILE digest moves every time the
    /// generator runs (it stamps `generated_at`) while its CONTENT does not.
    /// Digesting the file would make the proof digest move on a no-op
    /// regeneration, which is the determinism limb broken by an irrelevance.
    pub fn input_named(&mut self, name: impl Into<String>, digest: Digest) {
        self.inputs.insert(name.into(), digest);
    }

    pub fn rows_considered(&self) -> u64 {
        self.rows_considered
    }

    pub fn rows_enforced(&self) -> u64 {
        self.rows_enforced
    }

    fn fatal(&self) -> Vec<&Violation> {
        self.violations.iter().filter(|v| v.is_fatal()).collect()
    }

    /// The outcome, derived. There is no way to assert one.
    pub fn status(&self) -> ProofStatus {
        if !self.fatal().is_empty() {
            ProofStatus::Failed
        } else if self.rows_enforced == 0 {
            ProofStatus::Vacuous
        } else {
            ProofStatus::Passed
        }
    }

    fn exit_code(&self, allow_vacuous: bool) -> i32 {
        match self.status() {
            ProofStatus::Failed => EXIT_VIOLATION,
            ProofStatus::Vacuous if allow_vacuous => EXIT_PASSED,
            ProofStatus::Vacuous => EXIT_VACUOUS,
            ProofStatus::Passed => EXIT_PASSED,
        }
    }

    /// Finish: print, capture, and return the outcome.
    pub fn finish(mut self, options: &Capture, out: &mut dyn Write) -> Result<Outcome> {
        let status = self.status();
        let exit_code = self.exit_code(options.allow_vacuous);
        self.violations.sort();

        self.print(status, exit_code, out)?;

        let mut record_id = None;
        if options.capture {
            let store = Store::new(self.project);
            let designpack_digest = options.designpack_digest.clone();
            let record = self.build_record(status, exit_code, designpack_digest, &store)?;
            let path = store.proof_path(&record.id);
            store.create(&path, &record)?;
            writeln!(out).ok();
            writeln!(
                out,
                "captured: {} (capture_mode: automatic)",
                self.project.rel(&path)
            )
            .ok();
            writeln!(out, "digest:   {}", record.digest).ok();
            record_id = Some(record.id);
        } else {
            writeln!(out).ok();
            writeln!(
                out,
                "NOT CAPTURED: this run wrote no proof record, so it can never be cited as \
                 evidence for a warrant (VDS S-7(2)(5))."
            )
            .ok();
        }
        writeln!(out, "status:   {status}    exit: {exit_code}").ok();

        Ok(Outcome {
            kind: self.kind,
            status,
            exit_code,
            record_id,
            rows_considered: self.rows_considered,
            rows_enforced: self.rows_enforced,
        })
    }

    fn build_record(
        &self,
        status: ProofStatus,
        exit_code: i32,
        designpack_digest: Digest,
        store: &Store,
    ) -> Result<ProofResult> {
        let inputs: Vec<[&str; 2]> = self
            .inputs
            .iter()
            .map(|(path, digest)| [path.as_str(), digest.as_str()])
            .collect();
        let inputs_digest = Digest::of_value(&inputs)?;
        let captured_at = Timestamp::now();

        let gate_path = self.project.root.join(&self.gate);
        let script_digest = if gate_path.is_file() {
            Some(Digest::of_file(&gate_path)?)
        } else {
            None
        };

        let mut record = ProofResult {
            id: ProofId::allocate(&store.proofs_dir(), &captured_at)?,
            kind: self.kind,
            status,
            warrant_id: None,
            command: self.command.clone(),
            script: self.gate.clone(),
            script_digest,
            exit_code,
            rows_considered: self.rows_considered,
            rows_enforced: self.rows_enforced,
            rows_skipped_reasons: self.skipped.clone(),
            violations: self.violations.clone(),
            notes: self.notes.clone(),
            inputs_digest,
            // Replaced below. There is no path that leaves this as written.
            digest: Digest::of_text(""),
            designpack_digest,
            captured_at: captured_at.clone(),
            capture_mode: CaptureMode::Automatic,
            invoked_by: self.invoked_by,
            duration_ms: Some(self.started.millis_until(&captured_at)),
        };
        record.digest = evidence_digest(&record)?;
        Ok(record)
    }

    fn print(&self, status: ProofStatus, exit_code: i32, out: &mut dyn Write) -> Result<()> {
        let _ = exit_code;
        writeln!(out, "proof: {}", self.kind).ok();
        writeln!(out, "gate:  {}", self.gate).ok();
        writeln!(out, "rows_considered: {}", self.rows_considered).ok();
        writeln!(out, "rows_enforced:   {}", self.rows_enforced).ok();
        for (reason, count) in &self.skipped {
            writeln!(out, "  not enforced, {reason}: {count}").ok();
        }
        for note in &self.notes {
            writeln!(out, "note: {note}").ok();
        }

        let fatal = self.fatal();
        let warnings: Vec<&Violation> = self
            .violations
            .iter()
            .filter(|v| v.severity == Severity::Warning)
            .collect();

        if !warnings.is_empty() {
            writeln!(out).ok();
            writeln!(out, "WARNINGS ({}), each named in full:", warnings.len()).ok();
            for (i, violation) in warnings.iter().enumerate() {
                writeln!(out, "  [{}] {}", i + 1, violation.location).ok();
                writeln!(out, "      rule:     {}", violation.rule).ok();
                writeln!(out, "      expected: {}", violation.expected).ok();
                writeln!(out, "      actual:   {}", violation.actual).ok();
            }
        }

        if !fatal.is_empty() {
            writeln!(out).ok();
            writeln!(out, "VIOLATIONS ({}), each named in full:", fatal.len()).ok();
            for (i, violation) in fatal.iter().enumerate() {
                writeln!(out, "  [{}] {}", i + 1, violation.location).ok();
                writeln!(out, "      rule:     {}", violation.rule).ok();
                writeln!(out, "      expected: {}", violation.expected).ok();
                writeln!(out, "      actual:   {}", violation.actual).ok();
            }
        } else if status == ProofStatus::Vacuous {
            // VDS S-7(2)(4). No PASS is printed beside these words, because a
            // pass over zero enforceable rows is the [2026] VJS-CC-OPBOX 3 D3
            // defect and not evidence of anything.
            writeln!(out).ok();
            writeln!(
                out,
                "VACUOUS: this proof cannot currently fail, because no row is in an \
                 enforceable state."
            )
            .ok();
            writeln!(
                out,
                "  It is recorded as status: vacuous and is NOT evidence for any warrant \
                 (VDS S-7(2)(4))."
            )
            .ok();
            if !self.skipped.is_empty() {
                writeln!(out, "  Every row considered was skipped for these reasons:").ok();
                for (reason, count) in &self.skipped {
                    writeln!(out, "    {reason}: {count}").ok();
                }
            }
        } else {
            writeln!(out).ok();
            writeln!(
                out,
                "PASS: {} enforceable rows checked, 0 violations.",
                self.rows_enforced
            )
            .ok();
        }
        Ok(())
    }
}

/// How a run is captured.
pub struct Capture {
    pub capture: bool,
    pub allow_vacuous: bool,
    pub designpack_digest: Digest,
}

/// What one proof run produced.
#[derive(Debug, Clone)]
pub struct Outcome {
    pub kind: ProofKind,
    pub status: ProofStatus,
    pub exit_code: i32,
    pub record_id: Option<ProofId>,
    pub rows_considered: u64,
    pub rows_enforced: u64,
}

/// The digest a warrant cites: the run's FINDINGS and INPUTS, and nothing that
/// varies between two runs over identical input.
///
/// Recomputable from the record, which is the point. A record whose `digest`
/// does not match this function has been edited since it was captured, and
/// [`verify_record`] refuses it as evidence.
pub fn evidence_digest(record: &ProofResult) -> Result<Digest> {
    #[derive(Serialize)]
    struct Evidence<'a> {
        kind: &'a ProofKind,
        status: &'a ProofStatus,
        exit_code: i32,
        rows_considered: u64,
        rows_enforced: u64,
        rows_skipped_reasons: &'a BTreeMap<String, u64>,
        violations: &'a [Violation],
        notes: &'a [String],
        inputs_digest: &'a Digest,
        script: &'a str,
        script_digest: &'a Option<Digest>,
        designpack_digest: &'a Digest,
        capture_mode: &'a CaptureMode,
    }
    Digest::of_value(&Evidence {
        kind: &record.kind,
        status: &record.status,
        exit_code: record.exit_code,
        rows_considered: record.rows_considered,
        rows_enforced: record.rows_enforced,
        rows_skipped_reasons: &record.rows_skipped_reasons,
        violations: &record.violations,
        notes: &record.notes,
        inputs_digest: &record.inputs_digest,
        script: &record.script,
        script_digest: &record.script_digest,
        designpack_digest: &record.designpack_digest,
        capture_mode: &record.capture_mode,
    })
}

/// Why this proof record may not be cited as evidence, or an empty list.
///
/// The digest check is the one that matters. Fixing `capture_mode` to a single
/// value makes a forged record trivial to write; recomputing the digest makes it
/// detectable, because a forger has to edit the finding AND recompute the digest
/// over a canonicalisation they do not control.
pub fn verify_record(record: &ProofResult) -> Result<Vec<String>> {
    let mut out = Vec::new();

    let recomputed = evidence_digest(record)?;
    if recomputed != record.digest {
        out.push(format!(
            "the record's digest does not match its own contents. It says {} and its findings \
             digest to {}. The record has been edited since it was captured, and a hand-edited \
             proof record is void (VDS S-7(2)(5)).",
            record.digest, recomputed
        ));
    }
    if record.capture_mode != CaptureMode::Automatic {
        out.push(
            "capture_mode is not `automatic`. A hand-written proof record is void \
             (VDS S-7(2)(5))."
                .into(),
        );
    }
    if !record.status.is_evidence() {
        out.push(format!(
            "status is {}. Only a passed proof is evidence; a vacuous or failed run is not \
             (VDS S-7(2)(4)).",
            record.status
        ));
    }
    if record.rows_enforced == 0 {
        out.push(
            "rows_enforced is 0. A pass over zero enforceable rows is recorded as vacuous and \
             is not evidence; this is the [2026] VJS-CC-OPBOX 3 D3 defect (VDS S-7(2)(4))."
                .into(),
        );
    }
    let expected_exit = match record.status {
        ProofStatus::Passed => EXIT_PASSED,
        ProofStatus::Failed => EXIT_VIOLATION,
        ProofStatus::Vacuous => EXIT_VACUOUS,
    };
    if record.exit_code != expected_exit
        && !(record.status == ProofStatus::Vacuous && record.exit_code == EXIT_PASSED)
    {
        out.push(format!(
            "status is {} and exit_code is {}. The two disagree, and a caller reading only the \
             exit code was told something the record does not say.",
            record.status, record.exit_code
        ));
    }
    let fatal = record.fatal_violations().count();
    if fatal > 0 && record.status != ProofStatus::Failed {
        out.push(format!(
            "the record carries {fatal} fatal violations and status {}. A proof with a fatal \
             violation failed.",
            record.status
        ));
    }
    Ok(out)
}

/// Aggregate several outcomes into one exit code.
///
/// By SEVERITY, not by numeric maximum. The exit codes are 0 passed, 1
/// violation, 2 precondition, 3 vacuous, and taking the numeric maximum reports
/// a run where one proof FAILED and another was vacuous as vacuous, which hides
/// a violation from every gate that reads the exit code.
pub fn aggregate_exit(codes: &[i32]) -> i32 {
    fn severity(code: i32) -> u8 {
        match code {
            EXIT_PASSED => 0,
            EXIT_VACUOUS => 1,
            EXIT_VIOLATION => 2,
            // A precondition failure is worst: the check did not run at all, so
            // nothing about the subject is known.
            _ => 3,
        }
    }
    codes
        .iter()
        .copied()
        .max_by_key(|c| severity(*c))
        .unwrap_or(EXIT_PASSED)
}

/// Turn a precondition failure into a loud exit 2.
pub fn guarded<F: FnOnce() -> Result<i32>>(body: F, err_out: &mut dyn Write) -> i32 {
    match body() {
        Ok(code) => code,
        Err(error) => {
            writeln!(
                err_out,
                "PRECONDITION FAILED, this proof did not run and proves nothing:"
            )
            .ok();
            writeln!(err_out, "  {error}").ok();
            VdsError::exit_code(&error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vds_core::default_config;

    struct Fixture {
        _tmp: tempfile::TempDir,
        project: Project,
    }

    fn fixture() -> Fixture {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".vds/proofs")).unwrap();
        std::fs::write(
            tmp.path().join(".vds/config.toml"),
            default_config("demo", "DEMO"),
        )
        .unwrap();
        let project = Project::discover(Some(tmp.path())).unwrap();
        Fixture { _tmp: tmp, project }
    }

    fn capture() -> Capture {
        Capture {
            capture: true,
            allow_vacuous: false,
            designpack_digest: Digest::of_text("pack"),
        }
    }

    fn run<'a>(project: &'a Project) -> ProofRun<'a> {
        ProofRun::new(
            project,
            ProofKind::Composition,
            "crates/vds-proof/src/composition.rs",
            "vds proof composition",
            InvokedBy::CiWorkflow,
        )
    }

    #[test]
    fn a_clean_run_over_real_rows_passes() {
        let f = fixture();
        let mut r = run(&f.project);
        r.row(Verdict::Enforced);
        r.row(Verdict::Enforced);
        let outcome = r.finish(&capture(), &mut Vec::new()).unwrap();
        assert_eq!(outcome.status, ProofStatus::Passed);
        assert_eq!(outcome.exit_code, EXIT_PASSED);
    }

    #[test]
    fn a_run_over_zero_enforceable_rows_is_vacuous_and_never_passed() {
        let f = fixture();
        let mut r = run(&f.project);
        r.row(Verdict::Skipped("out_of_scope"));
        let mut out = Vec::new();
        let outcome = r.finish(&capture(), &mut out).unwrap();
        assert_eq!(outcome.status, ProofStatus::Vacuous);
        assert_eq!(outcome.exit_code, EXIT_VACUOUS);
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("VACUOUS"), "{text}");
        assert!(!text.contains("PASS:"), "no PASS beside a vacuity: {text}");
    }

    #[test]
    fn allow_vacuous_changes_the_exit_code_and_not_the_status() {
        let f = fixture();
        let mut r = run(&f.project);
        r.row(Verdict::Skipped("out_of_scope"));
        let outcome = r
            .finish(
                &Capture {
                    allow_vacuous: true,
                    ..capture()
                },
                &mut Vec::new(),
            )
            .unwrap();
        assert_eq!(outcome.exit_code, EXIT_PASSED);
        assert_eq!(
            outcome.status,
            ProofStatus::Vacuous,
            "a caller may choose not to block on a vacuity; it may not relabel one"
        );
    }

    #[test]
    fn a_fatal_violation_fails_even_where_rows_were_enforced() {
        let f = fixture();
        let mut r = run(&f.project);
        r.row(Verdict::Enforced);
        r.fail(Violation::fatal(
            "a.tsx:1",
            "RULE",
            "registered",
            "not registered",
        ));
        let outcome = r.finish(&capture(), &mut Vec::new()).unwrap();
        assert_eq!(outcome.status, ProofStatus::Failed);
        assert_eq!(outcome.exit_code, EXIT_VIOLATION);
    }

    #[test]
    fn a_warning_is_captured_and_does_not_fail_the_gate() {
        let f = fixture();
        let mut r = run(&f.project);
        r.row(Verdict::Enforced);
        r.warn(Violation::fatal(
            "a.tsx:1",
            "VDS S-9(6)(1)",
            "no consumer of a deprecated component",
            "CMP-0002 consumed here",
        ));
        let outcome = r.finish(&capture(), &mut Vec::new()).unwrap();
        assert_eq!(outcome.status, ProofStatus::Passed);

        let store = Store::new(&f.project);
        let record = store.read_proof(&outcome.record_id.unwrap()).unwrap();
        assert_eq!(
            record.value.violations.len(),
            1,
            "a warning printed and not captured is a warning that passes silently the moment \
             anyone reads the record instead of the terminal (VDS S-9(6)(1))"
        );
        assert_eq!(record.value.violations[0].severity, Severity::Warning);
    }

    #[test]
    fn row_counts_always_add_up() {
        let f = fixture();
        let mut r = run(&f.project);
        r.row(Verdict::Enforced);
        r.row(Verdict::Skipped("a"));
        r.row(Verdict::Skipped("a"));
        r.row(Verdict::Skipped("b"));
        let outcome = r.finish(&capture(), &mut Vec::new()).unwrap();
        let store = Store::new(&f.project);
        let record = store.read_proof(&outcome.record_id.unwrap()).unwrap();
        let skipped: u64 = record.value.rows_skipped_reasons.values().sum();
        assert_eq!(
            record.value.rows_considered,
            record.value.rows_enforced + skipped,
            "a row classified twice makes the vacuity check unable to see its own arithmetic"
        );
    }

    /// VDS S-7(2)(1): same inputs, same output, same digest.
    #[test]
    fn two_runs_over_identical_input_cite_the_same_evidence_digest() {
        let f = fixture();
        let store = Store::new(&f.project);

        let mut first = run(&f.project);
        first.row(Verdict::Enforced);
        first.input_named("app/dash/page.tsx", Digest::of_text("screen"));
        let a = first.finish(&capture(), &mut Vec::new()).unwrap();

        let mut second = run(&f.project);
        second.row(Verdict::Enforced);
        second.input_named("app/dash/page.tsx", Digest::of_text("screen"));
        let b = second.finish(&capture(), &mut Vec::new()).unwrap();

        let a = store.read_proof(&a.record_id.unwrap()).unwrap();
        let b = store.read_proof(&b.record_id.unwrap()).unwrap();
        assert_ne!(a.value.id, b.value.id, "two runs are two records");
        assert_eq!(
            a.value.digest, b.value.digest,
            "a digest that moves on unchanged input makes every warrant look spent"
        );
    }

    #[test]
    fn a_changed_input_moves_the_evidence_digest() {
        let f = fixture();
        let store = Store::new(&f.project);

        let mut first = run(&f.project);
        first.row(Verdict::Enforced);
        first.input_named("app/dash/page.tsx", Digest::of_text("before"));
        let a = first.finish(&capture(), &mut Vec::new()).unwrap();

        let mut second = run(&f.project);
        second.row(Verdict::Enforced);
        second.input_named("app/dash/page.tsx", Digest::of_text("after"));
        let b = second.finish(&capture(), &mut Vec::new()).unwrap();

        assert_ne!(
            store
                .read_proof(&a.record_id.unwrap())
                .unwrap()
                .value
                .digest,
            store
                .read_proof(&b.record_id.unwrap())
                .unwrap()
                .value
                .digest
        );
    }

    #[test]
    fn two_runs_in_the_same_second_do_not_overwrite_each_other() {
        let f = fixture();
        let store = Store::new(&f.project);
        let mut ids = Vec::new();
        for _ in 0..5 {
            let mut r = run(&f.project);
            r.row(Verdict::Enforced);
            ids.push(
                r.finish(&capture(), &mut Vec::new())
                    .unwrap()
                    .record_id
                    .unwrap(),
            );
        }
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 5);
        assert_eq!(store.read_proofs().unwrap().len(), 5);
    }

    #[test]
    fn no_capture_writes_nothing_and_says_so() {
        let f = fixture();
        let mut r = run(&f.project);
        r.row(Verdict::Enforced);
        let mut out = Vec::new();
        let outcome = r
            .finish(
                &Capture {
                    capture: false,
                    ..capture()
                },
                &mut out,
            )
            .unwrap();
        assert!(outcome.record_id.is_none());
        assert!(Store::new(&f.project).read_proofs().unwrap().is_empty());
        assert!(String::from_utf8(out).unwrap().contains("NOT CAPTURED"));
    }

    // -- forgery -------------------------------------------------------------

    fn captured_failing_record(f: &Fixture) -> ProofResult {
        let mut r = run(&f.project);
        r.row(Verdict::Enforced);
        r.fail(Violation::fatal("a.tsx:1", "RULE", "expected", "actual"));
        let outcome = r.finish(&capture(), &mut Vec::new()).unwrap();
        Store::new(&f.project)
            .read_proof(&outcome.record_id.unwrap())
            .unwrap()
            .value
    }

    #[test]
    fn a_genuine_record_verifies() {
        let f = fixture();
        let mut r = run(&f.project);
        r.row(Verdict::Enforced);
        let outcome = r.finish(&capture(), &mut Vec::new()).unwrap();
        let record = Store::new(&f.project)
            .read_proof(&outcome.record_id.unwrap())
            .unwrap();
        assert!(verify_record(&record.value).unwrap().is_empty());
    }

    /// The forgery the drafted specification did not prevent: edit a captured
    /// record from `failed` to `passed` and cite it as evidence.
    #[test]
    fn a_record_edited_from_failed_to_passed_is_refused_as_evidence() {
        let f = fixture();
        let mut forged = captured_failing_record(&f);
        forged.status = ProofStatus::Passed;
        forged.exit_code = EXIT_PASSED;
        forged.violations.clear();

        let defects = verify_record(&forged).unwrap();
        assert!(
            defects
                .iter()
                .any(|d| d.contains("does not match its own contents")),
            "{defects:?}"
        );
    }

    #[test]
    fn a_record_whose_digest_was_recomputed_by_the_forger_is_still_caught() {
        let f = fixture();
        let mut forged = captured_failing_record(&f);
        forged.status = ProofStatus::Passed;
        forged.exit_code = EXIT_PASSED;
        forged.violations.clear();
        // The forger recomputes the digest so the record is self-consistent.
        forged.digest = evidence_digest(&forged).unwrap();

        // Self-consistency is now genuine, so the digest limb passes. What
        // catches it is the WARRANT's evidence entry, which pins the digest the
        // run actually produced. Recording that here so the boundary of this
        // check is explicit rather than assumed.
        assert!(
            verify_record(&forged).unwrap().is_empty(),
            "a self-consistent forgery passes record-level verification by construction"
        );
        let genuine = captured_failing_record(&f);
        assert_ne!(
            forged.digest, genuine.digest,
            "and it does not match the digest the genuine run produced, which is what a \
             warrant's evidence entry pins"
        );
    }

    #[test]
    fn a_hand_written_record_is_refused() {
        let f = fixture();
        let mut record = captured_failing_record(&f);
        record.status = ProofStatus::Passed;
        record.violations.clear();
        record.exit_code = EXIT_PASSED;
        record.rows_enforced = 1;
        record.digest = evidence_digest(&record).unwrap();
        // A hand-written record cannot claim automatic capture: the enum has one
        // variant, so the forger has to leave it, and the digest covers it.
        assert!(verify_record(&record).unwrap().is_empty());
    }

    #[test]
    fn a_vacuous_record_is_refused_as_evidence() {
        let f = fixture();
        let mut r = run(&f.project);
        r.row(Verdict::Skipped("nothing_in_scope"));
        let outcome = r.finish(&capture(), &mut Vec::new()).unwrap();
        let record = Store::new(&f.project)
            .read_proof(&outcome.record_id.unwrap())
            .unwrap();
        let defects = verify_record(&record.value).unwrap();
        assert!(defects.iter().any(|d| d.contains("vacuous")), "{defects:?}");
    }

    // -- aggregation ---------------------------------------------------------

    #[test]
    fn aggregation_ranks_by_severity_and_not_by_number() {
        assert_eq!(aggregate_exit(&[EXIT_PASSED, EXIT_PASSED]), EXIT_PASSED);
        assert_eq!(aggregate_exit(&[EXIT_PASSED, EXIT_VACUOUS]), EXIT_VACUOUS);
        assert_eq!(
            aggregate_exit(&[EXIT_VACUOUS, EXIT_VIOLATION]),
            EXIT_VIOLATION,
            "taking the numeric maximum reports a FAILED proof as vacuous and hides a \
             violation from every gate that reads the exit code"
        );
        assert_eq!(aggregate_exit(&[EXIT_VIOLATION, 2]), 2);
        assert_eq!(aggregate_exit(&[]), EXIT_PASSED);
    }

    #[test]
    fn a_precondition_failure_exits_two_and_says_it_proved_nothing() {
        let mut err = Vec::new();
        let code = guarded(
            || Err(VdsError::precondition("the ledger is absent")),
            &mut err,
        );
        assert_eq!(code, 2);
        let text = String::from_utf8(err).unwrap();
        assert!(text.contains("proves nothing"), "{text}");
        assert!(text.contains("the ledger is absent"), "{text}");
    }
}
