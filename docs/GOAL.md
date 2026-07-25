# VDS Goal Statement

This file is engineering explanation. It binds nothing (VDS S-3(4)). It exists to say what
done looks like in conditions a command can settle, and to state the current position
without flattering it.

## North star

A design defect of the class that motivated VDS becomes a **failed proof at authoring
time** rather than a hand audit months later that finds a live production accessibility
failure across five themes. VDS holds the artefacts and produces the proofs. It decides
nothing; VJS decides.

Two things follow from that and are worth stating before the criteria:

- The register is the expensive part, and it costs the same whether or not VDS exists. VDS
  does not create that cost; it refuses to let it stay unpaid and invisible.
- Everything below is bounded to a **declared surface**. No criterion asserts a universal
  negative, because a finite check can prove the modelled paths and never the absence of an
  unmodelled one.

## Done criteria (measurable)

Each criterion names how it is settled. A criterion with no settling command is not a
criterion, it is a hope.

### D1 The register reconciles, in both directions

- `reconciliation` proof exits 0 with `rows_enforced` > 0.
- Zero register entries with no resolvable code counterpart.
- Zero components in the governed library directories with no register entry.
- Zero register entries whose Figma node id does not resolve in the pinned file.
- Zero prop or state contracts disagreeing between record and code.

Settled by: the `reconciliation` proof result, `status: passed`.

### D2 Every proof kind in the closed registry is valid on all five limbs of VDS S-7(2)

For each of the ten kinds: a named command, a named test that seeds a violation and asserts
the non-zero exit, an `invoked_by` entry in the enforcement lock, `rows_enforced` > 0 on the
last run, and `capture_mode: automatic`.

Settled by: count of kinds satisfying all five limbs. Target 10 of 10.

### D3 No vacuous passes

Zero proofs in `.vds/proofs/` whose most recent result carries `status: vacuous`. A pass
over zero enforceable rows is recorded as vacuous, so this is a count, not a judgement.

Settled by: a scan over `.vds/proofs/`.

### D4 Every gate is invoked by CI, not only by a hook

For each entry in `.vds/enforcement.lock`, at least one `invoked_by` entry with
`surface: ci_workflow` and `blocking: true`. A local hook alone is an interim state and is
recorded as such, because `git commit --no-verify` bypasses it.

Settled by: a scan over the lock, cross-checked against the workflow files it names.

### D5 Zero enforcement-surface drift

Every pinned path's sha256 matches the lock. The positive direction is itself tested: a
test edits a pinned file and asserts a fatal finding.

Settled by: the drift check, plus the existence of that test by name.

### D6 The warrant chain is complete for the declared surface

W1, W2, W3 and W4 each exist with `status: granted`, each carrying at least one evidence
entry whose `proof_id` and `digest` resolve to a `passed` proof record, and each carrying a
`case_file_digest` that matches its convening record. W3 carries an `acceptance_event` with
a `surface_digest`.

Settled by: resolving every id and digest in the four warrants.

### D7 `.vds/` holds no design value

The `no_stored_values` proof finds zero colour literals, zero length literals, zero font
families, zero durations and zero easing curves anywhere under `.vds/**`.

Settled by: the `no_stored_values` proof result.

### D8 Every ledger is current with its source

Each generated ledger has a staleness test, and every staleness test passes.

Settled by: the `ledger_staleness` proof, plus a count of ledgers with no staleness test.
Target: zero ledgers without one.

### D9 Proof records keep pace with decisions

`count(.vds/proofs/) >= count(.vds/warrants/ where status = granted)`, and the ratio of
proof records to decision logs is reported on every audit.

Why this is a criterion at all: the proof surface is the one that rots. Measured in VJS at
drafting time, 173 decision logs against 3 proof records. VDS's entire value sits on the
surface that decayed there, so the ratio is watched rather than assumed.

Settled by: two directory counts.

### D10 Every RESERVED clause resolves to an open or answered submission

For each of the five reserved matters at VDS S-13: a submission file exists, its
`reserved_clause` names the clause, its `fail_closed_interim` is non-empty, and its status
is `filed` or `answered`. Zero clauses depending on an unsettled point with no submission
behind it.

