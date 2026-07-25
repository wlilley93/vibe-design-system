"""The fixture harness every VDS test builds on.

VDS S-7(2)(2) is the reason this directory exists at all: a check is a proof only
if "a named test seeds a violation against a fixture and asserts the non-zero
exit". Until 2026-07-25 `ls -A tools/tests` returned 0, so by VDS's own statute
none of the three implemented proofs was a proof and none could lawfully be named
as evidence. That is the identical defect [2026] VJS-CC-OPBOX 3 found in the
Opbox token gate, reproduced inside the tool built to prevent it.

Three rules hold for every test in this directory, and this module is what makes
them cheap enough that nobody is tempted to skip them.

  OWN FIXTURE     every test builds its own project in its own temp directory
                  and tears it down, so no test can observe another's leftovers
                  and no ordering is load-bearing.
  NOTHING REAL    no test may write to an adopting repository's `.vds/` or to the
                  VDS install. `_assert_install_unchanged` re-digests the install
                  after every single test, so a test that crashes halfway cannot
                  leave the tool it was testing broken, and the failure names the
                  file. `test_isolation.py` adds the structural limb: no test
                  source may name an absolute path outside a temp directory.
  REAL SUBPROCESS every assertion runs the real script through a real subprocess
                  and reads the real exit code. Importing the module and calling
                  `run()` would test the function; the contract a caller reads is
                  the exit code, so the exit code is what is asserted.

Stdlib only. VDS has no install step and its tests must not introduce one.
"""

from __future__ import annotations

import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

TESTS_DIR = Path(__file__).resolve().parent
TOOLS_DIR = TESTS_DIR.parent
VDS_HOME = TOOLS_DIR.parent
VDS_PY = TOOLS_DIR / "vds.py"
SCHEMA_DIR = VDS_HOME / "schema"

PROOF_SCRIPT = {
    "register_completeness": TOOLS_DIR / "proofs" / "register_completeness.py",
    "composition": TOOLS_DIR / "proofs" / "composition.py",
    "states": TOOLS_DIR / "proofs" / "states.py",
}

sys.path.insert(0, str(TOOLS_DIR))

from vdslib import scan, yamlish  # noqa: E402
from vdslib.core import sha256_file  # noqa: E402

# The exit codes are the contract (vdslib/core.py). A test that asserts on text
# and not on these is asserting about a message, not about a gate.
EXIT_PASSED = 0
EXIT_VIOLATION = 1
EXIT_PRECONDITION = 2
EXIT_VACUOUS = 3

CLI_REFUSAL = "VDS REFUSED, and did nothing:"
PROOF_REFUSAL = "PRECONDITION FAILED, this proof did not run and proves nothing:"
REGENERATE_HINT = "Regenerate with: vds.py ledger screens"

# The trees a test may never modify. The proof scripts and the schemas ARE the
# subject under test, so a suite that could rewrite them would be marking its own
# homework with a pencil.
PROTECTED_TREES = (TOOLS_DIR, SCHEMA_DIR)


def install_manifest() -> dict[str, str]:
    """Digest every file of the VDS install that a test must not touch."""
    out: dict[str, str] = {}
    for tree in PROTECTED_TREES:
        if not tree.is_dir():
            continue
        for path in sorted(tree.rglob("*")):
            if not path.is_file() or "__pycache__" in path.parts:
                continue
            out[str(path)] = sha256_file(path)
    return out


def run(*argv: str, cwd: Path | None = None) -> subprocess.CompletedProcess:
    """Run a VDS entry point exactly as a caller would, and keep its exit code."""
    return subprocess.run(
        [sys.executable, *[str(a) for a in argv]],
        capture_output=True,
        text=True,
        check=False,
        cwd=str(cwd) if cwd else None,
    )


