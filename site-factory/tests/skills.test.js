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
 * The library lives outside this repo, so its absence is a SKIP, not a failure — a
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
      `the brief names ${rel}, which has no SKILL.md — a work order pointing at nothing`
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
    'a gap vanished between the audit and the brief — every line must be accounted for'
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
    'headline-lab must be offered even when the hero already has a line — a brief that only ' +
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
