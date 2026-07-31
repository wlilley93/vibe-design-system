'use strict';

/*
 * The writing brief must point at skills that exist, and hand them a config in the
 * shape they actually read.
 *
 * A brief naming `revenue/skills/headline-lab` when that folder has been renamed is
 * exactly the plausible-but-wrong artefact this project keeps guarding against: it
 * reads as a work order and resolves to nothing. So the paths are checked against
 * the real library.
 *
 * The library lives outside this repo, so its absence is a SKIP, not a failure - a
 * machine without agents-final has not broken anything. But when it IS present, a
 * missing skill is a hard failure, because then the reference really is dead.
 */

const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const { suggest } = require('../suggest.js');
const { configToManifest } = require('../compose.js');
const { auditCopy, BANNED } = require('../copy.js');
const { packConfig, blankFields, assignments, briefMarkdown, BLOCK_SKILLS, RUN_ORDER } = require('../skills.js');

const LIBRARY = path.join(os.homedir(), 'Documents', 'agents-final', 'skill-library');
const havelib = fs.existsSync(LIBRARY);

function cfg() {
  const identity = {
    name: 'Northgate Trust',
    tagline: 'I draw the structure first',
    category: 'marketing-site',
    description: 'A private trust and estate structuring advisory',
  };
  const c = suggest(identity);
  c.identity = identity;
  c.governance = { vds: false };
  return c;
}

test('every skill the brief can name exists in the library', { skip: havelib ? false : 'agents-final not present on this machine' }, () => {
  const named = new Set([...Object.values(BLOCK_SKILLS).map((s) => s.skill), ...RUN_ORDER]);
  for (const rel of named) {
    assert.ok(
      fs.existsSync(path.join(LIBRARY, rel, 'SKILL.md')),
      `the brief names ${rel}, which has no SKILL.md - a work order pointing at nothing`
    );
  }
});

test('the pack config uses the field names the skills read', () => {
  // Copied from revenue/skills/sales-page-setup's own JSON block. A near-miss name
  // makes every downstream skill fall back to interviewing, silently.
  const pc = packConfig(cfg());
  const expected = {
    brand: ['brand_name', 'website', 'niche', 'audience', 'offer', 'transformation', 'price_point'],
    voice: ['tone_words', 'we_sound_like', 'we_never_say', 'reading_level'],
    market: ['awareness_level', 'biggest_objection', 'main_alternative'],
    proof: ['has_testimonials', 'key_results', 'credentials'],
    tools: ['design', 'ai_media', 'planner', 'email'],
    preferences: ['page_length', 'save_outputs_to'],
  };
  for (const [group, keys] of Object.entries(expected)) {
    assert.ok(pc[group], `config is missing the "${group}" group the skills read`);
    for (const k of keys) {
      assert.ok(k in pc[group], `config.${group} is missing "${k}"`);
    }
  }
});

test('what site-factory genuinely knows is filled; what it cannot know is left blank', () => {
  const pc = packConfig(cfg());
  assert.equal(pc.brand.brand_name, 'Northgate Trust');
  assert.match(pc.brand.offer, /trust and estate/);
  assert.match(pc.brand.transformation, /draw the structure/);
  assert.ok(pc.voice.tone_words.length > 0, 'the copy register should yield tone words');

  // site-factory describes a SITE, not a market. Guessing these would poison every
  // skill downstream, because each treats config.json as settled fact.
  const blanks = blankFields(pc);
  for (const mustBeBlank of ['brand.audience', 'market.awareness_level', 'market.biggest_objection', 'market.main_alternative']) {
    assert.ok(blanks.includes(mustBeBlank), `${mustBeBlank} was invented rather than left for the setup interview`);
  }
});

test('the never-say list is the same constraint the copy tests enforce', () => {
  // One source. If these drifted, the pack would be told a different rule from the
  // one tests/copy.test.js actually checks.
  const pc = packConfig(cfg());
  for (const word of BANNED) {
    assert.ok(pc.voice.we_never_say.includes(word), `"${word}" is banned in tests but not declared to the skills`);
  }
});

test('every unwritten line is assigned to a skill, or explicitly unassigned', () => {
  const c = cfg();
  const m = configToManifest(c);
  const gaps = auditCopy(m);
  const { bySkill, unassigned } = assignments(m, gaps);

  const assigned = bySkill.reduce((n, s) => n + s.gaps.length, 0);
  assert.equal(
    assigned + unassigned.length, gaps.length,
    'a gap vanished between the audit and the brief - every line must be accounted for'
  );
  assert.ok(bySkill.length > 0, 'no skill was assigned any work');
});

