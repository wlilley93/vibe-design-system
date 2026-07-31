'use strict';
/*
 * GOLDEN BYTES FOR THE FIGMA DRAWING GENERATOR.
 *
 * `figma-draw.js` was already gated on two runs being byte-identical. That
 * catches a generator that MOVES ON ITS OWN and catches nothing else: an
 * intended edit to the emitter passes, silently, because both runs agree with
 * each other and neither is compared to anything committed. The script it emits
 * WRITES TO THE LIVE FIGMA FILE, so an unreviewed change there redraws 43
 * component sets. Determinism is the weaker half of the check.
 *
 * Borrowed with attribution from `southleft/ds-contracts-poc`, which keeps a
 * golden per emitted file rather than a determinism assertion.
 *
 * TWO CHOICES HERE ARE DELIBERATE.
 *
 * 1. THE GOLDEN IS THE LITERAL BYTES, not just a digest. A digest fails with
 *    "85ee1d4 != 3b91c02", which tells a reviewer that something changed and
 *    nothing about what. Committing the emitted script makes `git diff` the
 *    review surface, which is the entire reason to have a golden.
 *
 * 2. THE INPUT DIGESTS ARE DIAGNOSTICS, NEVER ASSERTIONS. The emitted script is
 *    a function of the emitter AND its data - `figma-variants.json` and the
 *    register. Asserting on those digests too would fail this gate for a comment
 *    edit that changes no emitted byte, which is the cry-wolf failure. They are
 *    recorded so a failure can say WHICH SIDE moved: a data change is ordinary
 *    and expected, while bytes that moved with every data input byte-identical
 *    mean the emitter itself was edited, and that is the case needing eyes.
 *
 *    The emitter's own digest is recorded and DELIBERATELY not classified on -
 *    see the note in `inputs()` for the unreachable branch that produced.
 *
 * The test never writes this file. A check that can rewrite its own expectation
 * cannot fail; updating is an explicit `node figma-golden.js --update`, which
 * puts the new bytes in the diff where a reader has to look at them.
 */

const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');

const HERE = __dirname;
const GOLDEN_SCRIPT = path.join(HERE, 'vendor', 'figma-draw.golden.js');
const GOLDEN_MANIFEST = path.join(HERE, 'vendor', 'figma-draw.golden.json');

const sha256 = (s) => crypto.createHash('sha256').update(s).digest('hex');

/**
 * The inputs the emitted script is derived from, and the register derived the
 * same way the test and the live sweep derive it.
 *
 * The register is built here rather than read, because there is no committed
 * register file for the master Figma library - the blocks ARE the register. If
 * that ever changes this is the one place to repoint.
 */
function inputs() {
  const variantsText = fs.readFileSync(path.join(HERE, 'figma-variants.json'), 'utf8');
  const variants = JSON.parse(variantsText);
  const register = Object.keys(variants.blocks).sort().map((blockType, i) => ({
    id: `CMP-${String(i + 1).padStart(4, '0')}`,
    name: blockType[0].toUpperCase() + blockType.slice(1),
    blockType,
  }));
  return {
    variants,
    register,
    // THE EMITTER IS NOT AN INPUT TO ITSELF. The first version of this file listed
    // `figma-draw.js` here alongside the data, which made the `emitter_moved` branch
    // UNREACHABLE: every edit to the generator moves that digest, so every edit reported
    // `inputs_moved` - "an ordinary consequence, read the diff" - and the one case the
    // whole distinction exists to surface could never fire. Caught by seeding a real edit
    // to `figma-draw.js` and reading which branch answered, which the pure-function
    // negative controls could not catch: they fabricate the digests they are handed.
    digests: {
      'figma-variants.json': sha256(variantsText),
      register: sha256(JSON.stringify(register)),
    },
    // Recorded, never classified on. A reader of a failure wants to know whether the
    // generator source moved; the VERDICT is already settled by the data inputs.
    emitter_sha256: sha256(fs.readFileSync(path.join(HERE, 'figma-draw.js'), 'utf8')),
  };
}

/** The bytes the generator emits right now. */
function emitted() {
  const { buildLibraryScript } = require('./figma-draw.js');
  const { variants, register, digests, emitter_sha256 } = inputs();
  return { script: buildLibraryScript(register, variants), digests, emitter_sha256 };
}

