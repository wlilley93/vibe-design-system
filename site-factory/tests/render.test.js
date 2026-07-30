'use strict';

/*
 * Every block type, every variant, rendered with the placeholder content the
 * factory itself would hand it.
 *
 * This is the bug class that actually bit: placeholderContent() had entries for
 * three of twelve types and every other one fell through to a generic `{note: ...}`
 * that no render function could destructure. It scaffolded clean and died at build
 * with "Cannot read properties of undefined (reading 'map')". Nothing caught it
 * because nothing ran it.
 */

const test = require('node:test');
const assert = require('node:assert');

const { BLOCKS, renderPage, STRUCTURE_CSS } = require('../build.js');
const { placeholderContent } = require('../scaffold.js');
const { listStylePacks } = require('../compose.js');
const path = require('path');

const PACKS = listStylePacks().map((n) => require(path.join('..', 'tokens', `${n}.json`)));

test('every block variant renders with its own placeholder content', () => {
  const types = Object.keys(BLOCKS);
  assert.ok(types.length >= 17, `expected at least 17 block types, found ${types.length}`);

  for (const type of types) {
    const content = placeholderContent(type);
    assert.ok(
      !Object.prototype.hasOwnProperty.call(content, 'note'),
      `block type "${type}" has no real placeholder - it fell through to the generic {note} object, ` +
      'which no render function can destructure'
    );
    for (const variant of Object.keys(BLOCKS[type])) {
      const manifest = { title: 't', page: [{ block: type, variant, content }] };
      const { html } = renderPage(manifest, PACKS[0], null);
      assert.ok(html.length > 0, `${variant} rendered empty`);
      assert.ok(!html.includes('undefined'), `${variant} leaked the string "undefined" into its markup`);
      assert.ok(!html.includes('[object Object]'), `${variant} leaked "[object Object]" into its markup`);
    }
  }
});

test('every variant renders under every style pack', () => {
  for (const type of Object.keys(BLOCKS)) {
    const content = placeholderContent(type);
    for (const variant of Object.keys(BLOCKS[type])) {
      for (const tokens of PACKS) {
        const manifest = { title: 't', page: [{ block: type, variant, content }] };
        assert.doesNotThrow(
          () => renderPage(manifest, tokens, null),
          `${variant} threw under style pack "${tokens.name}"`
        );
      }
    }
  }
});

test('an unknown block or variant fails loudly, not silently', () => {
  assert.throws(
    () => renderPage({ title: 't', page: [{ block: 'nope', variant: 'nope-1', content: {} }] }, PACKS[0], null),
    /no block type "nope"/,
    'an unknown block type must name itself in the error'
  );
  assert.throws(
    () => renderPage({ title: 't', page: [{ block: 'hero', variant: 'hero-99', content: {} }] }, PACKS[0], null),
    /no variant "hero-99"/,
    'an unknown variant must name itself in the error'
  );
});

test('the structural stylesheet holds no colour or font values', () => {
  // Values belong in the token block; structure may only reference them. A hex here
  // is a value that no style pack can override, which defeats the whole pipeline.
  const hex = STRUCTURE_CSS.match(/#[0-9a-fA-F]{3,8}\b/g);
  assert.equal(hex, null, `structural CSS contains hex literals: ${hex && hex.join(', ')}`);
});

test('rendered markup escapes HTML in content', () => {
  const nasty = '<script>alert(1)</script>';
  const { html } = renderPage(
    { title: 't', page: [{ block: 'hero', variant: 'hero-1', content: { ...placeholderContent('hero'), h1: nasty } }] },
    PACKS[0], null
  );
  assert.ok(!html.includes('<script>alert(1)</script>'), 'content was interpolated without escaping');
  assert.ok(html.includes('&lt;script&gt;'), 'expected the escaped form in the output');
});

test('the stylesheet holds no magic number: every value is a token or a documented sentinel', () => {
  // The README claimed "no hex, no font name, no px literal outside the --space multiplier"
  // and then offered only a grep for HEX as its evidence. Sixteen distinct px declarations
  // survived under that claim for weeks. A rule broader than its check is the same defect as
  // a control the renderer ignores, one level up: both look enforced and are not.
  //
  // So this is the check the same size as the claim.
  const { STRUCTURE_CSS } = require('../build.js');

  // Sentinels, each with a reason. A number that CANNOT be a token belongs here; a number
  // nobody has got round to naming does not, which is why the list is short and argued.
  const SENTINELS = [
    // A pill is "however round it takes", not a length. Any large number does the job and
    // 999px is the idiom; a --radius-pill token would be the same sentinel with a longer name.
    '999px',
  ];

  let scrubbed = STRUCTURE_CSS;
  for (const s of SENTINELS) scrubbed = scrubbed.split(s).join('<sentinel>');

  const hex = scrubbed.match(/#[0-9a-fA-F]{3,8}\b/g) || [];
  assert.deepEqual(hex, [], `hex literals in the stylesheet: ${hex.join(', ')}`);

  // Any surviving px is a magic number. Report it WITH its declaration, because "there is a
  // px somewhere" is not actionable and the selector is what tells you which token it wants.
  const magic = [...new Set(scrubbed.match(/[a-z-]+: [^;{}]*\b\d+px\b[^;{}]*/g) || [])];
  assert.deepEqual(
    magic, [],
    'magic numbers in the stylesheet. Each is a token that has not been named yet, or a ' +
    `sentinel that has not been argued for:\n  ${magic.join('\n  ')}`
  );

  // A font name in the structural sheet would be the same defect in another currency.
  //
  // The first attempt wrote `/font-family:\s*(?!var\()/` and flagged all fifteen declarations
  // INCLUDING the correct ones: `\s*` can match zero characters, so the lookahead was tested
  // against the space rather than the value. A greedy-optional before a lookahead is a
  // lookahead that never fires where you meant it to.
  const fonts = (scrubbed.match(/font-family:[^;}]+/g) || [])
    .filter((d) => !/font-family:\s+(var\(|inherit\b)/.test(d));
  assert.deepEqual(fonts, [], `font names outside a token: ${fonts.join(', ')}`);
});

test('the width tokens are all consumed, and none is a value nothing reads', () => {
  // The mirror of the rule above: a token nothing uses is the same dead weight as a literal
  // nothing named. Both directions, so adding a token obliges you to use it and removing a
  // use obliges you to remove the token.
  const { cssVars, STRUCTURE_CSS } = require('../build.js');
  const fs = require('node:fs');
  const path = require('node:path');

  const tokens = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'tokens', 'geist.json'), 'utf8'));
  tokens.scale = { density: 'comfortable', type: 'comfortable' };
  const declared = [...cssVars(tokens).matchAll(/^\s*--([a-z-]+):/gm)].map((m) => m[1]);

  const widthish = declared.filter((n) => /^(container|measure|rail|pane)/.test(n));
  assert.ok(widthish.length >= 8, `expected the width tokens to be declared, found ${widthish.join(', ')}`);

  const unused = widthish.filter((n) => !STRUCTURE_CSS.includes(`var(--${n})`));
  assert.deepEqual(unused, [], `width tokens declared and never read: ${unused.join(', ')}`);
});
