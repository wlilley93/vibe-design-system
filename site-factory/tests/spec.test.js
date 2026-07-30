'use strict';

/*
 * The spec sheet must show what the build actually does.
 *
 * figma-spec.js keeps its own copies of DENSITY, TYPE_SCALE, BORDER_WEIGHT and the
 * elevation set because build.js does not export them. Duplicated tables drift, and
 * a spec sheet drawing a spacing value the CSS does not use is worse than the text
 * table it replaced - it looks authoritative and it is wrong. These tests pin the
 * copies against the real rendered output, so a change in build.js that is not
 * mirrored here fails rather than quietly producing a lying sheet.
 */

const test = require('node:test');
const assert = require('node:assert');

const { LAYERS } = require('../config-schema.js');
const { suggest } = require('../suggest.js');
const { configToTokens } = require('../compose.js');
const { cssVars } = require('../build.js');
const { buildSpec, specimensFor, DENSITY, TYPE_SCALE, BORDER_WEIGHT, NOT_STATIC } = require('../figma-spec.js');

function cfg(over = {}) {
  const identity = {
    name: 'Spec Co',
    tagline: 'I draw the structure first',
    category: over.category || 'marketing-site',
    description: 'A private trust and estate structuring advisory',
  };
  const c = suggest(identity);
  c.identity = identity;
  c.governance = { vds: false };
  return c;
}
function cssVar(c, name) {
  const line = cssVars(configToTokens(c)).split('\n').find((l) => l.trim().startsWith(name + ':'));
  return line ? line.trim().split(':')[1].replace(';', '').trim() : null;
}
function field(layerKey, key) {
  return LAYERS.find((l) => l.key === layerKey).fields.find((f) => f.key === key);
}

test('the density specimen equals the --space the build emits', () => {
  for (const option of Object.keys(DENSITY)) {
    const c = cfg();
    c.spacing.density = option;
    const spec = specimensFor(field('spacing', 'density'), c);
    const shown = spec.options.find((o) => o.label === option).value;
    assert.equal(shown, cssVar(c, '--space'), `density "${option}" is drawn as ${shown} but the build emits a different --space`);
  }
});

test('the type-scale specimen equals the --type-scale the build emits', () => {
  for (const option of Object.keys(TYPE_SCALE)) {
    const c = cfg();
    c.typography.typeScale = option;
    const spec = specimensFor(field('typography', 'typeScale'), c);
    const shown = String(spec.options.find((o) => o.label === option).value);
    assert.equal(shown, cssVar(c, '--type-scale'), `typeScale "${option}" drawn as ${shown}, build says otherwise`);
  }
});

test('the border-weight specimen equals the --border-weight the build emits', () => {
  for (const option of field('spacing', 'borderWeight').options) {
    const c = cfg();
    c.spacing.borderWeight = option;
    const spec = specimensFor(field('spacing', 'borderWeight'), c);
    const shown = spec.options.find((o) => o.label === option).value;
    assert.equal(shown, cssVar(c, '--border-weight'), `borderWeight "${option}" drawn as ${shown}`);
    assert.ok(BORDER_WEIGHT[option], `${option} missing from the spec sheet's own table`);
  }
});

test('the corner-radius specimen equals the --radius-sm the build emits', () => {
  for (const option of field('spacing', 'cornerRadius').options) {
    const c = cfg();
    c.spacing.cornerRadius = option;
    const spec = specimensFor(field('spacing', 'cornerRadius'), c);
    const shown = spec.options.find((o) => o.label === option).value;
    assert.equal(shown, cssVar(c, '--radius-sm'), `cornerRadius "${option}" drawn as ${shown}`);
  }
});

test('the voice specimens are the copy the generator would really write', () => {
  const { copyFor } = require('../copy.js');
  const c = cfg();
  const spec = specimensFor(field('voice', 'copyRegister'), c);
  for (const opt of spec.options) {
    const probe = JSON.parse(JSON.stringify(c));
    probe.voice.copyRegister = opt.label;
    const real = copyFor('cta', probe.identity, probe.voice).heading;
    assert.equal(opt.value, real, `the sheet shows copy for "${opt.label}" that copy.js would not write`);
  }
});

test('every field is classified, and motion is refused rather than faked', () => {
  const spec = buildSpec(cfg());
  const rows = spec.sections.flatMap((s) => s.rows);
  assert.ok(rows.length > 25, 'too few rows to be the whole schema');

  for (const r of rows) {
    assert.ok(r.kind, `${r.key} has no specimen kind`);
    if (NOT_STATIC.has(r.key)) {
      assert.equal(r.kind, 'not-static', `${r.key} cannot be shown in a still frame and must say so`);
    }
  }
  // A still frame cannot show duration or easing. Drawing a swatch for it would be a
  // confident lie, so the sheet names the options and draws none of them.
  assert.equal(spec.counts.notStatic, 2);
  assert.ok(spec.counts.specimen >= 25, `only ${spec.counts.specimen} fields carry a drawn specimen`);
});

