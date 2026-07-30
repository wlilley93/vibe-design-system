'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

/*
 * Uber Base declares a Switch as 8 variants and site-factory had none, so a settings
 * surface had to reach for a checkbox and mean something different by it.
 *
 * The distinction between this and `checkbox` is not decoration and it decides which
 * one a page wants: A SWITCH TAKES EFFECT IMMEDIATELY, a checkbox states an intention
 * that some later Save commits. That is why a switch is a `button` with
 * `role="switch"` rather than an `input` inside a form: there is no form to submit,
 * the press IS the action. A switch rendered as a checkbox inside a form with a Save
 * button is a control that lies about when it applies.
 */

// switch-1: the bare labelled switch. The label is the click target as well, which is
// why it is a <label> wrapping the button rather than a sibling with a `for`.
function switchRow(content) {
  const on = content.on === true;
  return `<div class="switch">
  <button class="switch__track${on ? ' switch__track--on' : ''}" type="button" role="switch" aria-checked="${on ? 'true' : 'false'}" id="switch-1">
    <span class="switch__thumb"></span>
  </button>
  <label class="switch__label" for="switch-1">${esc(content.label || '')}</label>
</div>`;
}

/*
 * switch-2: the settings row, which is the shape a real preferences page uses. The
 * switch sits RIGHT and the description sits under the label, because the eye reads
 * the name of the setting before its state, and a column of switches down the right
 * edge is scannable in a way an inline row of them is not.
 */
function switchSetting(content) {
  const rows = (content.settings || []).map((s, i) => {
    const on = s.on === true;
    const id = `switch-set-${i + 1}`;
    return `  <div class="switch__setting">
    <div class="switch__text">
      <label class="switch__label" for="${id}">${esc(s.label)}</label>
      ${s.description ? `<p class="switch__desc">${esc(s.description)}</p>` : ''}
    </div>
    <button class="switch__track${on ? ' switch__track--on' : ''}" type="button" role="switch" aria-checked="${on ? 'true' : 'false'}" id="${id}">
      <span class="switch__thumb"></span>
    </button>
  </div>`;
  }).join('\n');
  return `<div class="switch__group" role="group" aria-label="${esc(content.label || 'Settings')}">
${rows}
</div>`;
}

module.exports = {
  'switch-1': switchRow,
  'switch-2': switchSetting,
};
