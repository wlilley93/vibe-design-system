#!/usr/bin/env node
'use strict';

/*
 * factory.js: one entry point.
 *
 *   node factory.js studio                     open the visual editor (default)
 *   node factory.js wizard [--vds]             the paged CLI
 *   node factory.js new --name X --brief "..." one shot, no questions
 *   node factory.js build <manifest>           render a manifest
 *   node factory.js ls                         what this factory can make
 *
 * There were four scripts to remember and no way to see what the factory held
 * without reading the source. Each still works standalone; this is the front door.
 */

const path = require('path');
const { execFileSync } = require('child_process');

const ROOT = __dirname;

function parseArgs(argv) {
  const out = { _: [] };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a.startsWith('--')) {
      const key = a.slice(2);
      const next = argv[i + 1];
      if (next === undefined || next.startsWith('--')) out[key] = true;
      else { out[key] = next; i++; }
    } else out._.push(a);
  }
  return out;
}

function ls() {
  const { listStylePacks, listBlockVariants } = require('./compose.js');
  const { LAYERS, ROUTES, fieldCount } = require('./config-schema.js');
  const { resolveVdsBin } = require('./vds-bridge.js');

  const variants = listBlockVariants();
  console.log('site-factory\n');
  console.log(`  routes        ${Object.keys(ROUTES).join(', ')}`);
  console.log(`  layers        ${LAYERS.length} (${fieldCount()} fields)`);
  console.log(`  style packs   ${listStylePacks().join(', ')}`);
  console.log(`  vds           ${resolveVdsBin() || 'not found — governance is opt-in and will be inert'}`);
  console.log(`\n  blocks (${Object.keys(variants).length} types, ${Object.values(variants).flat().length} variants)`);
  const width = Math.max(...Object.keys(variants).map((k) => k.length));
  for (const [type, vs] of Object.entries(variants)) {
    console.log(`    ${type.padEnd(width)}  ${vs.join('  ')}`);
  }
  console.log('\n  layers');
  for (const l of LAYERS) {
    console.log(`    ${l.title.padEnd(26)} ${l.fields.map((f) => f.key).join(', ')}`);
  }
}

// One shot: a brief in, a built project out, no prompts. This is the path a script
// or another agent should use — the wizard is for a human at a terminal.
function newProject(args) {
  const { suggest } = require('./suggest.js');
  const { createProject } = require('./project.js');

  if (!args.name) throw new Error('--name is required, e.g. --name "Northgate Trust"');
  const config = suggest({
    name: args.name,
    tagline: args.tagline || '',
    category: args.route || 'marketing-site',
    description: args.brief || '',
  });
  config.identity = {
    name: args.name,
    tagline: args.tagline || '',
    category: args.route || 'marketing-site',
    description: args.brief || '',
  };
  config.governance = { vds: Boolean(args.vds) };
  if (args.pack) config.palette.basePack = args.pack;
  if (args.radius) config.spacing.cornerRadius = args.radius;

  const r = createProject(config, { log: (m) => console.log(`  ${m}`) });
  console.log(`\n${r.relDir}`);
  if (r.note) console.log(`  note: ${r.note}`);
  return r;
}

function run() {
  const argv = process.argv.slice(2);
  const cmd = argv[0] && !argv[0].startsWith('--') ? argv[0] : 'studio';
  const args = parseArgs(argv[0] === cmd ? argv.slice(1) : argv);

  switch (cmd) {
    case 'studio':
      require('./studio.js').server.listen(Number(process.env.PORT) || 4321, () => {
        const { listStylePacks, listBlockVariants } = require('./compose.js');
        const { resolveVdsBin } = require('./vds-bridge.js');
        const port = Number(process.env.PORT) || 4321;
        console.log(`site-factory studio on http://localhost:${port}`);
        console.log(`  style packs: ${listStylePacks().join(', ')}`);
        console.log(`  block types: ${Object.keys(listBlockVariants()).length}`);
        console.log(`  vds: ${resolveVdsBin() || 'not found (governance toggle will be inert)'}`);
      });
      break;

    case 'wizard': {
      // Inherit stdio so the prompts behave exactly as they do standalone.
      const flags = args.vds ? ['--vds'] : [];
      execFileSync(process.execPath, [path.join(ROOT, 'wizard.js'), ...flags], { stdio: 'inherit' });
      break;
    }

    case 'new':
      newProject(args);
      break;

    case 'build': {
      const manifest = args._[0];
      if (!manifest) throw new Error('usage: node factory.js build manifests/<name>.json');
      execFileSync(process.execPath, [path.join(ROOT, 'build.js'), manifest], { stdio: 'inherit' });
      break;
    }

    case 'ls':
      ls();
      break;

    case 'help':
    case '--help':
    case '-h':
      console.log(require('fs').readFileSync(__filename, 'utf8').split('*/')[0].split('/*\n')[1].replace(/^ \* ?/gm, ''));
      break;

    default:
      console.error(`unknown command "${cmd}". Try: studio | wizard | new | build | ls | help`);
      process.exit(1);
  }
}

if (require.main === module) {
  try {
    run();
  } catch (err) {
    console.error(`factory: ${err.message}`);
    process.exit(1);
  }
}

module.exports = { run, ls, newProject };
