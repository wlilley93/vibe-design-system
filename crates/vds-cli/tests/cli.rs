//! Integration tests over the real `vds` binary.
//!
//! Everything else in this workspace is a library test. Those are the right
//! shape for logic, and they cannot see the thing a user actually meets: the
//! argument parser, the exit code, the refusal banner, and whether a command's
//! own printed advice parses.
//!
//! That gap is not hypothetical here. The audit of the retired toolchain found
//! that `vds proof --all` (the command `init` printed as the next step) died in
//! argparse and ran nothing, and that `--root` parsed only before the subcommand
//! so the tool's own error message recommended an invocation that failed. Both
//! were invisible to every library test, and both were found by a human typing.
//! These tests are that human, written down.
//!
//! VDS S-11(5): two front doors, exactly one wall. This file tests the door.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const PASSED: i32 = 0;
const VIOLATION: i32 = 1;
const PRECONDITION: i32 = 2;
const VACUOUS: i32 = 3;

/// A throwaway project, torn down with the test.
struct Fixture {
    dir: tempfile::TempDir,
}

struct Run {
    argv: Vec<String>,
    code: i32,
    out: String,
    err: String,
}

impl Run {
    fn text(&self) -> String {
        format!("{}{}", self.out, self.err)
    }

    #[track_caller]
    fn expect(&self, code: i32) -> &Self {
        assert_eq!(self.code, code, "expected exit {code}{self}");
        self
    }

    #[track_caller]
    fn says(&self, needle: &str) -> &Self {
        assert!(
            self.text().contains(needle),
            "expected {needle:?} in the output{self}"
        );
        self
    }

    #[track_caller]
    fn does_not_say(&self, needle: &str) -> &Self {
        assert!(
            !self.text().contains(needle),
            "did not expect {needle:?} in the output{self}"
        );
        self
    }
}

impl std::fmt::Display for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\n  $ vds {}", self.argv.join(" "))?;
        writeln!(f, "  exit {}", self.code)?;
        for line in self.text().lines() {
            writeln!(f, "  | {line}")?;
        }
        Ok(())
    }
}

impl Fixture {
    fn new() -> Fixture {
        Fixture {
            dir: tempfile::tempdir().expect("a temporary directory"),
        }
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }

    fn write(&self, relative: &str, contents: &str) -> PathBuf {
        let path = self.root().join(relative);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
        std::fs::write(&path, contents).expect("write");
        path
    }

