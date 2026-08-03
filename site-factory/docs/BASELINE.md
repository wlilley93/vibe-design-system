# The baseline: what we generate, measured against Uber Base

The list of components site-factory should build, derived rather than opinionated. Every
count here comes from `vendor/uber-base-keys.json`, harvested out of the Base Gallery Figma
file (`l2llhOXNz1bM4aoLKKb5qi`, page `3:4200`) by resolving all 3,504 instances to their main
components. Re-derive with the script in that file's `_how` field; do not hand-edit the
numbers here.

**92 component sets, 3,211 variants.** That is the vocabulary a mature product design system
declares, and it is the yardstick.

> **The 92 is this document's TIERING SCOPE, not the size of Base.** Corrected 2026-08-03
> against a depth-unlimited export: Base declares **at least 168 named component sets and
> 4,335 variants** (`vendor/uber-base-inventory.json`). The 3,504 above is the count of
> instances *not nested inside another instance*; the page holds **32,123** in total, and a
> top-level walk cannot see the other 28,619. Four readings of this one file each corrected
> the last and each was cropped the same way - see `docs/FIGMA-ROUND-TRIP.md`, "A read has a
> REACH".
>
> Nothing in the tiering below is withdrawn. **76 further sets carrying 698 variants are
> simply untiered**, listed in the inventory's `untiered_backlog`, and the largest of them -
> `Input element / Text field` (74), `Input element / Select` (36), `Input element / PIN slot`
> (31) - are form primitives, which is exactly where a SaaS surface needs an answer. The
> judgement in this document is sound about the 92 it judged. It is silent, not negative,
> about the rest.

## What Base is, and what it is not

Base is Uber's product design system: Jeremy Mickel's typeface, a deep date/time layer, mobile
navigation, tiles, tags. It is worth taking seriously as a vocabulary and as a type structure.

**It declares no data surface.** Probed all 148 instance names and 122 frame names for the
words that would have to appear: **no table, no grid, no data, no sort, no filter, no
toolbar, no sidebar** - and none of the five components the 224-route Opbox measurement put
first (object table, object view, inspector, facet strip, master-detail). The `column L` and
`column R` hits are layout frames, not table columns.

So Base is the baseline for **controls, messaging and type**, and site-factory's own
Opbox-measured taxonomy remains the baseline for **the data surface**. Adopting Base wholesale
would trade a measured SaaS priority order for a travel-app one. The two are complementary and
the split is deliberate.

## 43 of 92 are already answered

