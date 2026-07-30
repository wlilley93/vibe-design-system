'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

/*
 * Demand: 19 of 224 Opbox routes. It is also "Success Moments", a Modular Play listed
 * under Onboarding, Activation, Monetisation and Growth - more strategies than any
 * other play in the catalogue - whose argument is that an action which completes
 * silently is an action the user is not sure happened.
 *
 * Both variants carry `role="status"` and `aria-live`. A toast that a screen reader
 * never announces is not a confirmation, it is a decoration: the sighted user learns
 * the save worked and nobody else does. That is not a nicety here, it is the whole
 * function of the component.
 *
 * The undoable variant is the more important one and it is deliberately NOT the
 * default. An undo affordance is only honest if the action really is reversible;
 * offering it on something irreversible is worse than not offering it, because the
 * user relaxes about a thing they cannot take back.
 */

// toast-1: the plain confirmation. Something happened, it worked, carry on.
function toastConfirm(content) {
  return `<div class="toast" role="status" aria-live="polite">
  <span class="toast__mark" aria-hidden="true"></span>
  <p class="toast__body">${esc(content.body)}</p>
  ${content.dismissLabel ? `<button class="toast__dismiss" type="button">${esc(content.dismissLabel)}</button>` : ''}
</div>`;
}

/*
 * toast-2: the undoable confirmation, for a destructive-but-reversible action.
 *
 * `aria-live="assertive"` rather than polite: a polite region waits for the reader to
 * finish what it is saying, and this one carries a time-limited offer. An undo the
 * user hears about after the window closes is an undo that was never offered.
 */
function toastUndo(content) {
  return `<div class="toast toast--undo" role="status" aria-live="assertive">
  <span class="toast__mark" aria-hidden="true"></span>
  <p class="toast__body">${esc(content.body)}</p>
  <button class="toast__undo" type="button">${esc(content.undoLabel)}</button>
  ${content.window ? `<span class="toast__window">${esc(content.window)}</span>` : ''}
</div>`;
}

module.exports = {
  'toast-1': toastConfirm,
  'toast-2': toastUndo,
};
