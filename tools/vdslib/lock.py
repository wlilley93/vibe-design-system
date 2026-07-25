"""`.vds/enforcement.lock`: which checks are wired, by digest.

VDS S-8(1). The lock is held OUTSIDE the scripts it witnesses, so a weakening
edit bumps a digest and trips a loud blocking finding rather than passing under
its own possibly weakened logic. Weakening a gate becomes a visible diff instead
of a silent deletion.

Everything an entry ASSERTS about the tree is re-checked against the tree on
every `lock verify`, never once at write time and never on the entry's own word:

  - the invoker named in `invoked_by[].ref` is OPENED, and must plausibly invoke
    the pinned script. A declared invoker that cannot be read is not an invoker.
    This is [2026] VJS-CC-OPBOX 3 D2 ("a gate nothing invokes is not
    enforcement") applied to the lock itself, which previously reproduced that
    exact defect inside the tool built to prevent it.
  - the failing-direction test named in `failing_direction_test` is OPENED, and
    must still contain the named test. Checking it only at `lock add` meant
    deleting the test afterwards still verified clean.
  - the kinds in `proves` must be kinds this tooling actually implements. A lock
    entry claiming an unimplemented kind records a coverage the runner cannot
    deliver, which is a lie the lock exists to prevent, not to hold.

VDS S-8(5), stated plainly and not glossed: the lock CANNOT bind an author with
full write access who edits a gate and re-locks it in the same act. The backstops
for that residue are non-machine. The lock makes the act visible in a diff; it
does not prevent it. No VDS document may claim otherwise. Nor do the checks above
prove that an invoker RUNS: they prove it exists, is readable and names what it
claims to run. A workflow that is never triggered still passes them.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

from . import yamlish
from .core import (
    IMPLEMENTED_PROOF_KINDS,
    PROOF_KINDS,
    VDS_HOME,
    Project,
    VdsError,
    now_iso,
    sha256_file,
)

LOCK_SCHEMA_VERSION = 1
LOCK_NAME = "enforcement.lock"

# A lock entry names a file either in the PROJECT (repository-relative) or in the
# VDS INSTALL (`vds:`-prefixed). The second form exists because in a real
# adoption the proof scripts live in the install and NOT in the adopting
# repository, and a surface that cannot be named is a surface that cannot be
# pinned. Globbing only `<project>/tools/proofs/*.py` made the unpinned-proof
# warning unable to fire in every adoption but the one that authored VDS.
VDS_PATH_PREFIX = "vds:"
PROOFS_SUBDIR = ("tools", "proofs")
PROOFS_REL = "/".join(PROOFS_SUBDIR)

# Reading an invoker is a check, not an import. Bound it.
_MAX_READ = 512 * 1024
_MAX_CHASED = 12

_ANCHOR = re.compile(r"^(?P<path>.*?):(?P<line>\d+)$")
_PATHISH = re.compile(
    r"[A-Za-z0-9_.][A-Za-z0-9_./\-]*\.(?:py|sh|bash|js|cjs|mjs|ts|tsx|yml|yaml|json|toml)"
)
_SCRIPT_RUN = re.compile(
    r"\b(?:npm|pnpm|yarn|bun)\s+(?:run\s+(?:--\S+\s+)*)?([A-Za-z0-9_][A-Za-z0-9_:.\-]*)"
)
_UNSAFE = re.compile(r"(^|/)\.\.(/|$)")


def lock_path(project: Project) -> Path:
    return project.vds_dir / LOCK_NAME


# ------------------------------------------------------------------ path naming


def is_safe_ref(rel_path: str) -> bool:
    """A lock path is repository-relative or install-relative, never absolute and
    never an escape upwards. A path that can point anywhere pins nothing."""
    text = str(rel_path)
    if text.startswith(VDS_PATH_PREFIX):
        text = text[len(VDS_PATH_PREFIX) :]
    if not text or text.startswith("/"):
        return False
    return not _UNSAFE.search(text)


def resolve_path(project: Project, rel_path: str) -> Path:
    """Resolve a lock path against the project, or against the VDS install when
    it carries the `vds:` scheme."""
    text = str(rel_path)
    if text.startswith(VDS_PATH_PREFIX):
        return VDS_HOME / text[len(VDS_PATH_PREFIX) :]
    return project.root / text


def name_for(project: Project, path: Path) -> str:
    """The way a lock entry should NAME this file: project-relative when it is in
    the project, `vds:`-prefixed when it lives in the install instead."""
    resolved = Path(path).resolve()
    try:
        return str(resolved.relative_to(project.root.resolve()))
    except ValueError:
        pass
    try:
        return VDS_PATH_PREFIX + str(resolved.relative_to(VDS_HOME.resolve()))
    except ValueError:
        return str(resolved)


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
    target = resolve_path(project, rel_path)
    if not target.is_file():
        return None
    return sha256_file(target)


# ------------------------------------------------------------- reading invokers


def read_text(path: Path) -> str | None:
    try:
        if not path.is_file():
            return None
        with open(path, "rb") as fh:
            data = fh.read(_MAX_READ)
    except OSError:
        return None
    return data.decode("utf-8", "replace")


def split_ref(ref: str) -> tuple[str, str | None, int | None]:
    """`.githooks/pre-push:106` -> (path, None, 106).
    `.github/workflows/ci.yml#design-gates` -> (path, "design-gates", None)."""
    text = str(ref).strip()
    fragment: str | None = None
    if "#" in text:
        text, fragment = text.split("#", 1)
        fragment = fragment.strip() or None
    line: int | None = None
    match = _ANCHOR.fullmatch(text)
    if match:
        text = match.group("path")
        line = int(match.group("line"))
    return text.strip(), fragment, line


def _needles(entry_path: str) -> list[str]:
    """Literal strings whose presence in an invoker is evidence that the invoker
    names this script."""
    bare = str(entry_path)
    if bare.startswith(VDS_PATH_PREFIX):
        bare = bare[len(VDS_PATH_PREFIX) :]
    out = [bare]
    base = bare.rsplit("/", 1)[-1]
    if len(base) > 3:
        out.append(base)
    return out


def _cli_pattern(entry_path: str) -> re.Pattern | None:
    """A VDS proof script is legitimately invoked through the front door, as
    `vds.py proof composition` or `vds.py proof --all`, which never spells the
    script's own path. Recognise that form rather than calling it uninvoked."""
    bare = str(entry_path)
    if bare.startswith(VDS_PATH_PREFIX):
        bare = bare[len(VDS_PATH_PREFIX) :]
    parts = bare.split("/")
    if len(parts) < 2 or parts[-2] != "proofs" or not parts[-1].endswith(".py"):
        return None
    kind = parts[-1][: -len(".py")]
    if kind not in PROOF_KINDS:
        return None
    return re.compile(
        r"\bvds(?:\.py)?\b[^\n]*\bproof\b[^\n]*(?:--all|\b" + re.escape(kind) + r"\b)"
    )


