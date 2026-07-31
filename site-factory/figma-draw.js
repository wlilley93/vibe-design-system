'use strict';

/*
 * figma-draw.js: redraw the component library from the register, deterministically.
 *
 * BREACH-0010's remedy. All 46 component sets in the master file were drawn by scripts
 * written inside agent turns, run once, and never committed - a grep for
 * `createComponentSet`, `combineAsVariants` or `createComponent(` across the whole
 * repository returned zero hits. So `figma-nodes.json` could prove the sets EXIST and
 * pair correctly, and nothing could reproduce them. Delete a set and the only recovery
 * was to write a new script by hand.
 *
 * The shape follows `figma-push.js`, which is the committed precedent: this file BUILDS a
 * `use_figma` script and does not run one. `use_figma` is reachable only from inside an
 * agent turn or through the REST API with a real credential, so the division is that the
 * factory emits the script and an agent executes it.
 *
 * THREE THINGS THIS FILE IS DELIBERATELY NOT
 *
 * 1. It is not a renderer. It draws a NAMED, TYPED PLACEHOLDER per variant - the block
 *    type, the axis value, and the tokens the variant is bound to - not an approximation
 *    of the block's markup. A half-drawn approximation of a hero is worse than an honest
 *    frame, because it looks like the design and is not; and [2026] VJS-CC-OPBOX 3 D1
 *    makes the Figma file the system of record for what is DECIDED, so a generator that
 *    invented visual decisions would be the fourth authority that order forbids.
 * 2. It carries no realisation. Every fill and radius is applied by BINDING a Figma
 *    variable by name (`color/bg`, `radius/lg`), never by writing a value. VDS S-2(4)
 *    forbids a realisation in a governance artefact and this script is derived from one.
 *    It also means a redraw cannot fight the token collection: if a variable is missing
 *    the script REFUSES that binding by name rather than falling back to a literal.
 * 3. It does not decide the axis. The axis and its values are read from
 *    `figma-variants.json`, which was measured from the live file, so a redraw reproduces
 *    the axis that is already there rather than minting a new one.
 *
 * AMEND IN PLACE, NEVER DESTROY AND RECREATE
 *
 * Every set is stamped with `setSharedPluginData('vds', 'componentId', <CMP-id>)` and
 * `blockType`. A second run FINDS the stamped set and rebuilds each variant's INTERIOR in
 * place, keeping the set's node id, its component key and its property ids - so
 * `figma-nodes.json` stays valid and any placed instance keeps its overrides.
 *
 * That is the whole reason this file is longer than "draw the sets". A redraw that deleted
 * and recreated would invalidate all 43 node ids in one command, which is to say it would
 * break the pairing manifest that is the only evidence the two sides agree. A generator
 * that destroys the record of its own correctness is not a remedy.
 *
 * WHAT AMENDING CANNOT DO, stated rather than discovered: it matches variants BY NAME. If
 * the axis itself moved - `Style` became `Layout`, or a value was renamed - the names no
 * longer line up and the script REFUSES that set by name instead of guessing which old
 * variant became which new one. A wrong guess silently rewrites a component into a
 * different one. Handling that case deliberately and by hand is correct: changing a
 * variant axis is a breaking amendment under VDS S-9(2) and must not happen as a side
 * effect of running a generator.
 *
 * This is the one mechanism worth taking from southleft/ds-contracts-poc, whose plugin
 * does the same thing for the same reason.
 *
 * DETERMINISM
 *
 * `buildLibraryScript` is a pure function of (register, variants). Same inputs, same
 * bytes - asserted by a test rather than hoped for, because a generator whose output
 * moves on its own cannot be diffed, and a diff is the only way a reader can see what a
 * redraw would change before it changes it.
 */

const FIGMA_FILE_KEY = '4pPUFvaPdqYzPquBusSfWl';

// The page the generated sets live on. A redraw touches this page and no other, so a
// script that goes wrong cannot take the Uber Base catalogue or the prompts page with it.
const LIBRARY_PAGE = 'Library (generated)';

// Grid geometry. Fixed rather than computed from content, because a set whose position
// depends on its neighbours' sizes moves every sibling when one variant gains a line.
const CELL_W = 320;
const CELL_H = 180;
const GAP = 48;
const COLS = 4;

