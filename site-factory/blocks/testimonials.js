'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

// testimonials-1: grid of quote cards.
function testimonialsGrid(content) {
  const cards = content.items.map((t) => `<figure class="testimonials__card">
      <blockquote>${esc(t.quote)}</blockquote>
      <figcaption>${esc(t.name)}, <span>${esc(t.role)}</span></figcaption>
    </figure>`).join('\n    ');
  return `<section class="testimonials testimonials--grid">
  <h2 class="testimonials__heading">${esc(content.heading)}</h2>
  <div class="testimonials__gridInner">
    ${cards}
  </div>
</section>`;
}

// testimonials-2: single large featured quote.
function testimonialsFeatured(content) {
  const t = content.featured;
  return `<section class="testimonials testimonials--featured">
  <blockquote class="testimonials__big">${esc(t.quote)}</blockquote>
  <p class="testimonials__attribution">${esc(t.name)}, <span>${esc(t.role)}</span></p>
</section>`;
}

module.exports = {
  'testimonials-1': testimonialsGrid,
  'testimonials-2': testimonialsFeatured,
};