def invocation_texts(project: Project, invoker: Path) -> list[tuple[str, str]]:
    """The invoker's own text, plus ONE level of the indirection real invokers
    use: a file it names, and the body of an npm/pnpm/yarn script it runs. Two
    levels are not chased, so a deeper indirection must be declared by pointing
    `ref` at the file that actually names the script."""
    text = read_text(invoker)
    if text is None:
        return []
    out: list[tuple[str, str]] = [(project.rel(invoker), text)]
    root = project.root
    seen = {invoker.resolve()}

    for token in dict.fromkeys(_PATHISH.findall(text)):
        if len(out) >= _MAX_CHASED:
            break
        if not is_safe_ref(token):
            continue
        candidate = root / token
        if not candidate.is_file() or candidate.resolve() in seen:
            continue
        body = read_text(candidate)
        if body is None:
            continue
        seen.add(candidate.resolve())
        out.append((token, body))

    package = read_text(root / "package.json")
    if package:
        try:
            scripts = json.loads(package).get("scripts", {})
        except (ValueError, AttributeError):
            scripts = {}
        if isinstance(scripts, dict):
            for name in dict.fromkeys(_SCRIPT_RUN.findall(text)):
                if len(out) >= _MAX_CHASED:
                    break
                body = scripts.get(name)
                if isinstance(body, str):
                    out.append((f"package.json#scripts.{name}", body))
    return out


# ---------------------------------------------------------------- entry checks


