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

test('the variant-axis measurement matches the code it pairs, and every axis is really an axis', () => {
  // figma-nodes.json records how MANY variants a set draws. This file records WHICH, and the
  // difference is the whole reason `PropContract.figmaProperty` is null on all 43 records.
  //
  // Measured 2026-07-31 across every set: the Figma side is NOMINAL - twelve distinct axis
  // names (Style on 16 sets, Layout on 8, State on 7, Kind on 4, then Tone, Size, Panes,
  // Confirm, Pointer, Inset, Artwork, Class) with meaningful values like `Tone=Negative` and
  // `Confirm=Type to confirm`. The code side is POSITIONAL: `banner-1`, `banner-2`, and for
  // one block `footer-a`/`footer-b`. A positional key carries no value, so NO derivation can
  // bind it to a nominal axis - there is nothing to compare. That is the finding this file
  // exists to hold, and it is why the fix is a change to the block contract rather than a
  // cleverer matcher.
  //
  // WHAT THIS TEST CANNOT CHECK: the Figma half. Re-reading the axes needs the file, which is
  // a network call. It guards the code half and the file's internal consistency, so a renamed
  // export fails here and a renamed Figma AXIS fails only on the next re-measure. The same
  // honest limit figma-nodes.json and figma-variables.json both carry.
  const fs = require('node:fs');
  const path = require('node:path');
  const { FIGMA_NODES, FIGMA_FILE_KEY } = require('../vds-bridge.js');
  const { listBlockVariants } = require('../compose.js');
  const measured = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'figma-variants.json'), 'utf8'));
  const nodes = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'figma-nodes.json'), 'utf8'));
  const code = listBlockVariants();

  assert.equal(measured.file_key, FIGMA_FILE_KEY,
    'the variant measurement was taken from a different file than the bridge writes records against');

  // Every block type must be accounted for. A block absent from the measurement is a block
  // whose axis nobody has read, and it would silently be exempt from everything below.
  const missing = Object.keys(code).filter((t) => !measured.blocks[t]);
  assert.deepEqual(missing, [], `figma-variants.json has no entry for: ${missing.join(', ')}`);
  const phantom = Object.keys(measured.blocks).filter((t) => !code[t]);
  assert.deepEqual(phantom, [], `figma-variants.json names block types that do not exist: ${phantom.join(', ')}`);

  const keyDrift = [], nodeDrift = [], thinAxis = [];
  for (const [type, rec] of Object.entries(measured.blocks)) {
    // The code half, checked against what the module actually exports. This is the arm that
    // fires on a rename, which is the realistic way this file goes stale.
    assert.deepEqual(rec.codeKeys, code[type],
      `${type}: figma-variants.json records code keys ${JSON.stringify(rec.codeKeys)} but the ` +
      `module exports ${JSON.stringify(code[type])}. Re-measure rather than editing the record.`);
    if (rec.codeVariants !== code[type].length) keyDrift.push(type);

    // The two measurements must agree about the same node, or one of them is about a
    // different component.
    if (FIGMA_NODES[type] !== rec.nodeId) nodeDrift.push(`${type}: bridge says ${FIGMA_NODES[type]}, measurement says ${rec.nodeId}`);
    const set = nodes.sets[rec.nodeId];
    if (set) {
      assert.equal(rec.figmaVariants, set.variants,
        `${type}: figma-variants.json counted ${rec.figmaVariants} variants and figma-nodes.json ` +
        `counted ${set.variants} for the same node. Two measurements of one set disagree.`);
    }

    // An axis with one value is not an axis, it is a label. Recording one would make a
    // future binding look derivable when the Figma side offers no choice at all.
    for (const [axis, values] of Object.entries(rec.axes)) {
      if (values.length < 2) thinAxis.push(`${type}.${axis} has only ${JSON.stringify(values)}`);
      assert.ok(new Set(values).size === values.length, `${type}.${axis} lists a duplicate value`);
    }
    assert.ok(Object.keys(rec.axes).length >= 1, `${type}: no variant axis was measured at all`);
  }

  assert.deepEqual(keyDrift, [], `codeVariants disagrees with codeKeys for: ${keyDrift.join(', ')}`);
  assert.deepEqual(nodeDrift, [], `node id disagreement:\n  ${nodeDrift.join('\n  ')}`);
  assert.deepEqual(thinAxis, [], `these axes offer no choice:\n  ${thinAxis.join('\n  ')}`);

  // The header must state the axis tally, and it must be derived rather than typed - so the
  // recorded tally is recomputed from the per-block axes here.
  const recomputed = {};
  for (const rec of Object.values(measured.blocks)) {
    for (const axis of Object.keys(rec.axes)) recomputed[axis] = (recomputed[axis] || 0) + 1;
  }
  assert.deepEqual(measured.axisNames, recomputed,
    'the axisNames tally in the header does not match the per-block axes below it. A summary ' +
    'that disagrees with its own detail is the four-hand-kept-copies failure in one file.');

  // And the positional-versus-nominal finding itself, asserted so it cannot quietly stop
  // being true without someone noticing that the whole binding problem changed shape.
  const nominalCodeKeys = Object.entries(measured.blocks)
    .filter(([type, rec]) => rec.codeKeys.some((k) => {
      const suffix = k.slice(type.length + 1);
      return suffix && !/^[0-9]+$/.test(suffix) && !/^[a-z]$/.test(suffix);
    }))
    .map(([type]) => type);
  assert.deepEqual(nominalCodeKeys, [],
    `these blocks now export NOMINAL variant keys: ${nominalCodeKeys.join(', ')}. That is the ` +
    'intended direction and it means figmaProperty became derivable for them - update this ' +
    'test and bind them rather than leaving the binding null.');
});

