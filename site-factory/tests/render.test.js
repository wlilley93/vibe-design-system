'use strict';

/*
 * Every block type, every variant, rendered with the placeholder content the
 * factory itself would hand it.
 *
 * This is the bug class that actually bit: placeholderContent() had entries for
 * three of twelve types and every other one fell through to a generic `{note: ...}`
 * that no render function could destructure. It scaffolded clean and died at build
 * with "Cannot read properties of undefined (reading 'map')". Nothing caught it
 * because nothing ran it.
 */

const test = require('node:test');
const assert = require('node:assert');

const { BLOCKS, renderPage, STRUCTURE_CSS } = require('../build.js');
const { placeholderContent } = require('../scaffold.js');
const { listStylePacks } = require('../compose.js');
const path = require('path');

const PACKS = listStylePacks().map((n) => require(path.join('..', 'tokens', `${n}.json`)));

test('every block variant renders with its own placeholder content', () => {
  const types = Object.keys(BLOCKS);
  assert.ok(types.length >= 17, `expected at least 17 block types, found ${types.length}`);

  for (const type of types) {
    const content = placeholderContent(type);
    assert.ok(
      !Object.prototype.hasOwnProperty.call(content, 'note'),
      `block type "${type}" has no real placeholder - it fell through to the generic {note} object, ` +
      'which no render function can destructure'
    );
    for (const variant of Object.keys(BLOCKS[type])) {
      const manifest = { title: 't', page: [{ block: type, variant, content }] };
      const { html } = renderPage(manifest, PACKS[0], null);
      assert.ok(html.length > 0, `${variant} rendered empty`);
      assert.ok(!html.includes('undefined'), `${variant} leaked the string "undefined" into its markup`);
      assert.ok(!html.includes('[object Object]'), `${variant} leaked "[object Object]" into its markup`);
    }
  }
});

test('every variant renders under every style pack', () => {
  for (const type of Object.keys(BLOCKS)) {
    const content = placeholderContent(type);
    for (const variant of Object.keys(BLOCKS[type])) {
      for (const tokens of PACKS) {
        const manifest = { title: 't', page: [{ block: type, variant, content }] };
        assert.doesNotThrow(
          () => renderPage(manifest, tokens, null),
          `${variant} threw under style pack "${tokens.name}"`
        );
      }
    }
  }
});

test('an unknown block or variant fails loudly, not silently', () => {
  assert.throws(
    () => renderPage({ title: 't', page: [{ block: 'nope', variant: 'nope-1', content: {} }] }, PACKS[0], null),
    /no block type "nope"/,
    'an unknown block type must name itself in the error'
  );
  assert.throws(
    () => renderPage({ title: 't', page: [{ block: 'hero', variant: 'hero-99', content: {} }] }, PACKS[0], null),
    /no variant "hero-99"/,
    'an unknown variant must name itself in the error'
  );
});

