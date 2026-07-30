'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

// footer-a: one row — wordmark, inline links, copyright.
function footerSimple(content) {
  const links = content.links.map((l) => `<a class="footer__link" href="${esc(l.href)}">${esc(l.label)}</a>`).join('\n      ');
  return `<footer class="footer footer--simple">
  <div class="footer__row">
    <span class="footer__mark">${esc(content.wordmark)}</span>
    <nav class="footer__nav" aria-label="Footer">
      ${links}
    </nav>
    <span class="footer__copyright">${esc(content.copyright)}</span>
  </div>
</footer>`;
}

// footer-b: multi-column link groups over a copyright row.
function footerColumns(content) {
  const columns = content.columns.map((col) => `<div class="footer__col">
      <h3 class="footer__colTitle">${esc(col.title)}</h3>
      <nav class="footer__colLinks" aria-label="${esc(col.title)}">
        ${col.links.map((l) => `<a class="footer__link" href="${esc(l.href)}">${esc(l.label)}</a>`).join('\n        ')}
      </nav>
    </div>`).join('\n    ');
  return `<footer class="footer footer--columns">
  <div class="footer__grid">
    <div class="footer__brand">
      <span class="footer__mark">${esc(content.wordmark)}</span>
      <p class="footer__tagline">${esc(content.tagline)}</p>
    </div>
    ${columns}
  </div>
  <div class="footer__bottom">
    <span class="footer__copyright">${esc(content.copyright)}</span>
  </div>
</footer>`;
}

module.exports = {
  'footer-a': footerSimple,
  'footer-b': footerColumns,
};
