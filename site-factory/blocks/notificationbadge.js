'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

/*
 * Base gives Notification badge 16 variants across dot, count and overflow, and the
 * choice between the first two is a product decision rather than a visual one.
 *
 * A DOT says "there is something here". A COUNT says "there are eleven things here".
 * The count is only worth its space if the number is actionable: eleven unread messages
 * changes what you do, whereas "11" pending background jobs usually does not, and a
 * number nobody acts on is noise that trains the reader to ignore the badge.
 *
 * Both are announced. A badge that is only a coloured dot says nothing to a screen
 * reader, so the count carries a visually hidden phrase and the dot carries a label;
 * `aria-hidden` on the glyph stops it being read twice.
 */

// notificationbadge-1: presence only. No number, because there is no number worth saying.
function badgeDot(content) {
  return `<span class="nbadge">
  ${esc(content.label || '')}
  <span class="nbadge__dot" aria-hidden="true"></span>
  <span class="nbadge__sr">${esc(content.srLabel || 'has updates')}</span>
</span>`;
}

/*
 * notificationbadge-2: the count, with an overflow cap. The cap is the interesting
 * part: past a certain point the exact number stops mattering and the width of the
 * badge starts to. Base caps and so does this, and the screen-reader text says the
 * TRUE figure rather than the capped one, because "99+" is a layout decision and the
 * reader asking their machine to read it out wants the number.
 */
function badgeCount(content) {
  const n = Number(content.count) || 0;
  const cap = Number(content.cap) || 99;
  const shown = n > cap ? `${cap}+` : String(n);
  if (n === 0) {
    // Zero is not a small number here, it is the absence of the thing. A badge reading
    // "0" is a badge drawing attention to nothing.
    return `<span class="nbadge">${esc(content.label || '')}</span>`;
  }
  return `<span class="nbadge">
  ${esc(content.label || '')}
  <span class="nbadge__count" aria-hidden="true">${esc(shown)}</span>
  <span class="nbadge__sr">${n} ${esc(content.unit || 'unread')}</span>
</span>`;
}

module.exports = {
  'notificationbadge-1': badgeDot,
  'notificationbadge-2': badgeCount,
};
