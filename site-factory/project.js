'use strict';

/*
 * project.js: the ONE path from a config to a project on disk.
 *
 * Before this file there were two — wizard.js's compile() and studio.js's
 * commitProject() — each scaffolding, writing tokens, writing a manifest, building,
 * and optionally bridging to VDS in its own slightly different order. Two paths to
 * the same artefact is two things to keep in step, and the CLI and the studio had
 * already drifted: only one of them narrowed the SaaS route, and radiusPx was
 * defined twice with the same table.
 *
 * Everything that creates a project now comes through createProject().
 */

const fs = require('fs');
const path = require('path');
const { execFileSync } = require('child_process');

const { configToTokens, configToManifest } = require('./compose.js');
const { scaffold } = require('./scaffold.js');
const { bridge, resolveVdsBin } = require('./vds-bridge.js');

const ROOT = __dirname;

function slug(s) {
  return String(s == null ? '' : s)
    .trim().toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '') || 'my-project';
}

const SAAS_NOTE = 'SaaS route: the app-surface blocks are built (nav, sidebar, facetstrip, objecttable, ' +
  'objectview, inspector, masterdetail). The 102 other cataloged component types are recorded in ' +
  'SAAS-COMPONENTS.md as decisions, not built.';

function saasComponentSpec(config) {
  const cs = config.componentStyle || {};
  return `# ${config.identity.name} — component spec

This route compiled a real app surface (dist/home.html) from the seven block types
that exist in code: nav, sidebar, facetstrip, objecttable, objectview, inspector and
the masterdetail assembly. Everything below is a DECISION recorded in config.json.

## componentStyle layer
${Object.entries(cs).map(([k, v]) => `- ${k}: ${v}`).join('\n')}

## Built in code (blocks/, 2 variants each)
- FacetStrip      chips with counts / grouped with search
- ObjectTable     grid / selectable list, status column honours statusBadgeStyle
- Object View     header with gated actions / the same plus a tab strip
- Inspector       property panel / activity trail
- Master-Detail   two pane / three pane, composed from the four above

## Built as Figma specimens (VDS Site Builder 4pPUFvaPdqYzPquBusSfWl)
- Gated Action Button (node 20:12) — Style=Enabled / Style=Blocked
- StatusBadge (node 20:35) — Style=Pill / Style=Dot, matches statusBadgeStyle above

## Cataloged, not built (102 of 109 types)
See the "SaaS Components" page in the same file (roots 19:3, 19:206, 19:348). Priority
order per Opbox's COMPONENT_INVENTORY.md is complete; what remains is the long tail
across Forms & Inputs, Overlays & Dialogs, Communication and Domain & Commerce.
`;
}

/*
 * config -> a built project directory.
 *
 * opts.log      a sink for progress lines (defaults to a no-op, so the studio does
 *               not have to swallow console output)
 * opts.outDir   override the destination; defaults to scaffolds/<slug(name)>
 */
function createProject(config, opts = {}) {
  const log = opts.log || (() => {});
  const name = slug(config.identity.name);
  const outDir = opts.outDir || path.join(ROOT, 'scaffolds', name);

  if (fs.existsSync(outDir)) {
    throw new Error(`${path.relative(ROOT, outDir)} already exists — rename the project or remove that directory`);
  }

  // compose.js already narrows the SaaS route to the blocks that have real code, so
  // the manifest is the authority on which blocks to copy. Deriving the block list
  // from it (rather than from config.strategy.sitemap) is what keeps the CLI and the
  // studio from disagreeing about a SaaS build.
  const manifest = configToManifest(config);
  const blocks = manifest.page.map((e) => e.variant);
  if (!blocks.length) throw new Error('sitemap is empty — pick at least one block');

  const result = scaffold({ name, blocks: blocks.join(','), style: config.palette.basePack, outDir });
  log(`scaffolded ${blocks.length} blocks (${result.typesUsed.join(', ')})`);

  // Overwrite the scaffold's starter token file and manifest with the composed ones,
  // so what lands on disk is exactly what the preview showed.
  fs.writeFileSync(
    path.join(result.outDir, 'tokens', 'default.json'),
    JSON.stringify(configToTokens(config), null, 2)
  );
  fs.writeFileSync(
    path.join(result.outDir, 'manifests', 'home.json'),
    JSON.stringify({ ...manifest, stylePack: 'default' }, null, 2)
  );
  fs.writeFileSync(path.join(result.outDir, 'config.json'), JSON.stringify(config, null, 2));

  execFileSync(process.execPath, ['build.js', 'manifests/home.json'], { cwd: result.outDir, stdio: 'pipe' });
  log('built dist/home.html');

  const out = {
    name,
    outDir: result.outDir,
    relDir: path.relative(ROOT, result.outDir),
    blocks,
    typesUsed: result.typesUsed,
    governed: false,
    vds: null,
    note: null,
  };

  if (config.identity.category === 'saas-app') {
    fs.writeFileSync(path.join(result.outDir, 'SAAS-COMPONENTS.md'), saasComponentSpec(config));
    out.note = SAAS_NOTE;
  }

  if (config.governance && config.governance.vds) {
    const bin = resolveVdsBin();
    if (!bin) {
      out.vds = { error: 'no vds binary found (set VDS_BIN, or put vds on PATH)' };
      log(out.vds.error);
    } else {
      execFileSync(bin, ['init', '--jurisdiction', name, '--repo-code', name.slice(0, 12)], { cwd: result.outDir, stdio: 'pipe' });
      const b = bridge(result.outDir, result.typesUsed);
      out.governed = true;
      out.vds = {
        surface: b.config.changed,
        records: b.records.length,
        advanced: b.lifecycle.advanced.length,
        ledger: b.ledger,
      };
      log(`.vds/: ${b.records.length} records, ${b.lifecycle.advanced.length} advanced to built`);
    }
  }

  return out;
}

module.exports = { createProject, slug, saasComponentSpec, SAAS_NOTE };
