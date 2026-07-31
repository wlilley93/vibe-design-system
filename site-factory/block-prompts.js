'use strict';
/*
 * A PROMPT PER BLOCK TYPE, DERIVED RATHER THAN AUTHORED.
 *
 * The ask was "give every block type its own prompt". Forty-three hand-written
 * prompts would be forty-three hand-maintained artefacts, and this programme has
 * already filed what happens to those: a prompt still describing three variants
 * after a fourth lands is not out of date in any visible way, it just quietly
 * stops matching the thing it names. So every prompt here is BUILT from what is
 * already true:
 *
 *   PURPOSE       the block module's own doc comments - the prose already
 *                 written beside the code, at the source of truth
 *   VARIANTS      the exported keys, read from the module
 *   FIGMA AXIS    the measured axis from figma-variants.json, including the
 *                 eleven blocks where Figma and code vary DIFFERENT THINGS,
 *                 which the prompt states rather than papering over
 *   CONTENT       the `content.<field>` reads found in the module, which is the
 *                 block's real input contract
 *   STYLE         one shared block, held constant across all 43
 *
 * # Where the style block comes from, and why it is separate
 *
 * The GRIGOLETTO prompt kit's 82 prompts vary by SITE GENRE and hold a fixed
 * "cinematic DNA" constant across all of them. That separation is the reusable
 * idea and it is borrowed with attribution (vendor/grigoletto/): the STYLE half
 * is genre-independent and belongs in every prompt, and the CONTRACT half is
 * per-block and is the part a corpus of marketing prompts can never supply.
 *
 * The style block is the one thing here that is authored, and it is authored
 * ONCE. It carries no design value - no hex, no font, no length - because a
 * prompt that names a colour would be a fourth design authority, which
 * [2026] VJS-CC-OPBOX 3 D1 forbids. It names the TOKENS to read instead.
 */

const fs = require('node:fs');
const path = require('node:path');

const ROOT = __dirname;

/**
 * The one authored paragraph, held constant across all 43.
 *
 * Every instruction here is either a token reference or a behaviour. Nothing in
 * it is a realisation, so a generator following it cannot invent a design value
 * without going outside the prompt.
 */
const STYLE = [
  'Every visual value must be a CSS custom property that already exists: colour from',
  '--color-*, spacing as a multiple of var(--space), radius from --radius-*, type from',
  '--text-* paired with its --lh-* leading, boundary weight from --border-weight.',
  'No hex, no font name, no bare px. A value you cannot name as a token is a value',
  'this system has not decided yet - say so instead of choosing one.',
  'Semantic HTML with a real landmark or heading level, not a div carrying a class.',
  'Responsive by container, not by device. Honour prefers-reduced-motion.',
].join(' ');

/**
 * Every comment in a module, block AND line, in source order with offsets.
 *
 * The line half was missing and it mattered: the first version read only
 * `/* ... *\/` blocks, and SEVENTEEN of the forty-three blocks document
 * themselves entirely in `//` runs - `// nav-1: logo left, links + CTA right,
 * one row.` sits directly above `function navSimple`. Those seventeen got a
 * prompt with no purpose and no variant notes: structurally present, and empty
 * of the only thing that makes a prompt worth having. A generator that reports
 * "43 of 43" over seventeen hollow entries is the shape this repository keeps
 * filing, so the count now has to be earned.
 *
 * A RUN of consecutive line comments is ONE comment. Splitting them would make
 * a three-line note read as three notes and the last line alone would win the
 * "immediately preceding" test.
 */
function commentsIn(code) {
  const out = [];
  for (const m of code.matchAll(/\/\*([\s\S]*?)\*\//g)) {
    const body = m[1]
      .split('\n')
      .map((l) => l.replace(/^\s*\*\s?/, '').trimEnd())
      .join('\n')
      .trim();
    if (body) out.push({ at: m.index, body });
  }

  // Line-comment runs. `at` is the START of the run, so the "which comment
  // precedes this function" test measures from where the note begins.
  const lines = code.split('\n');
  let offset = 0;
  let run = null;
  const flush = () => {
    if (run && run.text.length) out.push({ at: run.at, body: run.text.join('\n').trim() });
    run = null;
  };
  for (const line of lines) {
    const t = line.trim();
    // Only a WHOLE-LINE comment. A trailing `foo(); // why` is about that
    // statement, not about the function below it.
    if (t.startsWith('//')) {
      if (!run) run = { at: offset, text: [] };
      run.text.push(t.replace(/^\/\/\s?/, ''));
    } else if (t.length) {
      flush();
    }
    offset += line.length + 1;
  }
  flush();

  out.sort((a, b) => a.at - b.at);
  return out;
}

/**
 * What a block reads off `content`.
 *
 * The block's real input contract, and derivable: a field the module never reads
 * is a field the prompt must not promise. Sorted so the output is stable.
 */
function contentFieldsIn(code) {
  const out = new Set();
  for (const m of code.matchAll(/\bcontent\.([a-zA-Z][\w]*)/g)) out.add(m[1]);
  // Destructured reads: `const { title, items } = content`.
  for (const m of code.matchAll(/const\s*\{([^}]*)\}\s*=\s*content\b/g)) {
    for (const part of m[1].split(',')) {
      const name = part.split(/[:=]/)[0].trim();
      if (/^[a-zA-Z][\w]*$/.test(name)) out.add(name);
    }
  }
  return [...out].sort();
}