/**
 * The whole verdict, as a pure function of three values.
 *
 * Pure on purpose: the only way to prove this gate can FAIL is to hand it a
 * golden that disagrees, and a check that only reads committed files can be
 * driven no other way than by writing to the tree - which is how a seed goes
 * missing and a dead gate reads as a passing one.
 */
function classify(golden, manifest, now) {
  // The manifest and the bytes beside it must agree before either is trusted.
  // Hand-editing one and not the other would otherwise produce a golden that
  // certifies bytes it is not sitting next to.
  if (sha256(golden) !== manifest.script_sha256) {
    return {
      ok: false,
      reason: 'manifest_disagrees_with_bytes',
      detail: `${path.basename(GOLDEN_MANIFEST)} records ${manifest.script_sha256.slice(0, 12)} but `
        + `${path.basename(GOLDEN_SCRIPT)} hashes to ${sha256(golden).slice(0, 12)}. One was edited `
        + 'without the other. Re-run `node figma-golden.js --update`.',
    };
  }

  if (now.script === golden) return { ok: true, moved: [], unchanged: Object.keys(now.digests) };

  const moved = Object.keys(now.digests).filter((k) => now.digests[k] !== manifest.inputs[k]);
  const unchanged = Object.keys(now.digests).filter((k) => now.digests[k] === manifest.inputs[k]);
  return {
    ok: false,
    reason: moved.length ? 'inputs_moved' : 'emitter_moved',
    moved,
    unchanged,
    detail: moved.length
      ? `the emitted script changed and so did ${moved.join(', ')} - an ordinary consequence. `
        + 'Read the diff and update the golden.'
      : 'the emitted script changed while EVERY input is byte-identical, so the emitter itself was '
        + 'edited. This redraws the live Figma library: read the diff before updating the golden.',
    lengths: { golden: golden.length, now: now.script.length },
  };
}

/** `classify` against what is actually committed and what the generator emits now. */
function check() {
  if (!fs.existsSync(GOLDEN_SCRIPT) || !fs.existsSync(GOLDEN_MANIFEST)) {
    return { ok: false, reason: 'absent', detail: `${GOLDEN_SCRIPT} has never been written` };
  }
  return classify(
    fs.readFileSync(GOLDEN_SCRIPT, 'utf8'),
    JSON.parse(fs.readFileSync(GOLDEN_MANIFEST, 'utf8')),
    emitted(),
  );
}

function update() {
  const { script, digests, emitter_sha256 } = emitted();
  fs.writeFileSync(GOLDEN_SCRIPT, script);
  fs.writeFileSync(GOLDEN_MANIFEST, `${JSON.stringify({
    _what: 'The bytes site-factory/figma-draw.js emits, committed so a change to the generator '
      + 'arrives as a reviewable diff rather than a silently passing test.',
    _why_bytes_not_a_digest: 'A digest mismatch names no change. The script writes to the live '
      + 'Figma file; a reviewer needs to see what a redraw would do before it does it.',
    _inputs_are_diagnostics: 'These digests are NOT asserted. They exist so a failure can say '
      + 'whether a DATA input moved (ordinary) or only the emitter did (needs eyes). '
      + 'emitter_sha256 is recorded and never classified on: listing the emitter among the '
      + 'inputs once made the emitter_moved branch unreachable.',
    script: path.basename(GOLDEN_SCRIPT),
    script_sha256: sha256(script),
    script_bytes: script.length,
    inputs: digests,
    emitter_sha256,
  }, null, 2)}\n`);
  return { script_sha256: sha256(script), bytes: script.length, inputs: digests };
}

module.exports = {
  check, classify, update, emitted, inputs, sha256, GOLDEN_SCRIPT, GOLDEN_MANIFEST,
};

if (require.main === module) {
  if (process.argv.includes('--update')) {
    const r = update();
    console.log(`golden updated: ${r.bytes} bytes, sha256 ${r.script_sha256.slice(0, 12)}`);
    for (const [k, v] of Object.entries(r.inputs)) console.log(`  input ${k} ${v.slice(0, 12)}`);
    process.exit(0);
  }
  const v = check();
  console.log(v.ok ? 'golden matches' : `GOLDEN DRIFT (${v.reason}): ${v.detail}`);
  process.exit(v.ok ? 0 : 1);
}