test('every block either binds figmaProperty to a real axis, or says why it cannot', () => {
  // `PropContract.figmaProperty` was null on every record in the programme's history, and
  // nothing in the kernel had ever set it to anything else. The reason was structural, not
  // an omission: Figma names variants NOMINALLY and the code named them POSITIONALLY, so
  // there was no value to compare.
  //
  // Measuring all 43 against the real file settled each case, and the split is the finding:
  // 31 aligned, 1 subset, 11 where the two sides vary DIFFERENT QUESTIONS. Every one of the
  // 11 is a control primitive, and that is coherent rather than careless - a static drawing
  // cannot hold a runtime value, so Figma varies what a designer must SEE (state, tone,
  // pointer, size) while the code varies COMPOSITION and takes the rest as content.
  //
  // So this test guards BOTH outcomes. A forced binding on a different_axis block would
  // publish that `banner-1` means `Tone=Warning`, which is false in both directions, and a
  // wrong binding is worse than an absent one because a wrong one gets used.
  const fs = require('node:fs');
  const path = require('node:path');
  const { listBlockVariants } = require('../compose.js');
  const measured = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'figma-variants.json'), 'utf8'));
  const code = listBlockVariants();

  const VERDICTS = new Set(['aligned', 'subset', 'different_axis']);
  const counts = { aligned: 0, subset: 0, different_axis: 0 };

  for (const [type, rec] of Object.entries(measured.blocks)) {
    assert.ok(VERDICTS.has(rec.axisVerdict),
      `${type}: axisVerdict ${JSON.stringify(rec.axisVerdict)} is not one of the three. An ` +
      'unclassified block is one nobody has looked at, and it would be silently exempt from ' +
      'everything below.');
    counts[rec.axisVerdict] += 1;

    if (rec.axisVerdict === 'different_axis') {
      assert.ok(!rec.bindsTo,
        `${type} is different_axis and still carries a binding. The two sides vary different ` +
        'questions, so any mapping is false in both directions.');
      assert.ok(rec.noBindingBecause && rec.noBindingBecause.length > 60,
        `${type} has no binding and no usable reason. "It does not map" is not a reason; the ` +
        'record must say WHAT each side varies, or the next author re-derives it.');
      continue;
    }

    // A binding must name an axis the file actually draws, and map only real values.
    assert.ok(rec.bindsTo, `${type} is ${rec.axisVerdict} and carries no binding`);
    const axis = rec.bindsTo.property;
    assert.ok(rec.axes[axis],
      `${type} binds to ${JSON.stringify(axis)}, which is not an axis measured on its set. ` +
      `Measured: ${Object.keys(rec.axes).join(', ')}`);
    const drawn = new Set(rec.axes[axis]);

    const mapped = rec.bindsTo.values;
    assert.deepEqual(Object.keys(mapped).sort(), [...code[type]].sort(),
      `${type}: the binding must map EVERY code variant and no others`);
    for (const [key, value] of Object.entries(mapped)) {
      assert.ok(drawn.has(value),
        `${type}: ${key} maps to ${JSON.stringify(value)}, which the set does not draw on ` +
        `${axis}. Drawn: ${[...drawn].join(', ')}`);
    }
    assert.equal(new Set(Object.values(mapped)).size, Object.keys(mapped).length,
      `${type}: two code variants map to the same Figma value, so the binding is not a pairing`);

    // A subset must declare which drawn values have no code counterpart, and an aligned
    // block must have none left over - otherwise `aligned` is claiming more than it paired.
    const unpaired = [...drawn].filter((d) => !Object.values(mapped).includes(d));
    if (rec.axisVerdict === 'subset') {
      assert.deepEqual(rec.bindsTo.unpairedFigmaValues, unpaired,
        `${type}: the declared unpaired values disagree with the measurement`);
      assert.ok(unpaired.length > 0, `${type} is declared subset and pairs everything`);
    } else {
      assert.deepEqual(unpaired, [],
        `${type} is declared aligned but the set draws ${unpaired.join(', ')} with no code ` +
        'counterpart. That is a subset, and calling it aligned overstates the pairing.');
    }
  }

  assert.deepEqual(measured._verdicts.counts, counts,
    'the verdict tally in the header disagrees with the per-block verdicts beneath it');
  assert.ok(counts.aligned > 0 && counts.different_axis > 0,
    'both outcomes must be present, or this test is only exercising one branch and the ' +
    'other could be broken without anything noticing');
});

test('the library generator is deterministic, refuses rather than guesses, and holds no value', () => {
  // BREACH-0010's remedy, and the test that stops it recurring. All 46 component sets in
  // the master file were drawn by scripts written inside agent turns and never committed:
  // a grep for `createComponentSet`, `combineAsVariants` and `createComponent(` across the
  // whole repository returned zero hits, so the file could be PROVEN correct and could not
  // be REPRODUCED.
  const fs = require('node:fs');
  const path = require('node:path');
  const { buildLibraryScript, specsFor, FIGMA_FILE_KEY } = require('../figma-draw.js');
  const variants = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'figma-variants.json'), 'utf8'));

  const register = Object.keys(variants.blocks).sort().map((blockType, i) => ({
    id: `CMP-${String(i + 1).padStart(4, '0')}`,
    name: blockType[0].toUpperCase() + blockType.slice(1),
    blockType,
  }));

  assert.equal(variants.file_key, FIGMA_FILE_KEY,
    'the generator and the measurement must be about the same file');

  // IT MUST PARSE. This is not paranoia: the first version emitted `'\n'` inside a
  // template literal, so the template interpolated it into a REAL newline and the emitted
  // script carried an unterminated string. Every other assertion here passed on that
  // script, because they all read it as text. Caught only by reading the output before
  // running it, which is not a check - it is luck.
  new (require('node:vm').Script)(`(async () => {${buildLibraryScript(register, variants)}})()`);

  // DETERMINISM. Two runs, byte-identical. A generator whose output moves on its own
  // cannot be diffed, and a diff is the only way a reader sees what a redraw would change
  // BEFORE it changes it.
  const a = buildLibraryScript(register, variants);
  const b = buildLibraryScript(register, variants);
  assert.equal(a, b, 'two runs over the same inputs produced different bytes');

  // Every registered block is drawn, and the axis is the one the file was MEASURED to
  // have - not one the generator chose. Minting a new axis would make a redraw a redesign.
  const specs = specsFor(register, variants);
  assert.equal(specs.length, register.length, 'every registered block must get a spec');
  for (const spec of specs) {
    const measured = variants.blocks[spec.blockType];
    assert.ok(measured.axes[spec.axis], `${spec.blockType}: axis ${spec.axis} is not measured`);
    assert.deepEqual(spec.variants.map((v) => v.value), measured.axes[spec.axis],
      `${spec.blockType}: the generator must draw the variants the file has`);
    assert.equal(spec.nodeId, measured.nodeId);
  }

  // NO REALISATION. Every fill and radius is a variable NAME bound at run time, never a
  // value. VDS S-2(4) forbids a realisation in a governance artefact, and this script is
  // derived from one. A hex here would also mean a redraw could fight the token collection
  // and win, which is the wrong-red failure that cost four inks and nine values.
  const forbidden = [/#[0-9a-fA-F]{3,8}\b/, /\brgba?\(/, /\boklch\(/, /\bhsla?\(/];
  for (const pattern of forbidden) {
    const hit = a.match(pattern);
    assert.equal(hit, null,
      `the generated script contains ${hit && hit[0]}, which is a realisation. Bind a ` +
      'variable by name instead.');
  }

  // The three behaviours that make a redraw safe, asserted against the emitted text
  // because they are the difference between a remedy and a second way to lose the file.
  assert.match(a, /await page\.loadAsync\(\)/,
    "page.children is EMPTY until loadAsync in dynamic-page mode - a survey that skipped " +
    'it once reported seven of ten pages empty and read as a wipe');
  assert.match(a, /setSharedPluginData\('vds', 'componentId'/,
    'without an identity stamp a second run cannot tell its own work from a hand-built ' +
    'set, and would duplicate or overwrite something it did not make');
  assert.match(a, /variant names moved/,
    'a set whose axis moved must be REFUSED by name, not guessed: mapping the wrong old ' +
    'variant onto the wrong new one silently rewrites a component into a different one');
  assert.match(a, /refusals\.push/,
    'a variable that does not resolve must be reported, never silently skipped - a filter ' +
    'that silently dropped three of four tones once reported success');

  // A throw rolls back the whole script, so refusals must be COLLECTED and returned. A
  // partial redraw reporting success is the worst outcome available.
  assert.ok(!/throw new Error/.test(a),
    'the script must not throw: a throw rolls back every set it had already drawn');
});

