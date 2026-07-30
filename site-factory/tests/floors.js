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
module.exports = {
  MIN_FILES: 7,
  MIN_TESTS: 82,
};
