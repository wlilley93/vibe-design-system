'use strict';

/*
 * figma-push.js: generate the use_figma script that records a project in Figma.
 *
 * This is NOT something the CLI can run itself - use_figma is reachable only from
 * inside an agent turn, or from the Figma REST API with a personal access token,
 * which is a real credential a user has to provide. So the shape of this step is:
 * the factory writes config.json (every route, every run), and an agent calls
 * use_figma with buildScript(config).
 *
 * Layout is COLUMNS AND ROWS, not a stack of text lines:
 *
 *   shell (HORIZONTAL)
 *     column 1  header · palette swatch row · decision table (LAYER | FIELD | VALUE)
 *     column 2  compiled screenshot slot · gate rows
 *
 * Two things learned building it, both encoded below so the generated script does
 * not repeat them:
 *   - Figma frames default to an OPAQUE WHITE fill, so every container has to have
 *     its fill cleared or the paper ground never shows and the zebra striping reads
 *     as the only background. 16 frames needed clearing on the first build.
 *   - WIDTH_AND_HEIGHT text does not wrap, so long values (modularPlays, sitemap)
 *     overflow the frame. Every cell is FIXED or FILL width with HEIGHT auto-resize.
 */

const VDS_SITE_BUILDER_KEY = '4pPUFvaPdqYzPquBusSfWl';

// Column widths for the decision table, in px. The value column takes the rest.
const W_LAYER = 118;
const W_FIELD = 150;
const COL1 = 720;
const COL2 = 400;

/*
 * Flatten a config into table rows, skipping the identity layer (it is the header)
 * and the palette colour fields (they are the swatch row). Arrays become a
 * comma-joined string; the sitemap gets arrows because it is an ordered sequence.
 */
function configRows(config) {
  const rows = [];
  for (const [layer, fields] of Object.entries(config)) {
    if (layer === 'identity' || layer === 'governance') continue;
    if (!fields || typeof fields !== 'object' || Array.isArray(fields)) continue;
    for (const [key, value] of Object.entries(fields)) {
      if (layer === 'palette' && key !== 'basePack') continue;
      let text;
      if (Array.isArray(value)) text = key === 'sitemap' ? value.join(' → ') : value.join(', ');
      else text = String(value);
      if (text === '') text = '—';
      rows.push([layer, key, text]);
    }
  }
  return rows;
}

/*
 * `gates` is optional and, when given, must be real readings. An unverified green
 * row on a record page is worse than no row: it looks like evidence.
 * Shape: [[name, status], ...]
 */
