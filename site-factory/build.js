#!/usr/bin/env node
'use strict';

/*
 * site-factory: prototype of the "instant Framer" pipeline.
 *
 *   node build.js manifests/manifest-a.json
 *
 * A manifest names one variant per block plus a style pack. This script never makes
 * a design decision itself - it reads the manifest, reads the named token file, renders
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
// A block type is never hardcoded here - dropping a new file in blocks/ makes it
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

// Named here beside DENSITY and TYPE_SCALE, so every scale this stylesheet understands is
// in one place and `tests/spec.test.js` can pin the spec sheet against the real values.
/*
 * TYPE. A rationalised ramp, and leading paired to ROLE.
 *
 * Measured before changing anything: 103 font-size declarations in this stylesheet and
 * THREE carried a line-height. A hundred inherited whatever the cascade gave them. The ramp
 * held 10, 11, 12, 13, 14, 15 - six sizes inside a 6px span, which is the same arbitrariness
 * the width tokens removed when 480, 460 and 420 turned out to be one role.
 *
 * The structure is taken from Uber Base, measured out of its Figma file rather than scraped
 * (vendor/uber-base-typescale.json). Its insight is that LEADING IS A FUNCTION OF ROLE, not
 * of size: Label and Paragraph share every size (18/16/14/12) and differ only in line height
 * (24/20/16/16 against 28/24/20/20). A label does not wrap so it can be tight; body copy
 * wraps so it needs air. Three global line-heights cannot express that.
 *
 * Base's ratios also rise as size falls - display 1.17 at 96px, 1.29 at 28px, paragraph 1.50
 * at 16px and 1.67 at 12px - because small text needs proportionally more leading. So the
 * leading here is a per-role ratio and not one number.
 *
 * The ramp is Base's, minus the steps nothing here uses. 10 and 11 fold to 12, 13 to 14,
 * 15 to 16: below 12px is under the floor for anything a reader has to read, and four sizes
 * between 10 and 13 were never four decisions.
 */
const TYPE_STEPS = { xs: 12, sm: 14, md: 16, lg: 18, xl: 20, '2xl': 24, '3xl': 28, '4xl': 32, '5xl': 36, '6xl': 44 };

// Leading by role, from Base's measured ratios at the sizes each role actually uses.
// `label` is tight because a label does not wrap. `body` is loose because it does.
const TYPE_LEADING = { display: 1.15, heading: 1.25, label: 1.2, body: 1.5 };

const BUTTON_RADIUS = { rounded: 'var(--radius-sm)', square: '0px', pill: '999px' };
const TABLE_DENSITY = { compact: 2, comfortable: 3.5 };
const MOTION_DURATION = { none: '0s', subtle: '160ms', expressive: '320ms' };
const MOTION_EASE = { fade: 'ease', slide: 'cubic-bezier(0.2, 0, 0, 1)', scale: 'cubic-bezier(0.34, 1.56, 0.64, 1)' };
const MOTION_DISTANCE = { fade: '0px', slide: 'calc(var(--space) * 2)', scale: '0px' };

function cssVars(tokens) {
  const lines = [':root {'];
  for (const [k, v] of Object.entries(tokens.colors)) lines.push(`  --color-${k}: ${v};`);
  lines.push(`  --font-family: ${tokens.font.family};`);
  lines.push(`  --font-mono: ${tokens.font.mono};`);
  // The body face, which pairingStyle chooses. Defaults to the display family so the four
  // token files on disk (and every scaffold's copy) stay valid with no `body` key.
  lines.push(`  --font-body: ${tokens.font.body || tokens.font.family};`);
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

  /*
   * The six controls that used to change nothing.
   *
   * The README's first rule is "a control the renderer ignores is a control that lies",
   * and it named four fields as the ones that had been caught. Eight more were never read
   * at all: an audit rendered every option of every enum field and found nineteen fields,
   * eleven reachable. That claim was false for nearly half the schema.
   *
   * Each of these folds into a variable rather than a per-rule branch, for the same reason
   * density does: one place to set it, and every rule that mentions it responds.
   */

  // Button shape. This is deliberately NOT --radius-sm: a pill button next to a sharp
  // panel is a real design language (Stripe does it), so the two have to be separable.
  lines.push(`  --button-radius: ${BUTTON_RADIUS[(tokens.componentStyle || {}).buttonShape] ?? 'var(--radius-sm)'};`);

  // Table row padding, as a multiplier on --space so it tracks density too.
  lines.push(`  --row-pad: ${TABLE_DENSITY[(tokens.componentStyle || {}).tableDensity] ?? 2.5};`);

  /*
   * Motion, in a static stylesheet.
   *
   * `figma-spec.js` REFUSES to draw motion, because a still frame cannot show easing -
   * that refusal is right and stands. But CSS transitions are not a still frame, and the
   * built page had no transition at all, so both motion fields were decorative.
   *
   * `none` emits 0s rather than omitting the property: a reader who chose "none" has made
   * a decision, and a missing declaration is indistinguishable from a field nobody wired.
   */
  const motion = tokens.motion || {};
  lines.push(`  --motion-duration: ${MOTION_DURATION[motion.intensity] ?? '160ms'};`);
  lines.push(`  --motion-ease: ${MOTION_EASE[motion.transition] ?? 'ease'};`);
  lines.push(`  --motion-distance: ${MOTION_DISTANCE[motion.transition] ?? '0px'};`);
  lines.push(`  --motion-scale: ${motion.transition === 'scale' ? '0.98' : '1'};`);

  /*
   * WIDTHS. The last magic numbers in the stylesheet.
   *
   * There were sixteen distinct px declarations, and the README claimed there were none
   * outside a --space multiplier while offering only a grep for HEX as its check. They were
   * not sixteen decisions, though - they were five ROLES with drifted values: a page frame
   * (1100 in eight places), long prose (800), a form column (720, and 980 split), a reading
   * measure (640 in three), and a narrow centred panel written 480, 460 and 420 in three
   * places for the same job. Three values within 60px of each other for one role is exactly
   * the arbitrariness a token exists to remove, so they collapse to one.
   *
   * The MEASURES scale with --type-scale, not with --space. A reading measure is a count of
   * characters per line, so when the type gets bigger the column has to get wider to hold
   * the same line length. Folding them into --space instead would make a compact page have
   * BOTH smaller text and a narrower column, which compounds rather than compensates.
   *
   * The CONTAINER and the RAILS do not scale: they are layout, not text. A navigation rail
   * is sized by its longest label and an inspector by its content, and neither gets easier
   * to read because the body copy grew.
   */
  const type = TYPE_SCALE[scale.type] ?? 1;
  const measure = (px) => `${Math.round(px * type)}px`;
  lines.push(`  --container: 1100px;`);
  lines.push(`  --measure-wide: ${measure(800)};`);
  lines.push(`  --measure-form: ${measure(720)};`);
  lines.push(`  --measure: ${measure(640)};`);
  lines.push(`  --measure-narrow: ${measure(460)};`);
  lines.push(`  --rail: 240px;`);
  lines.push(`  --pane-list: 320px;`);
  lines.push(`  --pane-inspector: 260px;`);

  // The type ramp, scaled by --type-scale so the whole system moves together, and the four
  // role leadings. Every font-size in STRUCTURE_CSS reads one of these, and every one is
  // paired with a leading - tests/render.test.js fails on a size that carries neither.
  for (const [name, px] of Object.entries(TYPE_STEPS)) {
    lines.push(`  --text-${name}: ${(px * type / 16).toFixed(4)}rem;`);
  }
  for (const [role, ratio] of Object.entries(TYPE_LEADING)) {
    lines.push(`  --lh-${role}: ${ratio};`);
  }

  lines.push('}');
  return lines.join('\n');
}

