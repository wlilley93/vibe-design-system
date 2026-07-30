#!/usr/bin/env node
'use strict';

/*
 * site-factory: prototype of the "instant Framer" pipeline.
 *
 *   node build.js manifests/manifest-a.json
 *
 * A manifest names one variant per block plus a style pack. This script never makes
 * a design decision itself — it reads the manifest, reads the named token file, renders
 * the named block variants, and emits one static HTML file plus one CSS file. The same
 * two block variants must produce different output for two reasons only: a different
 * variant was chosen, or a different style pack was chosen. Nothing else may vary.
 *
 * This mirrors the Balmoral site generator's shape (content JSON -> declarative schema
 * -> static HTML, zero dependencies) generalized to a block-selection manifest instead
 * of one fixed page shape.
 */

const fs = require('fs');
const path = require('path');

const ROOT = __dirname;
const DIST = path.join(ROOT, 'dist');
const BLOCKS_DIR = path.join(ROOT, 'blocks');

// One registry entry per block TYPE (filename), each holding its variant functions.
// A block type is never hardcoded here — dropping a new file in blocks/ makes it
// available to every manifest with no change to this file.
const BLOCKS = {};
for (const file of fs.readdirSync(BLOCKS_DIR)) {
  if (!file.endsWith('.js')) continue;
  const type = path.basename(file, '.js');
  BLOCKS[type] = require(path.join(BLOCKS_DIR, file));
}

function readJson(p) {
  return JSON.parse(fs.readFileSync(p, 'utf8'));
}

// Multipliers for the three scale choices the config layer offers. They live here,
// next to the stylesheet that consumes them, rather than in the wizard: a control
// the UI can rotate but the CSS never reads is a control that lies.
const DENSITY = { compact: 0.75, comfortable: 1, spacious: 1.35 };
const TYPE_SCALE = { compact: 0.9, comfortable: 1, spacious: 1.15 };
const BORDER_WEIGHT = { hairline: '1px', '1px': '1px', 'bold-2px': '2px' };
const ELEVATION = {
  flat: 'none',
  'soft-shadow': '0 1px 3px rgb(0 0 0 / 0.08), 0 8px 24px rgb(0 0 0 / 0.06)',
  'hard-offset': '4px 4px 0 var(--color-ink)',
};

function cssVars(tokens) {
  const lines = [':root {'];
  for (const [k, v] of Object.entries(tokens.colors)) lines.push(`  --color-${k}: ${v};`);
  lines.push(`  --font-family: ${tokens.font.family};`);
  lines.push(`  --font-mono: ${tokens.font.mono};`);
  for (const [k, v] of Object.entries(tokens.radius)) lines.push(`  --radius-${k}: ${v};`);

  // Density folds straight into --space, so every `calc(var(--space) * N)` in the
  // stylesheet responds without a second variable threaded through each rule.
  // Defaults keep the four token files on disk (and every scaffold's copy of them)
  // valid unchanged: a pack with no `scale` block renders exactly as it did before.
  const scale = tokens.scale || {};
  const density = DENSITY[scale.density] ?? 1;
  lines.push(`  --space: ${(tokens.space.unit * density).toFixed(2)}px;`);
  lines.push(`  --type-scale: ${TYPE_SCALE[scale.type] ?? 1};`);
  lines.push(`  --border-weight: ${BORDER_WEIGHT[(tokens.border || {}).weight] || '1px'};`);
  lines.push(`  --shadow: ${ELEVATION[tokens.elevation] || 'none'};`);
  lines.push('}');
  return lines.join('\n');
}

