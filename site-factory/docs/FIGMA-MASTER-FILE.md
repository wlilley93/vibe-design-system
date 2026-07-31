# The Figma master file

File key `4pPUFvaPdqYzPquBusSfWl`. One file, sixteen pages, 42 component sets and 25
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

**Styles (applied)** - the layer on top of everything else, and the one page that has to SHOW
rather than assert. The same specimen set - swatches, card, field, tone chips, buttons, table -
rendered in all four style packs side by side. Identical structure in every column, verified by
measurement: 57 nodes each, and a script that refuses to finish if the counts diverge or if
anything overflows its column. The only differences are palette, radius and typeface, because
the space unit is 4px and the border weight 1px in all four and every padding is a multiple of
the unit. Balmoral is the column that proves the point: radius 0 on both steps, so every card,
input, chip and table in it is square, which is the client's own binding decision expressed as
a token rather than typed into each component.

**Prompts (verbatim)** (8 frames) - the seven skill prompts the writing run invokes, in run
order, reproduced in full: 42,486 characters, nothing summarised. A prompt is the one part
of this system that never appears in the output it produces, which makes it the least
inspectable and most load-bearing artefact in the chain.

**System: Jellytot**, **System: Opbox marketing**, **System: Opbox design** - three brand and
product systems documented the way Uber Base documents itself: palette with an ink per tone,
type scale with its leading and tracking, radius and spacing scales, component counts, and
the provenance of every value. Each page names the files it was measured from and ends with
its findings, because in all three cases the interesting thing was a disagreement:

- Jellytot has THREE sources of truth and seven of eleven palette roles differ between the
  brand doc and the shipped CSS. The live stylesheet had a hex split across a line break at
  lines 5-6; the first reading of that (and the first version of the page) said the block was
  invalid and `--muted` and `--rule` were lost with it. THAT WAS WRONG. A custom property value
  is a token stream terminated by the next top-level semicolon, so `--sec` took the invalid
  value `#4A386 1` and the next two declarations parsed normally - and line 11 re-declared
  `--sec` correctly, so nothing rendered wrong. Tidied anyway, behaviour-neutrally. Its
  `.vds/config.toml` DOES point at `app/globals.css` and `app/**/page.tsx` in a repo that ships
  `public/style.css`, so every proof there runs over zero rows: present but blind.
- Opbox marketing is authored in oklch, which makes its ink and mute steps even in LIGHTNESS
  rather than even in hex. Its accent moved from a warm red to blue, and the old red survives
  as the corporate-services industry accent, so one hue now means two things. None of the six
  industry accents declares an ink.
- Opbox design already has the tone/ink pattern under another name (`--status-*-fg`), and its
  `statusMeaning` vocabulary - aligned / floor / migrating - is the most transferable idea in
  any of the three: `floor` says a per-theme value is legitimate and the CONTRAST RATIO is the
  contract, which is exactly what BREACH-0001 was about.

**Skills** - the agents-final survey. **Playbook** - the Product Design Playbook catalog.
**Glossary** - the Framer design vocabulary, two frames. **SaaS Components** - the
Opbox-measured taxonomy, three frames. **Examples** - design-system and creative-agency
references. **Project: Oneshot**, **Project: Meridian Labs**, **Spec: Balmoral & Co** - one
record each.

## The traps this file has sprung, most of them more than once

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

**The sizing modes are named for the AXIS, not for the dimension, and a helper that ignores
that is wrong on half its call sites.** This is the third and worst instance. A helper took a
width and did `resize(w, 100); primaryAxisSizingMode = 'AUTO'; counterAxisSizingMode = 'FIXED'`.
For a VERTICAL frame that is correct - the counter axis is horizontal, so the width is fixed and
the height hugs. For a HORIZONTAL frame the axes swap, so it pinned the HEIGHT at the 100 passed
as a placeholder and let the WIDTH hug. 241 frames across four pages were 100px tall, and every
`layoutWrap` container grew past its intended width instead of wrapping, because wrapping needs a
fixed primary axis. Releasing the heights then shifted every table column left of its own header,
because a column IS a width and those had been hugging too. For a horizontal frame that wants a
fixed width and a hugging height: `resize(w, h)`, then `primaryAxisSizingMode = 'FIXED'`, then
`counterAxisSizingMode = 'AUTO'`.

The repair was verified by measurement, not by eye: every row's Nth child must start at the same
offset as every other row's, and the header's cumulative column widths must equal those offsets.

A fourth, smaller one: `FILL` describes a child's relationship to its parent, so
`layoutSizingHorizontal = 'FILL'` throws on a node that has not been appended yet.

## And one trap that is not Figma's

Twice in one session I took a claim from a survey and wrote it down as established: that seven
pages had been wiped, and that a split hex had swallowed two custom properties. Both were
plausible, both were repeated into a commit message, and both were wrong - the first because
`page.children` needs `loadAsync`, the second because CSS custom property values are token
streams. Neither cost anything because both were checked before the work depended on them, but
the pattern is worth naming: A SURVEY REPORTS WHAT IT SAW, WHICH IS NOT THE SAME AS WHAT IS
TRUE. Verify the mechanism before writing the consequence down, especially when the consequence
is "something was destroyed".