test('the brief lists skills in the order the pack declares, not block order', () => {
  const c = cfg();
  const m = configToManifest(c);
  const { bySkill } = assignments(m, auditCopy(m));
  const positions = bySkill.map((s) => RUN_ORDER.indexOf(s.skill));
  const sorted = [...positions].sort((a, b) => a - b);
  assert.deepEqual(positions, sorted, 'the offer must be clear before anything is written about it');
});

test('the brief names its blanks so they can be filled before any skill runs', () => {
  const c = cfg();
  const m = configToManifest(c);
  const md = briefMarkdown(c, m, auditCopy(m));
  assert.match(md, /sales-page-setup/, 'the brief must say how to fill the blanks');
  assert.match(md, /market\.awareness_level/, 'the brief must name the specific blank fields');
  assert.match(md, /headline-lab/,
    'headline-lab must be offered even when the hero already has a line - a brief that only ' +
    'lists blanks never improves copy that exists');
});

test('a block with copy is still offered its skill, for improvement not gap-filling', () => {
  const c = cfg();
  const m = configToManifest(c);
  const { bySkill, improvable } = assignments(m, auditCopy(m));
  // The hero has a tagline, so it has no gaps and no assignment...
  assert.ok(!bySkill.some((s) => s.skill.includes('headline-lab')), 'hero should have no unwritten lines here');
  // ...but headline-lab generates and scores alternatives, which is worth running anyway.
  assert.ok(improvable.some((s) => s.skill.includes('headline-lab')), 'headline-lab was never offered');
});

test('every block type is paired with a Figma component set', () => {
  // The bank is code AND Figma. Adding a block type without its component set divides
  // it silently: the VDS register record gets `figma: null`, no gate complains, and the
  // divergence is only found by someone eyeballing the file. I did exactly that when
  // the four demand-measured blocks landed code-only, so it is a test now.
  const { FIGMA_NODES } = require('../vds-bridge.js');
  const { listBlockVariants } = require('../compose.js');

  const missing = Object.keys(listBlockVariants()).filter((t) => !FIGMA_NODES[t]);
  assert.deepEqual(
    missing, [],
    `these block types ship in code with no Figma component set: ${missing.join(', ')}. ` +
    'Build the set, then add its node id to FIGMA_NODES.'
  );
});

test('no Figma node id is claimed for a block type that does not exist', () => {
  // The other direction: a node id left behind after a block is renamed or removed
  // points a register record at a component nobody can find.
  const { FIGMA_NODES } = require('../vds-bridge.js');
  const { listBlockVariants } = require('../compose.js');
  const types = new Set(Object.keys(listBlockVariants()));

  const orphans = Object.keys(FIGMA_NODES).filter((t) => !types.has(t));
  assert.deepEqual(orphans, [], `FIGMA_NODES names block types that no longer exist: ${orphans.join(', ')}`);

  for (const [type, id] of Object.entries(FIGMA_NODES)) {
    assert.match(id, /^\d+:\d+$/, `${type} has a malformed Figma node id: ${id}`);
  }
});

