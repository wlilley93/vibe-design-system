# site-factory

A brief in, a built site out - with the design decisions visible, the writing handed
to the skills that do it well, and the gaps counted rather than hidden.

```
node factory.js ls        what this factory can make, measured from disk
node factory.js studio    the visual editor, http://localhost:4321
node factory.js wizard    the paged CLI
node factory.js new --name "X" --brief "..." [--vds]    one shot, no prompts
node factory.js build manifests/index.json              render one page
```

Zero dependencies. Node's built-in `http` and `node:test`, nothing installed.

---

## The shape

A **config** (36 fields, 9 layers) is composed into **tokens** + a **manifest**, which
a **block** registry renders into static HTML and CSS.

```
brief ──▶ suggest.js ──▶ config (36 fields) ──▶ compose.js ──┬──▶ tokens
                              │                              └──▶ manifest
                              │                                     │
                              ├──▶ copy.js      (the voice layer writes, or marks)
                              ├──▶ skills.js    (what it cannot write, assigned)
                              └──▶ figma-spec.js (one drawn specimen per decision)
                                                                    │
                                                       build.js ◀───┘
                                                          │
                                 dist/index.html + about/contact/404 + site.css
```

| File | Does |
|---|---|
| `config-schema.js` | The 36 fields, 9 layers, and the two routes |
| `suggest.js` | Brief → a filled config. Rule-based, not a model call |
| `compose.js` | The ONE place a config becomes (tokens, manifest) |
| `build.js` | `renderPage()` - the pure renderer. Also the CLI |
| `blocks/*.js` | 43 block types, 86 variants. Pure functions |
| `tokens/*.json` | 5 style packs: 4 real systems plus the wireframe projection |
| `copy.js` | The voice layer: derive what the brief supports, mark the rest |
| `skills.js` | Assigns each unwritten line to the agents-final skill that writes it |
| `scaffold.js` | Copies a project out of the bank, with transitive block deps |
| `project.js` | The ONE path from a config to a project on disk |
| `factory.js` | Front door |
| `studio.js` / `studio.html` | The visual editor |
| `vds-bridge.js` | The OPTIONAL seam to VDS governance |
| `figma-spec.js` / `figma-push.js` | Figma spec sheet and project record |
| `figma-nodes.json` | MEASURED: every page and component set in the master Figma file |
| `figma-variables.json` | MEASURED: the `VDS Tokens` collection, per mode, paired to the style packs |
| `figma-prompts.json` | MEASURED: the fingerprint of every prompt written into Figma verbatim |
| `figma-variants.json` | MEASURED: every set's variant AXIS and values, and whether the code varies the same question |
| `figma-draw.js` | Redraws the library from the register - deterministic, amends in place, binds variables by name |
| `vendor/uber-base-*.json` | The Uber Base harvest (92 sets, 3,211 variants) and each set's tier |
| `docs/FIGMA-MASTER-FILE.md` | What is on each Figma page, and the traps the file has sprung |
| `docs/REFERENCES.md` | Every external system looked at, what was taken, and the three access blockers |
| `docs/DESIGN-TWIN.md` | Why the Figma and code components are twins, and which direction automates |
| `vendor/relume-*.json` | The Relume kit harvest (1,799 sets) and site-factory's coverage against it |
| `tests/` | 104 tests, run by `make test-factory` |

---

## Rules this codebase is built on

Each of these was learned by breaking it. They are worth reading before changing
anything, because most of them look like fussiness until you have hit them.

**A control the renderer ignores is a control that lies.** `density`, `typeScale`,
`borderWeight`, `elevation` and the whole voice layer all sat in the schema, rotatable in
the studio, changing nothing.

That was fixed for those four and the rule was declared closed. It was not: an audit
rendered every option of every enum field and found **nineteen fields, eleven reachable**.
Eight more could be rotated and changed nothing - `pairingStyle`, both motion fields,
`buttonShape`, `tableDensity`, `navigationPattern`, `imageTreatment`, `iconStyle`. The
claim was false for nearly half the schema, because the test checked the four fields the
rule had named instead of the class the rule described.

