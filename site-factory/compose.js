'use strict';

/*
 * compose.js: the one place a 35-field config becomes (tokens, manifest).
 *
 * Both the CLI wizard and the studio server need this mapping. Two copies would
 * drift, and the drift would show up as the studio's live preview disagreeing with
 * what `node build.js` actually writes — which is the one thing a live preview must
 * never do. So neither owns it; this does.
 */

const fs = require('fs');
const path = require('path');

const { placeholderContent } = require('./scaffold.js');

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
 * elevation controls real rather than decorative — see cssVars in build.js.
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

/*
 * Config -> a manifest. Placeholder content per block type, with the identity layer
 * written over the fields it genuinely owns (wordmark, h1, sub, copyright).
 *
 * A SaaS route is narrowed to the blocks that have real code renderers. There are
 * 109 cataloged SaaS component types and only 2 exist as Figma specimens, none as
 * code, so composing a whole app here would be an overclaim.
 */
function configToManifest(config) {
  const isSaas = config.identity.category === 'saas-app';
  let blocks = config.strategy.sitemap.slice();
  if (isSaas) {
    const nav = blocks.find((b) => b.startsWith('nav'));
    const sidebar = blocks.find((b) => b.startsWith('sidebar'));
    blocks = [nav || 'nav-1', sidebar || 'sidebar-2'];
  }

  const page = blocks.map((variant) => {
    const idx = variant.lastIndexOf('-');
    const type = idx === -1 ? variant : variant.slice(0, idx);
    const content = JSON.parse(JSON.stringify(placeholderContent(type)));

    if (type === 'hero') {
      if (config.identity.tagline) content.h1 = config.identity.tagline;
      if (config.identity.description) content.sub = config.identity.description;
    }
    if (type === 'nav' || type === 'footer') {
      content.wordmark = config.identity.name;
      if (content.copyright) content.copyright = `© 2026 ${config.identity.name}`;
    }
    if (type === 'cta' && config.identity.tagline) {
      content.heading = config.identity.tagline;
    }
    return { block: type, variant, content };
  });

  return { title: config.identity.name, stylePack: config.palette.basePack, page };
}

module.exports = { configToTokens, configToManifest, radiusPx, listStylePacks, listBlockVariants, RADIUS };