test('every paired Figma node exists, and every drawn set is paired or declared unpaired', () => {
  // The two tests above check that a block type HAS an id and that the id LOOKS like an
  // id. Neither opens the file, so a deleted component set failed nothing.
  //
  // This test was written on the strength of a finding that turned out to be WRONG, and the
  // correction is recorded in figma-nodes.json under _correction. A survey read p.children on
  // every page and reported seven of ten pages EMPTY; I read that as a probable wipe. The file
  // runs in Figma's dynamic-page mode, where a page's children are not available until the
  // page is loaded, and the survey never awaited p.loadAsync() - so the pages that reported
  // content were simply the ones already loaded. Re-measured properly: 12 pages, 42 sets, 20
  // frames, nothing missing. A missing await manufactured a data-loss incident.
  //
  // The test stays, because the hole it closes is real whether or not anything has fallen
  // through it yet: an id-shape check cannot tell a live pairing from a dangling one.
  //
  // So the pairing is now against a MEASURED read of the file. It is a snapshot, and a
  // snapshot cannot see a deletion that happens after it is taken - the honest limit, and
  // the same one figma-variables.json has. What it does catch is a pairing that never
  // resolved and a set drawn with nothing pointing at it.
  const fs = require('node:fs');
  const path = require('node:path');
  const { FIGMA_NODES, FIGMA_FILE_KEY } = require('../vds-bridge.js');
  const measured = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'figma-nodes.json'), 'utf8'));

  assert.equal(measured.file_key, FIGMA_FILE_KEY,
    'the node measurement was taken from a different file than the bridge writes records against');

  // Direction one: every id a block type points at must be a set that was actually there.
  const dangling = Object.entries(FIGMA_NODES)
    .filter(([, id]) => !measured.sets[id])
    .map(([type, id]) => `${type} -> ${id}`);
  assert.deepEqual(dangling, [],
    `these block types point at a Figma node that does not exist in the measured file: ${dangling.join(', ')}. ` +
    'Either the set was deleted or the id was never right.');

  // Direction two: every set drawn in the file is either paired to a block type or listed
  // as deliberately unpaired WITH A REASON. A drawing nothing points at is either work in
  // progress or work forgotten, and the difference has to be written down by someone.
  const paired = new Set(Object.values(FIGMA_NODES));
  const orphaned = Object.keys(measured.sets)
    .filter((id) => !paired.has(id) && !measured.unpaired[id])
    .map((id) => `${id} (${measured.sets[id].name} on ${measured.sets[id].page})`);
  assert.deepEqual(orphaned, [],
    `these component sets are drawn and nothing points at them, and they are not declared ` +
    `unpaired: ${orphaned.join(', ')}`);

  for (const [id, why] of Object.entries(measured.unpaired)) {
    assert.ok(measured.sets[id], `unpaired declares ${id}, which is not a set in the file`);
    assert.ok(!paired.has(id), `${id} is declared unpaired and is also paired to a block type`);
    assert.ok(why && why.length > 20, `${id} is declared unpaired with no usable reason`);
  }

  // A set with one variant is a set that lost its siblings. Every set in this file was
  // built with at least two, so this catches a partial wipe rather than a total one -
  // which is the failure mode a whole-page check would miss.
  const thin = Object.entries(measured.sets)
    .filter(([, s]) => s.variants < 2)
    .map(([id, s]) => `${id} ${s.name} (${s.variants})`);
  assert.deepEqual(thin, [], `these component sets have fewer than two variants: ${thin.join(', ')}`);
});

// Shared by the three variable-pairing tests below. Reads the MEASURED collection and
// the packs it claims to mirror; asserts nothing on its own.
function figmaVariablePairing() {
  const fs = require('node:fs');
  const path = require('node:path');
  const { cssVars } = require('../build.js');
  const {
    FIGMA_VARIABLE_MODES, FIGMA_UNBOUND_PACKS, FIGMA_VARIABLE_PREFIXES, FIGMA_FILE_KEY,
  } = require('../vds-bridge.js');

  const root = path.join(__dirname, '..');
  const measured = JSON.parse(fs.readFileSync(path.join(root, 'figma-variables.json'), 'utf8'));

  // A Figma variable is `color/accentInk`; the emitted property is `--color-accentInk`.
  // One naming convention per side, converted in exactly one place.
  const toProperty = (varName) => `--${varName.replace('/', '-')}`;

  const packProperties = (pack) => {
    const tokens = JSON.parse(fs.readFileSync(path.join(root, 'tokens', `${pack}.json`), 'utf8'));
    tokens.scale = { density: 'comfortable', type: 'comfortable' };
    const out = {};
    for (const line of cssVars(tokens).split('\n')) {
      const m = line.match(/^\s*(--[a-zA-Z-]+):\s*([^;]+);/);
      if (!m) continue;
      if (!FIGMA_VARIABLE_PREFIXES.some((p) => m[1].startsWith(p))) continue;
      out[m[1]] = m[2].trim();
    }
    return out;
  };

  return {
    measured, toProperty, packProperties, FIGMA_FILE_KEY,
    FIGMA_VARIABLE_MODES, FIGMA_UNBOUND_PACKS, FIGMA_VARIABLE_PREFIXES,
  };
}