/** One block type, read from its module and the measured manifests. */
function readBlock(type, variantsManifest) {
  const file = path.join(ROOT, 'blocks', `${type}.js`);
  const code = fs.readFileSync(file, 'utf8');
  const comments = commentsIn(code);
  const measured = variantsManifest[type] || {};

  // The exported variant keys, from the module rather than from a list here.
  const exportsMatch = code.match(/module\.exports\s*=\s*\{([\s\S]*?)\}/);
  const keys = exportsMatch
    ? [...exportsMatch[1].matchAll(/'([a-z0-9-]+)'\s*:/g)].map((m) => m[1])
    : [];

  // The PURPOSE is the first comment that is prose rather than a lint pragma or
  // the escaping helper's header. Taking comments[0] blindly picked up `esc`.
  const purpose = comments.find((c) => c.body.length > 80 && !/^eslint|^@ts/.test(c.body));

  // A per-variant note is the comment immediately preceding that variant's
  // function. Located by offset, so a comment that merely MENTIONS a variant
  // name elsewhere in the file cannot be mistaken for its documentation.
  const notes = {};
  // ONE COMMENT CANNOT DOCUMENT TWO VARIANTS. Without this, deleting a
  // variant's comment made it silently inherit the previous variant's - the
  // nearest preceding comment is simply further away - so the note count stayed
  // at 86 of 86 and the guard reported full coverage over a variant nobody had
  // described. Seeded exactly that way, and it passed; a check that cannot fail
  // is the failure this repository files most often.
  const claimed = new Set();
  for (const key of keys) {
    const fnName = (code.match(new RegExp(`'${key}'\\s*:\\s*([A-Za-z_$][\\w$]*)`)) || [])[1];
    if (!fnName) continue;
    const at = code.indexOf(`function ${fnName}(`);
    if (at < 0) continue;
    const before = comments.filter((c) => c.at < at).pop();
    if (!before || at - before.at >= 900) continue;
    if (claimed.has(before.at)) continue;
    claimed.add(before.at);
    notes[key] = before.body;
  }

  return { type, keys, purpose: purpose ? purpose.body : null, notes, measured };
}

/** The prompt text for one block. */
function promptFor(block) {
  const { type, keys, purpose, notes, measured } = block;
  const lines = [];
  lines.push(`Build the \`${type}\` block for this design system.`);
  lines.push('');

  if (purpose) {
    lines.push('WHAT IT IS FOR (from the block module itself, not a description of it):');
    for (const l of purpose.split('\n')) lines.push(`  ${l}`);
    lines.push('');
  }

  lines.push(`VARIANTS - build all ${keys.length}, they are not styles of one thing:`);
  for (const k of keys) {
    lines.push(`  ${k}`);
    const note = notes[k];
    if (note) for (const l of note.split('\n').slice(0, 6)) lines.push(`      ${l}`);
  }
  lines.push('');

  const axes = measured.axes || {};
  const axisNames = Object.keys(axes);
  if (axisNames.length) {
    lines.push(`IN FIGMA this is \`${measured.setName}\`, varying on ` +
      axisNames.map((a) => `${a} (${axes[a].join(', ')})`).join(' and ') + '.');
    if (measured.axisVerdict === 'different_axis') {
      // The eleven. Stating it is the whole point: a generator told the two
      // sides correspond will invent a correspondence that does not exist.
      lines.push('  THE TWO SIDES VARY DIFFERENT THINGS, and that is measured, not an oversight:');
      lines.push(`  ${measured.noBindingBecause}`);
      lines.push('  So do NOT map a code variant onto a Figma variant here. Build the code');
      lines.push('  variants above; the Figma axis is a separate decision nobody has reconciled.');
    }
    lines.push('');
  }

  const fields = measured.contentFields || [];
  if (fields.length) {
    lines.push('CONTENT it accepts (every field the module actually reads - promise nothing else):');
    lines.push(`  ${fields.join(', ')}`);
    lines.push('');
  }

  lines.push('STYLE:');
  lines.push(`  ${STYLE}`);
  return lines.join('\n');
}

/** Every block type, with its derived prompt. */
function buildAll() {
  const variantsManifest = JSON.parse(
    fs.readFileSync(path.join(ROOT, 'figma-variants.json'), 'utf8'),
  );
  const manifest = variantsManifest.blocks || variantsManifest;
  const types = Object.keys(manifest).sort();
  const out = {};
  for (const type of types) {
    const block = readBlock(type, manifest);
    block.measured.contentFields = contentFieldsIn(
      fs.readFileSync(path.join(ROOT, 'blocks', `${type}.js`), 'utf8'),
    );
    out[type] = {
      variants: block.keys,
      hasPurpose: Boolean(block.purpose),
      notedVariants: Object.keys(block.notes),
      contentFields: block.measured.contentFields,
      prompt: promptFor(block),
    };
  }
  return out;
}

module.exports = { buildAll, promptFor, readBlock, contentFieldsIn, commentsIn, STYLE };

if (require.main === module) {
  const all = buildAll();
  const which = process.argv[2];
  if (which) {
    if (!all[which]) {
      console.error(`no such block type: ${which}`);
      process.exit(1);
    }
    console.log(all[which].prompt);
  } else {
    const noPurpose = Object.entries(all).filter(([, b]) => !b.hasPurpose).map(([t]) => t);
    console.log(`${Object.keys(all).length} block types, all with a prompt.`);
    console.log(`  variants covered:  ${Object.values(all).reduce((n, b) => n + b.variants.length, 0)}`);
    console.log(`  with a purpose:    ${Object.keys(all).length - noPurpose.length}`);
    if (noPurpose.length) console.log(`  WITHOUT a purpose: ${noPurpose.join(', ')}`);
    console.log('\nOne prompt:  node block-prompts.js <type>');
  }
}
