'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

// cta-1: centered banner, single button.
function ctaBanner(content) {
  return `<section class="cta cta--banner">
  <h2>${esc(content.heading)}</h2>
  <p>${esc(content.sub)}</p>
  <a class="cta__button" href="${esc(content.ctaHref)}">${esc(content.ctaLabel)}</a>
</section>`;
}

// cta-2: split, heading left, email-capture form right.
function ctaSignup(content) {
  return `<section class="cta cta--signup">
  <div class="cta__text">
    <h2>${esc(content.heading)}</h2>
    <p>${esc(content.sub)}</p>
  </div>
  <form class="cta__form" action="${esc(content.formAction)}" method="post">
    <input class="cta__input" type="email" name="email" placeholder="${esc(content.placeholder)}" aria-label="Email address">
    <button class="cta__button" type="submit">${esc(content.ctaLabel)}</button>
  </form>
</section>`;
}

module.exports = {
  'cta-1': ctaBanner,
  'cta-2': ctaSignup,
};