test('the measured Figma collection describes the file the bridge names, and every mode is a real pack', () => {
  const fs = require('node:fs');
  const path = require('node:path');
  const p = figmaVariablePairing();

  assert.equal(p.measured.file_key, p.FIGMA_FILE_KEY,
    'the measurement was taken from a different file than the one the bridge writes register records against');
  assert.deepEqual(
    p.measured.modes.slice().sort(), Object.keys(p.FIGMA_VARIABLE_MODES).sort(),
    'the collection\'s modes and the declared mode->pack map disagree'
  );

  for (const [mode, pack] of Object.entries(p.FIGMA_VARIABLE_MODES)) {
    assert.ok(fs.existsSync(path.join(__dirname, '..', 'tokens', `${pack}.json`)),
      `mode ${mode} claims to mirror tokens/${pack}.json, which does not exist`);
  }

  // Every pack is either bound to a mode or declared unbound with a reason. A pack in
  // neither list is a pack nothing checks, and it would sit there indefinitely.
  const packs = fs.readdirSync(path.join(__dirname, '..', 'tokens'))
    .filter((f) => f.endsWith('.json')).map((f) => f.replace(/\.json$/, ''));
  const bound = new Set(Object.values(p.FIGMA_VARIABLE_MODES));
  const unaccounted = packs.filter((k) => !bound.has(k) && !p.FIGMA_UNBOUND_PACKS[k]);
  assert.deepEqual(unaccounted, [],
    `these style packs are neither bound to a Figma mode nor declared unbound: ${unaccounted.join(', ')}`);
  for (const [pack, why] of Object.entries(p.FIGMA_UNBOUND_PACKS)) {
    assert.ok(packs.includes(pack), `FIGMA_UNBOUND_PACKS names a pack that does not exist: ${pack}`);
    assert.ok(why && why.length > 10, `${pack} is declared unbound with no usable reason`);
  }
});

test('every colour and radius the build emits has a Figma variable, and every variable is emitted', () => {
  // This is the direction that was missing, and its absence was not theoretical: four
  // ink variables (dangerInk, warningInk, successInk, infoInk) existed in every style
  // pack and in no Figma mode. A builder script that filtered tones to those whose
  // variables resolved therefore drew one tone of four and returned success.
  const p = figmaVariablePairing();
  const inFigma = new Set(Object.keys(p.measured.variables).map(p.toProperty));

  for (const [mode, pack] of Object.entries(p.FIGMA_VARIABLE_MODES)) {
    const emitted = Object.keys(p.packProperties(pack));
    const absent = emitted.filter((prop) => !inFigma.has(prop));
    assert.deepEqual(absent, [],
      `tokens/${pack}.json (Figma mode ${mode}) emits these with no Figma variable: ${absent.join(', ')}. ` +
      'A drawing cannot bind what the collection does not carry.');
  }

  // The reverse: a variable naming a property no pack emits is one a drawing can bind
  // to and the CSS will never define, which reads as a working binding.
  const anyPack = p.packProperties(Object.values(p.FIGMA_VARIABLE_MODES)[0]);
  const orphans = [...inFigma].filter((prop) => !(prop in anyPack));
  assert.deepEqual(orphans, [],
    `these Figma variables name custom properties the build does not emit: ${orphans.join(', ')}`);
});

test('every Figma mode value equals the style pack that defines it', () => {
  // The drift this catches is silent by construction. The Base palette measurement
  // landed in tokens/*.json and never reached Figma, so the Geist mode's danger was
  // #fc0035 while the shipped CSS said #de1135 - components drawn in Figma were a
  // different red from the ones the factory builds, and both looked deliberate.
  const p = figmaVariablePairing();

  // A Figma FLOAT radius is 6; the emitted property is `6px`. Compare in one unit.
  const normalise = (value) => (typeof value === 'number' ? `${value}px` : String(value).toLowerCase());

  const drifted = [];
  for (const [mode, pack] of Object.entries(p.FIGMA_VARIABLE_MODES)) {
    const emitted = p.packProperties(pack);
    for (const [varName, byMode] of Object.entries(p.measured.variables)) {
      const prop = p.toProperty(varName);
      if (!(prop in emitted)) continue; // absence is the previous test's finding, not this one's
      const figma = normalise(byMode[mode]);
      const code = normalise(emitted[prop]);
      if (figma !== code) drifted.push(`${mode}/${varName}: figma=${figma} code=${code}`);
    }
  }

  assert.deepEqual(drifted, [],
    `${drifted.length} Figma variable values disagree with the pack that defines them:\n  ` +
    `${drifted.join('\n  ')}\n` +
    'The pack is the source of truth. Fix Figma, then re-measure figma-variables.json.');
});

