'use strict';

/*
 * End to end: a config becomes a directory on disk that builds on its own.
 *
 * Two bug classes live here.
 *
 * 1. TRANSITIVE DEPENDENCIES. scaffold.js copies the block types the manifest names.
 *    masterdetail requires four siblings, so a project asking for it scaffolded
 *    clean and then died at build with MODULE_NOT_FOUND on ./objecttable.js. The
 *    scaffolded tree is the artefact, so the test builds it rather than inspecting it.
 *
 * 2. THE PREVIEW MUST NOT LIE. The studio renders through renderPage in-process; the
 *    committed project runs build.js in a copied tree. If those ever diverge the
 *    carousel is showing something that does not compile.
 */

const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { execFileSync } = require('node:child_process');

const { suggest } = require('../suggest.js');
const { configToTokens, configToManifest, listBlockVariants } = require('../compose.js');
const { renderPage, SITE_CSS } = require('../build.js');
const { createProject } = require('../project.js');

const ROOT = path.join(__dirname, '..');

function tmp(name) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), `sf-${name}-`));
  return dir;
}
function config(category, sitemap) {
  const c = suggest({ name: 'Probe Co', tagline: 'A tagline.', category, description: 'A legal ownership advisory.' });
  c.identity = { name: 'Probe Co', tagline: 'A tagline.', category, description: 'A legal ownership advisory.' };
  c.governance = { vds: false };
  if (sitemap) c.strategy.sitemap = sitemap;
  return c;
}

test('a scaffolded project builds on its own, with no path back to the factory', () => {
  const out = path.join(tmp('build'), 'proj');
  try {
    const r = createProject(config('marketing-site'), { outDir: out });
    assert.ok(fs.existsSync(path.join(out, 'dist', 'index.html')), 'no dist/index.html');
    assert.ok(fs.existsSync(path.join(out, 'tokens', 'default.json')), 'no tokens/default.json');

    // Rebuild from inside the copied tree: this is what proves it stands alone.
    execFileSync(process.execPath, ['build.js', 'manifests/index.json'], { cwd: out, stdio: 'pipe' });
    const html = fs.readFileSync(path.join(out, 'dist', 'index.html'), 'utf8');
    assert.ok(html.includes('Probe Co'), 'the identity layer did not reach the built page');
    assert.ok(r.blocks.length > 0);
  } finally {
    fs.rmSync(out, { recursive: true, force: true });
  }
});

test('scaffolding an assembly copies the blocks it requires, transitively', () => {
  const out = path.join(tmp('deps'), 'proj');
  try {
    // masterdetail requires objecttable, objectview, inspector and facetstrip.
    createProject(config('saas-app', ['nav-1', 'masterdetail-2']), { outDir: out });
    const copied = fs.readdirSync(path.join(out, 'blocks')).map((f) => path.basename(f, '.js')).sort();
    for (const needed of ['masterdetail', 'objecttable', 'objectview', 'inspector', 'facetstrip']) {
      assert.ok(copied.includes(needed), `blocks/${needed}.js was not copied; got ${copied.join(', ')}`);
    }
    // The real check: it builds. A missing require only shows up when Node loads it.
    execFileSync(process.execPath, ['build.js', 'manifests/index.json'], { cwd: out, stdio: 'pipe' });
  } finally {
    fs.rmSync(out, { recursive: true, force: true });
  }
});

test('every block type can be scaffolded together and still build', () => {
  const out = path.join(tmp('all'), 'proj');
  try {
    const first = Object.entries(listBlockVariants()).map(([, v]) => v[0]);
    const c = config('marketing-site', first);
    createProject(c, { outDir: out });
    execFileSync(process.execPath, ['build.js', 'manifests/index.json'], { cwd: out, stdio: 'pipe' });
    assert.ok(fs.statSync(path.join(out, 'dist', 'index.html')).size > 0);
  } finally {
    fs.rmSync(out, { recursive: true, force: true });
  }
});

test('the in-process preview is byte-identical to what the project compiles', () => {
  const out = path.join(tmp('parity'), 'proj');
  try {
    const c = config('marketing-site');
    createProject(c, { outDir: out });

    const preview = renderPage(configToManifest(c), configToTokens(c), null);
    const builtHtml = fs.readFileSync(path.join(out, 'dist', 'index.html'), 'utf8');
    const builtCss = fs.readFileSync(path.join(out, 'dist', SITE_CSS), 'utf8');

    const body = (s) => s.slice(s.indexOf('<body>'), s.indexOf('</body>'));
    assert.equal(body(preview.html), body(builtHtml), 'preview body differs from the compiled page');
    assert.equal(preview.css.trim(), builtCss.trim(), 'preview CSS differs from the compiled stylesheet');
  } finally {
    fs.rmSync(out, { recursive: true, force: true });
  }
});

