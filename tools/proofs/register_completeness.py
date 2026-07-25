#!/usr/bin/env python3
"""VDS proof: register_completeness.

VDS S-7(5): "every component referenced by any declared screen exists in the
register". This is the EXISTENCE question. Whether the record that exists is in
a state fit to be used is the composition proof's question, not this one, and
keeping the two apart is what lets W1 be granted on existence alone.

A row is one (screen, component reference) pair drawn from the generated screens
ledger. A row is ENFORCED when its import path falls inside a governed prefix
declared in `[surface] governed_import_prefixes`. Everything else is counted in
rows_considered, excluded from rows_enforced, and printed with its reason, so the
carve-out is visible rather than assumed.

Bare HTML elements are informational only, per VDS S-9(10), which is RESERVED
pending SUBMISSION-VDS-005. Any warrant relying on this proof must say so.

This proof reads component NAMES and import PATHS. It reads no design value
(VDS S-2(2)).
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from vdslib import scan  # noqa: E402
from vdslib.core import EXIT_PRECONDITION  # noqa: E402
from vdslib.proofbase import (  # noqa: E402
    RegisterIndex,
    guarded,
    new_run,
    open_project,
    proof_argparser,
)

KIND = "register_completeness"
SCRIPT = "tools/proofs/register_completeness.py"
RULE = "VDS S-7(5) register_completeness"
RESERVED_NOTE = (
    "relies on VDS S-9(10) RESERVED (SUBMISSION-VDS-005): bare HTML elements are "
    "informational rows only, so the anti-drift reach of this run stops at the "
    "component layer"
)


def run(argv: list[str] | None = None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    parser = proof_argparser(
        KIND, "Every component the declared surface references exists in the register."
    )
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
    run_.note(f"register holds {len(index)} records; surface holds {len(ledger['screens'])} screens")

    for screen in ledger.get("screens", []):
        route = screen.get("route", "<unknown route>")
        for ref in screen.get("references", []):
            run_.consider()
            name = str(ref.get("name", ""))
            root = name.split(".")[0]
            line = ref.get("line", 0)
            location = f"{route}:{line}"

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
                    location=f"{location} <{name}>",
                    rule=RULE,
                    expected=(
                        f"a register record with code.importPath {import_path!r} and "
                        f"code.exportName {root!r}"
                    ),
                    actual=f"no such record ({detail})",
                )

    return run_.report(allow_vacuous=args.allow_vacuous, capture=not args.no_capture)


def main() -> int:
    return guarded(run)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        sys.exit(EXIT_PRECONDITION)
