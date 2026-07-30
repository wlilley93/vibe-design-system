'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

/*
 * Demand: page-loading 45, page-error 38 of 217 routes. Together the third largest
 * unbuilt need.
 *
 * "Loading Feedback" is a Playbook play, and its Do is "design loading as a branded,
 * ownable surface" - which is why the skeleton uses the project's own tokens rather
 * than a generic grey shimmer.
 */

// pagestate-1: loading. A skeleton of the shape that is coming, not a spinner
// a spinner tells you to wait, a skeleton tells you what for.
function pageLoading(content) {
  const rows = Array.from({ length: content.rows || 4 }, (_, i) =>
    `<div class="pstate__bar" style="width:${[92, 74, 84, 61, 78][i % 5]}%"></div>`).join('\n    ');
  return `<section class="pstate pstate--loading" role="status" aria-live="polite">
  <span class="pstate__label">${esc(content.label)}</span>
  <div class="pstate__skeleton" aria-hidden="true">
    ${rows}
  </div>
</section>`;
}

// pagestate-2: error. Names what failed, what it means, and what to do - an error
// that only says "something went wrong" leaves the reader with no move.
function pageError(content) {
  return `<section class="pstate pstate--error" role="alert">
  <span class="pstate__code">${esc(content.code || '')}</span>
  <h2 class="pstate__title">${esc(content.title)}</h2>
  <p class="pstate__body">${esc(content.body)}</p>
  <div class="pstate__actions">
    <a class="pstate__retry" href="${esc(content.retryHref || '#')}">${esc(content.retryLabel)}</a>
    ${content.helpLabel ? `<a class="pstate__help" href="${esc(content.helpHref || '#')}">${esc(content.helpLabel)}</a>` : ''}
  </div>
  ${content.reference ? `<p class="pstate__ref">${esc(content.reference)}</p>` : ''}
</section>`;
}

module.exports = {
  'pagestate-1': pageLoading,
  'pagestate-2': pageError,
};
