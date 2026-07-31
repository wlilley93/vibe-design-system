'use strict';
/*
 * FOUR PROJECTIONS OF ONE MANIFEST.
 *
 * Sitemap, wireframe, branded, output. The point is the word PROJECTION: all
 * four are views of the SAME manifest through the SAME renderer, so they cannot
 * disagree about what is on the page.
 *
 * `vds-bridge.js` already recorded the principle - "a wireframe is a projection
 * and not a theme" - and then the repository shipped the opposite: a wireframe
 * was a token PACK, so the wireframe view was a differently-painted page that
 * could drift from the branded one silently. A wireframe drawn separately from
 * the page it wireframes is a drawing of a page that may not exist. That is the
 * whole defect this closes.
 *
 * # The two axes, and why there are exactly four
 *
 *              STYLE            CONTENT              refuses?
 *   sitemap    none (no HTML)   names only           no
 *   wireframe  neutral          structural           no
 *   branded    the token pack   whatever is there    no
 *   output     the token pack   whatever is there    YES - on any placeholder
 *
 * Two axes, four useful points. `sitemap` drops the markup entirely and answers
 * "what is on this page"; `wireframe` keeps every byte of markup and drops the
 * brand, which is what proves the structure works without the styling carrying
 * it; `branded` is the page; `output` is the page WITH A REFUSAL.
 *
 * # Why `output` is not just `branded` again
 *
 * It is the same bytes and a different question. `branded` renders whatever the
 * manifest says, placeholders included, because that is what you want while
 * writing. `output` is the projection you ship, and it REFUSES to render while a
 * CONFIRM marker or an unwritten placeholder is still in the copy. Eleven such
 * markers once reached a live client site in running body copy. A projection
 * that cannot refuse would have shipped them again.
 */

const { renderPage, cssVars, STRUCTURE_CSS, SITE_CSS, BLOCKS } = require('./build.js');
const { auditCopy } = require('./copy.js');

const KINDS = ['sitemap', 'wireframe', 'branded', 'output'];

/*
 * The wireframe stylesheet.
 *
 * Appended AFTER the token layer so it wins, and it works by REDEFINING TOKENS
 * rather than by writing rules against class names. That is the difference
 * between a projection and a second stylesheet: every rule in STRUCTURE_CSS
 * reads `var(--color-*)`, so redefining those variables re-paints the whole page
 * without this file needing to know a single selector. A wireframe written as
 * selectors would need editing every time a block gained one.
 *
 * It carries greys and one boundary weight, which are the only values a
 * wireframe can have and still be a wireframe.
 */
const WIREFRAME_CSS = `
/* WIREFRAME PROJECTION. Redefines the token layer; writes no block selector. */
:root {
  --color-bg: #ffffff;
  --color-surface: #f4f4f5;
  --color-ink: #3f3f46;
  --color-muted: #a1a1aa;
  --color-rule: #d4d4d8;
  --color-accent: #71717a;
  --color-accentInk: #ffffff;
  --color-success: #d4d4d8;   --color-successInk: #3f3f46;
  --color-warning: #d4d4d8;   --color-warningInk: #3f3f46;
  --color-danger: #d4d4d8;    --color-dangerInk: #3f3f46;
  --color-info: #d4d4d8;      --color-infoInk: #3f3f46;
  --shadow: none;
}
/* The one thing a wireframe adds rather than removes: a visible seam between
   blocks, because "where does this section end" is the question a wireframe is
   read to answer. */
[data-block] { outline: 1px dashed var(--color-rule); outline-offset: -1px; position: relative; }
[data-block]::before {
  content: attr(data-block) " / " attr(data-variant);
  position: absolute; top: 0; left: 0; z-index: 9;
  font: 10px/1.4 ui-monospace, monospace; letter-spacing: 0.04em;
  color: var(--color-bg); background: var(--color-muted);
  padding: 2px 6px;
}
`;

/**
 * The structure, with no markup at all.
 *
 * Read from the manifest and checked against the block registry, so a sitemap
 * naming a block that does not exist is a failure here rather than a surprise
 * three projections later.
 */
