'use strict';

/*
 * copy.js: make the voice layer reach the page.
 *
 * copyRegister, readingLevel and ctaStyle were in the schema, rotatable in the
 * studio, and read by NOTHING. That is the same defect the design layers had before
 * density and typeScale were wired to the stylesheet - a control the artefact
 * ignores is a control that lies - and it was worse here, because every generated
 * page still said "Replace this headline".
 *
 * WHAT THIS IS NOT: a language model. There is no API key in this repo and this file
 * does not pretend to write marketing copy. It is a rule-based generator, and it
 * obeys one hard constraint:
 *
 *   DERIVE, OR MARK. Anything genuinely derivable from the brief (the project name,
 *   the tagline the author wrote, the one-line description) is used. Anything that
 *   is NOT derivable - feature names, FAQ answers, plan tiers, testimonials - is
 *   emitted as `CONFIRM: <the specific thing needed>` rather than invented.
 *
 * That marker is Balmoral's own convention (site/build/templates.js,
 * `UNCONFIRMED_RE = /\bCONFIRM:/i`), where unverified strings are flagged and in
 * some slots are a hard build failure. It is used here for the same reason: invented
 * filler that READS finished is worse than an honest blank, because nobody goes back
 * for it. `auditCopy()` below makes the remaining gaps countable.
 *
 * The register definitions are Balmoral's competitor research, not invention: 13
 * real sites sorted into three registers (Jellytot docs,
 * brand-reference-teardown-2026-07-28.md, section 05). Register B - the
 * interchangeable "Innovative Software Solutions" middle - is deliberately not
 * offered, because that document names it as the failure mode rather than a style.
 */

// The words that research names as disqualifying. A line containing one of these
// could be pasted onto any competitor's site unchanged, which is the test it fails.
const BANNED = ['solutions', 'cutting-edge', 'cutting edge', 'empower', 'seamless', 'unlock', 'best-in-class', 'world-class', 'synergy', 'leverage'];

const CONFIRM = (what) => `CONFIRM: ${what}`;

function sentence(s) {
  const t = String(s || '').trim();
  if (!t) return '';
  return /[.!?]$/.test(t) ? t : `${t}.`;
}

function stripTrailingStop(s) {
  return String(s || '').trim().replace(/\.$/, '');
}

/*
 * A CTA label. `ctaStyle` decides the grammar, which is the whole point of the
 * field: Register A "states a fact and never asks for the meeting", so its CTA is a
 * noun phrase pointing at a thing, not an imperative asking for time.
 */
function ctaLabel(voice) {
  if (voice.ctaStyle === 'fact-stated') {
    return voice.readingLevel === 'technical' ? 'The documentation' : 'The approach';
  }
  return voice.readingLevel === 'technical' ? 'Read the docs' : 'Start here';
}

/*
 * The hero. h1 is the author's own tagline where they wrote one - no generator
 * beats the line the person actually chose - and a marked CONFIRM where they did
 * not, rather than a manufactured headline.
 */
function heroCopy(identity, voice) {
  const h1 = identity.tagline
    ? sentence(identity.tagline)
    : CONFIRM('one line stating what this is. Register ' +
        (voice.copyRegister === 'A-institutional-authority'
          ? 'A: a fact, no verb needed, no ask'
          : 'C: warm and specific, name the thing you replace'));
  const sub = identity.description
    ? sentence(identity.description)
    : CONFIRM('one sentence naming the consequence, not the capability');
  return { h1, sub, ctaLabel: ctaLabel(voice), ctaHref: '#', mediaAlt: identity.name };
}

/*
 * The closing CTA. Register A restates the claim and points; Register C addresses
 * the reader. Both are built from the tagline the author wrote, so neither invents
 * a promise the project has not made.
 */
function ctaCopy(identity, voice) {
  const claim = stripTrailingStop(identity.tagline);
  const heading = claim
    ? (voice.copyRegister === 'A-institutional-authority' ? `${claim}.` : `${claim} - see how.`)
    : CONFIRM('a closing line that repeats the single claim this site makes');
  return {
    heading,
    sub: identity.description
      ? sentence(identity.description)
      : CONFIRM('one sentence on what happens next'),
    ctaLabel: ctaLabel(voice),
    ctaHref: '#',
    formAction: '#',
    placeholder: 'Email address',
  };
}

function navCopy(identity) {
  return {
    wordmark: identity.name,
    links: [{ label: 'Approach', href: '#' }, { label: 'Work', href: '#' }, { label: 'Contact', href: '#' }],
    ctaLabel: 'Contact',
    ctaHref: '#',
  };
}

