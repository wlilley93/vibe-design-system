'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

/*
 * The two variants here are not two looks, they are two different contracts, and
 * conflating them is how tooltips end up holding content nobody can reach.
 *
 * A LABEL tooltip names a control that has no visible name - an icon button. The text
 * IS the button's accessible name, so it belongs in `aria-label` and the visible
 * bubble is a redundant copy for sighted users. Nothing is lost if it never appears.
 *
 * A DESCRIPTION tooltip adds information the control does not already carry. That
 * content is reachable only on hover or focus, so it must never be the only place
 * something important is said, and it is wired with `aria-describedby` rather than
 * `aria-label` so it supplements the name instead of replacing it.
 *
 * Both are rendered open here. A static page cannot hover, and a design system's job
 * is to show the state, not to hide it behind an interaction the reader cannot perform.
 */

// tooltip-1: naming an icon button. The bubble repeats the accessible name.
function tooltipLabel(content) {
  const label = esc(content.label || '');
  return `<div class="tip">
  <button class="tip__target" type="button" aria-label="${label}">
    <span class="tip__glyph" aria-hidden="true">${esc(content.glyph || 'i')}</span>
  </button>
  <span class="tip__bubble" role="presentation">${label}</span>
</div>`;
}

/*
 * tooltip-2: the rich tooltip, with a heading and a line of body copy. Described, not
 * labelled: the control keeps its own name and this adds to it.
 */
function tooltipRich(content) {
  return `<div class="tip tip--rich">
  <button class="tip__target tip__target--text" type="button" aria-describedby="tip-rich">
    ${esc(content.targetLabel || 'Details')}
  </button>
  <span class="tip__bubble tip__bubble--rich" id="tip-rich" role="tooltip">
    <span class="tip__title">${esc(content.title || '')}</span>
    <span class="tip__body">${esc(content.body || '')}</span>
  </span>
</div>`;
}

module.exports = {
  'tooltip-1': tooltipLabel,
  'tooltip-2': tooltipRich,
};