test('the structural stylesheet holds no colour or font values', () => {
  // Values belong in the token block; structure may only reference them. A hex here
  // is a value that no style pack can override, which defeats the whole pipeline.
  const hex = STRUCTURE_CSS.match(/#[0-9a-fA-F]{3,8}\b/g);
  assert.equal(hex, null, `structural CSS contains hex literals: ${hex && hex.join(', ')}`);
});

test('rendered markup escapes HTML in content', () => {
  const nasty = '<script>alert(1)</script>';
  const { html } = renderPage(
    { title: 't', page: [{ block: 'hero', variant: 'hero-1', content: { ...placeholderContent('hero'), h1: nasty } }] },
    PACKS[0], null
  );
  assert.ok(!html.includes('<script>alert(1)</script>'), 'content was interpolated without escaping');
  assert.ok(html.includes('&lt;script&gt;'), 'expected the escaped form in the output');
});

test('the stylesheet holds no magic number: every value is a token or a documented sentinel', () => {
  // The README claimed "no hex, no font name, no px literal outside the --space multiplier"
  // and then offered only a grep for HEX as its evidence. Sixteen distinct px declarations
  // survived under that claim for weeks. A rule broader than its check is the same defect as
  // a control the renderer ignores, one level up: both look enforced and are not.
  //
  // So this is the check the same size as the claim.
  const { STRUCTURE_CSS } = require('../build.js');

  // Sentinels, each with a reason. A number that CANNOT be a token belongs here; a number
  // nobody has got round to naming does not, which is why the list is short and argued.
  const SENTINELS = [
    // A pill is "however round it takes", not a length. Any large number does the job and
    // 999px is the idiom; a --radius-pill token would be the same sentinel with a longer name.
    '999px',
    // BREAKPOINTS CANNOT BE TOKENS. A media query is evaluated before the cascade, so
    // `@media (max-width: var(--bp))` does not work in any browser - the custom property is
    // not resolvable at that point. These two are therefore literals by necessity rather
    // than by omission, which is exactly what this list is for.
    //
    // 900px is where a two-column page stops having two readable columns at this type ramp,
    // and 600px is the widest phone in portrait. Both are chosen against the CONTENT, not
    // against a device: the grids collapse when a column can no longer hold a phrase.
    'max-width: 900px',
    'max-width: 600px',
    // The one width inside a component rather than on the page. A data table narrower than
    // this stops being a table and becomes a list, so below it the table scrolls sideways
    // inside its own box instead of squeezing the page. In rem so it tracks the type ramp.
    'min-width: 34rem',
  ];

  let scrubbed = STRUCTURE_CSS;
  for (const s of SENTINELS) scrubbed = scrubbed.split(s).join('<sentinel>');

  const hex = scrubbed.match(/#[0-9a-fA-F]{3,8}\b/g) || [];
  assert.deepEqual(hex, [], `hex literals in the stylesheet: ${hex.join(', ')}`);

  // Any surviving px is a magic number. Report it WITH its declaration, because "there is a
  // px somewhere" is not actionable and the selector is what tells you which token it wants.
  const magic = [...new Set(scrubbed.match(/[a-z-]+: [^;{}]*\b\d+px\b[^;{}]*/g) || [])];
  assert.deepEqual(
    magic, [],
    'magic numbers in the stylesheet. Each is a token that has not been named yet, or a ' +
    `sentinel that has not been argued for:\n  ${magic.join('\n  ')}`
  );

  // A font name in the structural sheet would be the same defect in another currency.
  //
  // The first attempt wrote `/font-family:\s*(?!var\()/` and flagged all fifteen declarations
  // INCLUDING the correct ones: `\s*` can match zero characters, so the lookahead was tested
  // against the space rather than the value. A greedy-optional before a lookahead is a
  // lookahead that never fires where you meant it to.
  const fonts = (scrubbed.match(/font-family:[^;}]+/g) || [])
    .filter((d) => !/font-family:\s+(var\(|inherit\b)/.test(d));
  assert.deepEqual(fonts, [], `font names outside a token: ${fonts.join(', ')}`);
});

test('the width tokens are all consumed, and none is a value nothing reads', () => {
  // The mirror of the rule above: a token nothing uses is the same dead weight as a literal
  // nothing named. Both directions, so adding a token obliges you to use it and removing a
  // use obliges you to remove the token.
  const { cssVars, STRUCTURE_CSS } = require('../build.js');
  const fs = require('node:fs');
  const path = require('node:path');

  const tokens = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'tokens', 'geist.json'), 'utf8'));
  tokens.scale = { density: 'comfortable', type: 'comfortable' };
  const declared = [...cssVars(tokens).matchAll(/^\s*--([a-z-]+):/gm)].map((m) => m[1]);

  const widthish = declared.filter((n) => /^(container|measure|rail|pane)/.test(n));
  assert.ok(widthish.length >= 8, `expected the width tokens to be declared, found ${widthish.join(', ')}`);

  const unused = widthish.filter((n) => !STRUCTURE_CSS.includes(`var(--${n})`));
  assert.deepEqual(unused, [], `width tokens declared and never read: ${unused.join(', ')}`);
});

