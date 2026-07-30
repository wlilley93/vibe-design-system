# site-factory

A brief in, a built site out - with the design decisions visible, the writing handed
to the skills that do it well, and the gaps counted rather than hidden.

```
node factory.js ls        what this factory can make, measured from disk
node factory.js studio    the visual editor, http://localhost:4321
node factory.js wizard    the paged CLI
node factory.js new --name "X" --brief "..." [--vds]    one shot, no prompts
node factory.js build manifests/home.json               render a manifest
```

Zero dependencies. Node's built-in `http` and `node:test`, nothing installed.

---

## The shape

A **config** (35 fields, 9 layers) is composed into **tokens** + a **manifest**, which
a **block** registry renders into static HTML and CSS.

```
brief ──▶ suggest.js ──▶ config (35 fields) ──▶ compose.js ──┬──▶ tokens
                              │                              └──▶ manifest
                              │                                     │
                              ├──▶ copy.js      (the voice layer writes, or marks)
                              ├──▶ skills.js    (what it cannot write, assigned)
                              └──▶ figma-spec.js (one drawn specimen per decision)
                                                                    │
                                                       build.js ◀───┘
                                                          │
                                              dist/home.html + home.css
```

| File | Does |
|---|---|
| `config-schema.js` | The 35 fields, 9 layers, and the two routes |
| `suggest.js` | Brief → a filled config. Rule-based, not a model call |
| `compose.js` | The ONE place a config becomes (tokens, manifest) |
| `build.js` | `renderPage()` - the pure renderer. Also the CLI |
| `blocks/*.js` | 21 block types, 42 variants. Pure functions |
| `tokens/*.json` | 4 style packs, real values from real systems |
| `copy.js` | The voice layer: derive what the brief supports, mark the rest |
| `skills.js` | Assigns each unwritten line to the agents-final skill that writes it |
| `scaffold.js` | Copies a project out of the bank, with transitive block deps |
| `project.js` | The ONE path from a config to a project on disk |
| `factory.js` | Front door |
| `studio.js` / `studio.html` | The visual editor |
| `vds-bridge.js` | The OPTIONAL seam to VDS governance |
| `figma-spec.js` / `figma-push.js` | Figma spec sheet and project record |
| `tests/` | 68 tests, run by `make test-factory` |

---

## Rules this codebase is built on

Each of these was learned by breaking it. They are worth reading before changing
anything, because most of them look like fussiness until you have hit them.

**A control the renderer ignores is a control that lies.** `density`, `typeScale`,
`borderWeight`, `elevation` and the whole voice layer all sat in the schema, rotatable
in the studio, changing nothing. `tests/compose.test.js` now fails if any enum option
stops moving the output.

**The preview must not lie.** The studio renders through the same `renderPage()` that
`build.js` calls. `tests/project.test.js` asserts the preview's body and CSS are
byte-identical to what the project compiles. A second renderer would drift, and the
carousel would show something that does not build.

**Derive, do not store.** Block dependencies are read out of the `require()` calls in
the source, not kept in a list. A hand-kept list is one more thing that can disagree
with the code, silently.

**Derive, or MARK - never invent.** `copy.js` writes what the brief genuinely supports
and emits `CONFIRM: <what is needed>` for the rest. Invented filler that reads finished
is worse than an honest blank, because nobody goes back for it. The convention is
Balmoral's own (`site/build/templates.js`).

**Count both markers, or the audit undercounts.** `copy.js` writes `CONFIRM:`;
`scaffold.js` leaves "Replace this…". Counting only the first reported 12 lines to
write on a page that had 17. An undercount reads as a finished audit.

**A gate that cannot fail is not a gate.** `node --test tests/*.test.js` EXITS 0 WHEN
THE GLOB MATCHES NOTHING - measured, not assumed. `tests/gate.js` asserts a floor on
files and tests actually run. Every check here has a test proving it can fire.

**Show the specimen, not the string.** `figma-spec.js` draws every option and marks the
chosen one. Values are computed from the build (`tests/spec.test.js` pins them against
`cssVars()`), because a sheet showing a value the CSS does not use looks authoritative
and is wrong. Motion is REFUSED rather than faked - a still frame cannot show easing.

**Check the red seed landed before reading the result.** A negative control whose `sed`
pattern silently matches nothing exits 0, which is indistinguishable from a gate that
cannot fire. Three seeds here "proved" the Figma-pairing test was dead; the pattern was
missing one space. Assert the seed is in the file, then run.

**Figma frames default to opaque white.** Containers need their fill cleared or they
hide the ground. But some frames' fill IS the specimen - a button with no fill is a
label. Name those `spec:*` at creation and skip them.

**`resize()` resets sizing modes.** Call it BEFORE setting `primaryAxisSizingMode`, or
every variant collapses. Eight Figma component variants were 92px tall because of this.

---

## The two routes

Not one flow with a toggle. A marketing site and an app want different layers and
compile differently.

- **marketing-site** - blocks stack full width. 12 block types.
- **saas-app** - manifest carries `layout: 'app'`; `renderPage` builds a real shell
  with the sidebar as a rail. 11 app-surface block types: the five priority components
  from Opbox's `COMPONENT_INVENTORY.md`, plus four chosen by MEASURED demand across
  217 real Opbox routes rather than by guess - formfield (input 54 + label 46 +
  textarea 23 + select 22), emptystate (49), pagestate (loading 45 + error 38) and
  confirmdialog (16). Each is also a Modular Play the Playbook already names: Empty
  States, Loading Feedback, Fail Safe. The measurement and the strategy layer agree.

98 of the 109 cataloged SaaS component types do **not** exist in code. A saas project
gets `SAAS-COMPONENTS.md` recording that honestly rather than implying otherwise.

## Governance is opt-in, both ways

VDS works with no knowledge of site-factory, and site-factory without `--vds` writes no
`.vds/` at all. With `--vds`, `vds-bridge.js` does what `vds init` alone cannot: point
the surface at what the project actually ships. Without that repointing the `.vds/` is
PRESENT BUT BLIND - measured: 3 proofs precondition-fail, the rest return
`rows_considered: 0`.

## The writing

`copy.js` is not a language model and does not pretend to be. The craft lives in the
agents-final content skills; `skills.js` emits `copy-brief.json` in the shape they read
and assigns each unwritten line to the skill that writes it.

`brand.audience` means **the person who reads the page**, which is not always the buyer.
Every skill in the pack takes it as the buyer. On a page whose job is pre-qualifying a
referral, the reader is the referrer and the buyer never visits. Get this wrong and all
seven skills write to the wrong person.

Grade an existing line **before** generating alternatives. Every new line is optimised
against the rubric; an incumbent carries what the rubric cannot see. See
`~/Projects/Balmoral/docs/HEADLINE-LAB-RUN.md` for a real run where scoring last nearly
replaced a headline that was working.
