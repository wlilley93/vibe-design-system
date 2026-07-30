'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

// team-1: photo-card grid.
function teamGrid(content) {
  const cards = content.people.map((p) => `<div class="team__card">
      <div class="team__photo" role="img" aria-label="${esc(p.name)}"></div>
      <h3>${esc(p.name)}</h3>
      <p>${esc(p.role)}</p>
    </div>`).join('\n    ');
  return `<section class="team team--grid">
  <h2 class="team__heading">${esc(content.heading)}</h2>
  <div class="team__gridInner">
    ${cards}
  </div>
</section>`;
}

// team-2: list rows, photo + name/role + one-line bio.
function teamList(content) {
  const rows = content.people.map((p) => `<div class="team__row">
      <div class="team__photo" role="img" aria-label="${esc(p.name)}"></div>
      <div class="team__rowText">
        <h3>${esc(p.name)}<span>${esc(p.role)}</span></h3>
        <p>${esc(p.bio)}</p>
      </div>
    </div>`).join('\n    ');
  return `<section class="team team--list">
  <h2 class="team__heading">${esc(content.heading)}</h2>
  <div class="team__listInner">
    ${rows}
  </div>
</section>`;
}

module.exports = {
  'team-1': teamGrid,
  'team-2': teamList,
};