test('a specimen frame is identified by name, not by what it contains', () => {
  const { isSpecimenFrame } = require('../figma-spec.js');
  // The blanket fill-clear wiped 3 buttons, 1 badge and 8 block chips on the first
  // build, because a container and a swatch are both FRAMEs. Naming is the rule;
  // inspecting children for the word "Enquire" was a one-off rescue.
  assert.ok(isSpecimenFrame({ name: 'spec:button' }));
  assert.ok(isSpecimenFrame({ name: 'spec:badge-pill' }));
  assert.ok(!isSpecimenFrame({ name: 'opt' }));
  assert.ok(!isSpecimenFrame({ name: 'Palette' }));
  assert.ok(!isSpecimenFrame({}));
});

test('the README states counts that are actually true', () => {
  // A README is an artefact like any other, and this one quotes numbers: block types,
  // variants, packs, fields, layers, tests. Numbers in prose rot silently - nobody
  // re-counts them - and a map that misdescribes the territory is worse than no map,
  // because it is trusted. Pinned here so adding a block or a token pack fails until
  // the README is updated too.
  //
  // The test count was in that sentence and NOT in `claims`, so it was the one number
  // nothing checked, and it rotted to 59 against a real 67 while every pinned number
  // stayed true. A comment that says a thing is covered is not coverage. It is in the
  // list now, read off the gate's floor rather than restated here, so there is one
  // number and not two.
  const fs = require('node:fs');
  const path = require('node:path');
  const { listBlockVariants, listStylePacks } = require('../compose.js');
  const { LAYERS, fieldCount } = require('../config-schema.js');
  const { MIN_TESTS } = require('./floors.js');

  const readme = fs.readFileSync(path.join(__dirname, '..', 'README.md'), 'utf8');
  const variants = listBlockVariants();

  const claims = [
    [Object.keys(variants).length, 'block types'],
    [Object.values(variants).flat().length, 'variants'],
    [listStylePacks().length, 'style packs'],
    [fieldCount(), 'fields'],
    [LAYERS.length, 'layers'],
    [MIN_TESTS, 'tests'],
  ];
  // Filenames rot the same way counts do, and nothing pinned them: the README still said
  // `manifests/home.json` and `dist/home.html + home.css` after the build started writing
  // index.json, index.html and site.css. A quick-start command that does not run is worse
  // than no quick start, because the reader blames themselves.
  const { SITE_CSS } = require('../build.js');
  const gone = [
    ['manifests/home.json', 'the home manifest is manifests/index.json'],
    ['dist/home.html', 'the home page builds as dist/index.html'],
    ['home.css', `the site shares one ${SITE_CSS}`],
  ];
  for (const [dead, why] of gone) {
    assert.ok(!readme.includes(dead), `the README still names "${dead}" - ${why}`);
  }
  assert.ok(readme.includes(SITE_CSS), `the README never mentions ${SITE_CSS}, the stylesheet the build writes`);

  // The number must appear NEXT TO THE THING IT COUNTS, not merely somewhere in the file.
  // `\b${n}\b` against the whole README passes whenever the new value happens to occur
  // anywhere else - and with counts like 4, 9, 11 and 24 in one document, it usually does.
  // That is the presence-not-value defect this suite has now hit three times.
  for (const [n, what] of claims) {
    const near = new RegExp(`\\b${n}\\b[^\\n]{0,40}${what.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}`, 'i');
    assert.ok(
      near.test(readme),
      `the README never states "${n} ${what}". A bare \\b${n}\\b somewhere in the file ` +
      'is not a claim about this count.'
    );
  }

  // Every file the README's table names must exist.
  for (const m of readme.matchAll(/^\| `([a-z-]+\.(?:js|json|html))`/gm)) {
    const rel = m[1];
    assert.ok(
      fs.existsSync(path.join(__dirname, '..', rel)),
      `the README table names ${rel}, which does not exist`
    );
  }
});

