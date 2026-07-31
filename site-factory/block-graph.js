'use strict';
/*
 * THE BLOCK COMPOSITION GRAPH, and the cycle nothing was checking.
 *
 * Most blocks are leaves. `masterdetail` is not: it requires `objecttable`,
 * `objectview`, `inspector` and `facetstrip` and assembles them, deliberately -
 * its own comment says an assembly ASSEMBLES rather than restating markup, so a
 * fix to objecttable reaches this layout too.
 *
 * That makes a cycle possible, and a cycle here fails in the worst way. A
 * circular `require` in CommonJS does NOT throw. Node hands the second requirer
 * a HALF-BUILT exports object, so `BLOCKS[type][variant]` comes back
 * `undefined` and the failure surfaces as a missing variant or a blank section,
 * a long way from the two files that actually disagree.
 *
 * Borrowed with attribution from `southleft/ds-contracts-poc`, whose
 * `core/emit-react.ts` refuses by name when a part's component reference creates
 * a cycle - "a contract cannot compose itself". VDS S-7(5) composition checks
 * MEMBERSHIP of `may_use` and has no equivalent: it can tell you a block used
 * something unregistered, and not that a block used itself.
 *
 * The graph is also worth having on its own. Nobody knows `masterdetail`
 * composes four blocks unless they open it, which means nobody knows that
 * changing `objecttable` changes two components.
 */

const fs = require('node:fs');
const path = require('node:path');

const BLOCKS_DIR = path.join(__dirname, 'blocks');

/**
 * Which blocks each block requires, read from its source.
 *
 * Only RELATIVE requires inside blocks/ count. A block requiring a shared helper
 * is not composing another block, and counting it would make the graph report
 * dependencies that carry no markup.
 */
function edges() {
  const out = {};
  for (const file of fs.readdirSync(BLOCKS_DIR).filter((f) => f.endsWith('.js'))) {
    const type = file.replace(/\.js$/, '');
    const code = fs.readFileSync(path.join(BLOCKS_DIR, file), 'utf8');
    const deps = new Set();
    for (const m of code.matchAll(/require\(\s*['"]\.\/([\w-]+)\.js['"]\s*\)/g)) {
      if (m[1] !== type) deps.add(m[1]);
    }
    out[type] = [...deps].sort();
  }
  return out;
}

/**
 * Every cycle in the graph, each as the path that closes it.
 *
 * Returns paths rather than a boolean, because "there is a cycle" is not a job
 * and `masterdetail -> objecttable -> masterdetail` is. A self-require is
 * reported too: it is the degenerate cycle and the one a copy-paste produces.
 */
function cycles(graph = edges()) {
  const found = [];
  const seen = new Set();
  const stack = [];
  const onStack = new Set();

  const walk = (node) => {
    stack.push(node);
    onStack.add(node);
    for (const next of graph[node] || []) {
      if (onStack.has(next)) {
        // Record from where the cycle actually closes, not from the root we
        // happened to start at - otherwise the same cycle reads differently
        // depending on traversal order.
        const at = stack.indexOf(next);
        const cycle = [...stack.slice(at), next];
        // Dedup on the SET OF NODES, dropping the repeated closing element.
        // The first version keyed on the whole path INCLUDING that repeat, so
        // `a -> b -> a` and `b -> a -> b` hashed differently and ONE cycle was
        // reported once per starting node: two findings for a two-block cycle,
        // three for a three-block one. A check that multiplies one defect into
        // N findings is the cry-wolf failure, and it was caught only because
        // the fixture asserts a COUNT rather than "at least one".
        const key = [...new Set(cycle)].sort().join('|');
        if (!seen.has(key)) { seen.add(key); found.push(cycle); }
      } else if (graph[next]) {
        walk(next);
      }
    }
    stack.pop();
    onStack.delete(node);
  };

  for (const node of Object.keys(graph).sort()) walk(node);
  return found;
}

/** Blocks that compose at least one other, and what depends on them. */
function summary(graph = edges()) {
  const composers = Object.entries(graph).filter(([, d]) => d.length);
  const dependents = {};
  for (const [type, deps] of Object.entries(graph)) {
    for (const d of deps) (dependents[d] = dependents[d] || []).push(type);
  }
  return {
    blocks: Object.keys(graph).length,
    composers: Object.fromEntries(composers),
    dependents,
    leaves: Object.keys(graph).length - composers.length,
  };
}

module.exports = { edges, cycles, summary, BLOCKS_DIR };

if (require.main === module) {
  const g = edges();
  const s = summary(g);
  const c = cycles(g);
  console.log(`${s.blocks} blocks, ${s.leaves} leaves, ${Object.keys(s.composers).length} composing others`);
  for (const [type, deps] of Object.entries(s.composers)) {
    console.log(`  ${type} -> ${deps.join(', ')}`);
  }
  console.log(c.length ? `\nCYCLES (${c.length}):` : '\nno cycles');
  for (const cycle of c) console.log(`  ${cycle.join(' -> ')}`);
  process.exit(c.length ? 1 : 0);
}
