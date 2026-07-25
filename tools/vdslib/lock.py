"""`.vds/enforcement.lock`: which checks are wired, by digest.

VDS S-8(1). The lock is held OUTSIDE the scripts it witnesses, so a weakening
edit bumps a digest and trips a loud blocking finding rather than passing under
its own possibly weakened logic. Weakening a gate becomes a visible diff instead
of a silent deletion.

VDS S-8(5), stated plainly and not glossed: the lock CANNOT bind an author with
full write access who edits a gate and re-locks it in the same act. The backstops
for that residue are non-machine. The lock makes the act visible in a diff; it
does not prevent it. No VDS document may claim otherwise.
"""

from __future__ import annotations

from pathlib import Path

from . import yamlish
from .core import Project, VdsError, now_iso, sha256_file

LOCK_SCHEMA_VERSION = 1
LOCK_NAME = "enforcement.lock"


def lock_path(project: Project) -> Path:
    return project.vds_dir / LOCK_NAME


def read(project: Project) -> dict | None:
    """Return the lock, or None. VDS S-8(3): the lock is opt-in, and a repository
    with no lock file produces no drift finding rather than being broken."""
    path = lock_path(project)
    if not path.is_file():
        return None
    data = yamlish.load(path)
    if not isinstance(data, dict) or "entries" not in data:
        raise VdsError(f"{project.rel(path)}: not an enforcement lock")
    if data.get("schema_version") != LOCK_SCHEMA_VERSION:
        raise VdsError(
            f"{project.rel(path)}: lock schema_version {data.get('schema_version')!r} "
            f"exceeds what this tool understands ({LOCK_SCHEMA_VERSION}). Refusing rather "
            "than skipping what it cannot parse (VDS S-11(2))."
        )
    return data


def write(project: Project, entries: list[dict]) -> Path:
    for entry in entries:
        project.validate_artefact(
            "enforcement-lock-entry", entry, f"{LOCK_NAME} entry for {entry.get('path')}"
        )
    seen: set[str] = set()
    for entry in entries:
        if entry["path"] in seen:
            raise VdsError(f"{LOCK_NAME}: duplicate entry for path {entry['path']!r}")
        seen.add(entry["path"])
    document = {
        "schema_version": LOCK_SCHEMA_VERSION,
        "generated_at": now_iso(),
        "entries": sorted(entries, key=lambda e: e["path"]),
    }
    path = lock_path(project)
    path.parent.mkdir(parents=True, exist_ok=True)
    yamlish.dump(document, path)
    return path


def current_digest(project: Project, rel_path: str) -> str | None:
    target = project.root / rel_path
    if not target.is_file():
        return None
    return sha256_file(target)


def verify(project: Project) -> tuple[list[str], list[str]]:
    """Recompute every pinned digest.

    Returns (findings, notes). A non-empty findings list is fatal (VDS S-8(4)).
    """
    data = read(project)
    if data is None:
        return [], [
            f"no {LOCK_NAME} present. The lock is opt-in (VDS S-8(3)), so this is quiet "
            "rather than broken, and no warrant may cite a proof whose script is absent "
            "from a present lock."
        ]

    findings: list[str] = []
    notes: list[str] = []
    pinned: set[str] = set()

    for entry in data.get("entries", []):
        rel_path = str(entry.get("path", ""))
        pinned.add(rel_path)
        expected = str(entry.get("digest", ""))
        actual = current_digest(project, rel_path)
        if actual is None:
            findings.append(
                f"MISSING  {rel_path}\n"
                f"           pinned: {expected}\n"
                f"           actual: the file does not exist. A pinned gate that is gone "
                f"is a deleted gate, not an absent finding."
            )
            continue
        if actual != expected:
            findings.append(
                f"DRIFT    {rel_path}\n"
                f"           pinned: {expected}\n"
                f"           actual: {actual}\n"
                f"           proves: {', '.join(entry.get('proves', []))}\n"
                f"           re-pin only after a recorded gate change, and self-file the "
                f"rationale (VDS S-8(4), S-12(3))."
            )
            continue
        surfaces = entry.get("invoked_by", [])
        blocking_ci = [
            i
            for i in surfaces
            if i.get("surface") == "ci_workflow" and i.get("blocking", True)
        ]
        if not blocking_ci:
            notes.append(
                f"INTERIM  {rel_path} is invoked by "
                f"{', '.join(sorted({str(i.get('surface')) for i in surfaces}))} and by no "
                "blocking ci_workflow. A hook is not CI: `git commit --no-verify` bypasses "
                "it, so this satisfies VDS S-7(2)(3) only as an interim state, and the "
                "interim is recorded here (VDS S-7(3))."
            )

    for script in sorted((project.root / "tools" / "proofs").glob("*.py")):
        rel = project.rel(script)
        if script.name == "__init__.py" or rel in pinned:
            continue
        notes.append(
            f"UNPINNED {rel} is a proof script that no lock entry witnesses. A warrant may "
            "not cite a proof whose script is absent from a present lock (VDS S-8(3))."
        )

    return findings, notes
