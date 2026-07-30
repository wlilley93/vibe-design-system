'use strict';

/*
 * figma-push.js: the use_figma script body for "new project -> a page in Figma".
 *
 * This is NOT something wizard.js can run itself — wizard.js is a plain Node script
 * with no MCP client, and use_figma is only reachable from inside an agent turn (or
 * from a real Figma REST API call with a personal access token, which is a genuine
 * external credential — a user-owned dependency, not something to fabricate here).
 *
 * So the real shape of this step is: wizard.js writes config.json (already does,
 * every route, every run); an agent reads it and calls use_figma with the script
 * this file generates. buildScript(config) returns the exact JS string that was
 * proven working against the VDS Site Builder file (fileKey 4pPUFvaPdqYzPquBusSfWl)
 * for the "Wizard Test Co" project, page id 21:2 — swatches, radius sample,
 * typography/spacing/strategy/sitemap text, all wrapped to the frame width (the
 * first version overflowed using WIDTH_AND_HEIGHT text nodes; this version fixes
 * that with HEIGHT auto-resize + a fixed inner width, verified by screenshot).
 *
 * Usage from an agent turn:
 *   const { buildScript } = require('./figma-push.js');
 *   const config = JSON.parse(fs.readFileSync('scaffolds/<name>/config.json'));
 *   // then call use_figma with code: buildScript(config), fileKey: VDS_SITE_BUILDER_KEY
 */

const VDS_SITE_BUILDER_KEY = '4pPUFvaPdqYzPquBusSfWl';

function buildScript(config) {
  const c = JSON.stringify(config);
  return `
function hex(h) {
  h = h.replace('#','');
  return { r: parseInt(h.slice(0,2),16)/255, g: parseInt(h.slice(2,4),16)/255, b: parseInt(h.slice(4,6),16)/255 };
}

const page = figma.createPage();
page.name = 'Project: ${config.identity.name.replace(/'/g, "\\'")}';
await figma.setCurrentPageAsync(page);

await figma.loadFontAsync({ family: 'Inter', style: 'Regular' });
await figma.loadFontAsync({ family: 'Inter', style: 'Semi Bold' });
await figma.loadFontAsync({ family: 'Inter', style: 'Bold' });

const config = ${c};

function makeText(str, size, weight, fill) {
  const t = figma.createText();
  t.fontName = { family: 'Inter', style: weight };
  t.fontSize = size;
  t.characters = str;
  t.fills = [{ type: 'SOLID', color: fill || { r: 0.1, g: 0.1, b: 0.1 } }];
  t.textAutoResize = 'WIDTH_AND_HEIGHT';
  return t;
}

const root = figma.createAutoLayout('VERTICAL', { name: 'Project Record', itemSpacing: 28, paddingTop: 48, paddingBottom: 48, paddingLeft: 48, paddingRight: 48 });
root.counterAxisSizingMode = 'FIXED';
root.resize(760, root.height);
root.fills = [{ type: 'SOLID', color: hex(config.palette.groundColor) }];
root.strokes = [{ type: 'SOLID', color: hex(config.palette.borderColor) }];
root.strokeWeight = 1;

root.appendChild(makeText(config.identity.name, 32, 'Bold', hex(config.palette.inkColor)));
root.appendChild(makeText(\`\${config.identity.tagline}  ·  route: \${config.identity.category}\`, 14, 'Regular', hex(config.palette.inkColor)));
root.appendChild(makeText(config.identity.description, 13, 'Regular', hex(config.palette.inkColor)));

root.appendChild(makeText('Palette', 16, 'Semi Bold', hex(config.palette.inkColor)));
const paletteRow = figma.createAutoLayout('HORIZONTAL', { name: 'Palette swatches', itemSpacing: 12 });
root.appendChild(paletteRow);
const swatchKeys = ['groundColor','surfaceColor','inkColor','accentColor','accentInkColor','borderColor'];
for (const key of swatchKeys) {
  const col = figma.createAutoLayout('VERTICAL', { name: key, itemSpacing: 4 });
  const sw = figma.createRectangle();
  sw.resize(88, 56);
  sw.fills = [{ type: 'SOLID', color: hex(config.palette[key]) }];
  sw.strokes = [{ type: 'SOLID', color: hex(config.palette.borderColor) }];
  sw.strokeWeight = 1;
  col.appendChild(sw);
  col.appendChild(makeText(\`\${key}\\n\${config.palette[key]}\`, 10, 'Regular', hex(config.palette.inkColor)));
  paletteRow.appendChild(col);
}

root.appendChild(makeText(\`Typography — \${config.typography.displayFont}\\nmono: \${config.typography.monoFont}\\npairing: \${config.typography.pairingStyle}, scale: \${config.typography.typeScale}\`, 12, 'Regular', hex(config.palette.inkColor)));
root.appendChild(makeText(\`Spacing & shape — unit \${config.spacing.spaceUnit}px, density \${config.spacing.density}, radius \${config.spacing.cornerRadius}, border \${config.spacing.borderWeight}, elevation \${config.spacing.elevation}\`, 12, 'Regular', hex(config.palette.inkColor)));

const radiusRow = figma.createAutoLayout('HORIZONTAL', { name: 'Radius sample', itemSpacing: 16 });
root.appendChild(radiusRow);
const rSm = figma.createRectangle(); rSm.resize(56,56); rSm.fills = [{type:'SOLID', color: hex(config.palette.accentColor)}];
const rLg = figma.createRectangle(); rLg.resize(56,56); rLg.fills = [{type:'SOLID', color: hex(config.palette.accentColor)}];
radiusRow.appendChild(rSm); radiusRow.appendChild(rLg);

if (config.strategy) {
  root.appendChild(makeText(\`Product strategies — \${config.strategy.productStrategies.join(', ')}\`, 12, 'Regular', hex(config.palette.inkColor)));
  root.appendChild(makeText(\`Modular plays — \${config.strategy.modularPlays.join(', ')}\`, 12, 'Regular', hex(config.palette.inkColor)));
  root.appendChild(makeText(\`Sitemap — \${config.strategy.sitemap.join(' → ')}\`, 12, 'Regular', hex(config.palette.inkColor)));
}
if (config.componentStyle) {
  root.appendChild(makeText(\`Component style — button: \${config.componentStyle.buttonShape}, table: \${config.componentStyle.tableDensity}, nav: \${config.componentStyle.navigationPattern}, badge: \${config.componentStyle.statusBadgeStyle}\`, 12, 'Regular', hex(config.palette.inkColor)));
}

// WIDTH_AND_HEIGHT text doesn't wrap — every text child overflowed the 760px frame
// in the first version of this script. Re-pass and force HEIGHT + fixed width.
const innerWidth = root.width - root.paddingLeft - root.paddingRight;
for (const child of root.children) {
  if (child.type === 'TEXT') {
    child.textAutoResize = 'HEIGHT';
    child.resize(innerWidth, child.height);
    child.layoutSizingHorizontal = 'FILL';
  }
}

root.x = 0;
root.y = 0;

return { pageId: page.id, rootFrameId: root.id };
`;
}

