'use strict';
/*
 * TOKEN REACH: does a declared custom property actually reach anything?
 *
 * The defect this closes was found in somebody else's work and then found here.
 *
 * Opbox's design kit states a rule in its README - "Ink acts, blue selects:
 * primary buttons are #171717" - and defines `--action: #171717` to carry it,
 * commented "solid primary-button fill (ink)". `var(--action)` is then consumed
 * ZERO times anywhere in the kit, while `.btn-primary` fills with `--accent`,
 * the blue the rule reserves for selection. The rule was written down, the
 * mechanism to honour it was built, and the two were never connected.
 *
 * Nothing catches that. A token file is valid, the CSS is valid, the rule is
 * documented, and every instrument reports green - because every instrument
 * checks ONE SIDE. The question no one asks is whether the declaration is
 * REACHABLE, and it is a question about the pair.
 *
 * # Two directions, and the quiet one is the dangerous one
 *
 * UNREFERENCED - declared and used by nothing. Usually harmless (a token for a
 * component not built yet), and occasionally the whole story, as above. It
 * cannot be an error on its own, so it is REPORTED with its declaration site
 * and left for a person.
 *
 * UNDECLARED - `var(--x)` where nothing declares `--x`. This one is a defect
 * every time: the browser silently falls back to the initial value or to the
 * fallback argument, so the surface renders as though the token layer did not
 * exist, and it looks exactly like a design decision.
 *
 * # What it does not do
 *
 * It does not resolve `var(--a, var(--b))` chains to decide which arm wins, and
 * it does not know that a token is referenced from a file it was not given. Both
 * are stated on the reading rather than assumed away: `sources` names what was
 * read, so a reading over one file is visibly a reading over one file.
 */

/**
 * CSS comments, blanked to spaces so offsets and line numbers survive.
 *
 * Found by the check's own negative direction: `--token` was reported as
 * referenced-but-never-declared, and every hit was a COMMENT using
 * `var(--token)` as a placeholder for "some token". A reader that counts prose
 * as code invents work, and an undeclared-token finding is supposed to be the
 * direction that is a defect every time.
 */
function stripComments(css) {
  return css.replace(/\/\*[\s\S]*?\*\//g, (m) => m.replace(/[^\n]/g, ' '));
}

/** Every `--name:` DECLARATION, with the line it is on. */
function declarationsIn(css, source) {
  const out = new Map();
  stripComments(css).split('\n').forEach((line, i) => {
    // A declaration is `--name:` at the start of a statement. Requiring the
    // preceding character to be a delimiter is what keeps `var(--x)` - where
    // `--x` is followed by `)` and never `:` - from being read as one.
    for (const m of line.matchAll(/(^|[;{,\s])(--[a-zA-Z][\w-]*)\s*:/g)) {
      const name = m[2];
      if (!out.has(name)) out.set(name, { source, line: i + 1, text: line.trim().slice(0, 90) });
    }
  });
  return out;
}

/** Every `var(--name)` REFERENCE, counted. */
function referencesIn(css) {
  const out = new Map();
  for (const m of stripComments(css).matchAll(/var\(\s*(--[a-zA-Z][\w-]*)/g)) {
    out.set(m[1], (out.get(m[1]) || 0) + 1);
  }
  return out;
}

/**
 * @param {Array<{source: string, css: string}>} files every file to consider,
 *   declarations and references together. Passing only the token file would make
 *   every token unreferenced, which is why this takes a LIST and names it.
 * @param {{ignoreDeclared?: string[]}} [opts]
 */
function tokenReach(files, opts = {}) {
  const ignore = new Set(opts.ignoreDeclared || []);
  const declared = new Map();
  const referenced = new Map();
  for (const f of files) {
    for (const [name, where] of declarationsIn(f.css, f.source)) {
      if (!declared.has(name)) declared.set(name, where);
    }
    for (const [name, n] of referencesIn(f.css)) {
      referenced.set(name, (referenced.get(name) || 0) + n);
    }
  }

  const unreferenced = [];
  for (const [name, where] of declared) {
    if (ignore.has(name)) continue;
    if (!referenced.has(name)) unreferenced.push({ name, ...where });
  }
  const undeclared = [];
  for (const [name, uses] of referenced) {
    if (!declared.has(name)) undeclared.push({ name, uses });
  }
  unreferenced.sort((a, b) => a.name.localeCompare(b.name));
  undeclared.sort((a, b) => b.uses - a.uses || a.name.localeCompare(b.name));

  return {
    sources: files.map((f) => f.source),
    declared: declared.size,
    referenced: referenced.size,
    unreferenced,
    undeclared,
    doesNotCover: [
      'a token referenced from a file not in `sources` reads as unreferenced here',
      'a `var(--a, var(--b))` fallback chain is counted as a reference to BOTH, because ' +
        'which arm wins depends on cascade state this reader does not have',
      'a token referenced only from markup (a `style=` attribute) or from JS',
    ],
  };
}

module.exports = { tokenReach, declarationsIn, referencesIn, stripComments };
