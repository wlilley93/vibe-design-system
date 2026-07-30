'use strict';

/*
 * vds-bridge.js: the OPTIONAL seam between site-factory and VDS.
 *
 * Both directions have to work, and neither may become a dependency of the other:
 *
 *   VDS without site-factory  - already true and untouched by this file. VDS is a
 *       Rust governance kernel with its own CLI, schema and proofs. Nothing here
 *       modifies it, and nothing in it knows this file exists.
 *   site-factory without VDS  - the default. scaffold.js and build.js never call
 *       into here; a project scaffolded without `--vds` has no `.vds/` at all and
 *       builds exactly the same.
 *   site-factory WITH VDS     - this file, invoked only when asked.
 *
 * What it fixes: `vds init` writes a Next.js-shaped default surface
 * (`app/**\/page.tsx`, `src/components/ui`, tsx/jsx, `app/globals.css`). A
 * site-factory project has none of those, so the `.vds/` it produced was PRESENT
 * BUT BLIND - measured, not assumed: 3 proofs precondition-failed on a missing
 * ledger and the rest came back `rows_considered: 0` / vacuous, with `vds doctor`
 * reporting 0 of the then-10 kinds valid. A gate that cannot fail is not a gate.
 *
 * Pointed at the real surface (`blocks/`, `js`, `dist/${SITE_CSS}`) the same proofs
 * bite: `reconciliation` returned `status: failed` naming all 8 shipped blocks as
 * ungoverned. That failure is the correct genesis state, so this file also emits
 * the register records that let it pass legitimately rather than leaving a red
 * gate nobody can green.
 *
 * Why the records are written here and not by `vds register import`: VDS's own
 * importer looks for "an export whose name starts with a capital, or a default
 * export" (React convention) and skipped all 8 blocks, because a block exports
 * `module.exports = {'hero-1': fn, 'hero-2': fn}`. Rather than bend VDS to know
 * about site-factory, site-factory - which knows its own blocks exactly - writes
 * the records in VDS's published schema.
 */

const fs = require('fs');
const path = require('path');
const { execFileSync } = require('child_process');

/*
 * Find the `vds` binary without assuming site-factory lives inside the VDS repo.
 *
 * Order: an explicit VDS_BIN override, then PATH (an installed kernel), then the
 * sibling build inside this repo. Returns null when VDS is simply not present,
 * which is not an error - site-factory runs perfectly well without it, and that is
 * half the requirement.
 */
function resolveVdsBin() {
  if (process.env.VDS_BIN && fs.existsSync(process.env.VDS_BIN)) return process.env.VDS_BIN;
  try {
    const onPath = execFileSync('sh', ['-c', 'command -v vds'], { encoding: 'utf8' }).trim();
    if (onPath) return onPath;
  } catch { /* not on PATH; fall through */ }
  for (const rel of [['..', 'target', 'release', 'vds'], ['..', 'target', 'debug', 'vds']]) {
    const candidate = path.join(__dirname, ...rel);
    if (fs.existsSync(candidate)) return candidate;
  }
  return null;
}

// The real component sets built in the VDS Site Builder Figma file earlier in this
// program of work. These are observed node ids, not placeholders - each is a
// 2-variant component set bound to the shared `VDS Tokens` collection.
const FIGMA_FILE_KEY = '4pPUFvaPdqYzPquBusSfWl';
const FIGMA_NODES = {
  hero: '9:25',
  footer: '10:28',
  nav: '15:18',
  pricing: '15:64',
  testimonials: '15:80',
  features: '15:107',
  faq: '16:21',
  cta: '16:35',
  sidebar: '16:54',
  team: '16:82',
  contact: '16:107',
  notfound: '16:119',
  // The five SaaS app components, built in code and in Figma together. Master-Detail
  // is an assembly in both places: the Figma set is made of INSTANCES of the other
  // four, exactly as masterdetail.js calls their render functions.
  facetstrip: '28:29',
  objecttable: '28:71',
  objectview: '29:50',
  inspector: '29:79',
  masterdetail: '31:138',
  // Chosen by measured demand across 217 Opbox routes, built in code and Figma in the
  // same pass. Adding a block type without its component set silently divides the bank
  // - the register record gets `figma: null` and nobody notices.
  formfield: '36:189',
  emptystate: '36:204',
  pagestate: '37:143',
  confirmdialog: '37:165',
  // Third measured tier, re-counted at 224 routes (the repo grew from 217): toast 19,
  // segmentedcontrol 13, card 12. property-list (54) and page-header (48) score higher
  // and are deliberately NOT here - inspector already renders row/label/value, and
  // objectview already renders title+actions. Building them again would have split one
  // component across two names, which is the drift the register exists to catch.
  toast: '39:139',
  segmentedcontrol: '39:161',
  card: '39:178',
};

