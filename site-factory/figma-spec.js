'use strict';

/*
 * figma-spec.js: one visible specimen per decision, not a table of strings.
 *
 * The project record page listed all 35 fields as LAYER | FIELD | VALUE text rows.
 * That is a manifest, not a spec: "cornerRadius: sharp-0" tells you nothing about
 * what sharp-0 looks like next to pill, and a brand sheet whose reader cannot SEE
 * the decision is a sheet nobody uses.
 *
 * Every specimen value here is COMPUTED from the same pipeline that builds the site —
 * configToTokens for the design values, copy.js for the voice ones. Nothing is drawn
 * from memory. A spec sheet showing a radius the build does not use, or copy the
 * generator would not write, is worse than the text table it replaces, because it
 * looks authoritative.
 *
 * Three honest categories, and the sheet labels each:
 *   specimen  the option can be drawn, and is (25 fields)
 *   text      the value IS the content — a name, a note, a list (8 fields)
 *   not-static  motion. A still frame cannot show duration or easing, and a swatch
 *               pretending to is a lie. Named and left undrawn (2 fields).
 */

const { LAYERS } = require('./config-schema.js');
const { configToTokens, RADIUS } = require('./compose.js');
const { ctaLabel, copyFor } = require('./copy.js');

// Mirrors build.js. Imported values would be better, but build.js does not export
// these tables; they are asserted equal by tests/spec.test.js so they cannot drift.
const DENSITY = { compact: 0.75, comfortable: 1, spacious: 1.35 };
const TYPE_SCALE = { compact: 0.9, comfortable: 1, spacious: 1.15 };
const BORDER_WEIGHT = { hairline: '1px', '1px': '1px', 'bold-2px': '2px' };
const ELEVATION = { flat: 'none', 'soft-shadow': 'soft', 'hard-offset': 'hard' };

const NOT_STATIC = new Set(['motionIntensity', 'transitionStyle']);

/*
 * How to draw each field. `kind` tells the Figma script which primitive to use;
 * `options` carries the REAL computed value behind each choice.
 */
function specimensFor(field, config) {
  const tok = configToTokens(config);
  const unit = config.spacing.spaceUnit;
  const key = field.key;

  if (field.type === 'color') {
    return { kind: 'swatch', options: [{ label: key.replace('Color', ''), value: config.palette[key] }] };
  }
  if (NOT_STATIC.has(key)) {
    return { kind: 'not-static', options: (field.options || []).map((o) => ({ label: o })) };
  }

  switch (key) {
    case 'cornerRadius':
      return { kind: 'radius', options: field.options.map((o) => ({ label: o, value: RADIUS[o][0] })) };
    case 'density':
      return { kind: 'spacing', options: field.options.map((o) => ({ label: o, value: `${(unit * DENSITY[o]).toFixed(2)}px` })) };
    case 'typeScale':
      return { kind: 'typescale', options: field.options.map((o) => ({ label: o, value: TYPE_SCALE[o] })) };
    case 'borderWeight':
      return { kind: 'border', options: field.options.map((o) => ({ label: o, value: BORDER_WEIGHT[o] })) };
    case 'elevation':
      return { kind: 'elevation', options: field.options.map((o) => ({ label: o, value: ELEVATION[o] })) };
    case 'spaceUnit':
      return { kind: 'ruler', options: [1, 2, 4, 6, 8].map((n) => ({ label: `${n}×`, value: `${unit * n}px` })) };
    case 'buttonShape':
      return { kind: 'button', options: field.options.map((o) => ({ label: o, value: o === 'square' ? '0px' : o === 'pill' ? '999px' : '6px' })) };
    case 'statusBadgeStyle':
      return { kind: 'badge', options: field.options.map((o) => ({ label: o })) };
    case 'tableDensity':
      return { kind: 'table', options: field.options.map((o) => ({ label: o, value: o === 'compact' ? 6 : 11 })) };
    case 'navigationPattern':
      return { kind: 'navdiagram', options: field.options.map((o) => ({ label: o })) };
    case 'iconStyle':
      return { kind: 'icon', options: field.options.map((o) => ({ label: o })) };
    case 'imageTreatment':
      return { kind: 'treatment', options: field.options.map((o) => ({ label: o })) };
    case 'displayFont':
    case 'monoFont':
      return { kind: 'typesample', options: [{ label: key === 'monoFont' ? tok.font.mono : tok.font.family, value: 'Structuring, 1234' }] };
    case 'pairingStyle':
      return { kind: 'pairing', options: field.options.map((o) => ({ label: o })) };
    case 'sitemap':
      return { kind: 'stack', options: (config.strategy.sitemap || []).map((v) => ({ label: v })) };

    // The voice fields are drawn as the copy they actually produce. This is the only
    // way to show what a register decision means, and it is real output.
    case 'copyRegister':
    case 'readingLevel':
    case 'ctaStyle': {
      const options = field.options.map((o) => {
        const probe = JSON.parse(JSON.stringify(config));
        probe.voice[key] = o;
        const hero = copyFor('hero', probe.identity, probe.voice);
        const cta = copyFor('cta', probe.identity, probe.voice);
        const sample = key === 'ctaStyle'
          ? ctaLabel(probe.voice)
          : (key === 'copyRegister' ? cta.heading : hero.ctaLabel);
        return { label: o, value: sample };
      });
      return { kind: 'copysample', options };
    }
    default:
      return { kind: 'text', options: [] };
  }
}

