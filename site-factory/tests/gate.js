#!/usr/bin/env node
'use strict';

/*
 * The site-factory gate. `node tests/gate.js`
 *
 * Why this exists rather than a bare `node --test tests/*.test.js` in the Makefile:
 * that command EXITS 0 WHEN THE GLOB MATCHES NOTHING. Measured, not assumed — an
 * empty tests/ directory returns exit code 0, so a rename, a move, or a bad path in
 * CI would turn the gate off and report success. A check that cannot fail is not a
 * check; it is a green light wired to nothing.
 *
 * So this runner asserts a FLOOR on what actually ran before it is willing to pass,
 * and it does not pipe the test output through anything that could swallow the
 * child's exit status.
 */

const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const TESTS_DIR = __dirname;

// Floors, not exact counts: adding a test must never be a reason to edit this file,
// but losing one must be caught. Raise them when the suite grows meaningfully.
const MIN_FILES = 7;
const MIN_TESTS = 54;

const files = fs.readdirSync(TESTS_DIR)
  .filter((f) => f.endsWith('.test.js'))
  .map((f) => path.join(TESTS_DIR, f))
  .sort();

if (files.length < MIN_FILES) {
  console.error(`gate: found ${files.length} test files, expected at least ${MIN_FILES}.`);
  console.error('gate: refusing to report a pass on a suite that has lost files.');
  process.exit(1);
}

const run = spawnSync(process.execPath, ['--test', ...files], { encoding: 'utf8' });
process.stdout.write(run.stdout || '');
process.stderr.write(run.stderr || '');

if (run.error) {
  console.error(`gate: could not run the suite: ${run.error.message}`);
  process.exit(1);
}

const out = `${run.stdout || ''}\n${run.stderr || ''}`;
const num = (label) => {
  const m = out.match(new RegExp(`^# ${label} (\\d+)$`, 'm'));
  return m ? Number(m[1]) : null;
};
const total = num('tests');
const passed = num('pass');
const failed = num('fail');

if (total === null || passed === null || failed === null) {
  console.error('gate: could not read the test summary from the runner output.');
  process.exit(1);
}
if (total < MIN_TESTS) {
  console.error(`gate: only ${total} tests ran, expected at least ${MIN_TESTS}.`);
  process.exit(1);
}
if (failed > 0 || run.status !== 0) {
  console.error(`gate: ${failed} of ${total} tests failed.`);
  process.exit(1);
}

console.log(`gate: ${passed}/${total} site-factory tests passed across ${files.length} files.`);
