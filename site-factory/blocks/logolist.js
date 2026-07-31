'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

/*
 * Relume gives Logos only 6 sets and it appears on very nearly every marketing page it
 * ships, which is the clearest case in the whole comparison of variant count NOT tracking
 * importance. site-factory had nothing for it.
 *
 * The strip does one job: it borrows credibility. So the honest question a caller has to
 * answer is WHOSE, and the block refuses to be vague about it. `caption` is required in
 * practice because "trusted by" over eight logos and "the stack we build on" over the same
 * eight logos are different claims, and only one of them may be true.
 *
 * Rendered as text wordmarks, not images. A generated site has no logo files, and a grey
 * box where a logo goes is worse than the name of the company set in the page's own type:
 * the box says "an image failed", the wordmark says "this is a placeholder for Acme".
 */

// logolist-1: one row, captioned. The default, because a single row reads as a footnote to
// the claim above it rather than as a section demanding attention.
function logoRow(content) {
  const items = (content.logos || []).map((l) => {
    const name = `<span class="logos__mark">${esc(l.name)}</span>`;
    return l.href ? `<a class="logos__link" href="${esc(l.href)}">${name}</a>` : name;
  }).join('\n    ');
  return `<section class="logos">
  <p class="logos__caption">${esc(content.caption || '')}</p>
  <div class="logos__row">
    ${items}
  </div>
</section>`;
}

/*
 * logolist-2: a grid, with the claim promoted to a heading.
 *
 * Worth its own variant because past about six logos a row either wraps unevenly or shrinks
 * every wordmark to fit, and both of those read as a technical failure. A grid is the shape
 * that admits the list is long. It also carries the count, since "40+ teams" is a stronger
 * claim than eight logos and costs nothing to state when it is true.
 */
function logoGrid(content) {
  const items = (content.logos || []).map((l) => {
    const name = `<span class="logos__mark">${esc(l.name)}</span>`;
    return `<div class="logos__cell">${l.href ? `<a class="logos__link" href="${esc(l.href)}">${name}</a>` : name}</div>`;
  }).join('\n    ');
  return `<section class="logos logos--grid">
  <h2 class="logos__heading">${esc(content.heading || '')}</h2>
  ${content.total ? `<p class="logos__caption">${esc(content.total)}</p>` : ''}
  <div class="logos__grid">
    ${items}
  </div>
</section>`;
}

module.exports = {
  'logolist-1': logoRow,
  'logolist-2': logoGrid,
};
