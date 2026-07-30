'use strict';

/*
 * The config -> (tokens, manifest) mapping, and the promise that every control the
 * UI can rotate actually reaches the artefact.
 *
 * "A control the renderer ignores is a control that lies" is the rule this file
 * enforces. density, typeScale, borderWeight, elevation and statusBadgeStyle were
 * all present in the schema and read by nothing for a while; they looked like
 * settings and changed no pixel.
 */

const test = require('node:test');
const assert = require('node:assert');

const { LAYERS, ROUTES } = require('../config-schema.js');
const { suggest } = require('../suggest.js');
const { configToTokens, configToManifest, listStylePacks } = require('../compose.js');
const { renderPage, cssVars } = require('../build.js');

function cfg(over = {}) {
  const category = over.category || 'marketing-site';
  const c = suggest({ name: 'Test Co', tagline: 'A tagline.', category, description: 'A legal ownership advisory.' });
  c.identity = { name: 'Test Co', tagline: 'A tagline.', category, description: 'A legal ownership advisory.' };
  c.governance = { vds: false };
  return c;
}
function render(c) {
  return renderPage(configToManifest(c), configToTokens(c), null).html;
}

test('suggest fills every field the schema declares', () => {
  for (const category of Object.keys(ROUTES)) {
    const c = cfg({ category });
    for (const layer of LAYERS) {
      if (layer.key === 'identity') continue;
      assert.ok(c[layer.key], `suggest produced no "${layer.key}" layer for route ${category}`);
      for (const f of layer.fields) {
        assert.notStrictEqual(
          c[layer.key][f.key], undefined,
          `suggest left ${layer.key}.${f.key} undefined for route ${category}`
        );
      }
    }
  }
});

test('every enum option a layer offers is a value the renderer accepts', () => {
  for (const layer of LAYERS) {
    for (const f of layer.fields) {
      if (f.type !== 'enum') continue;
      for (const option of f.options) {
        const c = cfg();
        if (!c[layer.key]) continue;
        c[layer.key][f.key] = option;
        assert.doesNotThrow(
          () => render(c),
          `${layer.key}.${f.key} = "${option}" is offered by the schema but breaks the renderer`
        );
      }
    }
  }
});

// Each of these must produce DIFFERENT css for at least two of its options, or it is
// a decorative control.
const REAL_CONTROLS = [
  ['spacing', 'density', ['compact', 'spacious'], '--space'],
  ['typography', 'typeScale', ['compact', 'spacious'], '--type-scale'],
  ['spacing', 'borderWeight', ['hairline', 'bold-2px'], '--border-weight'],
  ['spacing', 'elevation', ['flat', 'soft-shadow'], '--shadow'],
  ['spacing', 'cornerRadius', ['sharp-0', 'pill'], '--radius-sm'],
];

for (const [layerKey, field, [a, b], cssVar] of REAL_CONTROLS) {
  test(`${layerKey}.${field} actually changes ${cssVar}`, () => {
    const ca = cfg(); ca[layerKey][field] = a;
    const cb = cfg(); cb[layerKey][field] = b;
    const grab = (c) => {
      const line = cssVars(configToTokens(c)).split('\n').find((l) => l.trim().startsWith(cssVar + ':'));
      assert.ok(line, `${cssVar} is not emitted at all`);
      return line.trim();
    };
    assert.notStrictEqual(
      grab(ca), grab(cb),
      `${layerKey}.${field} produced identical ${cssVar} for "${a}" and "${b}" — the control is decorative`
    );
  });
}

test('componentStyle.statusBadgeStyle reaches the markup', () => {
  const count = (html, cls) => (html.match(new RegExp(cls, 'g')) || []).length;
  const pill = cfg({ category: 'saas-app' });
  pill.componentStyle.statusBadgeStyle = 'pill';
  const dot = cfg({ category: 'saas-app' });
  dot.componentStyle.statusBadgeStyle = 'dot';
  const hp = render(pill), hd = render(dot);
  assert.ok(count(hp, 'badge--pill') > count(hd, 'badge--pill'), 'choosing "pill" did not produce more pill badges');
  assert.ok(count(hd, 'badge--dot') > count(hp, 'badge--dot'), 'choosing "dot" did not produce more dot badges');
});

