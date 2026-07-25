"""Shared scaffolding for the proof scripts.

Every proof script is independently runnable, because `.vds/enforcement.lock`
pins the SCRIPT and something other than the author has to be able to invoke it
(VDS S-7(2)(3)). This module holds only what all of them share: argument
parsing, project resolution, the register index, and the error discipline that
turns a precondition failure into a loud exit 2 rather than a quiet pass.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from . import scan
from .core import (
    EXIT_PRECONDITION,
    LIFECYCLE,
    Project,
    ProofRun,
    VdsError,
    find_project,
)

# A component in one of these states is registered for composition purposes.
# `proposed` and `designed` are NOT: a design that is merely drawn is exactly
# what the anti-drift proof exists to catch being used.
ENFORCEABLE_STATUSES = ("registered", "built", "verified")

INVOCATION_SURFACES = (
    "githook_pre_commit",
    "githook_pre_push",
    "ci_workflow",
    "package_script",
    "build",
    "manual",
)


def proof_argparser(kind: str, description: str) -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog=f"proofs/{kind}.py",
        description=description,
    )
    parser.add_argument(
        "--root",
        default=None,
        help="project root holding .vds/config.toml (default: search upward from cwd)",
    )
    parser.add_argument(
        "--invoked-by",
        default="manual",
        choices=INVOCATION_SURFACES,
        help="recorded honestly on the proof record; 'manual' does not satisfy VDS S-7(2)(3)",
    )
    parser.add_argument(
        "--allow-vacuous",
        action="store_true",
        help="exit 0 instead of 3 when no row is in an enforceable state (the vacuity is "
        "still recorded and still says so in the output)",
    )
    parser.add_argument(
        "--no-capture",
        action="store_true",
        help="do not write a proof record; for local inspection only, and a run made this "
        "way can never be cited as evidence",
    )
    return parser


def open_project(args: argparse.Namespace) -> Project:
    return find_project(Path(args.root) if args.root else None)


def new_run(project: Project, kind: str, script: str, argv: list[str], args) -> ProofRun:
    return ProofRun(
        project=project,
        kind=kind,
        script=script,
        command=" ".join(["python3", script, *argv]),
        invoked_by=args.invoked_by,
    )


def guarded(main_fn) -> int:
    """Run a proof main, turning a precondition failure into a loud exit 2."""
    try:
        return main_fn()
    except VdsError as exc:
        print("PRECONDITION FAILED, this proof did not run and proves nothing:", file=sys.stderr)
        print(f"  {exc}", file=sys.stderr)
        return EXIT_PRECONDITION


class RegisterIndex:
    """Indexes the component register for the two lookups the proofs need."""

    def __init__(self, project: Project) -> None:
        self.project = project
        self.records = project.read_register()
        self.by_id: dict[str, dict] = {}
        self.by_code: dict[tuple[str, str], dict] = {}
        self.by_export: dict[str, list[dict]] = {}
        self.by_import: dict[str, list[dict]] = {}
        for record in self.records:
            record_id = str(record.get("id", ""))
            if record_id in self.by_id:
                raise VdsError(
                    f"duplicate register id {record_id} in "
                    f"{project.rel(Path(record['__path']))} and "
                    f"{project.rel(Path(self.by_id[record_id]['__path']))}. "
                    "An identifier collision is a fail-closed error, never a silent "
                    "overwrite (VDS S-4(4))."
                )
            self.by_id[record_id] = record
            code = record.get("code")
            if isinstance(code, dict):
                import_path = str(code.get("importPath", ""))
                export_name = str(code.get("exportName", ""))
                self.by_code[(import_path, export_name)] = record
                self.by_export.setdefault(export_name, []).append(record)
                self.by_import.setdefault(import_path, []).append(record)

    def __len__(self) -> int:
        return len(self.records)

    def lookup(self, import_path: str, export_name: str) -> dict | None:
        return self.by_code.get((import_path, export_name))

    def near_misses(self, import_path: str, export_name: str) -> list[str]:
        """Why a lookup missed, in terms a reader can act on."""
        out = []
        for record in self.by_export.get(export_name, []):
            code = record.get("code") or {}
            out.append(
                f"{record.get('id')} exports {export_name!r} but from "
                f"{code.get('importPath')!r}"
            )
        for record in self.by_import.get(import_path, []):
            code = record.get("code") or {}
            out.append(
                f"{record.get('id')} is at {import_path!r} but exports "
                f"{code.get('exportName')!r}"
            )
        return sorted(set(out))

    def paths_for(self, record: dict) -> str:
        return self.project.rel(Path(record["__path"]))


def lifecycle_index(status: str) -> int:
    try:
        return LIFECYCLE.index(status)
    except ValueError as exc:
        raise VdsError(f"unknown lifecycle status {status!r}") from exc
