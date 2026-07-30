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
  assert.notStrictEqual(label(fact), label(verb), 'ctaStyle produced the same CTA for both options — decorative');
});

test('voice.copyRegister changes the closing line', () => {
  const a = cfg({ voice: { copyRegister: 'A-institutional-authority' } });
  const c = cfg({ voice: { copyRegister: 'C-voice-with-a-face' } });
  const heading = (x) => configToManifest(x).page.find((p) => p.block === 'cta').content.heading;
  assert.notStrictEqual(heading(a), heading(c), 'copyRegister produced identical copy — decorative');
});

test('voice.readingLevel changes wording', () => {
  const plain = cfg({ voice: { readingLevel: 'plain', ctaStyle: 'verb-led' } });
  const tech = cfg({ voice: { readingLevel: 'technical', ctaStyle: 'verb-led' } });
  const label = (c) => configToManifest(c).page.find((p) => p.block === 'hero').content.ctaLabel;
  assert.notStrictEqual(label(plain), label(tech), 'readingLevel produced identical copy — decorative');
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
  assert.ok(gaps.length > 0, 'a one-line brief cannot supply features, FAQ and pricing — those must be marked');

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
  // that really had 17 — pricing and testimonials sat there uncounted. An undercount
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
