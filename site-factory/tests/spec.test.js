'use strict';

/*
 * The spec sheet must show what the build actually does.
 *
 * figma-spec.js keeps its own copies of DENSITY, TYPE_SCALE, BORDER_WEIGHT and the
 * elevation set because build.js does not export them. Duplicated tables drift, and
 * a spec sheet drawing a spacing value the CSS does not use is worse than the text
 * table it replaced — it looks authoritative and it is wrong. These tests pin the
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