    fn vds(&self, argv: &[&str]) -> Run {
        let output: Output = Command::new(env!("CARGO_BIN_EXE_vds"))
            .args(argv)
            .current_dir(self.root())
            .env("VDS_ACTOR", "integration-test")
            .output()
            .expect("the vds binary runs");
        Run {
            argv: argv.iter().map(|a| (*a).to_owned()).collect(),
            code: output.status.code().unwrap_or(-1),
            out: String::from_utf8_lossy(&output.stdout).into_owned(),
            err: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    /// A project with one screen rendering one governed component.
    fn ready(&self) -> &Self {
        self.write(
            "app/dash/page.tsx",
            "import { Button } from \"@/components/ui\";\n\
             export default function P(){ return <Button />; }\n",
        );
        self.write(
            "src/components/ui/button.tsx",
            "export function Button() { return <button />; }\n",
        );
        self.vds(&["init"]).expect(PASSED);
        self.vds(&["ledger", "screens"]).expect(PASSED);
        self
    }

    fn register_button(&self) -> String {
        let run = self.vds(&[
            "register",
            "add",
            "--name",
            "Button",
            "--import-path",
            "@/components/ui",
            "--source-file",
            "src/components/ui/button.tsx",
            "--export-name",
            "Button",
        ]);
        run.expect(PASSED);
        let id = run
            .out
            .split_whitespace()
            .nth(1)
            .expect("an allocated id")
            .to_owned();
        self.vds(&["register", "set-status", &id, "designed"])
            .expect(PASSED);
        self.vds(&["register", "set-status", &id, "registered"])
            .expect(PASSED);
        id
    }
}

// -- the front door ---------------------------------------------------------

#[test]
fn the_binary_runs_and_describes_itself() {
    let f = Fixture::new();
    f.vds(&["--help"]).expect(PASSED).says("decides nothing");
    f.vds(&["--version"]).expect(PASSED);
}

/// The retired tool accepted `--root` only before the subcommand, which made the
/// advice IT PRINTED (`vds init --root <project>`) an argparse error.
#[test]
fn root_parses_on_either_side_of_the_subcommand() {
    let f = Fixture::new();
    let root = f.root().to_string_lossy().into_owned();
    f.vds(&["init", "--root", &root]).expect(PASSED);
    f.vds(&["--root", &root, "register", "list"]).expect(PASSED);
}

/// A `--root` that is not there was a seed for an upward walk, so a mistyped
/// root silently found an ANCESTOR project and wrote the record into it.
#[test]
fn a_root_that_does_not_exist_is_refused_rather_than_searched_upward_from() {
    let f = Fixture::new();
    f.vds(&["init"]).expect(PASSED);
    f.vds(&["--root", "/nonexistent/elsewhere", "register", "list"])
        .expect(PRECONDITION)
        .says("does not exist")
        .says("the wrong repository");
}

#[test]
fn a_command_outside_a_project_says_what_to_run() {
    let f = Fixture::new();
    f.vds(&["register", "list"])
        .expect(PRECONDITION)
        .says("vds init");
}

/// Every refusal claims the command did nothing. That has to be true.
#[test]
fn a_refused_init_leaves_no_record_behind() {
    let f = Fixture::new();
    f.vds(&["init", "--jurisdiction", "acme \"web\""])
        .expect(PRECONDITION)
        .says("one fixed anchor");
    assert!(
        !f.root().join(".vds").exists(),
        "a refusal wrote a .vds/ despite the banner saying it did nothing"
    );
}

// -- the advice the tool prints ---------------------------------------------

/// The audit's finding in one test: `init` printed a next step that crashed.
#[test]
fn every_command_init_recommends_actually_runs() {
    let f = Fixture::new();
    f.write(
        "app/dash/page.tsx",
        "export default function P(){ return <div />; }\n",
    );
    let init = f.vds(&["init"]);
    init.expect(PASSED);

    let recommended: Vec<Vec<String>> = init
        .out
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            trimmed.strip_prefix("vds ").map(|rest| {
                rest.split("  ")
                    .next()
                    .unwrap_or(rest)
                    .split_whitespace()
                    .map(|s| s.to_owned())
                    .collect()
            })
        })
        .collect();
    assert!(
        recommended.len() >= 3,
        "init should recommend a next step: {}",
        init.out
    );
    for argv in recommended {
        let borrowed: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
        let run = f.vds(&borrowed);
        assert!(
            run.code != PRECONDITION || !run.text().contains("unrecognized"),
            "init recommended a command that does not parse{run}"
        );
    }
}

// -- the lifecycle ----------------------------------------------------------

#[test]
fn add_refuses_to_mint_part_way_along_the_lifecycle() {
    let f = Fixture::new();
    f.ready();
    f.vds(&[
        "register", "add", "--name", "Button", "--status", "verified",
    ])
    .expect(PRECONDITION)
    .says("the entire mechanism");
}

#[test]
fn set_status_refuses_to_skip_a_step() {
    let f = Fixture::new();
    f.ready();
    let run = f.vds(&["register", "add", "--name", "Button"]);
    run.expect(PASSED);
    let id = run.out.split_whitespace().nth(1).expect("an id").to_owned();
    f.vds(&["register", "set-status", &id, "verified"])
        .expect(PRECONDITION)
        .says("skipping is forbidden");
    f.vds(&["register", "set-status", &id, "designed"])
        .expect(PASSED);
}

#[test]
fn retirement_cannot_be_reached_by_assigning_a_status() {
    let f = Fixture::new();
    f.ready();
    let id = f.register_button();
    f.vds(&["register", "set-status", &id, "retired"])
        .expect(PRECONDITION)
        .says("three phases");
}

#[test]
fn deprecating_toward_a_component_that_does_not_exist_is_refused() {
    let f = Fixture::new();
    f.ready();
    let id = f.register_button();
    f.vds(&["register", "deprecate", &id, "--superseded-by", "CMP-9999"])
        .expect(PRECONDITION)
        .says("no register record");
}

