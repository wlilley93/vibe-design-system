'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

/*
 * The largest single gap in the Relume comparison: 60 sets, and `card` was the nearest thing
 * site-factory had. A card renders ONE figure; this is the ROW, and the row is a different
 * component because the figures are read against each other.
 *
 * The rule this block encodes, and the reason it is easy to get wrong: A FIGURE WITHOUT A
 * BASIS IS DECORATION. "99.9%" means nothing; "99.9% uptime, measured over the last 12
 * months" is a claim someone can check and therefore a claim worth making. So every stat
 * carries a `basis` and the block renders it, rather than leaving a caller to put four large
 * numbers on a page and hope they land.
 */

function stat(s, i) {
  return `    <div class="stats__item">
      <p class="stats__figure">${esc(s.figure)}</p>
      <p class="stats__label">${esc(s.label)}</p>
      ${s.basis ? `<p class="stats__basis">${esc(s.basis)}</p>` : ''}
    </div>`;
}

// stats-1: the row. For three or four figures that share a unit or a period, where the
// comparison between them is the content.
function statsRow(content) {
  const items = (content.stats || []).map(stat).join('\n');
  return `<section class="stats">
  ${content.heading ? `<h2 class="stats__heading">${esc(content.heading)}</h2>` : ''}
  <div class="stats__row">
${items}
  </div>
</section>`;
}

/*
 * stats-2: figures beside a paragraph that says what they mean.
 *
 * Not a layout preference. A bare row of numbers asks the reader to draw the conclusion, and
 * on a marketing page they usually will not: they will register that the numbers are large
 * and move on. Putting the argument next to the evidence is what turns a stat row into a
 * claim. Use this when the numbers need interpreting and the row when they do not.
 */
function statsWithClaim(content) {
  const items = (content.stats || []).map(stat).join('\n');
  return `<section class="stats stats--claim">
  <div class="stats__text">
    <h2 class="stats__heading">${esc(content.heading || '')}</h2>
    <p class="stats__body">${esc(content.body || '')}</p>
    ${content.ctaLabel ? `<a class="stats__cta" href="${esc(content.ctaHref || '#')}">${esc(content.ctaLabel)}</a>` : ''}
  </div>
  <div class="stats__grid">
${items}
  </div>
</section>`;
}

module.exports = {
  'stats-1': statsRow,
  'stats-2': statsWithClaim,
};