test('coverage reports three separate numbers per library and never invents a denominator', () => {
  // THE INSTRUMENT MUST NOT COMPUTE ITS OWN DENOMINATOR. Every coverage answer this
  // programme has given was a single fraction, and a fraction hides which of three
  // different things went wrong: how many units the reference library HAS, how many are
  // answered, and how much of the contract actually survived into a record.
  //
  // Borrowed from ds-contracts-poc's census, whose headline was 100% clean over 1,618 sets
  // and whose own caveat is the point - "Refusal-free is not pixel-right" - because it
  // scored perfectly while naming 3,316 degradations. CLEAN and COMPLETE are different
  // questions and one number cannot answer both.
  //
  // This test deliberately does NOT require compose.js. Deriving the denominator from the
  // block registry would make the instrument a function of the thing it measures, and
  // `conformance/README.md` states the consequence exactly: "An instrument built that way
  // cannot be surprised."
  const fs = require('node:fs');
  const path = require('node:path');
  const src = fs.readFileSync(__filename, 'utf8');
  const thisTest = src.slice(src.indexOf("test('coverage reports three separate numbers"));
  assert.ok(!/require\(['"]\.\.\/compose\.js['"]\)/.test(thisTest),
    'this test must not import the block registry: a denominator derived from what is ' +
    'covered can never report a gap');

  const cov = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'vendor', 'coverage.json'), 'utf8'));
  const libs = Object.entries(cov.libraries);
  assert.ok(libs.length >= 4, `expected at least four libraries, got ${libs.length}`);

  let unmeasured = 0;
  for (const [name, lib] of libs) {
    assert.ok(lib.denominator && 'sets' in lib.denominator,
      `${name}: no denominator field at all`);
    assert.ok(lib.answered && typeof lib.answered.sets === 'number',
      `${name}: answered.sets must be a number`);
    assert.ok(lib.facts_carried, `${name}: no facts_carried. "Answered" without it is a ` +
      'pairing presented as a contract');
    assert.ok(Array.isArray(lib.degradations), `${name}: degradations must be a list, ` +
      'even an empty one - a library with no known losses should say so rather than omit it');

    const d = lib.denominator.sets;
    if (d === null) {
      // AN UNMEASURED DENOMINATOR MUST SAY SO IN WORDS, not be quietly absent, and must
      // not be back-filled with an advertised figure. SDS advertises "400+ components";
      // writing 400 here would turn marketing into a statistic.
      unmeasured += 1;
      assert.match(lib.denominator.source, /UNMEASURED/,
        `${name}: a null denominator must be labelled UNMEASURED with the reason`);
      continue;
    }
    assert.ok(Number.isInteger(d) && d > 0, `${name}: denominator ${d} is not a count`);

    // A MEASURED DENOMINATOR MUST NAME A FILE THAT EXISTS. This arm was added because the
    // seed found the hole: writing SDS's advertised "400+" in as a real denominator, with
    // "the kit advertises 400+ components" as its source, PASSED every assertion here.
    // The null branch above was guarded and the non-null branch was not, so the check
    // caught the honest absence and waved through the dishonest presence - which is the
    // wrong way round, and is the defect the whole file exists to prevent.
    //
    // Requiring a real path is what distinguishes a measurement from a claim: an
    // advertisement has no artefact behind it, and a source naming a file a reader can
    // open is one they can disagree with.
    const cited = (lib.denominator.source || '').match(/[\w./-]+\.(json|md|yaml|yml)/);
    assert.ok(cited,
      `${name}: the denominator is ${d} and its source names no file - "${lib.denominator.source}". ` +
      'A measured denominator cites an artefact; anything else is a claim and belongs in ' +
      'the UNMEASURED branch.');
    assert.ok(fs.existsSync(path.join(__dirname, '..', cited[0])),
      `${name}: the denominator cites ${cited[0]}, which does not exist`);
    assert.ok(lib.answered.sets <= d,
      `${name}: answered ${lib.answered.sets} exceeds the denominator ${d}, so one of the ` +
      'two is measuring something the other is not');
  }

  assert.ok(unmeasured > 0,
    'at least one denominator is genuinely unknown today, and a run where all four are ' +
    'measured means this file was filled in rather than measured - check it');

  // No single score anywhere. The whole point is that there is not one.
  assert.ok(!('score' in cov) && !('coverage' in cov),
    'coverage.json must not carry a single score: three numbers per library, or none');
});

