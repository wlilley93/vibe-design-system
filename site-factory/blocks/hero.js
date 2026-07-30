'use strict';

/*
 * Hero block, two structural variants. Neither function makes a styling decision:
 * every visual property lives in site.css as a var(--token), bound at build time
 * from the chosen style pack. Swapping style packs must never require touching
 * this file, and picking a different variant must never require touching a token file.
 */

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

// hero-1: centered stack — headline, subhead, single CTA.
function heroCentered(content) {
  return `<section class="hero hero--centered">
  <div class="hero__inner">
    <h1 class="hero__h1">${esc(content.h1)}</h1>
    <p class="hero__sub">${esc(content.sub)}</p>
    <a class="hero__cta" href="${esc(content.ctaHref)}">${esc(content.ctaLabel)}</a>
  </div>
</section>`;
}

// hero-2: split — text column left, media placeholder right.
function heroSplit(content) {
  return `<section class="hero hero--split">
  <div class="hero__text">
    <h1 class="hero__h1">${esc(content.h1)}</h1>
    <p class="hero__sub">${esc(content.sub)}</p>
    <a class="hero__cta" href="${esc(content.ctaHref)}">${esc(content.ctaLabel)}</a>
  </div>
  <div class="hero__media" role="img" aria-label="${esc(content.mediaAlt || '')}"></div>
</section>`;
}

module.exports = {
  'hero-1': heroCentered,
  'hero-2': heroSplit,
};
