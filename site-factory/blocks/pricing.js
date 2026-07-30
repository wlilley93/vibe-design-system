'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

// pricing-1: three plan cards side by side, one highlighted.
function pricingCards(content) {
  const cards = content.plans.map((p) => `<div class="pricing__card${p.highlight ? ' pricing__card--highlight' : ''}">
      <h3 class="pricing__planName">${esc(p.name)}</h3>
      <p class="pricing__price">${esc(p.price)}</p>
      <ul class="pricing__features">
        ${p.features.map((f) => `<li>${esc(f)}</li>`).join('\n        ')}
      </ul>
      <a class="pricing__cta" href="${esc(p.ctaHref)}">${esc(p.ctaLabel)}</a>
    </div>`).join('\n    ');
  return `<section class="pricing pricing--cards">
  <h2 class="pricing__heading">${esc(content.heading)}</h2>
  <div class="pricing__grid">
    ${cards}
  </div>
</section>`;
}

// pricing-2: single comparison table, plans as columns.
function pricingTable(content) {
  const header = content.plans.map((p) => `<th>${esc(p.name)}<br><span class="pricing__price">${esc(p.price)}</span></th>`).join('\n        ');
  const rows = content.rows.map((r) => `<tr><td>${esc(r.label)}</td>${r.values.map((v) => `<td>${esc(v)}</td>`).join('')}</tr>`).join('\n      ');
  return `<section class="pricing pricing--table">
  <h2 class="pricing__heading">${esc(content.heading)}</h2>
  <table class="pricing__matrix">
    <thead><tr><th></th>${header}</tr></thead>
    <tbody>
      ${rows}
    </tbody>
  </table>
</section>`;
}

module.exports = {
  'pricing-1': pricingCards,
  'pricing-2': pricingTable,
};