test('the generated library covers every block type, with a stable identity per block', () => {
  // BREACH-0010 was that nothing could redraw the library. `figma-generated.json` is the
  // census read back OUT of the file after the sweep, so this checks a measurement rather
  // than a claim about what was sent.
  //
  // The identity assertions carry the weight, and they exist because of a real bug. The
  // first sweep batch minted ids from each record's POSITION IN THE CALL, so a three-block
  // slice made hero=CMP-0001 and the next batch made card=CMP-0002 - which was already
  // divider's stamp. Two components inherited two others' identities. The amend guard
  // refused both by name and nothing was overwritten, which is exactly what it is for; but
  // a guard is the last line, not the fix. `canonicalId()` derives the id from the block
  // type, and `specsFor` throws on a mismatch rather than trusting its caller.
  const fs = require('node:fs');
  const path = require('node:path');
  const { canonicalId, specsFor } = require('../figma-draw.js');
  const variants = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'figma-variants.json'), 'utf8'));
  const gen = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'figma-generated.json'), 'utf8'));

  // COVERAGE, both directions. A block with no drawing is the gap; a drawing for no block
  // is a set nothing owns.
  const drawn = new Set(gen.sets.map((s) => s.blockType));
  const known = Object.keys(variants.blocks);
  assert.deepEqual(known.filter((b) => !drawn.has(b)), [],
    'these block types were never drawn by the generator');
  assert.deepEqual([...drawn].filter((b) => !variants.blocks[b]), [],
    'these drawn sets name a block type that does not exist');

  // IDENTITY. Every id canonical, and every id and block type unique.
  const ids = gen.sets.map((s) => s.componentId);
  const types = gen.sets.map((s) => s.blockType);
  assert.deepEqual(ids.filter((v, i) => ids.indexOf(v) !== i), [],
    'two sets carry the same component id, so one is stamped as the other');
  assert.deepEqual(types.filter((v, i) => types.indexOf(v) !== i), [],
    'two sets claim the same block type');
  for (const s of gen.sets) {
    assert.equal(s.componentId, canonicalId(s.blockType, variants),
      `${s.blockType} is stamped ${s.componentId} and its canonical id is ` +
      `${canonicalId(s.blockType, variants)}. An id derived from a position in a call is ` +
      'not an identity.');
    assert.ok(/^\d+:\d+$/.test(s.nodeId), `${s.blockType}: ${s.nodeId} is not a node id`);
  }

  // The variant count per set must match the axis the measurement recorded, or the drawing
  // is of a different component from the one the register describes.
  for (const s of gen.sets) {
    const axis = Object.keys(variants.blocks[s.blockType].axes)[0];
    assert.equal(s.variants, variants.blocks[s.blockType].axes[axis].length,
      `${s.blockType}: drew ${s.variants} variants and the measured ${axis} axis has ` +
      `${variants.blocks[s.blockType].axes[axis].length}`);
  }
  assert.equal(gen.totals.sets, gen.sets.length, 'the totals disagree with the rows beneath them');
  assert.equal(gen.totals.variants, gen.sets.reduce((t, s) => t + s.variants, 0),
    'the variant total disagrees with the rows beneath it');

  // And the refusal itself, seeded here rather than trusted: passing a wrong id must throw.
  assert.throws(
    () => specsFor([{ id: 'CMP-9999', name: 'Hero', blockType: 'hero' }], variants),
    /canonical id/,
    'specsFor must refuse an id that is not the block type\'s canonical one',
  );
});

test('the Base axis reconciliation states a verdict per block and never flatters the match', () => {
  // REFERENCES.md records Uber Base as "the control and messaging baseline" and lists what
  // came from it: the palette, the role-based type ramp, 16 redrawn sets. The VARIANT AXIS
  // VOCABULARY is not on that list, and until Base's real contracts were harvested nothing
  // could check whether it came from Base either.
  //
  // It largely did not: 1 of 14 matches in both name and values. This test exists so the
  // count cannot quietly improve without the underlying axes actually changing - the
  // failure mode being an author who edits a verdict rather than an axis.
  const fs = require('node:fs');
  const path = require('node:path');
  const rec = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'vendor', 'base-axis-reconciliation.json'), 'utf8'));
  const mine = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'figma-variants.json'), 'utf8'));

  const VERDICTS = new Set(['exact', 'renamed', 'same_name_different_values', 'invented']);
  const tally = {};
  for (const row of rec.rows) {
    assert.ok(VERDICTS.has(row.verdict), `${row.block}: ${row.verdict} is not a verdict`);
    tally[row.verdict] = (tally[row.verdict] || 0) + 1;

    // Every row must name a block that exists, and quote MY axis as the measurement has it.
    const block = mine.blocks[row.block];
    assert.ok(block, `${row.block} is reconciled and is not a block type`);
    const myAxis = Object.keys(block.axes)[0];
    assert.equal(row.myAxis, myAxis,
      `${row.block}: the reconciliation says my axis is ${row.myAxis} and figma-variants.json ` +
      `says ${myAxis}. A reconciliation that misquotes one side settles nothing.`);
    assert.deepEqual(row.myValues, block.axes[myAxis],
      `${row.block}: the reconciliation misquotes my axis values`);

    // A verdict of `renamed` is a real claim - the VALUE SET matches a Base axis exactly -
    // so it must name which one. Without that it is an opinion.
    if (row.verdict === 'renamed' || row.verdict === 'exact') {
      assert.ok(row.sameValuesAsBaseAxis,
        `${row.block} is ${row.verdict} and names no Base axis whose values it matches`);
    }
    if (row.verdict === 'invented') {
      assert.equal(row.sameValuesAsBaseAxis, null,
        `${row.block} is called invented and its values match Base's ${row.sameValuesAsBaseAxis}`);
    }
    // Every paired Base set carries MORE axes than mine. If that ever stops being true the
    // collapse claim needs re-checking rather than restating.
    assert.ok(row.baseAxisCount >= 1, `${row.block}: no Base axes recorded`);
  }

  assert.deepEqual(rec.verdicts, tally,
    'the verdict tally in the header disagrees with the rows beneath it');
  assert.equal(rec.paired, rec.rows.length, 'the paired count disagrees with the rows');
  assert.ok(rec.verdicts.exact <= 1,
    `${rec.verdicts.exact} blocks now match Base exactly, up from 1. That is good news and ` +
    'this test must be re-read rather than edited: confirm the AXES changed, not the verdicts.');
  assert.ok((rec.verdicts.invented || 0) + (rec.verdicts.same_name_different_values || 0) > 0,
    'no block diverges from Base at all, which would mean the vocabulary was inherited after ' +
    'all - re-measure before believing it');
});

