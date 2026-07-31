# The design twin, and the station between the tracks

A component in Figma and a component in code are **twins, not stages**. Nothing compiles a
Figma frame into HTML. The design does not flow into the build; it stops, and something has to
cross a station to get onto the engineering tracks.

That is worth stating plainly because the whole industry's marketing implies otherwise, and
every tool that appears to solve it turns out to have staffed both platforms rather than
removed the station.

## What actually crosses

Not the component. A contract:

- **Token names and their values.** `--color-danger` is `#de1135` on both sides or the two
  sides are drawing different things.
- **The component's name and its variant axes.** `banner` with a `tone`; `Base/Banner` with a
  `Tone` property.
- **The section vocabulary and the order it appears in.** Which is a manifest.

Everything else is built twice, once per side, from that contract. A Figma component set is not
a source that becomes code. It is the second implementation of the same agreement.

## What the vendors are actually selling

Reshaped advertises "all components available as both React and Figma libraries with completely
aligned setup" and, in a case study, "a 100% match with its React counterpart." Read that
carefully: it is not a claim that one generates the other. It is a claim that **one vendor owns
both twins** and keeps them aligned by process. Park UI, Relume UI and Material You are the
same shape with less of the claim - they are component RUNTIMES, answering what a button is made
of, not whether the button in the file and the button in the repo agree.

So buying a paired system buys you someone else's discipline about their own components. It says
nothing about the moment you add a token or edit a component, which is the moment the drift
starts.

Figma's own answer is **Code Connect**, which binds a node to a source file so Dev Mode shows
the real component. That is the honest version of the crossing, and it is a LOOKUP rather than a
compilation: it tells you which code corresponds to this frame; it does not make one from the
other. Measured on this plan: `You need a Dev or Full seat on an Organization or Enterprise plan
to use Code Connect.`

## The three ways to stand at the station

**1. Own both twins and verify the pairing.** Where this programme is. Zero dependencies. The
cost is maintaining two of everything, and the protection is that a divergence FAILS rather
than accumulating:

- `figma-variables.json` pairs every emitted `--color-*` and `--radius-*` to a Figma variable,
  per mode, by value. It found four missing ink variables and nine drifted values; everything
  drawn before that fix was the wrong red, on both sides, invisibly.
- `figma-nodes.json` pairs every block type to a component set that exists, in both directions,
  with unpaired sets declared and a reason required.
- `figma-prompts.json` fingerprints every prompt written into the file against its source.

**2. Buy a vendor who owns both twins.** Reshaped is the strongest available. You trade a
dependency and their component opinions for not maintaining the pairing yourself. It is a real
option and it is not the same guarantee: it protects their alignment, not yours.

**3. Generate one twin from the other.** The only option that actually collapses the station -
and it works in exactly one direction.

## The direction that works, and the one that does not

**Code to Figma: works, and is already how this file is built.** Every one of the 46 component
sets in the master file was DRAWN BY SCRIPT, from the same token values `build.js` emits, bound
to the same variable collection. The Figma file is not a design source that feeds the code. It
is a RENDERING OF THE CODE. That is why the counts on its pages are derived rather than typed,
why the four style-pack columns can be asserted structurally identical, and why the sitemap and
wireframe cannot disagree with the build: all of it is projected from `manifests/*.json` and
`tokens/*.json`.

Put another way: the station is still there, but the traffic is one-way and automated, so nobody
carries anything across by hand.

### CORRECTION (BREACH-0010): "drawn by script" is true and "automated" is not

Both sentences above overstate, and the overstatement is the load-bearing kind, because it
describes the thing this whole file is about.

Each set WAS drawn by a script. But every one of those scripts was written inside an agent turn,
run once, and never committed. A grep for `createComponentSet`, `combineAsVariants` or
`createComponent(` across the whole repository returns nothing. So:

- the Figma file **cannot be redrawn**. Not from the register, not from the tokens, not from
  anything. Delete a set and the only recovery is to write a new script by hand;
- "a RENDERING OF THE CODE" describes a relationship nothing maintains. `figma-variables.json`,
  `figma-nodes.json` and `figma-prompts.json` check that the two sides AGREE, which is real and is
  what caught four missing inks and nine drifted values. None of them can PRODUCE either side;
- "nobody carries anything across by hand" is exactly backwards. Every crossing so far was
  hand-carried; what was automated was the CHECKING of what got carried.

This is the same class as the generated half nothing regenerates: a projection asserted in prose,
with the projector absent. It is worse here than in the usual case, because the paragraph above
was the evidence offered for the claim that the outbound direction is solved - so the claim was
resting on its own restatement.