// -- argument parsing that used to crash ------------------------------------

#[test]
fn a_malformed_floor_is_a_precondition_failure_with_a_sentence() {
    let f = Fixture::new();
    f.ready();
    f.vds(&[
        "register",
        "add",
        "--name",
        "Card",
        "--floor",
        "text:bg:high:WCAG",
    ])
    .expect(PRECONDITION)
    .says("is not a ratio");
}

/// The retired tool's own `--figma` help produced a value its schema rejected,
/// with a message about oneOf branches.
#[test]
fn a_malformed_figma_node_is_refused_where_the_author_can_act_on_it() {
    let f = Fixture::new();
    f.ready();
    f.vds(&[
        "register",
        "add",
        "--name",
        "Card",
        "--figma",
        "KEY#node:id",
    ])
    .expect(PRECONDITION)
    .says("<digits>:<digits>")
    .does_not_say("oneOf");
}

// -- proofs -----------------------------------------------------------------

/// The registry is closed at ten and every one of them is now built, so `--list`
/// must name all ten and claim no gap it does not have.
#[test]
fn proof_list_names_the_whole_closed_registry() {
    let f = Fixture::new();
    f.ready();
    let run = f.vds(&["proof", "--list"]);
    run.expect(PASSED).says("CLOSED registry");
    for kind in [
        "register_completeness",
        "reconciliation",
        "composition",
        "contrast",
        "states",
        "parity",
        "token_pin",
        "retirement_drain",
        "ledger_staleness",
        "no_stored_values",
    ] {
        run.says(kind);
    }
    run.does_not_say("NOT IMPLEMENTED");
}

#[test]
fn proof_all_runs_every_implemented_kind() {
    let f = Fixture::new();
    f.ready();
    f.register_button();
    let run = f.vds(&["proof", "--all"]);
    run.says("summary:")
        .says("register_completeness")
        .says("no_stored_values")
        .does_not_say("unrecognized");
    assert_ne!(run.code, PRECONDITION, "proof --all must run{run}");
}

#[test]
fn an_unregistered_component_fails_the_anti_drift_proof() {
    let f = Fixture::new();
    f.ready();
    f.vds(&["proof", "register_completeness"])
        .expect(VIOLATION)
        .says("app/dash/page.tsx")
        .says("Button");
}

#[test]
fn nothing_in_scope_is_vacuous_and_never_passed() {
    let f = Fixture::new();
    f.write(
        "app/plain/page.tsx",
        "export default function P(){ return <div />; }\n",
    );
    f.vds(&["init"]).expect(PASSED);
    f.vds(&["ledger", "screens"]).expect(PASSED);
    f.vds(&["proof", "composition"])
        .expect(VACUOUS)
        .says("VACUOUS")
        .does_not_say("PASS:");
}

#[test]
fn allow_vacuous_changes_the_exit_code_and_not_the_verdict() {
    let f = Fixture::new();
    f.write(
        "app/plain/page.tsx",
        "export default function P(){ return <div />; }\n",
    );
    f.vds(&["init"]).expect(PASSED);
    f.vds(&["ledger", "screens"]).expect(PASSED);
    f.vds(&["proof", "composition", "--allow-vacuous"])
        .expect(PASSED)
        .says("VACUOUS");
}

/// Every kind in the registry actually dispatches.
///
/// This test used to assert the opposite about `contrast`: that asking for an
/// unimplemented kind was exit 2 and never a pass. The refusal is still in the
/// dispatcher and `vds-proof` holds a unit test that the two arms agree, but
/// there is no longer an unimplemented kind to reach it through the CLI. What is
/// worth guarding at this level is the other direction, and it is the direction
/// that would actually regress: a kind that is named in the registry, listed by
/// `--list`, and quietly refuses when anybody runs it.
#[test]
fn every_kind_in_the_registry_dispatches_rather_than_reporting_itself_unbuilt() {
    let f = Fixture::new();
    f.ready();
    for kind in [
        "register_completeness",
        "reconciliation",
        "composition",
        "contrast",
        "states",
        "parity",
        "token_pin",
        "retirement_drain",
        "ledger_staleness",
        "no_stored_values",
    ] {
        let run = f.vds(&["proof", kind, "--allow-vacuous"]);
        run.does_not_say("NOT implemented");
        assert_ne!(
            run.code, PRECONDITION,
            "`vds proof {kind}` reports a precondition failure on a ready project, so a kind \
             the registry names cannot be run{run}"
        );
    }
}

