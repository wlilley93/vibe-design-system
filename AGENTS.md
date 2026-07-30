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

## Run the tests

**One command:**

```bash
make test           # the Rust workspace
make test-factory   # the site-factory JS suite, through its floor-asserting gate
make check          # everything, in the order CI runs it
```

`tools/` and `make test-py` are GONE. The Python v0 toolchain was deleted in the Rust port
(847e0f8) and `.gitignore` calls it "the retired v0 toolchain", but the Makefile still
advertised `make test-py` and this file still documented `tools/run-tests.sh`, so the
documented way to run the tests invoked a script that does not exist. A command in the
onboarding docs that cannot run is worse than an undocumented one.

The reason the suite exists is unchanged and is VDS S-7(2)(2): a check is a proof only if
**a named test seeds a violation against a fixture and asserts the non-zero exit**. On
2026-07-25 `ls -A tools/tests` returned 0, so by VDS's own statute none of the implemented
proofs was a proof and none could lawfully be named as evidence. That standard now lives in
the Rust suite and in `site-factory/tests/`, where every check has a negative control that
was verified to actually fire.

Two properties the runner enforces around the whole run, not merely per test:

- **Fenced.** Every fixture is built under a temporary directory and torn down, so a test that
  crashes half way cannot leave the thing it was testing broken. (`VDS_TEST_PROTECT` belonged to
  the retired `tools/` runner and no longer does anything.)
- **Not vacuous.** The runner refuses to start if its own manifest digested nothing, because an
  empty manifest compares equal to an empty manifest forever.

A test whose docstring says **KNOWN RED** is failing on purpose: it asserts the behaviour VDS
ought to have and does not. Do not delete it and do not weaken it to green. Fix the tool, or
leave it red and cite it.

## Quick reference

These commands exist and are tested. Anything not listed here does not exist.

```bash
# Set up
vds init                              # scaffold .vds/
vds doctor                            # measure against the ten done criteria

# The register. Register BEFORE designing (VDS S-6(2)).
vds register add --name Button --import-path @/components/ui \
    --source-file src/components/ui/button.tsx --export-name Button \
    --require default,focus --floor 'control-border:surface:3.0:WCAG 2.2 SC 1.4.11'
vds register list | show <id> | measure-demand --all
vds register set-status <id> <status>          # one step, no skipping
vds register amend <id> --kind non_breaking --what "..."
vds register deprecate <id> --superseded-by <id> | --withdraw
vds register retire <id> --drain-proof PROOF-...

# The SCREEN register (VDS S-5A). A separate series from the component register,
# because a screen record holds no props, no states and no contrast floor.
# `--columns` is a COUNT of content panes and never a width: a width is a
# realisation and has no field to live in (VDS S-2(4)). A screen with no split
# requires 1, not 0; 0 is a requirement nothing can fail and the proof refuses it.
vds screen add --route /settings --columns 2 --regions rail,body \
    --file-key <key> --node-id 100:2
vds screen list

# The FRAME ledger, read by screen_parity. Derived out of band from a SAVED
# `GET /v1/files/:key/nodes` capture, because VDS S-7(2)(1) forbids a network
# call inside a proof. Capture it yourself, batched, with your own token: there
# is deliberately no API transport here.
vds figma frames --file-key <key> --from capture-1.json capture-2.json

# The declared surface and the proofs
vds ledger screens
vds proof --list                      # the closed registry, and why three are unbuilt
vds proof <kind> | --all [--invoked-by ci_workflow] [--allow-vacuous] [--no-capture]

# The design round trip
vds brief                             # what an agent may draw into Figma
vds figma pull [--from response.json] # measure what it actually drew
vds impl <id>                         # what that drawing must become in code

# Warrants. VDS grants nothing; `record` writes down a grant made elsewhere.
vds warrant status | record --stage W1 ... | spend <id>

# The enforcement surface
vds lock verify | add <path> --invoked-by ... --test-path ... --test-name ...
vds lock repin --rationale "..."
vds pack verify | pin
vds schema emit | check               # schemas are GENERATED from the Rust types
```

**Not implemented, and named so nobody looks for them:** the permit lifecycle
(VDS S-12(1)), `install.lock` (VDS S-11(4)), decision logs and breach reports as
commands, and a `submit` command for referrals. Submissions are hand-authored
into `.vds/submissions/filed/` and validated on read. Where the machinery does
not exist, do the equivalent by hand and say so in the log; a command written in
a doc is not a command that runs.

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
- No hand-written proof record. Fixing `capture_mode` to the single value `automatic` never
  made that true: it is a string an author types, so it asserted the property it was meant to
  prove. What makes it true is that `warrant record` re-runs the named check and requires the
  same digest, after checking the kind is implemented, the script is the canonical one for that
  kind, its digest still matches, and the record digests to its own stated digest.
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

## This repository

A Rust workspace, edition 2024, toolchain pinned to 1.95.0 in `rust-toolchain.toml`, matching
VJS. Two governance systems with one purpose should not have two toolchains.

| crate | what it holds |
|---|---|
| `vds-core` | artefact types, digests, identifiers, project discovery |
| `vds-designpack` | the vendored normative corpus and its lock |
| `vds-store` | reading and writing `.vds/`, and the enforcement lock |
| `vds-scan` | the screens ledger, the JSX scanner, the staleness test |
| `vds-figma` | the decided-target ledger, the brief and the implementation contract |
| `vds-proof` | the closed registry of proof kinds and the capture that records them |
| `vds-cli` | the `vds` binary |

`make check` runs what CI runs, in the same order. Where they differ, CI wins: VDS S-7(3)
holds that a hook is not CI, and the same is true of a Makefile.