test('the route is inferred from the brief, and marketing wins over app words', () => {
  // `category` used to default to marketing-site with nothing in the brief able to move
  // it, so `factory.js new --brief "a matter-management app"` built a hero and a pricing
  // table. The saas route was unreachable from the one-shot path.
  const { inferRoute } = require('../suggest.js');

  assert.equal(inferRoute('A matter-management app for boutique law firms'), 'saas-app');
  assert.equal(inferRoute('An internal dashboard for the ops team'), 'saas-app');
  assert.equal(inferRoute('A marketing site for our analytics dashboard'), 'marketing-site',
    'both vocabularies present - it is a marketing site, the app words describe the product being sold');
  assert.equal(inferRoute('A landing page for our SaaS platform'), 'marketing-site');
  assert.equal(inferRoute('Northgate Trust, an advisory firm'), null,
    'nothing said either way must return null so the caller keeps its own default');
});

test('an explicit route beats inference', () => {
  const { suggest } = require('../suggest.js');
  const brief = { name: 'X', description: 'A dashboard for ops teams' };
  assert.equal(suggest(brief).identity.category, 'saas-app');
  assert.equal(suggest({ ...brief, category: 'marketing-site' }).identity.category, 'marketing-site');
});

test('the copy brief carries what the brief literally said, and invents nothing', () => {
  const { extractFromBrief } = require('../skills.js');

  const got = extractFromBrief('A matter-management app for boutique law firms. Replaces spreadsheets and email chains.');
  assert.equal(got.audience, 'boutique law firms');
  assert.equal(got.main_alternative, 'spreadsheets and email chains');

  // The negative control, and the one that matters: a brief that states neither must
  // yield neither. An extractor that produces a plausible value from silence is worse
  // than no extractor, because the interview then never asks.
  const silent = extractFromBrief('Northgate Trust. Established 1974.');
  assert.deepEqual(silent, {}, `invented a value from a brief that said nothing: ${JSON.stringify(silent)}`);
  assert.deepEqual(extractFromBrief(''), {});
});

test('a CONFIRM-marked field still counts as unsettled', () => {
  // The undercount auditCopy already had once: a field with a value passes a plain
  // emptiness test, so the brief would report fewer open fields than there are.
  const { suggest } = require('../suggest.js');
  const { packConfig, blankFields, unsettledFields } = require('../skills.js');

  const cfg = suggest({ name: 'Atlas Ops', description: 'A matter-management app for boutique law firms. Replaces spreadsheets and email chains.' });
  const pc = packConfig(cfg);

  assert.match(pc.brand.audience, /CONFIRM:/);
  assert.match(pc.market.main_alternative, /CONFIRM:/);

  const open = blankFields(pc);
  assert.ok(open.includes('brand.audience'), 'a CONFIRM-marked audience must still be reported as open');
  assert.ok(open.includes('market.main_alternative'));

  const { blank, toConfirm } = unsettledFields(pc);
  assert.equal(blank.length + toConfirm.length, open.length, 'the split must account for every open field');
  assert.equal(toConfirm.length, 2);
  assert.ok(blank.every((f) => !/CONFIRM:/.test(f.value)));
});

test('the writing brief shows confirm-these and ask-these separately', () => {
  const { suggest } = require('../suggest.js');
  const { briefMarkdown } = require('../skills.js');
  const { configToManifest } = require('../compose.js');
  const { auditCopy } = require('../copy.js');

  const cfg = suggest({ name: 'Atlas Ops', category: 'marketing-site', description: 'A matter-management app for boutique law firms. Replaces spreadsheets and email chains.' });
  const manifest = configToManifest(cfg);
  const md = briefMarkdown(cfg, manifest, auditCopy(manifest));

  assert.match(md, /need CONFIRMING, not asking from scratch/);
  assert.match(md, /boutique law firms/, 'the brief must quote back what the author already said');
  assert.match(md, /genuinely blank/);
});

test('a long extracted span is cut at a word, not through one', () => {
  // `{2,80}` clipped "reverse-engineer" to "reverse-enginee". A quote the author never
  // wrote is the same failure as an invented value, just harder to spot.
  const { extractFromBrief } = require('../skills.js');

  const long = 'Instead of a page builder that hides its choices, or a template you have to reverse-engineer yourself.';
  const got = extractFromBrief(long).main_alternative;

  assert.ok(got.length <= 82, `span not capped: ${got.length} chars`);
  assert.ok(got.endsWith('…'), 'a cut span must show that it was cut');
  const body = got.slice(0, -1);
  assert.ok(long.includes(body), `the kept text is not a substring of the brief: ${body}`);
  assert.ok(!/\breverse-enginee$/.test(body), 'cut through the middle of a word');

  // A span that fits is returned whole, with no ellipsis.
  const short = extractFromBrief('Replaces spreadsheets.').main_alternative;
  assert.equal(short, 'spreadsheets');
});

