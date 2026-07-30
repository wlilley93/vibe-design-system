'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

// Demand measured across 217 Opbox routes: input 54, label 46, textarea 23,
// native-select 22. Together the highest unbuilt need in the codebase, which is why
// this is one block rather than four - a field is a label AND a control AND its help
// text, and splitting them is how a label ends up orphaned from what it names.
function control(f) {
  const id = `f-${String(f.name).replace(/[^a-z0-9]+/gi, '-').toLowerCase()}`;
  const required = f.required ? ' required aria-required="true"' : '';
  const described = f.help || f.error ? ` aria-describedby="${id}-note"` : '';
  const invalid = f.error ? ' aria-invalid="true"' : '';

  let input;
  if (f.type === 'textarea') {
    input = `<textarea class="field__input field__input--area" id="${id}" name="${esc(f.name)}" rows="4" placeholder="${esc(f.placeholder || '')}"${required}${described}${invalid}></textarea>`;
  } else if (f.type === 'select') {
    const opts = (f.options || []).map((o) => `<option>${esc(o)}</option>`).join('');
    input = `<select class="field__input" id="${id}" name="${esc(f.name)}"${required}${described}${invalid}>${opts}</select>`;
  } else if (f.type === 'checkbox') {
    input = `<input class="field__check" type="checkbox" id="${id}" name="${esc(f.name)}"${described}>`;
  } else {
    input = `<input class="field__input" type="${esc(f.type || 'text')}" id="${id}" name="${esc(f.name)}" placeholder="${esc(f.placeholder || '')}"${required}${described}${invalid}>`;
  }

  // The note carries the error when there is one, the help text otherwise. One slot,
  // because a field showing both at once is a field the reader has to disambiguate.
  const note = f.error
    ? `<span class="field__note field__note--error" id="${id}-note">${esc(f.error)}</span>`
    : f.help
      ? `<span class="field__note" id="${id}-note">${esc(f.help)}</span>`
      : '';

  return `<div class="field${f.error ? ' field--invalid' : ''}">
      <label class="field__label" for="${id}">${esc(f.label)}${f.required ? '<span class="field__req" aria-hidden="true">*</span>' : ''}</label>
      ${input}
      ${note}
    </div>`;
}

// formfield-1: a single column of fields. The default for a settings or create form.
function fieldsStacked(content) {
  const fields = content.fields.map(control).join('\n    ');
  return `<section class="fieldset">
  <header class="fieldset__head">
    <h2 class="fieldset__title">${esc(content.heading)}</h2>
    ${content.sub ? `<p class="fieldset__sub">${esc(content.sub)}</p>` : ''}
  </header>
  <form class="fieldset__form" action="${esc(content.formAction || '#')}" method="post">
    ${fields}
    <div class="fieldset__actions">
      <button class="fieldset__submit" type="submit">${esc(content.submitLabel)}</button>
      ${content.cancelLabel ? `<a class="fieldset__cancel" href="${esc(content.cancelHref || '#')}">${esc(content.cancelLabel)}</a>` : ''}
    </div>
  </form>
</section>`;
}

// formfield-2: the same fields in two columns, for a wider surface where a long
// single column would push the submit below the fold.
function fieldsTwoColumn(content) {
  const fields = content.fields.map(control).join('\n    ');
  return `<section class="fieldset fieldset--split">
  <header class="fieldset__head">
    <h2 class="fieldset__title">${esc(content.heading)}</h2>
    ${content.sub ? `<p class="fieldset__sub">${esc(content.sub)}</p>` : ''}
  </header>
  <form class="fieldset__form fieldset__form--grid" action="${esc(content.formAction || '#')}" method="post">
    ${fields}
    <div class="fieldset__actions">
      <button class="fieldset__submit" type="submit">${esc(content.submitLabel)}</button>
      ${content.cancelLabel ? `<a class="fieldset__cancel" href="${esc(content.cancelHref || '#')}">${esc(content.cancelLabel)}</a>` : ''}
    </div>
  </form>
</section>`;
}

module.exports = {
  'formfield-1': fieldsStacked,
  'formfield-2': fieldsTwoColumn,
};
