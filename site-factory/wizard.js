#!/usr/bin/env node
'use strict';

/*
 * wizard.js: the "character creation" step ahead of scaffold.js.
 *
 *   node wizard.js
 *
 * Asks for a one-paragraph brief, runs suggest.js over config-schema.js's 9 layers
 * / 35 fields to get a fully-filled config, then pages through each layer letting
 * you accept or edit any field before compiling. This is the interactive front end;
 * scaffold.js + build.js underneath are unchanged and still work standalone from the
 * CLI flags they always took.
 */

const fs = require('fs');
const path = require('path');
const readline = require('readline/promises');
const { execFileSync } = require('child_process');

const { LAYERS, ROUTES } = require('./config-schema.js');
const { suggest } = require('./suggest.js');
const { scaffold } = require('./scaffold.js');

const { bridge, resolveVdsBin } = require('./vds-bridge.js');

const ROOT = __dirname;

function slug(s) {
  return s.trim().toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '') || 'my-project';
}

// Node's readline auto-closes its interface the moment a piped (non-TTY) stdin hits
// EOF, which can land between two `question()` calls and leave the later one hanging
// forever with no error. That only bites non-interactive stdin (piped input, a test
// harness, a future batch mode) — a real terminal never sends EOF mid-session — but
// it bites it silently, so route around it instead of assuming a human is typing.
// TTY: normal readline prompting. Non-TTY: read every line up front and hand them out
// one at a time, echoing the prompt so the transcript still reads like a real session.
function makeAsker() {
  if (process.stdin.isTTY) {
    const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
    return { ask: (q) => rl.question(q), close: () => rl.close() };
  }
  const lines = fs.readFileSync(0, 'utf8').split('\n');
  let i = 0;
  return {
    ask: async (q) => {
      const line = i < lines.length ? lines[i++] : '';
      process.stdout.write(q + line + '\n');
      return line;
    },
    close: () => {},
  };
}

async function askEnum(asker, label, options, def, help) {
  console.log(`${label}${help ? `\n  (${help})` : ''}`);
  console.log(`  options: ${options.join(' | ')}`);
  const raw = (await asker.ask(`  [${def}] `)).trim();
  if (!raw) return def;
  const idx = parseInt(raw, 10);
  if (!isNaN(idx) && options[idx - 1]) return options[idx - 1];
  return options.includes(raw) ? raw : def;
}

async function promptField(asker, field, current) {
  if (field.type === 'enum') return askEnum(asker, field.label, field.options, current, field.help);
  if (field.type === 'multi-enum') {
    console.log(`${field.label}${field.help ? `\n  (${field.help})` : ''}`);
    console.log(`  options: ${field.options.join(', ')}`);
    const raw = (await asker.ask(`  [${current.join(', ')}] `)).trim();
    if (!raw) return current;
    return raw.split(',').map((s) => s.trim()).filter((s) => field.options.includes(s));
  }
  if (field.type === 'block-list') {
    console.log(`${field.label} (comma-separated, "type:variant" or bare "hero-1")`);
    const raw = (await asker.ask(`  [${current.join(', ')}] `)).trim();
    if (!raw) return current;
    return raw.split(',').map((s) => s.trim()).filter(Boolean);
  }
  if (field.type === 'number') {
    const raw = (await asker.ask(`${field.label} [${current}] `)).trim();
    return raw ? Number(raw) : current;
  }
  const raw = (await asker.ask(`${field.label} [${current}] `)).trim();
  return raw || current;
}

