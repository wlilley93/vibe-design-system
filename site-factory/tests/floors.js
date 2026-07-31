'use strict';

/*
 * The floors the suite must clear, in ONE place.
 *
 * Floors, not exact counts: adding a test must never be a reason to edit this file,
 * but losing one must be caught.
 *
 * They live here rather than in gate.js because two things need to read them - the
 * gate, and the README test that checks the README quotes a true test count - and
 * gate.js RUNS THE SUITE when it is required. A test requiring it would spawn the
 * suite from inside the suite. Two hand-kept copies of the number would have been the
 * other option, and a number kept in two places is a number that will disagree.
 */
/*
 * THE GATE SURFACE: the files that DECIDE, and which the enforcement lock must
 * witness.
 *
 * Measured 2026-07-31: NOT ONE site-factory file was in `.vds/enforcement.lock`.
 * The Rust half has been pinned since the beginning, and `vds_proof::GATE_PATHS`
 * - the list the lock's UNPINNED report walks - is one path per proof kind, so
 * it is blind to this half by construction and could never have nagged about it.
 * 110 tests, a runner, and these floors, all unwitnessed: anyone could lower
 * MIN_TESTS or hollow out a test file and nothing outside the diff would say so.
 *
 * The distinction drawn here is DECIDES versus IS DECIDED ABOUT. `build.js` is
 * the subject of tests; `token-reach.js` renders a verdict. Pinning subjects
 * would make every ordinary edit a re-pin and the lock a thing people route
 * around, which is worse than not pinning at all.
 *
 * Adding a file here without pinning it FAILS - see tests/skills.test.js.
 */
const GATES = [
  // The runner. If it stops running files, everything passes.
  'site-factory/tests/gate.js',
  // This file. It holds the floors, so weakening it weakens every count.
  'site-factory/tests/floors.js',
  // An instrument that renders a verdict rather than being measured by one.
  'site-factory/token-reach.js',
];

/**
 * Which declared gates are unpinned, and which have drifted.
 *
 * Pure, and exported for exactly one reason: the seeded test for the UNPINNED
 * branch could not reach it. Adding a gate to the list above edits THIS file, so
 * its own digest drifts and the drift assertion fires first - the unpinned
 * branch was unreachable through the real artefacts and had never once run.
 * A branch that cannot be reached is a branch that is not tested, however green
 * the suite is.
 *
 * @param {string[]} gates repository-relative paths
 * @param {string} lockText the enforcement lock, verbatim
 * @param {(gate: string) => string|null} digestOf sha256 hex, or null if absent
 */
function lockFindings(gates, lockText, digestOf) {
  const missingFile = [];
  const unpinned = [];
  const drifted = [];
  for (const gate of gates) {
    const digest = digestOf(gate);
    if (digest === null) { missingFile.push(gate); continue; }
    if (!lockText.includes(`path: ${gate}\n`)) { unpinned.push(gate); continue; }
    if (!lockText.includes(`sha256:${digest}`)) drifted.push({ gate, digest });
  }
  return { missingFile, unpinned, drifted };
}

module.exports = {
  MIN_FILES: 7,
  MIN_TESTS: 111,
  GATES,
  lockFindings,
};