// Fixed structural stylesheet. Every value is a var(--token), a multiple of --space, or a
// sentinel argued for by name in tests/render.test.js. No hex, no font name, no magic width.
//
// That claim used to be broader than its evidence, which is the more interesting failure.
// It said "no px literal outside the --space multiplier" and then offered `grep -E
// "#[0-9a-f]{3,6}"` as the check - a grep for HEX standing in for a claim about px. Sixteen
// distinct px declarations lived under it. They were not sixteen decisions either: they were
// five ROLES with drifted values, including one narrow-panel width written 480, 460 and 420
// in three places for the same job.
//
// `tests/render.test.js` now checks the whole claim: zero hex, zero font names outside a
// token, and zero px outside the sentinel list. A rule and its check are the same size, or
// the rule is decoration.
const STRUCTURE_CSS = `
/* Motion. The page had none at all, so both motion fields were decorative controls.
   Intensity "none" resolves to 0s rather than omitting the property, because a reader who
   chose none made a decision and a missing declaration is indistinguishable from a field
   nobody wired. prefers-reduced-motion WINS over an expressive setting: a design choice
   does not get to override an accessibility preference stated at the OS level. */
a, button, .card, .seg__item, .facet__chip, .otable__item {
  transition: background-color var(--motion-duration) var(--motion-ease),
              color var(--motion-duration) var(--motion-ease),
              transform var(--motion-duration) var(--motion-ease),
              opacity var(--motion-duration) var(--motion-ease);
}
a:hover, button:hover, .card:hover {
  transform: translateY(calc(var(--motion-distance) * -1)) scale(var(--motion-scale));
}
@media (prefers-reduced-motion: reduce) {
  * { transition-duration: 0s !important; }
  a:hover, button:hover, .card:hover { transform: none; }
}

* { box-sizing: border-box; }
body {
  font-family: var(--font-body);
  margin: 0;
  background: var(--color-bg);
  color: var(--color-ink);
  font-family: var(--font-family);
  line-height: var(--lh-body);
}
a { color: var(--color-accent); }

.hero { padding: calc(var(--space) * 24) calc(var(--space) * 8); }
.hero--centered { text-align: center; }
.hero--centered .hero__inner { max-width: var(--measure); margin: 0 auto; }
.hero--split {
  display: grid;
  grid-template-columns: 1fr 1fr;
  align-items: center;
  gap: calc(var(--space) * 12);
  max-width: var(--container);
  margin: 0 auto;
}
.hero__h1 { font-size: var(--text-6xl); line-height: var(--lh-display); margin: 0 0 calc(var(--space) * 4); }
.hero__sub { line-height: var(--lh-body); font-size: var(--text-lg); color: var(--color-muted); margin: 0 0 calc(var(--space) * 6); }
.hero__cta {
  display: inline-block;
  background: var(--color-accent);
  color: var(--color-accentInk);
  text-decoration: none;
  padding: calc(var(--space) * 3) calc(var(--space) * 6);
  border-radius: var(--button-radius);
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
  max-width: var(--container);
  margin: 0 auto;
  flex-wrap: wrap;
}
.footer__mark { font-weight: 700; }
.footer__nav { display: flex; gap: calc(var(--space) * 5); }
.footer__link { color: var(--color-muted); text-decoration: none; line-height: var(--lh-label); font-size: var(--text-sm); }
.footer__link:hover { color: var(--color-accent); }
.footer__copyright { color: var(--color-muted); line-height: var(--lh-label); font-size: var(--text-sm); }

.footer--columns .footer__grid {
  display: grid;
  grid-template-columns: 1.4fr repeat(3, 1fr);
  gap: calc(var(--space) * 8);
  max-width: var(--container);
  margin: 0 auto calc(var(--space) * 8);
}
.footer__tagline { color: var(--color-muted); line-height: var(--lh-label); font-size: var(--text-sm); margin-top: calc(var(--space) * 2); }
.footer__colTitle { line-height: var(--lh-label); font-size: var(--text-xs); text-transform: uppercase; letter-spacing: 0.04em; color: var(--color-muted); margin: 0 0 calc(var(--space) * 3); }
.footer__colLinks { display: flex; flex-direction: column; gap: calc(var(--space) * 2); }
.footer--columns .footer__bottom {
  max-width: var(--container);
  margin: 0 auto;
  padding-top: calc(var(--space) * 6);
  border-top: var(--border-weight) solid var(--color-border);
}

.nav { display: flex; align-items: center; justify-content: space-between; padding: calc(var(--space) * 5) calc(var(--space) * 8); border-bottom: var(--border-weight) solid var(--color-border); }
.nav__mark { font-weight: 700; }
.nav__links, .nav__side { display: flex; gap: calc(var(--space) * 6); align-items: center; }
.nav__link { color: var(--color-ink); text-decoration: none; line-height: var(--lh-label); font-size: var(--text-sm); }
.nav__link:hover { color: var(--color-accent); }
.nav__cta { background: var(--color-accent); color: var(--color-accentInk); text-decoration: none; padding: calc(var(--space) * 2) calc(var(--space) * 4); border-radius: var(--button-radius); line-height: var(--lh-label); font-size: var(--text-sm); font-weight: 600; }
.nav--centered { justify-content: space-between; }
.nav--centered .nav__mark { position: absolute; left: 50%; transform: translateX(-50%); }
.nav--centered { position: relative; }

.pricing { padding: calc(var(--space) * 16) calc(var(--space) * 8); max-width: var(--container); margin: 0 auto; }
.pricing__heading { line-height: var(--lh-heading); font-size: var(--text-3xl); text-align: center; margin: 0 0 calc(var(--space) * 10); }
.pricing__grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: calc(var(--space) * 6); }
.pricing__card { border: var(--border-weight) solid var(--color-border); border-radius: var(--radius-lg); padding: calc(var(--space) * 6); box-shadow: var(--shadow); }
.pricing__card--highlight { border-color: var(--color-accent); border-width: calc(var(--border-weight) * 2); }
.pricing__planName { margin: 0 0 calc(var(--space) * 2); line-height: var(--lh-label); font-size: var(--text-md); }
.pricing__price { line-height: var(--lh-label); font-size: var(--text-3xl); font-weight: 700; margin: 0 0 calc(var(--space) * 4); }
.pricing__features { list-style: none; padding: 0; margin: 0 0 calc(var(--space) * 6); color: var(--color-muted); line-height: var(--lh-body); font-size: var(--text-sm); }
.pricing__features li { padding: calc(var(--space) * 1) 0; }
.pricing__cta { display: block; text-align: center; background: var(--color-accent); color: var(--color-accentInk); text-decoration: none; padding: calc(var(--space) * 3); border-radius: var(--button-radius); font-weight: 600; }
.pricing__matrix { width: 100%; border-collapse: collapse; }
.pricing__matrix th, .pricing__matrix td { border-bottom: var(--border-weight) solid var(--color-border); padding: calc(var(--space) * 3); text-align: left; }

.testimonials { padding: calc(var(--space) * 16) calc(var(--space) * 8); max-width: var(--container); margin: 0 auto; }
.testimonials__heading { line-height: var(--lh-heading); font-size: var(--text-3xl); text-align: center; margin: 0 0 calc(var(--space) * 10); }
.testimonials__gridInner { display: grid; grid-template-columns: repeat(3, 1fr); gap: calc(var(--space) * 6); }
.testimonials__card { border: var(--border-weight) solid var(--color-border); border-radius: var(--radius-lg); padding: calc(var(--space) * 6); margin: 0; box-shadow: var(--shadow); }
.testimonials__card blockquote { margin: 0 0 calc(var(--space) * 4); line-height: var(--lh-body); font-size: var(--text-md); }
.testimonials__card figcaption { color: var(--color-muted); line-height: var(--lh-body); font-size: var(--text-sm); }
.testimonials--featured { text-align: center; }
.testimonials__big { font-size: var(--text-3xl); line-height: var(--lh-heading); margin: 0 0 calc(var(--space) * 6); }
.testimonials__attribution { color: var(--color-muted); }

.features { padding: calc(var(--space) * 16) calc(var(--space) * 8); max-width: var(--container); margin: 0 auto; }
.features__heading { line-height: var(--lh-heading); font-size: var(--text-3xl); text-align: center; margin: 0 0 calc(var(--space) * 10); }
.features__gridInner { display: grid; grid-template-columns: repeat(3, 1fr); gap: calc(var(--space) * 8); }
.features__item h3 { margin: 0 0 calc(var(--space) * 2); line-height: var(--lh-heading); font-size: var(--text-md); }
.features__item p { margin: 0; color: var(--color-muted); line-height: var(--lh-label); font-size: var(--text-sm); }
.features__matrix { width: 100%; border-collapse: collapse; }
.features__matrix th, .features__matrix td { border-bottom: var(--border-weight) solid var(--color-border); padding: calc(var(--space) * 3); text-align: center; }
.features__matrix td:first-child, .features__matrix th:first-child { text-align: left; }

.faq { padding: calc(var(--space) * 16) calc(var(--space) * 8); max-width: var(--measure-wide); margin: 0 auto; }
.faq__heading { line-height: var(--lh-heading); font-size: var(--text-3xl); text-align: center; margin: 0 0 calc(var(--space) * 10); }
.faq__item { border-bottom: var(--border-weight) solid var(--color-border); padding: calc(var(--space) * 4) 0; }
.faq__item summary { cursor: pointer; font-weight: 600; }
.faq__item p { color: var(--color-muted); margin: calc(var(--space) * 3) 0 0; }
.faq--columns .faq__grid { display: grid; grid-template-columns: 1fr 1fr; gap: calc(var(--space) * 6); }
.faq__pair h3 { margin: 0 0 calc(var(--space) * 2); line-height: var(--lh-heading); font-size: var(--text-md); }
.faq__pair p { color: var(--color-muted); margin: 0; line-height: var(--lh-body); font-size: var(--text-sm); }

.cta { padding: calc(var(--space) * 16) calc(var(--space) * 8); text-align: center; background: var(--color-surface); }
.cta h2 { line-height: var(--lh-heading); font-size: var(--text-3xl); margin: 0 0 calc(var(--space) * 3); }
.cta p { color: var(--color-muted); margin: 0 0 calc(var(--space) * 6); }
.cta__button { display: inline-block; background: var(--color-accent); color: var(--color-accentInk); text-decoration: none; padding: calc(var(--space) * 3) calc(var(--space) * 6); border-radius: var(--button-radius); font-weight: 600; border: none; line-height: var(--lh-label); font-size: var(--text-md); cursor: pointer; }
.cta--signup { display: flex; align-items: center; justify-content: space-between; text-align: left; max-width: var(--container); margin: calc(var(--space) * 16) auto; }
.cta--signup .cta__form { display: flex; gap: calc(var(--space) * 3); }
.cta__input { padding: calc(var(--space) * 3); border: var(--border-weight) solid var(--color-border); border-radius: var(--radius-sm); line-height: var(--lh-body); font-size: var(--text-sm); }

.sidebar { width: var(--rail); padding: calc(var(--space) * 6); border-right: var(--border-weight) solid var(--color-border); }
.sidebar__nav, .sidebar__group { display: flex; flex-direction: column; gap: calc(var(--space) * 1); }
.sidebar__link { color: var(--color-muted); text-decoration: none; padding: calc(var(--space) * 2) calc(var(--space) * 3); border-radius: var(--radius-sm); line-height: var(--lh-label); font-size: var(--text-sm); }
.sidebar__link--active { background: var(--color-surface); color: var(--color-ink); font-weight: 600; }
.sidebar__group { margin-bottom: calc(var(--space) * 5); }
.sidebar__groupTitle { line-height: var(--lh-label); font-size: var(--text-xs); text-transform: uppercase; letter-spacing: 0.04em; color: var(--color-muted); margin: 0 0 calc(var(--space) * 2) calc(var(--space) * 3); }

.team { padding: calc(var(--space) * 16) calc(var(--space) * 8); max-width: var(--container); margin: 0 auto; }
.team__heading { line-height: var(--lh-heading); font-size: var(--text-3xl); text-align: center; margin: 0 0 calc(var(--space) * 10); }
.team__gridInner { display: grid; grid-template-columns: repeat(4, 1fr); gap: calc(var(--space) * 6); }
.team__photo { aspect-ratio: 1; background: var(--color-surface); border: var(--border-weight) solid var(--color-border); border-radius: var(--radius-lg); margin-bottom: calc(var(--space) * 3); }
.team__card h3, .team__row h3 { margin: 0; line-height: var(--lh-heading); font-size: var(--text-md); }
.team__card p { margin: 0; color: var(--color-muted); line-height: var(--lh-body); font-size: var(--text-sm); }
.team__listInner { display: flex; flex-direction: column; gap: calc(var(--space) * 6); }
.team__row { display: flex; gap: calc(var(--space) * 5); align-items: center; }
.team__row .team__photo { width: calc(var(--space) * 16); height: calc(var(--space) * 16); flex-shrink: 0; margin: 0; }
.team__row h3 span { display: block; font-weight: 400; color: var(--color-muted); line-height: var(--lh-heading); font-size: var(--text-sm); }
.team__row p { margin: calc(var(--space) * 1) 0 0; color: var(--color-muted); line-height: var(--lh-body); font-size: var(--text-sm); }

.contact { padding: calc(var(--space) * 16) calc(var(--space) * 8); max-width: var(--measure); margin: 0 auto; }
.contact--split { max-width: var(--container); display: grid; grid-template-columns: 1fr 1fr; gap: calc(var(--space) * 12); }
.contact__email { color: var(--color-accent); font-weight: 600; }
.contact__form { display: flex; flex-direction: column; gap: calc(var(--space) * 3); }
.contact__input, .contact__textarea { padding: calc(var(--space) * 3); border: var(--border-weight) solid var(--color-border); border-radius: var(--radius-sm); font-family: inherit; line-height: var(--lh-body); font-size: var(--text-sm); }
.contact__button { background: var(--color-accent); color: var(--color-accentInk); border: none; padding: calc(var(--space) * 3); border-radius: var(--radius-sm); font-weight: 600; cursor: pointer; line-height: var(--lh-label); font-size: var(--text-md); }

/* ---- SaaS app components. Same rule as the marketing blocks: every value is a
   var(--token) or a multiple of --space. No hex, no font name, no bare px. ---- */

.facet { display: flex; align-items: center; gap: calc(var(--space) * 3); flex-wrap: wrap; padding: calc(var(--space) * 4) calc(var(--space) * 6); border-bottom: var(--border-weight) solid var(--color-border); }
.facet__row { display: flex; gap: calc(var(--space) * 2); flex-wrap: wrap; }
.facet__chip { font: inherit; line-height: var(--lh-label); font-size: var(--text-sm); display: inline-flex; align-items: center; gap: calc(var(--space) * 2); padding: calc(var(--space) * 1.5) calc(var(--space) * 3); border-radius: var(--radius-sm); border: var(--border-weight) solid var(--color-border); background: var(--color-bg); color: var(--color-muted); cursor: pointer; }
.facet__chip--on { background: var(--color-accent); border-color: var(--color-accent); color: var(--color-accentInk); font-weight: 600; }
.facet__count { line-height: var(--lh-label); font-size: var(--text-xs); opacity: 0.75; }
.facet__result { line-height: var(--lh-label); font-size: var(--text-xs); color: var(--color-muted); margin-left: auto; }
.facet--grouped { flex-direction: column; align-items: stretch; }
.facet__input { width: 100%; padding: calc(var(--space) * 2) calc(var(--space) * 3); border: var(--border-weight) solid var(--color-border); border-radius: var(--radius-sm); font-family: inherit; line-height: var(--lh-label); font-size: var(--text-sm); background: var(--color-bg); color: var(--color-ink); }
.facet__groups { display: flex; gap: calc(var(--space) * 6); flex-wrap: wrap; }
.facet__group { display: flex; flex-direction: column; gap: calc(var(--space) * 2); }
.facet__groupTitle { line-height: var(--lh-label); font-size: var(--text-xs); text-transform: uppercase; letter-spacing: 0.06em; color: var(--color-muted); }

.badge { display: inline-flex; align-items: center; gap: calc(var(--space) * 1.5); line-height: var(--lh-label); font-size: var(--text-xs); font-weight: 600; white-space: nowrap; }
.badge--pill { padding: calc(var(--space) * 1) calc(var(--space) * 2.5); border-radius: 999px; border: var(--border-weight) solid var(--color-border); background: var(--color-surface); color: var(--color-ink); }
.badge--dot i { width: calc(var(--space) * 2); height: calc(var(--space) * 2); border-radius: 999px; background: var(--color-accent); display: inline-block; }
.badge--danger { color: var(--color-danger); }
.badge--success i { background: var(--color-success); }
.badge--info i { background: var(--color-info); }
.badge--warning i { background: var(--color-muted); }

.otable { padding: calc(var(--space) * 5) calc(var(--space) * 6); }
.otable__head { display: flex; align-items: baseline; gap: calc(var(--space) * 3); margin-bottom: calc(var(--space) * 4); }
.otable__title { line-height: var(--lh-heading); font-size: var(--text-md); margin: 0; }
.otable__meta { line-height: var(--lh-label); font-size: var(--text-xs); color: var(--color-muted); margin-left: auto; }
.otable__grid { width: 100%; border-collapse: collapse; line-height: var(--lh-label); font-size: var(--text-sm); }
.otable__grid th { text-align: left; line-height: var(--lh-label); font-size: var(--text-xs); text-transform: uppercase; letter-spacing: 0.06em; color: var(--color-muted); font-weight: 600; padding: calc(var(--space) * 2) calc(var(--space) * 3); border-bottom: var(--border-weight) solid var(--color-border); }
.otable__grid td { padding: calc(var(--space) * 2.5) calc(var(--space) * 3); border-bottom: var(--border-weight) solid var(--color-border); color: var(--color-muted); }
.otable__key { color: var(--color-ink); font-weight: 600; }
.otable__act a { line-height: var(--lh-label); font-size: var(--text-xs); }
.otable--list { padding: calc(var(--space) * 4); }
.otable__items { display: flex; flex-direction: column; }
.otable__item { display: flex; flex-direction: column; gap: calc(var(--space) * 1); padding: calc(var(--space) * var(--row-pad)); border-bottom: var(--border-weight) solid var(--color-border); text-decoration: none; border-radius: var(--radius-sm); }
.otable__item--on { background: var(--color-surface); }
.otable__itemKey { line-height: var(--lh-label); font-size: var(--text-sm); font-weight: 600; color: var(--color-ink); }
.otable__itemSub { line-height: var(--lh-label); font-size: var(--text-xs); color: var(--color-muted); }

.oview { padding: calc(var(--space) * 6); border-bottom: var(--border-weight) solid var(--color-border); }
.oview__head { display: flex; align-items: flex-start; gap: calc(var(--space) * 6); }
.oview__kind { line-height: var(--lh-label); font-size: var(--text-xs); text-transform: uppercase; letter-spacing: 0.08em; color: var(--color-muted); }
.oview__title { line-height: var(--lh-heading); font-size: var(--text-2xl); margin: calc(var(--space) * 1) 0 0; }
.oview__sub { line-height: var(--lh-label); font-size: var(--text-sm); color: var(--color-muted); margin: calc(var(--space) * 1) 0 0; }
.oview__actions { margin-left: auto; display: flex; gap: calc(var(--space) * 2); align-items: flex-start; flex-wrap: wrap; }
.oview__action { display: inline-block; background: var(--color-accent); color: var(--color-accentInk); text-decoration: none; padding: calc(var(--space) * 2) calc(var(--space) * 4); border-radius: var(--radius-sm); line-height: var(--lh-label); font-size: var(--text-sm); font-weight: 600; }
.oview__action--blocked { background: var(--color-surface); color: var(--color-muted); border: var(--border-weight) solid var(--color-border); display: inline-flex; flex-direction: column; gap: calc(var(--space) * 0.5); }
.oview__blockedWhy { line-height: var(--lh-label); font-size: var(--text-xs); font-weight: 400; }
.oview__facts { display: flex; gap: calc(var(--space) * 8); flex-wrap: wrap; margin: calc(var(--space) * 5) 0 0; }
.oview__fact dt { line-height: var(--lh-label); font-size: var(--text-xs); text-transform: uppercase; letter-spacing: 0.06em; color: var(--color-muted); margin: 0; }
.oview__fact dd { line-height: var(--lh-label); font-size: var(--text-sm); margin: calc(var(--space) * 1) 0 0; }
.oview__tabs { display: flex; gap: calc(var(--space) * 5); margin-top: calc(var(--space) * 5); border-bottom: var(--border-weight) solid var(--color-border); }
.oview__tab { line-height: var(--lh-label); font-size: var(--text-sm); color: var(--color-muted); padding-bottom: calc(var(--space) * 2); display: inline-flex; gap: calc(var(--space) * 1.5); }
.oview__tab--on { color: var(--color-ink); font-weight: 600; box-shadow: inset 0 calc(var(--border-weight) * -2) 0 var(--color-accent); }
.oview__tabCount { line-height: var(--lh-label); font-size: var(--text-xs); color: var(--color-muted); }

.insp { border-left: var(--border-weight) solid var(--color-border); padding: calc(var(--space) * 4); }
.insp__head { line-height: var(--lh-label); font-size: var(--text-xs); text-transform: uppercase; letter-spacing: 0.08em; color: var(--color-muted); margin-bottom: calc(var(--space) * 4); }
.insp__body { display: flex; flex-direction: column; gap: calc(var(--space) * 5); }
.insp__group { display: flex; flex-direction: column; gap: calc(var(--space) * 2); }
.insp__groupTitle { line-height: var(--lh-label); font-size: var(--text-xs); text-transform: uppercase; letter-spacing: 0.06em; color: var(--color-muted); }
.insp__row { display: flex; justify-content: space-between; gap: calc(var(--space) * 3); line-height: var(--lh-label); font-size: var(--text-xs); }
.insp__label { color: var(--color-muted); }
.insp__value { color: var(--color-ink); text-align: right; }
.insp__events { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: calc(var(--space) * 4); }
.insp__event { display: flex; flex-direction: column; gap: calc(var(--space) * 0.5); line-height: var(--lh-label); font-size: var(--text-xs); border-left: var(--border-weight) solid var(--color-border); padding-left: calc(var(--space) * 3); }
.insp__when { line-height: var(--lh-label); font-size: var(--text-xs); color: var(--color-muted); font-family: var(--font-mono); }
.insp__what { color: var(--color-ink); }
.insp__who { line-height: var(--lh-label); font-size: var(--text-xs); color: var(--color-muted); }

/* ---- Forms, empty/page states, destructive confirm. Token-only, as above. ---- */

.fieldset { padding: calc(var(--space) * 8) calc(var(--space) * 6); max-width: var(--measure-form); }
.fieldset--split { max-width: calc(var(--measure-form) * 1.35); }
.fieldset__head { margin-bottom: calc(var(--space) * 6); }
.fieldset__title { line-height: var(--lh-heading); font-size: var(--text-xl); margin: 0; }
.fieldset__sub { line-height: var(--lh-body); font-size: var(--text-sm); color: var(--color-muted); margin: calc(var(--space) * 2) 0 0; }
.fieldset__form { display: flex; flex-direction: column; gap: calc(var(--space) * 5); }
.fieldset__form--grid { display: grid; grid-template-columns: 1fr 1fr; gap: calc(var(--space) * 5) calc(var(--space) * 6); }
.field { display: flex; flex-direction: column; gap: calc(var(--space) * 2); min-width: 0; }
.field__label { line-height: var(--lh-label); font-size: var(--text-xs); font-weight: 600; color: var(--color-ink); }
.field__req { color: var(--color-accent); margin-left: calc(var(--space) * 1); }
.field__input { width: 100%; padding: calc(var(--space) * 2.5) calc(var(--space) * 3); border: var(--border-weight) solid var(--color-border); border-radius: var(--radius-sm); background: var(--color-bg); color: var(--color-ink); font-family: inherit; line-height: var(--lh-body); font-size: var(--text-sm); }
.field__input--area { resize: vertical; min-height: calc(var(--space) * 20); }
.field__check { width: calc(var(--space) * 4); height: calc(var(--space) * 4); accent-color: var(--color-accent); }
.field__note { line-height: var(--lh-label); font-size: var(--text-xs); color: var(--color-muted); }
.field__note--error { color: var(--color-danger); font-weight: 600; }
.field--invalid .field__input { border-color: var(--color-danger); border-width: calc(var(--border-weight) * 2); }
.fieldset__actions { display: flex; align-items: center; gap: calc(var(--space) * 4); grid-column: 1 / -1; margin-top: calc(var(--space) * 2); }
.fieldset__submit { background: var(--color-accent); color: var(--color-accentInk); border: none; padding: calc(var(--space) * 3) calc(var(--space) * 6); border-radius: var(--radius-sm); font: inherit; line-height: var(--lh-label); font-size: var(--text-sm); font-weight: 600; cursor: pointer; }
.fieldset__cancel { line-height: var(--lh-label); font-size: var(--text-sm); color: var(--color-muted); text-decoration: none; }

.empty { padding: calc(var(--space) * 16) calc(var(--space) * 8); text-align: center; max-width: var(--measure-narrow); margin: 0 auto; display: flex; flex-direction: column; align-items: center; gap: calc(var(--space) * 3); }
.empty__mark { width: calc(var(--space) * 16); height: calc(var(--space) * 16); border: var(--border-weight) solid var(--color-border); border-radius: var(--radius-lg); background: var(--color-surface); }
.empty__title { line-height: var(--lh-heading); font-size: var(--text-lg); margin: calc(var(--space) * 2) 0 0; }
.empty__body { line-height: var(--lh-body); font-size: var(--text-sm); color: var(--color-muted); margin: 0; }
.empty__query { font-family: var(--font-mono); line-height: var(--lh-label); font-size: var(--text-xs); color: var(--color-muted); background: var(--color-surface); padding: calc(var(--space) * 2) calc(var(--space) * 3); border-radius: var(--radius-sm); margin: 0; }
.empty__cta { display: inline-block; background: var(--color-accent); color: var(--color-accentInk); text-decoration: none; padding: calc(var(--space) * 2.5) calc(var(--space) * 5); border-radius: var(--button-radius); line-height: var(--lh-label); font-size: var(--text-sm); font-weight: 600; margin-top: calc(var(--space) * 2); }
.empty__cta--quiet { background: transparent; color: var(--color-accent); border: var(--border-weight) solid var(--color-border); }
.empty__secondary { line-height: var(--lh-label); font-size: var(--text-xs); color: var(--color-muted); text-decoration: none; }

.pstate { padding: calc(var(--space) * 10) calc(var(--space) * 6); max-width: var(--measure); }
.pstate__label { line-height: var(--lh-label); font-size: var(--text-xs); text-transform: uppercase; letter-spacing: 0.06em; color: var(--color-muted); }
.pstate__skeleton { display: flex; flex-direction: column; gap: calc(var(--space) * 3); margin-top: calc(var(--space) * 4); }
.pstate__bar { height: calc(var(--space) * 3); background: var(--color-surface); border-radius: var(--radius-sm); }
.pstate--error { text-align: left; }
.pstate__code { font-family: var(--font-mono); line-height: var(--lh-body); font-size: var(--text-xs); color: var(--color-muted); }
.pstate__title { line-height: var(--lh-heading); font-size: var(--text-lg); margin: calc(var(--space) * 2) 0 0; }
.pstate__body { line-height: var(--lh-body); font-size: var(--text-sm); color: var(--color-muted); margin: calc(var(--space) * 2) 0 0; }
.pstate__actions { display: flex; gap: calc(var(--space) * 4); align-items: center; margin-top: calc(var(--space) * 5); }
.pstate__retry { background: var(--color-accent); color: var(--color-accentInk); text-decoration: none; padding: calc(var(--space) * 2.5) calc(var(--space) * 5); border-radius: var(--button-radius); line-height: var(--lh-label); font-size: var(--text-sm); font-weight: 600; }
.pstate__help { line-height: var(--lh-body); font-size: var(--text-sm); color: var(--color-muted); text-decoration: none; }
.pstate__ref { font-family: var(--font-mono); line-height: var(--lh-label); font-size: var(--text-xs); color: var(--color-muted); margin-top: calc(var(--space) * 4); }

.toast { display: inline-flex; align-items: center; gap: calc(var(--space) * 3); background: var(--color-ink); color: var(--color-bg); padding: calc(var(--space) * 3) calc(var(--space) * 4); border-radius: var(--radius-sm); box-shadow: var(--shadow); max-width: var(--measure-narrow); }
.toast__mark { width: calc(var(--space) * 2); height: calc(var(--space) * 2); border-radius: 999px; background: var(--color-accent); flex: none; }
.toast__body { margin: 0; line-height: var(--lh-body); font-size: var(--text-sm); }
.toast__dismiss, .toast__undo { background: none; border: none; color: inherit; font: inherit; line-height: var(--lh-label); font-size: var(--text-sm); font-weight: 600; text-decoration: underline; cursor: pointer; padding: 0; margin-left: auto; }
.toast__window { line-height: var(--lh-label); font-size: var(--text-xs); opacity: 0.7; font-family: var(--font-mono); }

.seg { display: inline-block; }
.seg__track { display: inline-flex; gap: calc(var(--space) * 0.5); background: var(--color-surface); border: var(--border-weight) solid var(--color-border); border-radius: var(--radius-sm); padding: calc(var(--space) * 0.5); }
.seg__item { display: inline-flex; align-items: center; gap: calc(var(--space) * 1.5); padding: calc(var(--space) * 2) calc(var(--space) * 3.5); border: none; background: none; font: inherit; line-height: var(--lh-label); font-size: var(--text-sm); color: var(--color-muted); text-decoration: none; border-radius: var(--radius-sm); cursor: pointer; }
.seg__item--on { background: var(--color-bg); color: var(--color-ink); font-weight: 600; box-shadow: var(--shadow); }
.seg__count { font-family: var(--font-mono); line-height: var(--lh-label); font-size: var(--text-xs); color: var(--color-muted); }

.card { display: block; background: var(--color-surface); border: var(--border-weight) solid var(--color-border); border-radius: var(--radius-lg); padding: calc(var(--space) * 5); text-decoration: none; color: inherit; box-shadow: var(--shadow); }
.card__badge { display: inline-block; line-height: var(--lh-label); font-size: var(--text-xs); text-transform: uppercase; letter-spacing: 0.08em; color: var(--color-accent); font-weight: 600; margin-bottom: calc(var(--space) * 2); }
.card__title { line-height: var(--lh-heading); font-size: var(--text-md); margin: 0; }
.card__body { line-height: var(--lh-body); font-size: var(--text-sm); color: var(--color-muted); margin: calc(var(--space) * 2) 0 0; }
.card__metas { display: flex; flex-wrap: wrap; gap: calc(var(--space) * 3); margin-top: calc(var(--space) * 4); }
.card__meta { line-height: var(--lh-label); font-size: var(--text-xs); color: var(--color-muted); font-family: var(--font-mono); }
.card--metric { display: block; }
.card__label { line-height: var(--lh-label); font-size: var(--text-xs); text-transform: uppercase; letter-spacing: 0.08em; color: var(--color-muted); margin: 0; }
.card__figure { font-size: var(--text-4xl); font-weight: 600; margin: calc(var(--space) * 2) 0 0; line-height: var(--lh-display); }
.card__change { line-height: var(--lh-label); font-size: var(--text-sm); color: var(--color-ink); margin: calc(var(--space) * 2) 0 0; }
.card__asof { line-height: var(--lh-label); font-size: var(--text-xs); color: var(--color-muted); font-family: var(--font-mono); margin: calc(var(--space) * 3) 0 0; }

.cdialog { padding: calc(var(--space) * 8); display: flex; justify-content: center; background: var(--color-surface); }
.cdialog__panel { background: var(--color-bg); border: var(--border-weight) solid var(--color-border); border-radius: var(--radius-lg); box-shadow: var(--shadow); padding: calc(var(--space) * 6); max-width: var(--measure-narrow); width: 100%; }
.cdialog__title { line-height: var(--lh-heading); font-size: var(--text-md); margin: 0; }
.cdialog__body { line-height: var(--lh-body); font-size: var(--text-sm); color: var(--color-muted); margin: calc(var(--space) * 3) 0 0; }
.cdialog__consequence { line-height: var(--lh-body); font-size: var(--text-xs); color: var(--color-danger); font-weight: 600; margin: calc(var(--space) * 3) 0 0; }
.cdialog__form { margin-top: calc(var(--space) * 4); display: flex; flex-direction: column; gap: calc(var(--space) * 2); }
.cdialog__label { line-height: var(--lh-label); font-size: var(--text-xs); color: var(--color-muted); }
.cdialog__input { padding: calc(var(--space) * 2.5) calc(var(--space) * 3); border: var(--border-weight) solid var(--color-border); border-radius: var(--radius-sm); font-family: var(--font-mono); line-height: var(--lh-body); font-size: var(--text-sm); background: var(--color-bg); color: var(--color-ink); }
.cdialog__actions { display: flex; justify-content: flex-end; gap: calc(var(--space) * 4); align-items: center; margin-top: calc(var(--space) * 5); }
.cdialog__cancel { line-height: var(--lh-label); font-size: var(--text-sm); color: var(--color-muted); text-decoration: none; }
.cdialog__confirm { background: var(--color-danger); color: var(--color-dangerInk); border: none; text-decoration: none; padding: calc(var(--space) * 2.5) calc(var(--space) * 5); border-radius: var(--button-radius); font: inherit; line-height: var(--lh-label); font-size: var(--text-sm); font-weight: 600; cursor: pointer; }

/* The app shell: sidebar as a rail beside the content, not a band above it. */
.shell { display: grid; grid-template-columns: auto minmax(0, 1fr); align-items: start; min-height: 0; }
.shell__main { min-width: 0; }
.shell .sidebar { height: 100%; }

.md__panes { display: grid; align-items: start; }
.md--two .md__panes { grid-template-columns: minmax(0, var(--pane-list)) minmax(0, 1fr); }
.md--three .md__panes { grid-template-columns: minmax(0, calc(var(--pane-list) - var(--space) * 5)) minmax(0, 1fr) minmax(0, var(--pane-inspector)); }
.md__master { border-right: var(--border-weight) solid var(--color-border); }
.md__detail { min-width: 0; }

.notfound { padding: calc(var(--space) * 24) calc(var(--space) * 8); text-align: center; max-width: var(--measure-narrow); margin: 0 auto; }
.notfound__code { color: var(--color-muted); font-family: var(--font-mono); margin: 0 0 calc(var(--space) * 3); }
.notfound h1 { line-height: var(--lh-heading); font-size: var(--text-3xl); margin: 0 0 calc(var(--space) * 3); }
.notfound p { color: var(--color-muted); margin: 0 0 calc(var(--space) * 6); }
.notfound__link { color: var(--color-accent); font-weight: 600; text-decoration: none; }
.notfound__searchForm { margin: 0 0 calc(var(--space) * 6); }
.notfound__searchInput { width: 100%; padding: calc(var(--space) * 3); border: var(--border-weight) solid var(--color-border); border-radius: var(--radius-sm); line-height: var(--lh-body); font-size: var(--text-sm); }

/* ---- The control layer, measured out of Uber Base and drawn on the
   \`Base (redrawn)\` Figma page. These are controls rather than page blocks, and they
   live in the same flat registry as \`toast\` and \`segmentedcontrol\` already did:
   the bank is one registry of renderable units, not two. ---- */

/* Switch. The track is a fixed 2:1 so the thumb has exactly its own width to travel;
   expressing it as space multiples rather than a px pair keeps it on the density unit. */
.switch { display: flex; align-items: center; gap: calc(var(--space) * 3); }
.switch__track { width: calc(var(--space) * 11); height: calc(var(--space) * 6); border-radius: 999px; border: var(--border-weight) solid var(--color-border); background: var(--color-border); padding: 0; display: flex; align-items: center; cursor: pointer; }
.switch__track--on { background: var(--color-accent); border-color: var(--color-accent); justify-content: flex-end; }
.switch__thumb { width: calc(var(--space) * 5); height: calc(var(--space) * 5); border-radius: 999px; background: var(--color-bg); box-shadow: var(--shadow); margin: 0 calc(var(--space) * 0.5); }
.switch__label { line-height: var(--lh-label); font-size: var(--text-sm); color: var(--color-ink); cursor: pointer; }
.switch__group { display: flex; flex-direction: column; gap: calc(var(--space) * 4); border: 0; padding: 0; margin: 0; max-width: var(--measure-form); }
.switch__setting { display: flex; align-items: flex-start; justify-content: space-between; gap: calc(var(--space) * 6); }
.switch__text { display: flex; flex-direction: column; gap: calc(var(--space) * 1); }
.switch__desc { line-height: var(--lh-body); font-size: var(--text-xs); color: var(--color-muted); margin: 0; max-width: var(--measure); }

/* Check. The mixed state gets its own rule rather than borrowing the checked one,
   because "some of these" and "all of these" must not look the same. */
.check { display: flex; align-items: center; gap: calc(var(--space) * 3); }
.check__box { width: calc(var(--space) * 5); height: calc(var(--space) * 5); border-radius: var(--radius-sm); border: var(--border-weight) solid var(--color-border); background: var(--color-bg); display: flex; align-items: center; justify-content: center; padding: 0; cursor: pointer; }
.check__box--on { background: var(--color-accent); border-color: var(--color-accent); }
.check__box--mixed { background: var(--color-accent); border-color: var(--color-accent); }
.check__box--off .check__mark { visibility: hidden; }
.check__mark { line-height: var(--lh-label); font-size: var(--text-sm); color: var(--color-accentInk); }
.check__label { line-height: var(--lh-label); font-size: var(--text-sm); color: var(--color-ink); cursor: pointer; }
.check__group { border: 0; padding: 0; margin: 0; display: flex; flex-direction: column; gap: calc(var(--space) * 3); max-width: var(--measure); }
.check__legend { line-height: var(--lh-label); font-size: var(--text-xs); font-weight: 600; color: var(--color-ink); padding: 0; }
.check--parent { padding-bottom: calc(var(--space) * 2); border-bottom: var(--border-weight) solid var(--color-border); }
.check__count { color: var(--color-muted); font-weight: 400; }
.check__children { display: flex; flex-direction: column; gap: calc(var(--space) * 3); padding-left: calc(var(--space) * 8); }

/* Radio. The pip is a child rather than a background so the ring stays visible
   underneath it: a filled circle and a ringed circle with a dot read differently. */
.radio { border: 0; padding: 0; margin: 0; display: flex; flex-direction: column; gap: calc(var(--space) * 3); max-width: var(--measure); }
.radio__legend { line-height: var(--lh-label); font-size: var(--text-xs); font-weight: 600; color: var(--color-ink); padding: 0; }
.radio__option { display: flex; align-items: center; gap: calc(var(--space) * 3); }
.radio__dot { width: calc(var(--space) * 5); height: calc(var(--space) * 5); border-radius: 999px; border: var(--border-weight) solid var(--color-border); background: var(--color-bg); display: flex; align-items: center; justify-content: center; padding: 0; cursor: pointer; flex-shrink: 0; }
.radio__dot--on { border-color: var(--color-accent); }
.radio__dot--on .radio__pip { background: var(--color-accent); }
.radio__pip { width: calc(var(--space) * 2.5); height: calc(var(--space) * 2.5); border-radius: 999px; }
.radio__label { line-height: var(--lh-label); font-size: var(--text-sm); color: var(--color-ink); cursor: pointer; }
.radio--cards { gap: calc(var(--space) * 2); }
.radio__card { display: flex; align-items: flex-start; gap: calc(var(--space) * 3); padding: calc(var(--space) * 4); border: var(--border-weight) solid var(--color-border); border-radius: var(--radius-sm); cursor: pointer; }
.radio__card--on { border-color: var(--color-accent); background: var(--color-surface); }
.radio__cardText { display: flex; flex-direction: column; gap: calc(var(--space) * 1); }
.radio__cardTitle { line-height: var(--lh-label); font-size: var(--text-sm); font-weight: 600; color: var(--color-ink); }
.radio__cardDesc { line-height: var(--lh-body); font-size: var(--text-xs); color: var(--color-muted); }

/* Tooltip. Rendered open, because a static page cannot hover and a system that only
   shows the resting state has not shown the component. */
.tip { display: inline-flex; align-items: center; gap: calc(var(--space) * 2); position: relative; }
.tip__target { width: calc(var(--space) * 8); height: calc(var(--space) * 8); border-radius: 999px; border: var(--border-weight) solid var(--color-border); background: var(--color-bg); display: inline-flex; align-items: center; justify-content: center; cursor: help; padding: 0; }
.tip__target--text { width: auto; height: auto; border-radius: var(--radius-sm); padding: calc(var(--space) * 2) calc(var(--space) * 3); line-height: var(--lh-label); font-size: var(--text-sm); color: var(--color-ink); }
.tip__glyph { line-height: var(--lh-label); font-size: var(--text-xs); font-weight: 600; color: var(--color-muted); }
.tip__bubble { background: var(--color-ink); color: var(--color-bg); padding: calc(var(--space) * 2) calc(var(--space) * 3); border-radius: var(--radius-sm); line-height: var(--lh-label); font-size: var(--text-xs); box-shadow: var(--shadow); max-width: var(--measure-narrow); }
.tip--rich { flex-direction: column; align-items: flex-start; gap: calc(var(--space) * 1); }
.tip__bubble--rich { display: flex; flex-direction: column; gap: calc(var(--space) * 1); padding: calc(var(--space) * 3); }
.tip__title { line-height: var(--lh-label); font-size: var(--text-xs); font-weight: 600; }
.tip__body { line-height: var(--lh-body); font-size: var(--text-xs); }

/* Notification badge. The screen-reader span is the component, not an accessory: a
   coloured dot with no text says nothing at all to a reader that cannot see it. */
.nbadge { display: inline-flex; align-items: center; gap: calc(var(--space) * 2); line-height: var(--lh-label); font-size: var(--text-sm); color: var(--color-ink); }
.nbadge__dot { width: calc(var(--space) * 2); height: calc(var(--space) * 2); border-radius: 999px; background: var(--color-danger); flex-shrink: 0; }
.nbadge__count { min-width: calc(var(--space) * 5); height: calc(var(--space) * 5); padding: 0 calc(var(--space) * 1.5); border-radius: 999px; background: var(--color-danger); color: var(--color-dangerInk); line-height: var(--lh-label); font-size: var(--text-xs); font-weight: 600; display: inline-flex; align-items: center; justify-content: center; }
.nbadge__sr { position: absolute; width: var(--border-weight); height: var(--border-weight); overflow: hidden; clip-path: inset(50%); white-space: nowrap; }

/* Divider. The inset is the whole component: full-bleed separates SECTIONS, inset
   separates ROWS of the same kind. */
.rule { border: 0; border-top: var(--border-weight) solid var(--color-border); margin: calc(var(--space) * 6) 0; }
.rule--inset { margin-left: calc(var(--space) * 4); margin-right: 0; }
.rule__labelled { display: flex; align-items: center; gap: calc(var(--space) * 4); margin: calc(var(--space) * 6) 0; }
.rule__line { flex: 1; height: var(--border-weight); background: var(--color-border); }
.rule__label { line-height: var(--lh-label); font-size: var(--text-xs); font-weight: 600; color: var(--color-muted); text-transform: uppercase; letter-spacing: 0.06em; }

/* Pagination. Measured from Base: a 48 outer cell with 6 padding round a 36 chip at
   radius 8, so the hit area stays 48 square whether or not the chip is filled. */
.pag { display: flex; flex-direction: column; gap: calc(var(--space) * 2); }
.pag__row { display: flex; list-style: none; margin: 0; padding: 0; }
.pag__cell { width: calc(var(--space) * 12); height: calc(var(--space) * 12); display: flex; align-items: center; justify-content: center; }
.pag__chip { width: calc(var(--space) * 9); height: calc(var(--space) * 9); border-radius: var(--radius-sm); display: flex; align-items: center; justify-content: center; text-decoration: none; line-height: var(--lh-label); font-size: var(--text-sm); color: var(--color-ink); border: 0; background: none; cursor: pointer; }
.pag__chip:hover { background: var(--color-surface); }
.pag__chip--on { background: var(--color-ink); color: var(--color-bg); font-weight: 600; }
.pag__chip--off { color: var(--color-muted); cursor: default; }
.pag__chip--gap { color: var(--color-muted); }
.pag__status { line-height: var(--lh-label); font-size: var(--text-xs); color: var(--color-muted); margin: 0; }
.pag--cursor .pag__cell { width: auto; }
.pag--cursor .pag__chip { width: auto; padding: 0 calc(var(--space) * 3); }
.pag__sr { position: absolute; width: var(--border-weight); height: var(--border-weight); overflow: hidden; clip-path: inset(50%); white-space: nowrap; }

/* Page controls. The dot sizes are inline because each is derived from that dot's
   distance from the current page; a class per size could not express the relationship. */
.pctl { display: flex; flex-direction: column; align-items: center; gap: calc(var(--space) * 2); }
.pctl__row { display: flex; align-items: center; gap: calc(var(--space) * 2); padding: calc(var(--space) * 1) calc(var(--space) * 3); }
.pctl__dot { border-radius: 999px; border: 0; background: var(--color-border); padding: 0; cursor: pointer; }
.pctl__dot--on { background: var(--color-ink); }
.pctl__count { line-height: var(--lh-label); font-size: var(--text-xs); color: var(--color-muted); margin: 0; }
.pctl__sr { position: absolute; width: var(--border-weight); height: var(--border-weight); overflow: hidden; clip-path: inset(50%); white-space: nowrap; }

/* Menu. Measured from Base: radius 12 on the container, rows flush with no gap, and the
   two sizes differing only in the row's vertical padding. */
.menu { background: var(--color-surface); border: var(--border-weight) solid var(--color-border); border-radius: var(--radius-lg); box-shadow: var(--shadow); overflow: hidden; width: var(--pane-inspector); }
.menu__list, .menu__groups { list-style: none; margin: 0; padding: 0; }
.menu__item { width: 100%; display: flex; align-items: center; justify-content: space-between; gap: calc(var(--space) * 4); border: 0; background: none; cursor: pointer; text-align: left; line-height: var(--lh-label); font-size: var(--text-sm); color: var(--color-ink); }
/* The two sizes differ ONLY in the row's vertical padding: measured 8/20/8 for a 36 row
   and 12/24/12 for 48. Small is written out rather than baked into .menu__item, so
   neither size is the silent default and both say what they are. */
.menu__item--small { padding: calc(var(--space) * 2) calc(var(--space) * 4); }
.menu__item--medium { padding: calc(var(--space) * 3) calc(var(--space) * 4); }
.menu__item:hover { background: var(--color-bg); }
.menu__item--off { color: var(--color-muted); cursor: default; }
.menu__item--destructive { color: var(--color-danger); }
.menu__label { flex: 1; }
.menu__shortcut { color: var(--color-muted); line-height: var(--lh-label); font-size: var(--text-xs); }
.menu__groupTitle { line-height: var(--lh-label); font-size: var(--text-xs); font-weight: 600; color: var(--color-muted); text-transform: uppercase; letter-spacing: 0.06em; margin: 0; padding: calc(var(--space) * 2) calc(var(--space) * 4) calc(var(--space) * 1); }
.menu__group + .menu__group { border-top: var(--border-weight) solid var(--color-border); }
/* The destructive group is separated MORE than groups are from each other. A Delete flush
   against a Duplicate is a Delete one slip away from being pressed, and the block's own
   comment promised this separation while the stylesheet did not deliver it. */
.menu__group--danger { margin-top: calc(var(--space) * 2); }

/* Draggable list. The divider insets to the text, or past the artwork, so the rows read
   as rows of one thing rather than as a stack of sections. */
.drag { display: flex; flex-direction: column; gap: calc(var(--space) * 2); max-width: var(--measure-form); }
.drag__hint { line-height: var(--lh-label); font-size: var(--text-xs); color: var(--color-muted); margin: 0; }
.drag__list { list-style: none; margin: 0; padding: 0; border: var(--border-weight) solid var(--color-border); border-radius: var(--radius-sm); }
.drag__row { display: flex; align-items: center; gap: calc(var(--space) * 4); padding: calc(var(--space) * 4); position: relative; }
/* The divider is INSET so it starts under the text, which is Base's measured 16, or 80
   past the artwork. A border-top on the row would run full width and read as a boundary
   between sections rather than between rows of one kind - which is what this stylesheet
   actually did while the block's comment claimed the inset. A positioned rule can start
   where a border cannot. */
.drag__row + .drag__row::before { content: ''; position: absolute; top: 0; left: calc(var(--space) * 4); right: 0; height: var(--border-weight); background: var(--color-border); }
.drag--art .drag__row + .drag__row::before { left: calc(var(--space) * 20); }
.drag__art { width: calc(var(--space) * 12); height: calc(var(--space) * 12); border-radius: var(--radius-sm); background: var(--color-border); flex-shrink: 0; }
.drag__pos { width: calc(var(--space) * 6); line-height: var(--lh-label); font-size: var(--text-sm); font-weight: 600; color: var(--color-muted); flex-shrink: 0; }
.drag__text { display: flex; flex-direction: column; gap: calc(var(--space) * 1); flex: 1; min-width: 0; }
.drag__title { line-height: var(--lh-label); font-size: var(--text-sm); font-weight: 600; color: var(--color-ink); }
.drag__meta { line-height: var(--lh-body); font-size: var(--text-xs); color: var(--color-muted); }
.drag__controls { display: flex; align-items: center; gap: calc(var(--space) * 1); flex-shrink: 0; }
.drag__move { width: calc(var(--space) * 7); height: calc(var(--space) * 7); border: var(--border-weight) solid var(--color-border); border-radius: var(--radius-sm); background: var(--color-bg); color: var(--color-ink); cursor: pointer; line-height: var(--lh-label); font-size: var(--text-xs); }
.drag__move[aria-disabled="true"] { color: var(--color-muted); cursor: default; }
.drag__handle { display: flex; flex-direction: column; gap: calc(var(--space) * 0.75); padding: 0 calc(var(--space) * 2); cursor: grab; }
.drag__handle span { width: calc(var(--space) * 4); height: var(--border-weight); background: var(--color-muted); }
.drag__sr { position: absolute; width: var(--border-weight); height: var(--border-weight); overflow: hidden; clip-path: inset(50%); white-space: nowrap; }

/* Progress bar, determinate. Base measured 2px at Small and 4px at Medium. The fill's
   width is inline because it is the DATA: it is derived from value over max, and no class
   can carry a number that changes per render. */
.pbar { display: flex; flex-direction: column; gap: calc(var(--space) * 3); max-width: var(--measure-form); }
.pbar__track { width: 100%; background: var(--color-border); border-radius: 999px; overflow: hidden; }
.pbar__track--small { height: calc(var(--space) * 0.5); }
.pbar__track--medium { height: calc(var(--space) * 1); }
.pbar__fill { height: 100%; background: var(--color-accent); border-radius: 999px; }
.pbar__label { line-height: var(--lh-label); font-size: var(--text-sm); color: var(--color-ink); margin: 0; }
.pbar__figure { color: var(--color-muted); }

/* Progress steps. The vertical connector is a child of the step's rail, so the spacing
   around the step cannot squeeze it to nothing - which it did, measurably, in the Figma
   redraw when the gap lived in the row's padding instead. */
.psteps { max-width: var(--measure-wide); }
.psteps__row { display: flex; align-items: flex-start; justify-content: space-between; list-style: none; margin: 0; padding: 0 calc(var(--space) * 10); gap: calc(var(--space) * 4); }
.psteps__col { list-style: none; margin: 0; padding: 0; }
.psteps__step { display: flex; flex-direction: column; align-items: center; gap: calc(var(--space) * 2); flex: 1; }
.psteps__dot { width: calc(var(--space) * 9); height: calc(var(--space) * 9); border-radius: 999px; display: flex; align-items: center; justify-content: center; line-height: var(--lh-label); font-size: var(--text-sm); font-weight: 600; flex-shrink: 0; }
.psteps__dot--done, .psteps__dot--current { background: var(--color-accent); color: var(--color-accentInk); }
.psteps__dot--todo { border: calc(var(--border-weight) * 2) solid var(--color-border); color: var(--color-muted); }
/* All three states set their own label, so none of them is the silent default. The
   hierarchy is deliberate: the CURRENT step is the one a reader needs, so done recedes
   rather than competing with it. */
.psteps__label { line-height: var(--lh-label); font-size: var(--text-xs); color: var(--color-muted); font-weight: 400; }
.psteps__step--done .psteps__label { color: var(--color-muted); font-weight: 600; }
.psteps__step--current .psteps__label { color: var(--color-ink); font-weight: 600; }
.psteps__step--todo .psteps__label { color: var(--color-muted); font-weight: 400; }
.psteps__detail { line-height: var(--lh-body); font-size: var(--text-xs); color: var(--color-muted); }
.psteps--vertical .psteps__step { flex-direction: row; align-items: flex-start; gap: calc(var(--space) * 4); }
.psteps__rail { display: flex; flex-direction: column; align-items: center; gap: calc(var(--space) * 1); }
.psteps__line { width: var(--border-weight); height: calc(var(--space) * 5); background: var(--color-border); }
.psteps__body { display: flex; flex-direction: column; gap: calc(var(--space) * 0.5); padding-bottom: calc(var(--space) * 5); }
.psteps__sr { position: absolute; width: var(--border-weight); height: var(--border-weight); overflow: hidden; clip-path: inset(50%); white-space: nowrap; }

/* Banner and system banner. The tone is a name and the ink is a per-tone token, because
   warning is the one tone whose accessible ink is dark. One ink for every tone either
   fails contrast on amber or washes out on red. */
.banner { display: flex; align-items: flex-start; gap: calc(var(--space) * 4); padding: calc(var(--space) * 4); border-radius: var(--radius-lg); max-width: var(--measure-wide); }
.banner__mark { width: calc(var(--space) * 5); height: calc(var(--space) * 5); border-radius: 999px; background: currentColor; flex-shrink: 0; opacity: 0.9; }
.banner__text { display: flex; flex-direction: column; gap: calc(var(--space) * 1); flex: 1; }
.banner__headline { line-height: var(--lh-label); font-size: var(--text-md); font-weight: 600; margin: 0; }
.banner__body { line-height: var(--lh-body); font-size: var(--text-sm); margin: 0; }
.banner__action { color: currentColor; font-weight: 600; text-decoration: underline; line-height: var(--lh-label); font-size: var(--text-sm); flex-shrink: 0; }
.banner--inline { align-items: center; }
.banner--accent { background: var(--color-accent); color: var(--color-accentInk); }
.banner--info { background: var(--color-info); color: var(--color-infoInk); }
.banner--success { background: var(--color-success); color: var(--color-successInk); }
.banner--warning { background: var(--color-warning); color: var(--color-warningInk); }
.banner--danger { background: var(--color-danger); color: var(--color-dangerInk); }

.sysbanner { display: flex; align-items: center; gap: calc(var(--space) * 3); padding: calc(var(--space) * 4) calc(var(--space) * 6); width: 100%; }
.sysbanner__mark { width: calc(var(--space) * 3); height: calc(var(--space) * 3); border-radius: 999px; background: currentColor; flex-shrink: 0; }
.sysbanner__message { flex: 1; line-height: var(--lh-label); font-size: var(--text-sm); font-weight: 500; margin: 0; }
.sysbanner__dismiss { border: 0; background: none; color: currentColor; font: inherit; font-weight: 600; line-height: var(--lh-label); font-size: var(--text-sm); cursor: pointer; text-decoration: underline; }
.sysbanner__link { color: currentColor; font-weight: 600; text-decoration: underline; line-height: var(--lh-label); font-size: var(--text-sm); }
.sysbanner--accent { background: var(--color-accent); color: var(--color-accentInk); }
.sysbanner--info { background: var(--color-info); color: var(--color-infoInk); }
.sysbanner--success { background: var(--color-success); color: var(--color-successInk); }
.sysbanner--warning { background: var(--color-warning); color: var(--color-warningInk); }
.sysbanner--danger { background: var(--color-danger); color: var(--color-dangerInk); }

/* Message card. Bordered rather than tonal: a tone would say "state", and this is an
   invitation. Its action is tertiary so it cannot compete with the page's own. */
.mcard { display: flex; align-items: stretch; gap: calc(var(--space) * 2); background: var(--color-surface); border: var(--border-weight) solid var(--color-border); border-radius: var(--radius-lg); overflow: hidden; max-width: var(--measure); }
.mcard__content { display: flex; flex-direction: column; gap: calc(var(--space) * 1); padding: calc(var(--space) * 4); flex: 1; align-items: flex-start; }
.mcard__heading { line-height: var(--lh-label); font-size: var(--text-md); font-weight: 600; color: var(--color-ink); margin: 0; }
.mcard__body { line-height: var(--lh-body); font-size: var(--text-sm); color: var(--color-muted); margin: 0; }
.mcard__action { line-height: var(--lh-label); font-size: var(--text-sm); font-weight: 600; color: var(--color-accent); text-decoration: none; margin-top: calc(var(--space) * 1); }
.mcard__art { width: calc(var(--space) * 28); background: var(--color-border); flex-shrink: 0; }
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
  const rendered = manifest.page.map((entry) => {
    const blockRegistry = BLOCKS[entry.block];
    if (!blockRegistry) throw new Error(`no block type "${entry.block}" (looked in blocks/)`);
    const renderFn = blockRegistry[entry.variant];
    if (!renderFn) throw new Error(`no variant "${entry.variant}" for block "${entry.block}"`);
    return { block: entry.block, html: renderFn(entry.content) };
  });

  // A marketing page is a stack of full-width blocks, so the blocks are siblings and
  // that is the whole layout. An APP is not: its sidebar is a rail BESIDE the content,
  // not a band above it. Rendering both as flat siblings put the sidebar on its own
  // row with the workspace links floating over empty space.
  //
  // `layout: 'app'` is set by compose.js for the saas-app route and says so
  // structurally rather than leaving CSS to guess from a class name.
  let bodyHtml;
  if (manifest.layout === 'app') {
    const top = rendered.filter((r) => r.block === 'nav').map((r) => r.html).join('\n');
    const rail = rendered.filter((r) => r.block === 'sidebar').map((r) => r.html).join('\n');
    const main = rendered.filter((r) => r.block !== 'nav' && r.block !== 'sidebar').map((r) => r.html).join('\n');
    bodyHtml = `${top}
