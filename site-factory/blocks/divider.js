'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

/*
 * Base gives Divider only 3 variants and one of them is the whole point: the CELL
 * divider, inset from the leading edge rather than running the full width.
 *
 * The inset is not a stylistic preference, it changes what the rule MEANS. A
 * full-bleed rule reads as a boundary between sections: two different kinds of thing,
 * one above and one below. An inset rule, starting under the text rather than under the
 * avatar or the checkbox, reads as a boundary between rows OF THE SAME KIND. Use the
 * wrong one and a list of three matters reads as three unrelated sections, which is the
 * commonest way a table stops looking like a table.
 *
 * `role="separator"` on the labelled variant, because a labelled divider is announced;
 * the plain one is `aria-hidden` since the heading structure already says it.
 */

// divider-1: the section rule, full width, no label.
function dividerPlain(content) {
  const inset = content.inset === true;
  return `<hr class="rule${inset ? ' rule--inset' : ''}" aria-hidden="true">`;
}

/*
 * divider-2: the labelled rule, which is a divider that also names what follows. Worth
 * its own variant because the label makes it a heading in practice, and the rule has to
 * break around the text rather than run behind it.
 */
function dividerLabelled(content) {
  return `<div class="rule__labelled" role="separator" aria-label="${esc(content.label || '')}">
  <span class="rule__line" aria-hidden="true"></span>
  <span class="rule__label">${esc(content.label || '')}</span>
  <span class="rule__line" aria-hidden="true"></span>
</div>`;
}

module.exports = {
  'divider-1': dividerPlain,
  'divider-2': dividerLabelled,
};