async function runWizard() {
  const asker = makeAsker();

  console.log('=== site-factory: character creation ===');
  console.log('Every layer below gets an AI-suggested default from your brief. Press enter to');
  console.log('accept a whole layer, or "edit <field>" to change one field, or "back".\n');

  const name = (await asker.ask('Project name: ')).trim() || 'Your Project';
  const tagline = (await asker.ask('Tagline: ')).trim();
  const identityLayer = LAYERS.find((l) => l.key === 'identity');
  const category = await askEnum(asker, 'Route', identityLayer.fields.find((f) => f.key === 'category').options, 'marketing-site');
  const description = (await asker.ask('One-paragraph brief: ')).trim();

  const config = suggest({ name, tagline, category, description });
  config.identity = { name, tagline, category, description };
  // Opt-in governance, set from the CLI flag rather than asked as a 36th question.
  config.governance = { vds: process.argv.includes('--vds') };

  const routeLayerKeys = new Set(ROUTES[category] || ROUTES['marketing-site']);
  const layers = LAYERS.filter((l) => l.key !== 'identity' && routeLayerKeys.has(l.key));
  let i = 0;
  while (i < layers.length) {
    const layer = layers[i];
    console.log(`\n--- [${i + 1}/${layers.length}] ${layer.title} ---`);
    if (layer.source) console.log(`(source: ${layer.source})`);
    for (const f of layer.fields) {
      const v = config[layer.key][f.key];
      console.log(`  ${f.key}: ${Array.isArray(v) ? v.join(', ') : JSON.stringify(v)}`);
    }
    const action = (await asker.ask('[enter]=accept & next, "back", "edit <field>", "quit": ')).trim();
    if (action === '') { i++; continue; }
    if (action === 'back') { i = Math.max(0, i - 1); continue; }
    if (action === 'quit') { asker.close(); console.log('cancelled.'); process.exit(0); }
    const m = action.match(/^edit\s+(.+)$/);
    if (m) {
      const field = layer.fields.find((f) => f.key === m[1].trim());
      if (!field) { console.log('unknown field for this layer.'); continue; }
      config[layer.key][field.key] = await promptField(asker, field, config[layer.key][field.key]);
      continue;
    }
    console.log('not recognized, try again.');
  }

  asker.close();
  return config;
}

function radiusPx(choice) {
  const table = { 'sharp-0': ['0px', '0px'], 'soft-6': ['6px', '12px'], 'round-16': ['16px', '24px'], 'pill': ['999px', '999px'] };
  return table[choice] || table['soft-6'];
}

// SaaS route: only nav/sidebar have real code block renderers today (109 SaaS
// component types are cataloged, only 2 — Gated Action Button, StatusBadge — exist
// as Figma specimens, and none as code). Compiling a full app shell here would be a
// silent overclaim, so the SaaS route builds only what's real and records the rest
// as a spec, not a build.
function saasShellBlocks(config) {
  const nav = (config.strategy.sitemap || []).find((b) => b.startsWith('nav'));
  const sidebar = (config.strategy.sitemap || []).find((b) => b.startsWith('sidebar'));
  return [nav || 'nav-1', sidebar || 'sidebar-2'];
}

/*
 * Establish VDS governance in the scaffolded project. OPTIONAL by design: without
 * `--vds` the project has no `.vds/` and builds identically, and VDS itself is
 * usable with no knowledge of site-factory. See vds-bridge.js for why `vds init`
 * alone is not enough (it writes a Next.js-shaped surface that leaves every gate
 * blind in a project with no app/ directory).
 */
function runVds(outDir, jurisdiction, blockTypes) {
  const bin = resolveVdsBin();
  if (!bin) {
    console.log('  (skipped: no vds binary. Set VDS_BIN, put vds on PATH, or run "cargo build --release -p vds-cli")');
    return false;
  }
  try {
    execFileSync(bin, ['init', '--jurisdiction', jurisdiction, '--repo-code', jurisdiction.slice(0, 12)], { cwd: outDir, stdio: 'inherit' });
  } catch (err) {
    console.log(`  (vds init failed, continuing without .vds/: ${err.message})`);
    return false;
  }
  const result = bridge(outDir, blockTypes);
  console.log(`  surface repointed at this project: ${result.config.changed.join(', ')}`);
  console.log(`  register: ${result.records.length} records written, advanced to built: ${result.lifecycle.advanced.length}`);
  if (result.lifecycle.failedAt) console.log(`  lifecycle stopped at ${result.lifecycle.failedAt}`);
  console.log(`  screens ledger: ${result.ledger ? 'generated' : 'NOT generated'}`);
  return true;
}