/*
 * The other half of the code<->Figma pairing, and the half that was missing.
 *
 * FIGMA_NODES pairs block types to component sets, so a block that ships without a
 * drawing fails a test. Nothing paired the TOKENS. The Figma file carries a
 * `VDS Tokens` variable collection whose modes are meant to BE the style packs in
 * `tokens/`, and it had silently diverged in two independent ways at once:
 *
 *   - four variables the code emits had no variable at all (`dangerInk`,
 *     `warningInk`, `successInk`, `infoInk`), so a drawing that wanted an ink per
 *     tone could not ask for one. MEASURED by a builder script that filtered tones
 *     to those whose variables resolved: it silently drew ONE tone of four and
 *     reported success, which is the failure mode this file exists to prevent.
 *   - nine mode values had drifted from the pack that defines them. The Base
 *     palette measurement (`vendor/uber-base-keys.json`) landed in `tokens/*.json`
 *     and never reached Figma, so `color/danger` in the Geist mode was `#fc0035`
 *     while the shipped CSS said `#de1135`. Every component drawn against it was
 *     the wrong red, and no gate could tell.
 *
 * MODES maps a Figma mode name onto the pack file that DEFINES it: the code is the
 * source of truth and Figma follows, same direction as FIGMA_NODES. A pack with no
 * mode is declared unbound WITH ITS REASON rather than omitted, because a manifest
 * that lists only what it checks cannot be audited for what it skips.
 */
const FIGMA_VARIABLE_MODES = {
  Placeholder: 'placeholder',
  Geist: 'geist',
};

// Packs with no Figma mode. Figma caps a collection's modes by plan tier, and this
// file's collection already carries the two that the drawing work uses. These two are
// exercised through `build.js` and the contrast floor, not through the Figma file.
const FIGMA_UNBOUND_PACKS = {
  balmoral: 'a client brand, not a factory default; nothing is drawn in it',
  jellytot: 'a client brand, not a factory default; nothing is drawn in it',
};

// Custom properties the collection deliberately does not carry. Figma variables are
// used by the DRAWINGS, and a drawing binds fills and corner radii; it has no use for
// a shadow recipe or a font stack. Anything outside this prefix set is out of scope.
const FIGMA_VARIABLE_PREFIXES = ['--color-', '--radius-'];

// The surface a site-factory project actually has. Every value here is a path that
// exists in a scaffolded project, which is the whole point of the file.
const { SITE_CSS } = require('./build.js');

const SURFACE = {
  screen_globs: '["manifests/*.json"]',
  library_dirs: '["blocks"]',
  component_extensions: '["js"]',
  // The ONE stylesheet the whole site shares. Renaming home.css to site.css took the
  // contrast proof offline: it REFUSED and ran nothing, with the right reason - "a caller
  // told that every boundary clears its floor, about a stylesheet that was never opened,
  // has been told nothing." Read off build.js so the two cannot disagree again.
  stylesheet: `"dist/${SITE_CSS}"`,
};

function nowStamp() {
  return new Date().toISOString().replace(/\.\d{3}Z$/, 'Z');
}

/*
 * Rewrite the `[surface]` keys `vds init` defaulted to a Next.js shape.
 *
 * Each replacement is ASSERTED. A silent no-op here is the exact failure this file
 * exists to remove: the config would still parse, `vds proof` would still exit 0 on
 * a vacuous run, and the project would look governed while being blind. So a key
 * that did not match is a thrown error, not a warning.
 */
