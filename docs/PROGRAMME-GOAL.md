# Goal statement: what VDS and site-factory are for, together

This file is engineering explanation. It binds nothing (VDS S-3(4)).

There are two `GOAL.md` files in this repository and neither says what the whole thing is
for. `docs/GOAL.md` states the kernel's goal and settles it with ten criteria.
`site-factory/docs/GOAL.md` states the factory's and settles it with seven. Until now
nothing in `docs/` or the root README even mentioned site-factory: a grep returned zero. Two
halves, each measured, with no statement of why they are in one repository.

## The goal

**Make a design decision something you can disagree with, and make an undecided one
impossible to ship quietly.**

That is one sentence and it has two clauses, which is why there are two halves.

## Why two halves

The founding defect is on the record as BREACH-0001. A control-boundary token was declared
aligned between the decided target and production. The declaration was **prose in a token
pin**; nothing derived the ratio from the two records it named. The shipped checkbox and
text input failed WCAG 2.2 SC 1.4.11 across five themes, worst at 1.15:1, and it was found
by a hand audit months later.

Two things had to be true for that to happen, and they are separate problems.

**Nothing was checking.** A rule existed, it was enforced by discipline, and discipline is
not a gate. That is the kernel's problem: **VDS** holds the artefacts and produces the
proofs, so a defect of that class becomes a failed proof at authoring time instead of an
audit finding. It decides nothing; VJS decides.

**And there was nothing to check against.** A proof needs a register, a declared surface, a
stylesheet that is the system of record. On a project where the decisions were never written
down, VDS runs and returns `vacuous` on almost everything, which is honest and useless. That
is the factory's problem: **site-factory** turns a brief into a surface where every value
traces to a token somebody chose, every unwritten line is counted by name, and the Figma
file and the code cannot drift apart without a test failing.

So: the factory produces a surface worth governing, and the kernel governs it. Neither is
sufficient. A kernel with nothing to look at reports passes over an empty set. A factory
with no kernel is a generator whose output nobody has to answer for.

## How they meet, and how deliberately little they know about each other

The seam is **opt-in in both directions**, and that is a design commitment rather than an
accident of sequencing.

- VDS works with no knowledge of site-factory. It is a general kernel; its worked example is
  a Next.js storefront, not a generated site.
- site-factory without `--vds` writes no `.vds/` at all.
- With `--vds`, `vds-bridge.js` does the one thing `vds init` cannot: point the surface at
  what this project actually ships. Without that repointing the `.vds/` is **present but
  blind** - measured, not assumed: three proofs precondition-fail and the rest return
  `rows_considered: 0`.

Measured on a generated project, five of the eleven proof kinds enforce. Three are
structurally vacuous and no setting fixes them: `vds-scan` parses ESM `import`, and a
scaffold resolves its blocks through a dynamic `require()` because dropping a file in
`blocks/` is meant to register it. There is no static import graph to walk.

**A parallel screens file, generated to feed the scanner, was considered and refused.** It
would have turned three zeros green. It would also have been a second artefact describing the
build, drifting from it, which is the failure mode rather than the fix. The zeros are
recorded with their reason instead. That refusal is the clearest single statement of what
this programme is for: a number that looks like enforcement and is not is worse than an
honest zero, because only one of the two can be trusted later.

## What success looks like

Success is not "the proofs pass". A vacuous proof passes. Success is:

1. **A defect of the founding class fails at authoring time**, on a real project, and the
   failure names the boundary, the theme and the ratio. `examples/storefront` enforces all
   eleven kinds over rows that exist, so this is demonstrated rather than argued - and it is
   demonstrated by breaking one custom property and reading `expected 3.00:1 or more in
   :root, actual 1.47:1`, exit 1. The row counts are deliberately not quoted here: read them
   from `vds proof --all --root examples/storefront`. A count in prose is a count that rots,
   and this one already had - it was 230 when this paragraph was drafted and 290 an hour
   later, because regenerating a ledger changed it.
2. **A generated site is legible enough to argue with.** Not good. Legible. Someone who
   dislikes the spacing can find the token, see the density decision behind it, and say why
   it is wrong.
3. **What is not known is counted, not filled.** The reference writing run refused five of
   twenty lines and dropped two whole blocks, because that subject has no users and no price.
   A run reporting twenty of twenty would have fabricated two testimonials and two price
   tiers, and the page would have read finished while carrying its two most load-bearing
   claims as fiction.
4. **Every claim is the same size as its check.** This is the one the programme keeps
   failing and re-closing, and it is worth naming as a success condition because it is not
   naturally stable.

## What failure looks like

Stated plainly, because these are the shapes this programme has actually produced and
corrected, not hypotheticals:

- **A gate that cannot fail.** D4 certified that sixteen gates reached CI by scanning the
  lock's own declaration and never opening a workflow; renaming the step so zero steps
  matched left it reporting Met. `lock verify` checked that a failing-direction test existed
  by stat-ing a path identical to one whose digest had just been read successfully. Both are
  on the record as BREACH-0004 and BREACH-0005.
- **A rule broader than its check.** "No control the renderer ignores" was enforced against
  the four fields the rule named, while eight of nineteen were dead. "No px literal outside
  the `--space` multiplier" offered a grep for **hex** as its evidence, and sixteen px
  declarations lived under it.
- **A count restated instead of derived.** Four hand-kept copies of one number disagreed
  simultaneously; the test guarding it asserted that *a* number was present.
- **A fixture that describes a shape production never has.** The failing-direction check
  passed for months against a two-path fixture the real lock never produces.

Every one of those was found by seeding a defect and checking the gate fired, and several
were found only because the seed was verified to have landed first - a seed that silently
misses reads exactly like a dead gate.

## What is not claimed

- **Not that a generated site is well designed.** Every criterion in both GOAL files is
  about legibility and internal consistency. None is about whether a page works on a reader.
- **Not that VDS governs this repository.** It ships no screens, so most kinds are honestly
  vacuous here and `.vds/config.toml` says so in its own header. The worked example is where
  self-hosting is demonstrated.
- **Not that the register is cheap.** It is the expensive part, and it costs the same whether
  or not VDS exists. VDS does not create that cost; it refuses to let it stay unpaid and
  invisible.
- **Not a universal negative anywhere.** Every criterion is bounded to a declared surface,
  because a finite check can prove the modelled paths and never the absence of an unmodelled
  one.

## Where the measured position lives

Not here, deliberately. A third place stating counts is a third place for them to rot, and
this programme has already had four hand-kept copies of one number disagree at once.

- Kernel criteria and position: `docs/GOAL.md` (D1-D10)
- Factory criteria and position: `site-factory/docs/GOAL.md` (S1-S7)
- The registry itself: `vds proof --list`, which derives from the enum rather than restating it
- Enforcement surface: `vds lock verify`
- Self-assessment: `vds doctor`, which exits 1 on this tree and names why
