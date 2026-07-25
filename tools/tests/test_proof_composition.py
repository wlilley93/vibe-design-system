#!/usr/bin/env python3
"""Failing-direction tests for the `composition` proof (VDS S-7(2)(2)).

Where `register_completeness` asks whether a record EXISTS, `composition` asks
whether the thing being used is in a state fit to be used. It has three fatal
rules and one warning, and each is seeded here against its own fixture:

  R1  a governed reference with no register record at all
  R2  a governed reference whose record is not in an enforceable status
      (`proposed` and `designed` are records of an intention, not registrations)
  R3  a reference to a RETIRED component. VDS S-9(8) inverts the test after
      retirement: the code being there is the defect.
  W1  a reference to a DEPRECATED component is reported per site, by route, and
      does NOT fail the gate (VDS S-9(6)(1)).

Plus the clean direction, and the [2026] VJS-CC-OPBOX audit D1 ledger tampering
in both its naive and its resealed form.

Two of these states cannot be reached through the CLI, deliberately: `retire`
demands a `retirement_drain` proof, and that kind is specified and NOT
implemented, so retirement is fail-closed. The fixture therefore edits the
register record directly. The register is authored, not generated, so a hand-edit
of it is a lawful act and not the tampering D1 is about.
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from vdsfixture import VdsProjectCase  # noqa: E402

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

RETIRED_TOMBSTONE = 'retiredAt: "2026-01-01T00:00:00Z"\nretirementProofId: PROOF-19990101-000009\n'


class CompositionTest(VdsProjectCase):
    SCREENS = {"app/home/page.tsx": SCREEN}
    COMPONENTS = ("button", "rogue")

    def setUp(self) -> None:
        super().setUp()
        self.register("Button", "button")  # CMP-0001, registered

    # -- R1: unregistered ----------------------------------------------------

    def test_r1_unregistered_component_fails_and_names_the_row(self) -> None:
        result = self.proof("composition", "--no-capture")
        self.assert_violation(
            result,
            "app/home/page.tsx:8 <Rogue>",
            "composition R1: no screen uses an unregistered component",
            "unregistered: no such record",
            count=1,
        )

    # -- R2: registered too early -------------------------------------------

    def test_r2_proposed_record_is_drift_not_a_registration(self) -> None:
        """A design that is merely drawn is exactly what the proof exists to catch
        being used, so the message has to say `proposed`, not "not found"."""
        self.register("Rogue", "rogue", status="proposed", drawn="")
        result = self.proof("composition", "--no-capture")
        self.assert_violation(
            result,
            "<Rogue>",
            "status is not a registered state",
            "CMP-0002 status 'proposed': the record exists but the component is not registered",
            count=1,
        )

    # -- R3: retired ---------------------------------------------------------

    def test_r3_retired_component_still_consumed_fails(self) -> None:
        self.register("Rogue", "rogue")
        self.edit_record("CMP-0002", "status: registered", "status: retired")
        self.append_to_record("CMP-0002", RETIRED_TOMBSTONE)
        result = self.proof("composition", "--no-capture")
        self.assert_violation(
            result,
            "<Rogue>",
            "after retirement the code being there is the defect",
            "CMP-0002 is retired, so no screen may reference it",
            "still consumed here",
            count=1,
        )

    # -- W1: deprecated is a warning, and never silent -----------------------

    def test_w1_deprecated_component_warns_per_site_and_does_not_fail(self) -> None:
        """VDS S-9(6)(1). Getting this wrong in either direction is a defect: a
        failure would make deprecation unusable, and silence would make it
        pointless."""
        self.register("Rogue", "rogue")
        result = self.vds("register", "deprecate", "CMP-0002", "--superseded-by", "CMP-0001")
        self.assertEqual(result.returncode, 0, result.stderr)
        run = self.proof("composition")
        self.assert_clean_pass(run, rows=2)
        self.assertIn("consuming sites of DEPRECATED components", run.stdout)
        self.assertIn("deprecated-consumer: app/home/page.tsx:8 <Rogue>", run.stdout)
        self.assertIn("superseded by CMP-0001", run.stdout)

    # -- the passing direction ----------------------------------------------

    def test_clean_surface_passes(self) -> None:
        self.register("Rogue", "rogue")
        result = self.proof("composition")
        self.assert_clean_pass(result, rows=2)
        self.assertEqual(len(self.proof_records()), 1)

    def test_bare_elements_are_counted_and_not_enforced(self) -> None:
        """VDS S-9(10) RESERVED. The carve-out has to be visible in the output,
        because a silent carve-out is how rows_enforced quietly becomes zero."""
        self.register("Rogue", "rogue")
        result = self.proof("composition", "--no-capture")
        self.assertIn("not enforced, bare_element_informational_vds_s9_10: 1", result.stdout)
        self.assertIn("rows_considered: 3", result.stdout)
        self.assertIn("rows_enforced:   2", result.stdout)

    # -- the D1 tampering ----------------------------------------------------

    def test_d1_hand_deleted_ledger_row_is_refused(self) -> None:
        """The exact audit reproduction: exit 1 with a violation, hand-delete the
        five-line entry, and the identical command used to return exit 0 PASS."""
        before = self.proof("composition", "--no-capture")
        self.assert_violation(before, "<Rogue>")
        self.hand_delete_reference("Rogue")
        self.assert_proof_refused(self.proof("composition"), "do not match its own recorded")

    def test_d1_resealed_ledger_is_refused_and_names_the_deleted_row(self) -> None:
        self.drop_reference_and_reseal("Rogue")
        result = self.proof("composition")
        self.assert_proof_refused(result, "not the rows the declared surface actually produces")
        self.assertIn("rows deleted by hand", result.stderr)
        self.assertIn("app/home/page.tsx:8 <Rogue>", result.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=2)