function writeConfig(outDir) {
  const configPath = path.join(outDir, '.vds', 'config.toml');
  if (!fs.existsSync(configPath)) {
    throw new Error(`no .vds/config.toml in ${outDir} - run "vds init" there first`);
  }
  let toml = fs.readFileSync(configPath, 'utf8');
  const changed = [];
  for (const [key, value] of Object.entries(SURFACE)) {
    const re = new RegExp(`^${key} = .*$`, 'm');
    if (!re.test(toml)) {
      throw new Error(
        `.vds/config.toml has no "${key}" line to rewrite. vds init's template changed shape; ` +
        `refusing to leave a half-corrected surface, because a config that still points at ` +
        `app/**/page.tsx in a project with no app/ produces vacuous proofs that read as passes.`
      );
    }
    toml = toml.replace(re, `${key} = ${value}`);
    changed.push(key);
  }

  // permit_required names the enforcement surface. The default lists Next.js paths
  // this project does not have; point it at what this project actually ships.
  // Asserted for the same reason the surface keys are: a quiet no-op here leaves the
  // enforcement surface naming files that do not exist, which reads as configured.
  const permitRe = /permit_required = \[[\s\S]*?\]/m;
  if (!permitRe.test(toml)) {
    throw new Error(
      '.vds/config.toml has no permit_required block to rewrite. vds init\'s template changed ' +
      'shape; refusing to leave the enforcement surface pointing at paths this project does not have.'
    );
  }
  toml = toml.replace(
    permitRe,
    [
      'permit_required = [',
      // A TEMPLATE literal, not a single-quoted one. This line said "dist/home.css" while
      // the build wrote site.css, so VDS S-3(8)'s enforcement surface - the thing a permit
      // is supposed to protect from being edited - named a file no scaffolded project has.
      // Then the first fix put ${SITE_CSS} inside single quotes, where it is six literal
      // characters and not an interpolation. Both failures are silent: the config parses,
      // the proofs run, and the protected surface is simply not the one that ships.
      `  "dist/${SITE_CSS}",`,
      '  "blocks/**",',
      '  "build.js",',
      '  ".vds/register/**",',
      '  ".vds/config.toml",',
      ']',
    ].join('\n')
  );

  fs.writeFileSync(configPath, toml);
  return { configPath, changed };
}

/*
 * Measure demand: how many manifests in this project reference each block type.
 *
 * VDS S-5(7) requires demand to be MEASURED and carries the command that measured
 * it in the value, so this counts real manifest entries rather than estimating.
 */
function measureDemand(outDir) {
  const manifestsDir = path.join(outDir, 'manifests');
  const counts = {};
  if (!fs.existsSync(manifestsDir)) return counts;
  for (const file of fs.readdirSync(manifestsDir)) {
    if (!file.endsWith('.json')) continue;
    const manifest = JSON.parse(fs.readFileSync(path.join(manifestsDir, file), 'utf8'));
    const seen = new Set((manifest.page || []).map((e) => e.block));
    for (const block of seen) counts[block] = (counts[block] || 0) + 1;
  }
  return counts;
}


/*
 * The states a block ACTUALLY renders, read out of its source.
 *
 * `required: []` made the `states` proof vacuous on every generated project: 8 rows
 * considered, 0 enforced, `record_declares_no_required_state`. A proof that considers
 * every row and enforces none is switched off, and it reports a pass.
 *
 * The line the register note draws is right, though - "required states are decisions
 * nobody has made yet" - so this does NOT invent them. A state the code renders
 * CONDITIONALLY is a decision already made and shipped; deriving it is reading the
 * artefact, which is what every other field here does.
 *
 * Conditional is the whole test, and it is what separates a state from a variant.
 * `pagestate--error` is flat: it is variant 2 of 2, a different function. `field--invalid`
 * sits behind `f.error ? ... : ''`, so one component renders with and without it. Treat
 * the flat ones as states and every two-variant block would claim a spurious pair.
 *
 * This is a FLOOR, not a ceiling. Hover, focus and disabled may also be required and
 * nothing here can know that - the note on each record says so.
 */
// The RIGHT-HAND side is VDS's vocabulary, not the stylesheet's. VDS accepts exactly
// default | hover | focus | active | selected | disabled | loading | error | success,
// and refuses a record naming anything else - it did, on `on` and `invalid`, and
// refused to run rather than skipping the row.
//
// That refusal is the useful part. The CSS says `--on`, `--current` and `--invalid` for
// what are three instances of two governed states, and mapping them here is a design
// system doing its job: the block keeps the class name that reads well in markup, the
// register speaks the one vocabulary every proof can reason about.
const STATE_MARKERS = [
  [/--on\b|--current\b|--selected\b/, 'selected'],
  [/--active\b/, 'active'],
  [/--invalid\b|aria-invalid/, 'error'],
  [/--disabled\b|aria-disabled/, 'disabled'],
];