The remedy is a committed generator whose input is the register and whose output is deterministic,
with an identity stamp on every set so a redraw amends in place instead of duplicating. Until that
exists, read every "outbound is automated" line in this file as "outbound is CHECKED".

**Figma to code: not by reading the frame** - and the correction below is why that is a
narrower claim than it first looks.

A frame carries geometry and fills. It does not carry the reason. `radius: 0` in the Balmoral
column is not a corner value to be read off and reimplemented, it is a client's binding decision
that no rectangle is rounded, and a generator reading the frame gets the 0 and loses the
decision. That much stands: this is BREACH-0001 pointing the other way, since the founding defect
was a value asserted with nothing deriving it.

## CORRECTION: VDS already is the station, in both directions

An earlier draft of this file framed the three measured manifests as "the bridge" and left the
impression that nothing crossed the station deliberately. That under-described VDS, and the
under-description matters because it would have led to rebuilding something that exists.

VDS models the crossing EXPLICITLY, with a command per direction:

- **`vds brief`** is the outbound platform: "the design brief: what an agent generating into
  Figma may draw." Derived from the register and stamped with its digest. Its own opening line
  refuses to carry realisations - "It contains no colours, lengths, fonts, durations or easing
  curves: those live in the design system itself, and this brief does not overrule them" - and it
  states outright when W1 is not granted, so it tells you what the register says without
  authorising the work.
- **`vds impl <ID>`** is the inbound platform: "what the drawing must become in code." Same
  refusal in the other direction: "It states requirements and no realisations: it says what must
  be true, never what it must look like." Every requirement names its BASIS and the PROOF that
  checks it - a prop contract at a contractVersion checked by `parity`, an export path checked by
  `reconciliation`.
- **`vds pin generate`** is the crossing record: compare the shipped stylesheet against the
  decided-target file's variables, and record the verdicts.
- **`vds figma pull` / `status`** reads the decided target into a ledger and reports what the
  ledger CANNOT say, in its own words: "Whether any node LOOKS right. It records names, node ids
  and variant values, and no colour, length, font, duration or easing curve."

And that dissolves the objection above rather than contradicting it. THE REASON IS NEVER READ OFF
THE FRAME. It is read off the REGISTER - prop contract, required states, contrast floor, role,
accessible name - and the Figma node records only WHERE the visual decision was taken. So the
Figma file stays the system of record for what was decided ([2026] VJS-CC-OPBOX 3 D1, which VDS
cites and does not overrule), while the thing an implementer is judged against is a contract with
a basis, not a rectangle with a corner radius.

So the crossing does automate in both directions. What does not automate is inferring intent from
geometry, which nobody should attempt in either direction.

## Three stations, at three maturities

| | What it is | Both directions | Verified by |
|---|---|---|---|
| **VDS** | The generic kernel. A register, a brief, an impl contract, a pin, eleven proof kinds | yes, `brief` and `impl` | proofs, an enforcement lock over the gate source, and a grader that is itself pinned |
| **Opbox** | A concrete instance, maintained by hand: `figma-variable-parity.json`, `boundary-verdicts.json`, a 56-entry component map with per-entry status | partly - the parity file and the map are the contract | `statusMeaning`: aligned / floor / migrating, where `floor` says a per-theme value is fine and the CONTRAST RATIO is the contract |
| **site-factory** | The generated case. The Figma side is DRAWN from the code, so the outbound crossing is automated and there is nothing to carry | outbound only, by construction | the three manifests, each stating what it is blind to |

Opbox's `statusMeaning` is the same idea as VDS's `basis` arriving independently, which is the
strongest evidence that the concept is right rather than invented: a token is under a contract,
and the contract says whether the VALUE or the RATIO is the thing that must hold.

## Prior art: the idea has a name, and someone published it this month