test('the saas route declares an app layout and never renders two facet strips', () => {
  const c = cfg({ category: 'saas-app' });
  const m = configToManifest(c);
  assert.equal(m.layout, 'app', 'a saas manifest must declare layout:"app" so renderPage builds a shell');

  // masterdetail draws its own facet strip from content.facets; a standalone one in
  // the same page drew it twice on the first Atlas Ops build.
  c.strategy.sitemap = ['nav-1', 'sidebar-2', 'facetstrip-1', 'masterdetail-2'];
  const blocks = configToManifest(c).page.map((p) => p.block);
  assert.ok(
    !(blocks.includes('facetstrip') && blocks.includes('masterdetail')),
    'a standalone facetstrip survived alongside masterdetail, which renders its own'
  );

  const html = render(c);
  assert.equal((html.match(/class="facet /g) || []).length, 1, 'expected exactly one facet strip');
  assert.ok(html.includes('class="shell"'), 'the app layout did not wrap the panes in a shell');
});

test('the marketing route stacks blocks and builds no app shell', () => {
  const c = cfg({ category: 'marketing-site' });
  const m = configToManifest(c);
  assert.notEqual(m.layout, 'app');
  assert.ok(!render(c).includes('class="shell"'));
});

test('an empty sitemap falls back rather than producing a blank page', () => {
  const c = cfg({ category: 'saas-app' });
  c.strategy.sitemap = [];
  const m = configToManifest(c);
  assert.ok(m.page.length > 0, 'a saas route with an empty sitemap must fall back to a default app surface');
});

test('the palette layer overrides the base pack rather than being overwritten by it', () => {
  const c = cfg();
  c.palette.basePack = listStylePacks()[0];
  c.palette.accentColor = '#ABCDEF';
  const tok = configToTokens(c);
  assert.equal(tok.colors.accent, '#ABCDEF', 'an explicit palette value lost to the base pack');
  assert.ok(cssVars(tok).includes('--color-accent: #ABCDEF'));
});

test('keyword stems match inflected words, not just the bare stem', () => {
  // "structur" inside \b(...)\b can never match "structuring" — the boundary after
  // the stem fails on the following letter. The bug was masked for the whole life of
  // the file because "trust" and "estate" fired on the same briefs. A stem that
  // cannot match is a rule that silently does nothing.
  const { suggest: s } = require('../suggest.js');
  for (const text of ['Global structuring, agnostic', 'we structure holdings', 'offshore jurisdiction advice']) {
    const c = s({ name: 'X', category: 'marketing-site', description: text });
    assert.equal(c.palette.basePack, 'balmoral', `"${text}" should select the balmoral pack`);
  }
  // ...and the stem must not be so greedy it swallows unrelated briefs.
  const saas = s({ name: 'X', category: 'marketing-site', description: 'An analytics platform for developer teams' });
  assert.equal(saas.palette.basePack, 'geist');
});

test('the four demand-measured blocks render on the saas route', () => {
  // Chosen by counting imports across 217 real Opbox routes, not by guess. If the
  // route ever stops accepting them the measurement was wasted.
  const c = cfg({ category: 'saas-app' });
  c.strategy.sitemap = ['nav-1', 'formfield-1', 'emptystate-1', 'pagestate-2', 'confirmdialog-2'];
  const blocks = configToManifest(c).page.map((p) => p.block);
  for (const b of ['formfield', 'emptystate', 'pagestate', 'confirmdialog']) {
    assert.ok(blocks.includes(b), `${b} was filtered out of the saas route`);
  }
  assert.doesNotThrow(() => render(c));
});

test('an empty state always offers a next step', () => {
  // "Empty States" is a Playbook play whose Don't is explicit: "Leave the screen
  // cryptic — 'No items found.' is not enough." A blank with no action is the defect.
  const { BLOCKS } = require('../build.js');
  const { placeholderContent } = require('../scaffold.js');
  for (const variant of Object.keys(BLOCKS.emptystate)) {
    const html = BLOCKS.emptystate[variant](placeholderContent('emptystate'));
    assert.match(html, /class="empty__cta/, `${variant} renders no action`);
  }
});

test('a destructive confirm names the consequence, not just "are you sure"', () => {
  // "Fail Safe": add friction to risky actions. Friction that carries no information
  // is a click, not a safeguard.
  const { BLOCKS } = require('../build.js');
  const { placeholderContent } = require('../scaffold.js');
  for (const variant of Object.keys(BLOCKS.confirmdialog)) {
    const html = BLOCKS.confirmdialog[variant](placeholderContent('confirmdialog'));
    assert.match(html, /cdialog__consequence/, `${variant} states no consequence`);
    assert.match(html, /role="alertdialog"/, `${variant} is not announced as a dialog`);
  }
});

test('form fields keep their label bound to their control', () => {
  // A label that does not point at its input is a label the screen reader drops.
  const { BLOCKS } = require('../build.js');
  const { placeholderContent } = require('../scaffold.js');
  for (const variant of Object.keys(BLOCKS.formfield)) {
    const html = BLOCKS.formfield[variant](placeholderContent('formfield'));
    const forAttrs = [...html.matchAll(/<label[^>]*for="([^"]+)"/g)].map((m) => m[1]);
    assert.ok(forAttrs.length >= 4, `${variant} rendered too few labels`);
    for (const id of forAttrs) {
      assert.ok(new RegExp(`id="${id}"`).test(html), `${variant}: label points at "${id}" which no control carries`);
    }
  }
});
