'use strict';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

/*
 * Relume gives Timelines 21 sets. `progresssteps` looked like it already answered this and
 * does not, and the distinction is worth stating because it is the same shape doing a
 * different job: PROGRESS STEPS ARE ABOUT THE READER and a timeline is about the subject.
 *
 * Steps have a current position, because the reader is somewhere in them - three of five,
 * with two to go. A timeline has no current position and no reader-state at all: it is a
 * record of things that happened, or a plan of things that will. So this block takes DATES
 * and never a `current` index, and `progresssteps` takes a `current` index and never dates.
 * Conflating them produces a company history with a "you are here" marker on it.
 *
 * An ordered list, because the sequence is the content. `<time>` on every date that parses,
 * because a date a machine cannot read is a date only some readers get.
 */

function entry(e, last) {
  const when = e.iso
    ? `<time class="timeline__when" datetime="${esc(e.iso)}">${esc(e.when)}</time>`
    : `<span class="timeline__when">${esc(e.when)}</span>`;
  return `    <li class="timeline__entry${last ? ' timeline__entry--last' : ''}">
      <span class="timeline__rail" aria-hidden="true">
        <span class="timeline__dot"></span>
        ${last ? '' : '<span class="timeline__line"></span>'}
      </span>
      <span class="timeline__body">
        ${when}
        <span class="timeline__title">${esc(e.title)}</span>
        ${e.body ? `<span class="timeline__text">${esc(e.body)}</span>` : ''}
      </span>
    </li>`;
}

// timeline-1: vertical. The default, because a date and a paragraph need a line length, and
// a horizontal timeline gives each entry a column too narrow to say anything in.
function timelineVertical(content) {
  const items = content.entries || [];
  const rows = items.map((e, i) => entry(e, i === items.length - 1)).join('\n');
  return `<section class="timeline">
  ${content.heading ? `<h2 class="timeline__heading">${esc(content.heading)}</h2>` : ''}
  <ol class="timeline__list">
${rows}
  </ol>
</section>`;
}

/*
 * timeline-2: horizontal, for a short sequence of milestones with short labels.
 *
 * Its own variant rather than a flag, because it changes what the component can HOLD: four
 * entries of a few words each, and no paragraphs. Passing a body to this variant would
 * produce a row of unreadable columns, so it renders the title and the date and drops the
 * body rather than squeezing it - and the comment says so, because silently discarding
 * content a caller supplied is worse than either rendering or refusing it.
 */
function timelineHorizontal(content) {
  const items = content.entries || [];
  const rows = items.map((e, i) => {
    const when = e.iso
      ? `<time class="timeline__when" datetime="${esc(e.iso)}">${esc(e.when)}</time>`
      : `<span class="timeline__when">${esc(e.when)}</span>`;
    return `    <li class="timeline__milestone">
      <span class="timeline__rail" aria-hidden="true"><span class="timeline__dot"></span></span>
      ${when}
      <span class="timeline__title">${esc(e.title)}</span>
    </li>`;
  }).join('\n');
  return `<section class="timeline timeline--horizontal">
  ${content.heading ? `<h2 class="timeline__heading">${esc(content.heading)}</h2>` : ''}
  <ol class="timeline__track">
${rows}
  </ol>
</section>`;
}

module.exports = {
  'timeline-1': timelineVertical,
  'timeline-2': timelineHorizontal,
};
