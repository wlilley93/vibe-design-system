"""The screens ledger: a generated inventory of what the declared surface uses.

VDS S-4(2): ledgers are generated inventories, never hand-edited, and each must
have a staleness test that fails when its source changed and the generator was
not re-run. `check_fresh` is that test, and every proof that reads the ledger
calls it before it reads a single row, so a stale ledger cannot produce a pass.

This module reads component and element NAMES and import PATHS. It reads no
colour, length, radius, font, duration or easing curve, so nothing it writes is
a realisation under VDS S-2(4).
"""

from __future__ import annotations

import re
from pathlib import Path

from . import yamlish
from .core import Project, VdsError, digest_of, now_iso, sha256_file

LEDGER_SCHEMA_VERSION = 1
GENERATOR_COMMAND = "vds.py ledger screens"

_BLOCK_COMMENT = re.compile(r"/\*.*?\*/", re.DOTALL)
_LINE_COMMENT = re.compile(r"(?<![:\w])//[^\n]*")
_IMPORT = re.compile(
    r"^\s*import\s+(?P<type>type\s+)?(?P<clause>[^;'\"]*?)\s*from\s*['\"](?P<path>[^'\"]+)['\"]",
    re.MULTILINE,
)
_JSX_OPEN = re.compile(r"<([A-Za-z][A-Za-z0-9_$]*(?:\.[A-Za-z][A-Za-z0-9_$]*)*)")


def _strip_comments(source: str) -> str:
    """Blank out comments while preserving line numbering."""

    def blank(match: re.Match) -> str:
        return re.sub(r"[^\n]", " ", match.group(0))

    return _LINE_COMMENT.sub(blank, _BLOCK_COMMENT.sub(blank, source))


def parse_imports(source: str) -> dict[str, str]:
    """Map an imported local name to the module specifier it came from.

    Type-only imports are excluded: a type is not a rendered component.
    """
    bindings: dict[str, str] = {}
    for match in _IMPORT.finditer(source):
        if match.group("type"):
            continue
        module = match.group("path")
        clause = match.group("clause").strip()
        if not clause:
            continue
        brace = re.search(r"\{(.*)\}", clause, re.DOTALL)
        outside = clause[: brace.start()] if brace else clause
        for name in outside.split(","):
            name = name.strip()
            if name.startswith("* as "):
                name = name[5:].strip()
            if name and re.fullmatch(r"[A-Za-z_$][\w$]*", name):
                bindings[name] = module
        if brace:
            for entry in brace.group(1).split(","):
                entry = entry.strip()
                if not entry or entry.startswith("type "):
                    continue
                parts = re.split(r"\s+as\s+", entry)
                local = parts[-1].strip()
                if local and re.fullmatch(r"[A-Za-z_$][\w$]*", local):
                    bindings[local] = module
    return bindings


def parse_references(source: str) -> list[dict]:
    """Every JSX tag opened in the file, with its line number.

    A tag whose root identifier begins with a capital is a component reference.
    Anything else is a bare HTML element, which VDS S-9(10) holds RESERVED
    (SUBMISSION-VDS-005) and which is therefore recorded but not enforced.
    """
    clean = _strip_comments(source)
    line_starts = [0]
    for i, ch in enumerate(clean):
        if ch == "\n":
            line_starts.append(i + 1)

    def line_of(offset: int) -> int:
        low, high = 0, len(line_starts) - 1
        while low < high:
            mid = (low + high + 1) // 2
            if line_starts[mid] <= offset:
                low = mid
            else:
                high = mid - 1
        return low + 1

    seen: dict[tuple[str, int], dict] = {}
    for match in _JSX_OPEN.finditer(clean):
        full = match.group(1)
        root = full.split(".")[0]
        kind = "component" if root[0].isupper() else "element"
        line = line_of(match.start())
        key = (full, line)
        if key in seen:
            seen[key]["count"] += 1
            continue
        seen[key] = {"name": full, "root": root, "kind": kind, "line": line, "count": 1}
    return [seen[key] for key in sorted(seen)]


