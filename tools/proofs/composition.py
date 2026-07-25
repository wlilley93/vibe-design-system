#!/usr/bin/env python3
"""VDS proof: composition. The anti-drift proof.

VDS S-7(5): "no screen uses an unregistered component". Where
register_completeness asks whether a record EXISTS, composition asks whether the
thing being used is in a state fit to be used. A record sitting at `proposed` or
`designed` is not registered, so composing with it is drift, and drift authored
before anyone asked whether the thing was registered is the exact failure
VDS S-6(2) describes.

Three fatal rules and one warning:

  R1  a governed component reference with no register record at all
  R2  a governed component reference whose record is not in an enforceable
      status (registered, built, verified)
  R3  a reference to a RETIRED component. VDS S-9(8) inverts the test after
      retirement: the code being there is the defect.
  W1  a reference to a DEPRECATED component. VDS S-9(6)(1) requires every
      consuming site to be reported, per site, by route. A deprecated component
      never passes silently. It is a warning and not a violation, so it is
      printed in full and counted, and it does not by itself fail the gate.

Bare HTML elements are informational rows, counted in rows_considered and
excluded from rows_enforced, per VDS S-9(10) RESERVED (SUBMISSION-VDS-005).

This proof reads component NAMES, import PATHS and lifecycle STATUSES. It reads
no design value (VDS S-2(2)).
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from vdslib import scan  # noqa: E402
from vdslib.core import EXIT_PRECONDITION  # noqa: E402
from vdslib.proofbase import (  # noqa: E402
    ENFORCEABLE_STATUSES,
    RegisterIndex,
    guarded,
    new_run,
    open_project,
    proof_argparser,
)

KIND = "composition"
SCRIPT = "tools/proofs/composition.py"
RULE_UNREGISTERED = "VDS S-7(5) composition R1: no screen uses an unregistered component"
RULE_NOT_ENFORCEABLE = "VDS S-7(5) composition R2 / S-5(4): status is not a registered state"
RULE_RETIRED = "VDS S-9(8) composition R3: after retirement the code being there is the defect"
RESERVED_NOTE = (
    "relies on VDS S-9(10) RESERVED (SUBMISSION-VDS-005): bare HTML elements are "
    "informational rows only, excluded from rows_enforced. Any warrant citing this "
    "proof must record that reliance in its `reserved` array."
)


def run(argv: list[str] | None = None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    parser = proof_argparser(KIND, "No screen on the declared surface uses an unregistered component.")
    args = parser.parse_args(argv)

    project = open_project(args)
    run_ = new_run(project, KIND, SCRIPT, argv, args)
    run_.add_input(project.config_path)

    ledger = scan.load_fresh(project)
    run_.add_input(project.screens_ledger_path)

    index = RegisterIndex(project)
    for record in index.records:
        run_.add_input(Path(record["__path"]))

    prefixes = tuple(project.surface.get("governed_import_prefixes") or ())
    if not prefixes:
        run_.note(
            "[surface] governed_import_prefixes is empty, so no reference can be "
            "enforced; every row will be skipped and this run will be vacuous"
        )
    run_.note(RESERVED_NOTE)

    deprecated_sites: list[str] = []

    for screen in ledger.get("screens", []):
        route = screen.get("route", "<unknown route>")
        for ref in screen.get("references", []):
            run_.consider()
            name = str(ref.get("name", ""))
            root = name.split(".")[0]
            line = ref.get("line", 0)
            location = f"{route}:{line} <{name}>"

            if ref.get("kind") != "component":
                run_.skip("bare_element_informational_vds_s9_10")
                continue

            import_path = ref.get("importPath")
            if not import_path:
                run_.skip("component_reference_with_no_resolvable_import")
                continue
            if not str(import_path).startswith(prefixes):
                run_.skip("import_outside_governed_prefixes")
                continue

            run_.enforce()
            record = index.lookup(str(import_path), root)
            if record is None:
                misses = index.near_misses(str(import_path), root)
                detail = "; ".join(misses) if misses else "no register record names it at all"
                run_.fail(
                    location=location,
                    rule=RULE_UNREGISTERED,
                    expected=(
                        f"a register record with code.importPath {import_path!r} and "
                        f"code.exportName {root!r}, in status one of "
                        f"{', '.join(ENFORCEABLE_STATUSES)}"
                    ),
                    actual=f"unregistered: no such record ({detail})",
                )
                continue

            status = str(record.get("status", ""))
            record_id = str(record.get("id", "?"))
            if status == "retired":
                run_.fail(
                    location=location,
                    rule=RULE_RETIRED,
                    expected=f"{record_id} is retired, so no screen may reference it",
                    actual=(
                        f"{record_id} status {status!r}, retiredAt "
                        f"{record.get('retiredAt')!r}, still consumed here"
                    ),
                )
            elif status == "deprecated":
                superseded_by = record.get("supersededBy")
                successor = superseded_by if superseded_by else "nothing (withdrawn outright)"
                deprecated_sites.append(
                    f"{location} uses {record_id} ({record.get('name')!r}), deprecated at "
                    f"{record.get('deprecatedAt')!r}, superseded by {successor}"
                )
                run_.skip("warning_deprecated_component_consumed")
            elif status not in ENFORCEABLE_STATUSES:
                run_.fail(
                    location=location,
                    rule=RULE_NOT_ENFORCEABLE,
                    expected=(
                        f"{record_id} in status one of {', '.join(ENFORCEABLE_STATUSES)} "
                        "before any screen composes with it"
                    ),
                    actual=(
                        f"{record_id} status {status!r}: the record exists but the "
                        "component is not registered, so this is drift"
                    ),
                )

    if deprecated_sites:
        run_.note(
            f"{len(deprecated_sites)} consuming sites of DEPRECATED components "
            "(VDS S-9(6)(1): a deprecated component never passes silently):"
        )
        for site in sorted(deprecated_sites):
            run_.note(f"  deprecated-consumer: {site}")

    return run_.report(allow_vacuous=args.allow_vacuous, capture=not args.no_capture)


def main() -> int:
    return guarded(run)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        sys.exit(EXIT_PRECONDITION)
