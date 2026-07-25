#!/usr/bin/env python3
"""Failing-direction tests for the enforcement lock: audit defect D5.

D5, the lock that believed its own declaration. The auditor wrote a one-line
script that checks nothing, pinned it with
`--invoked-by "ci_workflow=.github/workflows/nonexistent.yml:1=blocking"` in a
tree with no such workflow, and `lock verify` printed "invoked:
ci_workflow(blocking)", "no enforcement drift", exit 0. Deleting the named
failing-direction test afterwards also verified clean, because the test was
checked once at `lock add` (vds.py:1088) and never at verify.

Both halves are seeded here, plus the directions that keep the lock usable: a
legitimate entry pins and verifies, and the S-8(5) caveat is still printed rather
than quietly dropped, because a lock that claimed to be tamper-proof would be
worse than one that says what it cannot do.

Nothing here writes to the VDS install. The pinned gate, the invoker and the
failing-direction test are all project-local files inside the fixture, so the
tests that DELETE them cannot damage the tool. One test pins a real install
script by its `vds:` name, which is read-only and is the case a real adoption
hits: `<project>/tools/proofs` does not exist there.
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from vdsfixture import EXIT_PASSED, EXIT_VIOLATION, VdsProjectCase  # noqa: E402

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

WORKFLOW = """\
name: gates
on: [push]
jobs:
  design-gates:
    runs-on: ubuntu-latest
    steps:
      - run: python3 scripts/design-gate.py