// Fixed structural stylesheet. Every value is a var(--token) or a multiple of --space.
// No hex, no font name, no px literal outside the --space multiplier: that is the whole
// point of the prototype, and it is what a `grep -E "#[0-9a-f]{3,6}"` on this block
// should find zero of.
const STRUCTURE_CSS = `
* { box-sizing: border-box; }
body {
  margin: 0;
  background: var(--color-bg);
  color: var(--color-ink);
  font-family: var(--font-family);
  line-height: 1.5;
}
a { color: var(--color-accent); }

.hero { padding: calc(var(--space) * 24) calc(var(--space) * 8); }
.hero--centered { text-align: center; }
.hero--centered .hero__inner { max-width: 640px; margin: 0 auto; }
.hero--split {
  display: grid;
  grid-template-columns: 1fr 1fr;
  align-items: center;
  gap: calc(var(--space) * 12);
  max-width: 1100px;
  margin: 0 auto;
}
.hero__h1 { font-size: calc(2.75rem * var(--type-scale)); line-height: 1.1; margin: 0 0 calc(var(--space) * 4); }
.hero__sub { font-size: calc(1.125rem * var(--type-scale)); color: var(--color-muted); margin: 0 0 calc(var(--space) * 6); }
.hero__cta {
  display: inline-block;
  background: var(--color-accent);
  color: var(--color-accentInk);
  text-decoration: none;
  padding: calc(var(--space) * 3) calc(var(--space) * 6);
  border-radius: var(--radius-sm);
  font-weight: 600;
}
.hero__media {
  background: var(--color-surface);
  border: var(--border-weight) solid var(--color-border);
  border-radius: var(--radius-lg);
  aspect-ratio: 4 / 3;
}

.footer { border-top: var(--border-weight) solid var(--color-border); padding: calc(var(--space) * 8); }
.footer--simple .footer__row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: calc(var(--space) * 6);
  max-width: 1100px;
  margin: 0 auto;
  flex-wrap: wrap;
}
.footer__mark { font-weight: 700; }
.footer__nav { display: flex; gap: calc(var(--space) * 5); }
.footer__link { color: var(--color-muted); text-decoration: none; font-size: calc(0.875rem * var(--type-scale)); }
.footer__link:hover { color: var(--color-accent); }
.footer__copyright { color: var(--color-muted); font-size: calc(0.8125rem * var(--type-scale)); }

.footer--columns .footer__grid {
  display: grid;
  grid-template-columns: 1.4fr repeat(3, 1fr);
  gap: calc(var(--space) * 8);
  max-width: 1100px;
  margin: 0 auto calc(var(--space) * 8);
}
.footer__tagline { color: var(--color-muted); font-size: calc(0.875rem * var(--type-scale)); margin-top: calc(var(--space) * 2); }
.footer__colTitle { font-size: calc(0.75rem * var(--type-scale)); text-transform: uppercase; letter-spacing: 0.04em; color: var(--color-muted); margin: 0 0 calc(var(--space) * 3); }
.footer__colLinks { display: flex; flex-direction: column; gap: calc(var(--space) * 2); }
.footer--columns .footer__bottom {
  max-width: 1100px;
  margin: 0 auto;
  padding-top: calc(var(--space) * 6);
  border-top: var(--border-weight) solid var(--color-border);
}

.nav { display: flex; align-items: center; justify-content: space-between; padding: calc(var(--space) * 5) calc(var(--space) * 8); border-bottom: var(--border-weight) solid var(--color-border); }
.nav__mark { font-weight: 700; }
.nav__links, .nav__side { display: flex; gap: calc(var(--space) * 6); align-items: center; }
.nav__link { color: var(--color-ink); text-decoration: none; font-size: calc(0.875rem * var(--type-scale)); }
.nav__link:hover { color: var(--color-accent); }
.nav__cta { background: var(--color-accent); color: var(--color-accentInk); text-decoration: none; padding: calc(var(--space) * 2) calc(var(--space) * 4); border-radius: var(--radius-sm); font-size: calc(0.875rem * var(--type-scale)); font-weight: 600; }
.nav--centered { justify-content: space-between; }
.nav--centered .nav__mark { position: absolute; left: 50%; transform: translateX(-50%); }
.nav--centered { position: relative; }

.pricing { padding: calc(var(--space) * 16) calc(var(--space) * 8); max-width: 1100px; margin: 0 auto; }
.pricing__heading { font-size: calc(1.75rem * var(--type-scale)); text-align: center; margin: 0 0 calc(var(--space) * 10); }
.pricing__grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: calc(var(--space) * 6); }
.pricing__card { border: var(--border-weight) solid var(--color-border); border-radius: var(--radius-lg); padding: calc(var(--space) * 6); box-shadow: var(--shadow); }
.pricing__card--highlight { border-color: var(--color-accent); border-width: 2px; }
.pricing__planName { margin: 0 0 calc(var(--space) * 2); font-size: calc(1rem * var(--type-scale)); }
.pricing__price { font-size: calc(1.75rem * var(--type-scale)); font-weight: 700; margin: 0 0 calc(var(--space) * 4); }
.pricing__features { list-style: none; padding: 0; margin: 0 0 calc(var(--space) * 6); color: var(--color-muted); font-size: calc(0.875rem * var(--type-scale)); }
.pricing__features li { padding: calc(var(--space) * 1) 0; }
.pricing__cta { display: block; text-align: center; background: var(--color-accent); color: var(--color-accentInk); text-decoration: none; padding: calc(var(--space) * 3); border-radius: var(--radius-sm); font-weight: 600; }
.pricing__matrix { width: 100%; border-collapse: collapse; }
.pricing__matrix th, .pricing__matrix td { border-bottom: var(--border-weight) solid var(--color-border); padding: calc(var(--space) * 3); text-align: left; }

.testimonials { padding: calc(var(--space) * 16) calc(var(--space) * 8); max-width: 1100px; margin: 0 auto; }
.testimonials__heading { font-size: calc(1.75rem * var(--type-scale)); text-align: center; margin: 0 0 calc(var(--space) * 10); }
.testimonials__gridInner { display: grid; grid-template-columns: repeat(3, 1fr); gap: calc(var(--space) * 6); }
.testimonials__card { border: var(--border-weight) solid var(--color-border); border-radius: var(--radius-lg); padding: calc(var(--space) * 6); margin: 0; box-shadow: var(--shadow); }
.testimonials__card blockquote { margin: 0 0 calc(var(--space) * 4); font-size: calc(0.9375rem * var(--type-scale)); }
.testimonials__card figcaption { color: var(--color-muted); font-size: calc(0.8125rem * var(--type-scale)); }
.testimonials--featured { text-align: center; }
.testimonials__big { font-size: calc(1.75rem * var(--type-scale)); line-height: 1.3; margin: 0 0 calc(var(--space) * 6); }
.testimonials__attribution { color: var(--color-muted); }

.features { padding: calc(var(--space) * 16) calc(var(--space) * 8); max-width: 1100px; margin: 0 auto; }
.features__heading { font-size: calc(1.75rem * var(--type-scale)); text-align: center; margin: 0 0 calc(var(--space) * 10); }
.features__gridInner { display: grid; grid-template-columns: repeat(3, 1fr); gap: calc(var(--space) * 8); }
.features__item h3 { margin: 0 0 calc(var(--space) * 2); font-size: calc(1rem * var(--type-scale)); }
.features__item p { margin: 0; color: var(--color-muted); font-size: calc(0.875rem * var(--type-scale)); }
.features__matrix { width: 100%; border-collapse: collapse; }
.features__matrix th, .features__matrix td { border-bottom: var(--border-weight) solid var(--color-border); padding: calc(var(--space) * 3); text-align: center; }
.features__matrix td:first-child, .features__matrix th:first-child { text-align: left; }

.faq { padding: calc(var(--space) * 16) calc(var(--space) * 8); max-width: 800px; margin: 0 auto; }
.faq__heading { font-size: calc(1.75rem * var(--type-scale)); text-align: center; margin: 0 0 calc(var(--space) * 10); }
.faq__item { border-bottom: var(--border-weight) solid var(--color-border); padding: calc(var(--space) * 4) 0; }
.faq__item summary { cursor: pointer; font-weight: 600; }
.faq__item p { color: var(--color-muted); margin: calc(var(--space) * 3) 0 0; }
.faq--columns .faq__grid { display: grid; grid-template-columns: 1fr 1fr; gap: calc(var(--space) * 6); }
.faq__pair h3 { margin: 0 0 calc(var(--space) * 2); font-size: calc(1rem * var(--type-scale)); }
.faq__pair p { color: var(--color-muted); margin: 0; font-size: calc(0.875rem * var(--type-scale)); }

.cta { padding: calc(var(--space) * 16) calc(var(--space) * 8); text-align: center; background: var(--color-surface); }
.cta h2 { font-size: calc(1.75rem * var(--type-scale)); margin: 0 0 calc(var(--space) * 3); }
.cta p { color: var(--color-muted); margin: 0 0 calc(var(--space) * 6); }
.cta__button { display: inline-block; background: var(--color-accent); color: var(--color-accentInk); text-decoration: none; padding: calc(var(--space) * 3) calc(var(--space) * 6); border-radius: var(--radius-sm); font-weight: 600; border: none; font-size: calc(1rem * var(--type-scale)); cursor: pointer; }
.cta--signup { display: flex; align-items: center; justify-content: space-between; text-align: left; max-width: 1100px; margin: calc(var(--space) * 16) auto; }
.cta--signup .cta__form { display: flex; gap: calc(var(--space) * 3); }
.cta__input { padding: calc(var(--space) * 3); border: var(--border-weight) solid var(--color-border); border-radius: var(--radius-sm); font-size: calc(0.875rem * var(--type-scale)); }

.sidebar { width: 240px; padding: calc(var(--space) * 6); border-right: var(--border-weight) solid var(--color-border); }
.sidebar__nav, .sidebar__group { display: flex; flex-direction: column; gap: calc(var(--space) * 1); }
.sidebar__link { color: var(--color-muted); text-decoration: none; padding: calc(var(--space) * 2) calc(var(--space) * 3); border-radius: var(--radius-sm); font-size: calc(0.875rem * var(--type-scale)); }
.sidebar__link--active { background: var(--color-surface); color: var(--color-ink); font-weight: 600; }
.sidebar__group { margin-bottom: calc(var(--space) * 5); }
.sidebar__groupTitle { font-size: calc(0.6875rem * var(--type-scale)); text-transform: uppercase; letter-spacing: 0.04em; color: var(--color-muted); margin: 0 0 calc(var(--space) * 2) calc(var(--space) * 3); }

.team { padding: calc(var(--space) * 16) calc(var(--space) * 8); max-width: 1100px; margin: 0 auto; }
.team__heading { font-size: calc(1.75rem * var(--type-scale)); text-align: center; margin: 0 0 calc(var(--space) * 10); }
.team__gridInner { display: grid; grid-template-columns: repeat(4, 1fr); gap: calc(var(--space) * 6); }
.team__photo { aspect-ratio: 1; background: var(--color-surface); border: var(--border-weight) solid var(--color-border); border-radius: var(--radius-lg); margin-bottom: calc(var(--space) * 3); }
.team__card h3, .team__row h3 { margin: 0; font-size: calc(0.9375rem * var(--type-scale)); }
.team__card p { margin: 0; color: var(--color-muted); font-size: calc(0.8125rem * var(--type-scale)); }
.team__listInner { display: flex; flex-direction: column; gap: calc(var(--space) * 6); }
.team__row { display: flex; gap: calc(var(--space) * 5); align-items: center; }
.team__row .team__photo { width: 64px; height: 64px; flex-shrink: 0; margin: 0; }
.team__row h3 span { display: block; font-weight: 400; color: var(--color-muted); font-size: calc(0.8125rem * var(--type-scale)); }
.team__row p { margin: calc(var(--space) * 1) 0 0; color: var(--color-muted); font-size: calc(0.875rem * var(--type-scale)); }

.contact { padding: calc(var(--space) * 16) calc(var(--space) * 8); max-width: 640px; margin: 0 auto; }
.contact--split { max-width: 1100px; display: grid; grid-template-columns: 1fr 1fr; gap: calc(var(--space) * 12); }
.contact__email { color: var(--color-accent); font-weight: 600; }
.contact__form { display: flex; flex-direction: column; gap: calc(var(--space) * 3); }
.contact__input, .contact__textarea { padding: calc(var(--space) * 3); border: var(--border-weight) solid var(--color-border); border-radius: var(--radius-sm); font-family: inherit; font-size: calc(0.875rem * var(--type-scale)); }
.contact__button { background: var(--color-accent); color: var(--color-accentInk); border: none; padding: calc(var(--space) * 3); border-radius: var(--radius-sm); font-weight: 600; cursor: pointer; font-size: calc(1rem * var(--type-scale)); }

.notfound { padding: calc(var(--space) * 24) calc(var(--space) * 8); text-align: center; max-width: 480px; margin: 0 auto; }
.notfound__code { color: var(--color-muted); font-family: var(--font-mono); margin: 0 0 calc(var(--space) * 3); }
.notfound h1 { font-size: calc(1.75rem * var(--type-scale)); margin: 0 0 calc(var(--space) * 3); }
.notfound p { color: var(--color-muted); margin: 0 0 calc(var(--space) * 6); }
.notfound__link { color: var(--color-accent); font-weight: 600; text-decoration: none; }
.notfound__searchForm { margin: 0 0 calc(var(--space) * 6); }
.notfound__searchInput { width: 100%; padding: calc(var(--space) * 3); border: var(--border-weight) solid var(--color-border); border-radius: var(--radius-sm); font-size: calc(0.875rem * var(--type-scale)); }
`;

