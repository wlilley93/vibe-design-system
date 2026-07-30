'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

// nav-1: logo left, links + CTA right, one row.
function navSimple(content) {
  const links = content.links.map((l) => `<a class="nav__link" href="${esc(l.href)}">${esc(l.label)}</a>`).join('\n    ');
  return `<header class="nav nav--simple">
  <span class="nav__mark">${esc(content.wordmark)}</span>
  <nav class="nav__links" aria-label="Main">
    ${links}
  </nav>
  <a class="nav__cta" href="${esc(content.ctaHref)}">${esc(content.ctaLabel)}</a>
</header>`;
}

// nav-2: logo centered, links split evenly left and right of it.
function navCentered(content) {
  const half = Math.ceil(content.links.length / 2);
  const left = content.links.slice(0, half);
  const right = content.links.slice(half);
  const renderLinks = (list) => list.map((l) => `<a class="nav__link" href="${esc(l.href)}">${esc(l.label)}</a>`).join('\n      ');
  return `<header class="nav nav--centered">
  <nav class="nav__side" aria-label="Primary left">
      ${renderLinks(left)}
  </nav>
  <span class="nav__mark">${esc(content.wordmark)}</span>
  <nav class="nav__side" aria-label="Primary right">
      ${renderLinks(right)}
  </nav>
</header>`;
}

module.exports = {
  'nav-1': navSimple,
  'nav-2': navCentered,
};