#[test]
fn a_stale_ledger_stops_a_proof_rather_than_letting_it_pass() {
    let f = Fixture::new();
    f.ready();
    f.register_button();
    f.write(
        "app/dash/page.tsx",
        "import { Button, Card } from \"@/components/ui\";\n\
         export default function P(){ return <div><Button /><Card /></div>; }\n",
    );
    f.vds(&["proof", "composition"])
        .expect(PRECONDITION)
        .says("STALE");
}

/// A file the scanner cannot read completely is a refusal, not a smaller ledger.
#[test]
fn a_screen_that_cannot_be_scanned_is_refused() {
    let f = Fixture::new();
    f.write(
        "app/dash/page.tsx",
        "const broken = `oops;\nexport default function P(){ return <div />; }\n",
    );
    f.vds(&["init"]).expect(PASSED);
    f.vds(&["ledger", "screens"])
        .expect(PRECONDITION)
        .says("not counted anywhere");
}

// -- warrants ---------------------------------------------------------------

#[test]
fn a_stage_cannot_be_recorded_before_its_predecessor_is_granted() {
    let f = Fixture::new();
    f.ready();
    f.vds(&[
        "warrant",
        "record",
        "--stage",
        "W3",
        "--issue",
        "i",
        "--holding",
        "h",
        "--runtime-summary",
        "s",
        "--acceptance-event",
        ".vds/config.toml",
        "--case-file-digest",
        "sha256:0000000000000000000000000000000000000000000000000000000000000000",
    ])
    .expect(PRECONDITION)
    .says("the entire mechanism");
}

#[test]
fn warrant_status_reports_the_chain_and_says_vds_grants_nothing() {
    let f = Fixture::new();
    f.ready();
    f.vds(&["warrant", "status"])
        .expect(VIOLATION)
        .says("VDS grants nothing")
        .says("NOT GRANTED");
}

// -- the enforcement surface ------------------------------------------------

#[test]
fn a_lock_entry_cannot_be_written_without_a_failing_direction_test() {
    let f = Fixture::new();
    f.ready();
    f.write("gate.rs", "fn gate() {}\n");
    f.vds(&[
        "lock",
        "add",
        "gate.rs",
        "--proves",
        "composition",
        "--invoked-by",
        "ci_workflow=w.yml",
    ])
    .expect(PRECONDITION)
    .says("VDS S-7(2)(2)");
}

#[test]
fn a_lock_entry_cannot_be_written_without_an_invocation() {
    let f = Fixture::new();
    f.ready();
    f.write("gate.rs", "fn gate() {}\n");
    f.write("gate_test.rs", "fn seeds() {}\n");
    f.vds(&[
        "lock",
        "add",
        "gate.rs",
        "--proves",
        "composition",
        "--test-path",
        "gate_test.rs",
        "--test-name",
        "seeds",
    ])
    .expect(PRECONDITION)
    .says("uninvoked gate");
}

/// VDS S-8(6): the positive direction of the drift check is itself tested.
#[test]
fn editing_a_pinned_gate_trips_a_drift_finding_through_the_cli() {
    let f = Fixture::new();
    f.ready();
    f.write("gate.rs", "fn gate() {}\n");
    f.write("gate_test.rs", "fn seeds() {}\n");
    f.vds(&[
        "lock",
        "add",
        "gate.rs",
        "--proves",
        "composition",
        "--invoked-by",
        "ci_workflow=w.yml=blocking",
        "--test-path",
        "gate_test.rs",
        "--test-name",
        "seeds",
    ])
    .expect(PASSED);
    f.vds(&["lock", "verify"]).expect(PASSED);

    f.write("gate.rs", "fn gate() { /* weakened */ }\n");
    f.vds(&["lock", "verify"])
        .expect(VIOLATION)
        .says("DRIFT")
        .says("VDS S-8(5)");
}

