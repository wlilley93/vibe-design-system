'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

/*
 * The same four tones as `banner` at APP level, above the shell: full width, one line,
 * and nothing below it moves. Base measures it at 1024x48 on web with 14 Medium text.
 *
 * The distinction from `banner` is scope, and it decides which one a page wants. A banner
 * is about the thing you are looking at - this matter is closed, this form has errors. A
 * system banner is about the whole system: read-only maintenance in an hour, your trial
 * ends on Friday, this is a staging environment. It sits outside the page because it
 * would still be true on any other page, and putting system state inside a page means
 * repeating it on every route or losing it on navigation.
 *
 * That scope is also why it is DISMISSIBLE and a banner is not. Page state resolves when
 * the page does; system state persists, so the reader needs a way to acknowledge it and
 * get on. The dismiss control is a real button with a real name, not a bare glyph.
 */

const TONES = new Set(['accent', 'success', 'warning', 'danger', 'info']);
function toneOf(content) {
  return TONES.has(content.tone) ? content.tone : 'info';
}

// systembanner-1: the announcement, dismissible. What most system state is.
function systemBannerDismiss(content) {
  const tone = toneOf(content);
  return `<div class="sysbanner sysbanner--${tone}" role="${tone === 'danger' ? 'alert' : 'status'}">
  <span class="sysbanner__mark" aria-hidden="true"></span>
  <p class="sysbanner__message">${esc(content.message || '')}</p>
  <button class="sysbanner__dismiss" type="button">${esc(content.dismissLabel || 'Dismiss')}</button>
</div>`;
}

/*
 * systembanner-2: with an action and NO dismiss.
 *
 * Deliberately not dismissible, and that is the whole variant: some system state is a
 * blocker rather than a notice - a required action, an expired card, a tenant over its
 * limit - and offering Dismiss on it invites the reader to hide the one thing standing
 * between them and a working account. If it can be dismissed it is variant 1.
 */
function systemBannerAction(content) {
  const tone = toneOf(content);
  return `<div class="sysbanner sysbanner--${tone}" role="${tone === 'danger' ? 'alert' : 'status'}">
  <span class="sysbanner__mark" aria-hidden="true"></span>
  <p class="sysbanner__message">${esc(content.message || '')}</p>
  <a class="sysbanner__link" href="${esc(content.actionHref || '#')}">${esc(content.actionLabel || '')}</a>
</div>`;
}

module.exports = {
  'systembanner-1': systemBannerDismiss,
  'systembanner-2': systemBannerAction,
};