<div class="shell">
${rail}
<main class="shell__main">
${main}
</main>
</div>`;
  } else {
    bodyHtml = rendered.map((r) => r.html).join('\n');
  }

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

/*
 * ONE stylesheet for the whole site, not one per page.
 *
 * A four-page build wrote four byte-identical CSS files (measured: same md5), which is
 * four chances for them to stop being identical and four times the bytes over the wire.
 * The name is fixed rather than derived from the manifest, because every page has to
 * point at the SAME file for a browser to reuse it - `about.css` and `index.css` with
 * identical contents are still two downloads.
 *
 * Every page's CSS comes out of the same `cssVars(tokens) + STRUCTURE_CSS`, so this is
 * not a merge: pages built from one token pack produce one stylesheet by construction.
 * `tests/render.test.js` pins that, so a future per-page rule cannot land here silently.
 */
const SITE_CSS = 'site.css';

function build(manifestPath) {
  const manifest = readJson(manifestPath);
  const tokens = readJson(path.join(ROOT, 'tokens', `${manifest.stylePack}.json`));
  const name = path.basename(manifestPath, '.json');

  const { html, css } = renderPage(manifest, tokens, SITE_CSS);

  fs.mkdirSync(DIST, { recursive: true });
  fs.writeFileSync(path.join(DIST, SITE_CSS), css);
  fs.writeFileSync(path.join(DIST, `${name}.html`), html);
  const summary = manifest.page.map((e) => `${e.block}=${e.variant}`).join(' ');
  console.log(`${name}: ${summary} style=${manifest.stylePack} -> dist/${name}.html`);
}

module.exports = {
  renderPage, build, cssVars, BLOCKS, STRUCTURE_CSS, SITE_CSS, TYPE_STEPS, TYPE_LEADING,
  BUTTON_RADIUS, TABLE_DENSITY, MOTION_DURATION, MOTION_EASE, MOTION_DISTANCE,
};

if (require.main === module) {
  const arg = process.argv[2];
  if (!arg) {
    console.error('usage: node build.js manifests/<name>.json');
    process.exit(1);
  }
  build(path.resolve(ROOT, arg));
}
