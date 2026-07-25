"""The screens ledger: a generated inventory of what the declared surface uses.

VDS S-4(2): ledgers are generated inventories, never hand-edited, and each must
have a staleness test that fails when its source changed and the generator was
not re-run. `check_authentic` is that test, and every proof that reads the
ledger calls it before it reads a single row.

Two things have to hold, and until 2026-07-25 only the first was checked.

  FRESH      the recorded `source_digest` still matches the live screen files,
             so nobody changed a screen and left the ledger behind.
  AUTHENTIC  the derived rows in the ledger are the rows the generator would
             produce from those same files, so nobody hand-edited the answer.

The freshness limb alone is worthless as an anti-drift guarantee, because the
rows a proof actually reads are the derived ones. [2026] VJS-CC-OPBOX audit D1:
an auditor deleted a five-line `- name: Rogue` entry from `screens.yaml` by
hand, touched nothing else, and turned `composition` from exit 1 with one
violation into exit 0 "PASS", captured as a `capture_mode: automatic` proof
record attesting to it.

The authenticating check here is RE-DERIVATION, not a stored digest, and the
choice matters. A digest kept inside the file it authenticates is only as
strong as the editor's laziness: whoever edits the rows can recompute it. The
derived block is a pure function of two things the ledger already pins - the
declared globs and the bytes of the screen files - so the honest answer can be
recomputed from the source at check time and compared. There is no secret to
hold and nothing for an editor to recalculate. To make a forged ledger verify
you must edit the screen files until they genuinely produce those rows, and at
that point the ledger is telling the truth and the proof is reading the truth.

The recorded `derived_digest` is kept as well, but as a diagnostic rather than
the guarantee: it separates "someone edited the rows and forgot the digest"
from "someone edited the rows and updated the digest", and both are refused.

Refusal is loud and total. This module never silently regenerates a ledger it
found tampered: quietly repairing your own input is how tampering stays
invisible, and the whole point is that it becomes visible.

This module reads component and element NAMES and import PATHS. It reads no
colour, length, radius, font, duration or easing curve, so nothing it writes is
a realisation under VDS S-2(4).
"""

from __future__ import annotations

import re
from pathlib import Path

from . import yamlish
from .core import Project, VdsError, canonical_json, digest_of, now_iso, sha256_file

# Bumped 1 -> 2 when `derived_digest` became a required field and the derived
# rows became re-derived rather than trusted. A version 1 ledger carries no
# derived_digest, so it cannot be authenticated, so it is refused outright
# rather than accepted under the weaker check it was written for.
LEDGER_SCHEMA_VERSION = 2
GENERATOR_COMMAND = "vds.py ledger screens"

# The keys of the ledger that are DERIVED: computed from the screen files
# rather than declared by a human. These are the rows every proof reads, and
# they are exactly what `check_authentic` re-derives.
DERIVED_KEYS = ("screens",)

REGENERATE_HINT = "  Regenerate with: vds.py ledger screens"

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


def derive_screens(project: Project, files: list[Path]) -> list[dict]:
    """The derived rows, computed from the screen files and nothing else.

    Deterministic by construction: `files` is sorted, `parse_references` returns
    its rows sorted, and no clock, environment or ledger value is consulted.
    That determinism is what makes re-derivation usable as an authenticity
    test rather than merely as a regeneration.
    """
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
    return screens


def derived_block(ledger: dict) -> dict:
    """The derived part of a ledger, isolated from its declared part.

    `generated_at` and `generated_by` are deliberately excluded: a re-run at a
    different second must not read as tampering.
    """
    return {key: ledger.get(key) for key in DERIVED_KEYS}


def derived_digest_of(screens: list[dict]) -> str:
    return digest_of({"screens": screens})


def generate(project: Project) -> dict:
    """Build the screens ledger from the declared surface."""
    files = screen_files(project)
    screens = derive_screens(project, files)
    return {
        "schema_version": LEDGER_SCHEMA_VERSION,
        "generated_at": now_iso(),
        "generated_by": GENERATOR_COMMAND,
        "source_globs": list(project.surface.get("screen_globs") or []),
        "source_digest": source_digest(project, files),
        # Covers the derived rows below. See the module docstring: this is a
        # diagnostic, not the guarantee. `check_authentic` re-derives.
        "derived_digest": derived_digest_of(screens),
        "screens": screens,
    }