test('every Uber Base set has exactly one tier, and the tiers reconcile with the harvest', () => {
  // BASELINE.md states the tier counts in prose. This is the machine-readable half, and it
  // exists because parsing the prose was tried first and LEAKED: a substring match put
  // "Message card" in tier 2, since "Message card - Carousel" is in tier 2 and contains it,
  // and the deferred tier is described by category ("every Date picker and Time picker set")
  // rather than by set name, so it matched nothing at all. A document written for a reader
  // is not a data source, and treating it as one produces an assignment that looks derived.
  const fs = require('node:fs');
  const path = require('node:path');
  const vendor = path.join(__dirname, '..', 'vendor');
  const harvest = JSON.parse(fs.readFileSync(path.join(vendor, 'uber-base-keys.json'), 'utf8'));
  const tiers = JSON.parse(fs.readFileSync(path.join(vendor, 'uber-base-tiers.json'), 'utf8'));

  // The tier file must describe the harvest, not a subset of it and not a superset.
  const harvestNames = Object.keys(harvest.sets).sort();
  const tierNames = Object.keys(tiers.sets).sort();
  assert.deepEqual(tierNames, harvestNames,
    'the tier assignment and the harvested set list disagree about which sets exist');

  const TIERS = new Set(['tier1', 'tier2', 'deferred', 'answered']);
  const bad = [];
  for (const [name, row] of Object.entries(tiers.sets)) {
    if (!TIERS.has(row.tier)) bad.push(`${name}: tier "${row.tier}"`);
    // The variant count and key must come from the harvest, not be restated. A second copy
    // of a measured number is a second place for it to rot.
    if (row.variants !== harvest.sets[name].variants) {
      bad.push(`${name}: variants ${row.variants} against harvested ${harvest.sets[name].variants}`);
    }
    if (row.key !== harvest.sets[name].key) bad.push(`${name}: key disagrees with the harvest`);
    // `answered` is the only tier that names a block, and it MUST name one - otherwise
    // "already covered" is a claim with nothing behind it.
    if (row.tier === 'answered' && !row.answeredBy) bad.push(`${name}: answered by nothing`);
    if (row.tier !== 'answered' && row.answeredBy) bad.push(`${name}: not answered but names a block`);
  }
  assert.deepEqual(bad, [], `tier assignment problems:\n  ${bad.join('\n  ')}`);

  // Every block named as answering a Base set must be a block that exists.
  const { listBlockVariants } = require('../compose.js');
  const types = new Set(Object.keys(listBlockVariants()));
  const phantom = [...new Set(Object.values(tiers.sets).map((r) => r.answeredBy).filter(Boolean))]
    .filter((b) => !types.has(b));
  assert.deepEqual(phantom, [], `these blocks are named as answering a Base set and do not exist: ${phantom.join(', ')}`);

  // And the totals must be DERIVED, not declared. Recompute both and compare.
  const counts = {}, sums = {};
  for (const row of Object.values(tiers.sets)) {
    counts[row.tier] = (counts[row.tier] || 0) + 1;
    sums[row.tier] = (sums[row.tier] || 0) + row.variants;
  }
  assert.deepEqual(counts, tiers.counts, 'the declared tier counts are not what the rows add up to');
  assert.deepEqual(sums, tiers.variantSums, 'the declared variant sums are not what the rows add up to');
  assert.equal(tiers.setCount, Object.keys(tiers.sets).length);
  assert.equal(tiers.totalVariants, Object.values(tiers.sets).reduce((a, r) => a + r.variants, 0));

  // Tier 1 is settled, so it must be: sixteen drawn plus Typography taken as the ramp.
  const t1 = Object.keys(tiers.sets).filter((n) => tiers.sets[n].tier === 'tier1');
  assert.equal(t1.length, 17, `tier 1 should hold 17 sets, holds ${t1.length}`);
  const drawn = new Set(
    Object.values(JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'figma-nodes.json'), 'utf8')).sets)
      .filter((s) => s.name.startsWith('Base/'))
      .map((s) => s.name.replace(/^Base\//, ''))
  );
  const notDrawn = t1.filter((n) => !drawn.has(n));
  assert.deepEqual(notDrawn, ['Typography'],
    `tier 1 is settled, so every set but Typography must be drawn. Not drawn: ${notDrawn.join(', ')}`);
});

