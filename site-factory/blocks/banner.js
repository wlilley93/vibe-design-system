'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

/*
 * Page-level state that is not an error. Base gives it 72 variants and four tones, and
 * the tones are the component: a banner that cannot change colour cannot say whether the
 * thing it reports is good news.
 *
 * The tone is a NAME, never a colour. `tone: 'warning'` resolves to --color-warning and
 * --color-warningInk, and warning is the one tone whose accessible ink is dark rather
 * than light - measured, and the reason the ink is a per-tone token instead of a single
 * "on-colour". A system with one ink for every tone either fails contrast on amber or
 * washes out on red.
 *
 * The ARIA role follows the tone rather than the layout, which is the part usually got
 * wrong: `alert` interrupts whatever a screen reader is saying, so it is right for
 * something gone wrong and rude for a tip. Everything else is a polite `status`.
 */

const TONES = new Set(['accent', 'success', 'warning', 'danger', 'info']);
function toneOf(content) {
  return TONES.has(content.tone) ? content.tone : 'info';
}
// `danger` is the only tone that earns an interruption.
function roleOf(tone) {
  return tone === 'danger' ? 'alert' : 'status';
}

/*
 * banner-1: the full banner - headline, a line of explanation, an action. For state a
 * reader has to understand before deciding, which is why it gets room to explain and one
 * button rather than three.
 */
function bannerFull(content) {
  const tone = toneOf(content);
  const action = content.actionLabel
    ? `    <a class="banner__action" href="${esc(content.actionHref || '#')}">${esc(content.actionLabel)}</a>\n`
    : '';
  return `<div class="banner banner--${tone}" role="${roleOf(tone)}">
  <span class="banner__mark" aria-hidden="true"></span>
  <div class="banner__text">
    <p class="banner__headline">${esc(content.headline || '')}</p>
    <p class="banner__body">${esc(content.body || '')}</p>
  </div>
${action}</div>`;
}

/*
 * banner-2: one line, no headline. For state that needs saying and not explaining.
 *
 * Worth a separate variant rather than an empty `body`, because the one-line form is what
 * belongs directly above a table or a form: it does not push the thing it is about below
 * the fold. A full banner used that way is a banner that costs the reader the content.
 */
function bannerInline(content) {
  const tone = toneOf(content);
  const action = content.actionLabel
    ? `  <a class="banner__action" href="${esc(content.actionHref || '#')}">${esc(content.actionLabel)}</a>\n`
    : '';
  return `<div class="banner banner--inline banner--${tone}" role="${roleOf(tone)}">
  <span class="banner__mark" aria-hidden="true"></span>
  <p class="banner__body">${esc(content.body || '')}</p>
${action}</div>`;
}

module.exports = {
  'banner-1': bannerFull,
  'banner-2': bannerInline,
};
