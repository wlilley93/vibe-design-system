'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

/*
 * Demand: 12 of 224 Opbox routes.
 *
 * "Card" is the most over-used name in any design system, so this one is narrow on
 * purpose: a card here is a SUMMARY OF ONE RECORD, standing in a grid, that leads
 * somewhere. It is not a generic bordered box - `features` already renders those, and
 * a component that means "a box" cannot be governed, because nothing is ever wrong.
 *
 * The whole card is the link, not a "View" button in its corner. A card with one
 * destination and a small hit target inside it is a card most people miss, and the
 * two-target version (card links here, button links there) is the pattern that makes
 * a grid unpredictable to keyboard users.
 */

// card-1: the record summary. One destination, whole surface clickable.
function cardRecord(content) {
  const meta = (content.meta || []).map((m) => `<span class="card__meta">${esc(m)}</span>`).join('\n    ');
  return `<a class="card" href="${esc(content.href || '#')}">
  ${content.badge ? `<span class="card__badge">${esc(content.badge)}</span>` : ''}
  <h3 class="card__title">${esc(content.title)}</h3>
  ${content.body ? `<p class="card__body">${esc(content.body)}</p>` : ''}
  ${meta ? `<div class="card__metas">\n    ${meta}\n  </div>` : ''}
</a>`;
}

/*
 * card-2: the metric card. A number, what it counts, and what it moved against.
 *
 * `change` is required to carry its own direction word rather than a bare arrow or a
 * colour, because "up 4%" is good on revenue and bad on time-to-close, and neither a
 * green tint nor a caret says which. The reader should not have to know the metric's
 * polarity to read the card.
 */
function cardMetric(content) {
  return `<div class="card card--metric">
  <p class="card__label">${esc(content.label)}</p>
  <p class="card__figure">${esc(content.figure)}</p>
  ${content.change ? `<p class="card__change">${esc(content.change)}</p>` : ''}
  ${content.asOf ? `<p class="card__asof">${esc(content.asOf)}</p>` : ''}
</div>`;
}

module.exports = {
  'card-1': cardRecord,
  'card-2': cardMetric,
};
