# The ceiling: which criteria cannot move from inside this repository

`vds doctor` reports 5 of 10 here and 6 of 10 on the worked example. This file
records WHY the remainder cannot be closed by work, so nobody spends another
session grinding at a criterion that is waiting on an act only somebody else can
perform.

**This is the most dangerous kind of document in the repository.** A written
account of what cannot be done becomes, very quickly, a written account of what
nobody tried. So every claim below is re-derived by a test
(`crates/vds-cli/tests/ceiling.rs`), and **that test FAILS when a blocker
clears**. A ceiling that quietly stays true after the ceiling lifts is an excuse.

Measured 2026-07-31.

## D1, D2, D3 in THIS repository: structural, and already documented as such

`.vds/config.toml` says it in its own words: *"VDS ships no screens. The glob is
kept rather than emptied because an empty `screen_globs` is refused at load ...
It matches nothing here, and every proof bounded by it comes out vacuous, which
is the correct and honest result."*

`vds ledger screens` returns 0 screens, 0 component references. Every proof
bounded by the declared surface is therefore vacuous, D3 correctly refuses to
call a vacuous pass a pass, and D1 has nothing to reconcile.

**Not a gap.** The kernel is a Rust CLI with no design surface of its own. This
is why `examples/storefront` exists, and there D1 and D3 are MET.

## D4 in BOTH: GitHub Actions billing

```
run 30613518500, enforce, failed in 9s
  The job was not started because recent account payments have failed or your
  spending limit needs to be increased.
```

53 runs concluded, **zero successes**, oldest 2026-07-25, newest 2026-07-31.
This is BREACH-0011's finding and it has not changed: seventeen pinned gates
declared CI-invoked over a job that has never once started.

**Blocked on: a billing setting in the GitHub account.** Nothing in this
repository can reach it. The interim is recorded in the Makefile and in the
lock: a committed pre-push hook runs `make check`, and `vds lock verify` prints
`INTERIM` for the hook precisely because *a hook is not CI* -
`git push --no-verify` bypasses it.

Note what IS now done: every `ci_workflow` reference resolves against the
workflow file it names (21 of 21, 0 unjudged). The declaration is sound. The job
still does not run.

## D6 in BOTH: warrants are not VDS's to grant

`vds warrant status` opens with the answer:

> VDS grants nothing. Granting W1, W2 and W4 is VJS's on a referred submission,
> and W3 is the Principal's alone (VDS S-1(3), S-6(2), S-6(7)). This is a report.

W1 through W4 are all `NOT GRANTED, no warrant record exists`. `vds warrant
record` exists to *write down a grant that already happened elsewhere*, and its
own help says **"This does NOT grant"**.

**Blocked on: a court grant (W1, W2, W4) and a Principal grant (W3).**
Manufacturing a warrant record for a grant that never happened would be the
exact ultra vires act S-2 forbids, and it would put a false record in the
governance chain.

## D10 on the worked example: an assent event

> this project holds no VDS.md and vendors no designpack, so it cannot see the
> specification's reserved clauses at all, answered or open. VDS S-15(1): the
> specification commences on a dated, digest-pinned assent event.

`vds pack verify` in this repository: *"This project pins the ABSENCE of a
designpack ... Until then no warrant may be granted, because there is nothing to
grant one under."*

**Blocked on: an assent event, which is a Principal act.** Note the dependency
runs downhill - no pack means no warrant, so D10 gates D6 as well. Fabricating a
dated assent would be forging the founding record of the specification.

D10 is MET in this repository because the kernel holds `VDS.md` itself and can
therefore see its own reserved clauses.

## What that leaves

| criterion | this repo | storefront | blocked on |
|---|---|---|---|
| D1 reconciles | vacuous by design | **MET** | nothing |
| D2 five limbs | vacuous by design | needs a lock | see below |
| D3 no vacuous passes | **MET** | **MET** | nothing |
| D4 CI invokes every gate | UNMET | UNMET | GitHub billing |
| D5 zero drift | **MET** | **MET** | nothing |
| D6 warrant chain | UNMET | UNMET | court + Principal |
| D7 no design value | **MET** | **MET** | nothing |
| D8 ledgers current | **MET** | **MET** | nothing |
| D9 proofs keep pace | **MET** | **MET** | nothing |
| D10 reserved clauses | **MET** | UNMET | an assent event |

**The one row that is neither structural nor externally blocked is D2 on the
worked example**, and it is an open question rather than a task: a subscriber
runs a RELEASED `vds` binary and does not hold the proof scripts' source, so it
has nothing of its own to pin, and D2's limb asks for "a lock entry whose named
failing-direction test resolves". Whether a subscriber pins the vendored
designpack instead, or whether D2 is a criterion only the kernel can satisfy, is
not settled by anything written down. It is not on the citator and it is not in
VDS.md.

That is the next thing worth a submission, and it is deliberately NOT resolved
here by a decisive call: it decides what conformance means for every future
subscriber, which is exactly the size of question S-11 sends to the court.