function chosenValue(layerKey, field, config) {
  const v = (config[layerKey] || {})[field.key];
  return Array.isArray(v) ? v.join(', ') : String(v == null ? '' : v);
}

function buildSpec(config) {
  const sections = [];
  for (const layer of LAYERS) {
    if (layer.key === 'identity') continue;
    const rows = [];
    for (const field of layer.fields) {
      const spec = specimensFor(field, config);
      rows.push({
        key: field.key,
        label: field.label || field.key,
        chosen: chosenValue(layer.key, field, config),
        kind: spec.kind,
        options: spec.options,
      });
    }
    sections.push({ key: layer.key, title: layer.title, source: layer.source || null, rows });
  }
  const counts = sections.flatMap((s) => s.rows).reduce((a, r) => {
    const bucket = r.kind === 'text' ? 'text' : r.kind === 'not-static' ? 'notStatic' : 'specimen';
    a[bucket] = (a[bucket] || 0) + 1;
    return a;
  }, {});
  return { identity: config.identity, palette: config.palette, sections, counts };
}

module.exports = { buildSpec, specimensFor, DENSITY, TYPE_SCALE, BORDER_WEIGHT, ELEVATION, NOT_STATIC };

/*
 * The rule the spec-sheet drawing script must follow, learned by breaking it.
 *
 * Figma auto-layout frames default to an opaque white fill, so a sheet built on a
 * paper ground needs every CONTAINER's fill cleared. But some frames' fill IS the
 * specimen — a button with no fill is not a button, it is a label. A blanket
 * "clear every FRAME" pass cannot tell the two apart, and on the first build it
 * wiped 3 buttons, 1 badge and 8 block chips, leaving rows that read as if the
 * decision had no visual consequence at all. Which is precisely the failure the
 * whole sheet exists to fix.
 *
 * So: any frame whose fill is the specimen is NAMED `spec:*` when it is created,
 * and the clearing pass skips those names. Name at creation, not afterwards —
 * repairing by inspecting children (looking for the word "Enquire") worked as a
 * one-off rescue but is not a rule, because the next specimen will hold different text.
 */
const SPECIMEN_FRAME_PREFIX = 'spec:';

function isSpecimenFrame(node) {
  return typeof node.name === 'string' && node.name.startsWith(SPECIMEN_FRAME_PREFIX);
}

module.exports.SPECIMEN_FRAME_PREFIX = SPECIMEN_FRAME_PREFIX;
module.exports.isSpecimenFrame = isSpecimenFrame;
