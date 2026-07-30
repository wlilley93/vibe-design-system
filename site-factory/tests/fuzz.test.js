'use strict';

/*
 * Randomised configs, DETERMINISTICALLY.
 *
 * The studio's Reroll button hands the renderer combinations nobody chose, so the
 * whole cross-product has to hold up. But a fuzz driven by Math.random that fails
 * once and passes on the retry is close to useless: you cannot reproduce it, so you
 * cannot fix it, so in practice it gets re-run until green. Seeded here instead —
 * a failure names the seed and the case, and re-running reproduces it exactly.
 *
 * SEED can be overridden to explore beyond the committed run:
 *   SF_FUZZ_SEED=12345 SF_FUZZ_CASES=5000 node --test tests/fuzz.test.js
 */

const test = require('node:test');
const assert = require('node:assert');

const { LAYERS, ROUTES } = require('../config-schema.js');
const { suggest } = require('../suggest.js');
const { configToTokens, configToManifest, listStylePacks, listBlockVariants } = require('../compose.js');
const { renderPage } = require('../build.js');

const SEED = Number(process.env.SF_FUZZ_SEED || 20260730);
const CASES = Number(process.env.SF_FUZZ_CASES || 1500);

// mulberry32: small, fast, and above all reproducible.
function rng(seed) {
  let a = seed >>> 0;
  return function next() {
    a |= 0; a = (a + 0x6D2B79F5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const PACKS = listStylePacks();
const VARIANTS = Object.values(listBlockVariants()).flat();
const ROUTE_NAMES = Object.keys(ROUTES);
const ENUMS = [];
for (const l of LAYERS) {
  for (const f of l.fields) if (f.type === 'enum') ENUMS.push([l.key, f.key, f.options]);
}

test(`${CASES} seeded random configs all render (seed ${SEED})`, () => {
  const rand = rng(SEED);
  const pick = (arr) => arr[Math.floor(rand() * arr.length)];

  for (let i = 0; i < CASES; i++) {
    const category = pick(ROUTE_NAMES);
    const c = suggest({ name: `Fuzz ${i}`, tagline: 'A tagline.', category, description: 'A platform.' });
    c.identity = { name: `Fuzz ${i}`, tagline: 'A tagline.', category, description: 'A platform.' };
    c.governance = { vds: false };

    for (const [lk, fk, opts] of ENUMS) if (c[lk]) c[lk][fk] = pick(opts);
    c.palette.basePack = pick(PACKS);
    c.spacing.spaceUnit = 1 + Math.floor(rand() * 16);
    c.strategy.sitemap = Array.from({ length: 1 + Math.floor(rand() * 10) }, () => pick(VARIANTS));

    // The case is reconstructible from the seed and index alone, so a failure here
    // is a bug report: re-run with the same SF_FUZZ_SEED and it recurs.
    const describe = `seed=${SEED} case=${i} route=${category} pack=${c.palette.basePack} sitemap=${c.strategy.sitemap.join(',')}`;
    let html;
    assert.doesNotThrow(() => { html = renderPage(configToManifest(c), configToTokens(c), null).html; }, describe);
    assert.ok(html.includes('<body>'), `empty render - ${describe}`);
    assert.ok(!html.includes('undefined'), `leaked "undefined" - ${describe}`);
    assert.ok(!html.includes('NaN'), `leaked "NaN" - ${describe}`);
  }
});

test('a sitemap of blocks that do not exist is refused, not silently dropped', () => {
  const c = suggest({ name: 'X', category: 'marketing-site', description: 'x' });
  c.identity = { name: 'X', tagline: '', category: 'marketing-site', description: '' };
  c.strategy.sitemap = ['nope-1'];
  assert.throws(() => renderPage(configToManifest(c), configToTokens(c), null), /no block type/);
});

test('an unknown style pack names itself rather than falling back silently', () => {
  const c = suggest({ name: 'X', category: 'marketing-site', description: 'x' });
  c.identity = { name: 'X', tagline: '', category: 'marketing-site', description: '' };
  c.palette.basePack = 'no-such-pack';
  assert.throws(() => configToTokens(c), /no style pack "no-such-pack"/);
});
