#!/usr/bin/env node
'use strict';

/*
 * studio.js: the character-creation UI.
 *
 *   node studio.js            # then open http://localhost:4321
 *
 * A local, zero-dependency HTTP server. The browser holds no render logic at all:
 * every preview is produced by the SAME renderPage() that `node build.js` calls, so
 * what the carousel shows is what compiles. That is the point - a preview with its
 * own renderer would eventually lie.
 *
 * Routes:
 *   GET  /            the UI
 *   GET  /schema      layers, fields, options, style packs, block variants
 *   POST /suggest     a brief -> a filled config
 *   POST /render      a config -> a self-contained HTML document
 *   POST /commit      a config -> a real scaffolded project on disk (+ optional VDS)
 */

const http = require('http');
const fs = require('fs');
const path = require('path');

const { LAYERS, ROUTES } = require('./config-schema.js');
const { suggest } = require('./suggest.js');
const { configToTokens, configToManifest, listStylePacks, listBlockVariants } = require('./compose.js');
const { renderPage } = require('./build.js');
const { resolveVdsBin } = require('./vds-bridge.js');
const { createProject } = require('./project.js');

const ROOT = __dirname;
const PORT = Number(process.env.PORT) || 4321;

function readBody(req) {
  return new Promise((resolve, reject) => {
    let raw = '';
    req.on('data', (c) => {
      raw += c;
      // A studio config is a few KB. Anything past this is not a config.
      if (raw.length > 1e6) { reject(new Error('body too large')); req.destroy(); }
    });
    req.on('end', () => {
      try { resolve(raw ? JSON.parse(raw) : {}); } catch (e) { reject(new Error(`bad JSON: ${e.message}`)); }
    });
    req.on('error', reject);
  });
}

function json(res, code, payload) {
  const body = JSON.stringify(payload);
  res.writeHead(code, { 'Content-Type': 'application/json', 'Content-Length': Buffer.byteLength(body) });
  res.end(body);
}

function commitProject(config) {
  const r = createProject(config);
  return {
    outDir: r.relDir,
    blocks: r.blocks,
    governed: r.governed,
    vds: r.vds && r.vds.error ? r.vds.error : r.vds,
    note: r.note,
  };
}

const server = http.createServer(async (req, res) => {
  try {
    if (req.method === 'GET' && (req.url === '/' || req.url.startsWith('/?'))) {
      const html = fs.readFileSync(path.join(ROOT, 'studio.html'), 'utf8');
      res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
      return res.end(html);
    }

    if (req.method === 'GET' && req.url === '/schema') {
      return json(res, 200, {
        layers: LAYERS,
        routes: ROUTES,
        stylePacks: listStylePacks(),
        blockVariants: listBlockVariants(),
        vdsAvailable: Boolean(resolveVdsBin()),
      });
    }

    // One style pack's real token values. The UI needs these so that rotating the
    // base pack re-seeds the six explicit colour fields and both fonts - without it
    // you would switch pack and see nothing move, because the colour fields still
    // hold the previous pack's hexes. Served from disk rather than duplicated in
    // browser JS, so tokens/ stays the single source.
    if (req.method === 'GET' && req.url.startsWith('/pack/')) {
      const name = decodeURIComponent(req.url.slice('/pack/'.length));
      if (!/^[a-z0-9-]+$/i.test(name)) return json(res, 400, { error: 'bad pack name' });
      const file = path.join(ROOT, 'tokens', `${name}.json`);
      if (!fs.existsSync(file)) return json(res, 404, { error: `no style pack "${name}"` });
      return json(res, 200, JSON.parse(fs.readFileSync(file, 'utf8')));
    }

    if (req.method === 'POST' && req.url === '/suggest') {
      const brief = await readBody(req);
      const config = suggest(brief);
      config.identity = {
        name: brief.name || 'Your Project',
        tagline: brief.tagline || '',
        category: brief.category || 'marketing-site',
        description: brief.description || '',
      };
      config.governance = { vds: Boolean(brief.vds) };
      return json(res, 200, config);
    }

    if (req.method === 'POST' && req.url === '/render') {
      const config = await readBody(req);
      const { html } = renderPage(configToManifest(config), configToTokens(config), null);
      res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
      return res.end(html);
    }

    if (req.method === 'POST' && req.url === '/commit') {
      const config = await readBody(req);
      return json(res, 200, commitProject(config));
    }

    json(res, 404, { error: `no route for ${req.method} ${req.url}` });
  } catch (err) {
    // Surface the real reason. A studio that swallows the error and shows a blank
    // preview is worse than one that says which block or pack is missing.
    json(res, 500, { error: err.message });
  }
});

if (require.main === module) {
  server.listen(PORT, () => {
    console.log(`site-factory studio on http://localhost:${PORT}`);
    console.log(`  style packs: ${listStylePacks().join(', ')}`);
    console.log(`  block types: ${Object.keys(listBlockVariants()).length}`);
    console.log(`  vds: ${resolveVdsBin() || 'not found (governance toggle will be inert)'}`);
  });
}

module.exports = { server, commitProject };