Six are now wired: button radius separate from panel radius (a pill button beside a sharp
panel is a real language), row padding as a multiplier on `--space`, real CSS transitions
with `prefers-reduced-motion` winning over an expressive setting, a paired body face, and
`navigationPattern` choosing the shell rather than the route forcing it. Two are exempt and
SAY SO IN THE TEST with a reason: nothing renders an image or an icon yet.

`tests/compose.test.js` now walks every enum field on both routes and fails on any whose
options all produce the same bytes - and fails the other way too, if a field listed as
exempt starts working. An exemption you can read is a decision; an uncovered field is a gap
nobody knows about.

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

**A rule and its check must be the same size.** The stylesheet rule said "no hex, no font
name, no px literal outside the `--space` multiplier" and offered a grep for HEX as its
evidence. Sixteen px declarations lived under it for weeks. The same shape as the
controls-that-lie failure above: a rule stated broadly, a check covering a named subset, and
nobody comparing the two. When you write a rule, write the check that fails if the rule is
broken ANYWHERE, not the one that passes on the instance that prompted it.

Those sixteen were five roles with drifted values, not sixteen decisions - a page frame
(`--container`), long prose (`--measure-wide`), a form column (`--measure-form`), a reading
measure (`--measure`) and a narrow centred panel written 480, 460 and 420 in three places for
the same job (`--measure-narrow`). The measures scale with `--type-scale` and not `--space`,
because a measure is characters per line: bigger type needs a wider column to hold the same
line length, where folding it into spacing would shrink both and compound the problem.

**Show the specimen, not the string.** `figma-spec.js` draws every option and marks the
chosen one. Values are computed from the build (`tests/spec.test.js` pins them against
`cssVars()`), because a sheet showing a value the CSS does not use looks authoritative
and is wrong. Motion is REFUSED rather than faked - a still frame cannot show easing.

**Check the red seed landed before reading the result.** A negative control whose `sed`
pattern silently matches nothing exits 0, which is indistinguishable from a gate that
cannot fire. Three seeds here "proved" the Figma-pairing test was dead; the pattern was
missing one space. Assert the seed is in the file, then run.

**A vacuous proof reports a pass.** `rows_enforced: 0` with `rows_considered: 8` is a
gate that looked at everything and decided nothing, and it prints green. Read
`rows_enforced`, never the status. Four of site-factory's components rendered a state in
code that the Figma file never drew, and the proof said nothing until the register
declared the states.

**Derive a state from the code, and its drawing from the file.** `states.required` comes
from markers the block renders CONDITIONALLY - `field--invalid` sits behind a ternary, so
one component renders with and without it; `pagestate--error` is flat, so it is variant 2
of 2 and not a state. `states.drawn` is measured out of Figma into `figma-states.json`
and cites the layer. Claiming a state is drawn to quiet the gate is the exact defect the
gate exists to catch.

**Figma frames default to opaque white.** Containers need their fill cleared or they
hide the ground. But some frames' fill IS the specimen - a button with no fill is a
label. Name those `spec:*` at creation and skip them.

**`resize()` resets sizing modes.** Call it BEFORE setting `primaryAxisSizingMode`, or
every variant collapses. Eight Figma component variants were 92px tall because of this.

---

## A site is more than a page

The factory built exactly ONE page, and every link in it was `href="#"`. The field named
`sitemap` is a BLOCK sequence - its own label says "Block sequence (type:variant,
ordered)" - so nothing in the config ever described a second page. A `notfound` block type
existed with no page for a 404 to be.

`strategy.pages` is the sitemap. A marketing site gets home, about, contact and a 404;
an app gets one page, because an app shell routes inside itself. A config with no `pages`
gets one page built from `sitemap`, exactly as before.