(43 of the 92 tiered here. Against Base's full 168 named sets, 76 are untiered and unassessed.)

Each existing block, and the Base names it covers:

| site-factory block | answers |
|---|---|
| `cta` | Button, Button group, Button dock, Sliding button, Timed button |
| `formfield` | Text field, Text area, Select, Search field, Password field, PIN code, Country field, Field group, Stepper field, Stepper, File drop |
| `card` | Card element - Media item / Line item / Button item, Tile |
| `segmentedcontrol` | Tabs, Tab element (Wide/Narrow x Selected/Unselected) |
| `confirmdialog` | Dialog, Modal full screen, Modal sheet |
| `nav` | Navigation header, Bottom navigation, Breadcrumbs element |
| `objectview` | Sheet header, Section heading |
| `inspector` | List item |
| `facetstrip` | Tag |
| `pagestate` | Progress bar - Indeterminate, Progress circle |
| `toast` | Snackbar |
| `emptystate` | Empty state |
| `faq` | Accordion |
| `sidebar` | Side navigation item |
| `testimonials` | Star rating |
| `team` | Avatar |

That coverage is not a coincidence: both systems converged on the same controls because the
controls are the part that is not optional.

## Tier 1: DONE (17 sets, 952 variants in Base; 16 drawn here, 46 variants)

The gaps that a SaaS surface actually needs. Ordered by the variant count Base gives them,
which is the closest thing to a declared importance.

| Base set | variants | why it matters here | drawn as |
|---|---|---|---|
| Pagination | 423 | a table without pagination is a table that lies about its length | 4: Layout Compact/Full x State Enabled/Disabled |
| Page controls | 162 | the mobile/compact twin of the above | 2: Enabled/Disabled |
| Banner | 72 | page-level state that is not an error | 4 tones |
| System banner | 64 | the same at app level, above the shell | 4 tones: warning, negative, accent, positive |
| Message card | 60 | in-flow messaging, not an overlay | 2: with and without artwork |
| Menu item | 57 | 18 control variants measured (checkbox, switch, drag, group x 3 sizes) | 6: Size x Enabled/Hover/Disabled |
| Typography | 36 | taken as STRUCTURE, not as components - `docs/GOAL.md` S3 and the type ramp | the ten-step ramp |
| Notification badge | 16 | dot, count, overflow | 3 |
| Menu | 12 | the container the items sit in | 2: Small 207, Medium 242 |
| Draggable list | 10 | reorderable rows, which a table needs and no marketing page does | 2: with and without artwork |
| Switch | 8 | | 2 |
| Check | 8 | unchecked / checked / indeterminate | 3 |
| Tooltip | 8 | | 2 |
| Progress bar - Determinate | 6 | | 4: Size x Label/None |
| Radio | 4 | | 2 |
| Divider | 3 | full and cell-inset | 2 |
| Progress steps | 3 | multi-step forms, which `formfield` has no answer for | 2: horizontal and vertical |

All 17 are settled: 16 drawn as component sets on the `Base (redrawn)` page, 46 variants in
total, and Typography taken as the type ramp rather than as components. Re-derive the count
from the file rather than trusting this line: list `COMPONENT_SET` children of that page.

### What the redraws taught, beyond the components

Four things, each found by measuring rather than by looking, and each cheaper to write down
than to rediscover.

- **Base's leading is a function of role, not size.** Already taken; see below.
- **`resize()` and `layoutMode` each reset both sizing modes to AUTO.** A 36 square chip set
  to auto-layout after being resized came out roughly 36x20, which reads as a deliberate
  pill. Nothing errored. Order is: `layoutMode`, then `resize`, then `FIXED`. Two sets were
  rebuilt for this and the fix is now asserted by reading the geometry back: 26 pagination
  chips and 6 step dots confirmed 36 square.
- **A gap in the parent's padding is not a gap the child can grow into.** The vertical
  progress connector was told to grow into the row's remaining height; the row's 20 bottom
  padding had already taken it, so the connector got 1px and read as absent. The row gap
  belongs inside the rail, as the connector's own length.
- **A blanket `findAll` by name crosses variants.** An assertion on connector height matched
  the horizontal variant's connectors, which are legitimately 2px TALL, and refused a build
  that was correct. Scope an assertion to the subtree it is about.

### The open half

Sixteen component sets now exist in Figma with **no code counterpart**, which is the exact
reverse of the divergence BREACH-0007 closed. `FIGMA_NODES` pairs site-factory block types to
component sets and both directions are tested, so nothing is currently claiming these are
paired. Building the code side is the next piece of work, not a gap in the record.

The messaging three cost more than the drawing. Redrawing them needed an ink per tone, and
the Figma variable collection carried one ink where the style packs carry five. Reading the
collection back also showed nine mode values disagreeing with the pack that defines them:
the Base palette landed in `tokens/*.json` and never reached Figma, so every component
already on this page had been drawn in the wrong red. Both faults are closed and paired by
test now (`figma-variables.json` plus three tests in `tests/skills.test.js`), and the whole
episode is on the record as BREACH-0007. The lesson worth carrying: the component-set
pairing had existed for months and covered one of the two dimensions in which a code base
and a Figma file can disagree.

The messaging three cost more than the drawing. Redrawing them needed an ink per tone, and
the Figma variable collection carried one ink where the style packs carry five. Reading the
collection back also showed nine mode values disagreeing with the pack that defines them:
the Base palette landed in `tokens/*.json` and never reached Figma, so every component
already on this page had been drawn in the wrong red. Both faults are closed and paired by
test now (`figma-variables.json` plus three tests in `tests/skills.test.js`), and the whole
episode is on the record as BREACH-0007. The lesson worth carrying: the component-set
pairing had existed for months and covered one of the two dimensions in which a code base
and a Figma file can disagree.

## Tier 2: later (10 sets, 111 variants)

Real but not load-bearing for a data surface: Progress steps elements (vertical 46,
horizontal 10), Segmented slider - Beta 22, Spacer 13, Hint badge 4, Slider - Beta 4,
Progress bar - Stepped 4, Rich text_Beta 3, Progress pill 3, Message card - Carousel 2.

Two of those carry `Beta` in their own name. Base is telling you not to depend on them yet.

## Deferred: 22 sets, 172 variants

Every Date picker and Time picker set (21 of them, 170 variants, including four Pinwheel
components), plus the mobile-only shells. Deferred, not rejected, and the reason is specific:
**a date picker is a component you should not build.** It is the highest-effort, highest-risk
control in any system - locales, time zones, keyboard access, range selection, screen readers -
and there are good accessible implementations to adopt. Base needs its own because Uber ships
in 70 countries. site-factory does not have that problem yet.

## What "getting their components in" actually requires

Two routes, and the difference matters:

**Import by key.** `vendor/uber-base-keys.json` holds all 92 set keys. They are correct and
they do not currently resolve. MEASURED: a real key and a deliberately invalid all-zeros key
returned the SAME `not found` error, and `get_libraries` reports this file subscribes to 8
libraries (Material 3, Simple Design System, the Apple kits) with Base absent and
`libraries_available_to_add` empty. So the keys are right and the subscription is missing.

To fix it, duplicate the Base **library** file - not the Gallery. Every component in the
Gallery is `remote: true`; it holds instances only, so duplicating it gives you 3,504 instances
and no components. Enable the library on this file and the 92 keys resolve immediately.

**Redraw.** What has been done for six sets: read the real geometry out of the source file and
rebuild locally, bound to VDS Tokens. Editable and themed with your packs, at the cost of no
link to Base and no updates. Right for a control you intend to own; wrong for 3,211 variants.

## What was taken and is already in the system

- **The type structure**, which was worth more than the components. Base's leading is a
  function of ROLE, not size: Label and Paragraph share every size and differ only in leading.
  Measured into `vendor/uber-base-typescale.json` and implemented as a ten-step ramp with four
  role leadings. See `docs/GOAL.md` S3.
- **The palette**, measured: `#276ef1`, `#0e1fc1`, `#ffc043`, `#0e8345`, `#9f6402`, `#8fa3ad`.
- **Six control sets**, redrawn from measured geometry.

## What was NOT taken, and why

**Uber Move.** The font's own metadata reads *"This custom font has been licensed exclusively
to Uber"* (MCKL / Jeremy Mickel, 2018). It cannot be the typeface of another product or ship
in generated CSS. Separately, Base runs THREE families - `Uber Move` Bold for display,
`Uber Move Text` for UI and body, `Uber Move Mono` - so a two-weight download of one family
was never parity in the first place. And the Figma MCP executes server-side against the hosted
font catalogue, so a locally installed face is invisible to any script here regardless.
