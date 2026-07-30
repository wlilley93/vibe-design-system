'use strict';

/*
 * skills.js: hand the writing to the skills, which is the point of the AI/MCP layer.
 *
 * copy.js writes what a one-line brief genuinely supports and marks the rest
 * `CONFIRM:`. Those markers are not meant to be filled by hand — they are the work
 * queue for the content skills in agents-final, which is where the actual writing
 * craft lives. This file is the seam between the two.
 *
 * The skills are real and were read, not assumed:
 *
 *   revenue/skills/headline-lab                 hero headline, pre-headline, subhead
 *   revenue/skills/cta-and-close-writer         button copy and the closing argument
 *   revenue/skills/pricing-section-designer     the pricing section
 *   revenue/skills/proof-and-testimonial-engine testimonials and proof
 *   product/skills/objection-and-faq-engine     the FAQ, written from real objections
 *   revenue/skills/offer-clarifier              the offer, before any of the above
 *
 * Every one of them opens by reading a shared `config.json` written by
 * revenue/skills/sales-page-setup, so a skill never has to re-interview the user.
 * PACK_CONFIG below emits exactly that shape. The field names are copied from that
 * skill's own JSON block, not invented, because a near-miss field name would make
 * every downstream skill silently fall back to interviewing.
 *
 * The discipline is the same one copy.js already follows, and it is the setup
 * skill's own instruction: "Fill every field you can; leave honest blanks rather
 * than inventing details."
 */

const { BANNED } = require('./copy.js');

const SKILL_ROOT = 'skill-library';

// Which skill writes which block. A block with no entry is one no skill in the pack
// speaks for; it keeps its neutral placeholder rather than being assigned to a skill
// that would be guessing.
const BLOCK_SKILLS = {
  hero: { skill: 'revenue/skills/headline-lab', writes: 'the headline, pre-headline and subhead' },
  cta: { skill: 'revenue/skills/cta-and-close-writer', writes: 'the button copy and the closing argument' },
  faq: { skill: 'product/skills/objection-and-faq-engine', writes: 'the FAQ, from the objections that actually stop buyers' },
  pricing: { skill: 'revenue/skills/pricing-section-designer', writes: 'the pricing section and its framing' },
  testimonials: { skill: 'revenue/skills/proof-and-testimonial-engine', writes: 'proof and testimonial copy' },
  features: { skill: 'revenue/skills/offer-stack-builder', writes: 'the capability stack and what each one means for the reader' },
};

// The pack's own order: the offer has to be clear before anything is written about
// it, and the close comes after pricing. Named here so an agent runs them in the
// order the skills themselves declare rather than in block order.
const RUN_ORDER = [
  'revenue/skills/offer-clarifier',
  'revenue/skills/headline-lab',
  'revenue/skills/offer-stack-builder',
  'revenue/skills/proof-and-testimonial-engine',
  'revenue/skills/pricing-section-designer',
  'product/skills/objection-and-faq-engine',
  'revenue/skills/cta-and-close-writer',
];

const TONE_WORDS = {
  'A-institutional-authority': ['authoritative', 'precise', 'unhurried', 'plain'],
  'C-voice-with-a-face': ['warm', 'direct', 'specific', 'human'],
};

const BLANK = '';

/*
 * site-factory config -> the sales-page pack's config.json.
 *
 * Deliberately leaves blanks. site-factory's 35 fields describe a SITE — its route,
 * palette, blocks, voice register. They do not describe a market: nothing in them
 * knows the buyer's awareness level, the biggest objection, or what the buyer does
 * instead. Guessing those would poison every skill downstream, because each one
 * treats config.json as settled fact and will not re-ask.
 *
 * A blank is a prompt for the setup interview. An invented value is a lie the whole
 * pack then builds on.
 */
function packConfig(config) {
  const id = config.identity || {};
  const voice = config.voice || {};
  return {
    brand: {
      brand_name: id.name || BLANK,
      website: BLANK,
      niche: BLANK,
      audience: BLANK,
      offer: id.description || BLANK,
      transformation: id.tagline || BLANK,
      price_point: BLANK,
    },
    voice: {
      tone_words: TONE_WORDS[voice.copyRegister] || [],
      we_sound_like: voice.copyRegister === 'A-institutional-authority'
        ? 'A firm that states a fact and does not ask for the meeting.'
        : 'A person who names the thing you are replacing, warmly and specifically.',
      // A direct, real mapping: the pack's "never say" list is the same constraint
      // tests/copy.test.js already enforces, so the two cannot drift apart.
      we_never_say: BANNED.join(', '),
      reading_level: voice.readingLevel === 'technical' ? 'technical' : 'simple',
    },
    market: { awareness_level: BLANK, biggest_objection: BLANK, main_alternative: BLANK },
    proof: { has_testimonials: false, key_results: BLANK, credentials: BLANK },
    tools: { design: 'Figma', ai_media: BLANK, planner: BLANK, email: BLANK },
    preferences: {
      page_length: config.identity && config.identity.category === 'saas-app'
        ? 'app surface (no long-form sales page)'
        : 'long-form (full sales page with all sections)',
      save_outputs_to: 'manifests/home.json in this project',
    },
  };
}

