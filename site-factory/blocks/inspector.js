'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

// inspector-1: a property panel — grouped label/value rows down a narrow column.
function inspectorProps(content) {
  const groups = content.groups.map((g) => `<div class="insp__group">
      <span class="insp__groupTitle">${esc(g.title)}</span>
      ${g.rows.map((r) => `<div class="insp__row">
        <span class="insp__label">${esc(r.label)}</span>
        <span class="insp__value">${esc(r.value)}</span>
      </div>`).join('\n      ')}
    </div>`).join('\n    ');
  return `<aside class="insp">
  <header class="insp__head">${esc(content.heading)}</header>
  <div class="insp__body">
    ${groups}
  </div>
</aside>`;
}

// inspector-2: the same column carrying an activity trail instead of properties.
// Every entry states who and when, because an audit line without an actor is not
// an audit line.
function inspectorActivity(content) {
  const items = content.events.map((e) => `<li class="insp__event">
      <span class="insp__when">${esc(e.when)}</span>
      <span class="insp__what">${esc(e.what)}</span>
      <span class="insp__who">${esc(e.who)}</span>
    </li>`).join('\n    ');
  return `<aside class="insp insp--activity">
  <header class="insp__head">${esc(content.heading)}</header>
  <ol class="insp__events">
    ${items}
  </ol>
</aside>`;
}

module.exports = {
  'inspector-1': inspectorProps,
  'inspector-2': inspectorActivity,
};
