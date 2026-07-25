#!/usr/bin/env python3
"""Failing-direction tests for the warrant door: audit defects D2 and D3.

D2, the forged proof record. `_warrant_record` never consulted
IMPLEMENTED_PROOF_KINDS and nothing tied a record to an execution, so the auditor
hand-wrote a record declaring `kind: reconciliation`, `status: passed`,
`capture_mode: automatic`, `rows_enforced: 999`, naming
`tools/proofs/reconciliation.py`, a script that does not exist, and `warrant
record` printed "recorded WARRANT-W1-001" and exited 0. Fixing an enum to one
value makes forgery trivial rather than impossible: `capture_mode` is a string an
author types, so it asserted the property it was supposed to prove.

D3, the missing ordering. VDS S-6(2): "a stage may not be entered before the
preceding warrant is granted, and the ordering is the entire mechanism." No such
check existed, so W3_PRINCIPAL_ACCEPTED was recorded as granted with
`.vds/warrants/` empty.

Every test here drives the real CLI in a subprocess and asserts the exit code and
that NOTHING was written, because a refusal that still leaves a warrant on disk
is not a refusal.

One test in this file is expected to FAIL against the current tooling and is
kept red on purpose: `test_forged_record_is_not_listed_as_available_evidence`.
See its docstring.
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from vdsfixture import (  # noqa: E402
    EXIT_PASSED,
    TOOLS_DIR,
    VdsProjectCase,
    yamlish,
)

sys.path.insert(0, str(TOOLS_DIR))

import vds  # noqa: E402
from vdslib.core import find_project  # noqa: E402

SCREEN = """\
import { Button } from "@/components/ui/button";

