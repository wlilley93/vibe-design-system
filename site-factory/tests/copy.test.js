'use strict';

/*
 * The voice layer must reach the artefact, and what it writes must pass the tests
 * the research itself sets.
 *
 * Two separate obligations, easy to confuse:
 *   - COMPLETENESS: what is still unwritten must be marked and countable, never
 *     silently invented. auditCopy() answers that.
 *   - QUALITY: what IS written must not be a line any competitor could paste onto
 *     their own site unchanged. bannedWords() answers that.
 *
 * A generator that scores well on the first and badly on the second is the exact
 * failure the source research names Register B.
 */

const test = require('node:test');
const assert = require('node:assert');

const { suggest } = require('../suggest.js');
const { configToTokens, configToManifest } = require('../compose.js');
const { renderPage } = require('../build.js');
const { auditCopy, bannedWords, BANNED } = require('../copy.js');
const { placeholderContent } = require('../scaffold.js');

function cfg(over = {}) {
  const identity = {
    name: over.name || 'Northgate Trust',
    tagline: over.tagline !== undefined ? over.tagline : 'I draw the structure first',
    category: over.category || 'marketing-site',
    description: over.description !== undefined ? over.description : 'A private trust and estate structuring advisory',
  };
  const c = suggest(identity);
  c.identity = identity;
  c.governance = { vds: false };
  if (over.voice) Object.assign(c.voice, over.voice);
  return c;
}
const html = (c) => renderPage(configToManifest(c), configToTokens(c), null).html;

test('the author\'s own tagline is used verbatim, not rewritten', () => {
  const c = cfg({ tagline: 'I draw the structure first' });
  const hero = configToManifest(c).page.find((p) => p.block === 'hero');
  assert.match(hero.content.h1, /I draw the structure first/, 'the generator replaced the line the author wrote');
});

test('voice.ctaStyle changes the call to action', () => {
  const fact = cfg({ voice: { ctaStyle: 'fact-stated' } });
  const verb = cfg({ voice: { ctaStyle: 'verb-led' } });
  const label = (c) => configToManifest(c).page.find((p) => p.block === 'hero').content.ctaLabel;
  assert.notStrictEqual(label(fact), label(verb), 'ctaStyle produced the same CTA for both options - decorative');
});

test('voice.copyRegister changes the closing line', () => {
  const a = cfg({ voice: { copyRegister: 'A-institutional-authority' } });
  const c = cfg({ voice: { copyRegister: 'C-voice-with-a-face' } });
  const heading = (x) => configToManifest(x).page.find((p) => p.block === 'cta').content.heading;
  assert.notStrictEqual(heading(a), heading(c), 'copyRegister produced identical copy - decorative');
});

test('voice.readingLevel changes wording', () => {
  const plain = cfg({ voice: { readingLevel: 'plain', ctaStyle: 'verb-led' } });
  const tech = cfg({ voice: { readingLevel: 'technical', ctaStyle: 'verb-led' } });
  const label = (c) => configToManifest(c).page.find((p) => p.block === 'hero').content.ctaLabel;
  assert.notStrictEqual(label(plain), label(tech), 'readingLevel produced identical copy - decorative');
});

test('no generated page contains a disqualifying filler word', () => {
  // Straight from the source research: "Does it use solutions, cutting-edge,
  // empower, seamless, unlock? Then cut it."
  for (const category of ['marketing-site', 'saas-app']) {
    for (const copyRegister of ['A-institutional-authority', 'C-voice-with-a-face']) {
      for (const ctaStyle of ['fact-stated', 'verb-led']) {
        const c = cfg({ category, voice: { copyRegister, ctaStyle } });
        const hits = bannedWords(configToManifest(c));
        assert.deepEqual(
          hits, [],
          `banned word in generated copy (${category}/${copyRegister}/${ctaStyle}): ` +
          hits.map((h) => `${h.word} at ${h.where}`).join('; ')
        );
      }
    }
  }
});

test('the banned-word check can actually fire', () => {
  // A check that never fires is not a check. Prove it catches a planted line.
  const planted = { page: [{ variant: 'hero-1', content: { h1: 'Innovative solutions that empower teams' } }] };
  const hits = bannedWords(planted);
  assert.ok(hits.length >= 1, 'the banned-word scan missed a line built from its own list');
  assert.ok(BANNED.includes(hits[0].word.toLowerCase()));
});