The problem is called **design system parity**, and the closest published relative to VDS
appeared in July 2026: Christine Vallaure's *"Design system contracts: the component lives in
neither Figma nor code"*, with a proof of concept at
[southleft/ds-contracts-poc](https://github.com/southleft/ds-contracts-poc) and a browser
playground.

Its central sentence is the design twin arrived at independently: *"Whichever you crown, the
other becomes a copy that someone has to keep updating."* And its resolution is the same -
*"Neither becomes the original; both are printouts of the same recipe."* Their contract is a
plain JSON/YAML file holding variants, colour tokens, allowed content and behaviour, with **no
visuals and no code** - which is close to word for word what `vds brief` and `vds impl` each say
about themselves.

Where it is AHEAD of VDS:

- **It generates both sides.** Code files are written out directly; the Figma side is a set of
  instructions a small plugin runs inside the file to build the components. VDS generates
  neither - it emits `impl` as a contract a human or agent implements, then proves the result.
  site-factory generates its Figma side by script, so it has half of this already, but the
  kernel does not offer it.
- **It has a three-way checker**, comparing contract against Figma against code.
- **It has an on-ramp.** Contracts were EXTRACTED from libraries that already exist, and run
  against Shoelace, Mantine, Carbon and Polaris. VDS has no importer for a foreign library; its
  own `vds register import` cannot even read a site-factory block, which is why vds-bridge
  writes the records instead.

Where VDS is ahead:

- **Governance rather than sync.** Warrants that gate design on a complete register, breach
  records, decision logs, an enforcement lock over the gate SOURCE, a criteria grader that is
  itself pinned, and a refusal to count a vacuous pass. The contracts PoC checks parity; VDS
  makes a failure answerable and records who answered.
- **It states its own blind spots unprompted** - `figma status` volunteers "whether any node
  LOOKS right" as something it cannot say.

Their stated limitation is VDS's too, and worth quoting because it bounds both: contracts
specify composition, *not* implementation quality - they do not address "drag, typeahead,
focus-trapping, motion, and good CSS."

## Does this make maximum use of Figma's dev functionality? No, and here are the four gaps

Measured, not guessed:

1. **`PropContract.figmaProperty` is null on every record.** The schema field exists and is
   described as "the corresponding Figma variant property, or null where the prop has no visual
   counterpart". That IS the prop-level binding Code Connect sells, available with no plan tier,
   and site-factory writes `null` for all 43 blocks.
2. **`vds figma pull` has never been run for a site-factory project.** So `brief` reports
   "States drawn measured from: the register's own claim (NOT measured)" and `impl` marks every
   node "(not measured)". The worked example DOES have a pulled ledger, so this is unused
   machinery rather than missing machinery - and it is the same defect class as BREACH-0006, a
   claim read off a field instead of off the thing.

   This is **unrun work, not a blocked dependency**, and the difference decides whether it sits on
   a list or gets done. `pull` needs `FIGMA_TOKEN` and a single agreed file key; a live 45-character
   `figd_` token is present in the environment, and every register record that carries a node
   already names the same file. Nothing external is missing.
3. **Variable Code Syntax is unused.** Figma can carry a per-platform code name on the variable
   itself, which would put `--color-danger` on `color/danger` inside the file rather than only in
   a manifest beside it.
4. **The a11y contract is a placeholder that has become a FALSE requirement.** Every record is
   written with `role: null` and `accessibleNameSource: none_decorative`, and the record's own
   note admits "role and keyboard are decisions nobody has made yet". But `vds impl CMP-0001` now
   prints, for a NAV: "take its accessible name from none_decorative". An honest placeholder in
   the register became a wrong instruction in the contract, because `impl` cannot tell a default
   from a decision.

5. **Nothing can redraw the library.** A grep for `createComponentSet`, `combineAsVariants` or
   `createComponent(` across the entire repository returns **zero hits**. All 46 component sets
   were drawn by scripts written inside agent turns and never committed. The only committed Figma
   generator is `figma-push.js`, which draws a project RECORD page - frames and text - and creates
   no component. See the correction below: this is BREACH-0010, and it is the largest of the five.

Gaps 4 and 5 are defects. Gaps 1 to 3 are work. None of the five needs a plan upgrade, and Code
Connect - which does - would sit on top of gap 1 rather than replace it.

## What follows for what to build

- Keep generating the Figma side from code, and keep extending the projections. That is the
  direction with no manual crossing.
- Do not plan work that depends on reading design decisions out of Figma frames.
- Reshaped is worth taking seriously for the RUNTIME - site-factory templates strings, and a
  real component runtime with WAI-ARIA primitives is a genuine upgrade for anything shipping an
  app rather than a marketing page. That is a separate decision from the twin question and
  should not be justified by it.
- Code Connect, if the plan ever allows it, is a nice-to-have on top of the manifests rather
  than a replacement for them: it makes the pairing browsable, and the manifests make it fail.
- Do not build a new crossing for site-factory. `vds brief` and `vds impl` are the crossing, and
  the work worth doing is pointing site-factory's register at them properly rather than inventing
  a parallel mechanism - which is exactly the "second description of the build" this programme
  keeps refusing everywhere else.