#[test]
fn repinning_without_a_rationale_is_refused() {
    let f = Fixture::new();
    f.ready();
    f.write("gate.rs", "fn gate() {}\n");
    f.write("gate_test.rs", "fn seeds() {}\n");
    f.vds(&[
        "lock",
        "add",
        "gate.rs",
        "--proves",
        "composition",
        "--invoked-by",
        "ci_workflow=w.yml=blocking",
        "--test-path",
        "gate_test.rs",
        "--test-name",
        "seeds",
    ])
    .expect(PASSED);
    f.write("gate.rs", "fn gate() { /* changed */ }\n");
    f.vds(&["lock", "repin"])
        .expect(PRECONDITION)
        .says("VDS S-8(4)");
}

// -- the design round trip --------------------------------------------------

#[test]
fn the_brief_lists_usable_components_and_refuses_the_unusable_ones() {
    let f = Fixture::new();
    f.ready();
    f.register_button();
    f.vds(&["register", "add", "--name", "Sketch"])
        .expect(PASSED);
    f.vds(&["brief"])
        .expect(PASSED)
        .says("Components you may use")
        .says("Button")
        .says("may NOT use")
        .says("Sketch");
}

#[test]
fn the_brief_carries_no_design_value() {
    let f = Fixture::new();
    f.ready();
    f.register_button();
    let run = f.vds(&["brief"]);
    run.expect(PASSED);
    for realisation in ["#ebebeb", "cubic-bezier", "oklch(", "rgb("] {
        run.does_not_say(realisation);
    }
}

/// The contract stays honest about the requirements no proof reaches.
///
/// It used to demonstrate that with the three unimplemented kinds, and all three
/// are now built, so the honest example moved: a KEYBOARD contract is checked by
/// nothing in this build, and `vds impl` says so per requirement rather than
/// listing it as covered.
#[test]
fn an_implementation_contract_names_what_nothing_checks() {
    let f = Fixture::new();
    f.ready();
    f.register_button();
    let run = f.vds(&[
        "register",
        "add",
        "--name",
        "Toggle",
        "--import-path",
        "@/components/ui",
        "--source-file",
        "src/components/ui/button.tsx",
        "--export-name",
        "Button",
        "--keyboard",
        "Enter=activate",
    ]);
    run.expect(PASSED);
    let id = run
        .out
        .split_whitespace()
        .nth(1)
        .expect("an allocated id")
        .to_owned();

    f.vds(&["impl", &id])
        .expect(PASSED)
        .says("What no check will catch")
        .says("respond to `Enter`")
        .says("checked by: **nothing in this build**");
}

#[test]
fn a_figma_ledger_derives_from_a_saved_response_with_no_token() {
    let f = Fixture::new();
    f.ready();
    let id = f.register_button();
    f.vds(&[
        "register",
        "amend",
        &id,
        "--kind",
        "non_breaking",
        "--what",
        "record the decided node",
        "--figma",
        "KEY#12:34",
    ])
    .expect(PASSED);
    f.write(
        "response.json",
        r#"{"name":"Target","version":"1","document":{"id":"0:0","type":"DOCUMENT","children":[
           {"id":"12:34","type":"COMPONENT_SET","name":"Button","children":[
             {"id":"12:35","type":"COMPONENT","name":"State=Default"}]}]}}"#,
    );
    f.vds(&["figma", "pull", "--from", "response.json"])
        .expect(PASSED)
        .says("nodes resolved: 1 of 1");
    f.vds(&["figma", "status"])
        .expect(PASSED)
        .says("draws: default");
}

// -- the honest position ----------------------------------------------------

#[test]
fn doctor_reports_every_criterion_and_flatters_nothing() {
    let f = Fixture::new();
    f.ready();
    let run = f.vds(&["doctor", "--report-only"]);
    run.expect(PASSED);
    for criterion in ["D1", "D2", "D3", "D4", "D5", "D6", "D7", "D8", "D9", "D10"] {
        run.says(criterion);
    }
    run.says("UNMET").says("settled by:");
}