function sitemap(manifest) {
  const rows = manifest.page.map((entry, i) => {
    const registry = BLOCKS[entry.block];
    const known = Boolean(registry && registry[entry.variant]);
    return {
      order: i + 1,
      block: entry.block,
      variant: entry.variant,
      known,
      // What the entry actually carries, so a sitemap shows an EMPTY section as
      // empty. A structure view that hides missing content is the one that lets
      // an unwritten page look finished.
      contentKeys: Object.keys(entry.content || {}).sort(),
    };
  });
  const unknown = rows.filter((r) => !r.known);
  return {
    layout: manifest.layout || 'page',
    stylePack: manifest.stylePack,
    blocks: rows.length,
    rows,
    unknown: unknown.map((r) => `${r.block}=${r.variant}`),
    text: [
      `${manifest.layout || 'page'} - ${rows.length} blocks - style=${manifest.stylePack}`,
      ...rows.map((r) => `  ${String(r.order).padStart(2)}. ${r.block}/${r.variant}` +
        `${r.known ? '' : '   <- NO SUCH VARIANT'}` +
        `${r.contentKeys.length ? '' : '   <- no content'}`),
    ].join('\n'),
  };
}

/**
 * Every block wrapped so the wireframe can label it.
 *
 * Applied to the RENDERED html rather than asked of each block, because asking
 * 43 blocks to emit a data attribute for one projection's benefit would put a
 * projection's concern inside every block.
 */
function labelBlocks(html, manifest) {
  let i = -1;
  // Each block renders one top-level element. Wrapping is done on the joined
  // output by walking the manifest in order, which is sound because renderPage
  // preserves manifest order and the app layout only regroups, never reorders
  // within a group.
  return html.replace(/(\n?)(<(?:header|section|footer|div|main|nav|aside|article)\b)/g, (m, nl, tag) => {
    i += 1;
    const entry = manifest.page[i];
    if (!entry) return m;
    return `${nl}${tag} data-block="${entry.block}" data-variant="${entry.variant}"`;
  });
}

/**
 * One projection.
 *
 * @param {'sitemap'|'wireframe'|'branded'|'output'} kind
 */
function project(kind, manifest, tokens) {
  if (!KINDS.includes(kind)) {
    throw new Error(`no such projection "${kind}" (have: ${KINDS.join(', ')})`);
  }
  if (kind === 'sitemap') {
    const map = sitemap(manifest);
    if (map.unknown.length) {
      throw new Error(`manifest names blocks that do not exist: ${map.unknown.join(', ')}`);
    }
    return { kind, ...map };
  }

  // THE SAME CALL for all three rendered projections. If this line ever forks
  // per kind, they stop being projections of one thing.
  const { html, css } = renderPage(manifest, tokens, SITE_CSS);

  if (kind === 'output') {
    // The refusal. `auditCopy` is the existing instrument and is reused rather
    // than reimplemented, so "what counts as a placeholder" has one definition.
    const gaps = auditCopy(manifest);
    if (gaps.length) {
      const err = new Error(
        `output refuses: ${gaps.length} unwritten placeholders remain. ` +
        'Use the `branded` projection while writing; `output` is the one that ships.',
      );
      err.gaps = gaps;
      throw err;
    }
    return { kind, html, css, gaps: [] };
  }

  if (kind === 'branded') {
    return { kind, html, css, gaps: auditCopy(manifest) };
  }

  // wireframe: the same html, the token layer overridden.
  return {
    kind,
    html: labelBlocks(html, manifest),
    css: `${css}\n${WIREFRAME_CSS}`,
    gaps: auditCopy(manifest),
  };
}

/**
 * All four, from one manifest.
 *
 * `output` is allowed to refuse, and its refusal is RETURNED rather than thrown,
 * because "three projections rendered and the fourth refuses, here is why" is
 * the useful answer. A throw here would lose the other three.
 */
function projectAll(manifest, tokens) {
  const out = {};
  for (const kind of KINDS) {
    try {
      out[kind] = project(kind, manifest, tokens);
    } catch (e) {
      out[kind] = { kind, refused: e.message, gaps: e.gaps || [] };
    }
  }
  return out;
}

module.exports = { project, projectAll, sitemap, labelBlocks, KINDS, WIREFRAME_CSS };
