'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

// notfound-1: centered message, one link home. Never a dead end.
function notFoundSimple(content) {
  return `<section class="notfound notfound--simple">
  <p class="notfound__code">${esc(content.code)}</p>
  <h1>${esc(content.heading)}</h1>
  <p>${esc(content.sub)}</p>
  <a class="notfound__link" href="${esc(content.homeHref)}">${esc(content.homeLabel)}</a>
</section>`;
}

// notfound-2: message plus a search bar as a second way out.
function notFoundSearch(content) {
  return `<section class="notfound notfound--search">
  <p class="notfound__code">${esc(content.code)}</p>
  <h1>${esc(content.heading)}</h1>
  <p>${esc(content.sub)}</p>
  <form class="notfound__searchForm" action="${esc(content.searchAction)}" method="get">
    <input class="notfound__searchInput" type="search" name="q" placeholder="${esc(content.searchPlaceholder)}" aria-label="Search">
  </form>
  <a class="notfound__link" href="${esc(content.homeHref)}">${esc(content.homeLabel)}</a>
</section>`;
}

module.exports = {
  'notfound-1': notFoundSimple,
  'notfound-2': notFoundSearch,
};