#[test]
fn doctor_exits_non_zero_where_a_criterion_is_unmet() {
    let f = Fixture::new();
    f.ready();
    f.vds(&["doctor"]).expect(VIOLATION);
}

#[test]
fn schema_check_passes_against_the_schemas_the_binary_emits() {
    let f = Fixture::new();
    f.ready();
    f.vds(&["schema", "emit"]).expect(PASSED);
    f.vds(&["schema", "check"]).expect(PASSED).says("match");
}

#[test]
fn a_hand_edited_schema_is_caught_as_drift() {
    let f = Fixture::new();
    f.ready();
    f.vds(&["schema", "emit"]).expect(PASSED);
    f.write("schema/pin.schema.json", "{\"edited\": true}\n");
    f.vds(&["schema", "check"])
        .expect(VIOLATION)
        .says("DRIFTED");
}

// -- adoption ---------------------------------------------------------------

#[test]
fn import_scaffolds_candidates_at_proposed_and_never_overwrites() {
    let f = Fixture::new();
    f.ready();
    f.vds(&["register", "import"])
        .expect(PASSED)
        .says("CANDIDATES, not contracts")
        .says("Button");

    let before = std::fs::read_dir(f.root().join(".vds/register"))
        .expect("register")
        .count();
    assert_eq!(before, 1, "one candidate for one exported component");

    // A second run must not duplicate it.
    f.vds(&["register", "import"])
        .expect(PASSED)
        .says("already registered: 1");
    let after = std::fs::read_dir(f.root().join(".vds/register"))
        .expect("register")
        .count();
    assert_eq!(after, before, "import must never overwrite or duplicate");
}

#[test]
fn an_imported_candidate_is_proposed_and_therefore_still_fails_composition() {
    let f = Fixture::new();
    f.ready();
    f.vds(&["register", "import"]).expect(PASSED);
    f.vds(&["proof", "composition"])
        .expect(VIOLATION)
        .says("this is drift");
}

// -- vds prune ---------------------------------------------------------------

/// The default must be inert. A delete whose default is to delete is a delete
/// somebody runs to see what it does.
#[test]
fn prune_removes_nothing_without_apply() {
    let f = Fixture::new();
    f.ready().register_button();
    for _ in 0..3 {
        f.vds(&["proof", "composition"]).expect(PASSED);
    }
    let before = proof_count(&f);

    f.vds(&["prune", "--keep", "1"])
        .expect(PASSED)
        .says("Nothing was removed")
        .says("--apply");
    assert_eq!(proof_count(&f), before, "a report deleted files");
}

#[test]
fn prune_keeps_the_most_recent_of_each_kind_and_removes_the_rest() {
    let f = Fixture::new();
    f.ready().register_button();
    for _ in 0..4 {
        f.vds(&["proof", "composition"]).expect(PASSED);
        f.vds(&["proof", "no_stored_values"]).expect(PASSED);
    }
    assert_eq!(proof_count(&f), 8);

    f.vds(&["prune", "--keep", "1", "--apply"]).expect(PASSED);
    assert_eq!(
        proof_count(&f),
        2,
        "one of each kind survives, not one overall: a kind that runs rarely must not be \
         evicted by a kind that runs on every commit"
    );
}

/// The rule that matters most. A warrant naming a record that is not there is a
/// signature on nothing (VDS S-6(3)), so no window and no --keep may reach it.
#[test]
fn prune_never_removes_a_record_a_warrant_cites() {
    let f = Fixture::new();
    f.ready().register_button();

    // The oldest record of its kind, and the one a --keep of 1 would evict.
    f.vds(&["proof", "register_completeness"]).expect(PASSED);
    let cited = newest_proof_id(&f, "register_completeness");
    for _ in 0..3 {
        f.vds(&["proof", "register_completeness"]).expect(PASSED);
    }
    assert_ne!(cited, newest_proof_id(&f, "register_completeness"));

    f.vds(&["proof", "reconciliation"]).expect(PASSED);
    let second = newest_proof_id(&f, "reconciliation");

    f.vds(&[
        "warrant",
        "record",
        "--stage",
        "W1",
        "--status",
        "granted",
        "--issue",
        "the register is complete",
        "--holding",
        "granted over the declared surface",
        "--runtime-summary",
        "one screen, one registered component",
        "--grantor-citation",
        "[2026] VJS-CC-VDS 1",
        "--bench",
        "first-instance-judge",
        "--assent-source",
        "sovereign_assent",
        "--case-file-digest",
        "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "--evidence",
        &cited,
        "--evidence",
        &second,
    ])
    .expect(PASSED);

    f.vds(&["prune", "--keep", "1", "--apply"]).expect(PASSED);
    assert!(
        f.root().join(format!(".vds/proofs/{cited}.yaml")).is_file(),
        "prune removed a record a granted warrant cites, which makes that warrant a signature \
         on nothing"
    );
}

