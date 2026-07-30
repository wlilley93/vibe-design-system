# The Figma master file

File key `4pPUFvaPdqYzPquBusSfWl`. One file, twelve pages, 42 component sets and 20
documentation frames. It is the design half of site-factory: the code half is `blocks/`,
and three measured manifests hold the two together.

Do not take the counts below on trust. They are recorded in `figma-nodes.json` and a test
reconciles them; re-derive with the method in that file's `_how`.

## The three manifests, and what each can and cannot see

| Manifest | Pairs | Blind to |
|---|---|---|
| `figma-nodes.json` | every block type to a real component set id, and every page to its contents | a change made in Figma after the measurement |
| `figma-variables.json` | every `--color-*` and `--radius-*` the build emits to a Figma variable, per mode, by value | the same |
| `figma-prompts.json` | every skill prompt written into Figma to the SKILL.md it came from, by checksum | an edit made inside Figma |

Each states its limit in its own `_what` or `_limit` field rather than implying it has none.
The pattern throughout: the measurement is EVIDENCE, the code is the source of truth, and
Figma follows. Never hand-edit a measured value to make a test pass; that converts the one
artefact able to detect drift into a restatement of the code.

## The pages

**Components** (26 sets) - the marketing and app blocks, one set per block type, each bound
to the shared `VDS Tokens` collection. Three sets here are deliberately unpaired and say so
in `figma-nodes.json`: Gated Action Button and StatusBadge, drawn early and superseded, and
Base/Menu item, folded into the `menu` block type.

**Base (redrawn)** (16 sets) - tier 1 of the Uber Base baseline, redrawn from geometry
measured out of the Base Gallery rather than eyeballed. See `BASELINE.md`.

**Uber Base - full catalogue** (1 frame) - all 92 Base component sets and 3,211 variants,
with each set's tier, the site-factory block that answers it, and its import key in full.
The import path is blocked on a user action and the page says exactly which one.

**Prompts (verbatim)** (8 frames) - the seven skill prompts the writing run invokes, in run
order, reproduced in full: 42,486 characters, nothing summarised. A prompt is the one part
of this system that never appears in the output it produces, which makes it the least
inspectable and most load-bearing artefact in the chain.

**Skills** - the agents-final survey. **Playbook** - the Product Design Playbook catalog.
**Glossary** - the Framer design vocabulary, two frames. **SaaS Components** - the
Opbox-measured taxonomy, three frames. **Examples** - design-system and creative-agency
references. **Project: Oneshot**, **Project: Meridian Labs**, **Spec: Balmoral & Co** - one
record each.

## Two traps this file has already sprung, twice each

**Reading a page's children without loading the page.** This file is in Figma's
dynamic-page mode, so `page.children` is empty until `await page.loadAsync()`. A survey that
skipped it reported seven pages empty and I recorded that as a probable wipe. Nothing was
missing. An empty collection from a lazily-loading API is the absence of evidence, not
evidence of absence, and the full account is in `figma-nodes.json` under `_correction`.

**`resize()` and `layoutMode` each reset both auto-layout sizing modes.** Assign the sizing
modes AFTER the resize, or the frame stays pinned at whatever height you passed and clips
its contents in silence. This produced a 36-square pagination chip that came out 36x20 and
read as a deliberate pill, and a prompt frame 200px tall holding 5,690 characters. The order
that works is `layoutMode`, then `resize`, then the sizing modes. Both times it was caught
by reading the geometry back and asserting it, never by looking.

A third, smaller one: `FILL` describes a child's relationship to its parent, so
`layoutSizingHorizontal = 'FILL'` throws on a node that has not been appended yet.
