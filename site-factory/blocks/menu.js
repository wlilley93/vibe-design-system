'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

/*
 * Base declares Menu (the container, 12 variants) and Menu item (57) as separate
 * component sets. They are one block here, because a menu item outside a menu is not a
 * thing a page ever places, and two block types would let a caller build a menu with no
 * container or an item list with no accessible grouping.
 *
 * Measured from Base: two sizes, and the ONLY difference is the item's vertical padding
 * (8/20/8 for a 36 row, 12/24/12 for 48). Container radius 12, no gap between rows,
 * container widths 207 and 242.
 *
 * NOT `role="menu"`. That role is a promise: arrow keys move between items, Home and End
 * jump to the ends, Escape closes and returns focus. A statically generated page ships
 * no script, so the promise would be unkept, and a screen reader told "menu" then given
 * a control that does not respond to arrow keys is worse off than one told "list". So
 * this is a list of buttons, which is exactly what it is. When the consuming app wires
 * up the keyboard behaviour it can upgrade the roles, and the markup says so.
 */

function item(it, size) {
  const destructive = it.destructive === true;
  const cls = `menu__item menu__item--${size}${destructive ? ' menu__item--destructive' : ''}${it.disabled ? ' menu__item--off' : ''}`;
  const shortcut = it.shortcut ? `<span class="menu__shortcut" aria-hidden="true">${esc(it.shortcut)}</span>` : '';
  const attrs = it.disabled ? ' aria-disabled="true"' : '';
  return `    <li><button class="${cls}" type="button"${attrs}>
      <span class="menu__label">${esc(it.label)}</span>
      ${shortcut}
    </button></li>`;
}

// menu-1: the flat action menu. One list, no grouping, which is right up to about seven
// items; past that a reader scans rather than reads and grouping starts to earn itself.
function menuFlat(content) {
  const size = content.size === 'medium' ? 'medium' : 'small';
  const items = (content.items || []).map((it) => item(it, size)).join('\n');
  return `<div class="menu">
  <ul class="menu__list" aria-label="${esc(content.label || 'Actions')}">
${items}
  </ul>
</div>`;
}

/*
 * menu-2: grouped, with the destructive action separated and last.
 *
 * The separation is the component, not a flourish. A Delete sitting flush against a
 * Duplicate is a Delete one slip away from being pressed, and a reader working down a
 * list at speed has no visual warning before the irreversible item. Base's own divider
 * is inset for rows of the same kind and full-bleed between sections; a destructive
 * action is a different KIND of thing, so it gets the full-bleed rule.
 */
function menuGrouped(content) {
  const size = content.size === 'medium' ? 'medium' : 'small';
  const groups = (content.groups || []).map((g) => {
    const rows = (g.items || []).map((it) => item(it, size)).join('\n');
    return `  <li class="menu__group">
    ${g.title ? `<p class="menu__groupTitle">${esc(g.title)}</p>` : ''}
    <ul class="menu__list">
${rows}
    </ul>
  </li>`;
  }).join('\n');

  const danger = (content.destructive || []).map((it) => item({ ...it, destructive: true }, size)).join('\n');

  return `<div class="menu">
  <ul class="menu__groups" aria-label="${esc(content.label || 'Actions')}">
${groups}
${danger ? `  <li class="menu__group menu__group--danger">
    <ul class="menu__list">
${danger}
    </ul>
  </li>` : ''}
  </ul>
</div>`;
}

module.exports = {
  'menu-1': menuFlat,
  'menu-2': menuGrouped,
};