/// The retention log has to name what went, or the command is an untraceable
/// delete however good its reasoning is.
#[test]
fn prune_writes_a_log_naming_every_record_it_removed() {
    let f = Fixture::new();
    f.ready().register_button();
    for _ in 0..3 {
        f.vds(&["proof", "composition"]).expect(PASSED);
    }
    let oldest = oldest_proof_id(&f, "composition");

    f.vds(&["prune", "--keep", "1", "--apply"]).expect(PASSED);

    let log = std::fs::read_to_string(f.root().join(".vds/logs/retention/RETENTION-0001.yaml"))
        .expect("a retention log");
    assert!(
        log.contains(&oldest),
        "the log does not name {oldest}:\n{log}"
    );
    assert!(
        log.contains("VDS S-2(5)"),
        "the log records no basis:\n{log}"
    );
    assert!(log.contains("sha256:"), "the log records no digest:\n{log}");
}

#[test]
fn prune_refuses_to_keep_nothing() {
    let f = Fixture::new();
    f.ready();
    f.vds(&["prune", "--keep", "0", "--apply"])
        .expect(PRECONDITION)
        .says("Keep at least one");
}

fn proof_count(f: &Fixture) -> usize {
    std::fs::read_dir(f.root().join(".vds/proofs"))
        .map(|d| d.filter_map(|e| e.ok()).count())
        .unwrap_or(0)
}

fn proof_ids(f: &Fixture, kind: &str) -> Vec<String> {
    let mut ids: Vec<String> = std::fs::read_dir(f.root().join(".vds/proofs"))
        .expect("a proofs directory")
        .filter_map(|e| e.ok())
        .filter(|e| {
            std::fs::read_to_string(e.path())
                .map(|t| t.contains(&format!("kind: {kind}")))
                .unwrap_or(false)
        })
        .map(|e| e.file_name().to_string_lossy().replace(".yaml", ""))
        .collect();
    ids.sort();
    ids
}

fn newest_proof_id(f: &Fixture, kind: &str) -> String {
    proof_ids(f, kind).pop().expect("a record of that kind")
}

fn oldest_proof_id(f: &Fixture, kind: &str) -> String {
    proof_ids(f, kind).remove(0)
}

// -- doctor, for a project that is not the jurisdiction ----------------------

/// A subject project has no standing to answer another jurisdiction's reserved
/// clauses, and D10 used to report it as failing for not duplicating them.
#[test]
fn doctor_does_not_ask_a_subject_project_to_refile_the_specifications_submissions() {
    let f = Fixture::new();
    f.ready();
    let run = f.vds(&["doctor", "--report-only"]);
    run.expect(PASSED)
        .does_not_say("SUBMISSION-VDS-001 MISSING")
        .says("vendors no designpack");
}

/// And once a pack IS vendored, the criterion is settled by the pin rather than
/// by a directory this project does not own.
#[test]
fn doctor_settles_the_reserved_clauses_against_a_vendored_designpack() {
    let f = Fixture::new();
    f.ready();
    let config = f.root().join(".vds/config.toml");
    let text = std::fs::read_to_string(&config).expect("a config");
    std::fs::write(
        &config,
        text.replace("designpack = \"none@0\"", "designpack = \"vds@1\""),
    )
    .expect("write");

    f.vds(&["doctor", "--report-only"])
        .expect(PASSED)
        .says("vendors vds@1")
        .says("answered upstream");
}