def write(project: Project) -> tuple[Path, dict]:
    ledger = generate(project)
    path = project.screens_ledger_path
    path.parent.mkdir(parents=True, exist_ok=True)
    yamlish.dump(ledger, path)
    return path, ledger


def _row_key(route: str, ref: object) -> str:
    """A stable, readable identity for one derived reference row."""
    if not isinstance(ref, dict):
        return f"{route}: <malformed row {ref!r}>"
    return (
        f"{route}:{ref.get('line')} <{ref.get('name')}> "
        f"kind={ref.get('kind')} importPath={ref.get('importPath')!r} "
        f"count={ref.get('count')}"
    )


def _rows_of(screens: object) -> dict[str, str]:
    """Flatten a screens block to {row identity: canonical row}, for diffing."""
    rows: dict[str, str] = {}
    if not isinstance(screens, list):
        return rows
    for screen in screens:
        if not isinstance(screen, dict):
            rows[f"<malformed screen {screen!r}>"] = canonical_json(screen)
            continue
        route = str(screen.get("route", "<unknown route>"))
        rows[f"{route} :: <screen digest>"] = canonical_json(screen.get("digest"))
        references = screen.get("references")
        if not isinstance(references, list):
            rows[f"{route} :: <references>"] = canonical_json(references)
            continue
        for index, ref in enumerate(references):
            rows[f"[{index:04d}] " + _row_key(route, ref)] = canonical_json(ref)
    return rows


def _derived_drift(recorded: object, derived: list[dict]) -> list[str]:
    """Name every derived row that differs, so the tampering is readable.

    Missing rows are listed first and deliberately: a row DELETED from the
    ledger is the attack that turns a violation into a pass, and it is the one
    a reader most needs to see named.
    """
    have = _rows_of(recorded)
    want = _rows_of(derived)
    detail: list[str] = []
    missing = sorted(set(want) - set(have))
    extra = sorted(set(have) - set(want))
    altered = sorted(k for k in set(have) & set(want) if have[k] != want[k])
    if missing:
        detail.append(
            f"  {len(missing)} derived rows the source produces but the ledger does NOT "
            "contain (rows deleted by hand):"
        )
        detail.extend(f"    - {k}" for k in missing)
    if extra:
        detail.append(
            f"  {len(extra)} derived rows the ledger contains but the source does NOT "
            "produce (rows invented by hand):"
        )
        detail.extend(f"    + {k}" for k in extra)
    if altered:
        detail.append(f"  {len(altered)} derived rows whose contents were altered:")
        for k in altered:
            detail.append(f"    ~ {k}")
            detail.append(f"        ledger: {have[k]}")
            detail.append(f"        source: {want[k]}")
    return detail


