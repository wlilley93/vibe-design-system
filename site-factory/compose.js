'use strict';

/*
 * compose.js: the one place a config becomes (tokens, manifest).
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

  /*
   * The layers that reach the stylesheet as variables rather than as block content.
   *
   * These were the last controls that lied: buttonShape, tableDensity, motionIntensity and
   * transitionStyle were all rotatable in the studio and changed nothing, because nothing
   * carried them from the config into cssVars(). The values themselves live in build.js
   * (BUTTON_RADIUS, TABLE_DENSITY, MOTION_*), so this is a hand-off, not a second opinion.
   */
  tok.componentStyle = {
    buttonShape: (config.componentStyle || {}).buttonShape,
    tableDensity: (config.componentStyle || {}).tableDensity,
  };
  tok.motion = {
    intensity: (config.motion || {}).motionIntensity,
    transition: (config.motion || {}).transitionStyle,
  };

  /*
   * pairingStyle, made real.
   *
   * `single-family` sets one family for headings and body, which is what every pack on
   * disk does today. `display-plus-body-pair` keeps the pack's display face for headings
   * and drops body text to the system stack - the actual reason to pair, which is that a
   * display face set at 16px over three paragraphs is harder to read than Helvetica.
   *
   * A second variable rather than a second token file: the packs stay valid unchanged.
   */
  tok.font.body = config.typography.pairingStyle === 'display-plus-body-pair'
    ? 'system-ui, -apple-system, Segoe UI, Roboto, sans-serif'
    : config.typography.displayFont;

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
const SAAS_BLOCKS = new Set(['nav', 'sidebar', 'facetstrip', 'objecttable', 'objectview', 'inspector', 'masterdetail', 'formfield', 'emptystate', 'pagestate', 'confirmdialog', 'toast', 'segmentedcontrol', 'card']);
// masterdetail-2 renders its own facet strip from content.facets, so a standalone
// facetstrip in the same page draws it twice. Verified: the first Atlas Ops build
// showed two identical strips stacked.
// The size of the catalogue extracted from Opbox's COMPONENT_INVENTORY.md. The only
// number here that is genuinely a constant: the gap is DERIVED from it and from
// SAAS_BLOCKS, never restated. Two hand-kept copies said 98 and 107 on the day the
// real figure was 95.
const SAAS_CATALOG_TOTAL = 109;

/*
 * The control layer, derived from Uber Base's 92 component sets rather than from Opbox's
 * 109-type catalogue. SEPARATE FROM SAAS_BLOCKS ON PURPOSE, and the reason is arithmetic
 * rather than tidiness.
 *
 * `project.js` states the gap as SAAS_CATALOG_TOTAL - SAAS_BLOCKS.size: 95 of 109
 * cataloged and not built. Dropping these fifteen into SAAS_BLOCKS would move that line
 * to 80 of 109 and claim fifteen more of OPBOX's catalogue had been built, when they came
 * out of a different taxonomy entirely. Two measurements of two different populations
 * cannot be added, and a coverage figure that silently mixes them is the kind of number
 * this repo exists to refuse.
 *
 * So the gap keeps reading the Opbox set alone, and the app route unions both, because
 * these ARE app-surface components and a control nothing can reach is a control nobody
 * built. A test holds the two sets disjoint and holds the gap to the Opbox set.
 */
const BASE_BLOCKS = new Set([
  'switch', 'checkbox', 'radio', 'tooltip', 'notificationbadge', 'divider',
  'pagination', 'pagecontrols', 'menu', 'draggablelist',
  'progressbar', 'progresssteps', 'banner', 'systembanner', 'messagecard',
]);

// What the saas-app route may place: everything from either provenance.
const APP_BLOCKS = new Set([...SAAS_BLOCKS, ...BASE_BLOCKS]);

const SAAS_DEFAULT = ['nav-1', 'sidebar-2', 'masterdetail-2'];

/*
 * The pages a site has, as opposed to the blocks one page stacks.
 *
 * `strategy.sitemap` is a BLOCK sequence - its own label in config-schema.js says
 * "Block sequence (type:variant, ordered)" - so the field named `sitemap` was never a
 * sitemap, and the factory shipped exactly one page. Every nav link in every generated
 * site pointed at `href="#"`. A `notfound` block type existed with no way to reach it,
 * because there was no second page for a 404 to be.
 *
 * `strategy.pages` is the real thing. A config that does not carry it gets ONE page,
 * home, built from `sitemap` exactly as before, so nothing that worked stops working.
 *
 * Marketing sites get a default set; an app gets one, because an app shell routes
 * inside itself and a second static HTML file is not what it needs.
 */
const DEFAULT_PAGES = [
  { slug: 'home', title: 'Home', nav: true },
  { slug: 'about', title: 'About', nav: true },
  { slug: 'contact', title: 'Contact', nav: true },
  // Off the nav on purpose. A 404 you can navigate to is not a 404, and a nav that
  // offers one reads as a broken link even when it resolves.
  { slug: '404', title: 'Not found', nav: false },
];

// Which blocks belong on a page that is not home. Home keeps the full sitemap; the
// others carry the frame (nav, footer) plus what the page is actually for, so an
// "about" page is not a second copy of the pitch.
const PAGE_BLOCKS = {
  about: ['team-1', 'features-1', 'cta-1'],
  contact: ['contact-1'],
  404: ['notfound-1'],
};

const FRAME_BEFORE = 'nav';
const FRAME_AFTER = 'footer';

