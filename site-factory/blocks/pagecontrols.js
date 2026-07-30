'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

/*
 * Base's compact twin of Pagination, 162 variants, and the measured detail worth
 * keeping is the taper: the dots run 8, 8, 8, 6, 4 away from the current page. That is
 * how a long carousel says where you are without a count, and it also caps the width of
 * the control, so twenty slides do not produce twenty dots in a row.
 *
 * The taper is derived from distance here, not carried as a per-dot size. A list of
 * sizes would let a caller write a taper that does not match the current index, which
 * is a control saying two different things about the same position.
 */

// Measured from Base: full size at the current page and the two either side of it, then
// down. Distances past the tapered range hold at the smallest size.
const TAPER = [8, 8, 8, 6, 4];
function dotSize(distance) {
  return TAPER[Math.min(distance, TAPER.length - 1)];
}

function dots(count, current, { withLabels = false } = {}) {
  return Array.from({ length: count }, (_, i) => {
    const on = i === current;
    const size = dotSize(Math.abs(i - current));
    // The size is inline because it is DERIVED per dot from the index. A class per size
    // would need one for every step of the taper and would still not tie a dot to its
    // distance from the current page.
    const style = `width: calc(var(--space) * ${size / 4}); height: calc(var(--space) * ${size / 4});`;
    const label = withLabels ? ` <span class="pctl__sr">Go to page ${i + 1}</span>` : '';
    return `  <button class="pctl__dot${on ? ' pctl__dot--on' : ''}" type="button" style="${style}"${on ? ' aria-current="true"' : ''} aria-label="Page ${i + 1}">${label}</button>`;
  }).join('\n');
}

// pagecontrols-1: dots alone. For a carousel where the count does not matter and the
// reader is swiping rather than navigating to a numbered position.
function pageControlsDots(content) {
  const count = Number(content.count) || 5;
  const current = Math.min(Math.max(Number(content.current) || 1, 1), count) - 1;
  return `<nav class="pctl" aria-label="${esc(content.label || 'Pages')}">
  <div class="pctl__row">
${dots(count, current)}
  </div>
  <p class="pctl__sr" aria-live="polite">Page ${current + 1} of ${count}</p>
</nav>`;
}

/*
 * pagecontrols-2: dots with the count said out loud. Worth its own variant because the
 * visible "3 of 7" changes who the control serves: a reader deciding whether to keep
 * going needs to know how much is left, and a taper cannot tell them that.
 */
function pageControlsCounted(content) {
  const count = Number(content.count) || 5;
  const current = Math.min(Math.max(Number(content.current) || 1, 1), count) - 1;
  return `<nav class="pctl" aria-label="${esc(content.label || 'Pages')}">
  <div class="pctl__row">
${dots(count, current, { withLabels: true })}
  </div>
  <p class="pctl__count" aria-live="polite">${current + 1} of ${count}</p>
</nav>`;
}

module.exports = {
  'pagecontrols-1': pageControlsDots,
  'pagecontrols-2': pageControlsCounted,
};
