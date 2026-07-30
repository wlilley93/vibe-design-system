'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

/*
 * Demand: destructive-action-dialog, 16 of 217 routes. It is also "Fail Safe", whose
 * Do is "add friction to risky actions, offer soft-deletes/undo" and whose Don't is
 * "let destructive actions fire on a single careless tap".
 *
 * Both variants therefore name WHAT is being destroyed and whether it can be undone.
 * A confirm dialog that says only "Are you sure?" has added a click without adding
 * any information, which is friction with no safety.
 */

// confirmdialog-1: confirm by acknowledging the consequence.
function confirmSimple(content) {
  return `<div class="cdialog" role="alertdialog" aria-labelledby="cdialog-t" aria-describedby="cdialog-b">
  <div class="cdialog__panel">
    <h2 class="cdialog__title" id="cdialog-t">${esc(content.title)}</h2>
    <p class="cdialog__body" id="cdialog-b">${esc(content.body)}</p>
    <p class="cdialog__consequence">${esc(content.consequence)}</p>
    <div class="cdialog__actions">
      <a class="cdialog__cancel" href="${esc(content.cancelHref || '#')}">${esc(content.cancelLabel)}</a>
      <a class="cdialog__confirm" href="${esc(content.confirmHref || '#')}">${esc(content.confirmLabel)}</a>
    </div>
  </div>
</div>`;
}

// confirmdialog-2: type-to-confirm, for the irreversible case. The friction IS the
// safety: the reader has to reproduce the name of the thing they are destroying.
function confirmTyped(content) {
  return `<div class="cdialog cdialog--typed" role="alertdialog" aria-labelledby="cdialog-t" aria-describedby="cdialog-b">
  <div class="cdialog__panel">
    <h2 class="cdialog__title" id="cdialog-t">${esc(content.title)}</h2>
    <p class="cdialog__body" id="cdialog-b">${esc(content.body)}</p>
    <p class="cdialog__consequence">${esc(content.consequence)}</p>
    <form class="cdialog__form" action="${esc(content.confirmHref || '#')}" method="post">
      <label class="cdialog__label" for="cdialog-typed">${esc(content.typePrompt)}</label>
      <input class="cdialog__input" id="cdialog-typed" name="confirm" placeholder="${esc(content.subject)}" autocomplete="off">
      <div class="cdialog__actions">
        <a class="cdialog__cancel" href="${esc(content.cancelHref || '#')}">${esc(content.cancelLabel)}</a>
        <button class="cdialog__confirm" type="submit">${esc(content.confirmLabel)}</button>
      </div>
    </form>
  </div>
</div>`;
}

module.exports = {
  'confirmdialog-1': confirmSimple,
  'confirmdialog-2': confirmTyped,
};
