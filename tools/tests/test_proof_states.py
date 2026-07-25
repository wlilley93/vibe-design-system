#!/usr/bin/env python3
"""Failing-direction tests for the `states` proof (VDS S-7(2)(2)).

VDS S-7(5): "every required state of every registered component is drawn". The
nine states are fixed by VDS S-5(3) and a record may require a subset and may not
invent a tenth.

Seeded here:

  MISSING   a record requiring a state it has not drawn exits 1, and the message
            names the record, the file, and which states are missing rather than
            how many.
  TENTH     a record whose states list contains something outside the nine exits
            1. The schema already refuses this, and the proof re-checks it,
            because a proof that trusts its input has proven the input.
  LIFECYCLE a `proposed` record is skipped rather than failed. Enforcing it would
            fail every new registration and teach the author to skip the stage,
            which converts a gate into a reason to route around the gate.
  CLEAN     a complete record exits 0 over a non-zero row count.

One negative result is recorded here rather than hidden: `states` does NOT read
the screens ledger, so the [2026] VJS-CC-OPBOX audit D1 fix does not reach it and
cannot. `test_states_does_not_consume_the_screens_ledger` asserts that as a fact
about the tool instead of leaving a reader to assume the D1 wall is everywhere.
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from vdsfixture import EXIT_VACUOUS, PROOF_SCRIPT, VdsProjectCase  # noqa: E402

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


class StatesTest(VdsProjectCase):
    SCREENS = {"app/home/page.tsx": SCREEN}
    COMPONENTS = ("button",)

    # -- the failing direction ----------------------------------------------

    def test_required_state_not_drawn_fails_and_names_the_states(self) -> None:
        """The seeded violation. It names `hover`, not "1 state missing"."""
        self.register("Button", "button", require="default,hover", drawn="default")
        result = self.proof("states", "--no-capture")
        self.assert_violation(
            result,
            ".vds/register/CMP-0001.yaml [CMP-0001]",
            "every required state must be drawn",
            "CMP-0001 ('Button') draws every required state: default, hover",
            "states.drawn is [default], missing: hover",
            count=1,
        )

    def test_all_missing_states_are_named_not_only_the_first(self) -> None:
        self.register(
            "Button", "button", require="default,hover,focus,disabled", drawn="default"
        )
        result = self.proof("states", "--no-capture")
        self.assert_violation(result, "missing: hover, focus, disabled", count=1)

    def test_a_tenth_state_fails_even_though_the_schema_refuses_it_too(self) -> None:
        """The CLI will not write a tenth state, so the fixture hand-edits one in.

        A proof that relies on its writer having been careful has proven the
        writer. VDS S-5(3) fixes the nine, so the proof re-checks the nine.
        """
        self.register("Button", "button", require="default", drawn="default")
        self.edit_record(
            "CMP-0001",
            "  required:\n    - default\n",
            "  required:\n    - default\n    - twilight\n",
        )
        result = self.proof("states", "--no-capture")
        self.assert_violation(
            result,
            "[CMP-0001]",
            "the nine states are fixed and a record may not invent a tenth",
            "states.required contains twilight",
        )

    # -- the lifecycle carve-outs -------------------------------------------

    def test_proposed_record_is_skipped_and_the_run_says_so(self) -> None:
        """A `proposed` record has nothing drawn by construction. With no other
        record, the run enforces zero rows, which is VACUOUS (exit 3) and not a
        pass: VDS S-7(2)(4), and the D3 defect of [2026] VJS-CC-OPBOX 3."""
        self.register("Button", "button", status="proposed", require="default", drawn="")
        result = self.proof("states")
        self.assertEqual(result.returncode, EXIT_VACUOUS, result.stdout + result.stderr)
        self.assertIn("not enforced, status_proposed_nothing_drawn_yet: 1", result.stdout)
        self.assertIn("VACUOUS: this proof cannot currently fail", result.stdout)

    def test_unbuilt_required_states_are_counted_not_failed(self) -> None:
        """The drawn-but-not-built gap belongs to the `parity` proof, which VDS
        specifies and does not implement. Counting it here keeps the omission
        visible instead of letting a green `states` run imply parity."""
        self.register(
            "Button", "button", status="built", require="default,hover",
            drawn="default,hover", built="default",
        )
        result = self.proof("states")
        self.assert_clean_pass(result, rows=1)
        self.assertIn("required but not built: hover", result.stdout)
        self.assertIn("That gap is the `parity` proof's to fail on", result.stdout)

    # -- the passing direction ----------------------------------------------

    def test_clean_record_passes(self) -> None:
        self.register("Button", "button", require="default,hover", drawn="default,hover")
        result = self.proof("states")
        self.assert_clean_pass(result, rows=1)
        self.assertEqual(len(self.proof_records()), 1)

    def test_empty_register_is_vacuous_not_passed(self) -> None:
        result = self.proof("states")
        self.assertEqual(result.returncode, EXIT_VACUOUS, result.stdout + result.stderr)
        self.assertIn("the register is empty", result.stdout)
        self.assertIn("VACUOUS", result.stdout)

    # -- the D1 wall does not reach here, and that is recorded ---------------

    def test_states_does_not_consume_the_screens_ledger(self) -> None:
        """A negative result, asserted rather than assumed.

        The D1 fix routes every ledger-reading proof through
        `proofbase.load_surface_ledger`. `states` reads the register and never
        the ledger, so tampering with the ledger changes nothing here. That is
        correct, and it is also a limit: the D1 guarantee is per-proof, not
        global, and a reader who assumes otherwise will over-claim.
        """
        source = PROOF_SCRIPT["states"].read_text(encoding="utf-8")
        self.assertNotIn("load_surface_ledger", source)
        self.assertNotIn("scan.", source)

        self.register("Button", "button", require="default,hover", drawn="default,hover")
        clean = self.proof("states", "--no-capture")
        self.assert_clean_pass(clean, rows=1)

        self.drop_reference_and_reseal("Button")
        tampered = self.proof("states", "--no-capture")
        self.assert_clean_pass(tampered, rows=1)

        # The proofs that DO read it refuse the same tree, which is what stops
        # the tampering being useful even though `states` is indifferent to it.
        self.assert_proof_refused(
            self.proof("composition"), "not the rows the declared surface actually produces"
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
