# References

Every external system this programme has looked at, what was taken from it, and whether it is
actually reachable. Kept because a reference nobody wrote down is a reference somebody
re-derives, and because three of these are blocked on an access grant rather than on work.

Access states are MEASURED, not assumed. Each was probed and the error recorded verbatim.

## Design systems and component libraries

| Reference | What it is | Access | Taken |
|---|---|---|---|
| **Uber Base** ([Gallery `l2llhOXNz1bM4aoLKKb5qi`](https://www.figma.com/design/l2llhOXNz1bM4aoLKKb5qi)) | Uber's product design system. 92 component sets, 3,211 variants | READ ok, IMPORT blocked | The control and messaging baseline. Palette, the role-based type ramp, 16 sets redrawn |
| **Relume Figma Kit v3.7** ([`NV8SrzLfRBiPfcwGyDGHJd`](https://www.figma.com/design/NV8SrzLfRBiPfcwGyDGHJd)) | Marketing section library. 74 pages, 1,799 sets, 3,634 variants, 58 categories | READ ok | The MARKETING SECTION baseline, and the four-view workspace pattern. Four sections built from its gaps |
| **Material You UI Kit** ([`iOwtVorRqWd8UlTDNTWxPf`](https://www.figma.com/design/iOwtVorRqWd8UlTDNTWxPf)) | Material 3 kit advertising tokens + preview | **BLOCKED** | Nothing yet |
| **Relume UI** (`@relume_io/relume-ui@1.3.1`) | React component library on Radix. 136 exports, ~30 distinct components | npm, public | Nothing yet. It is the PRIMITIVE layer; the sections are in the paid library |
| **Relume Tailwind preset** (`@relume_io/relume-tailwind@1.3.0`) | Their token preset | npm, public | Nothing yet |
| **Park UI** (park-ui.com) | Components on Ark UI + Panda CSS. React, Solid, Vue | public | Nothing yet |
| **Reshaped** (reshaped.so) | React + Figma libraries claimed as a "100% match"; semantic tokens, Figma variables mode, theme generated from one colour, Tailwind import, WAI-ARIA, RSC | npm `reshaped`, licence not stated on the page | Nothing yet. THE strongest off-the-shelf answer to the twin problem - see `DESIGN-TWIN.md` |
| **Relume Icons** (icons.relume.ai) | Icon set, presented for copy-paste into Webflow | public page, but it states no count, no grid, no formats and no licence | Nothing. Not adoptable on what the page discloses: an icon set with no stated grid or licence cannot be put in a design system |
| **Opbox design** (own repo) | The product system. 6 themes, 62 Figma colour tokens, 56-entry component map | local | Documented as a system page. `statusMeaning` (aligned/floor/migrating) is the idea worth stealing |
| **Opbox marketing** (own repo) | "Will Style", authored in oklch | local | Documented as a system page |
| **Jellytot** (own repos) | Brand system with three disagreeing sources of truth | local | Documented as a system page; style pack |

### The three blockers, each with its measured error

**Uber Base import.** `importComponentSetByKeyAsync` returns
`Component set with key "..." not found` for a genuine harvested key AND for an impossible
all-zeros key, byte-identical. So the failure is the missing library subscription, not the
keys. FIX: duplicate the Base **library** file, not the Gallery - every component in the
Gallery is `remote: true`, so duplicating it yields 3,504 instances and no components - then
enable it on this file and all 92 keys resolve at once.

The first version of that test used keys I had reconstructed rather than copied, and so
established only that invented keys do not resolve, which nobody doubted. Recorded because a
control that does not control produces a confident answer to a question it never asked.

**Material You kit.** Every tool refuses:
`Looks like you don't have edit access to this file.` - `use_figma`, `get_metadata` and
`get_variable_defs` alike, so it cannot even be READ. FIX: duplicate it to your own drafts.

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