function footerCopy(identity, voice) {
  return {
    wordmark: identity.name,
    tagline: identity.tagline ? sentence(identity.tagline) : CONFIRM('one line for the footer'),
    copyright: `© 2026 ${identity.name}`,
    links: [{ label: 'Approach', href: '#' }, { label: 'Contact', href: '#' }],
    columns: [{ title: 'Site', links: [{ label: 'Approach', href: '#' }, { label: 'Contact', href: '#' }] }],
  };
}

/*
 * Everything below is NOT derivable from a one-line brief. A features grid needs the
 * actual features; an FAQ needs the actual questions people ask; pricing needs real
 * tiers. Inventing them would produce exactly the interchangeable middle the
 * research says to avoid, and it would read as finished. So each is marked.
 */
function featuresCopy() {
  return {
    heading: CONFIRM('the heading over the three things this offers'),
    items: [
      { title: CONFIRM('first capability'), body: CONFIRM('what it means for the reader, one sentence') },
      { title: CONFIRM('second capability'), body: CONFIRM('what it means for the reader, one sentence') },
      { title: CONFIRM('third capability'), body: CONFIRM('what it means for the reader, one sentence') },
    ],
    columns: ['Free', 'Pro'],
    rows: [{ label: CONFIRM('a row to compare'), values: [true, true] }],
  };
}

function faqCopy() {
  return {
    heading: 'Questions',
    items: [
      { question: CONFIRM('a question a real prospect asks'), answer: CONFIRM('the honest answer, including the caveat') },
      { question: CONFIRM('the objection you hear most'), answer: CONFIRM('the answer that concedes the easy thing first') },
    ],
  };
}

// One generator per block type. A type with no entry keeps the neutral placeholder
// from scaffold.js, which is the correct fallback: this file speaks for the blocks
// whose copy the voice layer genuinely governs, not for every block that exists.
const GENERATORS = {
  hero: (id, v) => heroCopy(id, v),
  cta: (id, v) => ctaCopy(id, v),
  nav: (id) => navCopy(id),
  footer: (id, v) => footerCopy(id, v),
  features: () => featuresCopy(),
  faq: () => faqCopy(),
};

function copyFor(type, identity, voice) {
  const gen = GENERATORS[type];
  return gen ? gen(identity, voice || {}) : null;
}

/*
 * Count what is still unwritten, and refuse to call a page finished while it is.
 *
 * TWO markers, deliberately, because there are two sources. copy.js writes
 * `CONFIRM:` for the blocks the voice layer governs; scaffold.js writes "Replace
 * this…" neutral placeholders for the blocks it does not (pricing tiers,
 * testimonials, team bios - none of them derivable from a one-line brief).
 *
 * Counting only the first produced an UNDERCOUNT: a page reported 12 lines to write
 * while pricing and testimonials sat there saying "Replace this line", uncounted. An
 * undercount is worse than no count, because it reads as a finished audit. Both
 * conventions mean the same thing to the person holding the list, so both are found.
 */
const UNWRITTEN_RE = /\bCONFIRM:|\bReplace (this|me)\b/i;

function auditCopy(manifest) {
  const found = [];
  const walk = (value, where) => {
    if (typeof value === 'string') {
      if (UNWRITTEN_RE.test(value)) found.push({ where, value });
      return;
    }
    if (Array.isArray(value)) return value.forEach((v, i) => walk(v, `${where}[${i}]`));
    if (value && typeof value === 'object') {
      for (const [k, v] of Object.entries(value)) walk(v, `${where}.${k}`);
    }
  };
  (manifest.page || []).forEach((entry, i) => walk(entry.content, `page[${i}] ${entry.variant}`));
  return found;
}

/*
 * The disqualifying-word check, applied to authored strings. Kept separate from
 * auditCopy because they answer different questions: one asks what is missing, the
 * other asks whether what IS there could be pasted onto a competitor's site.
 */
function bannedWords(manifest) {
  const hits = [];
  const re = new RegExp(`\\b(${BANNED.join('|').replace(/[-]/g, '[- ]')})\\b`, 'i');
  const walk = (value, where) => {
    if (typeof value === 'string') {
      const m = value.match(re);
      if (m) hits.push({ where, word: m[0], value });
      return;
    }
    if (Array.isArray(value)) return value.forEach((v, i) => walk(v, `${where}[${i}]`));
    if (value && typeof value === 'object') {
      for (const [k, v] of Object.entries(value)) walk(v, `${where}.${k}`);
    }
  };
  (manifest.page || []).forEach((entry, i) => walk(entry.content, `page[${i}] ${entry.variant}`));
  return hits;
}

module.exports = { copyFor, auditCopy, bannedWords, ctaLabel, BANNED, CONFIRM, UNWRITTEN_RE };