def _check_proves(entry: dict) -> tuple[list[str], list[str]]:
    findings: list[str] = []
    notes: list[str] = []
    path = str(entry.get("path", ""))
    kinds = list(entry.get("proves", []))
    unimplemented = [k for k in kinds if k not in IMPLEMENTED_PROOF_KINDS]
    if unimplemented:
        findings.append(
            f"UNIMPLEMENTED KIND  {path}\n"
            f"           claims: {', '.join(unimplemented)}\n"
            f"           this tooling implements only: "
            f"{', '.join(IMPLEMENTED_PROOF_KINDS)}.\n"
            f"           A kind in the closed registry that no runner implements produces "
            f"no proof record, so pinning a script against it records a coverage nothing "
            f"delivers (VDS S-7(5), and the D2 defect of [2026] VJS-CC-OPBOX 3). Either "
            f"implement the kind, or drop it from `proves` and record the omission as a "
            f"comment in the lock so it stays visible."
        )
    for kind in kinds:
        if kind not in IMPLEMENTED_PROOF_KINDS:
            continue
        canonical = f"{PROOFS_REL}/{kind}.py"
        if not str(path).endswith(canonical):
            notes.append(
                f"ADDITIONAL {path} is pinned as proving {kind}, but `vds.py proof {kind}` "
                f"runs {VDS_PATH_PREFIX}{canonical}, not this file. This entry witnesses an "
                f"EXTRA project gate for that kind; no VDS proof record comes from it."
            )
    return findings, notes


def _check_invocations(project: Project, entry: dict) -> tuple[list[str], list[str]]:
    findings: list[str] = []
    notes: list[str] = []
    path = str(entry.get("path", ""))
    needles = _needles(path)
    cli = _cli_pattern(path)

    for invocation in entry.get("invoked_by", []):
        surface = str(invocation.get("surface", "?"))
        ref = str(invocation.get("ref", ""))
        ref_path, fragment, line = split_ref(ref)
        if not ref_path or not is_safe_ref(ref_path):
            findings.append(
                f"BAD INVOKER  {path}\n"
                f"           declares: {surface} = {ref!r}\n"
                f"           that ref names no readable repository-relative file."
            )
            continue
        invoker = project.root / ref_path
        if not invoker.is_file():
            findings.append(
                f"NO INVOKER  {path}\n"
                f"           declares: {surface} = {ref}\n"
                f"           but {ref_path} does not exist. A declared invoker that cannot "
                f"be opened is not an invoker, and an uninvoked gate is not enforcement "
                f"(VDS S-7(2)(3); [2026] VJS-CC-OPBOX 3 D2)."
            )
            continue

        texts = invocation_texts(project, invoker)
        joined = "\n".join(body for _, body in texts)
        where = next(
            (label for label, body in texts if any(n in body for n in needles)),
            None,
        )
        if where is None and cli is not None and cli.search(joined):
            where = next(
                (label for label, body in texts if cli.search(body)),
                texts[0][0] if texts else ref_path,
            )
        if where is None:
            findings.append(
                f"INVOKER DOES NOT INVOKE  {path}\n"
                f"           declares: {surface} = {ref}\n"
                f"           {ref_path} exists but neither it, nor any file or package "
                f"script it names, mentions {needles[0]}.\n"
                f"           Naming an invoker is not being invoked by it. Point `ref` at "
                f"the file that actually runs this script (VDS S-7(2)(3))."
            )
            continue
        if where != ref_path:
            notes.append(
                f"INDIRECT {path} is invoked from {ref_path} by way of {where}, one level "
                "of indirection. Only one level is chased."
            )

        own = next((body for label, body in texts if label == ref_path), "")
        if fragment and fragment not in own:
            findings.append(
                f"NO SUCH JOB  {path}\n"
                f"           declares: {surface} = {ref}\n"
                f"           {ref_path} contains no {fragment!r}. A fragment naming a job "
                f"that is not in the file is a declaration about nothing."
            )
        if line is not None:
            lines = own.splitlines()
            if line > len(lines):
                notes.append(
                    f"ANCHOR   {path}: {ref} points past the end of {ref_path} "
                    f"({len(lines)} lines). The invocation was found; only the line "
                    "anchor is stale."
                )
            elif not any(n in lines[line - 1] for n in needles):
                hits = [
                    str(i)
                    for i, text in enumerate(lines, start=1)
                    if any(n in text for n in needles)
                ]
                notes.append(
                    f"ANCHOR   {path}: {ref} does not name it on line {line}. "
                    + (
                        f"It is named on line(s) {', '.join(hits[:6])}."
                        if hits
                        else "It is reached indirectly."
                    )
                )
    return findings, notes


