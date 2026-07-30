# site-factory Goal Statement

This file is engineering explanation. It binds nothing. It exists to say what done looks
like in conditions a command can settle, and to state the current position without
flattering it.

Every number below was produced by the command beside it, measured 2026-07-30. A table
whose header asserts it was measured is worse when stale than one that makes no claim,
because the header is the reason to trust it. `docs/GOAL.md` at the repository root had six
cells wrong when it was checked; that is the failure this line exists to avoid repeating.

## North star

**A brief in, a built site out, with every design decision legible enough to argue with.**

The differentiated claim is not speed. "A site in one command" is what every generator
says. It is that the output is **readable**: every value traces to a token somebody chose,
every unwritten line is counted and assigned by name, and the Figma file and the code
cannot drift apart without a test failing.

Three things follow, and they are worth stating before the criteria:

- **The writing is the point of the AI/MCP layer, and skills do it.** `copy.js` is not a
  language model and does not pretend to be. It derives what a brief genuinely supports and
  marks the rest. The craft lives in the agents-final content skills; this codebase's job is
  to hand them a brief in the shape they read and to count what they have not yet written.
- **Refusing is a first-class output.** A run that reports 20 of 20 lines written, having
  fabricated two testimonials and two price tiers, has produced a page that reads finished
  and carries its two most load-bearing claims as fiction. The reference run refused five of
  twenty and dropped two whole blocks, and that ratio is the finding, not a shortfall.
- **Every criterion is bounded to what is actually built.** No criterion asserts that a
  generated site is *good*. A finite check can prove the modelled properties and never the
  absence of an unmodelled defect.

## Done criteria (measurable)

Each criterion names how it is settled. A criterion with no settling command is not a
criterion, it is a hope.

### S1 A brief produces a built, multi-page, navigable site with no manual step

- `node factory.js new --name X --brief "..."` exits 0 and writes a project that builds.
- More than one page, home compiled as `index.html`.
- Zero internal links pointing at a missing file, and zero bare `#` anchors.
- One stylesheet for the whole site.

**Settled by:** `tests/project.test.js`, which walks every `href` in every built page.
**Position: MET.** 4 pages, 1 stylesheet, 16 internal links, 0 broken.

### S2 No control lies

Every enum field the schema declares either moves the rendered output, or is listed as
exempt with a stated reason. Both directions fail: an exemption that starts working is
reported too, because a stale limitation is documentation claiming a problem that was fixed.

**Settled by:** `tests/compose.test.js`, which renders every option of every enum field on
both routes and compares bytes.
**Position: MET, recently and narrowly.** 19 enum fields, 17 wired, 2 exempt
(`imageTreatment`, `iconStyle` - nothing renders an image or an icon yet). This criterion
read as met for weeks while **8 of 19 were dead**, because the test checked the four fields
the rule had named instead of the class the rule described.

### S3 Every value in the stylesheet traces to a token

- Zero hex literals in `STRUCTURE_CSS`.
- Zero px literals outside a `--space` multiplier or a documented sentinel.

**Settled by:** a grep over `STRUCTURE_CSS`.
**Position: HALF MET, and the README overstates it.** Hex literals: 0. But **16 distinct px
declarations remain** - `max-width` on seven containers (640, 1100, 800, 720, 980, 480, 460,
420), an avatar at 64px, a rail at 240px, grid columns at 320/300/260px, a 2px border-width
and a 2px inset shadow. `999px` for a pill is a legitimate sentinel; the rest are not.

The README claims "no hex, no font name, no px literal outside the `--space` multiplier" and
then offers only a grep for hex as its check - a claim broader than its evidence, the same
shape as S2's failure. **To close this:** a `--measure` token for line-length caps, a
`--rail` token for the sidebar and grid columns, and `--border-weight` for the 2px cases;
then a test asserting the grep finds zero, so the claim and its check are the same size.

### S4 What cannot be honestly written is counted and assigned, never invented

- Every unwritten line carries a marker and appears in `WRITING-BRIEF.md`.
- Both marker conventions are counted (`CONFIRM:` and "Replace this…"), or the audit
  undercounts.
- Every gap is assigned to the agents-final skill that writes it, in the order the pack
  declares, or listed as unassigned.
- Zero banned words in any built page.
- Extraction from a brief is **literal**: a brief stating nothing yields nothing.

**Settled by:** `auditCopy()` and `bannedWords()` over the built manifest;
`tests/copy.test.js` and `tests/skills.test.js`.
**Position: MET.** A fresh home page reports 20 unwritten lines against 6 skills. The
reference run in `WRITING-RUN.md` worked all seven skills and refused 5 of 20 with reasons.

### S5 The code bank and the Figma bank cannot diverge