test('every page-level section takes the page frame, and none centres nothing', () => {
  // THE DEFECT THIS REPLACES: the inset from the viewport edge was restated in eight
  // section rules and omitted from four, so .stats, .gallery, .logos and .timeline ran
  // flush to the edge under neighbours that were inset. It was invisible in review
  // because each rule was individually reasonable; only a page with both showed it.
  const { STRUCTURE_CSS, BLOCKS } = require('../build.js');

  // A page-level block is one a marketing page drops straight in, so it owns the frame.
  // A nested block is rendered INSIDE another and must not take the gutter twice.
  //
  // The first version of this listed every type as nested, which made the partition check
  // below `[] === []` - true for any input, including a new block nobody had classified.
  // Two lists that must partition the registry is the check; one list is a formality.
  const CLASS_OF_DUAL = { objecttable: 'otable' };
  const PAGE_LEVEL = new Set([
    'nav', 'hero', 'features', 'pricing', 'testimonials', 'faq', 'cta', 'contact',
    'team', 'footer', 'logolist', 'stats', 'gallery', 'timeline', 'notfound',
  ]);
  const NESTED = new Set([
    'card', 'banner', 'divider', 'objecttable', 'objectview', 'inspector', 'facetstrip',
    'formfield', 'checkbox', 'radio', 'switch', 'segmentedcontrol', 'menu', 'toast',
    'tooltip', 'notificationbadge', 'progressbar', 'progresssteps', 'pagination',
    'pagecontrols', 'messagecard', 'systembanner', 'confirmdialog', 'draggablelist',
    'emptystate', 'pagestate', 'sidebar', 'masterdetail',
  ]);
  // DUAL-USE: page-level on a marketing page AND nested inside another block, so it takes
  // the frame only under a child combinator. Kept as its own category rather than forced
  // into one of the two, because calling it either would make the frame wrong somewhere.
  const DUAL = new Set(['objecttable']);
  for (const t of DUAL) {
    assert.match(STRUCTURE_CSS, new RegExp(`body > \\.${CLASS_OF_DUAL[t]}[\\s,{]`),
      `${t} is dual-use and must be framed as a direct child of body, not unconditionally`);
  }
  const overlap = [...PAGE_LEVEL].filter((t) => NESTED.has(t) || DUAL.has(t));
  assert.deepEqual(overlap, [], `block type(s) claimed as both page-level and nested: ${overlap.join(', ')}`);
  const unaccounted = Object.keys(BLOCKS).filter((t) => !PAGE_LEVEL.has(t) && !NESTED.has(t) && !DUAL.has(t));
  assert.deepEqual(unaccounted, [],
    `block type(s) with no stated position on the page frame: ${unaccounted.join(', ')}`);

  // Every page-level block's root class must appear in the frame selector list. This is the
  // check that would have caught the original defect: .stats, .gallery, .logos and .timeline
  // were page-level and absent from every gutter rule.
  // SCAN THE SELECTOR, NOT THE PROSE. The first version searched the whole rule INCLUDING
  // its comment, and the comment names .stats, .gallery, .logos and .timeline as the four
  // that used to be missing - so the check matched the explanation of the bug and passed
  // when the fix was removed. Verified by deleting .stats from the selector: still green.
  // Strip comments first, then read only the selector text before the brace.
  const CLASS_OF = { logolist: 'logos', objecttable: 'otable', emptystate: 'empty', pagestate: 'pstate' };
  const noComments = STRUCTURE_CSS.replace(/\/\*[\s\S]*?\*\//g, '');
  const gutterRule = noComments.match(/([^{}]*)\{[^{}]*padding-left: var\(--gutter\);[^{}]*\}/);
  assert.ok(gutterRule, 'no rule in the stylesheet applies the page gutter');
  const selectors = new Set(gutterRule[1].split(',').map((s) => s.trim()).filter(Boolean));
  const missing = [...PAGE_LEVEL]
    .map((t) => `.${CLASS_OF[t] || t}`)
    .filter((cls) => !selectors.has(cls));
  assert.deepEqual(missing, [],
    `page-level section(s) that do not take the page gutter: ${missing.join(', ')}`);

  // The gutter is stated ONCE. If a section rule sets a horizontal padding of its own it
  // will fight the frame, and the winner is decided by source order rather than by intent.
  const perSection = [...STRUCTURE_CSS.matchAll(
    /^\.(nav|hero|features|pricing|testimonials|faq|cta|contact|team|footer|logos|stats|gallery|timeline|notfound|empty) \{([^}]*)\}/gm
  )];
  const restated = perSection
    .filter(([, , body]) => /padding(-left|-right|-inline)?: [^;]*var\(--space\)[^;]*var\(--space\)/.test(body))
    .map(([, cls]) => cls);
  assert.deepEqual(restated, [],
    `section(s) restating a horizontal inset instead of taking --gutter: ${restated.join(', ')}`);

  // `margin: 0 auto` with no max-width centres nothing, and four rules carried it.
  const dead = perSection
    .filter(([, , body]) => /margin: 0 auto/.test(body) && !/max-width/.test(body))
    .map(([, cls]) => cls);
  assert.deepEqual(dead, [],
    `section(s) centring with no max-width, which does nothing: ${dead.join(', ')}`);
});

