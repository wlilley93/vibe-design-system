'use strict';

/*
 * compose.js: the one place a 35-field config becomes (tokens, manifest).
 *
 * Both the CLI wizard and the studio server need this mapping. Two copies would
 * drift, and the drift would show up as the studio's live preview disagreeing with
 * what `node build.js` actually writes - which is the one thing a live preview must
 * never do. So neither owns it; this does.
 */

const fs = require('fs');
const path = require('path');

const { placeholderContent } = require('./scaffold.js');
const { copyFor } = require('./copy.js');

const ROOT = __dirname;
const TOKENS_DIR = path.join(ROOT, 'tokens');

// The four radius languages the config layer offers, as the two values the
// stylesheet actually consumes. `sharp-0` is not a stylistic default: it is
// Balmoral's own binding decision ("no rounded corners on any rectangle, panel or
// frame"), which is why it is a first-class option rather than a slider at zero.
const RADIUS = {
  'sharp-0': ['0px', '0px'],
  'soft-6': ['6px', '12px'],
  'round-16': ['16px', '24px'],
  pill: ['999px', '999px'],
};

function radiusPx(choice) {
  return RADIUS[choice] || RADIUS['soft-6'];
}

function listStylePacks() {
  return fs.readdirSync(TOKENS_DIR)
    .filter((f) => f.endsWith('.json'))
    .map((f) => path.basename(f, '.json'))
    .sort();
}

function listBlockVariants() {
  const out = {};
  const blocksDir = path.join(ROOT, 'blocks');
  for (const file of fs.readdirSync(blocksDir)) {
    if (!file.endsWith('.js')) continue;
    const type = path.basename(file, '.js');
    out[type] = Object.keys(require(path.join(blocksDir, file)));
  }
  return out;
}

/*
 * Config -> the token object build.js consumes.
 *
 * The base pack supplies anything the config does not name (notably `space.unit`
 * and the mono font); every field the config DOES name wins. The scale/border/
 * elevation blocks are what make the density, typeScale, borderWeight and
 * elevation controls real rather than decorative - see cssVars in build.js.
 */
function configToTokens(config) {
  const packName = config.palette.basePack;
  const packPath = path.join(TOKENS_DIR, `${packName}.json`);
  if (!fs.existsSync(packPath)) throw new Error(`no style pack "${packName}" in tokens/`);
  const tok = JSON.parse(fs.readFileSync(packPath, 'utf8'));

  tok.colors.bg = config.palette.groundColor;
  tok.colors.surface = config.palette.surfaceColor;
  tok.colors.ink = config.palette.inkColor;
  tok.colors.accent = config.palette.accentColor;
  tok.colors.accentInk = config.palette.accentInkColor;
  tok.colors.border = config.palette.borderColor;

  tok.font.family = config.typography.displayFont;
  tok.font.mono = config.typography.monoFont;

  const [rsm, rlg] = radiusPx(config.spacing.cornerRadius);
  tok.radius.sm = rsm;
  tok.radius.lg = rlg;
  tok.space.unit = config.spacing.spaceUnit;

  tok.scale = { density: config.spacing.density, type: config.typography.typeScale };
  tok.border = { weight: config.spacing.borderWeight };
  tok.elevation = config.spacing.elevation;

  return tok;
}

// The block types that make sense on a SaaS app surface. The route is still
// narrowed - a marketing pricing table has no place in an app shell - but the
// narrowing is now to what genuinely exists in code, not to nav+sidebar alone.
// Opbox's COMPONENT_INVENTORY.md priority order (FacetStrip -> ObjectTable ->
// Object View -> Inspector -> Master-Detail Assembly) is all built.
// Extended by MEASURED demand, not by guess: counted across 217 real Opbox routes.
// input 54, label 46, textarea 23, native-select 22 (formfield, 145 combined);
// empty-state 49; page-loading 45 + page-error 38 (pagestate, 83);
// destructive-action-dialog 16. Each is also a Modular Play the Playbook already
// names - Empty States, Loading Feedback, Fail Safe - so the measurement and the
// strategy layer agree rather than one being invented to justify the other.
const SAAS_BLOCKS = new Set(['nav', 'sidebar', 'facetstrip', 'objecttable', 'objectview', 'inspector', 'masterdetail', 'formfield', 'emptystate', 'pagestate', 'confirmdialog']);
// masterdetail-2 renders its own facet strip from content.facets, so a standalone
// facetstrip in the same page draws it twice. Verified: the first Atlas Ops build
// showed two identical strips stacked.
const SAAS_DEFAULT = ['nav-1', 'sidebar-2', 'masterdetail-2'];

/*
 * Config -> a manifest. Placeholder content per block type, with the identity layer
 * written over the fields it genuinely owns (wordmark, h1, sub, copyright).
 *
 * The SaaS route keeps only app-surface blocks. 107 of the 109 cataloged SaaS
 * component types still do not exist in code, and SAAS-COMPONENTS.md records them
 * as decisions rather than claiming they were built.
 */
function configToManifest(config) {
  const isSaas = config.identity.category === 'saas-app';
  let blocks = config.strategy.sitemap.slice();
  if (isSaas) {
    blocks = blocks.filter((b) => SAAS_BLOCKS.has(b.slice(0, b.lastIndexOf('-'))));
    if (blocks.some((b) => b.startsWith('masterdetail'))) {
      blocks = blocks.filter((b) => !b.startsWith('facetstrip'));
    }
    if (!blocks.length) blocks = SAAS_DEFAULT.slice();
  }

  const page = blocks.map((variant) => {
    const idx = variant.lastIndexOf('-');
    const type = idx === -1 ? variant : variant.slice(0, idx);
    const content = JSON.parse(JSON.stringify(placeholderContent(type)));

    // The voice layer reaches the page here. copy.js derives what the brief genuinely
    // supports and marks the rest CONFIRM: rather than inventing filler - see that
    // file for why an invented line that reads finished is worse than a blank one.
    // Blocks it does not speak for keep scaffold.js's neutral placeholder.
    const authored = copyFor(type, config.identity, config.voice);
    if (authored) Object.assign(content, authored);
    // componentStyle.statusBadgeStyle reaches the artefact here. Without this the
    // field would be another control that changes the config and not the page.
    const badgeStyle = (config.componentStyle || {}).statusBadgeStyle || 'pill';
    if (type === 'objecttable') content.badgeStyle = badgeStyle;
    if (type === 'masterdetail' && content.master) content.master.badgeStyle = badgeStyle;
    if (type === 'objectview') content.title = config.identity.name;
    return { block: type, variant, content };
  });

  const manifest = { title: config.identity.name, stylePack: config.palette.basePack, page };
  if (isSaas) manifest.layout = 'app';
  return manifest;
}

module.exports = { configToTokens, configToManifest, radiusPx, listStylePacks, listBlockVariants, RADIUS };