// ---------------------------------------------------------------------------
// The Opbox deliverable states seven design rules in prose and ships CSS that
// breaks two of them. This guard RE-DERIVES both findings from the vendored
// bytes rather than restating the audit, because an audit that only quotes
// itself is the thing it is auditing: prose about enforcement, unenforced.
//
// The vendored files are a snapshot of somebody else's repository. This test
// must therefore be able to say THE SUBJECT WAS FIXED, and it does - a finding
// that stops reproducing fails LOUDLY with an instruction to re-read, not to
// edit. Silently passing on a fixed subject would let the audit rot into a
// claim about a system that no longer exists.
test('the Opbox audit re-derives from the vendored bytes', () => {
  const dir = path.join(__dirname, '..', 'vendor', 'opbox');
  const audit = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'vendor',
    'opbox-rules-audit.json'), 'utf8'));
  const read = (f) => fs.readFileSync(path.join(dir, f), 'utf8');
  const readme = read('README.figma-kit.md');
  const geist = read('tokens.geist.css');
  const css = read('opbox.css');

  // The RULES must be present in the README, or the audit is measuring against a
  // rule nobody wrote. This is the half that makes the finding a contradiction
  // rather than an opinion about colour.
  assert.match(readme, /Ink acts, blue selects/,
    'the README no longer states the ink/blue rule; the audit measures against a rule ' +
    'that has been withdrawn and must be re-read');
  assert.match(readme, /blocked ≠ disabled/,
    'the README no longer states the blocked-vs-disabled rule');
  assert.match(readme, /in danger text/,
    'the blocked rule no longer requires danger text');

  // FINDING 1. The ink token is defined, and consumed nowhere.
  assert.match(geist, /--action:\s*#171717/,
    '--action is no longer #171717 in the Geist token set');
  assert.match(geist, /--accent:\s*#006bff/,
    '--accent is no longer #006bff in the Geist token set');
  const actionUses = (css.match(/var\(--action\b/g) || []).length;
  const primary = css.match(/\.btn-primary\s*\{[^}]*\}/);
  assert.ok(primary, 'opbox.css no longer defines .btn-primary at all');
  assert.match(primary[0], /background:\s*var\(--accent\)/,
    'FIXED, PROBABLY: .btn-primary no longer fills with var(--accent). If it now uses ' +
    'var(--action) the finding is CLOSED and this test should record that, not be deleted.');
  assert.equal(actionUses, 0,
    `var(--action) is now consumed ${actionUses} times in opbox.css. The ink token was ` +
    'defined-and-unused when audited; if it is wired up the finding is CLOSED - re-read ' +
    'the audit rather than relaxing this number.');

  // FINDING 2. The blocked note is muted, where the rule says danger.
  const note = css.match(/\.btn-blocked-note\s*\{[^}]*\}/);
  assert.ok(note, 'opbox.css no longer defines .btn-blocked-note');
  assert.match(note[0], /color:\s*var\(--muted\)/,
    'FIXED, PROBABLY: .btn-blocked-note no longer uses var(--muted). Check whether it now ' +
    'uses a danger token, and close the finding if so.');
  assert.doesNotMatch(note[0], /danger|--negative|--critical/,
    '.btn-blocked-note now references a danger token, which closes the second finding');

  // FINDING 3. Two token sets ship together and one has no --action at all, so
  // which blue the system MEANS cannot be read off the artefact.
  const old = read('tokens.css');
  assert.doesNotMatch(old, /--action:/,
    'tokens.css now defines --action, which would close the two-sources finding');
  assert.match(old, /--accent:\s*#1677ff/,
    'tokens.css no longer carries the pre-Geist blue');

  // The audit must not have quietly turned into a pass. Every rule it records as
  // broken has just been re-derived above; a row claiming otherwise is a rewrite.
  const broken = audit.rules_checked.filter((r) => r.verdict.startsWith('BROKEN'));
  assert.equal(broken.length, 2,
    `the audit now records ${broken.length} broken rules and this test re-derives 2`);
  for (const row of audit.rules_checked) {
    assert.ok(row.evidence && row.source && row.why_it_matters,
      `${row.rule.slice(0, 40)}: a row without evidence, source and consequence is an opinion`);
  }
});

// ---------------------------------------------------------------------------
// The GRIGOLETTO Blueprint states design-system rules in prose; the same
// author's 39-template pack is shipped work. `blueprint-vs-pack.json` measures
// the second against the first. This guard holds the measurement to the same
// standard the measurement holds the pack to.
//
// It checks three things a write-up like this gets wrong: that the tallies
// match the rows beneath them, that the corpus admits what it did NOT read,
// and that a rule with no stated threshold is REPORTED rather than judged.
test('the Blueprint measurement tallies match its own rows', () => {
  const dir = path.join(__dirname, '..', 'vendor', 'grigoletto');
  const doc = JSON.parse(fs.readFileSync(path.join(dir, 'blueprint-vs-pack.json'), 'utf8'));
  const pack = JSON.parse(fs.readFileSync(path.join(dir, 'templates.json'), 'utf8'));
  const blueprint = fs.readFileSync(path.join(dir, 'blueprint.txt'), 'utf8');

  // THE RULES MUST BE IN THE SOURCE. Measuring nine files against a rule the
  // Blueprint does not actually state would be this instrument inventing the
  // standard and then grading against it - which is precisely what the
  // type_scale carve-out below refuses to do.
  assert.match(blueprint, /Choose exactly two typefaces/,
    'the Blueprint no longer states the two-typeface rule');
  assert.match(blueprint, /never flat/,
    'the Blueprint no longer states the flat-white rule');
  assert.match(blueprint, /8 \/ 16 \/ 24 \/ 32 \/ 48 \/ 64/,
    'the Blueprint no longer states the 8-point scale');
  // And it must still state NO maximum on font sizes, which is the whole basis
  // for reporting type_scale instead of judging it.
  assert.doesNotMatch(blueprint, /at most \d+ (font )?sizes|no more than \d+ sizes/i,
    'the Blueprint now states a maximum number of sizes, so type_scale can and ' +
    'should be judged rather than merely reported');

  // A header count that disagrees with the rows is the defect that made the
  // Base reconciliation worth guarding, and it is invisible by eye.
  const recount = (pick) => doc.rows.filter(pick).length;
  assert.equal(doc.rules.typefaces.holds, recount((r) => r.typefaces.holds),
    'the typeface tally disagrees with the rows beneath it');
  assert.equal(doc.rules.canvas.holds, recount((r) => r.canvas.holds),
    'the canvas tally disagrees with the rows beneath it');
  assert.equal(doc.rules.eight_point.holds, recount((r) => r.eightPoint.holds === true),
    'the 8-point tally disagrees with the rows beneath it');
  assert.equal(doc.rules.eight_point.not_applicable,
    recount((r) => r.eightPoint.holds === null),
    'the n/a count disagrees with the rows');

  // Every rule must account for every row. A row that is neither hold, break
  // nor n/a has been dropped, which is how a corpus silently shrinks.
  for (const [name, rule] of Object.entries(doc.rules)) {
    if (rule.holds === undefined) continue;
    const total = rule.holds + (rule.breaks || 0) + (rule.not_applicable || 0);
    assert.equal(total, doc.rows.length,
      `rule ${name} accounts for ${total} templates and there are ${doc.rows.length} rows`);
  }

  // THE CORPUS MUST ADMIT ITS OWN LIMIT. Nine of 39 were read; a write-up that
  // let "measured" quietly stand in for "the pack" would be the exact claim-
  // wider-than-its-test failure this repo has already filed twice.
  assert.equal(doc.corpus.templates_in_pack, pack.length,
    'the stated pack size disagrees with the harvested template list');
  assert.equal(doc.corpus.templates_measured, doc.rows.length,
    'the stated measured count disagrees with the rows');
  assert.ok(doc.corpus.templates_measured < doc.corpus.templates_in_pack,
    'measured now equals the pack size - GOOD NEWS, and the rate-limit note in ' +
    '_why_not_all must be rewritten rather than left standing as a stale excuse');
  assert.ok(/rate limit|429|budget/i.test(doc.corpus._why_not_all),
    'the corpus is short of the pack and does not say why');

  // A rule the source states no threshold for must NOT carry a pass/fail tally.
  // Inventing one would make this instrument the author of the standard it is
  // supposed to be measuring against.
  assert.equal(doc.rules.type_scale.holds, undefined,
    'type_scale now has a hold/break tally, but the Blueprint states no maximum ' +
    'number of sizes - a threshold has been invented somewhere');
  assert.ok(doc.rules.type_scale._not_judged, 'type_scale must say why it is not judged');

  // The first pass measured the wrong subject and flipped a rule from 0/9 to
  // 7/9. That correction stays visible: it is the most useful thing here.
  assert.ok(doc.first_pass_was_wrong && doc.first_pass_was_wrong._lesson,
    'the record of the first measurement being aimed at the wrong subject has been dropped');

  for (const r of doc.rows) {
    assert.ok(r.typefaces.families.length === r.typefaces.count,
      `${r.name}: the family list and the family count disagree`);
    assert.ok(r.nodes > 0 && r.textNodes > 0, `${r.name}: an empty read is not a measurement`);
    if (r.eightPoint.holds !== null) {
      assert.equal(r.eightPoint.holds, r.eightPoint.offGrid === 0,
        `${r.name}: the 8-point verdict disagrees with its own off-grid count`);
    }
  }
});