test('the page frame is overridable, and the override reaches the CSS', () => {
  // A token a project cannot set is the same as a hard-coded value with extra steps.
  // --container was literally `1100px` in the emitter, so a brand drawn to a different
  // page width had no way to say so except editing the engine.
  const { cssVars } = require('../build.js');
  const base = { colors: {}, font: {}, radius: {}, space: { unit: 4 }, scale: { density: 'comfortable', type: 'comfortable' } };

  const dflt = cssVars(base);
  assert.match(dflt, /--container: 1100px;/, 'the default page width is gone');
  assert.match(dflt, /--gutter: calc\(var\(--space\) \* 8\);/, 'the default gutter is gone');

  const custom = cssVars({ ...base, layout: { container: '1210px', gutter: '115px' } });
  assert.match(custom, /--container: 1210px;/, 'a project cannot set its page width');
  assert.match(custom, /--gutter: 115px;/, 'a project cannot set its gutter');
});

test('every font-size comes from the ramp and carries its role leading', () => {
  // Measured before this existed: 103 font-size declarations and THREE carried a
  // line-height. A hundred inherited whatever the cascade gave them, so a 13px label and an
  // 18px paragraph could land on the same leading. The ramp itself held 10, 11, 12, 13, 14
  // and 15 - six sizes inside a 6px span, which is the arbitrariness the width tokens
  // already taught us to collapse.
  //
  // The rule is Uber Base's, measured out of its Figma file: LEADING IS A FUNCTION OF ROLE.
  // Label and Paragraph share every size and differ only in leading, because a label does not
  // wrap and body copy does. A system with one line-height per size cannot say that.
  const { STRUCTURE_CSS, TYPE_STEPS, TYPE_LEADING } = require('../build.js');

  // No size may be a literal. It has to name a step.
  //
  // Matched then FILTERED, not `\s*(?!var\()`. That lookahead sits after a greedy-optional
  // that can consume nothing, so it is tested against the space and passes on every value -
  // the same mistake the font-name check made an hour earlier, which flagged all fifteen
  // correct declarations. A greedy-optional before a lookahead is a lookahead that never
  // fires where you meant it to.
  const literals = [...new Set(STRUCTURE_CSS.match(/font-size:[^;}]+/g) || [])]
    .filter((d) => !/font-size:\s+var\(--text-/.test(d));
  assert.deepEqual(literals, [], `font sizes outside the ramp: ${literals.join(', ')}`);

  // Nor may a leading be a bare number: that is how 1.1, 1.3 and 1.5 survived as magic.
  const bareLh = [...new Set(STRUCTURE_CSS.match(/line-height:[^;}]+/g) || [])]
    .filter((d) => !/line-height:\s+var\(--lh-/.test(d));
  assert.deepEqual(bareLh, [], `hardcoded line-heights: ${bareLh.join(', ')}`);

  // And every size must be PAIRED with a leading in its own rule. An unleaded size is the
  // original defect, and it is invisible because the cascade always supplies something.
  const unleaded = [];
  let selector = '';
  for (const line of STRUCTURE_CSS.split('\n')) {
    const m = line.match(/^([.a-z][^{]*)\{/);
    if (m) selector = m[1].trim();
    if (/font-size: var\(--text-/.test(line) && !/line-height: var\(--lh-/.test(line)) {
      unleaded.push(`${selector}: ${line.trim().slice(0, 60)}`);
    }
  }
  assert.deepEqual(unleaded, [],
    `font-size with no leading in the same rule:\n  ${unleaded.join('\n  ')}`);

  // Every step and role referenced must be declared, and every one declared must be used.
  const usedSteps = new Set([...STRUCTURE_CSS.matchAll(/var\(--text-([a-z0-9]+)\)/g)].map((x) => x[1]));
  const usedRoles = new Set([...STRUCTURE_CSS.matchAll(/var\(--lh-([a-z]+)\)/g)].map((x) => x[1]));
  for (const s of usedSteps) assert.ok(TYPE_STEPS[s] !== undefined, `--text-${s} is used and not declared`);
  for (const r of usedRoles) assert.ok(TYPE_LEADING[r] !== undefined, `--lh-${r} is used and not declared`);
  const deadRoles = Object.keys(TYPE_LEADING).filter((r) => !usedRoles.has(r));
  assert.deepEqual(deadRoles, [], `leading roles declared and never read: ${deadRoles.join(', ')}`);
});

test('every element class the blocks emit has a rule in the stylesheet', () => {
  // Found eleven of these at once, and they were not cosmetic. `.md__rail` is the third
  // pane of the master-detail assembly and had no border, so the inspector ran into the
  // detail with nothing between them. `.md` is that assembly's own root and had no rule at
  // all, so the facet strip sat above the panes only by accident of document flow. Every
  // one was a container the layout leaned on while the cascade happened to hold it up.
  //
  // The line is drawn at ELEMENT vs MODIFIER, and the distinction is real rather than
  // convenient. A `--modifier` on a block root is a documented hook: `.pricing--table`
  // exists so a consuming project can tell one variant from another in the DOM, and it is
  // legitimately unstyled here because the structural sheet has no opinion on it. An
  // `__element` class is a part of the component, and an unstyled part is a part whose
  // layout is inherited by luck.
  const { STRUCTURE_CSS } = require('../build.js');
  const { placeholderContent } = require('../scaffold.js');
  const { listBlockVariants } = require('../compose.js');

  // Comments are STRIPPED before extracting, and finding out why cost a negative control.
  // The first version read the sheet whole, so a class named in a comment counted as
  // declared - and this test's own comment names .md__rail. Deleting that rule left the
  // test green, because the prose explaining the rule satisfied the check that the rule
  // existed. A check a comment can satisfy is not a check.
  const rules = STRUCTURE_CSS.replace(/\/\*[\s\S]*?\*\//g, ' ');
  const declared = new Set(
    [...rules.matchAll(/\.([a-zA-Z][a-zA-Z0-9_-]*)/g)].map((m) => m[1])
  );

  const unstyled = new Set();
  let rendered = 0;
  const variants = listBlockVariants();
  for (const type of Object.keys(variants)) {
    const content = placeholderContent(type);
    for (const variant of variants[type]) {
      const { html } = renderPage({ title: 't', page: [{ block: type, variant, content }] }, PACKS[0], null);
      rendered++;
      for (const m of html.matchAll(/class="([^"]+)"/g)) {
        for (const cls of m[1].split(/\s+/)) {
          // A modifier is exempt; an element is not. A bare block root with no modifier
          // is an element too - `.md` was exactly that case.
          if (!cls || declared.has(cls) || cls.includes('--')) continue;
          unstyled.add(`${type}/${variant}: .${cls}`);
        }
      }
    }
  }

  // Guard the guard: a check that renders nothing passes trivially, and this one walks a
  // directory to find its work.
  assert.ok(rendered >= 70, `only ${rendered} variants rendered - the walk found almost nothing`);
  assert.deepEqual([...unstyled].sort(), [],
    'these element classes are emitted by a block and have no rule in the stylesheet. Each is ' +
    `a part of a component whose layout is inherited by luck:\n  ${[...unstyled].sort().join('\n  ')}`);
});
