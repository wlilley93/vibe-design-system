# Opbox Design System — Figma Import Kit (Geist Edition v1.2)

**Why this isn't a `.fig`:** `.fig` is Figma's proprietary binary format — only Figma can author one. This kit is the honest equivalent: everything imports cleanly into Figma as editable frames, text layers, and variables, and you save it **as** a `.fig` from inside Figma in one step (see step 4).

## Contents

| File | What it becomes in Figma |
|---|---|
| `01-foundations.svg` | Colors (light + dark), typography specimens, spacing, radii, elevation |
| `02-components-core.svg` | Buttons (all variants × states, incl. GatedButton + blocked reasons + gate chips), badges, StatusDot, facet chips, inputs, selects, toggles, RID chip, kbd, tooltip |
| `03-components-data.svg` | Dense Table with selection/progress/blocked-first, object rows (Entity), append-only Timeline with HEAD, progress/gauge/skeleton, Cap Table |
| `04-overlays.svg` | Modal, Destructive Action Modal, the Inspector (Sheet + traversal stack), dropdown + context menu, Command Menu (⌘K), toasts ×4, note, banner, empty state |
| `05-patterns.svg` | Master-detail (See-Decide-Act) assembly annotated, the 14-hue EntityIcon set, ontology graph vocabulary, DAG with release gate |
| `06-dark-mode.svg` | Key components under the dark token set — ink polarity flip, tonal elevation |
| `tokens.figma.json` | All tokens as **Figma variables** via the Tokens Studio plugin — Light + Dark themes, colors, type, spacing, radii, shadows |

## Import (5 minutes)

1. **Fonts.** In Figma, both `Geist` and `Geist Mono` are available via Google Fonts — no install needed in the browser app. (Desktop app with local fonts: grab them from `vercel.com/font` or `npm i geist`.)
2. **Pages.** Create a new Figma file. Make six pages named after the SVGs. Drag each `.svg` onto its page (or File → *Place image* → it imports as vectors, not a bitmap). Each arrives as one frame with real text layers and groups — ungroup/restructure into components as you wish.
3. **Variables.** Install the **Tokens Studio for Figma** plugin (free tier is fine). Plugin → Tools → *Load from file/JSON* → pick `tokens.figma.json`. Two themes arrive: **Light** and **Dark**. Use *Styles & Variables → Export to Figma* to materialize them as native Figma variables/styles.
4. **Save the `.fig`.** File → *Save local copy…* → you now have `opbox-design-system.fig` containing everything.

## After import — componentization order

The SVGs are specimens; turn them into a real library in this order (each earlier item is consumed by later ones):

1. Color/type styles from the variables (step 3 does most of this)
2. `Badge` + `StatusDot` (variants: intent × subtle/solid)
3. `Button` (variants: primary-ink / secondary / tertiary / destructive / accent × 5 states) → then `GatedButton` (adds gate chip + blocked-reason slot)
4. `RidChip`, `EntityIcon` (14 hue variants), `FacetChip` (active/inactive)
5. `Table` row + header as components → assemble the dense table
6. `Modal`, `Sheet/Inspector`, `Menu`, `CommandMenu`, `Toast`
7. The master-detail assembly as a frame template

## The rules the pixels encode (don't lose these when componentizing)

- **Ink acts, blue selects** — primary buttons are `#171717`; `#006bff` is selection/focus/links only.
- **blocked ≠ disabled** — a gated button that can't fire shows *why*, adjacent, in danger text.
- **Counts filter** — the facet chip is the only sanctioned "stat"; if it doesn't filter, it doesn't ship.
- **RIDs everywhere** — mono, middle-truncated, one-click copy.
- **Entity hues live only** in EntityIcon and graph edges — never on chrome or badges.
- **Dark mode is tonal** — raised surfaces + hairlines, not heavier shadows; ink flips polarity.
- **32px dense tier** is the app default; 40px only for portal/auth.

Full written spec: `opbox-design/` (26 pages), page-by-page app transformation: `opbox-transform/` (197 routes), both in `opbox-deliverables.zip`.