test('states are derived from what the code conditionally renders', () => {
  // `required: []` made the states proof vacuous on every generated project: 8 rows
  // considered, 0 enforced. A proof that looks at every row and enforces none is
  // switched off, and it reports a pass.
  const fs = require('node:fs');
  const path = require('node:path');
  const { deriveStates } = require('../vds-bridge.js');

  const ROOT = path.join(__dirname, '..');
  const read = (b) => fs.readFileSync(path.join(ROOT, 'blocks', `${b}.js`), 'utf8');

  // Conditional markers ARE states.
  assert.deepEqual(deriveStates(read('formfield'), 'formfield', {}).required, ['error'],
    'field--invalid sits behind a ternary, so one component renders with and without it');
  assert.deepEqual(deriveStates(read('sidebar'), 'sidebar', {}).required, ['active']);
  assert.deepEqual(deriveStates(read('segmentedcontrol'), 'segmentedcontrol', {}).required, ['selected']);

  // Flat markers are hard-coded EXAMPLES, not states, and objectview is the case that
  // proves the rule bites: line 15 renders `aria-disabled="true"` unconditionally, so
  // that variant always draws one blocked action as an illustration. Its only real
  // state is the conditional tab marker.
  //
  // Written against objectview and NOT pagestate on purpose. `pstate--error` also looks
  // like a flat state, but `--error` is not in STATE_MARKERS at all, so asserting []
  // there passes whether the conditional filter works or not - a green light wired to
  // nothing. Removing the filter entirely left that assertion passing.
  assert.deepEqual(deriveStates(read('objectview'), 'objectview', {}).required, ['selected'],
    'a flat aria-disabled is a drawn example, not a state the component takes');
  assert.deepEqual(deriveStates(read('card'), 'card', {}).required, []);
  assert.deepEqual(deriveStates(read('toast'), 'toast', {}).required, []);

  // Only VDS's vocabulary. It REFUSES a record naming anything else and runs nothing,
  // which is how `on` and `invalid` took the whole proof offline.
  const VDS_STATES = new Set(['default', 'hover', 'focus', 'active', 'selected', 'disabled', 'loading', 'error', 'success']);
  for (const f of fs.readdirSync(path.join(ROOT, 'blocks'))) {
    const b = path.basename(f, '.js');
    for (const st of deriveStates(read(b), b, {}).required) {
      assert.ok(VDS_STATES.has(st), `block ${b} derives "${st}", which VDS's state enum does not accept`);
    }
  }
});

test('a state is only claimed drawn if the Figma evidence names a layer', () => {
  const fs = require('node:fs');
  const path = require('node:path');
  const { deriveStates } = require('../vds-bridge.js');

  const ROOT = path.join(__dirname, '..');
  const evidence = JSON.parse(fs.readFileSync(path.join(ROOT, 'figma-states.json'), 'utf8'));

  // Nothing may be claimed drawn for a state the code does not require: that would be a
  // measurement of a component that does not exist.
  for (const [block, states] of Object.entries(evidence.drawn)) {
    const src = fs.readFileSync(path.join(ROOT, 'blocks', `${block}.js`), 'utf8');
    const required = deriveStates(src, block, {}).required;
    for (const st of Object.keys(states)) {
      assert.ok(required.includes(st),
        `figma-states.json claims ${block}/${st} is drawn, but ${block}.js never renders it`);
      // Non-empty is not a layer name. The value being waved through was
      // "That reference does not exist." - an ERROR MESSAGE stored where a layer name
      // belongs. `figma-states.json` is the evidence for a claim the states proof gates,
      // so the citation has to be a thing you can find in the file.
      const cited = String(states[st]).trim();
      assert.ok(cited.length > 0, `${block}/${st} is claimed drawn with no layer cited`);
      assert.ok(
        /^(spec:|state:)/.test(cited),
        `${block}/${st} cites "${cited}" as its evidence. A layer name must start with ` +
        'spec: or state: - a sentence is a description, and a description cannot be looked up.'
      );
    }
  }

  // With no evidence file at all, every state reports NOT drawn. Understating is safe;
  // a bridge that claimed drawn because it could not reach Figma would be the worst case.
  assert.deepEqual(deriveStates(fs.readFileSync(path.join(ROOT, 'blocks', 'sidebar.js'), 'utf8'), 'sidebar', {}).drawn, []);
});

test('the VDS surface points at paths a scaffolded project actually has', () => {
  // Renaming home.css to site.css took the `contrast` proof offline. It REFUSED and ran
  // nothing, which is the right behaviour and the reason the break was findable at all:
  // "a caller told that every boundary clears its floor, about a stylesheet that was
  // never opened, has been told nothing." A proof that had instead skipped its rows would
  // have reported a pass over a file that does not exist.
  //
  // So the declared surface is checked against the build, not against a memory of it.
  const path = require('node:path');
  const { SURFACE } = require('../vds-bridge.js');
  const { SITE_CSS } = require('../build.js');

  assert.equal(SURFACE.stylesheet, `"dist/${SITE_CSS}"`,
    'the surface names a stylesheet the build does not write');

  // Every path in the surface must be one a project has. Globs are checked by their
  // literal directory prefix, which is the part that either exists or does not.
  const dirs = [
    ['library_dirs', 'blocks'],
    ['screen_globs', 'manifests'],
  ];
  for (const [key, dir] of dirs) {
    assert.match(SURFACE[key], new RegExp(dir), `${key} no longer names ${dir}/`);
  }
});