/*
 * The pure core: a manifest plus a token object in, a page out. No disk, no
 * process, no console.
 *
 * Extracted so studio.js can render a live preview through THIS code path rather
 * than keeping a second copy of the render logic in browser JS. Two renderers
 * would be two sources of truth, and the preview would eventually disagree with
 * what actually compiles. `stylesheetHref` is null for a self-contained document
 * with the CSS inlined (what the studio serves into an iframe), or a filename for
 * the linked pair the CLI writes.
 */
function renderPage(manifest, tokens, stylesheetHref) {
  const bodyHtml = manifest.page.map((entry) => {
    const blockRegistry = BLOCKS[entry.block];
    if (!blockRegistry) throw new Error(`no block type "${entry.block}" (looked in blocks/)`);
    const renderFn = blockRegistry[entry.variant];
    if (!renderFn) throw new Error(`no variant "${entry.variant}" for block "${entry.block}"`);
    return renderFn(entry.content);
  }).join('\n');

  const css = cssVars(tokens) + '\n' + STRUCTURE_CSS;
  const head = stylesheetHref
    ? `<link rel="stylesheet" href="${stylesheetHref}">`
    : `<style>\n${css}\n</style>`;

  const html = `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${manifest.title || 'untitled'}</title>
${head}
</head>
<body>
${bodyHtml}
</body>
</html>
`;
  return { html, css };
}

function build(manifestPath) {
  const manifest = readJson(manifestPath);
  const tokens = readJson(path.join(ROOT, 'tokens', `${manifest.stylePack}.json`));
  const name = path.basename(manifestPath, '.json');

  const { html, css } = renderPage(manifest, tokens, `${name}.css`);

  fs.mkdirSync(DIST, { recursive: true });
  fs.writeFileSync(path.join(DIST, `${name}.css`), css);
  fs.writeFileSync(path.join(DIST, `${name}.html`), html);
  const summary = manifest.page.map((e) => `${e.block}=${e.variant}`).join(' ');
  console.log(`${name}: ${summary} style=${manifest.stylePack} -> dist/${name}.html`);
}

module.exports = { renderPage, build, cssVars, BLOCKS, STRUCTURE_CSS };

if (require.main === module) {
  const arg = process.argv[2];
  if (!arg) {
    console.error('usage: node build.js manifests/<name>.json');
    process.exit(1);
  }
  build(path.resolve(ROOT, arg));
}