def _check_failing_direction_test(project: Project, entry: dict) -> list[str]:
    path = str(entry.get("path", ""))
    test = entry.get("failing_direction_test") or {}
    test_path = str(test.get("path", ""))
    test_name = str(test.get("test_name", ""))
    if not test_path or not is_safe_ref(test_path):
        return [
            f"NO TEST  {path}\n"
            f"           failing_direction_test.path is {test_path!r}, which names no file."
        ]
    target = resolve_path(project, test_path)
    if not target.is_file():
        return [
            f"TEST GONE  {path}\n"
            f"           names: {test_path}::{test_name}\n"
            f"           that file does not exist. A script whose failing direction is "
            f"asserted nowhere has proven only its happy path (VDS S-7(2)(2)). This is "
            f"checked on every verify, not once at pin time, because deleting the test "
            f"afterwards used to verify clean."
        ]
    body = read_text(target) or ""
    if test_name and test_name not in body:
        return [
            f"TEST RENAMED OR REMOVED  {path}\n"
            f"           names: {test_path}::{test_name}\n"
            f"           {test_path} exists but contains no such test name. The pinned "
            f"failing-direction assertion is gone (VDS S-7(2)(2))."
        ]
    return []


def check_entry(project: Project, entry: dict) -> tuple[list[str], list[str]]:
    """Everything an entry asserts about the tree, except its digest.

    Used by `lock add` as a refusal and by `lock verify` as the wall, from one
    implementation, so the door and the wall cannot drift apart.
    """
    proves_findings, proves_notes = _check_proves(entry)
    invoke_findings, invoke_notes = _check_invocations(project, entry)
    test_findings = _check_failing_direction_test(project, entry)
    return (
        proves_findings + invoke_findings + test_findings,
        proves_notes + invoke_notes,
    )


# --------------------------------------------------------------- proof surface


def proof_surface(project: Project) -> list[Path]:
    """Every proof script that a lock should witness.

    In a real adoption the proof scripts live in the VDS INSTALL, not in the
    adopting repository: `<project>/tools/proofs` does not exist there, so
    globbing it alone reported an empty surface and the unpinned warning could
    never fire. Resolve the install surface first, then any project-local one.
    """
    out: list[Path] = []
    install = VDS_HOME.joinpath(*PROOFS_SUBDIR)
    local = project.root.joinpath(*PROOFS_SUBDIR)
    seen: set[Path] = set()
    for directory in (install, local):
        if not directory.is_dir() or directory.resolve() in seen:
            continue
        seen.add(directory.resolve())
        for script in sorted(directory.glob("*.py")):
            if script.name == "__init__.py":
                continue
            out.append(script)
    return out


# ---------------------------------------------------------------------- verify


def verify(project: Project) -> tuple[list[str], list[str]]:
    """Recompute every pinned digest, and re-check every claim the entry makes.

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
    pinned: set[Path] = set()

    for entry in data.get("entries", []):
        rel_path = str(entry.get("path", ""))
        if not is_safe_ref(rel_path):
            findings.append(
                f"BAD PATH  {rel_path!r} is not a repository-relative or `{VDS_PATH_PREFIX}`"
                "-relative path. A path that can point anywhere pins nothing."
            )
            continue
        pinned.add(resolve_path(project, rel_path).resolve())
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

        entry_findings, entry_notes = check_entry(project, entry)
        findings.extend(entry_findings)
        notes.extend(entry_notes)

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

    for script in proof_surface(project):
        if script.resolve() in pinned:
            continue
        notes.append(
            f"UNPINNED {name_for(project, script)} is a proof script that no lock entry "
            "witnesses. A warrant may not cite a proof whose script is absent from a "
            "present lock (VDS S-8(3))."
        )

    return findings, notes
