#!/usr/bin/env python3
"""Failing-direction tests for screens-ledger authenticity (VDS S-7(2)).

VDS S-7(2) holds that a proof is only a proof if a named test SEEDS a violation
and asserts the non-zero exit. Every test here does that: it builds a real
project, tampers with one thing, runs the real proof script through a real
subprocess, and asserts the exit code it observes.

The case under test is [2026] VJS-CC-OPBOX audit D1. An auditor hand-deleted a
five-line `- name: Rogue` entry from `.vds/ledgers/screens.yaml`, touched
nothing else, and turned `composition` from exit 1 with one violation into
exit 0 "PASS: 1 enforceable rows checked, 0 violations", captured as a
`capture_mode: automatic` proof record attesting to it. `test_naive_hand_edit`
is that exact attack.

Two assertions matter on every tampering test and both are made:

  the exit code is NOT 0        the tampering did not buy a pass
  no proof record was written   the tampering did not mint evidence either

Stdlib only, in keeping with the rest of the VDS tooling. Runnable directly
(`python3 tools/tests/test_ledger_authenticity.py`) and collectable by pytest
or `python3 -m unittest`.
"""

from __future__ import annotations

import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

TOOLS = Path(__file__).resolve().parents[1]
VDS_PY = TOOLS / "vds.py"
COMPOSITION = TOOLS / "proofs" / "composition.py"
REGISTER_COMPLETENESS = TOOLS / "proofs" / "register_completeness.py"

sys.path.insert(0, str(TOOLS))

from vdslib import scan, yamlish  # noqa: E402

EXIT_PASSED = 0
EXIT_VIOLATION = 1
EXIT_PRECONDITION = 2

SCREEN_WITH_ROGUE = """\
import { Button } from "@/components/ui/button";
import { Rogue } from "@/components/ui/rogue";

export default function Page() {
  return (
    <div>
      <Button />
      <Rogue />
    </div>
  );
}
"""


def run(*argv: str) -> subprocess.CompletedProcess:
    return subprocess.run(
        [sys.executable, *argv], capture_output=True, text=True, check=False
    )


