'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

/*
 * Relume splits this into Gallery (27 sets) and Portfolio Sections (23), and the difference
 * between them is the only reason there are two variants here rather than one: a gallery is
 * a set of IMAGES and a portfolio is a set of LINKS that happen to have images. One is
 * looked at, the other is clicked, and a portfolio rendered as a gallery is a portfolio
 * nobody can get into.
 *
 * Every tile carries an `alt`. Not as politeness: a generated site has no image files yet,
 * so the alt text is the ONLY thing in the tile that says what belongs there, and it is
 * what the writing skill later fills. A tile with no alt is a grey box with no brief.
 */

function tile(item, i, linked) {
  const media = `<span class="gallery__media" role="img" aria-label="${esc(item.alt || '')}"></span>`;
  const caption = item.caption ? `<span class="gallery__caption">${esc(item.caption)}</span>` : '';
  if (!linked) {
    return `    <figure class="gallery__tile">
      ${media}
      ${item.caption ? `<figcaption class="gallery__caption">${esc(item.caption)}</figcaption>` : ''}
    </figure>`;
  }
  return `    <a class="gallery__tile gallery__tile--link" href="${esc(item.href || '#')}">
      ${media}
      <span class="gallery__meta">
        <span class="gallery__title">${esc(item.title || '')}</span>
        ${caption}
      </span>
    </a>`;
}

// gallery-1: images. A grid of figures, no links, captions optional.
function galleryGrid(content) {
  const items = (content.items || []).map((it, i) => tile(it, i, false)).join('\n');
  return `<section class="gallery">
  ${content.heading ? `<h2 class="gallery__heading">${esc(content.heading)}</h2>` : ''}
  <div class="gallery__grid">
${items}
  </div>
</section>`;
}

/*
 * gallery-2: the portfolio. Every tile is a link with a title, and the section carries a
 * show-more.
 *
 * The show-more is part of the component rather than an afterthought, because a portfolio
 * grid is almost always a TRUNCATION of a longer list, and a grid that silently shows the
 * first eight of forty is a grid that misrepresents the body of work. It renders as a real
 * link to a real page, not as a control that needs script to mean anything.
 */
function portfolioGrid(content) {
  const items = (content.items || []).map((it, i) => tile(it, i, true)).join('\n');
  const more = content.moreLabel
    ? `  <a class="gallery__more" href="${esc(content.moreHref || '#')}">${esc(content.moreLabel)}</a>\n`
    : '';
  return `<section class="gallery gallery--portfolio">
  ${content.heading ? `<h2 class="gallery__heading">${esc(content.heading)}</h2>` : ''}
  <div class="gallery__grid">
${items}
  </div>
${more}</section>`;
}

module.exports = {
  'gallery-1': galleryGrid,
  'gallery-2': portfolioGrid,
};
