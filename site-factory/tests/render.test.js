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
      `block type "${type}" has no real placeholder — it fell through to the generic {note} object, ` +
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