// ---------------------------------------------------------------------------
// The prompt kit measured against the block library. The value here is the
// AXIS finding, not the coverage number, and this guard exists mostly to stop
// the number being read as a score - 15 of 43 is not a mark out of 43 when the
// corpus was never aiming at 43 sections.
test('the prompt coverage is measured, not scored', () => {
  const dir = path.join(__dirname, '..', 'vendor', 'grigoletto');
  const doc = JSON.parse(fs.readFileSync(path.join(dir, 'prompts-vs-blocks.json'), 'utf8'));
  const prompts = JSON.parse(fs.readFileSync(path.join(dir, 'prompts.json'), 'utf8'));
  const variants = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'figma-variants.json'), 'utf8'));
  const blocks = Object.keys(variants.blocks || variants);

  // The denominator is the REAL block list, read from the manifest rather than
  // written down here. A hand-copied denominator drifts the moment a block is
  // added, and then the coverage claim is about a library that no longer exists.
  assert.equal(doc.coverage.of, blocks.length,
    `the measurement counts ${doc.coverage.of} block types and figma-variants.json has ` +
    `${blocks.length}`);
  const named = Object.keys(doc.coverage.hits);
  assert.equal(named.length, doc.coverage.named_by_at_least_one_site_prompt,
    'the named count disagrees with the hit table');
  assert.equal(named.length + doc.coverage.named_by_none.length, blocks.length,
    'named plus unnamed does not account for every block type');
  for (const t of [...named, ...doc.coverage.named_by_none]) {
    assert.ok(blocks.includes(t), `${t} is measured and is not a block type`);
  }

  // The corpus size must come from the corpus.
  assert.equal(doc.corpus.total, prompts.length,
    'the stated prompt count disagrees with the captured prompts');
  assert.equal(doc.corpus.site, prompts.filter((p) => p.part === 'SITE prompts').length,
    'the site-prompt count disagrees with the corpus');

  // BOTH MATCHER ERRORS STAY ON THE RECORD. They are the most useful thing in
  // the file: one keyword matched the word and never the thing, and one missed
  // the thing for being too specific. A later reader trusting the middle of the
  // table needs to know the ends were hand-checked and the middle was not.
  assert.ok(doc.matcher.corrections_made.length >= 2,
    'the record of the two matcher corrections has been dropped');
  assert.ok(doc.matcher._limit, 'the matcher must state what it cannot do');
  // The specific false positive must not creep back: `badge` matches App Store
  // badges in four prompts and the component in none.
  assert.ok(!Object.keys(doc.coverage.hits).includes('notificationbadge'),
    'notificationbadge is named again - check whether a real notification badge ' +
    'appeared in the corpus, or whether the bare "badge" keyword came back');

  // And the finding must outrank the number.
  assert.ok(/genre/i.test(doc.the_finding.claim) && /section/i.test(doc.the_finding.claim),
    'the axis finding - genre versus section - has been lost from the claim');
});

// ---------------------------------------------------------------------------
// token-reach: does a declared custom property reach anything, and does every
// reference resolve? Built after Opbox's `--action` was found defined-and-unused
// while the rule it existed to carry was broken by the CSS beside it.
//
// Guarded in three parts: the instrument works on a fixture, it holds on THIS
// repo's own output (where it already found and removed a dead token), and it
// still reproduces the Opbox findings.
test('token-reach finds both directions', () => {
  const { tokenReach } = require('../token-reach.js');

  const r = tokenReach([
    { source: 'tokens.css', css: ':root { --used: red; --dead: blue; }' },
    { source: 'app.css', css: '.a { color: var(--used); background: var(--missing); }' },
  ]);
  assert.deepEqual(r.unreferenced.map((u) => u.name), ['--dead'],
    'a declared-and-unused token must be reported');
  assert.equal(r.unreferenced[0].source, 'tokens.css', 'a finding must name where to go');
  assert.deepEqual(r.undeclared.map((u) => u.name), ['--missing'],
    'a var() with no declaration must be reported - this is the direction that is a ' +
    'defect every time, because the browser falls back silently');

  // A COMMENT IS NOT CODE. `--token` was reported undeclared because build.js
  // and blocks/hero.js write `var(--token)` in prose as a placeholder. An
  // instrument that invents work from comments is one people learn to ignore.
  const commented = tokenReach([
    { source: 'x.css', css: '/* every value is a var(--token) */\n:root { --real: 1px; }\n' +
      '.a { width: var(--real); }' },
  ]);
  assert.deepEqual(commented.undeclared, [],
    'a var() inside a CSS comment was counted as a reference');

  // Passing ONLY a token file would make every token look unreferenced, which
  // is a confident wrong answer rather than a missing one.
  const alone = tokenReach([{ source: 'tokens.css', css: ':root { --a: 1px; --b: 2px; }' }]);
  assert.equal(alone.unreferenced.length, 2);
  assert.ok(alone.doesNotCover.some((l) => /not in `sources`/.test(l)),
    'the reading must say that a token used elsewhere reads as unreferenced here');
});

