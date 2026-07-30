'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

/*
 * Demand: 49 of 217 Opbox routes. It is also "Empty States", a Modular Play the
 * Playbook lists under both Onboarding and Activation, whose whole argument is that
 * a blank screen is a dead end and the play is to teach from it. The play's own
 * "Don't" is explicit: "Leave the screen cryptic — 'No items found.' is not enough."
 *
 * So neither variant is allowed to render a bare message: both require an action,
 * because an empty state without a next step is the defect the play names.
 */

// emptystate-1: the first-run case. Nothing exists yet, and the action creates one.
function emptyFirstRun(content) {
  return `<section class="empty">
  <div class="empty__mark" role="img" aria-label="${esc(content.markAlt || '')}"></div>
  <h2 class="empty__title">${esc(content.title)}</h2>
  <p class="empty__body">${esc(content.body)}</p>
  <a class="empty__cta" href="${esc(content.ctaHref || '#')}">${esc(content.ctaLabel)}</a>
  ${content.secondaryLabel ? `<a class="empty__secondary" href="${esc(content.secondaryHref || '#')}">${esc(content.secondaryLabel)}</a>` : ''}
</section>`;
}

// emptystate-2: the filtered-to-nothing case. Records exist; this query found none.
// A different situation needing a different action — clear the filter, not create.
function emptyNoResults(content) {
  return `<section class="empty empty--noresults">
  <h2 class="empty__title">${esc(content.title)}</h2>
  <p class="empty__body">${esc(content.body)}</p>
  ${content.query ? `<p class="empty__query">${esc(content.query)}</p>` : ''}
  <a class="empty__cta empty__cta--quiet" href="${esc(content.ctaHref || '#')}">${esc(content.ctaLabel)}</a>
</section>`;
}

module.exports = {
  'emptystate-1': emptyFirstRun,
  'emptystate-2': emptyNoResults,
};
