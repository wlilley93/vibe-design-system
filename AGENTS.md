# VDS V1 Agent Contract

This repo defines VDS. VDS itself is governed by VJS, and so is this repo: judgement calls
about VDS go to the VJS bench, not to a bench here, because there is no bench here.

**VDS is not commenced** (VDS S-15). Until a dated, digest-pinned assent event exists in
`designpack/v1/provenance/assent/` naming the digest of `VDS.md`, no warrant may be granted,
because there is nothing to grant one under. An agent that grants, records or implies a warrant
before commencement has authored a signature on nothing.

## The two rules you will break first

1. **Do not put a design value in `.vds/`.** A requirement is lawful (`minRatio: 3.0` is drawn
   from WCAG 2.2 SC 1.4.11). A realisation is not (`"#ebebeb"`, `12px`, `Geist Mono`, `160ms`,
   `cubic-bezier(...)`), wherever it appears and whatever it is called. VDS S-2(4).
2. **Do not decide anything.** Referring a fork feels slower than settling it. Settling it
   makes VDS the fourth authority that [2026] VJS-CC-OPBOX 3 forbids. VDS S-1(2).

## Before you write an artefact

Apply the four-limb test at VDS S-2(5). All four must hold, and a proposed artefact failing any
one of them is in the storing form and is forbidden.

| limb | the question |
|---|---|
| deletion | delete it. is any shipped or decided value lost? then it stored. |
| divergence | make the two named records disagree. does it fail closed, or keep serving its own value? |
| authorship | can a reader change a shipped pixel by editing only this artefact? then it is an authority. |
| regeneration | is a pin or ledger byte-reproducible by a named command from the named records? |

## Quick reference

**None of these commands exist yet.** They are the intended front door, recorded so that the
front door and the wall are designed together. Until they exist, do the equivalent by hand and
say so in the log. A command written in a doc is not a command that runs.

```bash
# Before governed design work
vds route --stage <W1|W2|W3|W4> --intent "<description>"

# Run a proof (captures its own result record; never hand-write one)
vds proof run <kind>

# Refer a judgement call to VJS
vds submit --trigger <first-impression|distinction|overrule|conflict|breach> --question "<q>"

# After a reversible call
vds log decision --decision "<decision>" --basis <authority> --why "<reason>"

# Before commit
vds validate --staged
```

## Lifecycle

```
route -> permit -> obligations -> proof -> log -> validate
```

Adopted from VJS unchanged. The stage lifecycle sits on top of it and is strict:

```
W1 REGISTER-COMPLETE -> W2 DESIGN-COMPLETE -> W3 PRINCIPAL-ACCEPTED -> W4 PARITY
```

A stage may not be entered before the preceding warrant is `granted`. The ordering is the whole
mechanism: every drift defect measured in the motivating project was authored before anyone
asked whether the thing being used was registered.

## Rules

- No design value in `.vds/`. Requirements only.
- No self-granted warrant. W1, W2 and W4 are referred to VJS; W3 is the Principal's alone.
- No hand-written proof record. `capture_mode` is fixed to `automatic` by schema.
- No proof kind outside the closed registry at VDS S-7(5). Adding one is an amendment to the
  specification and the invariant registry, not a script anyone may drop in.
- No identifier asserted by hand. Read the live record off disk, take the maximum plus one. A
  collision is a fail-closed validation error, never a silent overwrite.
- No lowering a floor. Tighten freely. To go below an inherited floor, change the component's
  **scope** with the basis stated, which is a factual claim a reviewer can contest.
- No claim that the enforcement surface is tamper-proof. It is not, and VDS S-8(5) says so.
- No bench, citator or appeal route in this repo.

## Numbers

If you assert a number, produce it with a command and name the command. An unmeasured number is
an opinion. This applies to `demand`, to `rows_considered`, to `rows_enforced`, and to every
figure in a doc under `docs/`. A `demand` figure older than its ledger's generation is stale,
and the reconciliation proof says so.

## Permits and logs

Governed writes need a permit, scoped by path glob and closed by proof. `permit_required` must
name the proof scripts themselves and `.vds/config.toml`, or the gate is editable by the same
hand it constrains. `permit_exempt` covers the append-only record directories.

Every self-issued permit carries the standing note adopted from VJS: self-issue proves the
actor took the front door, is not an external authority's approval, and cannot satisfy a check
reserved to the Sovereign or to a constituted bench.

A reversible call with low blast radius is a decision log carrying `court_required: false` and
`why`, not a referral. That is what keeps referral cheap enough to actually use.

## Breaches

Self-file. A breach report carries `what_happened`, `law_breached[]` each citing an instrument,
`discovered_by`, `containment` and `remedy[]`. Remedy is restorative: the work is made good and
the lawful route resumes. There is no punishment.

The two founding defects are filed as breaches, not described as background, because a system
whose first act is to excuse the failures that motivated it has taught itself the wrong lesson
(VDS S-12(4)).

## Authority hierarchy

1. Real-world law
2. The VJS constitution and primary Acts
3. VJS orders binding on this jurisdiction, including [2026] VJS-CC-OPBOX 3
4. `VDS.md`, once commenced under S-15
5. The pinned designpack: statutes, regulations, invariants, obligations, orders
6. Warrants granted over the declared surface, while unspent
7. Local decision logs

The named systems of record (`app/globals.css`, the decided-target Figma file) are **not** in
this hierarchy. They are facts, not authority. VDS reads them and never overrules them.

`docs/` is not in this hierarchy either. Nothing there binds, and no warrant, order or
invariant may cite it.

## Non-goals

VDS does NOT:
- decide a contested design question
- adjudicate taste
- hold a bench, a citator or an appeal route
- store a colour, a length, a font, a duration or an easing curve
- call a model, or make a network call, inside a proof
- infer the Principal's acceptance from silence
- claim "no unregistered component anywhere", which is not provable; every claim is bounded by
  a declared surface named by digest in the warrant that relies on it

VDS IS a deterministic artefact store and proof producer.
