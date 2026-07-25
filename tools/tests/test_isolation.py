#!/usr/bin/env python3
"""The suite's own guardrails.

Two things have to be true of a test suite that drives a governance tool with a
`--root` flag, and neither is self-evident:

  1. no test may name a path outside a temp directory. A suite that can reach an
     adopting repository's `.vds/` can corrupt the record it exists to protect,
     and a crashing test would leave it corrupt.
  2. the guard that watches the VDS install has to actually detect a change.
     `VdsProjectCase` re-digests `tools/` and `schema/` after every test, and an
     integrity check nobody has ever seen fail is a comment, not a check.

Both are asserted here rather than assumed, because "the tests are careful" is
the same species of claim as "the proof is automatic": true only while someone is
watching, unless something checks it.
"""

from __future__ import annotations

import ast
import re
import sys
import tempfile
import unittest
from pathlib import Path

STDLIB = set(sys.stdlib_module_names)

sys.path.insert(0, str(Path(__file__).resolve().parent))

import vdsfixture  # noqa: E402
from vdsfixture import TESTS_DIR  # noqa: E402

# A quoted absolute path in a test source. Shebangs are comments and are excluded
# by requiring the quote.
ABSOLUTE_IN_STRING = re.compile(r"""["'](/[^"'\s]*)["']""")


class SuiteIsolationTest(unittest.TestCase):
    def test_no_test_source_names_an_absolute_path_outside_a_temp_dir(self) -> None:
        """The structural limb. Every fixture is built by mkdtemp, so any
        absolute path literal in this directory is either a temp path or a
        mistake about to be made on someone's real repository."""
        temp_root = str(Path(tempfile.gettempdir()).resolve())
        offenders: list[str] = []
        for source in sorted(TESTS_DIR.glob("*.py")):
            text = source.read_text(encoding="utf-8")
            for line_number, line in enumerate(text.splitlines(), start=1):
                for found in ABSOLUTE_IN_STRING.findall(line):
                    if found.startswith(temp_root):
                        continue
                    offenders.append(f"{source.name}:{line_number}: {found}")
        self.assertEqual(
            offenders,
            [],
            "a test names an absolute path outside the temp directory. Every fixture "
            "must be built under tempfile.mkdtemp, so no test can touch a real "
            "repository even when it crashes half way through.",
        )

    def test_the_install_integrity_guard_detects_a_change(self) -> None:
        """Mutation check on the guard itself.

        `install_manifest` is pointed at a throwaway tree, which is then changed
        three ways: content, addition, deletion. If any of the three came back
        equal, the per-test guard would be silently permitting the suite to
        rewrite the tool it tests.
        """
        with tempfile.TemporaryDirectory(prefix="vds-guard-") as raw:
            tree = Path(raw)
            (tree / "a.py").write_text("original\n", encoding="utf-8")
            (tree / "sub").mkdir()
            (tree / "sub" / "b.json").write_text("{}\n", encoding="utf-8")

            original = vdsfixture.PROTECTED_TREES
            vdsfixture.PROTECTED_TREES = (tree,)
            self.addCleanup(setattr, vdsfixture, "PROTECTED_TREES", original)

            before = vdsfixture.install_manifest()
            self.assertEqual(len(before), 2, "the guard must see every file in the tree")
            self.assertEqual(vdsfixture.install_manifest(), before, "it must be stable")

            (tree / "a.py").write_text("weakened\n", encoding="utf-8")
            self.assertNotEqual(vdsfixture.install_manifest(), before, "edit undetected")

            (tree / "a.py").write_text("original\n", encoding="utf-8")
            self.assertEqual(vdsfixture.install_manifest(), before)

            (tree / "c.py").write_text("added\n", encoding="utf-8")
            self.assertNotEqual(vdsfixture.install_manifest(), before, "addition undetected")

            (tree / "c.py").unlink()
            (tree / "sub" / "b.json").unlink()
            self.assertNotEqual(vdsfixture.install_manifest(), before, "deletion undetected")

    def test_every_test_module_is_runnable_on_its_own(self) -> None:
        """`python3 tools/tests/test_x.py` has to work, because that is what a
        person does when one test fails and they want only that one."""
        modules = [p for p in sorted(TESTS_DIR.glob("test_*.py"))]
        self.assertGreaterEqual(len(modules), 6, "the suite lost a module")
        for module in modules:
            text = module.read_text(encoding="utf-8")
            self.assertIn(
                'if __name__ == "__main__":',
                text,
                f"{module.name} cannot be run directly",
            )
            self.assertIn("unittest.main", text, f"{module.name} has no runner")

    def test_the_suite_imports_only_the_stdlib_and_vds_itself(self) -> None:
        """VDS runs anywhere with no install step, and its tests must not be the
        thing that breaks that.

        Parsed rather than grepped: a grep for the banned names matches the list
        of banned names, which is how a check ends up failing on its own text.
        """
        allowed_local = {"vdsfixture", "vdslib", "vds", "proofs"}
        offenders: list[str] = []
        for source in sorted(TESTS_DIR.glob("*.py")):
            tree = ast.parse(source.read_text(encoding="utf-8"), filename=source.name)
            for node in ast.walk(tree):
                if isinstance(node, ast.Import):
                    roots = [alias.name.split(".")[0] for alias in node.names]
                elif isinstance(node, ast.ImportFrom):
                    roots = [(node.module or "").split(".")[0]] if node.level == 0 else []
                else:
                    continue
                for root in roots:
                    if not root or root in allowed_local or root in STDLIB:
                        continue
                    offenders.append(f"{source.name}:{node.lineno}: {root}")
        self.assertEqual(
            offenders, [], "the suite acquired a third-party dependency"
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
