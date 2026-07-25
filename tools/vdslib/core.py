"""Shared VDS machinery: project discovery, digests, identifier allocation and
proof capture.

Nothing in this module reads or writes a design VALUE. It reads registrations,
ledgers, locks and proof records, and it digests file bytes. VDS S-2(2): `.vds/`
stores no design values, and neither does anything that walks it.
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import sys
import tomllib
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path

from . import yamlish
from .schema import ValidationError, load_schema

# Exit codes are a contract. A caller reads them; a human reads the text above them.
EXIT_PASSED = 0
EXIT_VIOLATION = 1
EXIT_PRECONDITION = 2
EXIT_VACUOUS = 3

TOOLS_DIR = Path(__file__).resolve().parents[1]
VDS_HOME = TOOLS_DIR.parent

NINE_STATES = (
    "default",
    "hover",
    "focus",
    "active",
    "selected",
    "disabled",
    "loading",
    "error",
    "success",
)

# VDS S-5(4). The path is directed and skipping is forbidden.
LIFECYCLE = (
    "proposed",
    "designed",
    "registered",
    "built",
    "verified",
    "deprecated",
    "retired",
)

PROOF_KINDS = (
    "register_completeness",
    "reconciliation",
    "composition",
    "contrast",
    "states",
    "parity",
    "token_pin",
    "retirement_drain",
    "ledger_staleness",
    "no_stored_values",
)

# The proof kinds this tooling actually implements. Everything else in
# PROOF_KINDS is specified and unbuilt, and saying so is cheaper than letting a
# reader assume ten scripts exist.
IMPLEMENTED_PROOF_KINDS = ("register_completeness", "composition", "states")


class VdsError(Exception):
    """A precondition failed. Always fatal, never downgraded to a warning."""


# --------------------------------------------------------------------- digests


def sha256_bytes(data: bytes) -> str:
    return "sha256:" + hashlib.sha256(data).hexdigest()


def sha256_text(text: str) -> str:
    return sha256_bytes(text.encode("utf-8"))


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(65536), b""):
            digest.update(chunk)
    return "sha256:" + digest.hexdigest()


def canonical_json(value: object) -> str:
    """The one normalisation used before digesting any structure."""
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True)


def digest_of(value: object) -> str:
    return sha256_text(canonical_json(value))


def now_iso() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def actor() -> str:
    return os.environ.get("VDS_ACTOR") or os.environ.get("USER") or "unknown"


# ---------------------------------------------------------------------- config

DEFAULT_CONFIG = """# VDS project configuration. The one fixed anchor (VDS S-3(7)).
# This file holds NO design value (VDS S-2(2)). Paths, globs and governance only.
version = 1
jurisdiction_id = "{jurisdiction_id}"
repo_code = "{repo_code}"
designpack = "none@0"

[paths]
register = ".vds/register"
warrants = ".vds/warrants"
proofs = ".vds/proofs"
pins = ".vds/pins"
ledgers = ".vds/ledgers"
submissions = ".vds/submissions"
logs = ".vds/logs"
permits = ".vds/permits"
# Where the artefact schemas live. Defaults to the schema/ directory shipped
# beside these tools when left unset.
schema = ""

[surface]
# The DECLARED SURFACE. Every VDS claim is bounded by it, and a screen outside
# these globs is outside every proof (docs/GOAL.md, "what VDS will not prove").
screen_globs = ["app/**/page.tsx"]
# A component reference whose import path starts with one of these is IN SCOPE
# for enforcement. Anything else is counted and not enforced, and the count is
# printed, so the carve-out is visible rather than assumed.
governed_import_prefixes = ["@/components/"]
# Directories the register is expected to cover, used by reconciliation.
library_dirs = ["src/components/ui"]
screens_ledger = ".vds/ledgers/screens.yaml"