function compile(config) {
  const isSaas = config.identity.category === 'saas-app';
  const blocks = isSaas ? saasShellBlocks(config) : config.strategy.sitemap;
  if (!blocks.length) throw new Error('sitemap is empty — need at least one block to compile a page');

  const result = scaffold({ name: slug(config.identity.name), blocks: blocks.join(','), style: config.palette.basePack });

  const tokPath = path.join(result.outDir, 'tokens', 'default.json');
  const tok = JSON.parse(fs.readFileSync(tokPath, 'utf8'));
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
  fs.writeFileSync(tokPath, JSON.stringify(tok, null, 2));

  const manPath = path.join(result.outDir, 'manifests', 'home.json');
  const man = JSON.parse(fs.readFileSync(manPath, 'utf8'));
  man.title = config.identity.name;
  for (const entry of man.page) {
    if (entry.block === 'hero') {
      if (config.identity.tagline) entry.content.h1 = config.identity.tagline;
      if (config.identity.description) entry.content.sub = config.identity.description;
    }
    if (entry.block === 'nav' || entry.block === 'footer') {
      entry.content.wordmark = config.identity.name;
      if (entry.content.copyright) entry.content.copyright = `© 2026 ${config.identity.name}`;
    }
  }
  fs.writeFileSync(manPath, JSON.stringify(man, null, 2));

  fs.writeFileSync(path.join(result.outDir, 'config.json'), JSON.stringify(config, null, 2));

  execFileSync(process.execPath, ['build.js', 'manifests/home.json'], { cwd: result.outDir, stdio: 'inherit' });

  // Governance is opt-in. Without --vds this project is a plain static-site build
  // with no .vds/ anywhere in it, which is the "site-factory without VDS" half of
  // the requirement.
  if (config.governance && config.governance.vds) {
    console.log('\n=== vds (governed) ===');
    runVds(result.outDir, slug(config.identity.name), result.typesUsed);
  } else {
    console.log('\n(ungoverned build — pass --vds to establish .vds/ for this project)');
  }

  if (isSaas) {
    fs.writeFileSync(path.join(result.outDir, 'SAAS-COMPONENTS.md'), `# ${config.identity.name} — component spec

This route only compiled a real app shell (nav + sidebar — dist/home.html), because
those are the only SaaS-adjacent blocks with actual code renderers today. Everything
else below is a DECISION, recorded in config.json, not a build.

## componentStyle layer (config.json)
${Object.entries(config.componentStyle).map(([k, v]) => `- ${k}: ${v}`).join('\n')}

## Built so far (Figma specimens, VDS Site Builder file 4pPUFvaPdqYzPquBusSfWl)
- Gated Action Button (node 20:12) — Style=Enabled / Style=Blocked
- StatusBadge (node 20:35) — Style=Pill / Style=Dot, matches statusBadgeStyle above

## Cataloged but not yet built (107 of 109 SaaS component types)
See the "SaaS Components" page in the same Figma file (roots 19:3, 19:206, 19:348) for
the full taxonomy across Actions, Forms & Inputs, Feedback & Status, Overlays &
Dialogs, Navigation, Data Display, Timelines & History, Object & Page Assemblies,
Communication & Collaboration, Domain & Commerce, Canvas/Graph & Shell. Priority build
order per Opbox's COMPONENT_INVENTORY.md: FacetStrip -> ObjectTable -> Object View ->
Inspector -> Master-Detail Assembly.
`);
  }

  return result.outDir;
}

if (require.main === module) {
  runWizard().then((config) => {
    console.log('\n=== compiling ===');
    const outDir = compile(config);
    console.log(`\nDone. ${path.relative(ROOT, outDir)}/config.json holds every layer decided above.`);
  }).catch((err) => {
    console.error(`wizard failed: ${err.message}`);
    process.exit(1);
  });
}

module.exports = { runWizard, compile, slug };
