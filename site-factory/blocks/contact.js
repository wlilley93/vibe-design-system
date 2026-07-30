'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

// contact-1: single centered form.
function contactForm(content) {
  return `<section class="contact contact--form">
  <h2>${esc(content.heading)}</h2>
  <form class="contact__form" action="${esc(content.formAction)}" method="post">
    <input class="contact__input" type="text" name="name" placeholder="Name" aria-label="Name">
    <input class="contact__input" type="email" name="email" placeholder="Email" aria-label="Email">
    <textarea class="contact__textarea" name="message" placeholder="Message" aria-label="Message" rows="4"></textarea>
    <button class="contact__button" type="submit">${esc(content.ctaLabel)}</button>
  </form>
</section>`;
}

// contact-2: split - info column left, form right.
function contactSplit(content) {
  return `<section class="contact contact--split">
  <div class="contact__info">
    <h2>${esc(content.heading)}</h2>
    <p>${esc(content.sub)}</p>
    <p class="contact__email">${esc(content.email)}</p>
  </div>
  <form class="contact__form" action="${esc(content.formAction)}" method="post">
    <input class="contact__input" type="text" name="name" placeholder="Name" aria-label="Name">
    <input class="contact__input" type="email" name="email" placeholder="Email" aria-label="Email">
    <textarea class="contact__textarea" name="message" placeholder="Message" aria-label="Message" rows="4"></textarea>
    <button class="contact__button" type="submit">${esc(content.ctaLabel)}</button>
  </form>
</section>`;
}

module.exports = {
  'contact-1': contactForm,
  'contact-2': contactSplit,
};