function buildScript(config, opts = {}) {
  const rows = configRows(config);
  const gates = opts.gates || null;
  const shot = opts.screenshot || null; // {width, height}
  const pageName = `Project: ${config.identity.name}`;

  // Aspect-correct the screenshot slot, or FILL crops it.
  const shotH = shot ? Math.round((shot.height / shot.width) * COL2) : 500;

  const data = JSON.stringify({
    pageName,
    identity: config.identity,
    palette: config.palette,
    governed: Boolean(config.governance && config.governance.vds),
    rows,
    gates,
    shotH,
  });

  return `
const D = ${data};

function hex(h) {
  h = String(h).replace('#','');
  return { r: parseInt(h.slice(0,2),16)/255, g: parseInt(h.slice(2,4),16)/255, b: parseInt(h.slice(4,6),16)/255 };
}
const INK = hex(D.palette.inkColor);
const MUTED = hex(D.palette.borderColor);
const LINE = hex(D.palette.borderColor);
const PAPER = hex(D.palette.groundColor);
const TINT = hex(D.palette.surfaceColor);
const ACCENT = hex(D.palette.accentColor);
const GREEN = { r: 0.18, g: 0.42, b: 0.31 };
const GREY = { r: 0.43, g: 0.41, b: 0.38 };

const page = figma.createPage();
page.name = D.pageName;
await figma.setCurrentPageAsync(page);

for (const s of ['Regular','Medium','Semi Bold','Bold']) {
  await figma.loadFontAsync({ family: 'Inter', style: s });
}

function T(str, size, weight, fill) {
  const t = figma.createText();
  t.fontName = { family: 'Inter', style: weight };
  t.fontSize = size;
  t.characters = String(str);
  t.fills = [{ type: 'SOLID', color: fill || INK }];
  return t;
}
// A cell is FIXED or FILL with HEIGHT auto-resize. Never WIDTH_AND_HEIGHT: that
// does not wrap, and the long values overflow the frame.
function cell(parent, str, size, weight, fill, width) {
  const t = T(str, size, weight, fill);
  parent.appendChild(t);
  if (width) { t.layoutSizingHorizontal = 'FIXED'; t.resize(width, t.height); }
  else t.layoutSizingHorizontal = 'FILL';
  t.textAutoResize = 'HEIGHT';
  return t;
}

const shell = figma.createAutoLayout('HORIZONTAL', {
  name: D.identity.name + ' - project record', itemSpacing: 40,
  paddingTop: 48, paddingBottom: 48, paddingLeft: 48, paddingRight: 48,
});
shell.fills = [{ type: 'SOLID', color: PAPER }];
shell.counterAxisAlignItems = 'MIN';
shell.x = 0; shell.y = 0;

// ---------- column 1 ----------
const col1 = figma.createAutoLayout('VERTICAL', { name: 'Column - decisions', itemSpacing: 24 });
shell.appendChild(col1);
col1.layoutSizingHorizontal = 'FIXED';
col1.resize(${COL1}, col1.height);

const head = figma.createAutoLayout('VERTICAL', { name: 'Header', itemSpacing: 6 });
col1.appendChild(head);
head.layoutSizingHorizontal = 'FILL';
cell(head, D.identity.name, 34, 'Bold', INK);
cell(head, [D.identity.tagline, 'route: ' + D.identity.category, D.governed ? 'governed' : 'ungoverned'].filter(Boolean).join('   ·   '), 13, 'Medium', GREY);
if (D.identity.description) cell(head, D.identity.description, 13, 'Regular', GREY);

const palWrap = figma.createAutoLayout('VERTICAL', { name: 'Palette', itemSpacing: 10 });
col1.appendChild(palWrap);
palWrap.layoutSizingHorizontal = 'FILL';
cell(palWrap, 'PALETTE', 10, 'Semi Bold', GREY);
const palRow = figma.createAutoLayout('HORIZONTAL', { name: 'Swatch row', itemSpacing: 10 });
palWrap.appendChild(palRow);
palRow.layoutSizingHorizontal = 'FILL';
for (const key of ['groundColor','surfaceColor','inkColor','accentColor','accentInkColor','borderColor']) {
  const c = figma.createAutoLayout('VERTICAL', { name: key, itemSpacing: 6 });
  palRow.appendChild(c);
  c.layoutSizingHorizontal = 'FILL';
  const chip = figma.createRectangle();
  chip.resize(10, 52);
  chip.fills = [{ type: 'SOLID', color: hex(D.palette[key]) }];
  chip.strokes = [{ type: 'SOLID', color: LINE }];
  chip.strokeWeight = 1;
  c.appendChild(chip);
  chip.layoutSizingHorizontal = 'FILL';
  cell(c, key.replace('Color',''), 9, 'Medium', GREY);
  cell(c, D.palette[key], 9, 'Regular', GREY);
}

// ---------- the table: rows of three column cells ----------
const table = figma.createAutoLayout('VERTICAL', { name: 'Decision table', itemSpacing: 0 });
col1.appendChild(table);
table.layoutSizingHorizontal = 'FILL';

function row(a, b, c, o) {
  o = o || {};
  const r = figma.createAutoLayout('HORIZONTAL', {
    name: a + '/' + b, itemSpacing: 12,
    paddingTop: 7, paddingBottom: 7, paddingLeft: 10, paddingRight: 10,
  });
  table.appendChild(r);
  r.layoutSizingHorizontal = 'FILL';
  r.counterAxisAlignItems = 'MIN';
  r.fills = o.tint ? [{ type: 'SOLID', color: TINT }] : [];
  r.strokes = [{ type: 'SOLID', color: LINE }];
  r.strokeWeight = 1;
  r.strokeTopWeight = 0; r.strokeLeftWeight = 0; r.strokeRightWeight = 0;
  const w = o.head ? 'Semi Bold' : 'Medium';
  const size = o.head ? 9 : 10.5;
  cell(r, a, size, w, o.head ? GREY : ACCENT, ${W_LAYER});
  cell(r, b, size, w, o.head ? GREY : INK, ${W_FIELD});
  cell(r, c, size, o.head ? 'Semi Bold' : 'Regular', GREY, null);
  return r;
}
row('LAYER', 'FIELD', 'VALUE', { head: true });
D.rows.forEach(function (t, i) { row(t[0], t[1], t[2], { tint: i % 2 === 0 }); });

// ---------- column 2 ----------
const col2 = figma.createAutoLayout('VERTICAL', { name: 'Column - artefact', itemSpacing: 14 });
shell.appendChild(col2);
col2.layoutSizingHorizontal = 'FIXED';
col2.resize(${COL2}, col2.height);

cell(col2, 'COMPILED - dist/home.html', 10, 'Semi Bold', GREY);
const shot = figma.createRectangle();
shot.name = 'Compiled site screenshot';
shot.resize(${COL2}, D.shotH);
shot.fills = [{ type: 'SOLID', color: TINT }];
shot.strokes = [{ type: 'SOLID', color: LINE }];
shot.strokeWeight = 1;
col2.appendChild(shot);
shot.layoutSizingHorizontal = 'FILL';

if (D.gates) {
  cell(col2, 'VDS GATES', 10, 'Semi Bold', GREY);
  const gates = figma.createAutoLayout('VERTICAL', { name: 'Gate rows', itemSpacing: 0 });
  col2.appendChild(gates);
  gates.layoutSizingHorizontal = 'FILL';
  for (const g of D.gates) {
    const r = figma.createAutoLayout('HORIZONTAL', { name: g[0], itemSpacing: 10, paddingTop: 6, paddingBottom: 6 });
    gates.appendChild(r);
    r.layoutSizingHorizontal = 'FILL';
    r.fills = [];
    r.strokes = [{ type: 'SOLID', color: LINE }];
    r.strokeWeight = 1; r.strokeTopWeight = 0; r.strokeLeftWeight = 0; r.strokeRightWeight = 0;
    cell(r, g[0], 10.5, 'Medium', INK, 140);
    cell(r, g[1], 10.5, 'Regular', String(g[1]).startsWith('passed') ? GREEN : ACCENT, null);
  }
}

// Figma frames default to an opaque white fill, which hides the paper ground and
// makes the zebra read as the only background. Clear every container, then re-assert
// the tint the table rows set for themselves.
for (const n of shell.findAll(function (x) { return x.type === 'FRAME'; })) {
  const isDataRow = n.parent && n.parent.name === 'Decision table' && n.name.indexOf('/') !== -1;
  if (!isDataRow && Array.isArray(n.fills) && n.fills.length) n.fills = [];
}

return { pageId: page.id, shellId: shell.id, screenshotNodeId: shot.id, tableRows: D.rows.length };
`;
}

module.exports = { buildScript, configRows, VDS_SITE_BUILDER_KEY };
