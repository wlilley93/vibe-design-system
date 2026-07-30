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
const { renderPage } = require('../build.js');
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
    assert.ok(fs.existsSync(path.join(out, 'dist', 'home.html')), 'no dist/home.html');
    assert.ok(fs.existsSync(path.join(out, 'tokens', 'default.json')), 'no tokens/default.json');

    // Rebuild from inside the copied tree: this is what proves it stands alone.
    execFileSync(process.execPath, ['build.js', 'manifests/home.json'], { cwd: out, stdio: 'pipe' });
    const html = fs.readFileSync(path.join(out, 'dist', 'home.html'), 'utf8');
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
    execFileSync(process.execPath, ['build.js', 'manifests/home.json'], { cwd: out, stdio: 'pipe' });
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
    execFileSync(process.execPath, ['build.js', 'manifests/home.json'], { cwd: out, stdio: 'pipe' });
    assert.ok(fs.statSync(path.join(out, 'dist', 'home.html')).size > 0);
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
    const builtHtml = fs.readFileSync(path.join(out, 'dist', 'home.html'), 'utf8');
    const builtCss = fs.readFileSync(path.join(out, 'dist', 'home.css'), 'utf8');

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
