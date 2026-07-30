'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

/*
 * The single biggest set in Uber Base at 423 variants, and the one a data surface
 * cannot do without: a table with no pagination is a table that lies about its length.
 *
 * The geometry is measured from Base and it is one cell repeated: a 48 outer with 6
 * padding round a 36 inner at radius 8. THE SELECTION IS THE INNER CHIP, not the outer
 * cell, which is why the current page reads as a chip inside a hit area rather than a
 * full-height block. The hit area stays 48 square either way, which is what makes the
 * control usable on a touch screen without the selected state looking oversized.
 *
 * The two variants are not two looks. They are two different pagination MODELS and the
 * choice is forced by the data, not by taste. See each.
 */

// A cell. Numbered pages are links because each is a place you can bookmark; the
// current one is a span, because a link to where you already are is a dead control.
function cell(page, { current = false, href = '#', label = null } = {}) {
  const text = label || String(page);
  if (current) {
    return `    <li class="pag__cell"><span class="pag__chip pag__chip--on" aria-current="page">${esc(text)}</span></li>`;
  }
  return `    <li class="pag__cell"><a class="pag__chip" href="${esc(href)}">${esc(text)}</a></li>`;
}

// A step control that has nowhere to go is rendered as a span with aria-disabled
// rather than a link, because a link that goes nowhere is worse than an absent one.
function step(text, { href = '#', enabled = true, label } = {}) {
  if (!enabled) {
    return `    <li class="pag__cell"><span class="pag__chip pag__chip--off" aria-disabled="true"><span aria-hidden="true">${esc(text)}</span><span class="pag__sr">${esc(label)}</span></span></li>`;
  }
  return `    <li class="pag__cell"><a class="pag__chip" href="${esc(href)}"><span aria-hidden="true">${esc(text)}</span><span class="pag__sr">${esc(label)}</span></a></li>`;
}

/*
 * pagination-1: NUMBERED. Requires a stable total and stable offsets, and gives you
 * something cursor pagination cannot: a link to page 7 that still means page 7
 * tomorrow, and a reader who can see how much there is.
 *
 * The elision is a real cell rather than a gap, so the row does not change width when
 * it appears. A row that reflows as you page through it is a row whose buttons move
 * under the cursor.
 */
function paginationNumbered(content) {
  const total = Number(content.total) || 1;
  const current = Math.min(Math.max(Number(content.current) || 1, 1), total);
  const href = content.hrefPattern || '#page=';

  // Show the ends and a window round the current page. Everything else elides.
  const window = new Set([1, total, current, current - 1, current + 1]);
  const shown = [...window].filter((n) => n >= 1 && n <= total).sort((a, b) => a - b);

  const cells = [];
  let previous = 0;
  for (const n of shown) {
    if (previous && n - previous > 1) {
      cells.push('    <li class="pag__cell"><span class="pag__chip pag__chip--gap" aria-hidden="true">&hellip;</span></li>');
    }
    cells.push(cell(n, { current: n === current, href: `${href}${n}` }));
    previous = n;
  }

  return `<nav class="pag" aria-label="${esc(content.label || 'Pagination')}">
  <ol class="pag__row">
${step('‹', { href: `${href}${current - 1}`, enabled: current > 1, label: 'Previous page' })}
${cells.join('\n')}
${step('›', { href: `${href}${current + 1}`, enabled: current < total, label: 'Next page' })}
  </ol>
  <p class="pag__status" aria-live="polite">Page ${current} of ${total}</p>
</nav>`;
}

/*
 * pagination-2: CURSOR. Prev and next only, no page numbers and no total.
 *
 * This is not the lesser variant, it is the correct one for a set that changes under
 * the reader: a feed, an audit log, anything ordered by time where a new row at the top
 * shifts every offset by one. Numbered pagination over such a set shows the same record
 * twice and skips another, silently. The absence of a total here is HONEST rather than
 * lazy, and the caption says so instead of leaving the reader to wonder.
 */
function paginationCursor(content) {
  const hasPrev = content.prevHref !== undefined && content.prevHref !== null;
  const hasNext = content.nextHref !== undefined && content.nextHref !== null;
  return `<nav class="pag pag--cursor" aria-label="${esc(content.label || 'Pagination')}">
  <ol class="pag__row">
${step('‹ ' + (content.prevLabel || 'Newer'), { href: content.prevHref || '#', enabled: hasPrev, label: 'Newer results' })}
${step((content.nextLabel || 'Older') + ' ›', { href: content.nextHref || '#', enabled: hasNext, label: 'Older results' })}
  </ol>
  <p class="pag__status">${esc(content.caption || 'Ordered by date. There is no page count because the set changes as you read it.')}</p>
</nav>`;
}

module.exports = {
  'pagination-1': paginationNumbered,
  'pagination-2': paginationCursor,
};
