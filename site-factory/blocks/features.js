'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

// features-1: icon-less title+description grid.
function featuresGrid(content) {
  const items = content.items.map((f) => `<div class="features__item">
      <h3>${esc(f.title)}</h3>
      <p>${esc(f.body)}</p>
    </div>`).join('\n    ');
  return `<section class="features features--grid">
  <h2 class="features__heading">${esc(content.heading)}</h2>
  <div class="features__gridInner">
    ${items}
  </div>
</section>`;
}

// features-2: comparison table, checkmarks/dashes per plan column.
function featuresComparison(content) {
  const header = content.columns.map((c) => `<th>${esc(c)}</th>`).join('');
  const rows = content.rows.map((r) => `<tr><td>${esc(r.label)}</td>${r.values.map((v) => `<td>${v ? '&#10003;' : '&#8212;'}</td>`).join('')}</tr>`).join('\n      ');
  return `<section class="features features--comparison">
  <h2 class="features__heading">${esc(content.heading)}</h2>
  <table class="features__matrix">
    <thead><tr><th></th>${header}</tr></thead>
    <tbody>
      ${rows}
    </tbody>
  </table>
</section>`;
}

module.exports = {
  'features-1': featuresGrid,
  'features-2': featuresComparison,
};
