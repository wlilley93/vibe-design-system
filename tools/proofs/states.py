#!/usr/bin/env python3
"""VDS proof: states.

VDS S-7(5): "every required state of every registered component is drawn". The
nine states are fixed by VDS S-5(3); a record may require a subset and may not
invent a tenth.

A row is one register record.

  ENFORCED when the record's status is at or past `designed`, and is not
    `retired`. A `proposed` record has nothing drawn yet by construction, so
    enforcing it would fail every new registration and teach the author to skip
    the stage; a `retired` record is a tombstone and is kept forever (VDS S-9(6)).
  FATAL when required minus drawn is non-empty. The message names the missing
    states, not a count.
  FATAL when a state appears in `drawn` or `built` that is not in `required`
    only where it is also not one of the nine, which the schema already refuses;
    this proof re-checks it because a proof that trusts its input proves the
    input, not the claim.
  INFORMATIONAL where a record at `built` or `verified` has required states that
    are drawn but not built. That gap is the parity proof's to fail on, and
    counting it here means nobody has to remember it exists.

This proof reads state NAMES and lifecycle statuses. It reads no design value
(VDS S-2(2)); whether a drawn state looks right is taste, and taste is the
Principal's under VDS S-1(6).
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from vdslib.core import EXIT_PRECONDITION, NINE_STATES  # noqa: E402
from vdslib.proofbase import (  # noqa: E402
    RegisterIndex,
    guarded,
    lifecycle_index,
    new_run,
    open_project,
    proof_argparser,
)

KIND = "states"
SCRIPT = "tools/proofs/states.py"
RULE_MISSING = "VDS S-7(5) states / S-6(2) W2: every required state must be drawn"
RULE_TENTH = "VDS S-5(3): the nine states are fixed and a record may not invent a tenth"
DESIGNED = 1  # index of "designed" in the VDS S-5(4) lifecycle path


def _state_list(record: dict, bucket: str) -> list[str]:
    states = record.get("states") or {}
    value = states.get(bucket)
    if value is None:
        return []
    if not isinstance(value, list):
        return []
    return [str(v) for v in value]


def run(argv: list[str] | None = None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    parser = proof_argparser(KIND, "Every required state of every registered component is drawn.")
    args = parser.parse_args(argv)

    project = open_project(args)
    run_ = new_run(project, KIND, SCRIPT, argv, args)
    run_.add_input(project.config_path)

    index = RegisterIndex(project)
    if not index.records:
        run_.note(
            "the register is empty, so there is nothing whose states could be checked"
        )

    unbuilt_gaps: list[str] = []

    for record in index.records:
        run_.consider()
        path = Path(record["__path"])
        run_.add_input(path)
        location = project.rel(path)
        record_id = str(record.get("id", "?"))
        name = record.get("name", "?")
        status = str(record.get("status", ""))

        # The schema already refuses a tenth state. Re-checking is not
        # duplication: a proof that trusts its input has proven the input.
        for bucket in ("required", "drawn", "built"):
            invented = [s for s in _state_list(record, bucket) if s not in NINE_STATES]
            if invented:
                run_.fail(
                    location=f"{location} [{record_id}]",
                    rule=RULE_TENTH,
                    expected=f"states.{bucket} drawn only from: {', '.join(NINE_STATES)}",
                    actual=f"states.{bucket} contains {', '.join(sorted(invented))}",
                )

        if status == "proposed":
            run_.skip("status_proposed_nothing_drawn_yet")
            continue
        if status == "retired":
            run_.skip("status_retired_tombstone_vds_s9_6")
            continue
        if lifecycle_index(status) < DESIGNED:
            run_.skip("status_before_designed")
            continue

        run_.enforce()
        required = _state_list(record, "required")
        drawn = _state_list(record, "drawn")
        built = _state_list(record, "built")

        missing = [s for s in NINE_STATES if s in required and s not in drawn]
        if missing:
            run_.fail(
                location=f"{location} [{record_id}]",
                rule=RULE_MISSING,
                expected=(
                    f"{record_id} ({name!r}) draws every required state: "
                    f"{', '.join(s for s in NINE_STATES if s in required)}"
                ),
                actual=(
                    f"states.drawn is [{', '.join(drawn) or 'empty'}], missing: "
                    f"{', '.join(missing)}"
                ),
            )

        if status in ("built", "verified"):
            not_built = [s for s in NINE_STATES if s in required and s not in built]
            if not_built:
                unbuilt_gaps.append(
                    f"{location} [{record_id}] status {status}, required but not built: "
                    f"{', '.join(not_built)}"
                )

    if unbuilt_gaps:
        run_.skip("informational_required_states_not_built", len(unbuilt_gaps))
        run_.note(
            f"{len(unbuilt_gaps)} records have required states that are drawn but not "
            "built. That gap is the `parity` proof's to fail on (VDS S-7(5)), not this "
            "one, and it is counted here so it is not forgotten:"
        )
        for gap in sorted(unbuilt_gaps):
            run_.note(f"  not-built: {gap}")

    return run_.report(allow_vacuous=args.allow_vacuous, capture=not args.no_capture)


def main() -> int:
    return guarded(run)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        sys.exit(EXIT_PRECONDITION)