test('every prompt written into Figma still matches the skill file it came from', { skip: havelib ? false : 'agents-final not present on this machine' }, () => {
  // The seven prompts are reproduced VERBATIM on the `Prompts (verbatim)` page of the Figma
  // file, because a prompt is the one part of this system that never appears in the output it
  // produces: a page can be read and argued with, the instruction that generated it usually
  // cannot. figma-prompts.json fingerprints each one at the moment it was written out.
  //
  // What this catches: a prompt edited in agents-final after it was written into Figma, which
  // makes the Figma copy stale. What it CANNOT catch is an edit made inside Figma, and the
  // manifest says so rather than implying otherwise - the write script verified that side at
  // the time by reading the characters back off the node and comparing.
  const fs = require('node:fs');
  const path = require('node:path');
  const manifest = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'figma-prompts.json'), 'utf8'));

  // The same trivial checksum the Figma plugin sandbox had to be able to recompute, since it
  // has no crypto. Deliberately weak against an adversary, entirely adequate against a typo.
  const ck = (t) => {
    let a = 0;
    for (let i = 0; i < t.length; i++) a = (a + t.charCodeAt(i) * (i % 7 + 1)) % 2147483647;
    return a;
  };

  const drifted = [];
  for (const [slug, rec] of Object.entries(manifest.prompts)) {
    const file = path.join(LIBRARY, rec.path);
    if (!fs.existsSync(file)) { drifted.push(`${slug}: ${rec.path} no longer exists`); continue; }
    const t = fs.readFileSync(file, 'utf8');
    if (t.length !== rec.chars) drifted.push(`${slug}: ${t.length} chars against ${rec.chars} written to Figma`);
    else if (ck(t) !== rec.checksum) drifted.push(`${slug}: same length, different content (checksum ${ck(t)} against ${rec.checksum})`);
    if (t.split('\n').length !== rec.lines) drifted.push(`${slug}: ${t.split('\n').length} lines against ${rec.lines}`);
  }
  assert.deepEqual(drifted, [],
    'these prompts have changed since they were written into Figma, so the page is now stale:\n  ' +
    `${drifted.join('\n  ')}\n Re-run the write and update figma-prompts.json.`);

  // The manifest must cover the whole run order, in order, and nothing else.
  assert.deepEqual(Object.keys(manifest.prompts).sort(), manifest.runOrder.slice().sort(),
    'the fingerprinted prompts and the declared run order disagree');
  const orders = manifest.runOrder.map((s) => manifest.prompts[s].order);
  assert.deepEqual(orders, orders.slice().sort((a, b) => a - b),
    'the declared order does not follow the run order, so the page is numbered against the sequence');

  // The total is derived, not declared.
  assert.equal(
    manifest.totalChars,
    Object.values(manifest.prompts).reduce((a, r) => a + r.chars, 0),
    'the declared total is not the sum of the parts'
  );

  // Every skill in RUN_ORDER must be fingerprinted. Adding an eighth skill to the run without
  // writing it out would otherwise leave a prompt nobody can inspect.
  const fingerprinted = new Set(Object.values(manifest.prompts).map((r) => r.path.replace(/\/SKILL\.md$/, '')));
  const missing = RUN_ORDER.filter((rel) => !fingerprinted.has(rel));
  assert.deepEqual(missing, [],
    `these skills are in the run order and have no prompt written out for inspection: ${missing.join(', ')}`);
});