/*
 * The stable identity for a block type: its 1-based position in the FULL sorted list of
 * every block the measurement knows about.
 *
 * Stable under batching, which is the whole point, and stable under a block being ADDED
 * only if the addition sorts after it - which it will not always. That is a real limit and
 * it is stated rather than hidden: adding `accordion` would shift every id after it, and
 * the correct response is to re-stamp the file, not to renumber quietly. A register that
 * allocated ids once and stored them would not have the problem at all, and that is what
 * VDS's own `ComponentId::allocate` does; this function exists because site-factory mints
 * a register per project rather than keeping one.
 */
function canonicalId(blockType, variants) {
  const all = Object.keys(variants.blocks).sort();
  const i = all.indexOf(blockType);
  return i < 0 ? null : `CMP-${String(i + 1).padStart(4, '0')}`;
}

/*
 * One drawable spec per registered block: what to draw, and which variables to bind.
 *
 * `bindings` names Figma variables, never values. A name that does not resolve in the
 * file is reported by the script as a refusal rather than silently skipped - the same
 * lesson as the tones filter that drew one variant of four and reported success.
 */
function specsFor(register, variants) {
  // THE IDENTITY MUST BE DERIVED FROM THE THING, NOT FROM ITS POSITION IN A CALL.
  //
  // This is checked here rather than trusted, because the first sweep got it wrong and the
  // consequence was severe. A caller minted ids from each record's index in whatever list
  // it happened to pass, so a three-block slice made hero=CMP-0001, divider=CMP-0002,
  // banner=CMP-0003, and the next batch - indexing a forty-block list - made card=CMP-0002
  // and checkbox=CMP-0003. Two components inherited two other components' identities.
  //
  // The amend path refused both by name and nothing was overwritten, which is the guard
  // doing exactly its job. But a guard is the last line, not the fix: the ids should never
  // have collided. `canonicalId` derives one from the block type's position in the FULL
  // sorted block list, so the same block yields the same id from any call, in any batch,
  // in any order.
  const known = Object.keys(variants.blocks).sort();
  for (const record of register) {
    const want = canonicalId(record.blockType, variants);
    if (want && record.id !== want) {
      throw new Error(
        `${record.blockType} was passed as ${record.id} and its canonical id is ${want}. ` +
        'A component id derived from a position in a call is not an identity: the same ' +
        'block gets a different id in a different batch, and the stamp then names another ' +
        "component's set. Use canonicalId().",
      );
    }
  }
  void known;

  const specs = [];
  for (const record of register) {
    const type = record.blockType;
    const measured = variants.blocks[type];
    if (!measured) continue;

    // ONE AXIS IS DRAWN, AND THE REST ARE NOW NAMED RATHER THAN DROPPED.
    //
    // This was `Object.keys(measured.axes)[0]` and nothing else. Two of the 43
    // blocks carry TWO measured axes - `pagination` is Layout x State and
    // `progressbar` is Size x Layout - so the generated set is a PROJECTION of
    // the measured one onto its first axis, and no artefact said so. An
    // independent read of the live file found it: 41 of 43 blocks agree with
    // `figma-variants.json` on their axis names and two do not, because the
    // drawn set has fewer axes than the measurement it was drawn from.
    //
    // The module comment above says this generator "does not decide the axis",
    // which was true of the axis it picked and silent about the one it did not.
    // A generator that narrows its input silently is the shape BREACH-0010 was
    // filed for wearing different clothes: the output looks derived because it
    // IS derived, and the derivation quietly lost something.
    //
    // Not a throw. Refusing would leave two blocks undrawn, and an
    // undrawn block is worse than a projected one - what was missing was the
    // RECORD, so the record is what this adds. `droppedAxes` travels into the
    // emitted script's return value and into the census beside it.
    const axisNames = Object.keys(measured.axes);
    const axis = axisNames[0];
    const values = measured.axes[axis];
    const droppedAxes = axisNames.slice(1);
    specs.push({
      componentId: record.id,
      blockType: type,
      name: record.name,
      nodeId: measured.nodeId,
      axis,
      droppedAxes,
      // The variant list is what the FILE draws, so a redraw reproduces the set that is
      // there. Taking the code's variant count instead would silently drop the four Tone
      // values `banner` draws and the code takes as content.
      variants: values.map((value) => ({
        value,
        label: `${axis}=${value}`,
      })),
      // A binding is a NAME, and the names are the ones `figma-variables.json` measured
      // in the file - not the ones the CSS uses. The first run asked for `color/rule`,
      // which is what the stylesheet calls it; the collection calls it `color/border`,
      // and the script reported `no variable named color/rule` rather than drawing an
      // edgeless frame and returning success. That refusal is the mechanism working: a
      // filter that silently dropped three of four tones once reported a clean run.
      bindings: { fill: 'color/bg', stroke: 'color/border', text: 'color/ink' },
      radius: 'radius/lg',
    });
  }
  specs.sort((a, b) => a.componentId.localeCompare(b.componentId));
  return specs;
}

