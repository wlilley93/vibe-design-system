'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

// sidebar-1: flat link list, app context.
function sidebarSimple(content) {
  const items = content.links.map((l) => `<a class="sidebar__link${l.active ? ' sidebar__link--active' : ''}" href="${esc(l.href)}">${esc(l.label)}</a>`).join('\n    ');
  return `<aside class="sidebar sidebar--simple">
  <nav class="sidebar__nav" aria-label="Sidebar">
    ${items}
  </nav>
</aside>`;
}

// sidebar-2: grouped sections, each with a heading.
function sidebarGrouped(content) {
  const groups = content.groups.map((g) => `<div class="sidebar__group">
      <h3 class="sidebar__groupTitle">${esc(g.title)}</h3>
      ${g.links.map((l) => `<a class="sidebar__link${l.active ? ' sidebar__link--active' : ''}" href="${esc(l.href)}">${esc(l.label)}</a>`).join('\n      ')}
    </div>`).join('\n    ');
  return `<aside class="sidebar sidebar--grouped">
  ${groups}
</aside>`;
}

module.exports = {
  'sidebar-1': sidebarSimple,
  'sidebar-2': sidebarGrouped,
};
