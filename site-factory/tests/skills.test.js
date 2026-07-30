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
  // id. Neither opens the file, so a deleted component set failed nothing. That is not
  // hypothetical here: on 2026-07-30 seven pages of this same file were found empty having
  // been recorded as built, and the only reason no pairing broke is that what was lost was
  // frames rather than component sets. The id-shape check would not have told us either way.
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