- **Home is `index.html`**, because that is what a server returns for `/`. A nav linking
  `home.html` against a host serving `index.html` 404s on its own logo.
- **The nav and footer links are DERIVED from the page set**, so adding a page cannot
  leave the navigation behind - which is the failure the one-page version made permanent.
- **The 404 is off the nav on purpose.** A 404 you can navigate to is not a 404.
- **A secondary page keeps home's frame** - the same nav variant, the same footer variant
  - so the site does not change shape when you click a link.
- **CTAs point at contact, DERIVED from the page set.** A site without a contact page
  keeps `#`: inventing a URL that 404s is worse than an honest dead anchor.
- **One `site.css` for the whole site.** A four-page build wrote four byte-identical
  stylesheets (measured, same md5). Identical contents under different names are still
  separate downloads.

`tests/project.test.js` walks every `href` in every built page and fails on one that
points at a missing file, or at a bare `#`. Nothing caught the original defect because
nothing ever asked where a link went.

## The two routes

Not one flow with a toggle. A marketing site and an app want different layers and
compile differently.

- **marketing-site** - blocks stack full width. 12 block types.
- **saas-app** - manifest carries `layout: 'app'`; `renderPage` builds a real shell
  with the sidebar as a rail. 14 app-surface block types: the five priority components
  from Opbox's `COMPONENT_INVENTORY.md`, plus four chosen by MEASURED demand across
  217 real Opbox routes rather than by guess - formfield (input 54 + label 46 +
  textarea 23 + select 22), emptystate (49), pagestate (loading 45 + error 38) and
  confirmdialog (16). Each is also a Modular Play the Playbook already names: Empty
  States, Loading Feedback, Fail Safe. The measurement and the strategy layer agree.

95 of the 109 cataloged SaaS component types do **not** exist in code. A saas project
gets `SAAS-COMPONENTS.md` recording that honestly rather than implying otherwise.

## Governance is opt-in, both ways

VDS works with no knowledge of site-factory, and site-factory without `--vds` writes no
`.vds/` at all. With `--vds`, `vds-bridge.js` does what `vds init` alone cannot: point
the surface at what the project actually ships. Without that repointing the `.vds/` is
PRESENT BUT BLIND - measured: 3 proofs precondition-fail, the rest return
`rows_considered: 0`.

**What the repointing actually buys, measured on a generated app project.** The registry
is CLOSED at eleven kinds (`vds proof --list`), all eleven implemented. Five enforce here;
six are vacuous, and the reasons are different:

| kind | rows enforced | |
|---|---|---|
| `no_stored_values` | 20 | |
| `reconciliation` | 14 of 35 | |
| `states` | 7 | |
| `contrast` | 7 | |
| `ledger_staleness` | 1 | |
| `retirement_drain` | 0 | correct - nothing is deprecated |
| `token_pin` | 0 | correct - no pins yet |
| `register_completeness` | 0 | **structural, see below** |
| `composition` | 0 | **structural, see below** |
| `parity` | 0 | **structural, see below** |
| `screen_parity` | 0 | no screen declares a required arrangement |

That last row is the one this table used to omit, which made "five of ten" a count over a
registry that holds eleven. A table presented as the whole accounting has to BE the whole
accounting; a missing row reads as a kind that does not exist.

The three structural ones are not a configuration mistake and no setting fixes them.
`vds-scan` parses ESM `import` statements; a scaffold resolves its blocks through a
dynamic `require()` because dropping a file in `blocks/` is meant to register it. There
is no static import graph to walk, so no screen yields a governed reference. `parity`
additionally wants a capitalised or default export, and a block exports a kebab-keyed
object. Recorded rather than papered over, because a vacuous proof reports a pass.

`states` used to be vacuous too, and that one WAS a real gap: `required: []` meant 8 rows
considered and 0 enforced. A proof that considers every row and enforces none is switched
off. It is derived now - see below.

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