test('this repository declares no token that reaches nothing', () => {
  const { tokenReach } = require('../token-reach.js');
  const { cssVars, STRUCTURE_CSS } = require('../build.js');
  const tokens = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'tokens', 'geist.json'), 'utf8'));

  const r = tokenReach([
    { source: 'cssVars(tokens/geist.json)', css: cssVars(tokens.tokens || tokens) },
    { source: 'STRUCTURE_CSS', css: STRUCTURE_CSS },
  ], { ignoreDeclared: ['--type-scale'] });

  // THE ONE EXEMPTION, AND WHY IT IS AN EXEMPTION RATHER THAN A FIX.
  //
  // `--type-scale` is the only token this check reports here, and the first
  // response was to delete the emission as dead weight. Two existing tests
  // refused: `typography.typeScale actually changes --type-scale`, and the one
  // pairing the Figma type-scale specimen against the value the build emits.
  // It is a PUBLISHED READING consumed by an instrument, not an input to any
  // rule - so "no CSS reads it" was true and "nothing reads it" was not.
  //
  // Kept as a named exemption with the reason attached, because the alternative
  // is a check that people learn to override. If a second name ever joins this
  // list, the reason has to be as good as this one.
  assert.deepEqual(r.unreferenced.map((u) => u.name), [],
    'a custom property is emitted that nothing reads - either wire it up or stop emitting it');
  assert.deepEqual(r.undeclared.map((u) => u.name), [],
    'the stylesheet references a token nothing declares, so those surfaces render ' +
    'with the browser fallback and it looks like a design decision');
  assert.ok(r.declared > 40, `only ${r.declared} tokens considered - the reading looks empty`);
});

test('the Opbox kit references three tokens neither of its token sets declares', () => {
  const { tokenReach } = require('../token-reach.js');
  const dir = path.join(__dirname, '..', 'vendor', 'opbox');
  const read = (f) => ({ source: f, css: fs.readFileSync(path.join(dir, f), 'utf8') });

  // Checked against BOTH token sets, because "it must be declared in the other
  // one" is the first thing anyone would say, and it is not true.
  for (const set of ['tokens.geist.css', 'tokens.css']) {
    const r = tokenReach([read(set), read('opbox.css')]);
    assert.deepEqual(r.undeclared.map((u) => u.name).sort(), ['--mono', '--radius-sm', '--sans'],
      `with ${set}, the undeclared set changed. If it is now empty the defect is FIXED ` +
      'and this test should record that rather than be relaxed.');
  }
  const geist = tokenReach([read('tokens.geist.css'), read('opbox.css')]);
  assert.ok(geist.undeclared.find((u) => u.name === '--mono').uses >= 16,
    '--mono was used 16 times and declared nowhere; every mono surface falls back');
  assert.ok(geist.unreferenced.some((u) => u.name === '--action'),
    '--action is the token that started this: defined to carry "ink acts, blue selects" ' +
    'and consumed by nothing');
});

// ---------------------------------------------------------------------------
// A prompt per block type, DERIVED. The guard's job is to stop "43 of 43"
// becoming a count of structurally-present, empty entries - which is exactly
// what the first version produced for seventeen blocks that document
// themselves in `//` runs rather than `/* */` blocks.
test('every block type has a prompt that is not hollow', () => {
  const { buildAll, STYLE } = require('../block-prompts.js');
  const variants = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'figma-variants.json'), 'utf8'));
  const manifest = variants.blocks || variants;
  const all = buildAll();

  assert.deepEqual(Object.keys(all).sort(), Object.keys(manifest).sort(),
    'the prompt set and the measured block set are not the same set');

  let variantCount = 0;
  let noted = 0;
  for (const [type, b] of Object.entries(all)) {
    // NOT HOLLOW. A prompt with no purpose AND no variant note is a heading.
    assert.ok(b.hasPurpose || b.notedVariants.length > 0,
      `${type}: the prompt carries neither a purpose nor a single variant note, so it ` +
      'names the block and says nothing about it');
    assert.ok(b.variants.length > 0, `${type}: no variants read from the module`);
    variantCount += b.variants.length;
    noted += b.notedVariants.length;

    // The prompt must name the block's REAL variants. This is what stops it
    // drifting: add a variant to the module and the prompt gains it, or this
    // fails.
    for (const v of b.variants) {
      assert.ok(b.prompt.includes(v), `${type}: the prompt does not name variant ${v}`);
    }
    assert.ok(b.prompt.includes(STYLE), `${type}: the shared style block is missing`);

    // NO DESIGN VALUE may reach a prompt. A prompt naming a hex or a px is a
    // fourth design authority - the thing [2026] VJS-CC-OPBOX 3 D1 forbids -
    // and it would be one nobody could see, because prompts are prose.
    // The token NAMES (--radius-sm, --text-*) are references, not values.
    const body = b.prompt.replace(/--[a-z][\w-]*/g, '');
    assert.doesNotMatch(body, /#[0-9a-fA-F]{3,8}\b/,
      `${type}: the prompt contains a hex colour, which is a design realisation`);
  }

  // Every variant carries a note, and the number is asserted rather than
  // described: 86 of 86 today, and a new variant with no comment fails here
  // rather than silently lowering the rate.
  assert.equal(noted, variantCount,
    `${noted} of ${variantCount} variants carry a note. A variant with no comment beside ` +
    'it produces a prompt that lists its name and explains nothing.');
  assert.ok(variantCount >= 86, `only ${variantCount} variants covered`);

  // The eleven divergent blocks must SAY they diverge. Telling a generator the
  // two sides correspond, where measurement says they do not, is worse than
  // saying nothing: it invents a mapping.
  const divergent = Object.entries(manifest).filter(([, m]) => m.axisVerdict === 'different_axis');
  assert.ok(divergent.length >= 11, `${divergent.length} divergent blocks, expected at least 11`);
  for (const [type] of divergent) {
    assert.match(all[type].prompt, /VARY DIFFERENT THINGS/,
      `${type}: Figma and the code vary different things and the prompt does not say so`);
  }
});