def screen_files(project: Project) -> list[Path]:
    globs = project.surface.get("screen_globs") or []
    if not globs:
        raise VdsError(
            f"{project.rel(project.config_path)}: [surface] screen_globs is empty. "
            "A declared surface of nothing proves nothing."
        )
    found: set[Path] = set()
    for pattern in globs:
        for path in project.root.glob(pattern):
            if path.is_file():
                found.add(path)
    return sorted(found)


def source_digest(project: Project, files: list[Path]) -> str:
    return digest_of([[project.rel(f), sha256_file(f)] for f in files])


def generate(project: Project) -> dict:
    """Build the screens ledger from the declared surface."""
    files = screen_files(project)
    screens = []
    for path in files:
        source = path.read_text(encoding="utf-8", errors="replace")
        imports = parse_imports(source)
        references = []
        for ref in parse_references(source):
            references.append(
                {
                    "name": ref["name"],
                    "kind": ref["kind"],
                    "importPath": imports.get(ref["root"]),
                    "line": ref["line"],
                    "count": ref["count"],
                }
            )
        screens.append(
            {
                "route": project.rel(path),
                "digest": sha256_file(path),
                "references": references,
            }
        )
    return {
        "schema_version": LEDGER_SCHEMA_VERSION,
        "generated_at": now_iso(),
        "generated_by": GENERATOR_COMMAND,
        "source_globs": list(project.surface.get("screen_globs") or []),
        "source_digest": source_digest(project, files),
        "screens": screens,
    }


def write(project: Project) -> tuple[Path, dict]:
    ledger = generate(project)
    path = project.screens_ledger_path
    path.parent.mkdir(parents=True, exist_ok=True)
    yamlish.dump(ledger, path)
    return path, ledger


def check_fresh(project: Project, ledger: dict) -> None:
    """VDS S-4(2). Refuse to proceed on a stale ledger, and say what moved."""
    recorded_globs = list(ledger.get("source_globs") or [])
    configured_globs = list(project.surface.get("screen_globs") or [])
    if recorded_globs != configured_globs:
        raise VdsError(
            "the screens ledger is STALE: [surface] screen_globs changed since it was "
            f"generated.\n  ledger: {recorded_globs}\n  config: {configured_globs}\n"
            "  Regenerate with: vds.py ledger screens"
        )
    files = screen_files(project)
    current = source_digest(project, files)
    if current != ledger.get("source_digest"):
        recorded = {s.get("route"): s.get("digest") for s in ledger.get("screens", [])}
        live = {project.rel(f): sha256_file(f) for f in files}
        added = sorted(set(live) - set(recorded))
        removed = sorted(set(recorded) - set(live))
        changed = sorted(p for p in set(live) & set(recorded) if live[p] != recorded[p])
        detail = []
        if added:
            detail.append("  added since generation:   " + ", ".join(added))
        if removed:
            detail.append("  removed since generation: " + ", ".join(removed))
        if changed:
            detail.append("  changed since generation: " + ", ".join(changed))
        raise VdsError(
            "the screens ledger is STALE, so no result from it can be trusted "
            "(VDS S-4(2)).\n"
            + "\n".join(detail or ["  the source set digest differs and no per-file cause was found"])
            + "\n  Regenerate with: vds.py ledger screens"
        )


def load_fresh(project: Project) -> dict:
    """Load the screens ledger and refuse it if it is stale."""
    path = project.screens_ledger_path
    if not path.is_file():
        raise VdsError(
            f"{project.rel(path)} is absent. The declared surface is a generated ledger "
            "(VDS S-4(2)). Run: vds.py ledger screens"
        )
    ledger = yamlish.load(path)
    if not isinstance(ledger, dict) or "screens" not in ledger:
        raise VdsError(f"{project.rel(path)}: not a screens ledger")
    if ledger.get("schema_version") != LEDGER_SCHEMA_VERSION:
        raise VdsError(
            f"{project.rel(path)}: ledger schema_version "
            f"{ledger.get('schema_version')!r} exceeds what this tool understands "
            f"({LEDGER_SCHEMA_VERSION}). Refusing rather than skipping what it cannot "
            "parse (VDS S-11(2))."
        )
    check_fresh(project, ledger)
    return ledger
