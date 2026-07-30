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
      `${layerKey}.${field} produced identical ${cssVar} for "${a}" and "${b}" - the control is decorative`
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
  // "structur" inside \b(...)\b can never match "structuring" - the boundary after
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
  // cryptic - 'No items found.' is not enough." A blank with no action is the defect.
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

/*
 * The rule the README opens with is "a control the renderer ignores is a control that
 * lies", and it named four fields as the ones that had been caught. The test written for
 * it checked those four. Eleven of the nineteen enum fields were reachable; EIGHT could be
 * rotated in the studio and change nothing, so the claim that the class was closed was
 * false for nearly half of it.
 *
 * This covers every enum field, and the exemptions are LISTED WITH REASONS rather than
 * simply absent. An exemption you can read is a decision; an uncovered field is a gap
 * nobody knows about.
 */
const NO_SURFACE = {
  imageTreatment: 'no block renders an image yet, so there is no treatment to apply. Wiring this needs an image slot first.',
  iconStyle: 'no block renders an icon yet. The icon system exists in Figma (381 normalised Lucide glyphs) and has no code counterpart here.',
};

test('every enum field moves the output, or is exempt with a stated reason', () => {
  const { LAYERS } = require('../config-schema.js');
  const { configToTokens, configToManifest } = require('../compose.js');
  const { renderPage } = require('../build.js');
  const { suggest } = require('../suggest.js');

  // BOTH routes. componentStyle applies to the app surface, so probing only the marketing
  // route reported statusBadgeStyle as dead when it is wired and simply has no table to
  // reach on a marketing page. A field counts as live if it moves the output on ANY route
  // where its layer applies - and a test that got that wrong would send someone to "fix"
  // a control that already works.
  const bases = ['marketing-site', 'saas-app'].map((route) => {
    const cfg = suggest({ name: 'Probe', category: route, description: 'A matter tool for law firms.' });
    if (route === 'saas-app') {
      // Reach every app block, so a field wired to one of them is not reported dead for
      // want of a host on the page.
      cfg.strategy.sitemap = ['nav-1', 'sidebar-2', 'objecttable-1', 'card-1', 'segmentedcontrol-2', 'formfield-1'];
    }
    return cfg;
  });

  const renderWith = (base, layerKey, key, value) => {
    const cfg = JSON.parse(JSON.stringify(base));
    cfg[layerKey][key] = value;
    const r = renderPage(configToManifest(cfg), configToTokens(cfg), null);
    return r.html + r.css;
  };

  const dead = [];
  for (const layer of LAYERS) {
    for (const field of layer.fields || []) {
      if (field.type !== 'enum' || !field.options || field.options.length < 2) continue;
      const movesOn = bases.filter((base) =>
        new Set(field.options.map((o) => renderWith(base, layer.key, field.key, o))).size > 1);
      const outputs = new Set(movesOn.length ? ['moved'] : ['same']);
      if (outputs.size === 1 && !movesOn.length) {
        if (NO_SURFACE[field.key]) continue;      // declared, with a reason, above
        dead.push(`${layer.key}.${field.key} (${field.options.length} options, one output)`);
      } else if (movesOn.length && NO_SURFACE[field.key]) {
        // The other direction: an exemption that is no longer true is stale documentation
        // claiming a limitation that has been fixed.
        dead.push(`${layer.key}.${field.key} is listed in NO_SURFACE but DOES move the output - remove the exemption`);
      }
    }
  }

  assert.deepEqual(dead, [],
    'controls the renderer ignores. Wire them, or add them to NO_SURFACE with the reason:\n  ' + dead.join('\n  '));
});

/*
 * The contrast maths, once, so both tests below derive rather than assert.
 * WCAG 2.2 relative luminance, SC 1.4.3.
 */
function contrast(a, b) {
  const lum = (h) => {
    const c = [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16) / 255)
      .map((v) => (v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4));
    return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
  };
  const [hi, lo] = [lum(a), lum(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

test('a selector named for a meaning does not resolve to the brand accent', () => {
  // `.badge--danger` rendered in `--color-accent`. So did an invalid field's border, a
  // destructive dialog's consequence line and its confirm button. A danger badge and a
  // "Get started" button were the same colour, which means the page could not say danger.
  //
  // The drift is the interesting part: the Figma VDS Tokens collection ALREADY declared
  // color/danger, color/warning, color/success and color/info. The code had none of them, so
  // one half of the system had a vocabulary the other half could not express - exactly what
  // this repo exists to catch, found in the repo itself.
  const { STRUCTURE_CSS } = require('../build.js');
  const MEANS = /danger|error|invalid|consequence|destruct|warning|success/i;

  const offenders = [];
  let selector = '';
  for (const line of STRUCTURE_CSS.split('\n')) {
    const m = line.match(/^([.a-z][^{]*)\{/);
    if (m) selector = m[1].trim();
    if (MEANS.test(selector) && /var\(--color-accent(Ink)?\)/.test(line)) {
      offenders.push(`${selector}: ${line.trim().slice(0, 60)}`);
    }
  }
  assert.deepEqual(offenders, [],
    `these selectors are named for a meaning and painted with the brand accent:\n  ${offenders.join('\n  ')}`);
});

test('every tone clears WCAG AA against the ink chosen for it, in every pack', () => {
  // This is BREACH-0001's own gate, one level down. That breach was a control boundary
  // DECLARED aligned and never measured, shipping at 1.15:1 across five themes. So no tone
  // here is asserted: the ratio is computed from the two values in the pack.
  //
  // It has already earned itself. Deriving the inks flagged jellytot's danger at 4.17:1, and
  // the first repair walked the red DARKER against a dark ink, which made it worse and found
  // no candidate at all. That failure was the loop telling the truth rather than settling for
  // the closest miss.
  const fs = require('node:fs');
  const path = require('node:path');
  const { listStylePacks } = require('../compose.js');

  const AA = 4.5;
  const TONES = ['danger', 'warning', 'success', 'info'];
  const failures = [];

  for (const pack of listStylePacks()) {
    const c = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'tokens', `${pack}.json`), 'utf8')).colors;
    for (const tone of TONES) {
      assert.ok(c[tone], `${pack} declares no ${tone} tone`);
      assert.ok(c[`${tone}Ink`], `${pack} declares ${tone} with no ${tone}Ink to read it against`);
      const r = contrast(c[tone], c[`${tone}Ink`]);
      if (r < AA) failures.push(`${pack}/${tone}: ${c[tone]} on ${c[`${tone}Ink`]} is ${r.toFixed(2)}:1`);
    }
    // The brand accent too, since it carries accentInk on every button.
    const ar = contrast(c.accent, c.accentInk);
    if (ar < AA) failures.push(`${pack}/accent: ${c.accent} on ${c.accentInk} is ${ar.toFixed(2)}:1`);
  }
  assert.deepEqual(failures, [], `tone pairs below WCAG AA ${AA}:1:\n  ${failures.join('\n  ')}`);
});