function blankFields(pc) {
  const out = [];
  for (const [group, fields] of Object.entries(pc)) {
    if (!fields || typeof fields !== 'object') continue;
    for (const [k, v] of Object.entries(fields)) {
      const empty = v === '' || (Array.isArray(v) && v.length === 0);
      if (empty) out.push(`${group}.${k}`);
    }
  }
  return out;
}

/*
 * The work order: every unwritten line, grouped under the skill that writes it.
 * `gaps` is auditCopy()'s output, so the two stay in step by construction.
 */
function assignments(manifest, gaps) {
  const blockOf = (where) => {
    const m = where.match(/^page\[(\d+)\]/);
    if (!m) return null;
    const entry = (manifest.page || [])[Number(m[1])];
    return entry ? entry.block : null;
  };
  const bySkill = new Map();
  const unassigned = [];
  for (const g of gaps) {
    const block = blockOf(g.where);
    const spec = block && BLOCK_SKILLS[block];
    if (!spec) { unassigned.push({ ...g, block }); continue; }
    if (!bySkill.has(spec.skill)) bySkill.set(spec.skill, { skill: spec.skill, writes: spec.writes, blocks: new Set(), gaps: [] });
    const bucket = bySkill.get(spec.skill);
    bucket.blocks.add(block);
    bucket.gaps.push(g);
  }

  /*
   * Skills whose block IS on the page and HAS no unwritten line.
   *
   * Gap-filling is only half of what these skills do. headline-lab's actual value is
   * generating fifteen headlines across proven formulas and scoring them on a rubric
   * — that is worth running against a headline the author already wrote, and a brief
   * that only ever lists blanks would never offer it. Copy that exists is not
   * automatically copy that works.
   */
  const withGaps = new Set([...bySkill.keys()]);
  const improvable = [];
  const seenBlocks = new Set();
  for (const entry of manifest.page || []) {
    const spec = BLOCK_SKILLS[entry.block];
    if (!spec || seenBlocks.has(entry.block)) continue;
    seenBlocks.add(entry.block);
    if (withGaps.has(spec.skill)) continue;
    improvable.push({ skill: spec.skill, writes: spec.writes, block: entry.block });
  }

  return {
    bySkill: [...bySkill.values()]
      .map((b) => ({ ...b, blocks: [...b.blocks] }))
      .sort((a, b) => RUN_ORDER.indexOf(a.skill) - RUN_ORDER.indexOf(b.skill)),
    unassigned,
    improvable: improvable.sort((a, b) => RUN_ORDER.indexOf(a.skill) - RUN_ORDER.indexOf(b.skill)),
  };
}

function briefMarkdown(config, manifest, gaps) {
  const pc = packConfig(config);
  const blanks = blankFields(pc);
  const { bySkill, unassigned, improvable } = assignments(manifest, gaps);

  const lines = [];
  lines.push(`# ${config.identity.name} — writing brief`);
  lines.push('');
  lines.push(`${gaps.length} lines are unwritten. They are not a manual to-do list: each one belongs to a`);
  lines.push('content skill in agents-final, which is where the writing craft lives.');
  lines.push('');
  lines.push('## Before any skill runs');
  lines.push('');
  lines.push('`copy-brief.json` is this project mapped into the shape every skill in the pack reads');
  lines.push('(`revenue/skills/sales-page-setup` writes the canonical one). It is deliberately');
  lines.push('incomplete: site-factory knows the site, not the market.');
  lines.push('');
  if (blanks.length) {
    lines.push(`**${blanks.length} fields are blank and every skill downstream depends on them:**`);
    lines.push('');
    for (const b of blanks) lines.push(`- \`${b}\``);
    lines.push('');
    lines.push('Run `revenue/skills/sales-page-setup` to fill these by interview. Guessing them');
    lines.push('would be worse than leaving them: each skill treats this file as settled fact and');
    lines.push('will not re-ask.');
  } else {
    lines.push('Every field is filled.');
  }
  lines.push('');
  lines.push('## Run order');
  lines.push('');
  lines.push('The pack declares its own order — the offer must be clear before anything is written');
  lines.push('about it, and the close comes after pricing.');
  lines.push('');
  for (const s of bySkill) {
    lines.push(`### \`${SKILL_ROOT}/${s.skill}\``);
    lines.push('');
    lines.push(`Writes ${s.writes}. Blocks: ${s.blocks.join(', ')}. ${s.gaps.length} lines.`);
    lines.push('');
    for (const g of s.gaps) lines.push(`- \`${g.where}\` — ${g.value}`);
    lines.push('');
  }
  if (improvable.length) {
    lines.push('## Written, but not yet worked');
    lines.push('');
    lines.push('These blocks have copy. That is not the same as copy that works — these skills');
    lines.push('generate alternatives across proven formulas and score them, which is worth doing');
    lines.push('against a line that already exists.');
    lines.push('');
    for (const s of improvable) {
      lines.push(`- \`${SKILL_ROOT}/${s.skill}\` — ${s.writes} (${s.block})`);
    }
    lines.push('');
  }
  if (unassigned.length) {
    lines.push('## No skill assigned');
    lines.push('');
    lines.push('No skill in this pack speaks for these. Write them directly, or leave them.');
    lines.push('');
    for (const g of unassigned) lines.push(`- \`${g.where}\` — ${g.value}`);
    lines.push('');
  }
  return lines.join('\n');
}

module.exports = { packConfig, blankFields, assignments, briefMarkdown, BLOCK_SKILLS, RUN_ORDER, SKILL_ROOT };
