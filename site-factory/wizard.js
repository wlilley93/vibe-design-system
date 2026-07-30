#!/usr/bin/env node
'use strict';

/*
 * wizard.js: the "character creation" step ahead of scaffold.js.
 *
 *   node wizard.js
 *
 * Asks for a one-paragraph brief, runs suggest.js over config-schema.js's 9 layers
 * / every field to get a fully-filled config, then pages through each layer letting
 * you accept or edit any field before compiling. This is the interactive front end;
 * scaffold.js + build.js underneath are unchanged and still work standalone from the
 * CLI flags they always took.
 */

const fs = require('fs');
const path = require('path');
const readline = require('readline/promises');

const { LAYERS, ROUTES } = require('./config-schema.js');
const { suggest } = require('./suggest.js');

const { createProject, slug } = require('./project.js');

const ROOT = __dirname;

// Node's readline auto-closes its interface the moment a piped (non-TTY) stdin hits
// EOF, which can land between two `question()` calls and leave the later one hanging
// forever with no error. That only bites non-interactive stdin (piped input, a test
// harness, a future batch mode) - a real terminal never sends EOF mid-session - but
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
  // Opt-in governance, set from the CLI flag rather than asked as one more question.
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

/*
 * Everything from here to the project on disk is project.js's job. The wizard's
 * only remaining responsibility is collecting the config.
 */
function compile(config) {
  const result = createProject(config, { log: (m) => console.log(`  ${m}`) });
  if (result.note) console.log(`  note: ${result.note}`);
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