test('the studio can edit every field type the schema declares', () => {
  // `pages` was declared as `page-list` and the studio had no branch for it, so it fell
  // through to the text input: it rendered "[object Object],[object Object]" and the
  // first edit replaced the page array with that string. A field the editor cannot edit
  // is a control that lies; one that corrupts the config is worse.
  //
  // The fall-through `else` is what makes this invisible - every unhandled type gets a
  // text box that looks deliberate. So the types are compared, not eyeballed.
  const fs = require('node:fs');
  const path = require('node:path');
  const { LAYERS } = require('../config-schema.js');

  const html = fs.readFileSync(path.join(__dirname, '..', 'studio.html'), 'utf8');
  // Any receiver, not just `f`: the enum branch lives in a helper whose parameter is
  // named `field`, so a pattern hard-coded to `f.type` reported enum as unhandled. The
  // test failed for the wrong reason, which is its own kind of wrong.
  const handled = new Set([...html.matchAll(/\b\w+\.type === '([a-z-]+)'/g)].map((m) => m[1]));
  // `text` is the fall-through and needs no branch: a text box IS the right editor for it.
  handled.add('text');

  const declared = new Set(LAYERS.flatMap((l) => (l.fields || []).map((f) => f.type)).filter(Boolean));
  const unhandled = [...declared].filter((t) => !handled.has(t));
  assert.deepEqual(unhandled, [],
    `the schema declares field types the studio has no editor for: ${unhandled.join(', ')}`);
});

test('no spec sheet row prints a default-stringified object', () => {
  // "[object Object]" is the tell that a value arrived which the renderer has no idea how
  // to describe, and printed something that looks like output anyway. The spec sheet said
  // exactly that next to `pages`, which is a design decision - a sheet that looks
  // authoritative and says nothing is the failure figma-spec.js exists to avoid.
  //
  // The studio had the identical bug and the test written for it could NOT see this one,
  // because it only read studio.html. Two renderers consume the schema; checking one is
  // checking half.
  const { buildSpec, describe } = require('../figma-spec.js');
  const { suggest } = require('../suggest.js');

  for (const route of ['marketing-site', 'saas-app']) {
    const cfg = suggest({ name: 'Probe', category: route, description: 'A site for a law firm.' });
    const spec = buildSpec(cfg);
    for (const section of spec.sections) {
      for (const row of section.rows || []) {
        assert.ok(!/\[object Object\]/.test(String(row.chosen)),
          `${route} ${section.title}/${row.key} prints a stringified object: ${row.chosen}`);
        for (const opt of row.options || []) {
          assert.ok(!/\[object Object\]/.test(String(opt.label)),
            `${route} ${section.title}/${row.key} has an option labelled with a stringified object`);
        }
      }
    }
  }

  // describe() is the guard, so it is tested directly on the shapes that broke it.
  assert.equal(describe([{ slug: 'home' }, { slug: '404' }]), 'home, 404');
  assert.equal(describe({ nav: true }), '{nav}', 'an object with no identifying key names its keys rather than stringifying');
  assert.equal(describe(null), '');
  assert.equal(describe(['a', 'b']), 'a, b');
});

test('every field the schema declares reaches the spec sheet as a real specimen', () => {
  // A field with no `case` in specimensFor falls to the text default. That is correct for
  // some fields and wrong for any field whose VALUE is not a string - which is how `pages`
  // ended up stringified. Pinned by shape rather than by a hand-kept list of exceptions.
  const { LAYERS } = require('../config-schema.js');
  const { buildSpec } = require('../figma-spec.js');
  const { suggest } = require('../suggest.js');

  const cfg = suggest({ name: 'Probe', category: 'marketing-site', description: 'A site for a law firm.' });
  const spec = buildSpec(cfg);
  const rows = new Map(spec.sections.flatMap((s) => (s.rows || []).map((r) => [r.key, r])));

  for (const layer of LAYERS) {
    if (layer.key === 'identity') continue;
    for (const field of layer.fields) {
      const row = rows.get(field.key);
      assert.ok(row, `the schema declares ${layer.key}.${field.key} and the spec sheet has no row for it`);
      const value = (cfg[layer.key] || {})[field.key];
      const isStructured = Array.isArray(value) && value.some((v) => v && typeof v === 'object');
      if (isStructured) {
        assert.notEqual(row.kind, 'text',
          `${field.key} holds objects, so a text row would stringify them - it needs its own specimen`);
      }
    }
  }
});