[governance]
# VDS S-3(8): the enforcement machinery must not be editable without a permit.
permit_required = [
  "app/globals.css",
  "src/components/**",
  "designpack/v1/**",
  ".vds/register/**",
  ".vds/config.toml",
  "tools/proofs/**",
  "tools/vds.py",
]
permit_exempt = [".vds/logs/**", ".vds/permits/**", ".vds/proofs/**"]
"""


@dataclass
class Project:
    root: Path
    config: dict
    config_path: Path

    # -- paths ---------------------------------------------------------------

    def path(self, role: str) -> Path:
        paths = self.config.get("paths", {})
        rel = paths.get(role)
        if not rel:
            raise VdsError(f"config {self.config_path} has no [paths] entry for {role!r}")
        return self.root / rel

    @property
    def vds_dir(self) -> Path:
        return self.root / ".vds"

    @property
    def schema_dir(self) -> Path:
        rel = self.config.get("paths", {}).get("schema") or ""
        if rel:
            return self.root / rel
        return VDS_HOME / "schema"

    @property
    def surface(self) -> dict:
        return self.config.get("surface", {})

    @property
    def screens_ledger_path(self) -> Path:
        return self.root / self.surface.get("screens_ledger", ".vds/ledgers/screens.yaml")

    @property
    def jurisdiction_id(self) -> str:
        return str(self.config.get("jurisdiction_id", "unknown"))

    def rel(self, path: Path) -> str:
        try:
            return str(Path(path).resolve().relative_to(self.root.resolve()))
        except ValueError:
            return str(path)

    # -- artefact IO ---------------------------------------------------------

    def validate_artefact(self, schema_name: str, instance: object, where: str) -> None:
        errors = load_schema(self.schema_dir, schema_name).errors_for(instance)
        if errors:
            raise VdsError(
                f"{where} does not validate against {schema_name}.schema.json:\n  "
                + "\n  ".join(errors)
            )

    def write_artefact(self, schema_name: str, path: Path, instance: dict) -> None:
        self.validate_artefact(schema_name, instance, self.rel(path))
        path.parent.mkdir(parents=True, exist_ok=True)
        yamlish.dump(instance, path)

    def read_register(self) -> list[dict]:
        directory = self.path("register")
        if not directory.is_dir():
            return []
        records = []
        for file in sorted(directory.glob("*.yaml")):
            try:
                record = yamlish.load(file)
            except Exception as exc:
                raise VdsError(f"{self.rel(file)}: unreadable register record: {exc}") from exc
            if not isinstance(record, dict):
                raise VdsError(f"{self.rel(file)}: register record is not a mapping")
            record["__path"] = str(file)
            records.append(record)
        return records

    def designpack_digest(self) -> str:
        lock = self.vds_dir / "designpack.lock"
        if not lock.is_file():
            raise VdsError(
                f"{self.rel(lock)} is absent. A proof records the designpack digest in force "
                f"when it ran (VDS S-11(1)). Run: vds.py init"
            )
        data = yamlish.load(lock)
        if not isinstance(data, dict) or not str(data.get("digest", "")).startswith("sha256:"):
            raise VdsError(f"{self.rel(lock)}: no usable 'digest' field")
        return str(data["digest"])

    # -- identifier allocation (VDS S-4(4)) ----------------------------------

    def next_component_id(self) -> str:
        highest = 0
        directory = self.path("register")
        if directory.is_dir():
            for file in directory.glob("*.yaml"):
                match = re.fullmatch(r"CMP-([0-9]{4})", file.stem)
                if match:
                    highest = max(highest, int(match.group(1)))
        if highest >= 9999:
            raise VdsError("component id space CMP-0001..CMP-9999 is exhausted")
        return f"CMP-{highest + 1:04d}"

    def next_warrant_id(self, stage_number: int) -> str:
        highest = 0
        directory = self.path("warrants")
        if directory.is_dir():
            for file in directory.glob("*.yaml"):
                match = re.fullmatch(rf"WARRANT-W{stage_number}-([0-9]{{3}})", file.stem)
                if match:
                    highest = max(highest, int(match.group(1)))
        return f"WARRANT-W{stage_number}-{highest + 1:03d}"

    def next_proof_id(self) -> str:
        directory = self.path("proofs")
        stamp = datetime.now(timezone.utc)
        for bump in range(0, 3600):
            candidate = (stamp.timestamp() + bump)
            text = datetime.fromtimestamp(candidate, timezone.utc).strftime("PROOF-%Y%m%d-%H%M%S")
            if not (directory / f"{text}.yaml").exists():
                return text
        raise VdsError("could not allocate a free proof id within an hour of now")


def find_project(start: Path | None = None) -> Project:
    """Walk up from `start` looking for `.vds/config.toml`."""
    here = Path(start or Path.cwd()).resolve()
    for candidate in [here, *here.parents]:
        config_path = candidate / ".vds" / "config.toml"
        if config_path.is_file():
            with open(config_path, "rb") as fh:
                config = tomllib.load(fh)
            return Project(root=candidate, config=config, config_path=config_path)
    raise VdsError(
        f"no .vds/config.toml found at or above {here}. Run: vds.py init --root <project>"
    )


# ---------------------------------------------------------------- proof capture


@dataclass
class Violation:
    location: str
    rule: str
    expected: str
    actual: str
    severity: str = "fatal"

    def as_dict(self) -> dict:
        return {
            "location": self.location,
            "rule": self.rule,
            "expected": self.expected,
            "actual": self.actual,
            "severity": self.severity,
        }


@dataclass
class ProofRun:
    """Accumulates a proof's result, prints it, captures it and exits.

    The capture is a side effect of running (VDS S-7(2)(5)), which is why there
    is no way to hand-write a proof record through this class.
    """

    project: Project
    kind: str
    script: str
    command: str
    invoked_by: str = "manual"
    rows_considered: int = 0
    rows_enforced: int = 0
    violations: list[Violation] = field(default_factory=list)
    skipped: dict[str, int] = field(default_factory=dict)
    notes: list[str] = field(default_factory=list)
    inputs: dict[str, str] = field(default_factory=dict)
    started: datetime = field(default_factory=lambda: datetime.now(timezone.utc))

    def consider(self, count: int = 1) -> None:
        self.rows_considered += count

    def enforce(self, count: int = 1) -> None:
        self.rows_enforced += count

    def skip(self, reason: str, count: int = 1) -> None:
        self.skipped[reason] = self.skipped.get(reason, 0) + count

    def note(self, line: str) -> None:
        self.notes.append(line)

    def fail(self, location: str, rule: str, expected: str, actual: str) -> None:
        self.violations.append(Violation(location, rule, expected, actual, "fatal"))

    def add_input(self, path: Path) -> None:
        try:
            self.inputs[self.project.rel(path)] = sha256_file(path)
        except OSError:
            self.inputs[self.project.rel(path)] = "sha256:" + "0" * 64

    # -- outcome -------------------------------------------------------------

    @property
    def fatal(self) -> list[Violation]:
        return [v for v in self.violations if v.severity == "fatal"]

    def status_and_exit(self, allow_vacuous: bool) -> tuple[str, int]:
        if self.fatal:
            return "failed", EXIT_VIOLATION
        if self.rows_enforced == 0:
            return "vacuous", (EXIT_PASSED if allow_vacuous else EXIT_VACUOUS)
        return "passed", EXIT_PASSED

    def _result_record(self, status: str, exit_code: int, proof_id: str) -> dict:
        inputs_digest = digest_of(sorted(self.inputs.items()))
        core = {
            "kind": self.kind,
            "status": status,
            "rows_considered": self.rows_considered,
            "rows_enforced": self.rows_enforced,
            "rows_skipped_reasons": dict(sorted(self.skipped.items())),
            "violations": sorted(
                (v.as_dict() for v in self.violations),
                key=lambda v: (v["location"], v["rule"], v["actual"]),
            ),
            "inputs_digest": inputs_digest,
        }
        record = {
            "id": proof_id,
            "kind": self.kind,
            "status": status,
            "warrant_id": None,
            "command": self.command,
            "script": self.script,
            "exit_code": exit_code,
            "rows_considered": self.rows_considered,
            "rows_enforced": self.rows_enforced,
            "rows_skipped_reasons": core["rows_skipped_reasons"],
            "violations": core["violations"],
            "inputs_digest": inputs_digest,
            "digest": digest_of(core),
            "designpack_digest": self.project.designpack_digest(),
            "captured_at": now_iso(),
            "capture_mode": "automatic",
            "invoked_by": self.invoked_by,
            "duration_ms": int(
                (datetime.now(timezone.utc) - self.started).total_seconds() * 1000
            ),
        }
        script_path = self.project.root / self.script
        if script_path.is_file():
            record["script_digest"] = sha256_file(script_path)
        return record

    def report(self, allow_vacuous: bool = False, capture: bool = True, stream=None) -> int:
        """Print the result, capture the record, and return the exit code."""
        out = stream or sys.stdout
        status, exit_code = self.status_and_exit(allow_vacuous)

        print(f"proof: {self.kind}", file=out)
        print(f"script: {self.script}", file=out)
        print(f"rows_considered: {self.rows_considered}", file=out)
        print(f"rows_enforced:   {self.rows_enforced}", file=out)
        for reason, count in sorted(self.skipped.items()):
            print(f"  not enforced, {reason}: {count}", file=out)
        for line in self.notes:
            print(f"note: {line}", file=out)

        if self.fatal:
            print("", file=out)
            print(f"VIOLATIONS ({len(self.fatal)}), each named in full:", file=out)
            for i, violation in enumerate(
                sorted(self.fatal, key=lambda v: (v.location, v.rule)), start=1
            ):
                print(f"  [{i}] {violation.location}", file=out)
                print(f"      rule:     {violation.rule}", file=out)
                print(f"      expected: {violation.expected}", file=out)
                print(f"      actual:   {violation.actual}", file=out)
        elif self.rows_enforced == 0:
            # VDS S-7(2)(4). The exact words are required, and no PASS is printed
            # beside them, because a pass over zero enforceable rows is the defect
            # [2026] VJS-CC-OPBOX 3 D3 found, not evidence of parity.
            print("", file=out)
            print(
                "VACUOUS: this proof cannot currently fail, because no row is in an "
                "enforceable state.",
                file=out,
            )
            print(
                "  It is recorded as status: vacuous and is NOT evidence for any warrant "
                "(VDS S-7(2)(4)).",
                file=out,
            )
            if self.skipped:
                print("  Every row considered was skipped for these reasons:", file=out)
                for reason, count in sorted(self.skipped.items()):
                    print(f"    {reason}: {count}", file=out)
        else:
            print("", file=out)
            print(
                f"PASS: {self.rows_enforced} enforceable rows checked, 0 violations.",
                file=out,
            )

        record_id = "(not captured)"
        if capture:
            record = self._result_record(status, exit_code, self.project.next_proof_id())
            path = self.project.path("proofs") / f"{record['id']}.yaml"
            self.project.write_artefact("proof-result", path, record)
            record_id = record["id"]
            print("", file=out)
            print(f"captured: {self.project.rel(path)} (capture_mode: automatic)", file=out)
            print(f"digest:   {record['digest']}", file=out)
        print(f"status:   {status}    exit: {exit_code}", file=out)
        return exit_code


def load_ledger(project: Project, path: Path, what: str) -> dict:
    if not path.is_file():
        raise VdsError(
            f"{project.rel(path)} is absent. {what} is a generated ledger "
            f"(VDS S-4(2)). Run: vds.py ledger screens"
        )
    data = yamlish.load(path)
    if not isinstance(data, dict):
        raise VdsError(f"{project.rel(path)}: ledger is not a mapping")
    return data