// Second half of the push: place a real screenshot of dist/home.html beside the
// record frame this file just built. Two-step because upload_assets needs a node id
// that already exists on the page — build the placeholder rect first (this
// function), run it, then call upload_assets with the returned rectId and POST the
// screenshot bytes to the submitUrl it returns. Proven against 'Wizard Test Co':
// the record alone is a claim about what was decided; the screenshot is proof of
// what it actually built. Width/height should be the real screenshot's pixel size
// (e.g. from `identify` or PIL) scaled down by a constant factor so aspect ratio
// matches exactly — mismatched aspect ratio + scaleMode FILL crops the image.
function buildImagePlacementScript(pageId, screenshotWidth, screenshotHeight, scale) {
  const w = Math.round(screenshotWidth * scale);
  const h = Math.round(screenshotHeight * scale);
  return `
const page = await figma.getNodeByIdAsync('${pageId}');
await figma.setCurrentPageAsync(page);
await figma.loadFontAsync({ family: 'Inter', style: 'Semi Bold' });

const label = figma.createText();
label.fontName = { family: 'Inter', style: 'Semi Bold' };
label.fontSize = 16;
label.characters = 'Compiled — dist/home.html';
label.x = 832;
label.y = 0;

const rect = figma.createRectangle();
rect.name = 'Compiled site screenshot';
rect.resize(${w}, ${h});
rect.x = 832;
rect.y = 32;
rect.fills = [{ type: 'SOLID', color: { r: 0.9, g: 0.9, b: 0.9 } }];

figma.currentPage.appendChild(label);
figma.currentPage.appendChild(rect);

return { labelId: label.id, rectId: rect.id };
`;
}

module.exports = { buildScript, buildImagePlacementScript, VDS_SITE_BUILDER_KEY };