function deriveStates(source, block, drawnMap) {
  const required = [];
  for (const line of source.split('\n')) {
    // A conditional: a ternary, a short-circuit, or an if. Comments cannot qualify.
    if (line.trim().startsWith('*') || line.trim().startsWith('//')) continue;
    if (!/\?|&&|\bif\b/.test(line)) continue;
    for (const [re, name] of STATE_MARKERS) {
      if (re.test(line) && !required.includes(name)) required.push(name);
    }
  }
  // `drawn` is MEASURED from the Figma file (figma-states.json), never assumed. A state
  // required and not drawn is a real finding and the `states` proof is the gate for it;
  // claiming it drawn to make the gate quiet is the defect the gate exists to catch.
  const drawnFor = (drawnMap || {})[block] || {};
  const drawn = required.filter((st) => Object.prototype.hasOwnProperty.call(drawnFor, st));
  return { required, drawn };
}

function nextIdNumber(registerDir) {
  if (!fs.existsSync(registerDir)) return 1;
  let highest = 0;
  for (const file of fs.readdirSync(registerDir)) {
    const m = file.match(/^CMP-(\d{4})\.yaml$/);
    if (m) highest = Math.max(highest, parseInt(m[1], 10));
  }
  return highest + 1;
}

function yamlList(items, indent) {
  if (!items.length) return ' []';
  return '\n' + items.map((i) => `${indent}- ${i}`).join('\n');
}

/*
 * One record per block type this project actually copied.
 *
 * Deliberately at `proposed` with EMPTY required states and EMPTY keyboard: those
 * are the contract, and the contract is a decision nobody has made yet. VDS's own
 * importer takes the same line, and its note says why - a register filled in from
 * the code "describes the code rather than what it must do, and a register that
 * describes the code cannot disagree with it."
 *
 * The one exception is a single contrast floor, and it is a genuine REQUIREMENT
 * rather than a reading of the code: body ink must clear WCAG AA against the page
 * ground. Every block renders text on that ground, and the reason it is worth
 * asserting is that this is a reskinning factory - swapping a style pack is exactly
 * the move that can drop text below legibility, and this floor is what makes that
 * swap fail a gate instead of shipping. It names both ends BY TOKEN NAME, never a
 * value, which is what `no_stored_values` requires (VDS S-2(2)). It is recorded as
 * an amendment with its basis so a reviewer can contest it.
 */
function writeRegister(outDir, blockTypes) {
  const registerDir = path.join(outDir, '.vds', 'register');
  fs.mkdirSync(registerDir, { recursive: true });
  const demand = measureDemand(outDir);
  const stamp = nowStamp();
  // Measured evidence of which states the Figma file actually draws. Read once here
  // rather than per record, and tolerated as absent: a project scaffolded without it
  // declares nothing drawn, which understates rather than overstates.
  let drawnMap = {};
  try {
    drawnMap = JSON.parse(fs.readFileSync(path.join(__dirname, 'figma-states.json'), 'utf8')).drawn || {};
  } catch { /* no measurement on file; every state reports as not drawn */ }
  let n = nextIdNumber(registerDir);
  const written = [];

  for (const type of blockTypes) {
    const id = `CMP-${String(n).padStart(4, '0')}`;
    n++;
    const name = type.charAt(0).toUpperCase() + type.slice(1);
    const node = FIGMA_NODES[type];
    const blockSrc = fs.readFileSync(path.join(outDir, 'blocks', `${type}.js`), 'utf8');
    const st = deriveStates(blockSrc, type, drawnMap);
    const figmaBlock = node
      ? [
          'figma:',
          `  fileKey: ${FIGMA_FILE_KEY}`,
          `  nodeId: '${node}'`,
          `  capturedAt: ${stamp}`,
        ].join('\n')
      : 'figma: null';

    const record = [
      `id: ${id}`,
      `name: ${name}`,
      'status: proposed',
      'contractVersion: 1',
      figmaBlock,
      'code:',
      `  importPath: ./blocks/${type}`,
      `  sourceFile: blocks/${type}.js`,
      `  exportName: ${type}`,
      'props:',
      '- name: content',
      '  type: object',
      '  required: true',
      '  figmaProperty: null',
      'states:',
      '  required:' + yamlList(['default', ...st.required], '  '),
      '  drawn:' + yamlList(['default', ...st.drawn], '  '),
      '  built:' + yamlList(['default', ...st.required], '  '),
      'a11y:',
      '  role: null',
      '  accessibleNameSource: none_decorative',
      '  keyboard: []',
      '  contrastFloors:',
      '  - boundary: color-ink',
      '    against: color-bg',
      '    minRatio: 4.5',
      '    basis: WCAG 2.2 SC 1.4.3 contrast minimum',
      '    scope: text',
      'demand:',
      `  routes: ${demand[type] || 0}`,
      `  measuredAt: ${stamp}`,
      '  measuredBy: node vds-bridge.js measure-demand',
      'supersedes: []',
      'supersededBy: null',
      'amendments:',
      `- at: ${stamp}`,
      '  by: site-factory',
      '  kind: non_breaking',
      "  what: 'the body-ink floor: text must clear WCAG AA against the page ground in every style pack, so a reskin that drops text below legibility fails a gate rather than shipping'",
      '  contractVersion: 1',
      'basis:',
      '- ACT-VDS-001:s5',
      `notes: 'Written by site-factory''s vds-bridge from blocks/${type}.js. A CANDIDATE, not a contract: required states, role and keyboard are decisions nobody has made yet. Its one contrast floor IS asserted, and is contestable. vds register import cannot see this file, because a block exports a lowercase kebab-keyed object rather than a capitalised or default export.'`,
      '',
    ].join('\n');

    const file = path.join(registerDir, `${id}.yaml`);
    fs.writeFileSync(file, record);
    written.push({ id, type, routes: demand[type] || 0, figmaNode: node || null });
  }

  return written;
}

