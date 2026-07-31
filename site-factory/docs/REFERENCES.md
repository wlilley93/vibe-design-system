# References

Every external system this programme has looked at, what was taken from it, and whether it is
actually reachable. Kept because a reference nobody wrote down is a reference somebody
re-derives, and because several are blocked on an access grant rather than on work.

Access states are MEASURED, not assumed. Each was probed and the error recorded verbatim.

## Design systems and component libraries

| Reference | What it is | Access | Taken |
|---|---|---|---|
| **Uber Base** ([Gallery `l2llhOXNz1bM4aoLKKb5qi`](https://www.figma.com/design/l2llhOXNz1bM4aoLKKb5qi)) | Uber's product design system. 92 component sets, 3,211 variants | READ ok, IMPORT blocked | The control and messaging baseline. Palette, the role-based type ramp, 16 sets redrawn |
| **Relume Figma Kit v3.7** ([`NV8SrzLfRBiPfcwGyDGHJd`](https://www.figma.com/design/NV8SrzLfRBiPfcwGyDGHJd)) | Marketing section library. 74 pages, 1,799 sets, 3,634 variants, 58 categories | READ ok | The MARKETING SECTION baseline, and the four-view workspace pattern. Four sections built from its gaps |
| **Material 3 Design Kit** (the LIBRARY, v1.25) | Google's Material 3 kit. Its component descriptions carry the semantic contract in prose | **SUBSCRIBED** on the master file. Search returns real component keys | Nothing yet. A denominator for W4, and a source of contract prose to compare against |
| **Material You UI Kit** ([`iOwtVorRqWd8UlTDNTWxPf`](https://www.figma.com/design/iOwtVorRqWd8UlTDNTWxPf)) | The community FILE of the above | **BLOCKED** as a file, and it does not matter: the library is subscribed | See above. The file was never the way in |
| **Figma Simple Design System** (the LIBRARY) | Figma's own reference system. 400+ components, light/dark variables, and its code is published at [`figma/sds`](https://github.com/figma/sds) under **MIT** with Code Connect templates and `scripts/tokens/tokensCodeSyntaxes.js` | **SUBSCRIBED** on the master file | THE extraction test subject: both twins exist, openly licensed, and neither is mine. See `DESIGN-TWIN.md` |
| **Apple UI kits** (iOS/iPadOS 26 + 27, macOS 26 + 27, watchOS 26, visionOS 26) | Apple's official kits | **SUBSCRIBED** on the master file | Nothing yet. Recorded because they are reachable and were not known to be |
| **Glow UI Preview 1.8** ([`Jx78qVnHi73p1yTuoAtBYb`](https://www.figma.com/design/Jx78qVnHi73p1yTuoAtBYb)) | A UI kit | **BLOCKED**, not subscribed as a library either | Nothing yet |
| **Relume UI** (`@relume_io/relume-ui@1.3.1`) | React component library on Radix. 136 exports, ~30 distinct components | npm, public | Nothing yet. It is the PRIMITIVE layer; the sections are in the paid library |
| **Relume Tailwind preset** (`@relume_io/relume-tailwind@1.3.0`) | Their token preset | npm, public | Nothing yet |
| **Park UI** (park-ui.com) | Components on Ark UI + Panda CSS. React, Solid, Vue | public | Nothing yet |
| **Reshaped** (reshaped.so) | React + Figma libraries claimed as a "100% match"; semantic tokens, Figma variables mode, theme generated from one colour, Tailwind import, WAI-ARIA, RSC | npm `reshaped`, licence not stated on the page | Nothing yet. THE strongest off-the-shelf answer to the twin problem - see `DESIGN-TWIN.md` |
| **Relume Icons** (icons.relume.ai) | Icon set, presented for copy-paste into Webflow | public page, but it states no count, no grid, no formats and no licence | Nothing. Not adoptable on what the page discloses: an icon set with no stated grid or licence cannot be put in a design system |
| **Opbox design** (own repo) | The product system. 6 themes, 62 Figma colour tokens, 56-entry component map | local | Documented as a system page. `statusMeaning` (aligned/floor/migrating) is the idea worth stealing |
| **Opbox marketing** (own repo) | "Will Style", authored in oklch | local | Documented as a system page |
| **Jellytot** (own repos) | Brand system with three disagreeing sources of truth | local | Documented as a system page; style pack |

### The blockers, each with its measured error

**Uber Base import.** `importComponentSetByKeyAsync` returns
`Component set with key "..." not found` for a genuine harvested key AND for an impossible
all-zeros key, byte-identical. So the failure is the missing library subscription, not the
keys. FIX: duplicate the Base **library** file, not the Gallery - every component in the
Gallery is `remote: true`, so duplicating it yields 3,504 instances and no components - then
enable it on this file and all 92 keys resolve at once.

The first version of that test used keys I had reconstructed rather than copied, and so
established only that invented keys do not resolve, which nobody doubted. Recorded because a
control that does not control produces a confident answer to a question it never asked.

**Every community Figma FILE, unless it has been duplicated first.** Both the Material You kit
and Glow UI refuse every tool with the same line:
`Looks like you don't have edit access to this file.` That includes the READ tools - `get_metadata`
and `get_variable_defs` fail exactly as `use_figma` does - so such a file cannot be inspected at
all, only viewed in a browser by a human.

That is a fact about FILES, and the word matters: the same system reached as a subscribed LIBRARY
answers normally. See the correction below the table, which is why this heading now says FILE.

Four states, not three, and naming them saves probing each new link:

| State of the file | What works |
|---|---|
| Duplicated into your drafts | everything. This is why the Relume kit could be harvested in full: 74 pages, 1,799 sets |
| A library you can view but have not duplicated | nothing, not even reads. Glow UI |
| Readable but its LIBRARY not subscribed | reads work, `importComponentSetByKeyAsync` does not. Uber Base's Gallery |
| **Its LIBRARY subscribed on a file you own** | **`get_libraries` lists it and `search_design_system` returns real component keys - even though the source file itself still refuses every read.** Material 3, Simple Design System, six Apple kits |

FIX for the first three: **duplicate the file to your own drafts** (and for Base, duplicate the
LIBRARY file rather than the Gallery, then enable it).

### CORRECTION: the fourth row, and why three rows was the wrong shape

The table above had three rows and reported Material You as reachable by nothing. That conflated
two different access paths. A community FILE and a subscribed LIBRARY are separate artefacts with
separate permissions, and the second was never probed.

Measured on the master file (`4pPUFvaPdqYzPquBusSfWl`), `get_libraries` returns **eight libraries
already added**: Material 3 Design Kit, Figma's Simple Design System, and six Apple kits.
`search_design_system` scoped to their library keys returns real assets with real
`componentKey` values - 7 button component sets from Simple Design System, 20 button and toggle
sets from Material 3 - and Material's set descriptions carry the semantic contract in prose:
*"Five color options: elevated, filled, tonal, outlined, and text ... Two shape options: round and
square"*.

So the earlier reading was not wrong about what it tested. It was a claim wider than its test: a
file-level refusal was written up as a system-level unreachability, and the fix was recorded as a
user-owned duplication that turns out not to be needed for most of the list.

**And the open question is now SETTLED, with a negative control.** The correction above left one
thing untested - whether `importComponentSetByKeyAsync` actually RESOLVES a key that
`search_design_system` returned, because search returning a key is not the same as an import
resolving it. Probed 2026-07-31 against Simple Design System, which is subscribed:

| key | result |
|---|---|
| `0000...0000` (impossible, the control) | `Component set with key "0000..." not found` |
| `cc8b558d...` Button | **RESOLVED** - 18 variants, axes `Variant / State / Size`, `remote: true` |
| `e098805c...` Icon Button | **RESOLVED** - 18 variants, same three axes |
| `3d307317...` Button Danger | **RESOLVED** - 12 variants, same three axes |

The control ran FIRST and was refused, so the three passes are not an API that returns something
for anything. And the refusal text is BYTE-IDENTICAL to the one Uber Base gives, which closes the
loop on that row: Base's keys fail for the reason recorded there and no other, and a subscribed
library's keys import cleanly.

**Consequence for Base, and it is a two-step fix rather than one.** Duplicating the library file
puts a copy in your drafts; it does not make it a library this file can use. `get_libraries` on
the master file lists eight subscribed libraries and `libraries_available_to_add` is EMPTY, so a
duplicate exists somewhere and nothing here can reach it. Either publish the duplicate as a
library and enable it on this file, or hand over its file key - a file in your own drafts is
fully readable by every tool, which is the first row of the table above, so the key alone
unblocks the harvest even with no library subscription at all.

Uber Base is now the only entry genuinely blocked on an action nobody but the Principal can take.

**Figma Code Connect.** `get_code_connect_map` returns
`You need a Dev or Full seat on an Organization or Enterprise plan to use Code Connect.`
This matters more than it looks: Code Connect is Figma's own answer to binding a component in
a file to a component in a repo, and it is the thing usually meant by "connecting the two".
It is unavailable on this plan.

## What actually connects Figma to the code here, given that

> The full argument, and the reason the crossing only works in one direction, is in
> [`DESIGN-TWIN.md`](DESIGN-TWIN.md). Short version: a Figma component and a code component are
> TWINS rather than stages, every vendor that seems to have solved it has staffed both platforms
> instead, and the only direction that automates is code to Figma - which is how this file is
> already built.


Not a component library. Park UI, Relume UI and Material You are all COMPONENT RUNTIMES: they
answer what a button is made of, not whether the button in the file and the button in the repo
agree. Adopting one would be an upgrade to how site-factory renders (it templates strings; they
have real component runtimes with proper a11y primitives) and would move the Figma question not
one inch.

What connects the two is a single source of truth plus a check that fails on divergence, and
that already exists here:

- `figma-variables.json` pairs every emitted `--color-*` and `--radius-*` to a Figma variable,
  per mode, BY VALUE. It found four missing ink variables and nine drifted values, and
  everything drawn before that fix was the wrong red.
- `figma-nodes.json` pairs every block type to a component set that exists, in both directions,
  with unpaired sets declared and a reason required.
- `figma-prompts.json` fingerprints every prompt written into the file against the file it came
  from.

Each states what it is blind to. That is the bridge, it needs no plan tier, and Code Connect
would make it nicer rather than make it possible.

## Tooling and agent layers (cloned, queued for combing)

All cloned to `/var/tmp/claude/refs/`. Sizes are the working tree.

| Reference | What it is | Why it matters here |
|---|---|---|
| [southleft/ds-contracts-poc](https://github.com/southleft/ds-contracts-poc) (274M, 3,701 files) | The contracts PoC. `contracts/ core/ extract/ figma-sync/ parity/ conformance/ evals/ catalog/ dashboard/ playground/` | The closest published relative to VDS. Being combed |
| [bennypowers/cem](https://github.com/bennypowers/cem) (35M, Go) | Custom Elements Manifest tooling: generate, list, export, `breaking`, `health`, an LSP | THE component contract as an ADOPTED STANDARD rather than a bespoke schema. It also ships `breaking` (API-break detection) and `health`, which are VDS concerns under different names |
| [murphytrueman/design-system-ops](https://github.com/murphytrueman/design-system-ops) (5.7M) | A design-system OPS pack of agent skills and slash commands: `drift-check`, `token-audit`, `governance-review`, `component-audit`, `docs-coverage`, `release-check`, `system-health`, `full-diagnostic`, `codemod-generator`, `cicd-integration` | Someone else's answer to the same governance surface, expressed as agent skills rather than a kernel. `drift-check` and `governance-review` are directly comparable to `vds proof` and `vds doctor` |
| [southleft/story-ui](https://github.com/southleft/story-ui) (3.2M) | Story generation with an `mcp-server/`, `story-generator/`, `cli/` | An MCP server for a design-system task, by the same authors as the contracts PoC |
| [contains-studio/agents](https://github.com/contains-studio/agents) (708K) | A subagent collection organised by function: `design/ engineering/ marketing/ product/ testing/ studio-operations/` | Comparable to the agents-final skill library this programme already writes briefs against |
| [Preline Figma](https://preline.co/figma/) | Tailwind component library with a paired Figma kit | Another instance of the same pattern: one vendor owning both twins |

Four of these are queued rather than combed. The first fan-out is combing the contracts PoC,
Reshaped, Park UI and Relume UI; a second pass takes cem, design-system-ops, story-ui and the
agents collection. They are queued rather than run in parallel deliberately: this box has 18GB
and overlapping two many-agent fan-outs is what took the desktop down on 2026-07-29.

## Prior art on the station itself

| Reference | What it is | Access | Relevance |
|---|---|---|---|
| [southleft/ds-contracts-poc](https://github.com/southleft/ds-contracts-poc) + [playground](https://ds-contracts-playground.pages.dev) | Design-system CONTRACTS proof of concept, July 2026. A JSON/YAML contract with no visuals and no code; generates code files directly and the Figma side via a plugin; three-way checker over contract, Figma and code. Run against Shoelace, Mantine, Carbon and Polaris | public | The closest published relative to VDS. Ahead on generation and on having an on-ramp for existing libraries; behind on governance. See `DESIGN-TWIN.md` |
| [Christine Vallaure, "Design system contracts"](https://christinevallaure.substack.com/p/design-system-contracts-the-component) | The article. "Whichever you crown, the other becomes a copy that someone has to keep updating"; "Neither becomes the original; both are printouts of the same recipe" | public | The design twin, arrived at independently. The problem's name in the wild is **design system parity** |

## Reference sites

| Site | Why | Read |
|---|---|---|
| [minimal-square.com](https://www.minimal-square.com/) | The register wanted for Jellytot. Itself Relume-built: its testimonial is from a Relume co-founder and Relume is in its own tech-stack strip | Eight sections: hero, 4-col value grid, testimonial, logo strip, project grid, two-tier pricing, FAQ, footer. Six were already built; the two that were not - the logo strip and the project grid - are exactly two of the four gaps now closed |

## The workspace pattern, and where it came from

Relume's kit ships four dedicated pages - **Sitemap, Wireframe, Style Guide, Design** - and
three of the four are EMPTY in the community file. They are working surfaces, not deliverables.
That is the whole argument for building them as PROJECTIONS: `Sitemap` and `Wireframe` in this
programme are both derived from `manifests/*.json`, verified to agree by reading the section
lists back out of each drawing, so neither can drift from the built site.
