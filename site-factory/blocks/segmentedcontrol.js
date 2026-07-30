'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

/*
 * Demand: 13 of 224 Opbox routes. The control that switches one surface between a
 * small, fixed set of views - list/board, all/mine/archived, month/quarter/year.
 *
 * The distinction that decides which variant you want is NOT visual, it is what the
 * choice does. Switching a view is navigation and belongs in the URL, so a reader can
 * bookmark "the board" and a reload does not silently return them to "the list".
 * Filtering a set is a query. Rendering both as the same anonymous row of buttons is
 * how a view switcher ends up unlinkable.
 *
 * So: tabs render as links with `aria-current`, filters render as buttons in a
 * radiogroup. Same shape on screen, different contract underneath.
 */

// segmentedcontrol-1: switching the view. Links, because each state is a place.
function segmentedTabs(content) {
  const items = (content.items || []).map((it) => {
    const on = it.value === content.value;
    return `<a class="seg__item${on ? ' seg__item--on' : ''}" href="${esc(it.href || '#')}"${on ? ' aria-current="page"' : ''}>${esc(it.label)}</a>`;
  }).join('\n    ');
  return `<nav class="seg" aria-label="${esc(content.label || 'View')}">
  <div class="seg__track">
    ${items}
  </div>
</nav>`;
}

/*
 * segmentedcontrol-2: narrowing the set. Buttons in a radiogroup, because the page
 * does not change, the query does.
 *
 * The count belongs on the option, not next to the control. "Archived (0)" tells the
 * reader not to bother before they click and find an empty state - which is the
 * cheapest way to avoid rendering one at all.
 */
function segmentedFilter(content) {
  const items = (content.items || []).map((it) => {
    const on = it.value === content.value;
    const count = it.count === undefined || it.count === null ? '' : ` <span class="seg__count">${esc(it.count)}</span>`;
    return `<button class="seg__item${on ? ' seg__item--on' : ''}" type="button" role="radio" aria-checked="${on ? 'true' : 'false'}">${esc(it.label)}${count}</button>`;
  }).join('\n    ');
  return `<div class="seg seg--filter" role="radiogroup" aria-label="${esc(content.label || 'Filter')}">
  <div class="seg__track">
    ${items}
  </div>
</div>`;
}

module.exports = {
  'segmentedcontrol-1': segmentedTabs,
  'segmentedcontrol-2': segmentedFilter,
};
