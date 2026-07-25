#!/usr/bin/env bash
# Run the whole VDS test suite. This is the one command.
#
#   tools/run-tests.sh                 every test
#   tools/run-tests.sh test_cli_lock   one module (a name, a path, or a pattern)
#
# VDS S-7(2)(2): a check is a proof only if a named test seeds a violation and
# asserts the non-zero exit. Until 2026-07-25 `ls -A tools/tests` returned 0, so
# by VDS's own statute none of the three implemented proofs was a proof. This
# script exists so that never again depends on anyone remembering the incantation.
#
# Stdlib only, no install step, no third-party runner. `python3` is the only
# requirement, and 3.11+ because the tooling reads TOML with tomllib.
#
# The suite is also fenced. Every test builds its own project under mkdtemp, and
# the harness re-digests the VDS install after each test. This script repeats that
# around the WHOLE run, and will additionally fence any tree named in
# VDS_TEST_PROTECT (colon separated), which is how an adopting repository keeps
# its own `.vds/` out of harm's way:
#
#   VDS_TEST_PROTECT=/path/to/repo/.vds tools/run-tests.sh

set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$HERE")"
TESTS="$HERE/tests"

PY="${PYTHON:-python3}"
PROTECT="${VDS_TEST_PROTECT:-}"

manifest() {
  # path + sha256 for every file in every fenced tree, sorted, __pycache__ aside.
  local tree
  for tree in "$HERE" "$ROOT/schema" ${PROTECT//:/ }; do
    [ -d "$tree" ] || continue
    find "$tree" -type f -not -path '*/__pycache__/*' -print0 \
      | sort -z \
      | xargs -0 -r sha256sum
  done
}

BEFORE="$(manifest)"
BEFORE_COUNT="$(printf '%s\n' "$BEFORE" | grep -c . || true)"
if [ "$BEFORE_COUNT" -lt 2 ]; then
  # An empty manifest compares equal to an empty manifest, so a fence that
  # digested nothing would report "intact" forever. Refuse instead.
  echo "the fence digested $BEFORE_COUNT files, so it would prove nothing. Refusing to run." >&2
  exit 2
fi

echo "VDS test suite"
echo "  python:  $("$PY" --version 2>&1)"
echo "  tests:   $TESTS"
echo "  fenced:  $BEFORE_COUNT files under $HERE, $ROOT/schema${PROTECT:+, $PROTECT}"
echo ""

if [ "$#" -gt 0 ]; then
  "$PY" -m unittest discover -v -s "$TESTS" -t "$TESTS" -p "$1*.py"
else
  "$PY" -m unittest discover -v -s "$TESTS" -t "$TESTS" -p "test_*.py"
fi
SUITE_EXIT=$?

AFTER="$(manifest)"

echo ""
if [ "$BEFORE" != "$AFTER" ]; then
  echo "FENCE BREACHED: the run modified a protected tree. Diff of the manifest:"
  diff <(printf '%s\n' "$BEFORE") <(printf '%s\n' "$AFTER") || true
  echo ""
  echo "A suite that can rewrite the tool it tests proves nothing about it."
  exit 2
fi
echo "fence intact: no protected tree changed during the run"

if [ "$SUITE_EXIT" -ne 0 ]; then
  echo ""
  echo "suite exit $SUITE_EXIT. Before assuming the harness is broken, read the"
  echo "docstring of each failing test: a test marked KNOWN RED is asserting the"
  echo "behaviour VDS ought to have and does not, and is failing on purpose."
fi
exit "$SUITE_EXIT"