/*
 * The `use_figma` script.
 *
 * Written as one template with the data injected as JSON, exactly as `figma-push.js`
 * does, so the executable text is fixed and only the data varies. A script assembled by
 * string concatenation per block would be a different program every run and could not be
 * diffed or golden-filed.
 *
 * The Figma Plugin API traps encoded here, each learned by hitting it:
 *   - `page.children` is EMPTY until `await page.loadAsync()` in dynamic-page mode. A
 *     survey that skipped it once reported seven of ten pages empty and read as a wipe.
 *   - assigning `layoutMode` and calling `resize()` each RESET both sizing modes, so the
 *     order is layoutMode, then resize, then sizing modes. Reversed, frames pin at 100px.
 *   - sizing modes are named for the AXIS, not the dimension: on a HORIZONTAL frame,
 *     `primaryAxisSizingMode` is the WIDTH.
 *   - `layoutSizingHorizontal = 'FILL'` throws unless the node already has a parent.
 *   - a throw rolls back the whole script, so every refusal is COLLECTED and returned
 *     rather than thrown: a partial redraw that reported success would be the worst
 *     outcome available.
 */
function buildLibraryScript(register, variants, opts = {}) {
  const specs = specsFor(register, variants);
  // The grid index this batch STARTS at. Without it every batch computes its column and
  // row from its own 0, so batch two draws on top of batch one - which the amend path
  // would then not even notice, because amend matches on the identity stamp and never on
  // position. A sweep in batches needs the grid to continue, not restart.
  const startIndex = opts.startIndex || 0;
  const data = JSON.stringify({
    pageName: LIBRARY_PAGE,
    cellW: CELL_W,
    cellH: CELL_H,
    gap: GAP,
    cols: COLS,
    startIndex,
    specs,
  });

  return `
const D = ${data};

// Resolve every variable ONCE, by name, and record what did not resolve. A binding that
// silently no-ops is how everything got drawn in the wrong red before anyone noticed.
const collections = await figma.variables.getLocalVariableCollectionsAsync();
const allVars = await figma.variables.getLocalVariablesAsync();
const byName = new Map(allVars.map((v) => [v.name, v]));
const refusals = [];
function variable(name) {
  const v = byName.get(name);
  if (!v) { refusals.push('no variable named ' + name); return null; }
  return v;
}
function bind(node, field, name) {
  const v = variable(name);
  if (!v) return false;
  if (field === 'fills' || field === 'strokes') {
    const paint = { type: 'SOLID', color: { r: 0, g: 0, b: 0 } };
    node[field] = [figma.variables.setBoundVariableForPaint(paint, 'color', v)];
  } else {
    node.setBoundVariable(field, v);
  }
  return true;
}

// The page. Found by name or created, and LOADED before its children are read.
let page = figma.root.children.find((p) => p.name === D.pageName);
if (!page) { page = figma.createPage(); page.name = D.pageName; }
await page.loadAsync();

// Index what is already stamped, so a redraw AMENDS rather than duplicates.
const stamped = new Map();
for (const node of page.children) {
  if (node.type !== 'COMPONENT_SET') continue;
  const id = node.getSharedPluginData('vds', 'componentId');
  if (id) stamped.set(id, node);
}

const drawn = [];
const amended = [];
const skipped = [];

// One variant's interior, shared by the create and the amend paths so the two cannot
// drift. A redraw that produced a different interior from a fresh draw would make
// "amended" and "drawn" mean different things.
async function fillInterior(component, spec, label) {
  for (const child of [...component.children]) child.remove();

  // ORDER: layoutMode, then resize, then sizing modes. Each of the first two RESETS both
  // sizing modes, so setting them earlier is setting them and losing them.
  component.layoutMode = 'VERTICAL';
  component.resize(D.cellW, D.cellH);
  component.primaryAxisSizingMode = 'FIXED';
  component.counterAxisSizingMode = 'FIXED';
  component.paddingLeft = 16; component.paddingRight = 16;
  component.paddingTop = 16; component.paddingBottom = 16;
  component.itemSpacing = 8;
  bind(component, 'fills', spec.bindings.fill);
  component.strokeWeight = 1;
  bind(component, 'strokes', spec.bindings.stroke);
  const r = variable(spec.radius);
  if (r) {
    for (const corner of ['topLeftRadius', 'topRightRadius', 'bottomLeftRadius', 'bottomRightRadius']) {
      component.setBoundVariable(corner, r);
    }
  }

  const title = figma.createText();
  title.fontName = { family: 'Inter', style: 'Semi Bold' };
  title.characters = spec.name;
  title.fontSize = 16;
  component.appendChild(title);
  // FILL only AFTER appendChild: it throws on a parentless node.
  title.layoutSizingHorizontal = 'FILL';
  bind(title, 'fills', spec.bindings.text);

  const sub = figma.createText();
  sub.fontName = { family: 'Inter', style: 'Regular' };
  sub.characters = label + '\\n' + spec.componentId + '  \\u00b7  ' + spec.blockType;
  sub.fontSize = 12;
  component.appendChild(sub);
  sub.layoutSizingHorizontal = 'FILL';
  bind(sub, 'fills', spec.bindings.text);
  sub.opacity = 0.6;
}

await figma.loadFontAsync({ family: 'Inter', style: 'Semi Bold' });
await figma.loadFontAsync({ family: 'Inter', style: 'Regular' });

for (let i = 0; i < D.specs.length; i++) {
  const spec = D.specs[i];
  const g = D.startIndex + i;
  const col = g % D.cols;
  const row = Math.floor(g / D.cols);
  const originX = col * (D.cellW + D.gap) * 2;
  const originY = row * (D.cellH + D.gap) * 3;
  const prior = stamped.get(spec.componentId);

  if (prior) {
    // AMEND IN PLACE, matching by variant name. Refuse the whole set if the names moved.
    const byLabel = new Map(prior.children.map((c) => [c.name, c]));
    const wanted = spec.variants.map((v) => v.label);
    const missing = wanted.filter((w) => !byLabel.has(w));
    const extra = prior.children.map((c) => c.name).filter((n) => !wanted.includes(n));
    if (missing.length || extra.length) {
      skipped.push(spec.componentId + ': variant names moved (missing ' +
        JSON.stringify(missing) + ', extra ' + JSON.stringify(extra) +
        '). Changing a variant axis is a breaking amendment, not a generator side effect.');
      continue;
    }
    for (const v of spec.variants) {
      await fillInterior(byLabel.get(v.label), spec, v.label);
    }
    prior.name = spec.name;
    prior.setSharedPluginData('vds', 'blockType', spec.blockType);
    amended.push(spec.componentId);
    continue;
  }

  // CREATE. Only where nothing this generator made is already present.
  const members = [];
  for (let j = 0; j < spec.variants.length; j++) {
    const v = spec.variants[j];
    const component = figma.createComponent();
    component.name = v.label;
    page.appendChild(component);
    await fillInterior(component, spec, v.label);
    component.x = originX;
    component.y = originY + j * (D.cellH + 16);
    members.push(component);
  }
  const set = figma.combineAsVariants(members, page);
  set.name = spec.name;
  set.x = originX;
  set.y = originY;
  // The stamp is the identity. Without it a second run cannot tell its own work from a
  // hand-built set, and would duplicate or overwrite something it did not make.
  set.setSharedPluginData('vds', 'componentId', spec.componentId);
  set.setSharedPluginData('vds', 'blockType', spec.blockType);
  drawn.push(spec.componentId);
}

return {
  page: page.name,
  drawn: drawn.length,
  amended: amended.length,
  sets: drawn.length + amended.length,
  skipped,
  refusals: [...new Set(refusals)],
  // Every set drawn as a PROJECTION of a multi-axis measurement onto one axis.
  // Returned rather than left to be inferred: a run that reports 43 sets drawn
  // and says nothing about this reads as a faithful reproduction of 43
  // measured sets, and for two of them it is not.
  projectedOntoOneAxis: D.specs
    .filter((s) => (s.droppedAxes || []).length)
    .map((s) => s.blockType + ' drew ' + s.axis + ', not also ' + s.droppedAxes.join('/')),
};
`;
}

module.exports = { buildLibraryScript, specsFor, canonicalId, FIGMA_FILE_KEY, LIBRARY_PAGE };
