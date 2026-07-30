'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

// A gated action: enabled, or blocked WITH its reason stated inline. Opbox's own
// inventory names the Gated Action Button as a distinct component precisely because
// a disabled control with no reason is the defect - the user cannot tell whether
// they lack a permission, a prerequisite, or the feature is simply off.
function action(a) {
  if (a.blocked) {
    return `<span class="oview__action oview__action--blocked" aria-disabled="true">
        ${esc(a.label)}<span class="oview__blockedWhy">${esc(a.blockedReason)}</span>
      </span>`;
  }
  return `<a class="oview__action" href="${esc(a.href || '#')}">${esc(a.label)}</a>`;
}

// objectview-1: the object header - identity, key facts as a definition row, and
// the actions available on it. This is the "Theatre shell" head.
function objectHeader(content) {
  const facts = content.facts.map((f) => `<div class="oview__fact">
        <dt>${esc(f.label)}</dt><dd>${esc(f.value)}</dd>
      </div>`).join('\n      ');
  const actions = (content.actions || []).map(action).join('\n      ');
  return `<section class="oview">
  <header class="oview__head">
    <div class="oview__id">
      <span class="oview__kind">${esc(content.kind)}</span>
      <h1 class="oview__title">${esc(content.title)}</h1>
      <p class="oview__sub">${esc(content.sub || '')}</p>
    </div>
    <div class="oview__actions">
      ${actions}
    </div>
  </header>
  <dl class="oview__facts">
      ${facts}
  </dl>
</section>`;
}

// objectview-2: the same header plus a tab strip, for an object with more than one
// panel of content. The tabs are marks only; routing is the app's concern.
function objectTabs(content) {
  const base = objectHeader(content);
  const tabs = (content.tabs || []).map((t) => `<span class="oview__tab${t.active ? ' oview__tab--on' : ''}">${esc(t.label)}${t.count == null ? '' : `<span class="oview__tabCount">${esc(t.count)}</span>`}</span>`).join('\n    ');
  return base.replace('</section>', `  <nav class="oview__tabs">
    ${tabs}
  </nav>
</section>`);
}

module.exports = {
  'objectview-1': objectHeader,
  'objectview-2': objectTabs,
};
