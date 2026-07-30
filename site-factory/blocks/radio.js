'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

/*
 * Base gives Radio only 4 variants, which understates it: the radio is the control
 * that says the options are MUTUALLY EXCLUSIVE and there is exactly one answer. A
 * column of checkboxes cannot say that, and a select hides the options behind a click.
 *
 * The rule this block encodes: SHOW the options when the choice is the point and there
 * are few enough to show. Use `formfield`'s select when the list is long or the choice
 * is incidental. Both variants here render a real `radiogroup`, so keyboard users get
 * one tab stop and arrow keys between options rather than one tab stop per option.
 */

function options(items, name, cls) {
  return (items || []).map((it, i) => {
    const on = it.value === undefined ? i === 0 : it.value === name;
    const id = `${cls}-${i + 1}`;
    return `  <div class="radio__option">
    <button class="radio__dot${on ? ' radio__dot--on' : ''}" type="button" role="radio" aria-checked="${on ? 'true' : 'false'}" id="${id}">
      <span class="radio__pip" aria-hidden="true"></span>
    </button>
    <label class="radio__label" for="${id}">${esc(it.label)}</label>
  </div>`;
  }).join('\n');
}

// radio-1: the plain stack. One line per option, which is the default because it reads
// as a list of answers rather than a row of buttons.
function radioStack(content) {
  return `<fieldset class="radio" role="radiogroup" aria-label="${esc(content.label || 'Choose one')}">
  <legend class="radio__legend">${esc(content.label || 'Choose one')}</legend>
${options(content.items, content.value, 'radio-s')}
</fieldset>`;
}

/*
 * radio-2: the option as a card, with a description under each label. Worth having as
 * its own variant rather than a flag, because the card form changes what the control is
 * FOR: it is the shape for a choice that needs explaining (a plan, a delivery speed, a
 * jurisdiction), where the description is not decoration but the thing being compared.
 * The whole card is the target, not just the dot.
 */
function radioCards(content) {
  const items = (content.items || []).map((it, i) => {
    const on = it.value === undefined ? i === 0 : it.value === content.value;
    const id = `radio-c-${i + 1}`;
    return `  <label class="radio__card${on ? ' radio__card--on' : ''}" for="${id}">
    <button class="radio__dot${on ? ' radio__dot--on' : ''}" type="button" role="radio" aria-checked="${on ? 'true' : 'false'}" id="${id}">
      <span class="radio__pip" aria-hidden="true"></span>
    </button>
    <span class="radio__cardText">
      <span class="radio__cardTitle">${esc(it.label)}</span>
      ${it.description ? `<span class="radio__cardDesc">${esc(it.description)}</span>` : ''}
    </span>
  </label>`;
  }).join('\n');
  return `<fieldset class="radio radio--cards" role="radiogroup" aria-label="${esc(content.label || 'Choose one')}">
  <legend class="radio__legend">${esc(content.label || 'Choose one')}</legend>
${items}
</fieldset>`;
}

module.exports = {
  'radio-1': radioStack,
  'radio-2': radioCards,
};
