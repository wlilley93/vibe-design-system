'use strict';

/*
 * project.js: the ONE path from a config to a project on disk.
 *
 * Before this file there were two - wizard.js's compile() and studio.js's
 * commitProject() - each scaffolding, writing tokens, writing a manifest, building,
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

const {
  configToTokens, configToManifest, configToSite, listBlockVariants, SAAS_BLOCKS, SAAS_CATALOG_TOTAL,
} = require('./compose.js');

const variantsOf = (block) => listBlockVariants()[block] || [];
const { auditCopy } = require('./copy.js');
const { packConfig, briefMarkdown } = require('./skills.js');
const { scaffold } = require('./scaffold.js');
const { bridge, resolveVdsBin } = require('./vds-bridge.js');

const ROOT = __dirname;

function slug(s) {
  return String(s == null ? '' : s)
    .trim().toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '') || 'my-project';
}

// DERIVED, never restated. Hand-kept copies of this sentence said "seven block types",
// "98 other" and "107 of the 109" simultaneously, on a day when the truth was 14 and 95.
// A count in prose is a value like any other, and this one is now read off the code.
const saasBuilt = () => [...SAAS_BLOCKS].sort();
const saasGap = () => SAAS_CATALOG_TOTAL - SAAS_BLOCKS.size;

const saasNote = () =>
  `SaaS route: the app-surface blocks are built (${saasBuilt().join(', ')}). ` +
  `The ${saasGap()} other cataloged component types are recorded in SAAS-COMPONENTS.md ` +
  'as decisions, not built.';

function saasComponentSpec(config) {
  const cs = config.componentStyle || {};
  return `# ${config.identity.name} - component spec

This route compiled a real app surface (dist/home.html) from the ${SAAS_BLOCKS.size}
block types that exist in code: ${saasBuilt().join(', ')}. Everything below is a
DECISION recorded in config.json.

## componentStyle layer
${Object.entries(cs).map(([k, v]) => `- ${k}: ${v}`).join('\n')}

## Built in code (blocks/, 2 variants each)
${saasBuilt().map((b) => `- ${b}` + (variantsOf(b).length ? `  (${variantsOf(b).join(' / ')})` : '')).join('\n')}

## Built as Figma specimens (VDS Site Builder 4pPUFvaPdqYzPquBusSfWl)
- Gated Action Button (node 20:12) - Style=Enabled / Style=Blocked
- StatusBadge (node 20:35) - Style=Pill / Style=Dot, matches statusBadgeStyle above

## Cataloged, not built (${saasGap()} of ${SAAS_CATALOG_TOTAL} types)
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
    throw new Error(`${path.relative(ROOT, outDir)} already exists - rename the project or remove that directory`);
  }

  // compose.js already narrows the SaaS route to the blocks that have real code, so
  // the manifest is the authority on which blocks to copy. Deriving the block list
  // from it (rather than from config.strategy.sitemap) is what keeps the CLI and the
  // studio from disagreeing about a SaaS build.
  const manifest = configToManifest(config);
  // The UNION across every page, not just home. Scaffolding from the home manifest
  // alone copied 8 blocks and then the about page failed to build on `team`, because
  // a secondary page uses blocks the home page never mentions. Deduped in first-seen
  // order so the copy list stays stable between runs.
  const site = configToSite(config);
  const blocks = [...new Set(site.flatMap((pg) => pg.manifest.page.map((e) => e.variant)))];
  if (!blocks.length) throw new Error('sitemap is empty - pick at least one block');

  const result = scaffold({ name, blocks: blocks.join(','), style: config.palette.basePack, outDir });
  log(`scaffolded ${blocks.length} blocks across ${site.length} page${site.length === 1 ? '' : 's'} (${result.typesUsed.join(', ')})`);

  // Overwrite the scaffold's starter token file and manifest with the composed ones,
  // so what lands on disk is exactly what the preview showed.
  fs.writeFileSync(
    path.join(result.outDir, 'tokens', 'default.json'),
    JSON.stringify(configToTokens(config), null, 2)
  );
  // Every page, not just home. `home` is written as `index.html` because that is what a
  // web server serves for `/`, and because navLinks() points at it - a nav linking to
  // `home.html` on a host that serves `index.html` is a nav that 404s on its own logo.
  const built = [];
  for (const { slug, manifest: pm } of site) {
    const file = slug === 'home' ? 'index' : slug;
    fs.writeFileSync(
      path.join(result.outDir, 'manifests', `${file}.json`),
      JSON.stringify({ ...pm, stylePack: 'default' }, null, 2)
    );
    execFileSync(process.execPath, ['build.js', `manifests/${file}.json`], { cwd: result.outDir, stdio: 'pipe' });
    built.push(`${file}.html`);
  }
  fs.writeFileSync(path.join(result.outDir, 'config.json'), JSON.stringify(config, null, 2));
  log(`built ${built.length} page${built.length === 1 ? '' : 's'}: ${built.join(', ')}`);

  // An audit nobody reads is the same as no audit. Report the unwritten lines at the
  // moment the project is made, and write them to a file the author can work through,
  // so "CONFIRM:" is a task list rather than a marker that ships.
  const gaps = auditCopy(manifest);
  if (gaps.length) {
    fs.writeFileSync(
      path.join(result.outDir, 'COPY-TODO.md'),
      `# ${config.identity.name} - lines still to write\n\n` +
      `${gaps.length} strings are marked CONFIRM: because a one-line brief cannot supply them.\n` +
      'They are visible in the built page on purpose. Replace them in the page manifest each one names.\n\n' +
      gaps.map((g) => `- **${g.where}**\n  ${g.value}`).join('\n') + '\n'
    );
    // The gaps are a work queue for the content skills, not a manual to-do list.
    // copy-brief.json is this project in the shape every skill in the pack reads.
    fs.writeFileSync(
      path.join(result.outDir, 'copy-brief.json'),
      JSON.stringify(packConfig(config), null, 2)
    );
    fs.writeFileSync(path.join(result.outDir, 'WRITING-BRIEF.md'), briefMarkdown(config, manifest, gaps));
    log(`${gaps.length} lines still to write - see WRITING-BRIEF.md (skill-assigned)`);
  }

  const out = {
    name,
    outDir: result.outDir,
    relDir: path.relative(ROOT, result.outDir),
    blocks,
    typesUsed: result.typesUsed,
    governed: false,
    vds: null,
    note: null,
    copyGaps: gaps.length,
  };

  if (config.identity.category === 'saas-app') {
    fs.writeFileSync(path.join(result.outDir, 'SAAS-COMPONENTS.md'), saasComponentSpec(config));
    out.note = saasNote();
  }

  if (config.governance && config.governance.vds) {
    const bin = resolveVdsBin();
    if (!bin) {
      out.vds = { error: 'no vds binary found (set VDS_BIN, or put vds on PATH)' };
      log(out.vds.error);
    } else {
      execFileSync(bin, ['init', '--jurisdiction', name, '--repo-code', name.slice(0, 12)], { cwd: result.outDir, stdio: 'pipe' });
      const b = bridge(result.outDir, result.typesUsed);

      /*
       * `governed` is EARNED, not set because the call returned.
       *
       * It used to be `true` unconditionally right here, and neither of the two ways the
       * bridge fails throws: `refreshLedger` catches and returns false, and
       * `advanceToBuilt` returns `{failedAt, error}` instead of raising. So a project
       * with no screens ledger and every record stuck at `proposed` reported itself
       * governed - which is the exact shape of claim this whole seam exists to refuse.
       *
       * A record at `proposed` is a candidate, not a contract: `parity` skips it
       * (`record_below_registered_is_a_candidate_not_a_contract`), so a stuck lifecycle
       * silently switches proofs off rather than failing them.
       */
      const failures = [];
      if (b.ledger !== true) failures.push('the screens ledger could not be generated');
      if (b.lifecycle.skipped) failures.push(b.lifecycle.skipped);
      if (b.lifecycle.failedAt) failures.push(`lifecycle stopped at ${b.lifecycle.failedAt}: ${b.lifecycle.error}`);
      if (b.records.length && b.lifecycle.advanced.length !== b.records.length) {
        failures.push(`${b.records.length - b.lifecycle.advanced.length} of ${b.records.length} records did not reach built`);
      }

      out.governed = failures.length === 0;
      out.vds = {
        surface: b.config.changed,
        records: b.records.length,
        advanced: b.lifecycle.advanced.length,
        ledger: b.ledger,
      };
      if (failures.length) out.vds.failures = failures;

      log(`.vds/: ${b.records.length} records, ${b.lifecycle.advanced.length} advanced to built`);
      if (failures.length) {
        log(`.vds/: NOT governed - ${failures.join('; ')}`);
      }
    }
  }

  return out;
}

module.exports = { createProject, slug, saasComponentSpec, saasNote };