- Every block type has a Figma component set.
- No Figma node id names a block type that does not exist.
- Every state a block renders conditionally is declared in the register, and claimed drawn
  only where a measured layer name cites it.

**Settled by:** `tests/skills.test.js` and `tests/spec.test.js` against `FIGMA_NODES` and
`figma-states.json`.
**Position: MET.** 24 block types, 24 component sets, 48 variants. This criterion was
introduced *because* four blocks shipped code-only; the test it produced then caught the next
three before they landed.

### S6 The governance seam is honest about what it buys

With `--vds`, a generated project's `.vds/` must point at paths the project has, and the
project must report itself governed **only** when the bridge actually succeeded. What each
proof kind does and does not establish is recorded, and `rows_enforced` is the number quoted
- never the status.

**Settled by:** `vds proof <kind>` per kind on a generated project;
`tests/project.test.js` for the governed claim.
**Position: MET as to honesty, and 5 of 11 kinds enforce.** Measured on a generated project:

| kind | rows enforced | |
|---|---|---|
| `no_stored_values` | 24 | |
| `reconciliation` | 22 | |
| `contrast` | 11 | |
| `states` | 11 | |
| `ledger_staleness` | 1 | |
| `retirement_drain` | 0 | correct - nothing is deprecated |
| `token_pin` | 0 | correct - no pins |
| `screen_parity` | 0 | no screen declares a required arrangement |
| `register_completeness` | 0 | **structural** |
| `composition` | 0 | **structural** |
| `parity` | 0 | **structural** |

The three structural ones are not a configuration mistake and no setting fixes them.
`vds-scan` parses ESM `import`; a scaffold resolves blocks through a dynamic `require()`
because dropping a file in `blocks/` is meant to register it. There is no static import
graph to walk. **A parallel screens file generated to feed the scanner was considered and
refused:** a second artefact describing the build would drift from it, which is the failure
mode rather than the fix.

### S7 Every check has a negative control that was verified to fire

A check whose failure has never been observed is a green light wired to nothing. Each gate
in this codebase has had a defect seeded against it and the non-zero exit confirmed - and
the seed itself is asserted present before the result is read, because a seed that silently
misses reads exactly like a dead gate.

**Settled by:** `tests/gate.js`, which asserts a floor on files and tests actually run,
because `node --test tests/*.test.js` **exits 0 on an empty glob** - measured, not assumed.
**Position: MET.** 79 tests across 7 files.

## Current position, stated plainly

| what | measured | command |
|---|---|---|
| block types | 24 | `node factory.js ls` |
| variants | 48 | `node factory.js ls` |
| Figma component sets paired | 24 of 24 | `tests/skills.test.js` |
| config fields | 36 in 9 layers | `config-schema.js` `fieldCount()` |
| enum fields wired | 17 of 19, 2 exempt with reasons | `tests/compose.test.js` |
| style packs | 4 | `node factory.js ls` |
| tests | 79 across 7 files | `node tests/gate.js` |
| hex literals in the stylesheet | 0 | grep over `STRUCTURE_CSS` |
| px literals outside `--space` | **16 distinct - see S3** | grep over `STRUCTURE_CSS` |
| SaaS component types built | 14 of 109 cataloged | `SAAS_BLOCKS` / `SAAS_CATALOG_TOTAL` |
| proof kinds enforcing on a generated project | 5 of 11 | `vds proof <kind>` |

## What is NOT claimed

- **That a generated site is well designed.** Every criterion here is about legibility and
  internal consistency. None is about whether the page works on a reader.
- **That 109 SaaS component types will be built.** 14 are. The next measured tier is
  single-digit demand across 224 real routes, which is past the point where measurement
  justifies the build. `SAAS-COMPONENTS.md` records the remainder as decisions, not as work
  in progress.
- **That the three structurally-vacuous proof kinds will be closed.** They need either a
  static import graph this factory deliberately does not have, or a parallel artefact that
  would drift. Both are worse than the honest zero.
- **That `copy.js` writes well.** It derives and marks. Anything that reads as written prose
  came from a skill or from a person.

## The rules this codebase was built on, and what each cost

Every rule in `README.md` under "Rules this codebase is built on" was learned by breaking
it. Two are worth repeating here because they bear on the criteria above rather than on the
code:

**A claim is only as good as the check that is the same size as it.** S2 and S3 both failed
the same way: a rule stated broadly, a check covering a named subset, and nobody comparing
the two. When writing a rule, write the check that would fail if the rule were violated
anywhere - not the check that passes on the instance that prompted it.

**Read `rows_enforced`, never the status.** A proof that considers every row and enforces
none reports a pass. The `states` kind was vacuous on every generated project for exactly
that reason, and turning it on immediately found four components rendering a state in code
that the Figma file never drew.