test('what cannot be derived is marked CONFIRM, and every marker carries an instruction', () => {
  const c = cfg();
  const gaps = auditCopy(configToManifest(c));
  assert.ok(gaps.length > 0, 'a one-line brief cannot supply features, FAQ and pricing - those must be marked');

  // Scoped to the markers copy.js itself writes. auditCopy deliberately also returns
  // scaffold.js's neutral "Replace this…" placeholders so the count is not an
  // undercount, but those are a different contract: they are a blank, whereas a
  // CONFIRM is a blank WITH the instruction for filling it. Only the second is this
  // test's subject.
  const confirms = gaps.filter((g) => /^CONFIRM:/.test(g.value));
  assert.ok(confirms.length > 0, 'the voice layer produced no CONFIRM markers at all');
  for (const g of confirms) {
    assert.match(g.value, /^CONFIRM: .+/, `a marker with no instruction is not actionable: ${g.value}`);
    assert.ok(g.value.length > 'CONFIRM: '.length + 8, `instruction too thin to act on: ${g.value}`);
  }
});

test('a brief with no tagline marks the headline rather than manufacturing one', () => {
  const c = cfg({ tagline: '', description: '' });
  const hero = configToManifest(c).page.find((p) => p.block === 'hero');
  assert.match(hero.content.h1, /^CONFIRM:/, 'an empty brief produced a headline nobody wrote');
});

test('the blocks the voice layer governs carry no "Replace this" placeholder', () => {
  const c = cfg();
  const m = configToManifest(c);
  for (const entry of m.page) {
    if (!['hero', 'cta', 'nav', 'footer'].includes(entry.block)) continue;
    const json = JSON.stringify(entry.content);
    assert.ok(
      !/Replace this|Replace me/i.test(json),
      `${entry.variant} still carries a neutral placeholder the voice layer should have written`
    );
  }
});

test('generated copy survives rendering intact', () => {
  const out = html(cfg());
  assert.ok(out.includes('I draw the structure first'), 'the authored line did not reach the page');
  assert.ok(out.includes('CONFIRM:'), 'the CONFIRM markers must be visible in the output, not swallowed');
});

test('the audit counts BOTH unwritten conventions, not just its own', () => {
  // copy.js marks CONFIRM:; scaffold.js leaves "Replace this…" on the blocks copy.js
  // does not govern. Counting only the first reported 12 lines to write on a page
  // that really had 17 - pricing and testimonials sat there uncounted. An undercount
  // reads as a finished audit, which is worse than no audit.
  const c = cfg();
  c.strategy.sitemap = ['hero-1', 'features-1', 'pricing-1', 'testimonials-1'];
  const gaps = auditCopy(configToManifest(c));
  const kinds = gaps.reduce((a, g) => {
    const k = /^CONFIRM:/.test(g.value) ? 'confirm' : 'replace';
    a[k] = (a[k] || 0) + 1;
    return a;
  }, {});
  assert.ok(kinds.confirm > 0, 'expected CONFIRM markers from the voice layer');
  assert.ok(kinds.replace > 0, 'the audit missed scaffold.js "Replace this" placeholders entirely');
});

test('a fully written page audits clean', () => {
  // The audit must be able to reach zero, or it is a counter nobody can satisfy.
  const written = {
    page: [{
      variant: 'hero-1',
      content: { h1: 'A real headline.', sub: 'A real subhead.', ctaLabel: 'The approach' },
    }],
  };
  assert.deepEqual(auditCopy(written), []);
});

test('no source file uses an em dash as prose punctuation', () => {
  // A standing writing rule that lives only in someone's head is not a rule, it is a
  // habit - 220 of these had accumulated across 40 files before anyone counted. Prose
  // is not enforcement; a test is.
  //
  // Scoped to the SPACED form (space, U+2014, space), which is the punctuation use. A
  // bare U+2014 as an empty-cell glyph in a table or a UI placeholder is a different
  // thing: it means "nothing here", and a hyphen there reads as a minus sign. Those
  // are left alone deliberately, and this test is narrow so it does not sweep them up.
  //
  // The needle is built from its code point rather than typed. A guard written with a
  // literal copy of the thing it bans FLAGS ITS OWN SOURCE, and the only ways out are
  // to except the guard from itself - which is how a check quietly stops covering the
  // file most likely to be edited - or to not write the character at all.
  const fs = require('node:fs');
  const path = require('node:path');

  const NEEDLE = ` ${String.fromCharCode(0x2014)} `;
  const ROOT = path.join(__dirname, '..');
  const SKIP = new Set(['node_modules', 'scaffolds', 'dist', '.vds', '.git']);
  const offenders = [];

  (function walk(dir) {
    for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
      if (SKIP.has(e.name)) continue;
      const full = path.join(dir, e.name);
      if (e.isDirectory()) { walk(full); continue; }
      if (!/\.(js|md|html|json)$/.test(e.name)) continue;
      const text = fs.readFileSync(full, 'utf8');
      const n = text.split(NEEDLE).length - 1;
      if (n) offenders.push(`${path.relative(ROOT, full)} (${n})`);
    }
  })(ROOT);

  assert.deepEqual(offenders, [],
    `em dash used as prose punctuation - replace with a comma, a colon, or a spaced hyphen:\n  ${offenders.join('\n  ')}`);
});
