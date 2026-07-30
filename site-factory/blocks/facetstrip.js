'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

// facetstrip-1: a horizontal row of filter chips, the active ones marked, with a
// count of what the filter currently resolves to. The count matters: a facet strip
// that does not say how many rows it left is a control with no feedback.
function facetChips(content) {
  const chips = content.facets.map((f) => `<button class="facet__chip${f.active ? ' facet__chip--on' : ''}" type="button">
      ${esc(f.label)}${f.count == null ? '' : `<span class="facet__count">${esc(f.count)}</span>`}
    </button>`).join('\n    ');
  return `<div class="facet facet--chips">
  <div class="facet__row">
    ${chips}
  </div>
  <span class="facet__result">${esc(content.resultLabel)}</span>
</div>`;
}

// facetstrip-2: grouped facets with a search field ahead of them, for a surface
// where the filter set is too long to read as one flat row.
function facetGrouped(content) {
  const groups = content.groups.map((g) => `<div class="facet__group">
      <span class="facet__groupTitle">${esc(g.title)}</span>
      <div class="facet__row">
        ${g.facets.map((f) => `<button class="facet__chip${f.active ? ' facet__chip--on' : ''}" type="button">${esc(f.label)}</button>`).join('\n        ')}
      </div>
    </div>`).join('\n    ');
  return `<div class="facet facet--grouped">
  <form class="facet__search" action="${esc(content.searchAction)}" method="get" role="search">
    <input class="facet__input" type="search" name="q" placeholder="${esc(content.searchPlaceholder)}" aria-label="Filter">
  </form>
  <div class="facet__groups">
    ${groups}
  </div>
  <span class="facet__result">${esc(content.resultLabel)}</span>
</div>`;
}

module.exports = {
  'facetstrip-1': facetChips,
  'facetstrip-2': facetGrouped,
};