"""

TEST_NAME = "fails when an unregistered component is seeded"
TEST_FILE = f"""\
it("{TEST_NAME}", () => {{
  seedUnregisteredComponent();
  expect(runGate().exitCode).not.toBe(0);
}});
"""


class LockTest(VdsProjectCase):
    SCREENS = {"app/home/page.tsx": SCREEN}
    COMPONENTS = ("button",)

    def setUp(self) -> None:
        super().setUp()
        self.register("Button", "button")
        (self.root / "scripts").mkdir()
        (self.root / "tests").mkdir()
        (self.root / ".github" / "workflows").mkdir(parents=True)
        self.gate = self.root / "scripts" / "design-gate.py"
        self.gate.write_text("print('a gate that checks something')\n", encoding="utf-8")
        self.test_file = self.root / "tests" / "gate.test.ts"
        self.test_file.write_text(TEST_FILE, encoding="utf-8")
        self.workflow = self.root / ".github" / "workflows" / "gates.yml"
        self.workflow.write_text(WORKFLOW, encoding="utf-8")

    # -- helpers -------------------------------------------------------------

    @property
    def lock_file(self) -> Path:
        return self.root / ".vds" / "enforcement.lock"

    def add(self, *overrides: str, invoker: str | None = None, proves: str = "composition",
            test_path: str | None = "tests/gate.test.ts", test_name: str | None = TEST_NAME,
            path: str = "scripts/design-gate.py"):
        args = ["lock", "add", path, "--kind", "proof_script", "--proves", proves]
        args += ["--invoked-by", invoker or "ci_workflow=.github/workflows/gates.yml#design-gates=blocking"]
        if test_path:
            args += ["--test-path", test_path]
        if test_name:
            args += ["--test-name", test_name]
        return self.vds(*args, *overrides)

    def verify(self):
        return self.vds("lock", "verify")

    # -- D5, first half: a declared invoker that does not exist --------------

    def test_invoker_that_does_not_exist_is_refused(self) -> None:
        """The auditor's exact attack. Nothing is written, so no false claim of
        enforcement reaches the disk in the first place."""
        result = self.add(invoker="ci_workflow=.github/workflows/nonexistent.yml:1=blocking")
        self.assert_cli_refused(
            result,
            "this entry would not survive `lock verify`, so it is not written",
            "NO INVOKER  scripts/design-gate.py",
            "but .github/workflows/nonexistent.yml does not exist",
            "an uninvoked gate is not enforcement",
        )
        self.assertFalse(self.lock_file.exists(), "a refused `lock add` wrote a lock file")

    def test_invoker_that_exists_but_never_names_the_script_is_refused(self) -> None:
        """Naming an invoker is not being invoked by it. This is the case the
        auditor's fixture could not distinguish, because it never opened the file."""
        self.workflow.write_text(
            "jobs:\n  design-gates:\n    steps:\n      - run: echo hello\n", encoding="utf-8"
        )
        result = self.add()
        self.assert_cli_refused(
            result,
            "INVOKER DOES NOT INVOKE  scripts/design-gate.py",
            "mentions scripts/design-gate.py",
            "Point `ref` at the file that actually runs this script",
        )
        self.assertFalse(self.lock_file.exists())

    def test_a_fragment_naming_a_job_that_is_absent_is_refused(self) -> None:
        result = self.add(
            invoker="ci_workflow=.github/workflows/gates.yml#no-such-job=blocking"
        )
        self.assert_cli_refused(
            result,
            "NO SUCH JOB  scripts/design-gate.py",
            "contains no 'no-such-job'",
        )
        self.assertFalse(self.lock_file.exists())

    def test_claiming_an_unimplemented_proof_kind_is_refused(self) -> None:
        """A lock entry claiming a kind no runner implements records a coverage
        nothing delivers, which is the D2 defect of [2026] VJS-CC-OPBOX 3."""
        result = self.add(proves="parity")
        self.assert_cli_refused(
            result,
            "UNIMPLEMENTED KIND  scripts/design-gate.py",
            "claims: parity",
            "this tooling implements only: register_completeness, composition, states",
        )
        self.assertFalse(self.lock_file.exists())

    def test_an_entry_cannot_be_written_without_naming_a_failing_direction_test(self) -> None:
        """VDS S-7(2)(2) made structural: the field is required, so the condition
        is enforced rather than requested."""
        result = self.add(test_path=None, test_name=None)
        self.assert_cli_refused(
            result,
            "pass --test-path and --test-name",
            "which is how VDS S-7(2)(2) is made structural rather than aspirational",
        )
        self.assertFalse(self.lock_file.exists())

    def test_naming_a_failing_direction_test_that_does_not_exist_is_refused(self) -> None:
        result = self.add(test_path="tests/imaginary.test.ts")
        self.assert_cli_refused(result, "--test-path 'tests/imaginary.test.ts' does not exist")
        self.assertFalse(self.lock_file.exists())

    # -- D5, second half: verify must re-check, not trust the pin ------------

    def test_a_legitimate_entry_pins_and_verifies_clean(self) -> None:
        """The direction that keeps the lock usable, asserted before every test
        below tears one thing out of it."""
        added = self.add()
        self.assertEqual(added.returncode, EXIT_PASSED, added.stderr)
        self.assertIn("invoker opened and confirmed: .github/workflows/gates.yml", added.stdout)
        self.assertIn("failing-direction test opened and confirmed", added.stdout)
        self.assertTrue(self.lock_file.exists())

        result = self.verify()
        self.assertEqual(result.returncode, EXIT_PASSED, result.stdout + result.stderr)
        self.assertIn("no enforcement drift", result.stdout)
        self.assertIn("every failing-direction test was OPENED", result.stdout)
        # VDS S-8(5). The caveat is part of the output, not a footnote to drop.
        self.assertIn("What this does NOT establish", result.stdout)
        self.assertIn("that any invoker ever RAN", result.stdout)

    def test_verify_fails_after_the_named_test_is_deleted(self) -> None:
        """The headline D5 leg. Before the fix this still printed "no enforcement
        drift" and exited 0, because the test was checked once at pin time."""
        self.assertEqual(self.add().returncode, EXIT_PASSED)
        self.assertEqual(self.verify().returncode, EXIT_PASSED)

        self.test_file.unlink()

        result = self.verify()
        self.assertEqual(result.returncode, EXIT_VIOLATION, result.stdout)
        self.assertIn("ENFORCEMENT DRIFT, 1 findings", result.stdout)
        self.assertIn("TEST GONE  scripts/design-gate.py", result.stdout)
        self.assertIn("that file does not exist", result.stdout)
        self.assertIn("has proven only its happy path", result.stdout)

    def test_verify_fails_after_the_named_test_is_renamed(self) -> None:
        """Deleting the file is the loud version. Renaming the assertion inside
        it is the quiet one, and it is the one that happens by accident."""
        self.assertEqual(self.add().returncode, EXIT_PASSED)
        self.test_file.write_text(
            'it("renders", () => { expect(true).toBe(true); });\n', encoding="utf-8"
        )
        result = self.verify()
        self.assertEqual(result.returncode, EXIT_VIOLATION, result.stdout)
        self.assertIn("TEST RENAMED OR REMOVED  scripts/design-gate.py", result.stdout)
        self.assertIn("The pinned failing-direction assertion is gone", result.stdout)

    def test_verify_fails_after_the_invoker_is_deleted(self) -> None:
        self.assertEqual(self.add().returncode, EXIT_PASSED)
        self.workflow.unlink()
        result = self.verify()
        self.assertEqual(result.returncode, EXIT_VIOLATION, result.stdout)
        self.assertIn("NO INVOKER  scripts/design-gate.py", result.stdout)

    def test_verify_fails_after_the_pinned_gate_is_weakened(self) -> None:
        """VDS S-8(1): a weakening edit bumps a digest and trips a blocking
        finding. It does not PREVENT the edit, and the output says so."""
        self.assertEqual(self.add().returncode, EXIT_PASSED)
        self.gate.write_text("print('i check nothing now')\n", encoding="utf-8")
        result = self.verify()
        self.assertEqual(result.returncode, EXIT_VIOLATION, result.stdout)
        self.assertIn("DRIFT    scripts/design-gate.py", result.stdout)
        self.assertIn("re-pin only after a recorded gate change", result.stdout)
        self.assertIn("the lock cannot bind an author with write access", result.stdout)

    def test_verify_fails_after_the_pinned_gate_is_deleted(self) -> None:
        self.assertEqual(self.add().returncode, EXIT_PASSED)
        self.gate.unlink()
        result = self.verify()
        self.assertEqual(result.returncode, EXIT_VIOLATION, result.stdout)
        self.assertIn("MISSING  scripts/design-gate.py", result.stdout)
        self.assertIn("a pinned gate that is gone is a deleted gate", result.stdout.lower())

    # -- the unpinned warning must be able to fire in a real adoption --------

    def test_unpinned_install_proof_scripts_are_reported(self) -> None:
        """An adopting repository has no `tools/proofs/` of its own, so globbing
        only the project made this warning unfireable everywhere except the repo
        that authored VDS. The install surface has to be reachable by name."""
        self.assertEqual(self.add().returncode, EXIT_PASSED)
        result = self.verify()
        for kind in ("register_completeness", "composition", "states"):
            self.assertIn(f"UNPINNED vds:tools/proofs/{kind}.py", result.stdout)

    def test_an_install_proof_script_can_be_pinned_by_its_vds_name(self) -> None:
        """And once pinned, it stops being reported unpinned. Read-only: the
        fixture pins the install file, it never writes to it."""
        self.workflow.write_text(
            "jobs:\n  design-gates:\n    steps:\n      - run: python3 tools/vds.py proof composition\n",
            encoding="utf-8",
        )
        added = self.add(path="vds:tools/proofs/composition.py")
        self.assertEqual(added.returncode, EXIT_PASSED, added.stderr)
        result = self.verify()
        self.assertEqual(result.returncode, EXIT_PASSED, result.stdout + result.stderr)
        self.assertNotIn("UNPINNED vds:tools/proofs/composition.py", result.stdout)
        self.assertIn("UNPINNED vds:tools/proofs/states.py", result.stdout)

    def test_no_lock_is_quiet_rather_than_broken(self) -> None:
        """VDS S-8(3): the lock is opt-in. A repository with none produces a note
        and exit 0, not a finding."""
        result = self.verify()
        self.assertEqual(result.returncode, EXIT_PASSED, result.stdout + result.stderr)
        self.assertIn("no enforcement.lock present", result.stdout)
        self.assertIn("The lock is opt-in", result.stdout)


if __name__ == "__main__":
    unittest.main(verbosity=2)
