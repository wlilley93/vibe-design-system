'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

// A status cell renders as a pill or a dot depending on the project's
// componentStyle.statusBadgeStyle, which is passed through on the content. This is
// what makes that config field real rather than decorative: choose "dot" and the
// table's status column actually changes shape.
function badge(status, style) {
  if (!status) return '';
  const tone = String(status.tone || 'info');
  if (style === 'dot') {
    return `<span class="badge badge--dot badge--${esc(tone)}"><i></i>${esc(status.label)}</span>`;
  }
  return `<span class="badge badge--pill badge--${esc(tone)}">${esc(status.label)}</span>`;
}

// objecttable-1: the default object list. Columns declared once, rows are cells in
// the same order, with an optional status column and a row-level action.
function objectTable(content) {
  const style = content.badgeStyle || 'pill';
  const head = content.columns.map((c) => `<th>${esc(c)}</th>`).join('');
  const rows = content.rows.map((r) => {
    const cells = r.cells.map((c) => `<td>${esc(c)}</td>`).join('');
    const st = r.status ? `<td>${badge(r.status, style)}</td>` : '';
    const act = content.actionLabel ? `<td class="otable__act"><a href="${esc(r.href || '#')}">${esc(content.actionLabel)}</a></td>` : '';
    return `<tr><td class="otable__key">${esc(r.key)}</td>${cells}${st}${act}</tr>`;
  }).join('\n      ');
  const statusHead = content.rows.some((r) => r.status) ? '<th>Status</th>' : '';
  const actHead = content.actionLabel ? '<th></th>' : '';
  return `<section class="otable">
  <header class="otable__head">
    <h2 class="otable__title">${esc(content.heading)}</h2>
    <span class="otable__meta">${esc(content.meta || '')}</span>
  </header>
  <table class="otable__grid">
    <thead><tr><th>${esc(content.keyColumn)}</th>${head}${statusHead}${actHead}</tr></thead>
    <tbody>
      ${rows}
    </tbody>
  </table>
</section>`;
}

// objecttable-2: the same data as a selectable list rather than a grid, for a
// narrow column (a master pane) where a wide table cannot fit.
function objectList(content) {
  const style = content.badgeStyle || 'pill';
  const items = content.rows.map((r) => `<a class="otable__item${r.selected ? ' otable__item--on' : ''}" href="${esc(r.href || '#')}">
      <span class="otable__itemKey">${esc(r.key)}</span>
      <span class="otable__itemSub">${esc((r.cells || []).join(' · '))}</span>
      ${badge(r.status, style)}
    </a>`).join('\n    ');
  return `<section class="otable otable--list">
  <header class="otable__head">
    <h2 class="otable__title">${esc(content.heading)}</h2>
    <span class="otable__meta">${esc(content.meta || '')}</span>
  </header>
  <div class="otable__items">
    ${items}
  </div>
</section>`;
}

module.exports = {
  'objecttable-1': objectTable,
  'objecttable-2': objectList,
};