function pagesOf(config) {
  const declared = (config.strategy || {}).pages;
  if (Array.isArray(declared) && declared.length) return declared;
  if (config.identity.category === 'saas-app') return [{ slug: 'home', title: 'Home', nav: false }];
  return DEFAULT_PAGES.map((p) => ({ ...p }));
}

// Every page in the nav, as {label, href}. Derived, so adding a page cannot leave the
// navigation behind - which is the failure the single-page version made permanent.
function navLinks(config) {
  return pagesOf(config)
    .filter((p) => p.nav !== false)
    .map((p) => ({ label: p.title, href: p.slug === 'home' ? 'index.html' : `${p.slug}.html` }));
}

/*
 * Config -> a manifest for ONE page. Placeholder content per block type, with the
 * identity layer written over the fields it genuinely owns (wordmark, h1, sub,
 * copyright).
 *
 * The SaaS route keeps only app-surface blocks. SAAS-COMPONENTS.md records the
 * cataloged-and-unbuilt types as decisions rather than claiming they were built; the
 * count is derived there, never restated here.
 */
function configToManifest(config, pageSlug = 'home') {
  const isSaas = config.identity.category === 'saas-app';
  const links = navLinks(config);
  const contact = links.find((l) => /contact/i.test(l.href));
  const cta = contact ? contact.href : null;
  let blocks = config.strategy.sitemap.slice();

  if (pageSlug !== 'home') {
    // A secondary page keeps the frame the home page uses - the same nav variant, the
    // same footer variant - so the site does not change shape when you click a link.
    const frame = (which) => blocks.find((b) => b.startsWith(`${which}-`));
    const body = PAGE_BLOCKS[pageSlug] || ['features-1'];
    blocks = [frame(FRAME_BEFORE), ...body, frame(FRAME_AFTER)].filter(Boolean);
  }
  if (isSaas) {
    // APP_BLOCKS, not SAAS_BLOCKS: the control layer is reachable here too. The gap
    // arithmetic in project.js deliberately still reads SAAS_BLOCKS alone.
    blocks = blocks.filter((b) => APP_BLOCKS.has(b.slice(0, b.lastIndexOf('-'))));
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

    // The navigation reaches the artefact HERE, and this is the whole point of the
    // change: every generated nav and footer link used to be `href="#"`. A site whose
    // links go nowhere is a mockup, not a site.
    if (type === 'nav' || type === 'footer') {
      if (links.length) content.links = links.map((l) => ({ ...l }));
    }

    /*
     * Where a call to action goes.
     *
     * Every generated CTA - the nav button, the hero button, both pricing buttons, the
     * closing banner - shipped as `href="#"`. On a page whose entire job is to get one
     * click, the one link that must resolve was the only one that never did.
     *
     * `contact` is the destination, and it is DERIVED from the page set rather than
     * assumed: a site without a contact page keeps `#`, because inventing a URL that
     * 404s is worse than an honest dead anchor. The 404's own link goes home, which is
     * the only sensible destination a not-found page has.
     */
    if (cta) {
      if (type === 'nav') content.ctaHref = cta;
      if (type === 'hero' || type === 'cta') content.ctaHref = cta;
      if (type === 'pricing' && Array.isArray(content.plans)) {
        for (const plan of content.plans) plan.ctaHref = cta;
      }
    }
    if (type === 'notfound') content.homeHref = 'index.html';

    return { block: type, variant, content };
  });

  const pageMeta = pagesOf(config).find((p) => p.slug === pageSlug);
  const manifest = {
    title: pageSlug === 'home' || !pageMeta
      ? config.identity.name
      : `${pageMeta.title} - ${config.identity.name}`,
    slug: pageSlug,
    stylePack: config.palette.basePack,
    page,
  };
  /*
   * navigationPattern, made real.
   *
   * The app route hard-coded `layout: 'app'` and built a nav + rail shell whatever this
   * field said, so all three options produced identical output - the last of the six
   * controls that lied. The field now decides the SHELL, which is the only thing a
   * navigation pattern can mean once the blocks are chosen:
   *
   *   sidebar   a rail and no top bar. The nav block is dropped, not hidden: rendering a
   *             top nav into a layout that has no slot for it puts it above the shell,
   *             which is the stacked-sidebar bug this file already fixed once.
   *   both      rail plus top bar, which is what the route used to force.
   *   top-nav   no rail. Blocks stack full width, so a sidebar on the page would be a
   *             column with nothing to sit beside; it is dropped for the same reason.
   */
  if (isSaas) {
    const pattern = (config.componentStyle || {}).navigationPattern || 'both';
    if (pattern === 'top-nav') {
      manifest.page = manifest.page.filter((e) => e.block !== 'sidebar');
    } else {
      manifest.layout = 'app';
      if (pattern === 'sidebar') manifest.page = manifest.page.filter((e) => e.block !== 'nav');
    }
  }
  return manifest;
}

// Every page of the site, as {slug, manifest}. The one place that knows a site is more
// than a page, so nothing downstream has to reimplement the loop.
function configToSite(config) {
  return pagesOf(config).map((p) => ({
    slug: p.slug,
    title: p.title,
    manifest: configToManifest(config, p.slug),
  }));
}

module.exports = { configToTokens, configToManifest, configToSite, pagesOf, navLinks, radiusPx, listStylePacks, listBlockVariants, RADIUS, SAAS_BLOCKS, BASE_BLOCKS, APP_BLOCKS, SAAS_CATALOG_TOTAL };