Settled by: a cross-check between VDS S-13 and `.vds/submissions/`.

## Current state (measured 2026-07-25)

**Nothing is built.** The specification is drafted and not commenced (VDS S-15). No
designpack exists, no `.vds/` exists in any project, no warrant has been granted, and no
VDS proof has ever run. Ten of ten done criteria are therefore unmet. The honest summary is
that this repository currently contains a specification and six schemas, and that is all.

The measurements below are of the Opbox frontend as it stands, and they are what the
criteria will be run against first.

| measurement | value | how measured |
|---|---|---|
| `component-map.json` component entries | 56 | `json.load(...)['components']` length |
| `.tsx` files in `src/components/ui` | 55 | `ls src/components/ui/*.tsx \| wc -l` |
| `.tsx` files in `src/components/onyx` | 35 | `ls src/components/onyx/*.tsx \| wc -l` |
| `SCREEN_MANIFEST.csv` data rows | 1206 | `csv.reader` row count minus header |
| `CONTROL_BOUNDARY_LEDGER.md` table rows | 5118 | `grep -c "^| "` |
| design gates invoked from `.githooks/pre-push` | 2, at lines 106 and 123 | `grep -n` over `.githooks/pre-push` |
| design gates invoked from CI workflows | 0 of 10 workflows | `grep -rln` over `.github/` |
| proof kinds with a failing-direction test | 0 of 10 | none exists yet |

Three things that table says plainly:

1. **56 entries against 90 files is not a contradiction, and that is the problem.** One
   entry may legitimately cover several files, and not every file is a component. But no
   command derives either number from the other, so nobody can say which of the two is
   wrong. D1 exists to make that question answerable rather than arguable.
2. **The two existing gates satisfy the hook limb of VDS S-7(2)(3) and not the CI limb.**
   They block a push and do not block a merge. That is a real improvement over the state
   [2026] VJS-CC-OPBOX 3 found, where nothing invoked them at all, and it is not yet D4.
3. **The founding defect is measured, not remembered.** The control boundary was 1.20:1
   against both planes in light against a 3.0:1 requirement, worst at 1.15:1 in ember
   (`internal-docs/design/CONTRAST_AUDIT.md`, lines 293, 294, 56, 377, 378).

## What VDS will still not be able to prove

Stated here rather than discovered later, because a system that overclaims its own
guarantees has taught its users to ignore its findings.

- **The enforcement lock cannot bind an author with write access who edits a gate and
  re-locks in the same act.** The backstops for that residue are non-machine: the
  Principal's gate and the duty of reasonable care. The lock makes the act visible in a
  diff. It does not prevent it.
- **"No unregistered component anywhere" is not provable.** The `composition` proof covers
  the declared screen set. A screen outside the declared set is outside the proof, and a
  new one added tomorrow is outside it until the surface digest changes and the warrant is
  re-granted. Every claim VDS makes is bounded by its declared surface, and every warrant
  names that surface by digest for exactly this reason.
- **VDS cannot tell you the design is good.** W3 exists because that judgement is the
  Principal's, and no accumulation of passing proofs substitutes for it.
- **A proof proves its own claim, not the claim you hoped it made.** The failing-direction
  test at VDS S-7(2)(2) proves that the check can fail. It does not prove that the seeded
  violation is the violation that matters. That is a review question, which is why the lock
  entry records what the test seeds, in one line, where a reviewer will see it.

## Boundaries

- VDS holds artefacts and runs checks. VJS holds law, benches and precedent. A change that
  would give VDS a bench, an appeal route or a citator of its own is out of scope by
  VDS S-1(2), and belongs in VJS if it belongs anywhere.
- The named systems of record ([2026] VJS-CC-OPBOX 3 D1) are not VDS's to move. A proposal
  to relocate the source of truth for a shipped value is a referral, not an implementation.
- Whether a designpack binds one project or the realm is reserved (`SUBMISSION-VDS-003`).
  Until it is answered, everything here is scoped to the single project whose
  `.vds/config.toml` pins the pack, and no plan should assume otherwise.