/*
 * Advance each record proposed -> designed -> registered -> built, one step at a
 * time through VDS's own `register set-status`, which enforces the directed path
 * (VDS S-5(4)) rather than letting this file write a status straight into the YAML.
 *
 * `built` is a fact the factory can vouch for: the code file exists, its Figma
 * component set exists, and it is wired into a manifest that compiles. `verified`
 * is NOT claimed - that needs green proofs, and claiming it here would be the
 * overclaim the whole lifecycle exists to prevent.
 *
 * It matters because a `proposed` record ships nothing by construction, so the
 * contrast floor stays unenforceable and the gate reports `vacuous` - measured:
 * `rows_considered: 8, rows_enforced: 0, proposed_nothing_shipped_by_construction: 8`.
 * At `built` the same run reads the real stylesheet and enforces all 8.
 */
function advanceToBuilt(outDir, ids, vdsBin) {
  const bin = vdsBin || resolveVdsBin();
  if (!bin) return { advanced: [], skipped: 'no vds binary found (set VDS_BIN, or put vds on PATH)' };
  const advanced = [];
  for (const id of ids) {
    for (const status of ['designed', 'registered', 'built']) {
      try {
        execFileSync(bin, ['register', 'set-status', id, status], { cwd: outDir, stdio: 'pipe' });
      } catch (err) {
        return { advanced, failedAt: `${id} -> ${status}`, error: err.message };
      }
    }
    advanced.push(id);
  }
  return { advanced };
}

/*
 * Regenerate the screens ledger. `register_completeness`, `reconciliation` and
 * `composition` all precondition-fail without it (VDS S-4(2)), so a bridge that
 * skipped this would leave three gates unable to run at all.
 */
function refreshLedger(outDir, vdsBin) {
  const bin = vdsBin || resolveVdsBin();
  if (!bin) return false;
  try {
    execFileSync(bin, ['ledger', 'screens'], { cwd: outDir, stdio: 'pipe' });
    return true;
  } catch {
    return false;
  }
}

function bridge(outDir, blockTypes, opts = {}) {
  const cfg = writeConfig(outDir);
  const records = writeRegister(outDir, blockTypes);
  const bin = resolveVdsBin();
  const ledger = refreshLedger(outDir, bin);
  const lifecycle = opts.advance === false
    ? { advanced: [], skipped: 'advance disabled by caller' }
    : advanceToBuilt(outDir, records.map((r) => r.id), bin);
  return { config: cfg, records, vdsBin: bin, ledger, lifecycle };
}

module.exports = {
  deriveStates,
  bridge, writeConfig, writeRegister, measureDemand, advanceToBuilt, refreshLedger,
  resolveVdsBin, SURFACE, FIGMA_NODES, FIGMA_FILE_KEY,
  FIGMA_VARIABLE_MODES, FIGMA_UNBOUND_PACKS, FIGMA_VARIABLE_PREFIXES,
};
