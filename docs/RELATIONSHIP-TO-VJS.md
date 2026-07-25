# Relationship to VJS

This file is engineering explanation. It binds nothing (VDS S-3(4)). Where it disagrees with
`VDS.md`, `VDS.md` wins.

## The one-line division

**VDS holds artefacts and proves. VJS decides.**

VDS is a store and a checker. It can say a proof exited non-zero, that a register entry has no
code counterpart, that two records disagree on a digest, and that a warrant cites evidence
which does not resolve. It cannot say whether a rule is right, whether a precedent fits, or
whether a warrant is earned on contested facts. Those are judgements, and every one of them
routes to VJS.

## Why it is split this way rather than merged

The obvious cheaper design is to give VDS a small bench of its own for design questions, since
they are narrow and a round trip to VJS costs a sitting. That design is closed.

[2026] VJS-CC-OPBOX 3 held that "an artifact that STORES token values is an authority and here
would be the fourth; one that DERIVES and enforces over a named record is a gate and no
authority at all". The ratio is about locus of authority, not about tokens specifically. A
second bench is the same defect one layer up: a second place where a contested question can be
settled, differently, by a body with no appeal route into the first. VJS already has a bench, a
citator, three tiers, an appeal route and an enforcement surface. Duplicating any of them
creates the fork VDS exists to prevent.

So VDS S-1(2) is absolute: no bench, no citator, no appeal route, no power to resolve a
contested question.

## What each side owns

| | VDS | VJS |
|---|---|---|
| holds | component records, warrants, proofs, pins, ledgers | statutes, regulations, invariants, orders, judgments |
| produces | machine evidence with an exit code | rulings with a ratio |
| answers | "does this check pass, over how many enforced rows" | "is this right, and does precedent already settle it" |
| on a fork | files a submission | convenes, or disposes on citation |
| on a breach | self-files the report | rules on the remedy where contested |
| binds | the project whose `.vds/config.toml` pins the designpack | the jurisdiction that pins the lawpack |

## What VDS adopts from VJS unchanged

Adopting rather than reinventing is the point. VDS copies, with the same meaning:

- The **three-tree split**: normative pack at the repository root, runtime record in the
  dot-directory, non-binding explanation in `docs/`.
- The **accession model**: vendored read-only, digest-pinned, fail-closed, with a schema
  version handshake. A loader that skips clauses it cannot parse is silently lawless.
- The **enforcement lock**: a digest pin of the gate surface, held outside the gates it
  witnesses, so a weakening edit trips a loud blocking finding rather than passing under its
  own possibly weakened logic.
- The **order shape** for warrants: short and operative, with the long rationale in a separate
  judgment file, so the instrument stays loadable at runtime.
- **Identifier allocation from the live record on disk**, maximum plus one, fail closed on
  collision. VJS deleted an in-memory citation registry for exactly this defect: it restarted
  every series at genesis.
- **Permits, decision logs and breach reports**, including the standing self-issue note.
- The **lifecycle**: `route -> permit -> obligations -> proof -> log -> validate`.

## The interface

1. VDS reaches a judgement call it may not settle.
2. It writes a submission under `schema/submission.schema.json` into
   `.vds/submissions/draft/`, then files it to `.vds/submissions/filed/` carrying the VJS
   submission id.
3. Before filing, the citator is checked. The submission must list every near authority
   considered and say why each is not on all fours. The schema requires at least one citator
   entry, because a submission that skips the check re-litigates settled law.
4. VJS convenes, or disposes on citation. A convening record pins the bench and the case file
   by digest.
5. The order's citation and ratio are recorded back onto the submission, and any warrant
   granted repeats the same `case_file_digest`, so what was decided on is provable afterwards.
6. An appeal runs in VJS, by VJS's own tiers. VDS records the outcome and holds no appellate
   function of its own (VDS S-10(6)).

Citations retain their VJS form. A VDS submission answered by the County Court in the Opbox
jurisdiction produces `[2026] VJS-CC-OPBOX n`, not a VDS series. There is no VDS series,
because there is no VDS court.

---

# The routing rule

The expensive failure mode is an agent told "refer when unsure", which refers on every trivial
fork and costs a fortune. The cheap failure mode is an agent that settles a first-impression
point silently. The rule below is what separates them.

## Refer to the bench only on these five triggers

Taken from the VJS conditions, unchanged:

1. **First impression.** No existing ruling covers the question.
2. **Distinction.** A precedent exists but genuinely does not fit these facts.
3. **Overruling.** A ruling is wrong or outdated and should be set aside.
4. **Conflict.** An instruction clashes with the designpack, with `VDS.md`, or with a binding
   VJS order.
5. **Breach.** Work fell below the duty of care. Self-report, then fix.

**Check the citator first, always.** A binding ratio on all fours disposes of the matter
instantly, with no sitting: cite it and move on. That fast path is what keeps referral
affordable.

## Everything else is a decisive call plus a one-line note

A reversible call with low blast radius is a decision log carrying `court_required: false` and
`why`. Make the call, write the line, keep going. Do not convene, and never route the fork to
the Principal.

## The test, in one question

> **If this turns out wrong, what does it cost to undo?**

- **A file rename, a re-run, a follow-up commit.** Decisive call plus a one-line note.
- **A granted warrant is void, a shipped surface must be redrawn, a floor was relaxed, or a
  precedent now points two ways.** Refer.

Two secondary tells. If the answer changes what a *proof* is allowed to accept, refer, because
that changes the wall rather than the work behind it. If reasonable engineers on the same
evidence would reach different answers, refer, because that is what a contested question is.

## Worked examples

| question | route | why |
|---|---|---|
| Is `Badge` one register entry or three variants of one? | decisive call | reversible by an amendment; `contractVersion` records it |
| A component needs a tenth state, `pressed` | refer | VDS S-5(3) fixes nine; a tenth is an amendment to the specification |
| Which of two equivalent id formats for proof records | decisive call | a rename, entirely reversible |
| Should a bare `<button>` count as unregistered for the `composition` proof | refer | RESERVED, `SUBMISSION-VDS-005`; it changes the reach of the anti-drift proof |
| A floor should be 2.5:1 rather than 3.0:1 for this component | refer | lowering a floor is a breaking amendment; the honest alternative is a scope change with a stated basis |
| A component is decoration and carries no control boundary | decisive call, with the basis recorded | a factual claim a reviewer can contest, unlike a quietly lowered floor |
| Adding an eleventh proof kind | refer | VDS S-7(6); an open registry is a free-form script surface, and that is not a gate |
| Which directory a generated ledger is written to | decisive call | regenerable by command, so the cost of undoing is one re-run |
| The reconciliation proof fails and the register looks right, so relax the proof | refer, and expect to lose | relaxing a failing gate to make it pass is the defect, not the fix |
| Whether the Principal accepted the surface | neither | W3 is the Principal's alone. No proof, no bench and no inference from silence substitutes for it |

## The failure this rule exists to prevent

Do not route a design fork to the Principal. The Principal has exactly one reserved decision in
VDS, W3 acceptance under S-6(7), plus their standing offices as Sovereign and executive.
Everything else is either a citation, a sitting, or a decisive call with a line written down.
Asking the Principal to choose between two proof designs is not deference, it is a refusal to
use the process that exists.