// ---------------------------------------------------------------------------
// Four projections of ONE manifest. The property under test is not that each
// renders - it is that they cannot DISAGREE, which is the only reason to call
// them projections rather than four builders.
test('the four projections agree about what is on the page', () => {
  const { project, projectAll, KINDS } = require('../projections.js');
  const dir = path.join(__dirname, '..');
  const manifest = JSON.parse(fs.readFileSync(path.join(dir, 'manifests', 'kitchen-sink.json'), 'utf8'));
  const pack = JSON.parse(fs.readFileSync(path.join(dir, 'tokens', `${manifest.stylePack}.json`), 'utf8'));
  const tokens = pack.tokens || pack;

  assert.deepEqual(KINDS, ['sitemap', 'wireframe', 'branded', 'output']);
  const all = projectAll(manifest, tokens);
  for (const k of KINDS) assert.ok(!all[k].refused, `${k} refused on a clean manifest: ${all[k].refused}`);

  // THE AGREEMENT. The wireframe labels every block from the manifest, in
  // order, and the sitemap lists the same ones. A wireframe drawn separately
  // from the page is a drawing of a page that may not exist - that was the
  // defect, and this is the assertion that closes it.
  const expected = manifest.page.map((e) => `${e.block}/${e.variant}`);
  const labelled = [...all.wireframe.html.matchAll(/data-block="([^"]+)" data-variant="([^"]+)"/g)]
    .map((m) => `${m[1]}/${m[2]}`);
  assert.deepEqual(labelled, expected,
    'the wireframe does not label the manifest blocks in manifest order');
  assert.deepEqual(all.sitemap.rows.map((r) => `${r.block}/${r.variant}`), expected,
    'the sitemap and the manifest disagree');

  // branded and output are the SAME BYTES. If they ever differ, `output` has
  // become a second renderer and the refusal is no longer about the page that
  // ships.
  assert.equal(all.output.html, all.branded.html,
    'output and branded produced different markup - output is meant to be branded ' +
    'plus a refusal, not a different build');

  // The wireframe keeps every byte of markup and only overrides the token
  // layer. Same length modulo the labels it adds.
  const unlabelled = all.wireframe.html.replace(/ data-block="[^"]*" data-variant="[^"]*"/g, '');
  assert.equal(unlabelled, all.branded.html,
    'the wireframe changed the markup. It must re-paint by redefining tokens, never ' +
    'by rewriting the page, or it stops being a projection of it.');
  assert.match(all.wireframe.css, /--color-accent:\s*#71717a/,
    'the wireframe stylesheet no longer neutralises the accent');

  // THE REFUSAL, seeded. `branded` renders a placeholder and notes it, because
  // that is what you want while writing; `output` refuses, because eleven
  // CONFIRM markers once reached a live client site in running body copy.
  const seeded = JSON.parse(JSON.stringify(manifest));
  const target = seeded.page.find((e) => e.content && typeof e.content.h1 === 'string')
    || seeded.page[0];
  const key = Object.keys(target.content).find((k) => typeof target.content[k] === 'string');
  assert.ok(key, 'no string content to seed - the seed must land or this proves nothing');
  target.content[key] = 'CONFIRM: seeded';

  const after = projectAll(seeded, tokens);
  assert.ok(!after.branded.refused, 'branded must still render while copy is unwritten');
  assert.ok(after.branded.gaps.length > 0, 'branded must NOTE the placeholder it renders');
  assert.ok(after.output.refused, 'output rendered a page carrying a CONFIRM placeholder');
  assert.match(after.output.refused, /placeholder/i);
  // And the other three survive the refusal, which is why projectAll returns it.
  assert.ok(after.sitemap.rows.length > 0 && after.wireframe.html,
    'one projection refusing must not lose the other three');

  // A manifest naming a block that does not exist fails at the SITEMAP, which
  // is the cheapest projection, rather than three projections later.
  const bad = JSON.parse(JSON.stringify(manifest));
  bad.page[0].variant = 'no-such-variant';
  assert.throws(() => project('sitemap', bad, tokens), /do not exist|NO SUCH/i);
});

// ---------------------------------------------------------------------------
// The JS half, witnessed. Measured 2026-07-31: NOT ONE site-factory file was in
// `.vds/enforcement.lock`. `vds_proof::GATE_PATHS` - the list the lock's
// UNPINNED report walks - is one path per PROOF KIND, so it is blind to this
// half by construction and could never have nagged about it. 110 tests, a
// runner and the floors, none of them witnessed.
//
// This test checks the pins from the JS side, so the JS half does not depend on
// the Rust binary having been built to know whether it is protected.
test('every declared site-factory gate is pinned, at its current digest', () => {
  const crypto = require('node:crypto');
  const { GATES, lockFindings } = require('./floors.js');
  const repoRoot = path.join(__dirname, '..', '..');
  const lockPath = path.join(repoRoot, '.vds', 'enforcement.lock');
  const lockText = fs.readFileSync(lockPath, 'utf8');

  assert.ok(GATES.length >= 3, 'the declared gate surface has shrunk below three');

  for (const gate of GATES) {
    const file = path.join(repoRoot, gate);
    assert.ok(fs.existsSync(file), `${gate} is declared a gate and does not exist`);

    // Pinned at all.
    assert.ok(lockText.includes(`path: ${gate}\n`),
      `${gate} decides a pass or a fail and is in no enforcement lock entry. Pin it:\n` +
      `  vds lock add ${gate} --kind hook --invoked-by ... --test-path ... --test-name ...`);

    // AND at the digest it currently has. Pinning without this is a check that
    // the file was pinned ONCE, which is not what the lock is for: the whole
    // point is catching an edit that was never re-pinned.
    const digest = crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
    assert.ok(lockText.includes(`sha256:${digest}`),
      `${gate} has drifted from its pinned digest. Its current content hashes to\n` +
      `  sha256:${digest}\n` +
      'which appears nowhere in the lock. Re-pin it with a rationale:\n' +
      `  vds lock repin --rationale "..."`);
  }

  // THE NEGATIVE CONTROL. Everything above passes trivially if GATES is empty
  // or if the lock file is empty, and both are silent failures.
  assert.match(lockText, /^entries:/m, 'the lock has no entries block');
  assert.ok(lockText.split('\n- path:').length > 10,
    'the lock looks nearly empty; the assertions above would pass over nothing');

  // BOTH BRANCHES, ON A FIXTURE. Seeding the UNPINNED branch through the real
  // artefacts is impossible: adding a gate to the list edits floors.js, so its
  // own digest drifts and the drift assertion fires first. That branch had
  // never run. Driving the pure function directly is the only way to know it
  // works, and a branch nothing reaches is untested however green the suite is.
  // c/drifted.js must be IN the lock with a DIFFERENT digest. My first fixture
  // left it out entirely, which makes it unpinned rather than drifted - so the
  // drift branch went untested on the very attempt to test it.
  const lock = 'entries:\n- path: a/pinned.js\n  digest: sha256:aaa\n' +
    '- path: c/drifted.js\n  digest: sha256:old\n';
  const found = lockFindings(
    ['a/pinned.js', 'b/unpinned.js', 'c/drifted.js', 'd/gone.js'],
    lock,
    (g) => ({ 'a/pinned.js': 'aaa', 'c/drifted.js': 'ccc' }[g] ?? (g === 'd/gone.js' ? null : 'bbb')),
  );
  assert.deepEqual(found.unpinned, ['b/unpinned.js'], 'the unpinned branch does not fire');
  assert.deepEqual(found.drifted.map((d) => d.gate), ['c/drifted.js'],
    'the drifted branch does not fire');
  assert.deepEqual(found.missingFile, ['d/gone.js'], 'a declared gate that is gone must be named');
  // And clean when everything is in order, so it is not a function that always finds fault.
  assert.deepEqual(
    lockFindings(['a/pinned.js'], lock, () => 'aaa'),
    { missingFile: [], unpinned: [], drifted: [] },
  );
});
