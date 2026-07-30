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
