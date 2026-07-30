'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

// faq-1: native <details> accordion, one column.
function faqAccordion(content) {
  const items = content.items.map((q) => `<details class="faq__item">
      <summary>${esc(q.question)}</summary>
      <p>${esc(q.answer)}</p>
    </details>`).join('\n    ');
  return `<section class="faq faq--accordion">
  <h2 class="faq__heading">${esc(content.heading)}</h2>
  <div class="faq__list">
    ${items}
  </div>
</section>`;
}

// faq-2: two-column question/answer list, always expanded.
function faqColumns(content) {
  const items = content.items.map((q) => `<div class="faq__pair">
      <h3>${esc(q.question)}</h3>
      <p>${esc(q.answer)}</p>
    </div>`).join('\n    ');
  return `<section class="faq faq--columns">
  <h2 class="faq__heading">${esc(content.heading)}</h2>
  <div class="faq__grid">
    ${items}
  </div>
</section>`;
}

module.exports = {
  'faq-1': faqAccordion,
  'faq-2': faqColumns,
};
