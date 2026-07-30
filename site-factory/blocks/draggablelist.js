'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

/*
 * In tier 1 despite only 10 variants in Base, because a data surface needs reorderable
 * rows and no marketing page ever does.
 *
 * THE DRAG HANDLE IS NOT THE CONTROL. It is one way to operate the control, and it is
 * the way that excludes anyone not using a mouse or a steady hand. So every row here
 * carries explicit Move up and Move down buttons alongside the handle, and the handle is
 * `aria-hidden` because it is the mouse affordance for an action the buttons already
 * name. A list that can only be reordered by dragging is a list some readers cannot
 * reorder at all, and that is a defect rather than a limitation.
 *
 * Measured from Base: 80 high with artwork (an 80 square frame, 16 padding, 48
 * container), 76 without, a 64-wide control frame, and the divider INSET 16, or 80 with
 * artwork, so it starts under the text.
 */

function handle() {
  // Three bars, the conventional grip. Hidden from the reader because Move up and Move
  // down say the same thing in words.
  return `<span class="drag__handle" aria-hidden="true"><span></span><span></span><span></span></span>`;
}

function moveButtons(i, total, label) {
  const first = i === 0;
  const last = i === total - 1;
  return `      <button class="drag__move" type="button"${first ? ' aria-disabled="true"' : ''}>
        <span aria-hidden="true">&uarr;</span><span class="drag__sr">Move ${esc(label)} up</span>
      </button>
      <button class="drag__move" type="button"${last ? ' aria-disabled="true"' : ''}>
        <span aria-hidden="true">&darr;</span><span class="drag__sr">Move ${esc(label)} down</span>
      </button>`;
}

function row(it, i, total, { artwork = false } = {}) {
  const art = artwork
    ? `    <span class="drag__art" aria-hidden="true"></span>\n`
    : '';
  return `  <li class="drag__row${artwork ? ' drag__row--art' : ''}">
${art}    <span class="drag__text">
      <span class="drag__title">${esc(it.label)}</span>
      ${it.meta ? `<span class="drag__meta">${esc(it.meta)}</span>` : ''}
    </span>
    <span class="drag__controls">
${moveButtons(i, total, it.label)}
      ${handle()}
    </span>
  </li>`;
}

/*
 * draggablelist-1: text rows. An ORDERED list, because the order is the data - that is
 * the entire point of the component - and an unordered list would say the sequence does
 * not matter.
 */
function dragPlain(content) {
  const items = content.items || [];
  const rows = items.map((it, i) => row(it, i, items.length)).join('\n');
  return `<div class="drag">
  <p class="drag__hint" id="drag-hint">${esc(content.hint || 'Drag a row, or use the arrow buttons.')}</p>
  <ol class="drag__list" aria-describedby="drag-hint" aria-label="${esc(content.label || 'Order')}">
${rows}
  </ol>
</div>`;
}

/*
 * draggablelist-2: with artwork and an explicit position number.
 *
 * The number is the reason this is a separate variant. When the order carries meaning a
 * reader must be able to state - the second stage of a workflow, the third priority -
 * the position needs saying rather than inferring from where the row happens to sit. An
 * ordered list numbers itself visually; it does not put the number where a reader can
 * quote it back.
 */
function dragArtwork(content) {
  const items = content.items || [];
  const rows = items.map((it, i) => {
    const base = row(it, i, items.length, { artwork: true });
    return base.replace('<span class="drag__text">', `<span class="drag__pos" aria-hidden="true">${i + 1}</span>\n    <span class="drag__text">`);
  }).join('\n');
  return `<div class="drag drag--art">
  <p class="drag__hint" id="drag-hint-art">${esc(content.hint || 'Drag a row, or use the arrow buttons.')}</p>
  <ol class="drag__list" aria-describedby="drag-hint-art" aria-label="${esc(content.label || 'Order')}">
${rows}
  </ol>
</div>`;
}

module.exports = {
  'draggablelist-1': dragPlain,
  'draggablelist-2': dragArtwork,
};