class VdsProjectCase(unittest.TestCase):
    """A real, initialised VDS project in a throwaway directory.

    Subclasses declare their surface with SCREENS and COMPONENTS and get a tree
    that `vds.py init` has scaffolded and `vds.py ledger screens` has inventoried.
    Nothing here fakes a VDS artefact: every file the tests read was written by
    the tool under test, which is the only way a fixture can catch the tool
    changing its own output format underneath the assertions.
    """

    # route (relative path) -> TSX source
    SCREENS: dict[str, str] = {}
    # component module basenames created under src/components/ui/
    COMPONENTS: tuple[str, ...] = ()
    # generate the ledger during setUp. A test about a missing ledger turns it off.
    GENERATE_LEDGER = True

    def setUp(self) -> None:
        self._install_before = install_manifest()
        self.root = Path(tempfile.mkdtemp(prefix="vds-test-")).resolve()
        # Belt and braces: everything below deletes trees, so prove first that the
        # tree about to be created is inside the system temp directory.
        self.assertTrue(
            str(self.root).startswith(str(Path(tempfile.gettempdir()).resolve())),
            f"fixture root {self.root} is not under the temp directory; refusing to run",
        )
        self.addCleanup(self._assert_install_unchanged)
        self.addCleanup(shutil.rmtree, self.root, True)

        (self.root / "src" / "components" / "ui").mkdir(parents=True)
        for module in self.COMPONENTS:
            (self.root / "src" / "components" / "ui" / f"{module}.tsx").write_text(
                f"export function {module.title().replace('-', '')}() {{ return null }}\n",
                encoding="utf-8",
            )
        for route, source in self.SCREENS.items():
            self.write_screen(route, source)

        result = self.vds("init", "--jurisdiction", "fx", "--repo-code", "FX")
        self.assertEqual(result.returncode, EXIT_PASSED, result.stderr)
        if self.GENERATE_LEDGER:
            self.regenerate()

    def _assert_install_unchanged(self) -> None:
        after = install_manifest()
        before = self._install_before
        changed = sorted(
            path for path in set(before) | set(after) if before.get(path) != after.get(path)
        )
        self.assertEqual(
            changed,
            [],
            "a test modified the VDS install itself. A suite that can rewrite the "
            "scripts it tests proves nothing about them.",
        )

    # -- paths ---------------------------------------------------------------

    @property
    def ledger_path(self) -> Path:
        return self.root / ".vds" / "ledgers" / "screens.yaml"

    @property
    def proofs_dir(self) -> Path:
        return self.root / ".vds" / "proofs"

    @property
    def register_dir(self) -> Path:
        return self.root / ".vds" / "register"

    @property
    def warrants_dir(self) -> Path:
        return self.root / ".vds" / "warrants"

    def proof_records(self) -> list[Path]:
        return sorted(self.proofs_dir.glob("*.yaml"))

    def warrant_files(self) -> list[Path]:
        return sorted(self.warrants_dir.glob("*.yaml"))

    # -- driving the tool ----------------------------------------------------

    def vds(self, *args: str) -> subprocess.CompletedProcess:
        return run(VDS_PY, "--root", self.root, *args, cwd=self.root)

    def proof(self, kind: str, *extra: str) -> subprocess.CompletedProcess:
        return run(PROOF_SCRIPT[kind], "--root", self.root, *extra, cwd=self.root)

    def write_screen(self, route: str, source: str) -> Path:
        path = self.root / route
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(source, encoding="utf-8")
        return path

    def regenerate(self) -> None:
        result = self.vds("ledger", "screens")
        self.assertEqual(result.returncode, EXIT_PASSED, result.stderr)

    def register(
        self,
        name: str,
        module: str,
        status: str = "registered",
        require: str = "default",
        drawn: str = "default",
        built: str | None = None,
    ) -> None:
        args = [
            "register", "add",
            "--name", name,
            "--status", status,
            "--import-path", f"@/components/ui/{module}",
            "--source-file", f"src/components/ui/{module}.tsx",
            "--export-name", name,
            "--figma", "FXKEY#1:2",
            "--require", require,
            "--drawn", drawn,
        ]
        if built:
            args += ["--built", built]
        result = self.vds(*args)
        self.assertEqual(result.returncode, EXIT_PASSED, result.stderr)

    def record_path(self, component_id: str) -> Path:
        return self.register_dir / f"{component_id}.yaml"

    def edit_record(self, component_id: str, old: str, new: str) -> None:
        """Hand-edit a register record. The register is authored, not generated,
        so this is a legal act; it is used to reach states the CLI's own
        lifecycle guards make unreachable (retirement needs a `retirement_drain`
        proof, which VDS specifies and does not implement)."""
        path = self.record_path(component_id)
        text = path.read_text(encoding="utf-8")
        self.assertIn(old, text, f"{path.name} does not contain {old!r}")
        path.write_text(text.replace(old, new, 1), encoding="utf-8")

    def append_to_record(self, component_id: str, text: str) -> None:
        path = self.record_path(component_id)
        path.write_text(path.read_text(encoding="utf-8").rstrip("\n") + "\n" + text, "utf-8")

    # -- the D1 attack -------------------------------------------------------

    def load_ledger(self) -> dict:
        return yamlish.load(self.ledger_path)

    def hand_delete_reference(self, name: str) -> None:
        """[2026] VJS-CC-OPBOX audit D1, performed exactly as the auditor did it.

        Delete the five-line `- name: X` entry from the generated ledger by hand
        and touch nothing else. Before the fix this turned a proof from exit 1
        with a named violation into exit 0 "PASS", captured as a
        `capture_mode: automatic` record attesting to it.
        """
        lines = self.ledger_path.read_text(encoding="utf-8").splitlines(True)
        kept, i, removed = [], 0, 0
        while i < len(lines):
            if lines[i].strip() == f"- name: {name}":
                i += 5
                removed += 1
                continue
            kept.append(lines[i])
            i += 1
        self.assertEqual(removed, 1, f"the fixture has no single `- name: {name}` entry")
        self.ledger_path.write_text("".join(kept), encoding="utf-8")

    def drop_reference_and_reseal(self, name: str) -> None:
        """The same attack by an attacker who read scan.py and recomputed the
        in-file `derived_digest`. This is why the guarantee has to be
        re-derivation from the source and not a digest stored beside the rows."""
        ledger = self.load_ledger()
        for screen in ledger["screens"]:
            screen["references"] = [r for r in screen["references"] if r["name"] != name]
        ledger["derived_digest"] = scan.derived_digest_of(ledger["screens"])
        yamlish.dump(ledger, self.ledger_path)

    # -- assertions ----------------------------------------------------------

    def assert_violation(
        self, result: subprocess.CompletedProcess, *needles: str, count: int | None = None
    ) -> None:
        """A seeded violation must exit 1 AND name the row and the reason.

        Both limbs matter. An exit code with no location tells an author that
        something is wrong and not what, which is how a gate gets disabled.
        """
        self.assertEqual(
            result.returncode,
            EXIT_VIOLATION,
            f"expected exit {EXIT_VIOLATION}, got {result.returncode}\n"
            f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}",
        )
        if count is not None:
            self.assertIn(f"VIOLATIONS ({count})", result.stdout)
        for needle in needles:
            self.assertIn(needle, result.stdout)

    def assert_proof_refused(self, result: subprocess.CompletedProcess, needle: str) -> None:
        """A tampered input must refuse loudly (exit 2) and mint NO record.

        The second limb is the one D1 turned on: the original defect was not only
        that the tampering bought a pass, it was that the pass was written to
        disk as automatic-capture evidence a warrant could then cite.
        """
        self.assertEqual(
            result.returncode,
            EXIT_PRECONDITION,
            f"expected exit {EXIT_PRECONDITION}, got {result.returncode}\n"
            f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}",
        )
        self.assertIn(PROOF_REFUSAL, result.stderr)
        self.assertIn(needle, result.stderr)
        self.assertEqual(
            self.proof_records(),
            [],
            "a refused run captured a proof record. Evidence of a run that did not "
            "happen is the defect, not the fix.",
        )

    def assert_cli_refused(self, result: subprocess.CompletedProcess, *needles: str) -> None:
        self.assertEqual(
            result.returncode,
            EXIT_PRECONDITION,
            f"expected exit {EXIT_PRECONDITION}, got {result.returncode}\n"
            f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}",
        )
        self.assertIn(CLI_REFUSAL, result.stderr)
        for needle in needles:
            self.assertIn(needle, result.stderr)

    def assert_clean_pass(self, result: subprocess.CompletedProcess, rows: int) -> None:
        """A proof that always fails gets switched off, and then nothing is
        checked at all. The clean direction is as load-bearing as the failing
        one, and the row count is asserted because a pass over zero rows is
        vacuous rather than good (VDS S-7(2)(4))."""
        self.assertEqual(
            result.returncode,
            EXIT_PASSED,
            f"expected exit {EXIT_PASSED}, got {result.returncode}\n"
            f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}",
        )
        self.assertIn(f"PASS: {rows} enforceable rows checked, 0 violations.", result.stdout)

    def capture(self, kind: str) -> str:
        """Run a proof for real and return the id of the record it captured."""
        before = set(self.proof_records())
        result = self.proof(kind)
        self.assertEqual(result.returncode, EXIT_PASSED, result.stdout + result.stderr)
        new = sorted(set(self.proof_records()) - before)
        self.assertEqual(len(new), 1, "a passing capture run must write exactly one record")
        return new[0].stem