class LedgerAuthenticityTest(unittest.TestCase):
    """Each test gets its own project, because each one mutates it."""

    def setUp(self) -> None:
        self.root = Path(tempfile.mkdtemp(prefix="vds-ledger-auth-"))
        self.addCleanup(shutil.rmtree, self.root, True)
        (self.root / "app" / "rogue").mkdir(parents=True)
        (self.root / "src" / "components" / "ui").mkdir(parents=True)
        (self.root / "app" / "rogue" / "page.tsx").write_text(SCREEN_WITH_ROGUE)
        for name in ("button", "rogue"):
            (self.root / "src" / "components" / "ui" / f"{name}.tsx").write_text(
                f"export function {name.capitalize()}(){{return null}}\n"
            )
        self.assertEqual(
            run(str(VDS_PY), "--root", str(self.root), "init",
                "--jurisdiction", "fx", "--repo-code", "FX").returncode,
            EXIT_PASSED,
        )
        self.regenerate()
        self.register("Button", "button")

    # -- helpers -------------------------------------------------------------

    @property
    def ledger_path(self) -> Path:
        return self.root / ".vds" / "ledgers" / "screens.yaml"

    @property
    def proofs_dir(self) -> Path:
        return self.root / ".vds" / "proofs"

    def regenerate(self) -> None:
        result = run(str(VDS_PY), "--root", str(self.root), "ledger", "screens")
        self.assertEqual(result.returncode, EXIT_PASSED, result.stderr)

    def register(self, export_name: str, module: str) -> None:
        result = run(
            str(VDS_PY), "--root", str(self.root), "register", "add",
            "--name", export_name, "--status", "registered",
            "--import-path", f"@/components/ui/{module}",
            "--source-file", f"src/components/ui/{module}.tsx",
            "--export-name", export_name, "--figma", "FXKEY#1:2",
            "--require", "default", "--drawn", "default",
        )
        self.assertEqual(result.returncode, EXIT_PASSED, result.stderr)

    def load_ledger(self) -> dict:
        return yamlish.load(self.ledger_path)

    def rewrite_ledger(self, ledger: dict, *, reseal: bool) -> None:
        """Write a ledger back. `reseal` models the attacker who read the code."""
        if reseal:
            ledger["derived_digest"] = scan.derived_digest_of(ledger["screens"])
        yamlish.dump(ledger, self.ledger_path)

    def proof_records(self) -> list[Path]:
        return sorted(self.proofs_dir.glob("*.yaml"))

    def assert_refused(self, result: subprocess.CompletedProcess, needle: str) -> None:
        """A tampered ledger must refuse loudly and mint nothing."""
        self.assertEqual(
            result.returncode,
            EXIT_PRECONDITION,
            f"expected exit {EXIT_PRECONDITION}, got {result.returncode}\n"
            f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}",
        )
        self.assertIn("PRECONDITION FAILED", result.stderr)
        self.assertIn(needle, result.stderr)
        self.assertIn("Regenerate with: vds.py ledger screens", result.stderr)
        self.assertEqual(
            self.proof_records(),
            [],
            "a refused run must capture no proof record: a record is evidence, and "
            "evidence of a run that did not happen is the defect, not the fix",
        )

    def run_composition(self, *extra: str) -> subprocess.CompletedProcess:
        return run(str(COMPOSITION), "--root", str(self.root), *extra)

    # -- the honest baseline -------------------------------------------------

    def test_honest_unregistered_component_fails(self) -> None:
        """The seeded violation. If this ever stops failing, everything below is void."""
        result = self.run_composition("--no-capture")
        self.assertEqual(result.returncode, EXIT_VIOLATION, result.stdout)
        self.assertIn("<Rogue>", result.stdout)
        self.assertIn("VIOLATIONS (1)", result.stdout)

    def test_clean_surface_passes(self) -> None:
        """A proof that always fails is as useless as one that never does."""
        self.register("Rogue", "rogue")
        self.regenerate()
        result = self.run_composition()
        self.assertEqual(result.returncode, EXIT_PASSED, result.stdout + result.stderr)
        self.assertIn("PASS: 2 enforceable rows checked, 0 violations.", result.stdout)
        self.assertEqual(len(self.proof_records()), 1)

    # -- the tampering tests -------------------------------------------------

    def test_naive_hand_edit(self) -> None:
        """[2026] VJS-CC-OPBOX audit D1, exactly as the auditor performed it."""
        before = self.run_composition("--no-capture")
        self.assertEqual(before.returncode, EXIT_VIOLATION)

        lines = self.ledger_path.read_text().splitlines(True)
        kept, i = [], 0
        while i < len(lines):
            if lines[i].strip() == "- name: Rogue":
                i += 5  # the five-line entry, and nothing else
                continue
            kept.append(lines[i])
            i += 1
        self.assertEqual(len(kept), len(lines) - 5, "the fixture's Rogue entry moved")
        self.ledger_path.write_text("".join(kept))

        self.assert_refused(self.run_composition(), "do not match its own recorded")

    def test_hand_edit_with_recomputed_digest(self) -> None:
        """The attacker who read scan.py and recomputed the in-file digest.

        This is why the guarantee is re-derivation and not the stored digest.
        """
        ledger = self.load_ledger()
        ledger["screens"][0]["references"] = [
            ref for ref in ledger["screens"][0]["references"] if ref["name"] != "Rogue"
        ]
        self.rewrite_ledger(ledger, reseal=True)

        result = self.run_composition()
        self.assert_refused(result, "not the rows the declared surface actually produces")
        self.assertIn("rows deleted by hand", result.stderr)
        self.assertIn("<Rogue>", result.stderr)

    def test_reference_rewritten_in_place(self) -> None:
        """Repointing a row at a registered component is an edit, not a fix."""
        ledger = self.load_ledger()
        for ref in ledger["screens"][0]["references"]:
            if ref["name"] == "Rogue":
                ref["importPath"] = "@/components/ui/button"
                ref["name"] = "Button"
        self.rewrite_ledger(ledger, reseal=True)
        self.assert_refused(
            self.run_composition(), "not the rows the declared surface actually produces"
        )

    def test_rows_reordered(self) -> None:
        """Row order is derived content, so a reordered ledger is an edited one."""
        ledger = self.load_ledger()
        ledger["screens"][0]["references"].reverse()
        self.rewrite_ledger(ledger, reseal=True)
        result = self.run_composition()
        self.assert_refused(result, "in a different ORDER")

    def test_invented_screen(self) -> None:
        """A route the surface does not produce cannot be added to the surface."""
        ledger = self.load_ledger()
        ledger["screens"].append(
            {"route": "app/ghost/page.tsx", "digest": "sha256:" + "0" * 64,
             "references": []}
        )
        self.rewrite_ledger(ledger, reseal=True)
        result = self.run_composition()
        self.assert_refused(result, "rows invented or altered by hand")
        self.assertIn("app/ghost/page.tsx", result.stderr)

    def test_derived_digest_removed(self) -> None:
        """Deleting the seal is not a way around the seal."""
        text = "".join(
            line for line in self.ledger_path.read_text().splitlines(True)
            if not line.startswith("derived_digest:")
        )
        self.ledger_path.write_text(text)
        self.assert_refused(self.run_composition(), "carries no usable `derived_digest`")

    def test_schema_version_1_refused(self) -> None:
        """A pre-authenticity ledger cannot be authenticated, so it is not read."""
        text = self.ledger_path.read_text().replace(
            "schema_version: 2", "schema_version: 1", 1
        )
        self.ledger_path.write_text(text)
        self.assert_refused(self.run_composition(), "predates this tool")

    def test_source_change_reports_stale_not_tampered(self) -> None:
        """The accurate diagnosis is the useful one: this is staleness, not forgery."""
        screen = self.root / "app" / "rogue" / "page.tsx"
        screen.write_text(screen.read_text() + "\n// touched\n")
        result = self.run_composition()
        self.assert_refused(result, "is STALE")
        self.assertIn("changed since generation: app/rogue/page.tsx", result.stderr)

    # -- the same wall on the other ledger-reading proof ----------------------

    def test_register_completeness_refuses_the_same_tampering(self) -> None:
        """Both ledger-reading proofs come through one door, so both refuse."""
        ledger = self.load_ledger()
        ledger["screens"][0]["references"] = [
            ref for ref in ledger["screens"][0]["references"] if ref["name"] != "Rogue"
        ]
        self.rewrite_ledger(ledger, reseal=True)
        self.assert_refused(
            run(str(REGISTER_COMPLETENESS), "--root", str(self.root)),
            "not the rows the declared surface actually produces",
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
