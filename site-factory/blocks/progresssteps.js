'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

/*
 * The component `formfield` has no answer for, which is why it is in tier 1 on only 3
 * variants in Base: a multi-step form with no step indicator is a form that will not say
 * how long it is, and the commonest reason people abandon one.
 *
 * Three states, derived rather than carried: everything before the current step is done,
 * the current step is current, everything after is to come. Passing a per-step state
 * would let a caller write step 3 as done while step 2 is still to come, which is a
 * progress indicator that has lost track of the progress.
 *
 * An ORDERED list, because the sequence is the content. `aria-current="step"` on the one
 * in progress, and the done steps say "done" in text rather than only as a tick, since a
 * tick glyph read aloud is "check mark" at best and nothing at worst.
 */

const DONE = 'done', CURRENT = 'current', TODO = 'todo';

function stateOf(index, currentIndex) {
  if (index < currentIndex) return DONE;
  if (index === currentIndex) return CURRENT;
  return TODO;
}

function marker(i, state) {
  const glyph = state === DONE ? '&check;' : String(i + 1);
  return `      <span class="psteps__dot psteps__dot--${state}" aria-hidden="true">${glyph}</span>`;
}

function stateWord(state) {
  return state === DONE ? 'Completed' : (state === CURRENT ? 'In progress' : 'Not started');
}

// progresssteps-1: horizontal, labels under the markers. The compact form, for a form
// whose steps have short names and no explaining to do.
function stepsHorizontal(content) {
  const items = content.items || [];
  const current = Math.min(Math.max(Number(content.current) || 1, 1), items.length) - 1;
  const rows = items.map((it, i) => {
    const state = stateOf(i, current);
    return `    <li class="psteps__step psteps__step--${state}"${state === CURRENT ? ' aria-current="step"' : ''}>
${marker(i, state)}
      <span class="psteps__label">${esc(it.label)}</span>
      <span class="psteps__sr">${stateWord(state)}</span>
    </li>`;
  }).join('\n');
  return `<nav class="psteps" aria-label="${esc(content.label || 'Progress')}">
  <ol class="psteps__row">
${rows}
  </ol>
</nav>`;
}

/*
 * progresssteps-2: vertical, with a line of detail per step.
 *
 * Not just a rotation. The vertical form has room for the detail that decides whether a
 * reader can act - who a step is waiting on, what date it cleared - and that detail is
 * what turns a progress indicator into a status report. The connector is drawn as part of
 * each step rather than as a separate element, so it cannot be squeezed to nothing by the
 * spacing around it. That failure is on the record from the Figma redraw.
 */
function stepsVertical(content) {
  const items = content.items || [];
  const current = Math.min(Math.max(Number(content.current) || 1, 1), items.length) - 1;
  const rows = items.map((it, i) => {
    const state = stateOf(i, current);
    const last = i === items.length - 1;
    return `    <li class="psteps__step psteps__step--${state}"${state === CURRENT ? ' aria-current="step"' : ''}>
      <span class="psteps__rail">
${marker(i, state)}
        ${last ? '' : '<span class="psteps__line" aria-hidden="true"></span>'}
      </span>
      <span class="psteps__body">
        <span class="psteps__label">${esc(it.label)}</span>
        ${it.detail ? `<span class="psteps__detail">${esc(it.detail)}</span>` : ''}
        <span class="psteps__sr">${stateWord(state)}</span>
      </span>
    </li>`;
  }).join('\n');
  return `<nav class="psteps psteps--vertical" aria-label="${esc(content.label || 'Progress')}">
  <ol class="psteps__col">
${rows}
  </ol>
</nav>`;
}

module.exports = {
  'progresssteps-1': stepsHorizontal,
  'progresssteps-2': stepsVertical,
};