def check_authentic(project: Project, ledger: dict) -> None:
    """VDS S-4(2). Refuse to proceed on a ledger that is stale OR hand-edited.

    Four limbs, in this order, because the order determines which diagnosis a
    reader is given and the accurate diagnosis is the useful one:

      1. the declared globs still match the config
      2. the recorded source_digest still matches the live screen files
      3. the recorded derived_digest still matches the ledger's own rows
      4. the ledger's rows are the rows the live source actually produces

    Limb 4 subsumes limb 3 and is the one that cannot be defeated by an editor,
    since it consults the source rather than anything the editor can rewrite.
    Limb 3 is kept because it distinguishes a careless hand-edit from a careful
    one, and naming which you are looking at is worth five lines of code.

    This function NEVER regenerates. It raises.
    """
    recorded_globs = list(ledger.get("source_globs") or [])
    configured_globs = list(project.surface.get("screen_globs") or [])
    if recorded_globs != configured_globs:
        raise VdsError(
            "the screens ledger is STALE: [surface] screen_globs changed since it was "
            f"generated.\n  ledger: {recorded_globs}\n  config: {configured_globs}\n"
            + REGENERATE_HINT
        )

    files = screen_files(project)
    current = source_digest(project, files)
    if current != ledger.get("source_digest"):
        recorded = {
            s.get("route"): s.get("digest")
            for s in (ledger.get("screens") or [])
            if isinstance(s, dict)
        }
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
            + "\n"
            + REGENERATE_HINT
        )

    # Limb 3. The ledger's own statement about its own rows.
    recorded_derived = ledger.get("derived_digest")
    actual_derived = derived_digest_of(ledger.get("screens") or [])
    if not isinstance(recorded_derived, str) or not recorded_derived.startswith("sha256:"):
        raise VdsError(
            "the screens ledger is TAMPERED: it carries no usable `derived_digest`, so "
            "it does not authenticate its own derived rows (VDS S-4(2)).\n"
            f"  found: {recorded_derived!r}\n"
            "  A generated ledger always carries one. Its absence means the file was "
            "written by something other than the generator.\n"
            + REGENERATE_HINT
        )
    if recorded_derived != actual_derived:
        raise VdsError(
            "the screens ledger is TAMPERED: its derived rows do not match its own "
            "recorded `derived_digest`, so the rows were edited after generation "
            "(VDS S-4(2)).\n"
            f"  derived_digest recorded: {recorded_derived}\n"
            f"  derived_digest of rows:  {actual_derived}\n"
            "  Nothing derived from these rows can be trusted, and this proof will not "
            "run on them. VDS does not silently regenerate a tampered ledger, because a "
            "tool that repairs its own input hides the edit it repaired.\n"
            + REGENERATE_HINT
        )

    # Limb 4. The source is the authority, not the file's account of itself.
    derived = derive_screens(project, files)
    if derived_digest_of(derived) != actual_derived:
        detail = _derived_drift(ledger.get("screens"), derived)
        raise VdsError(
            "the screens ledger is TAMPERED: its derived rows are not the rows the "
            "declared surface actually produces, and the screen files themselves are "
            "unchanged since generation, so the ledger was hand-edited (VDS S-4(2)).\n"
            + "\n".join(detail or ["  the derived block differs and no per-row cause was found"])
            + "\n  Nothing derived from these rows can be trusted, and this proof will not "
            "run on them. VDS does not silently regenerate a tampered ledger, because a "
            "tool that repairs its own input hides the edit it repaired.\n"
            + REGENERATE_HINT
        )


# The old name. `check_authentic` gained the two tampering limbs on 2026-07-25;
# anything still calling the old name gets the stronger check, never the weaker.
check_fresh = check_authentic


def load_authentic(project: Project) -> dict:
    """Load the screens ledger and refuse it if it is stale OR hand-edited.

    This is the ONE door onto the ledger. Every proof that reads a derived row
    comes through here, so there is no path on which a row is read without
    having been re-derived from the source first.
    """
    path = project.screens_ledger_path
    if not path.is_file():
        raise VdsError(
            f"{project.rel(path)} is absent. The declared surface is a generated ledger "
            "(VDS S-4(2)). Run: vds.py ledger screens"
        )
    try:
        ledger = yamlish.load(path)
    except Exception as exc:
        raise VdsError(
            f"{project.rel(path)}: unreadable screens ledger: {exc}\n"
            "  A ledger that will not parse is refused, never partially read "
            "(VDS S-11(2))."
        ) from exc
    if not isinstance(ledger, dict) or "screens" not in ledger:
        raise VdsError(f"{project.rel(path)}: not a screens ledger")
    version = ledger.get("schema_version")
    if version != LEDGER_SCHEMA_VERSION:
        if isinstance(version, int) and version < LEDGER_SCHEMA_VERSION:
            raise VdsError(
                f"{project.rel(path)}: ledger schema_version {version!r} predates this "
                f"tool ({LEDGER_SCHEMA_VERSION}). Version {LEDGER_SCHEMA_VERSION} added "
                "`derived_digest` and re-derivation of the ledger's rows; a version "
                f"{version} ledger carries neither and so cannot be authenticated. "
                "Refusing rather than reading rows it cannot check.\n"
                + REGENERATE_HINT
            )
        raise VdsError(
            f"{project.rel(path)}: ledger schema_version {version!r} exceeds what this "
            f"tool understands ({LEDGER_SCHEMA_VERSION}). Refusing rather than skipping "
            "what it cannot parse (VDS S-11(2))."
        )
    check_authentic(project, ledger)
    return ledger


# The old name, kept so no caller can accidentally reach a weaker load path.
load_fresh = load_authentic
