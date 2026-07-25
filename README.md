# VDS: the Vibe Design System

*A design-artefact store and a proof producer. It decides nothing.*

**Status: drafted, not commenced.** The normative text is [`VDS.md`](VDS.md). It has no assent
event behind it yet, so nothing in it binds until it is enacted by the process at VDS S-15.
This repository currently holds a specification, six JSON Schemas and this documentation, and
that is all. No designpack exists, no project carries a `.vds/`, and no proof has ever run.

---

## The argument

Two defects motivate VDS, and they are the same defect twice: a rule stated in prose, enforced
by discipline, and discipline fails silently.

**One. A live production accessibility failure, found by hand, months late.**
The control boundary was declared aligned between the decided target and what ships. Measured,
it sat at 1.20:1 against both planes in light against a 3.0:1 requirement, and the shipped
checkbox and text input therefore failed WCAG 2.2 SC 1.4.11 across five themes, worst at
1.15:1 in ember. The evidence is `internal-docs/design/CONTRAST_AUDIT.md` in the Opbox
frontend, lines 56, 293 and 294. Nothing checked the declaration against the measurement,
because the declaration was prose.

**Two. The showpiece screen family was drawn in the outgoing idiom.**
The doctrine said workbench panels run edge to edge on a hairline rather than as generic card
plus table screens. The doctrine existed only in prose, nothing read that prose, and the
declared showpiece family was drawn entirely in the idiom the doctrine had retired.

Neither defect is a mistake of skill. Both are a mistake of enforcement: there was no command
whose exit code disagreed with the author.

**What VDS changes.** A defect of that class becomes a failed proof at authoring time instead
of a hand audit months later. That is the entire return. VDS does not make the design good, it
does not reduce the work of designing, and it does not remove the need for the Principal to
look at the screens (VDS S-14(4)).

---

## What VDS is

VDS holds, per project, in a committed `.vds/` directory:

| artefact | what it is |
|---|---|
| component register | one record per component: its contract, states, a11y floors, lineage |
| warrants | four stage gates (W1 register, W2 design, W3 Principal, W4 parity), each granted on an evidence digest |
| proofs | machine output a warrant was granted against, captured automatically by the checker |
| pins | derived one-way agreement assertions between two named records, by digest |
| ledgers | generated inventories, never hand-edited, each with a staleness test |
| submissions | questions referred to VJS, and the orders that answered them |
| logs | decision logs and self-filed breach reports |
| locks | the designpack digest, the install state, and the enforcement surface |

Ten proof kinds are fixed as a closed registry (VDS S-7(5)). A proof is valid only if it is
re-runnable, falsifiable by a named test that seeds a violation, invoked by something other
than the author, non-vacuous, and captured automatically (VDS S-7(2)). Anything failing one of
those five limbs is not a proof and may not be named as evidence.

## What VDS is not

**VDS decides nothing** (VDS S-1(2)). It has no bench, no citator, no appeal route and no power
to resolve a contested question. Every judgement call routes to [VJS](https://github.com/wlilley93/vibe-justice-system),
which already has all four. Building a second adjudicator would repeat the mistake that
[2026] VJS-CC-OPBOX 3 forbids in the token layer: a second authority beside one that already
works. See [`docs/RELATIONSHIP-TO-VJS.md`](docs/RELATIONSHIP-TO-VJS.md).

**`.vds/` stores no design values.** [2026] VJS-CC-OPBOX 3 permits a design kernel in a
deriving and enforcing form and forbids the storing form: "an artifact that STORES token values
is an authority and here would be the fourth; one that DERIVES and enforces over a named record
is a gate and no authority at all." So an artefact may hold a **requirement** (a contrast floor
drawn from WCAG, a required state, a prop contract). It may never hold a **realisation** (a
colour, a length, a radius, a duration, an easing curve). The realisations live where
CC-OPBOX 3 D1 put them: `app/globals.css` for what ships, the decided-target Figma file for
what is decided. A machine check enforces this, and a colour literal anywhere under `.vds/**`
is a fatal finding (VDS S-2(8)).

**VDS does not adjudicate taste.** It checks contracts, floors, composition and parity. Whether
a surface is good is reserved to the Principal at W3, and no accumulation of passing proofs
substitutes for it (VDS S-6(7)).

---

## The three trees

Copied from VJS, where the split is declared rather than incidental.

```
designpack/v1/     normative. statutes, regulations, invariants, obligations,
                   orders, judgments, specs, provenance, manifest.toml.
                   versioned, digest-pinnable, vendorable on its own.

.vds/              this project's record. register, warrants, proofs, pins,
                   ledgers, submissions, logs, permits, locks.
                   committed, not scratch.

docs/              engineering explanation. binds nothing, and no warrant,
                   order or invariant may cite it as authority.
```

The normative tree sits outside the dot-directory deliberately. A project subscribes by
vendoring a designpack read-only and pinning its digest in `.vds/designpack.lock`, exactly as a
VJS subscriber pins a lawpack. That is what lets a second project carry the same doctrine
without copying this project's register.

---

## Honest position

- **The register is the expensive part, and it costs the same whether or not VDS exists**,
  because it is just "write down every component and what it must do". Skipping VDS does not
  avoid that cost, it only decides not to write it down, which is the state that produced both
  defects above.
- **The reconciliation proof will fail on day one and keep failing** until the register is
  genuinely complete. That is a feature and will not feel like one.
- **The enforcement lock cannot bind an author with full write access** who edits a gate and
  re-pins it in the same act. The backstops for that residue are non-machine. VDS records this
  as a named remainder, and no VDS document may claim the enforcement surface is tamper-proof
  (VDS S-8(5)).
- **Five matters are RESERVED** (VDS S-13) and fail closed until VJS answers them: provisional
  W1, who grants W2, whether a designpack binds one project or the realm, forced-drain
  retirement, and where the primitive floor sits.

## Where to read next

| file | what it is |
|---|---|
| [`VDS.md`](VDS.md) | the normative specification. S-1 to S-15. |
| [`AGENTS.md`](AGENTS.md) | the agent contract for working in this repo |
| [`docs/RELATIONSHIP-TO-VJS.md`](docs/RELATIONSHIP-TO-VJS.md) | the division of labour, and the routing rule |
| [`docs/ADOPTING.md`](docs/ADOPTING.md) | how a project adopts VDS, and in what order |
| [`docs/GOAL.md`](docs/GOAL.md) | ten measurable done criteria, and the current measured position |
| [`schema/`](schema/) | six JSON Schemas. A file that does not validate is not an artefact. |
