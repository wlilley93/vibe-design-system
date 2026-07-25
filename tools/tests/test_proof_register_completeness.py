#!/usr/bin/env python3
"""Failing-direction tests for the `register_completeness` proof (VDS S-7(2)(2)).

VDS S-7(5): "every component referenced by any declared screen exists in the
register". This is the EXISTENCE question and nothing more; whether the record
that exists is fit to be used belongs to `composition`.

The suite asserts three directions, because only all three together make a gate:

  FAILS   a screen importing a component with no register record exits 1, and the
          message names the route, the line, the component and why the lookup
          missed.
  PASSES  a clean surface exits 0 over a non-zero row count. A proof that always
          fails is switched off by the second author who meets it, and then
          nothing is checked at all.
  REFUSES the [2026] VJS-CC-OPBOX audit D1 ledger tampering exits 2 and captures
          no record, in both the naive and the resealed form.

Run directly, or through `tools/run-tests.sh`.
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from vdsfixture import EXIT_VACUOUS, VdsProjectCase  # noqa: E402

SCREEN = """\
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


class RegisterCompletenessTest(VdsProjectCase):
    SCREENS = {"app/home/page.tsx": SCREEN}
    COMPONENTS = ("button", "rogue")

    def setUp(self) -> None:
        super().setUp()
        self.register("Button", "button")

    # -- the failing direction ----------------------------------------------

    def test_unregistered_component_fails_and_names_the_row(self) -> None:
        """The seeded violation. Everything else in this file is void without it."""
        result = self.proof("register_completeness", "--no-capture")
        self.assert_violation(
            result,
            "app/home/page.tsx:8 <Rogue>",
            "a register record with code.importPath '@/components/ui/rogue'",
            "no register record names it at all",
            count=1,
        )

    def test_near_miss_is_named_rather_than_merely_reported_missing(self) -> None:
        """Why the lookup missed, in terms the author can act on.

        A record that exports the right name from the wrong path is the common
        real mistake, and "not found" would send the author to write a second
        record rather than fix the path on the first.
        """
        self.register("Rogue", "elsewhere")
        result = self.proof("register_completeness", "--no-capture")
        self.assert_violation(
            result,
            "<Rogue>",
            "exports 'Rogue' but from '@/components/ui/elsewhere'",
            count=1,
        )

    def test_a_failing_run_captures_a_record_that_is_not_evidence(self) -> None:
        """A failure is still a run, so it is still recorded, as `failed`.

        The record exists so the failure is citable; `warrant record` refuses to
        treat it as evidence, which is tested in test_cli_warrant.py.
        """
        result = self.proof("register_completeness")
        self.assert_violation(result, "<Rogue>")
        records = self.proof_records()
        self.assertEqual(len(records), 1)
        self.assertIn("status: failed", records[0].read_text(encoding="utf-8"))

    # -- the passing direction ----------------------------------------------

    def test_clean_surface_passes(self) -> None:
        self.register("Rogue", "rogue")
        result = self.proof("register_completeness")
        self.assert_clean_pass(result, rows=2)
        self.assertEqual(len(self.proof_records()), 1)

    def test_pass_over_zero_governed_rows_is_vacuous_not_passed(self) -> None:
        """[2026] VJS-CC-OPBOX 3 D3: a green gate that enforces nothing.

        With no governed import prefix every row is skipped, so the proof cannot
        fail, so it must not report a pass. Exit 3, and the record says vacuous.
        """
        config = self.root / ".vds" / "config.toml"
        config.write_text(
            config.read_text(encoding="utf-8").replace(
                'governed_import_prefixes = ["@/components/"]',
                "governed_import_prefixes = []",
            ),
            encoding="utf-8",
        )
        result = self.proof("register_completeness")
        self.assertEqual(result.returncode, EXIT_VACUOUS, result.stdout + result.stderr)
        self.assertIn("VACUOUS: this proof cannot currently fail", result.stdout)
        self.assertIn("is NOT evidence for any warrant", result.stdout)
        self.assertIn("status: vacuous", self.proof_records()[0].read_text(encoding="utf-8"))

    # -- the D1 tampering ----------------------------------------------------

    def test_d1_hand_deleted_ledger_row_is_refused(self) -> None:
        """The auditor's attack: delete the five-line entry, touch nothing else."""
        before = self.proof("register_completeness", "--no-capture")
        self.assert_violation(before, "<Rogue>")
        self.hand_delete_reference("Rogue")
        self.assert_proof_refused(
            self.proof("register_completeness"), "do not match its own recorded"
        )

    def test_d1_resealed_ledger_is_refused(self) -> None:
        """The attacker who recomputed the in-file digest is refused as well."""
        self.drop_reference_and_reseal("Rogue")
        result = self.proof("register_completeness")
        self.assert_proof_refused(result, "not the rows the declared surface actually produces")
        self.assertIn("rows deleted by hand", result.stderr)
        self.assertIn("<Rogue>", result.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=2)