export default function Page() {
  return (
    <div>
      <Button />
    </div>
  );
}
"""

# A hand-written proof record, exactly as the auditor wrote it: a passed,
# automatic-capture, 999-row result for a script VDS specifies and has never
# implemented.
FORGED_RECONCILIATION = """\
id: PROOF-19990101-000001
kind: reconciliation
status: passed
warrant_id: null
command: python3 tools/proofs/reconciliation.py
script: tools/proofs/reconciliation.py
exit_code: 0
rows_considered: 999
rows_enforced: 999
rows_skipped_reasons: {}
violations: []
inputs_digest: "sha256:0000000000000000000000000000000000000000000000000000000000000000"
digest: "sha256:1111111111111111111111111111111111111111111111111111111111111111"
designpack_digest: "sha256:0000000000000000000000000000000000000000000000000000000000000000"
captured_at: "1999-01-01T00:00:00Z"
capture_mode: automatic
invoked_by: ci_workflow
duration_ms: 1
"""


class WarrantDoorTest(VdsProjectCase):
    SCREENS = {"app/home/page.tsx": SCREEN}
    COMPONENTS = ("button",)

    def setUp(self) -> None:
        super().setUp()
        self.register("Button", "button")
        self.case_file = self.root / "casefile.md"
        self.case_file.write_text("the case file the bench decided on\n", encoding="utf-8")

    # -- helpers -------------------------------------------------------------

    def forge(self, name: str, body: str) -> str:
        (self.proofs_dir / f"{name}.yaml").write_text(body, encoding="utf-8")
        return name

    def record(self, stage: str, *extra: str, status: str = "granted"):
        args = [
            "warrant", "record",
            "--stage", stage,
            "--issue", "is the surface in the state this stage asserts",
            "--holding", "recorded for the fixture",
            "--runtime-summary", "fixture run",
            "--case-file", str(self.case_file),
            "--status", status,
        ]
        return self.vds(*args, *extra)

    def with_grantor(self, *extra: str) -> list[str]:
        return ["--grantor-citation", "[2026] VJS-CC-FX 1", "--bench", "one justice", *extra]

    # -- D2: a record that is not bound to an execution ----------------------

    def test_forged_record_for_an_unimplemented_kind_is_refused(self) -> None:
        """The auditor's attack verbatim, and the whole of it: exit 2, nothing written."""
        proof_id = self.forge("PROOF-19990101-000001", FORGED_RECONCILIATION)
        result = self.record("W1", *self.with_grantor("--evidence", proof_id), status="refused")
        self.assert_cli_refused(
            result,
            "declares kind 'reconciliation', which VDS specifies but does not implement",
            "Implemented kinds: register_completeness, composition, states",
            "A record for a script that does not exist is not evidence of anything",
        )
        self.assertEqual(self.warrant_files(), [])

    def test_record_naming_another_kinds_script_is_refused(self) -> None:
        """A forgery that borrows a REAL script's path and digest under a kind
        that script does not implement. The canonical-script rule is what stops
        the digest being transferable between kinds."""
        real = self.capture("register_completeness")
        record = yamlish.load(self.proofs_dir / f"{real}.yaml")
        record["script"] = "tools/proofs/composition.py"
        yamlish.dump(record, self.proofs_dir / f"{real}.yaml")
        result = self.record("W1", *self.with_grantor("--evidence", real), status="refused")
        self.assert_cli_refused(
            result,
            "names script 'tools/proofs/composition.py'",
            "may only come from 'tools/proofs/register_completeness.py'",
        )
        self.assertEqual(self.warrant_files(), [])

    def test_edited_result_fails_its_own_integrity_check(self) -> None:
        """Inflating rows_enforced on a genuine record. This is the cheap forgery,
        and it is caught before anything is re-run."""
        real = self.capture("register_completeness")
        path = self.proofs_dir / f"{real}.yaml"
        record = yamlish.load(path)
        record["rows_enforced"] = 999
        yamlish.dump(record, path)
        result = self.record("W1", *self.with_grantor("--evidence", real), status="refused")
        self.assert_cli_refused(result, "FAILS ITS OWN INTEGRITY CHECK")
        self.assertEqual(self.warrant_files(), [])

    def test_self_consistent_forgery_is_caught_by_re_execution(self) -> None:
        """The expensive forgery, and the one that decides whether D2 is fixed.

        Here the attacker has read the code: the record names the canonical
        script, carries that script's true digest, and its stated `digest` is
        recomputed so it passes its own integrity check. Every static limb
        accepts it. Only re-running the check and comparing the digest separates
        a record from an assertion, which is why `warrant record` re-executes.
        """
        real = self.capture("register_completeness")
        path = self.proofs_dir / f"{real}.yaml"
        record = yamlish.load(path)
        record["rows_enforced"] = 999
        record["digest"] = vds.proof_core_digest(record)
        yamlish.dump(record, path)

        # Prove the cheap limb really does accept it, so the test is about
        # re-execution and not about the integrity check firing early.
        self.assertEqual(vds.proof_core_digest(yamlish.load(path)), record["digest"])

        result = self.record("W1", *self.with_grantor("--evidence", real), status="refused")
        self.assert_cli_refused(
            result,
            "DOES NOT REPRODUCE",
            "re-running the command must reproduce the same digest",
        )
        self.assertEqual(self.warrant_files(), [])

    def test_stale_evidence_after_the_script_changes_is_refused(self) -> None:
        """A record captured against a script that has since been edited is stale
        evidence, not good evidence. The fixture edits its OWN copy of the script
        rather than the install, so the test cannot damage the tool."""
        real = self.capture("register_completeness")
        path = self.proofs_dir / f"{real}.yaml"
        record = yamlish.load(path)
        record["script_digest"] = "sha256:" + "0" * 64
        record["digest"] = vds.proof_core_digest(record)
        yamlish.dump(record, path)
        result = self.record("W1", *self.with_grantor("--evidence", real), status="refused")
        self.assert_cli_refused(result, "is STALE EVIDENCE")
        self.assertEqual(self.warrant_files(), [])

    def test_a_failed_proof_may_not_be_cited(self) -> None:
        """Only a passed proof is evidence (VDS S-7(2)(4))."""
        self.write_screen(
            "app/rogue/page.tsx",
            'import { Rogue } from "@/components/ui/rogue";\n'
            "export default function Page() { return <Rogue /> }\n",
        )
        self.regenerate()
        failing = self.proof("register_completeness")
        self.assertEqual(failing.returncode, 1, failing.stdout)
        proof_id = self.proof_records()[0].stem
        result = self.record("W1", *self.with_grantor("--evidence", proof_id), status="refused")
        self.assert_cli_refused(result, "A warrant may only cite a passed proof")
        self.assertEqual(self.warrant_files(), [])

    # -- D3: the ordering ----------------------------------------------------

    def test_w3_may_not_be_entered_with_no_granted_predecessor(self) -> None:
        """The auditor recorded W3_PRINCIPAL_ACCEPTED as granted with
        `.vds/warrants/` empty. The refusal must name the missing predecessor,
        not merely say no."""
        self.assertEqual(self.warrant_files(), [], "the fixture must start with no warrants")
        assent = self.root / "designpack" / "v1" / "provenance" / "assent"
        assent.mkdir(parents=True)
        event = assent / "anything.md"
        event.write_text("this file is NOT an assent event\n", encoding="utf-8")
        result = self.record("W3", "--acceptance-event", str(event))
        self.assert_cli_refused(
            result,
            "W3 may not be entered: its predecessor W1 (W1_REGISTER_COMPLETE) is not granted",
            "nothing on disk",
            "the ordering is the entire mechanism",
        )
        self.assertEqual(self.warrant_files(), [])

    def test_w2_may_not_be_entered_before_w1_is_granted(self) -> None:
        real = self.capture("register_completeness")
        refused = self.record("W1", *self.with_grantor("--evidence", real), status="refused")
        self.assertEqual(refused.returncode, EXIT_PASSED, refused.stderr)
        result = self.record("W2", *self.with_grantor())
        self.assert_cli_refused(
            result,
            "W2 may not be entered: its predecessor W1 (W1_REGISTER_COMPLETE) is not granted",
            "WARRANT-W1-001=refused",
        )
        self.assertEqual(len(self.warrant_files()), 1, "only the W1 refusal may exist")

    def test_w3_refuses_a_proof_in_place_of_an_acceptance(self) -> None:
        """STAGE_EVIDENCE['W3'] is the empty tuple, which used to mean any proof
        of any kind satisfied it. Acceptance is a human act and no machine result
        substitutes for it (VDS S-6(7))."""
        real = self.capture("register_completeness")
        result = self.record("W3", "--evidence", real, "--acceptance-event", str(self.case_file))
        self.assert_cli_refused(
            result,
            "W3 does not take --evidence",
            "how a machine result gets dressed as a human decision",
        )
        self.assertEqual(self.warrant_files(), [])

    def test_acceptance_event_must_be_an_acceptance_of_this_surface(self) -> None:
        """The acceptance gate, tested directly.

        It cannot be reached through the CLI: W3 is behind the ordering check,
        and W1 and W2 can never be GRANTED because their evidence includes
        `reconciliation` and `contrast`, which VDS specifies and does not
        implement. So the CLI path to `verify_acceptance_event` is dead code
        today, and testing it through the door would test the door's refusal
        instead. It is called directly, and the dead path is named in the
        suite's limitations rather than papered over.
        """
        project = find_project(self.root)
        assent = self.root / "designpack" / "v1" / "provenance" / "assent"
        assent.mkdir(parents=True)
        event = assent / "anything.md"
        event.write_text("this file is NOT an assent event\n", encoding="utf-8")
        live = vds.surface_digests(project)

        with self.assertRaises(Exception) as caught:
            vds.verify_acceptance_event(project, str(event), live)
        self.assertIn("is not an acceptance event", str(caught.exception))
        self.assertIn("it does not declare vds_acceptance_event: 1", str(caught.exception))

        # The same file, made into a real acceptance event over the live bytes.
        event.write_text(
            "vds_acceptance_event: 1\n"
            "project: fx\n"
            "stage: W3_PRINCIPAL_ACCEPTED\n"
            "accepted_by: Will Lilley\n"
            "accepted_at: \"2026-07-25T12:00:00Z\"\n"
            "surface:\n"
            f"  screens_digest: \"{live['screens_digest']}\"\n"
            f"  register_digest: \"{live['register_digest']}\"\n"
            "statement: I have seen the surface and I accept it as it stands today.\n",
            encoding="utf-8",
        )
        accepted = vds.verify_acceptance_event(project, str(event), live)
        self.assertEqual(accepted["accepted_by"], "Will Lilley")

        # Move one byte of the surface and the same event no longer accepts it.
        self.write_screen("app/home/page.tsx", SCREEN + "\n// touched\n")
        self.regenerate()
        moved = vds.surface_digests(project)
        with self.assertRaises(Exception) as caught:
            vds.verify_acceptance_event(project, str(event), moved)
        self.assertIn("the surface it accepts is not the surface on disk", str(caught.exception))

    # -- the legitimate path still works -------------------------------------

    def test_a_legitimate_warrant_on_verified_evidence_succeeds(self) -> None:
        """The direction that keeps the gate usable.

        A gate that refuses everything is indistinguishable from a broken gate,
        and the author's response to both is the same. This records a real
        warrant on a really captured, really re-executed proof, and asserts the
        cited digests were taken from the record on disk rather than from flags.
        """
        proof_id = self.capture("register_completeness")
        result = self.record(
            "W1", *self.with_grantor("--evidence", proof_id), status="refused"
        )
        self.assertEqual(result.returncode, EXIT_PASSED, result.stderr)
        self.assertIn("recorded WARRANT-W1-001", result.stdout)
        self.assertIn("to confirm", result.stdout)  # the re-execution happened
        self.assertIn("recording is not granting", result.stdout)

        files = self.warrant_files()
        self.assertEqual([p.name for p in files], ["WARRANT-W1-001.yaml"])
        warrant = yamlish.load(files[0])
        on_disk = yamlish.load(self.proofs_dir / f"{proof_id}.yaml")
        self.assertEqual(warrant["status"], "refused")
        self.assertEqual(len(warrant["evidence"]), 1)
        self.assertEqual(warrant["evidence"][0]["proof_id"], proof_id)
        self.assertEqual(warrant["evidence"][0]["kind"], "register_completeness")
        self.assertEqual(warrant["evidence"][0]["digest"], on_disk["digest"])
        self.assertEqual(warrant["evidence"][0]["script"], "tools/proofs/register_completeness.py")
        self.assertEqual(warrant["predecessors"], [])
        self.assertEqual(warrant["surface"], vds.surface_digests(find_project(self.root)))

    def test_no_stage_can_currently_be_granted_and_the_refusal_says_why(self) -> None:
        """A fact about VDS today, pinned so it cannot drift silently.

        W1's evidence set includes `reconciliation` and W2's includes `contrast`.
        Neither kind is implemented, so no warrant can lawfully be recorded as
        `granted` by anyone, and the tool says exactly that rather than letting
        an author route around it. When the missing scripts land, this test goes
        red and that is the correct signal.
        """
        proof_id = self.capture("register_completeness")
        result = self.record("W1", *self.with_grantor("--evidence", proof_id))
        self.assert_cli_refused(
            result,
            "W1 requires evidence of kind register_completeness, reconciliation",
            "missing: reconciliation",
            "reconciliation is specified and NOT IMPLEMENTED",
            "this stage cannot be granted by anyone until the script exists",
        )
        self.assertEqual(self.warrant_files(), [])

    # -- a hole the D2 fix does not close ------------------------------------

    def test_forged_record_is_not_listed_as_available_evidence(self) -> None:
        """KNOWN RED. `warrant status` still believes a forged proof record.

        `_warrant_record` now runs `verify_proof_record` over every cited proof,
        and that is what closed D2 at the recording door. The REPORT did not get
        the same treatment: vds.py:1288-1298 calls `latest_passed(proofs, kind)`
        for each required kind and prints whatever it finds, with no call to
        `verify_proof_record`. So dropping the auditor's hand-written
        `reconciliation` record into `.vds/proofs/` makes `warrant status` print

            evidence reconciliation: PROOF-19990101-000001 sha256:1111...

        in place of

            evidence reconciliation: no passed proof on disk, and this kind is
            NOT IMPLEMENTED

        A reader of that report is told the W1 evidence limb that CANNOT be
        satisfied is satisfied, by a file naming a script that does not exist.
        Nothing is granted by it, so this is a false statement of the record
        rather than a forged warrant, and it is the same defect class as D2 on
        the surface D2 did not reach. Left failing on purpose: a suite that
        asserts the buggy behaviour would make the bug permanent.
        """
        self.forge("PROOF-19990101-000001", FORGED_RECONCILIATION)
        result = self.vds("warrant", "status")
        self.assertIn("evidence reconciliation:", result.stdout)
        self.assertNotIn(
            "evidence reconciliation: PROOF-19990101-000001",
            result.stdout,
            "warrant status listed a hand-written record for an unimplemented kind as "
            "available evidence. It must run verify_proof_record before printing a "
            "proof id, exactly as warrant record does.",
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