test('creating a project over an existing directory refuses rather than overwriting', () => {
  const dir = tmp('exists');
  const out = path.join(dir, 'proj');
  fs.mkdirSync(out, { recursive: true });
  fs.writeFileSync(path.join(out, 'keep.txt'), 'do not clobber me');
  try {
    assert.throws(() => createProject(config('marketing-site'), { outDir: out }), /already exists/);
    assert.equal(fs.readFileSync(path.join(out, 'keep.txt'), 'utf8'), 'do not clobber me');
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

test('a saas project writes its component spec and records the gap honestly', () => {
  const out = path.join(tmp('saas'), 'proj');
  try {
    const r = createProject(config('saas-app'), { outDir: out });
    const spec = fs.readFileSync(path.join(out, 'SAAS-COMPONENTS.md'), 'utf8');
    assert.ok(spec.includes('Cataloged, not built'), 'the spec must state what was NOT built');

    // `/\d+ of 109/` only proved A number was there, not the RIGHT one, so the spec
    // said 98 of 109 for three commits while the truth moved to 95. A check that any
    // digit is present is not a check on the count.
    const { SAAS_BLOCKS, SAAS_CATALOG_TOTAL } = require('../compose.js');
    const gap = SAAS_CATALOG_TOTAL - SAAS_BLOCKS.size;
    assert.match(spec, new RegExp(`${gap} of ${SAAS_CATALOG_TOTAL} types`),
      `the spec must state the real gap, ${gap} of ${SAAS_CATALOG_TOTAL}`);

    // And every built block must be named, so adding one to the route cannot leave the
    // spec claiming it was only cataloged.
    for (const b of SAAS_BLOCKS) {
      assert.ok(spec.includes(b), `SAAS-COMPONENTS.md never mentions the built block "${b}"`);
    }

    assert.ok(r.note, 'a saas build must return a note about the narrowing');
    assert.ok(r.note.includes(String(gap)), 'the returned note must quote the same gap as the spec');
  } finally {
    fs.rmSync(out, { recursive: true, force: true });
  }
});

test('a built site has no link pointing at a file that does not exist', () => {
  // The defect this is the gate for: the factory shipped ONE page and every nav link in
  // it was `href="#"`. A site whose navigation goes nowhere is a mockup. Nothing caught
  // it because nothing ever asked where a link went.
  const out = path.join(tmp('links'), 'proj');
  try {
    createProject(config('marketing-site'), { outDir: out });
    const dist = path.join(out, 'dist');
    const pages = fs.readdirSync(dist).filter((f) => f.endsWith('.html'));
    assert.ok(pages.length > 1, `a marketing site must build more than one page, got ${pages.length}`);

    const dead = [];
    for (const f of pages) {
      const html = fs.readFileSync(path.join(dist, f), 'utf8');
      const hrefs = [...new Set((html.match(/href="[^"]+"/g) || []).map((h) => h.slice(6, -1)))];
      for (const h of hrefs) {
        if (/^(https?:|mailto:|tel:)/.test(h)) continue;
        // A bare `#` is the failure, not an exemption: it is what every CTA and nav
        // link used to be, and it is indistinguishable from "we forgot".
        if (h === '#') { dead.push(`${f} -> "#"`); continue; }
        if (h.startsWith('#')) continue;                     // a real in-page anchor
        if (!fs.existsSync(path.join(dist, h))) dead.push(`${f} -> ${h} (missing)`);
      }
    }
    assert.deepEqual(dead, [], `links that go nowhere:\n  ${dead.join('\n  ')}`);
  } finally {
    fs.rmSync(out, { recursive: true, force: true });
  }
});

test('the whole site shares one stylesheet', () => {
  // A four-page build wrote four byte-identical CSS files. Four chances to diverge, and
  // four downloads for one stylesheet - identical contents under different names are
  // still separate files to a browser.
  const out = path.join(tmp('css'), 'proj');
  try {
    createProject(config('marketing-site'), { outDir: out });
    const dist = path.join(out, 'dist');
    const css = fs.readdirSync(dist).filter((f) => f.endsWith('.css'));
    assert.deepEqual(css, [SITE_CSS], `expected exactly ${SITE_CSS}, got ${css.join(', ')}`);

    // And every page must point at THAT file, or the sharing is nominal.
    for (const f of fs.readdirSync(dist).filter((x) => x.endsWith('.html'))) {
      const html = fs.readFileSync(path.join(dist, f), 'utf8');
      assert.match(html, new RegExp(`href="${SITE_CSS}"`), `${f} does not link ${SITE_CSS}`);
    }
  } finally {
    fs.rmSync(out, { recursive: true, force: true });
  }
});

test('every page is scaffolded the blocks it needs, not just the ones home uses', () => {
  // Scaffolding from the home manifest copied 8 blocks and the about page then failed to
  // build on `team`. A secondary page uses blocks home never mentions.
  const out = path.join(tmp('union'), 'proj');
  try {
    const c = config('marketing-site');
    createProject(c, { outDir: out });
    const { configToSite } = require('../compose.js');

    const needed = new Set(configToSite(c).flatMap((pg) => pg.manifest.page.map((e) => e.block)));
    const copied = new Set(fs.readdirSync(path.join(out, 'blocks')).map((f) => path.basename(f, '.js')));
    const missing = [...needed].filter((b) => !copied.has(b));
    assert.deepEqual(missing, [], `pages reference blocks the scaffold never copied: ${missing.join(', ')}`);
  } finally {
    fs.rmSync(out, { recursive: true, force: true });
  }
});

test('a project reports itself governed only when the bridge actually succeeded', () => {
  // `out.governed = true` was set unconditionally after bridge() returned, and NEITHER of
  // the two ways it fails throws: refreshLedger catches and returns false, advanceToBuilt
  // returns {failedAt, error}. So a project with no screens ledger and every record stuck
  // at `proposed` reported itself governed.
  //
  // That is not cosmetic. A record at `proposed` is a candidate, not a contract - `parity`
  // skips it as `record_below_registered_is_a_candidate_not_a_contract` - so a stuck
  // lifecycle switches proofs OFF rather than failing them, under a banner saying governed.
  const { resolveVdsBin } = require('../vds-bridge.js');
  if (!resolveVdsBin()) return;                     // no binary here; the seam is opt-in

  const out = path.join(tmp('gov'), 'proj');
  try {
    const c = config('marketing-site');
    c.governance = { vds: true };
    const r = createProject(c, { outDir: out });

    assert.equal(r.governed, true, `governed should be true on a clean run: ${JSON.stringify(r.vds)}`);
    assert.deepEqual(r.vds.failures, undefined, 'a clean run must report no failures');

    // And the claim has to be backed: every record advanced, and the ledger exists.
    assert.equal(r.vds.advanced, r.vds.records,
      `${r.vds.records - r.vds.advanced} records did not reach built, yet governed was true`);
    assert.equal(r.vds.ledger, true);
    assert.ok(fs.existsSync(path.join(out, '.vds', 'ledgers', 'screens.yaml')),
      'governed with no screens ledger on disk');
  } finally {
    fs.rmSync(out, { recursive: true, force: true });
  }
});

test('two taxonomies are never added together, and the Opbox gap reads the Opbox set alone', () => {
  // The control layer came out of Uber Base's 92 component sets. The gap line in
  // project.js reads SAAS_CATALOG_TOTAL - SAAS_BLOCKS.size, and SAAS_CATALOG_TOTAL is
  // the size of OPBOX's catalogue. Dropping fifteen Base-derived controls into
  // SAAS_BLOCKS would have moved that line from 95 of 109 to 80 of 109 and claimed
  // fifteen more of Opbox's catalogue had been built. Two populations, one denominator.
  //
  // So this is the check that keeps them apart, rather than the comment that asks nicely.
  const { SAAS_BLOCKS, BASE_BLOCKS, APP_BLOCKS, SAAS_CATALOG_TOTAL } = require('../compose.js');

  const both = [...BASE_BLOCKS].filter((b) => SAAS_BLOCKS.has(b));
  assert.deepEqual(both, [],
    `these block types are claimed by both taxonomies, so the gap arithmetic double-counts them: ${both.join(', ')}`);

  // The union is what the route may place, and it must be exactly the two sets. A block
  // in neither is a block the saas route silently drops.
  assert.equal(APP_BLOCKS.size, SAAS_BLOCKS.size + BASE_BLOCKS.size,
    'APP_BLOCKS is not the union of the two provenances');

  // Every Base-derived control must actually exist in blocks/, or the route filter admits
  // a name nothing can render.
  const types = new Set(Object.keys(listBlockVariants()));
  const phantom = [...BASE_BLOCKS].filter((b) => !types.has(b));
  assert.deepEqual(phantom, [], `BASE_BLOCKS names block types with no file: ${phantom.join(', ')}`);

  // And the denominator must stay bigger than the numerator it is measured against. If
  // SAAS_BLOCKS ever exceeds the catalogue it claims to be a subset of, one of the two
  // numbers has been repurposed.
  assert.ok(SAAS_BLOCKS.size < SAAS_CATALOG_TOTAL,
    `SAAS_BLOCKS (${SAAS_BLOCKS.size}) is not a subset of a ${SAAS_CATALOG_TOTAL}-type catalogue`);
});
