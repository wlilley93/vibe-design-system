# VDS: the Vibe Design System

*A design-artefact store and a proof producer. It decides nothing.*

VDS governs the round trip between a design system's register, the Figma file that
decides what it looks like, and the code that ships it. It is a Rust workspace and a
single binary, `vds`, and it is a sibling of [VJS](https://github.com/wlilley93/vibe-justice-system),
which it refers every judgement call to.

**Status: the specification is drafted and not commenced** (VDS S-15). No warrant has been
granted, because until a dated, digest-pinned assent event exists there is nothing to grant
one under. The tooling is built, tested and self-hosted; the law it enforces is not yet in
force.

---

## The argument

Two defects motivate VDS, and they are the same defect twice: a rule stated in prose,
enforced by discipline, and discipline fails silently.

**One. A live production accessibility failure, found by hand, months late.**
A control boundary was declared aligned between the decided target and what ships.
Measured, it sat at 1.20:1 against both planes in light against a 3.0:1 requirement, so the
shipped checkbox and text input failed WCAG 2.2 SC 1.4.11 across five themes, worst at
1.15:1. Nothing checked the declaration against the measurement, because the declaration
was prose.

**Two. The showpiece screen family was drawn in the outgoing idiom.**
The doctrine said workbench panels run edge to edge on a hairline rather than as card plus
table. The doctrine existed only in prose, **nothing read that prose**, and the declared
showpiece family was drawn entirely in the idiom the doctrine had retired.

Neither is a mistake of skill. Both are a mistake of enforcement: there was no command whose
exit code disagreed with the author.

**What VDS changes.** A defect of that class becomes a failed proof at authoring time
instead of a hand audit months later. That is the entire return. VDS does not make the
design good, it does not reduce the work of designing, and it does not remove the need for
the Principal to look at the screens (VDS S-14(4)).

---

## The round trip

```
                vds brief                              vds impl <CMP-id>
  register  ─────────────────▶  an agent draws     ─────────────────────▶  an agent writes
      ▲                          into Figma                                 the code
      │                              │                                          │
      │                              ▼                                          ▼
      │                       vds figma pull                             vds proof --all
      └───────────────────────────────┘
              measured, not claimed
```

Both directions are projections of the **same register**, which is what keeps them from
drifting at the join.

- **`vds brief`** is the prompt-to-Figma half. It emits the contract a generating agent must
  draw inside: the components that may be used, the states each must still draw, the props,
  roles, keyboard contracts and contrast floors, and the components that may **not** be used
  with the reason and the successor. It is the second founding defect answered directly:
  the doctrine that nothing read is now a document a generator reads.
- **`vds figma pull`** reads the decided-target file into a ledger. It is a ledger generator
  and never a proof, because VDS S-7(2)(1) forbids a network call inside a proof. It measures
  which of the nine states each component actually draws, so `states.drawn` stops being a
  hand-maintained claim that rots (VDS S-5(5)).
- **`vds impl <CMP-id>`** is the Figma-to-code half. It hands the implementing agent the
  criteria it will be judged against, before it writes, each with its basis and the proof
  kind that checks it. Requirements that **nothing** in this build checks say so.

None of the three carries a design value. The brief and the contract state requirements; the
ledger records names, node ids and variant values. What things look like stays in the Figma
file and in `app/globals.css`, which is where [2026] VJS-CC-OPBOX 3 D1 put them.

---

## What VDS is

Per project, in a committed `.vds/`:

| artefact | what it is |
|---|---|
| component register | one record per component: contract, states, a11y floors, lineage |
| warrants | four stage gates (W1 register, W2 design, W3 Principal, W4 parity), each granted on an evidence digest |
| proofs | machine output a warrant is granted against, captured automatically by the checker |
| pins | derived one-way agreement assertions between two named records |
| ledgers | generated inventories, never hand-edited, each with a staleness test |
| submissions | questions referred to VJS, and the orders that answered them |
| logs | decision logs and self-filed breach reports |
| locks | the designpack digest, the install state, and the enforcement surface |

**Ten proof kinds are a closed registry** (VDS S-7(5)), and closure is enforced by the type
system: `ProofKind` is an enum, so a kind outside the registry does not fail validation, it
fails to compile. Seven are implemented. The other three each say **why** they are not:

| kind | state |
|---|---|
| `register_completeness` | implemented |
| `reconciliation` | implemented, with limbs (c) and (d) of VDS S-5(6) disclosed as out of reach |
| `composition` | implemented |
| `states` | implemented |
| `retirement_drain` | implemented |
| `ledger_staleness` | implemented |
| `no_stored_values` | implemented, with the preimage limb disclosed as undischarged |
| `contrast` | needs the subject project's shipped CSS and theme set |
| `parity` | needs TypeScript analysis of the component source |
| `token_pin` | needs both named records, one of which is a network read |

A proof is valid only if it is re-runnable, falsifiable by a named test that seeds a
violation, invoked by something other than the author, non-vacuous, and captured
automatically (VDS S-7(2)). Anything failing one of those five limbs is not a proof and may
not be named as evidence.

## What VDS is not

**VDS decides nothing** (VDS S-1(2)). No bench, no citator, no appeal route, no power to
resolve a contested question. Every judgement call routes to VJS, which already has all
four. Building a second adjudicator would repeat the mistake that [2026] VJS-CC-OPBOX 3
forbids in the token layer: a second authority beside one that already works.

**`.vds/` stores no design values.** An artefact may hold a **requirement** (a contrast floor
drawn from WCAG, a required state, a prop contract). It may never hold a **realisation** (a
colour, a length, a radius, a duration, an easing curve). Three things enforce that rather
than one: no artefact type has a field one could live in; a test walks every generated schema
and fails on a property named like one; and the `no_stored_values` proof scans the bytes on
disk, because a claim enforced by discipline is the defect VDS exists to convert into a
failed proof.

**VDS does not adjudicate taste.** Whether a surface is good is reserved to the Principal at
W3, and no accumulation of passing proofs substitutes for it (VDS S-6(7)).

---

## Getting started

```bash
cargo build --release --bin vds

cd /path/to/your/project
vds init                    # scaffold .vds/
vds ledger screens          # generate the declared surface
vds proof --all             # run every implemented proof
vds doctor                  # measure against the ten done criteria
```

`vds doctor` is the honest position. It reports MET, UNMET or NOT CHECKED per criterion,
names the command that settled each, and counts the not-checked separately, because a report
listing only what it can check reads as a clean bill of health.

See [`docs/ADOPTING.md`](docs/ADOPTING.md) for the full order of operations, and
[`docs/FIGMA-ROUND-TRIP.md`](docs/FIGMA-ROUND-TRIP.md) for the design loop.

---

## Honest position

- **The register is the expensive part, and it costs the same whether or not VDS exists**,
  because it is just "write down every component and what it must do". Skipping VDS does not
  avoid that cost, it decides not to write it down, which is the state that produced both
  founding defects.
- **The reconciliation proof will fail on day one and keep failing** until the register is
  genuinely complete. That is a feature and will not feel like one.
- **The enforcement lock cannot bind an author with full write access** who edits a gate and
  re-pins it in the same act. The backstops for that residue are non-machine. VDS records
  this as a named remainder, and no VDS document may claim the enforcement surface is
  tamper-proof (VDS S-8(5)).
- **Six matters are RESERVED** and fail closed until VJS answers them, filed in
  `.vds/submissions/filed/`: provisional W1, who grants W2, what a designpack binds,
  forced-drain retirement, where the primitive floor sits, and whether a pin may carry a
  per-value digest.
- **VDS was found unsound once already, by an adversarial audit of its own first commit.**
  [`AUDIT-2026-07-25.md`](AUDIT-2026-07-25.md) is that record, kept deliberately. The
  history of a governance tool that shows only its clean state teaches nothing.

## Where to read next

| file | what it is |
|---|---|
| [`VDS.md`](VDS.md) | the normative specification, S-1 to S-15 |
| [`AGENTS.md`](AGENTS.md) | the agent contract for working in this repo |
| [`AUDIT-2026-07-25.md`](AUDIT-2026-07-25.md) | the audit that found the first commit unsound |
| [`docs/ADOPTING.md`](docs/ADOPTING.md) | how a project adopts VDS, and in what order |
| [`docs/FIGMA-ROUND-TRIP.md`](docs/FIGMA-ROUND-TRIP.md) | prompt to Figma, Figma to code |
| [`docs/RELATIONSHIP-TO-VJS.md`](docs/RELATIONSHIP-TO-VJS.md) | the division of labour, and the routing rule |
| [`docs/GOAL.md`](docs/GOAL.md) | ten measurable done criteria, and the current measured position |
| [`schema/`](schema/) | six JSON Schemas, GENERATED from the Rust types. Do not hand-edit: `vds schema check` diffs them. |
