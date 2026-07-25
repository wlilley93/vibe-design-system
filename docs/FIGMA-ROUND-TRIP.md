# The design round trip: prompt to Figma, Figma to code

This file is engineering explanation. It binds nothing (VDS S-3(4)).

The goal is narrow and worth stating exactly: **a generating agent should not be able to
draw something the register does not permit, and an implementing agent should not have to
guess what "correct" means.** Everything below is machinery for those two sentences.

## Why one register and not two documents

The obvious way to do this is a design brief written by hand and an implementation ticket
written by hand. That is two documents, maintained by two people, describing one contract,
and they drift. The drift shows up as an implementation that satisfies its ticket and
contradicts the design, and nobody can say which of the two was wrong because both were
written down.

So both directions are **projections of the register**, generated on demand:

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

There is one contract. A brief and a contract that disagree is not possible, because neither
is authored.

---

## Prompt to Figma: `vds brief`

```bash
vds brief                       # markdown, for pasting into a prompt
vds brief --format yaml         # structured, for a tool
vds brief --out brief.md
```

The brief answers, for a generating agent:

- **which components exist and may be composed with**, with their Figma node ids;
- **which states each must still draw** (`STILL TO DRAW: focus`), which is the actual work;
- the prop contract, role, accessible-name source, keyboard contract and contrast floors;
- **which components may NOT be used**, with the reason and the successor, so a deprecated
  component is not reached for by accident;
- **the rules**, in the imperative, each citing the clause it comes from;
- **what the brief does not settle**, so an agent does not read silence as permission.

It also says, on its face, whether W1 REGISTER-COMPLETE is granted over this surface. If it
is not, the brief says the work is being done ahead of its warrant and will have to be
re-justified. It tells you what the register says; it does not authorise the work.

### What a brief will not contain

No colour, length, radius, font, duration or easing curve. Three reasons, in order of
weight:

1. [2026] VJS-CC-OPBOX 3 D1 makes the Figma file the system of record for what is decided.
   A brief carrying values would be a second opinion about what is decided, which is the
   fourth authority that order forbids.
2. A brief that carried values would go stale the moment the design system moved, and an
   agent following a stale brief produces a design that is wrong in a way that looks
   deliberate.
3. It is not needed. The agent is looking at the Figma file. It has the values.

The division is: **the brief says what must exist, and the design system says what it looks
like.** A test asserts no realisation appears in a serialised brief, using a token-aware
detector rather than a substring scan, because the first version of that test fired on the
word "requirement" (it contains "rem"), and a check that cries wolf gets disabled.

---

## Measuring what was actually drawn: `vds figma pull`

```bash
export FIGMA_TOKEN=...
vds figma pull                         # reads the file every record names
vds figma pull --from response.json    # from a saved response: no token, no network
vds figma status
```

This writes `.vds/ledgers/figma.yaml`, recording per registered component:

- whether the node id the register claims **resolves** in the file. This is limb (c) of
  VDS S-5(6), and no offline check could ever answer it;
- the node's name and whether it is a component set;
- its variant properties and values;
- **which of the nine states it actually draws**, derived from those variants;
- component sets in the file that **no register record claims**: a component design has
  committed to and governance has never seen.

### Why this is a ledger and not a proof

VDS S-7(2)(1) requires a proof to be re-runnable and deterministic with no network call.
Reading Figma is a network call. A proof that called Figma would be neither, so the read is a
**ledger generator**, run out of band, and the proofs read what it wrote and refuse it when
it is stale. This is the same arrangement the screens ledger uses, and it is the only
arrangement under which a proof can reach Figma at all.

`--from` makes a pull reproducible: the same saved bytes derive the same ledger forever, with
no token, which also means an air-gapped build can derive one.

### What the pull buys

VDS S-5(5) says a hand-maintained register decays, and `states.drawn` was hand-maintained.
With a ledger present, `vds brief` stops asking the record what is drawn and asks the file.
The brief says which of the two it used, on its face, so a reader is never left guessing
whether a number was measured.

In the worked example in the commit history: the register claimed `default` and `hover` were
drawn, the file agreed, and `focus` (required, drawn by nothing) came out as the one thing
still to draw. That is the loop closing.

### Variant values are mapped conservatively

`Pressed` does not become `active`. Only the nine state names are recognised, plus spellings
that are the same word (`hovered`, `focus-visible`). Guessing a synonym would let the ledger
claim a state is drawn on the strength of a word VDS invented. Unmapped values are **counted
and reported**, so a design system that names its states differently sees a number rather
than a silence.

---

## Figma to code: `vds impl <CMP-id>`

```bash
vds impl CMP-0001               # a markdown checklist
vds impl CMP-0001 --format json
```

The contract answers, for an implementing agent:

- **where to read the design from**: the Figma file and node, and whether that node was
  measured to resolve. If it did not resolve, the contract says so, because implementing
  against a node that is not there is implementing against nothing;
- **where the code goes**, and whether the file exists yet;
- **every requirement**, as a checklist, each with its basis and **the proof kind that will
  check it**;
- **what no check will catch**;
- **the commands to run before calling it done**.

The last two are the part that matters most. A requirement marked `checked_by: parity` is
listed alongside a note saying `parity` is specified and not implemented in this build, and
why. An implementer who reads the contract knows exactly which of its requirements are
enforced and which rest on care. A contract that listed only the enforced requirements would
be shorter and would teach the reader that the others do not matter.

It also refuses to issue a contract for a retired component: VDS S-9(8) inverts the test
after retirement, so implementing one would make the code the defect.

---

## The order of operations

```bash
vds register add --name Button ...      # register BEFORE designing (VDS S-6(2))
vds register set-status CMP-0001 designed
vds register set-status CMP-0001 registered
vds brief                                # hand this to the generating agent
# ... the agent draws ...
vds register amend CMP-0001 --kind non_breaking --what "record the node" --figma KEY#12:34
vds figma pull                           # measure what was drawn
vds brief                                # now measured, not claimed
vds impl CMP-0001                        # hand this to the implementing agent
# ... the agent writes the code ...
vds ledger screens
vds proof --all
```

The ordering is not a style preference. VDS S-6(2) calls it "the entire mechanism": every
drift defect measured in the motivating project was authored before anyone asked whether the
thing being used was registered. `vds register add` will only mint a record at `proposed` or
`designed` for exactly this reason.

---

## What this loop still does not do

Stated here rather than discovered later.

- **It does not check that the code matches the Figma node.** That is the `parity` proof,
  which is specified and unimplemented: it needs TypeScript analysis of the component
  source, not a digest comparison.
- **It does not check contrast.** That is the `contrast` proof, which needs the subject
  project's shipped CSS and its theme set. It is the proof that would have caught the first
  founding defect, and it is not built.
- **It does not check that a token value agrees between Figma and the CSS.** That is
  `token_pin`, and the Figma side is a network read, so the pin has to be generated out of
  band and then checked.
- **It cannot tell you the design is good.** W3 exists because that judgement is the
  Principal's.

Three of the ten kinds unimplemented is the honest state, and `vds proof --list` prints the
reason for each rather than a blanket note. A warrant relying on a run must not be described
as covering them (VDS S-6(3)).
