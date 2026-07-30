'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

/*
 * IN-FLOW messaging, which is the whole distinction from `toast` and `confirmdialog`.
 * Base measures it at 343x100, radius 12, a 1px border, 16 padding and a 112 artwork.
 *
 * A toast is transient and a dialog is modal, so neither can hold something the reader
 * needs to come back to. A message card sits in the page and stays: an onboarding
 * prompt, a nudge to connect a bank feed, an explanation of why a list is short. It is
 * bordered rather than tonal for the same reason - a tone would say "state", and this is
 * not state, it is an invitation.
 *
 * Its action is TERTIARY on purpose. A message card competing with the page's own primary
 * button is a message card that has stopped being an aside, and the surest way to make a
 * reader ignore the real call to action is to put a second one next to it.
 */

// messagecard-1: with artwork. The illustration earns its space when the message is an
// invitation rather than an instruction; it is decorative, so it is aria-hidden.
function messageCardArtwork(content) {
  return `<div class="mcard">
  <div class="mcard__content">
    <p class="mcard__heading">${esc(content.heading || '')}</p>
    <p class="mcard__body">${esc(content.body || '')}</p>
    ${content.actionLabel ? `<a class="mcard__action" href="${esc(content.actionHref || '#')}">${esc(content.actionLabel)}</a>` : ''}
  </div>
  <span class="mcard__art" aria-hidden="true"></span>
</div>`;
}

/*
 * messagecard-2: text only.
 *
 * Not a lesser variant. Artwork on a message inside a dense surface - a table, a form, an
 * inspector - is a picture in a place the reader came to read, and it pushes the content
 * they wanted further down. The text-only form is the one to reach for by default, and the
 * artwork form is for a page that has room to be welcoming.
 */
function messageCardText(content) {
  return `<div class="mcard">
  <div class="mcard__content">
    <p class="mcard__heading">${esc(content.heading || '')}</p>
    <p class="mcard__body">${esc(content.body || '')}</p>
    ${content.actionLabel ? `<a class="mcard__action" href="${esc(content.actionHref || '#')}">${esc(content.actionLabel)}</a>` : ''}
  </div>
</div>`;
}

module.exports = {
  'messagecard-1': messageCardArtwork,
  'messagecard-2': messageCardText,
};
