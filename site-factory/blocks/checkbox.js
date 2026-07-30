'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

/*
 * Base calls this Check and gives it 8 variants across three states, and the third
 * state is the reason this block exists separately from `switch`.
 *
 * A checkbox has THREE states, not two: checked, unchecked, and indeterminate. The
 * third is not a decoration, it is the only honest thing a parent checkbox can say
 * when some of its children are ticked. Rendering that parent as unchecked tells the
 * reader nothing is selected, and rendering it as checked tells them everything is;
 * both are wrong, and both are what happens when a system has no third state. It is
 * expressed as `aria-checked="mixed"`, which is why these are buttons: an
 * `<input type=checkbox>` carries `indeterminate` only as a DOM property that no
 * markup can set, so a statically generated page cannot produce one at all.
 */

// checkbox-1: one checkbox with its label. The label is the target too.
function checkboxSingle(content) {
  const state = content.checked === 'mixed' ? 'mixed' : (content.checked === true ? 'true' : 'false');
  return `<div class="check">
  <button class="check__box check__box--${state === 'true' ? 'on' : (state === 'mixed' ? 'mixed' : 'off')}" type="button" role="checkbox" aria-checked="${state}" id="check-1">
    <span class="check__mark" aria-hidden="true">${state === 'mixed' ? '&ndash;' : '&check;'}</span>
  </button>
  <label class="check__label" for="check-1">${esc(content.label || '')}</label>
</div>`;
}

/*
 * checkbox-2: the parent-and-children group, which is where the third state earns
 * itself. The parent's state is DERIVED from the children here rather than carried as
 * its own field, because a parent whose state can be set independently of its children
 * is a parent that can contradict them.
 */
function checkboxGroup(content) {
  const items = content.items || [];
  const ticked = items.filter((i) => i.checked === true).length;
  const parent = ticked === 0 ? 'false' : (ticked === items.length ? 'true' : 'mixed');
  const parentClass = parent === 'true' ? 'on' : (parent === 'mixed' ? 'mixed' : 'off');

  const rows = items.map((it, i) => {
    const on = it.checked === true;
    const id = `check-g-${i + 1}`;
    return `    <div class="check">
      <button class="check__box check__box--${on ? 'on' : 'off'}" type="button" role="checkbox" aria-checked="${on ? 'true' : 'false'}" id="${id}">
        <span class="check__mark" aria-hidden="true">&check;</span>
      </button>
      <label class="check__label" for="${id}">${esc(it.label)}</label>
    </div>`;
  }).join('\n');

  return `<fieldset class="check__group">
  <legend class="check__legend">${esc(content.label || 'Select')}</legend>
  <div class="check check--parent">
    <button class="check__box check__box--${parentClass}" type="button" role="checkbox" aria-checked="${parent}" id="check-g-all">
      <span class="check__mark" aria-hidden="true">${parent === 'mixed' ? '&ndash;' : '&check;'}</span>
    </button>
    <label class="check__label" for="check-g-all">${esc(content.allLabel || 'Select all')} <span class="check__count">${ticked} of ${items.length}</span></label>
  </div>
  <div class="check__children">
${rows}
  </div>
</fieldset>`;
}

module.exports = {
  'checkbox-1': checkboxSingle,
  'checkbox-2': checkboxGroup,
};