test('the page inventory accounts for every page, and no page is silently empty', () => {
  // The counterpart to the correction in figma-nodes.json. A page with no content is a page
  // whose work is gone or was never done, and the only reason that read as normal for a whole
  // session is that nothing had ever written down what each page should hold.
  const fs = require('node:fs');
  const path = require('node:path');
  const measured = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'figma-nodes.json'), 'utf8'));

  const pages = measured.pages;
  assert.equal(Object.keys(pages).length, measured.pageTotals.pages,
    'the page inventory and the declared page count disagree');
  assert.equal(measured.pageTotals.emptyPages, 0,
    `${measured.pageTotals.emptyPages} pages were measured empty and that has not been explained`);

  // A page is recorded in one of three shapes, because pages come in three kinds: a NUMBER of
  // component sets, the LIST of frames it holds, or BOTH - the Components page carries 26 sets
  // and an index frame over them. Anything that resolves to nothing is a page with nothing in
  // it, which is the whole point of the check.
  //
  // The both-shape was added after the index frame landed, and it broke this test first: the
  // reader knew two shapes and got a third, so it reported the Components page as holding
  // nothing while that page held more than any other. A schema check that cannot describe a
  // legitimate new case reports it as a defect, which is its own kind of false alarm.
  const setsOf = (c) => (typeof c === 'number' ? c : (c && typeof c === 'object' && !Array.isArray(c) ? c.sets : null));
  const framesOf = (c) => (Array.isArray(c) ? c : (c && typeof c === 'object' ? c.frames || [] : []));

  const empty = [];
  for (const [name, content] of Object.entries(pages)) {
    const sets = setsOf(content);
    const frames = framesOf(content);
    if (sets === null && frames.length === 0) { empty.push(`${name} (unrecognised shape)`); continue; }
    if ((sets || 0) < 1 && frames.length === 0) { empty.push(`${name} (no sets and no frames)`); continue; }
    for (const frame of frames) {
      if (typeof frame !== 'string' || !frame.trim()) empty.push(`${name} (unnamed frame)`);
    }
  }
  assert.deepEqual(empty, [], `these pages hold nothing: ${empty.join(', ')}`);

  // The component-set totals must reconcile with the per-id set list, so the two halves of
  // this manifest cannot drift from each other.
  const setsByPage = {};
  for (const s of Object.values(measured.sets)) setsByPage[s.page] = (setsByPage[s.page] || 0) + 1;
  for (const [name, content] of Object.entries(pages)) {
    const claimed = setsOf(content);
    if (claimed === null) continue;
    assert.equal(claimed, setsByPage[name] || 0,
      `page "${name}" claims ${claimed} component sets; the set list holds ${setsByPage[name] || 0}`);
  }
  // And a page that declares NO sets must genuinely have none in the set list, or the manifest
  // is describing a documentation page that is quietly also a component page.
  for (const [name, content] of Object.entries(pages)) {
    if (setsOf(content) !== null) continue;
    assert.ok(!setsByPage[name],
      `page "${name}" is recorded as frames only, but the set list puts ${setsByPage[name]} component sets on it`);
  }
  assert.equal(
    Object.keys(measured.sets).length, measured.pageTotals.componentSets,
    'the set list and the declared component-set total disagree'
  );
});

test('any block whose Figma set draws more variants than the code exports says why', () => {
  // Every block type exports exactly two render functions. Six component sets draw three to
  // six. That is a modelling difference rather than a gap - an axis the code takes as CONTENT
  // is an axis Figma has to draw as a VARIANT, because a static drawing cannot take a prop -
  // but silent, it reads as four missing variants to anyone comparing the counts.
  const fs = require('node:fs');
  const path = require('node:path');
  const { FIGMA_NODES } = require('../vds-bridge.js');
  const { listBlockVariants } = require('../compose.js');
  const measured = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'figma-nodes.json'), 'utf8'));
  const axes = measured.variantAxes || {};
  const code = listBlockVariants();

  const undeclared = [], stale = [];
  for (const [type, id] of Object.entries(FIGMA_NODES)) {
    const set = measured.sets[id];
    if (!set) continue; // the existence test owns that failure
    const codeCount = code[type].length;
    if (set.variants > codeCount) {
      if (!axes[type]) undeclared.push(`${type}: figma draws ${set.variants}, code exports ${codeCount}`);
      else if (axes[type].length < 40) undeclared.push(`${type}: declared with no usable reason`);
    } else if (axes[type]) {
      // The other direction: a declaration left behind after the counts converge is a note
      // explaining a difference that no longer exists.
      stale.push(`${type}: declared as drawing more than code, but figma has ${set.variants} and code ${codeCount}`);
    }
    // Figma must never draw FEWER than the code exports: that really is a missing drawing.
    assert.ok(set.variants >= codeCount,
      `${type}: the Figma set draws ${set.variants} variants and the code exports ${codeCount}. A render ` +
      'function with no drawing is the divergence FIGMA_NODES exists to prevent.');
  }

  assert.deepEqual(undeclared, [],
    `these blocks draw more variants in Figma than they export in code, with no reason recorded in ` +
    `figma-nodes.json variantAxes:\n  ${undeclared.join('\n  ')}`);
  assert.deepEqual(stale, [], `stale variantAxes declarations:\n  ${stale.join('\n  ')}`);

  // Every declaration must name a real block type.
  const phantom = Object.keys(axes).filter((k) => !k.startsWith('_') && !code[k]);
  assert.deepEqual(phantom, [], `variantAxes names block types that do not exist: ${phantom.join(', ')}`);
});
