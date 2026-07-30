'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

/*
 * The DETERMINATE bar. `pagestate` already answers Base's indeterminate bar and progress
 * circle, and the difference between them is the only thing that matters here: an
 * indeterminate bar says "working", a determinate one says "this much of that much".
 * Showing a determinate bar when the total is unknown means inventing a denominator, and
 * a bar that reaches 90% and sits there is worse than a spinner.
 *
 * Measured from Base: 2px at Small and 4px at Medium, with the label above at a 12 or 14
 * gap. Base builds the fill by making the bar frame the fill colour and pushing a grey
 * remainder right with paddingLeft, so the visible progress IS a padding value. Drawn
 * here as a track with a fill inside it: same picture, and it says what it is.
 *
 * `role="progressbar"` with the real numbers, so a screen reader announces progress
 * rather than reading a decorative div. The percentage is DERIVED from value and max in
 * one place; carrying a separate percent field would let the number and the bar disagree.
 */

function bar(value, max, size) {
  const clamped = Math.min(Math.max(Number(value) || 0, 0), Number(max) || 1);
  const pct = Math.round((clamped / (Number(max) || 1)) * 100);
  return {
    pct,
    clamped,
    html: `  <div class="pbar__track pbar__track--${size}" role="progressbar" aria-valuenow="${clamped}" aria-valuemin="0" aria-valuemax="${esc(max)}">
    <div class="pbar__fill" style="width: ${pct}%;"></div>
  </div>`,
  };
}

// progressbar-1: the percentage. Right when the unit is meaningless to the reader -
// nobody cares how many bytes an upload has, they care how far along it is.
function progressPercent(content) {
  const size = content.size === 'medium' ? 'medium' : 'small';
  const b = bar(content.value, content.max, size);
  return `<div class="pbar">
${b.html}
  <p class="pbar__label">${esc(content.label || '')} <span class="pbar__figure">${b.pct}%</span></p>
</div>`;
}

/*
 * progressbar-2: the absolute count. Right when the unit IS the information: "12 of 40
 * files" tells a reader whether to wait, and how bad it would be to cancel, in a way
 * "30%" does not. Same bar, different caption, and the caption is the component.
 */
function progressCount(content) {
  const size = content.size === 'medium' ? 'medium' : 'small';
  const b = bar(content.value, content.max, size);
  const unit = content.unit || 'items';
  return `<div class="pbar">
${b.html}
  <p class="pbar__label">${esc(content.label || '')} <span class="pbar__figure">${b.clamped} of ${esc(content.max)} ${esc(unit)}</span></p>
</div>`;
}

module.exports = {
  'progressbar-1': progressPercent,
  'progressbar-2': progressCount,
};
