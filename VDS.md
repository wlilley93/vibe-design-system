# VDS V1 Specification

**Status: drafted, not commenced.** This text is the normative specification of the Vibe
Design System. It has no assent event behind it yet, so nothing in it binds until it is
enacted by the process at S-15. Clauses marked **RESERVED** depend on a point that is not
settled and must not be implemented until the named submission is answered by VJS.

Citation form for this document: `VDS S-<section>(<clause>)`, for example `VDS S-2(4)`.

---

## S-1 Nature, purpose and limits

**S-1(1)** VDS is a design-artefact store and a proof producer. It holds registrations,
warrants, proofs, pins, ledgers and referrals for one project's design system, and it runs
deterministic checks over the project's named systems of record.

**S-1(2)** VDS **decides nothing**. It has no bench, no citator, no appeal route and no
power to resolve a contested question. Every judgement call that arises inside VDS is
referred to VJS under S-10. Building a second adjudicator would repeat the mistake that
[2026] VJS-CC-OPBOX 3 forbids in the token layer: a second authority beside one that
already works.

**S-1(3)** In particular VDS may not, by its own force: rule on a contested design
question; grant itself a warrant; declare a proof satisfied that exited non-zero; relax a
floor; adopt a new designpack digest; or accept a surface on the Principal's behalf.

**S-1(4)** VDS exists because a rule stated in prose and enforced by discipline fails
silently. Two measured defects motivate it, and both are of that one class:

- (a) A control-boundary token declared aligned between the decided target and production
  was measured at 1.20:1 against both planes in light, and the shipped checkbox and text
  input therefore failed WCAG 2.2 SC 1.4.11 across five themes, worst at 1.15:1 in ember.
  Source: `internal-docs/design/CONTRAST_AUDIT.md` lines 293, 294, 56, 377.
- (b) The declared showpiece screen family was drawn entirely in the outgoing card idiom
  while the doctrine requiring flush panels on a hairline existed only in prose, and
  nothing read that prose.

**S-1(5)** The saving VDS offers is not effort. It is that a defect of that class becomes
a failed proof at authoring time rather than a hand audit months later that finds a live
production accessibility failure.

**S-1(6)** Nothing in VDS adjudicates taste. VDS checks contracts, floors, composition and
parity. Whether a surface is good is reserved to the Principal under S-6(7).

---

## S-2 The storing line and the deriving line

**S-2(1)** This is the constraint everything else is built inside. [2026] VJS-CC-OPBOX 3
holds that "an artifact that STORES token values is an authority and here would be the
fourth; one that DERIVES and enforces over a named record is a gate and no authority at
all", and permits only the second form.

**S-2(2)** Accordingly: **`.vds/` stores no design values.** It stores registrations,
warrants, proofs, pins, ledgers, referrals and locks.

**S-2(3)** The named systems of record are fixed by [2026] VJS-CC-OPBOX 3 D1 and are not
VDS's to move:

| record | is the system of record for |
|---|---|
| `app/globals.css` | what ships |
| the decided-target Figma file | what is decided |
| the committed snapshot | a derived one-way pin between them, and nothing else |

**S-2(4)** The operative rule. A VDS artefact may hold a **requirement**. It may never hold
a **realisation**. A requirement is a duty imposed from outside the design (a contrast
floor drawn from WCAG, a required state, a prop contract, a keyboard obligation). A
realisation is the design's own answer to that duty (a colour, a length, a radius, a
spacing step, a font family or size, a duration, an easing curve, a shadow). Requirements
come from statute or external law. Realisations come from the named records in S-2(3).

**S-2(5)** The test a reader applies to a proposed artefact. All four limbs must hold, and
a proposed artefact that fails any one of them is in the storing form and is forbidden.

1. **Deletion.** Delete the artefact entirely. If any shipped or decided design value is
   thereby lost, it stored. If everything it held is recomputable by command from
   `app/globals.css`, the Figma file and the codebase, it derived.
2. **Divergence.** Change one named record so the two records disagree. A deriving artefact
   fails closed and says which rows diverged. A storing artefact keeps serving its own
   value and has become a third opinion nobody asked for.
3. **Authorship.** Ask whether a reader can change a shipped pixel by editing only this
   artefact. If yes, it is an authority, whatever it is called.
4. **Regeneration.** A pin or a ledger must be byte-reproducible by a named command from
   the named records. A registration need not be regenerable, because it carries intent
   rather than value, but it must carry no value that a named record also carries.

**S-2(6)** A numeral is not automatically a value. `minRatio: 3.0` in a component record is
a requirement drawn from WCAG 2.2 SC 1.4.11 and is lawful under S-2(4). `"#ebebeb"` is a
realisation and is not, wherever it appears in `.vds/`.

**S-2(7) AMENDED, and the original text is recorded because it was wrong.** As drafted this
clause read: "Where a proof must compare two values, it compares digests of the normalised
values, not the values. A pin row therefore carries `source_value_digest` and
`target_value_digest` and an agreement flag, and never the two strings. This is what keeps
the pin a gate rather than a store, and it is why the pin schema forbids a value field."

That construction does not do what the clause says it does, and the difference was measured
rather than argued. An unsalted SHA-256 over a low-entropy domain is not one-way in any
practical sense, and a design token value is a tiny domain: a hex colour is 24 bits, about
16.7 million candidates, and a spacing step or a duration is smaller. An adversarial agent
recovered all 52 values from a 26-row pin in 27 seconds on one CPU. A pin built as the
original clause required therefore STORES the decided and the shipped values, in a form that
is inconvenient to read and trivial to recover, which is exactly the storing form
[2026] VJS-CC-OPBOX 3 forbids. The guard specified at S-2(8) to catch that would have
certified it clean, because it looks for colour literals.

Salting does not rescue it. A salt recorded in the pin is a salt the reader has, so the
search is unchanged. A salt not recorded in the pin makes the pin irreproducible, which fails
the regeneration limb at S-2(5)(4).

**The operative rule is therefore:** a pin row carries the NAME of the thing compared and the
AGREEMENT between the two records, and nothing else. No per-value digest appears anywhere in
`.vds/`. Whether the records as a whole moved is answered by a digest of each whole record,
which is not a low-entropy domain and is safe.

This is a departure from text that is not commenced (S-15) and that mandated a construction
that provably does the opposite of what it claims. It is referred as `SUBMISSION-VDS-006`,
and the fail-closed interim is the rule stated above.

**S-2(8)** The rule S-2(2) actually needs is about **recoverability, not spelling**. An
artefact is in the storing form if a design value can be reconstructed from `.vds/**`,
whether it is written as a literal, an encoding, a digest, an index into an ordered set, or
any other reversible representation. The machine check that enforces S-2(2) is the
`no_stored_values` proof, and it has two limbs, both fatal:

1. **Literal limb.** Any colour literal, length literal, font family, duration or easing
   curve appearing verbatim anywhere under `.vds/**`. This is the limb the specification
   used to have, and on its own it certified the leaking pin clean, because a digest is not
   a literal.
2. **Preimage limb.** Any design value **recovered** from `.vds/**` within the recovery
   budget of S-2(9). The limb enumerates the candidate space, applies each reversible
   transform an artefact could have used, and matches the result against every digest-shaped
   and encoded-looking token harvested from the tree. A single match is a fatal finding
   naming the recovered value and the file it came from.

A guard that passes the very artefact it exists to catch is not a guard, so the preimage
limb, not the literal limb, is the one that decides whether this specification is honest.

**S-2(9)** "Practical" needs an operational definition or the rule is unfalsifiable, so it
gets one. Both quantities below are engineering choices, and moving either is an amendment.

- **Recovery budget: 2^40 evaluations.** About a CPU-day of commodity 2026 hardware. A
  value whose candidate space is smaller than the budget is recoverable, and digesting it
  does not take it out of the storing form. A joint digest is lawful where its preimage
  domain exceeds **2^128**, which is the same statement with the margin an adversary with
  more than a CPU-day deserves.
- **Candidate space: closed and enumerable**, which is what makes the limb decidable rather
  than a matter of opinion. It is exactly: the 2^24 srgb 8-bit colours in each spelling the
  named records use; lengths to three decimal places up to the largest the target record
  declares, in each unit it uses; durations to millisecond granularity up to ten seconds;
  and the font families named in the target record. Design realisations come from a small,
  human-authored, human-readable space, which is precisely why digesting them protects
  nothing.
- **Transform set:** identity, and each of `sha256`, `sha1`, `md5`, hexadecimal and base64,
  applied to each candidate. The set is open to extension and never to reduction. Adding a
  transform can only find more; removing one is a weakening edit under S-8(4).

The proof therefore fails in the direction that matters: seed `.vds/` with a digest of any
one colour and the run must exit non-zero and print that colour back. A `no_stored_values`
implementation that cannot do that has not satisfied S-7(2)(2) and is not a proof.

---

## S-3 The three trees

**S-3(1)** VDS separates normative text, this project's record, and engineering
explanation into three physically distinct trees. The split is copied from VJS, where it
is declared rather than incidental.

**S-3(2)** `designpack/v1/` at the repository root holds the normative corpus: `statutes/`,
`regulations/`, `invariants/`, `obligations/`, `orders/`, `judgments/`, `specs/`,
`provenance/`, and `manifest.toml`. It is versioned, digest-pinnable and vendorable on its
own.

**S-3(3)** `.vds/` holds this project's runtime record: `config.toml`, `register/`,
`warrants/`, `proofs/`, `pins/`, `ledgers/`, `submissions/`, `court/convenings/`,
`logs/decisions/`, `logs/breaches/`, `permits/`, `designpack.lock`, `install.lock` and
`enforcement.lock`.

**S-3(4)** `docs/` holds engineering explanation. Nothing in `docs/` binds, and no warrant,
order or invariant may cite it as authority.

**S-3(5)** The normative tree lives outside the dot-directory deliberately. A project
subscribes to a designpack by vendoring it read-only and pinning its digest in
`.vds/designpack.lock`, exactly as a VJS subscriber pins a lawpack. That is what allows a
second project to carry the same doctrine without copying this project's register.

**S-3(6) RESERVED.** Whether one designpack binds a single project, a tenant, or the whole
realm is not settled. The nearest authority is the locus ratio of [2026] VJS-CC-OPBOX 1.
Referred as `SUBMISSION-VDS-003`. Until it is answered, a designpack binds exactly the
project whose `.vds/config.toml` pins it, and no other.

**S-3(7)** `.vds/config.toml` is the one fixed anchor. Every other path is configurable
from its `[paths]` table by role. The file carries `version`, `jurisdiction_id`,
`repo_code`, `designpack = "<id>@<version>"`, `[paths]`, and `[governance]` with
`permit_required` and `permit_exempt` glob lists.

**S-3(8)** `permit_required` must name, at minimum: `app/globals.css`, the component
library directories, `designpack/v1/**`, `.vds/register/**`, `.vds/config.toml`, and the
proof scripts themselves. The enforcement machinery must not be editable without a permit,
or the gate can be removed by the same hand it constrains. `permit_exempt` covers the
append-only record directories: `.vds/logs/**`, `.vds/permits/**`, `.vds/proofs/**`.

**S-3(9)** The record is committed, not scratch. Only `.vds/cache/` and `.vds/private/` are
ignored. A governance record that is gitignored is not a record.

---

## S-4 The artefact set

**S-4(1)** VDS holds exactly ten artefact kinds (nine before the retention log was enacted at S-4(5)). Seven have a JSON Schema under `schema/`,
and a file that does not validate against its schema is not an artefact of that kind. The
remaining two, the decision log and the breach report, are ADOPTED from VJS and are validated
against VJS's schemas rather than redefined here; saying "each has a schema under `schema/`"
was wrong, because two of the nine never did.

The schemas under `schema/` are GENERATED from the implementation's types and are not maintained
beside them. A hand-written schema and a hand-written parser are two opinions about one
shape, and two opinions drift; `vds schema check` regenerates and diffs, so a divergence is
a failing check rather than a discovery months later.

The ninth kind, the SCREEN RECORD, was added by amendment on 2026-07-30 under S-7(6). The
reason is recorded at S-5A(1) and is not a matter of completeness: every artefact kind before
it described a COMPONENT, so a page could render only registered components, each in an
enforceable status, arranged in a way its frame does not draw, and every proof stayed green.

| artefact | path | schema | what it is |
|---|---|---|---|
| component record | `.vds/register/<id>.yaml` | `component-record.schema.json` | one registered component, its contract and its lineage |
| screen record | `.vds/screens/<id>.yaml` | `screen-record.schema.json` | one governed screen and the ARRANGEMENT it requires |
| warrant | `.vds/warrants/<id>.yaml` | `warrant.schema.json` | a stage gate granted on an evidence digest |
| proof result | `.vds/proofs/<id>.yaml` | `proof-result.schema.json` | machine output a warrant was granted against |
| pin | `.vds/pins/<id>.yaml` | `pin.schema.json` | a derived one-way agreement assertion between two named records |
| enforcement lock entry | `.vds/enforcement.lock` | `enforcement-lock-entry.schema.json` | a digest pin of one proof script, plus what invokes it |
| submission | `.vds/submissions/{draft,filed,docket}/<id>.yaml` | `submission.schema.json` | a question referred to VJS and the order that answered it |
| decision log | `.vds/logs/decisions/<id>.yaml` | (VJS log schema, adopted) | a reversible call, recorded not adjudicated |
| breach report | `.vds/logs/breaches/<id>.yaml` | (VJS breach schema, adopted) | a self-reported failure and its restorative remedy |

**S-4(2)** Ledgers (`.vds/ledgers/`) are generated inventories, never hand-edited. Each
ledger must have a staleness test that fails when its source changed and the generator was
not re-run. A ledger with no staleness test decays, and the evidence for that is in this
project already.

**S-4(3)** Permits (`.vds/permits/`) are adopted from VJS unchanged in form and meaning,
including the standing note carried on every self-issued permit: self-issue proves the
actor took the front door and is not an external authority's approval, and cannot satisfy
a check reserved to the Sovereign or to a constituted bench.

**S-4(4)** All identifiers are allocated by reading the live record off disk and taking the
maximum plus one. No identifier may be asserted by hand or held in memory across a run. A
collision is a fail-closed validation error, never a silent overwrite. VJS deleted an
in-memory citation registry for exactly this defect: it restarted every series at genesis.

**S-4(5)** The RETENTION LOG is the tenth artefact kind, enacted by
[2026] VJS-CC-VIBE-DESIGN-SYSTEM 6, and it is the only lawful exhaust of the only delete
power in the toolchain. `vds prune` operates on the WORKING SET, not on the record: git is
the append-only store behind `.vds/`, which is what makes pruning housekeeping rather than
destruction. The keep-rules are these, on the statute's face and not in the tool's help
text: the most recent record per proof kind is kept (D2, D3, D7 and D8 each settle on it);
every FAILURE is kept; anything a warrant cites is kept; and every removal is written to
the retention log, which lives in `.vds/`, is schema-governed like its nine siblings, and
is never pruned by the run it explains. Prune never runs unattended - it stays out of every
automated path, held there by an enforced test, because deletion is an act a person
initiates with these rules in front of them.

---

## S-5 The register

**S-5(1)** The register is what turns "confine design to the library" from a wish into a
checkable condition. Without it the composition proof at S-7 has nothing to check against.

**S-5(2)** One record per component, per `schema/component-record.schema.json`. The
load-bearing fields:

| field | why it is there |
|---|---|
| `id`, `name`, `status` | the lifecycle position, S-5(4) |
| `contractVersion` | so an amendment is a versioned event, not a silent edit |
| `figma` | file key and node id in the decided-target file |
| `code` | import path, source file and export name, or null if unbuilt |
| `props` | the contract, so Figma variants and code props are comparable |
| `states` | which of the nine states are required, which are drawn, which are built |
| `a11y` | role, accessible-name source, keyboard contract, and the contrast floors that bind it |
| `demand` | how many routes consume it, measured by a named command, so build order is evidence-led |
| `supersedes` / `supersededBy` | so retirement is traceable, S-9 |
| `amendments` | the contract's own history |

**S-5(3)** The nine states are fixed: `default`, `hover`, `focus`, `active`, `selected`,
`disabled`, `loading`, `error`, `success`. A component record may require a subset. It may
not invent a tenth.

**S-5(4)** The status lifecycle is a directed path and skipping is forbidden:

```
proposed -> designed -> registered -> built -> verified -> deprecated -> retired
```

`registered` means the contract is complete and binding. `verified` means a parity proof
passed against the built counterpart. `deprecated` and `retired` are governed by S-9.

**S-5(5)** A hand-maintained register decays. The measured evidence: this project's
`component-map.json` carries 56 component entries, while `src/components/ui` and
`src/components/onyx` together hold 90 `.tsx` files. Those are not the same measurement,
and one entry may legitimately cover several files, but nothing today derives either number
from the other or reconciles them. That gap is precisely where a register rots.

**S-5(6)** Therefore the register is reconciled by proof, in both directions, and the
reconciliation is a gate. A `reconciliation` proof (S-7) must report: register entries with
no resolvable code counterpart; code components in the governed library directories with no
register entry; register entries whose Figma node id does not resolve in the pinned file;
and prop or state contracts that disagree between the record and the code. Any non-empty
set is a violation.

**S-5(7)** `demand` is measured, never estimated. The record carries the command that
measured it and the timestamp. A `demand` figure older than its ledger's generation is
stale and the reconciliation proof says so.

---

## S-5A The screen register

**S-5A(1)** The register at S-5 governs COMPONENTS, and parity is a claim about SCREENS. That
gap is not a matter of coverage, it is structural: with a component register alone, a page
that renders only registered components, each in an enforceable status, in an arrangement its
frame does not draw, passes every proof at S-7(5)(1) to (10). The evidence is a real one. In
the subject this amendment was drawn from, a route was marked complete on 2026-07-30 while
rendering a whole extra column its Figma frame does not draw, and nothing in VDS could have
seen it, because the two proof kinds that say the word "screen" read a screen's REFERENCES
(which components it names, at which line) and never its arrangement.

**S-5A(2)** One record per governed screen, per `schema/screen-record.schema.json`. The
record holds: the screen's own name for itself (a route, a path), a lifecycle status from
S-5(4), the frame in the decided-target file that DRAWS it, and an arrangement contract.

**S-5A(3)** The arrangement contract holds a COUNT of side-by-side content panes and a list of
region NAMES. It may not hold a width, a height or any other length, because S-2(4) admits a
requirement and refuses a realisation and a width is the design's own answer. The prior art
this is drawn from records a frame's columns as `[924, 420]`; under `.vds/**` that is the
storing form S-2(2) prohibits, `no_stored_values` would fail on it, and it would go on failing
forever on a file VDS wrote itself, because a record is never deleted (S-9(1)).

**S-5A(4)** A screen with no split requires ONE column and never zero. One register for both
sides: where the two halves of a comparison count different things, agreement reads as a
deviation, and the prior art scored a route `frame=0 code=1` for agreeing. A contract of zero
columns is refused by the proof under S-7(2)(4), because no arrangement can render fewer than
zero panes and a requirement nothing can fail is not a requirement. A contract above the
ceiling the implementation fixes is refused for the twin reason: a row that can never pass is
the one people route around, and they take the other checks with them.

**S-5A(5)** The AUTHORITY of a frame is read from its layer names, and a frame is not one
drawing. It carries several and says which one governs. The vocabulary belongs to the project
that drew it and is configured, never fixed here; fixing it would make VDS an authority on
what a design file may call its own layers, which is the fourth authority
[2026] VJS-CC-OPBOX 3 forbids. A frame may also DISCLAIM ITSELF, saying in its own name that
it is not source-current or was never built. Such a frame states no contract, and a difference
measured against one is real and means nothing.

**S-5A(6)** The frame ledger is generated out of band from a SAVED capture and is a ledger
under S-4(2), with a staleness test. It records the CAPTURE DEPTH it derived and marks every
reading taken at that boundary. This is not caution. A Figma response carries no flag saying a
subtree was cut off, so a childless node at the capture depth and a genuinely empty one are
the same bytes, and only the depth asked for knows the difference. "The frame draws nothing
here" and "we did not look" may not be the same value, and a reading taken at the boundary may
not be enforced against.

**S-5A(7)** COVERAGE IS PART OF THE RESULT. The screen proof reports, per run, how many
registered screens were SCORED, how many were UNSCORED, and how many were EXCLUDED, and a run
whose tally does not account for every screen it considered is a precondition failure rather
than a pass. UNSCORED means a requirement exists and the run could not measure it, and every
one is a violation. EXCLUDED means the DESIGN states nothing to measure against, which is a
different fact and is not a failure. The rule exists because a screen gate that measures the
routes it happens to understand and prints a clean pass is the exact failure this whole
capability was added to prevent: the prior art's own gate scored 32% of routes and would have
reported zero deviations while 75 routes with a real multi-column contract were never compared
once.

**S-5A(8)** The CODE side is a seam and not a rule. A route's column count is the SUM of the
contribution of every node on the path from the route entry to the leaf: the innermost layout
component, every enclosing layout, and any shell whose count is fixed in the component rather
than in a prop. Reading that requires parsing the subject's own layout components and there is
no general answer, so the implementation states an interface a subject implements and VDS sums
what it returns. A node the subject cannot read makes the total UNKNOWN and the screen
UNSCORED, never a smaller number: contributing zero for a node nobody understood is
indistinguishable from a node that genuinely adds nothing, and the screen would then be scored
against a total that is quietly too low and passed.

---

## S-6 Warrants and the staged lifecycle

**S-6(1)** A warrant is an operative record: what was granted, on what evidence digest, by
whom, when, and what it unlocks. Its shape is the VJS order shape, per
`schema/warrant.schema.json`. Long rationale lives in `designpack/v1/judgments/`, so the
warrant itself stays short enough to load at runtime.

**S-6(2)** The four stages, in strict order. A stage may not be entered before the
preceding warrant is `granted`, and the ordering is the entire mechanism: every drift
defect measured in this project was authored before anyone asked whether the thing being
used was registered.

| warrant | granted on this evidence | granted by | unlocks |
|---|---|---|---|
| **W1 REGISTER-COMPLETE** | `register_completeness` proof exit 0 and `reconciliation` proof exit 0, both non-vacuous, over the declared surface | VJS on a referred submission | design may begin |
| **W2 DESIGN-COMPLETE** | `composition` exit 0 over every declared screen, plus `states` exit 0 and `contrast` exit 0 | RESERVED, S-6(6) | Principal review |
| **W3 PRINCIPAL-ACCEPTED** | a dated, digest-pinned acceptance event in `designpack/v1/provenance/assent/` | the Principal alone, S-6(7) | parity work may begin |
| **W4 PARITY** | `parity` exit 0 for every registered component, plus `token_pin` and `contrast` re-run against the shipped CSS | VJS on a referred submission | the system is done |

**S-6(3)** Every warrant names `case_file_digest`, the sha256 of the evidence file it was
granted against, and every proof it relies on by id and digest. A warrant carrying no
evidence entry is a signature on nothing and is void on its face.

**S-6(4)** A warrant is spent when the surface it was granted over changes. Adding a screen
after W2, or a component after W1, does not inherit the warrant: the proof re-runs and the
warrant is re-granted or refused. `status: spent` is recorded, never deleted.

**S-6(5) RESERVED.** Whether W1 may be granted provisionally is unsettled. W1-before-design
is what makes drift structurally impossible, and it may also be impossible to satisfy on a
greenfield surface, where the register cannot be completed without designing first. A
provisional registration that design may use and that W1 later ratifies is the obvious
relief and is also a hole large enough to drive the whole mechanism through. Referred as
`SUBMISSION-VDS-001`. Until answered, no provisional registration exists and W1 is strict.

**S-6(6) RESERVED.** Who may grant W2 is unsettled. Composition is fully machine-checkable,
so granting by proof alone with no bench is cheaper and faster, and a self-granted warrant
is the laundering vector the VJS assent-resolution line closed. Referred as
`SUBMISSION-VDS-002`. Until answered, W2 is referred to VJS like W1 and W4, and VDS may
record a proof-only candidate but may not treat it as granted.

**S-6(7)** W3 is the Principal's and no one else's. Acceptance is reserved to the Sovereign
under ACT-001:s2. No proof substitutes for it, no bench may grant it, and VDS may never
infer acceptance from silence. The acceptance event carries the digest of exactly what was
accepted, so a later claim that a screen was accepted is checkable against the bytes.

**S-6(8)** VJS adjudicates whether W1, W2 and W4 are earned on the evidence. It does not
adjudicate taste, and a submission that asks it to is defective.

---

## S-7 What makes a proof valid

**S-7(1)** The ratio that forces this section, from [2026] VJS-CC-OPBOX 3: "a gate that no
build invokes, over a contract in which no row is in an enforceable state, is not
enforcement and cannot be relied on as the reason not to consolidate."

**S-7(2)** A proof is valid only if all five conditions hold. A check failing any of them
is not a proof and may not be named as evidence for a warrant.

1. **Re-runnable.** One named command, deterministic, no network call, no model call. Same
   inputs give the same output and the same digest.
2. **Falsifiable.** It exits non-zero on a violation, **and** a named test seeds a violation
   against a fixture and asserts the non-zero exit. A check whose failing direction is
   asserted nowhere has proven only its happy path. The enforcement lock entry cannot be
   written without naming that test, which is how the condition is enforced rather than
   requested.
3. **Invoked.** Something runs it that is not the author choosing to run it. The invocation
   is declared in the enforcement lock entry.
4. **Non-vacuous.** It reports `rows_considered` and `rows_enforced` and warns when
   `rows_enforced` is zero. A pass over zero enforceable rows is recorded with
   `status: vacuous`, never `passed`. This is the exact defect [2026] VJS-CC-OPBOX 3 D3
   found: a printed drift PASS while 32 of 34 mapped tokens were `migrating`, 2 were
   `floor`, none was `aligned`, and the checker skipped both branches.
5. **Captured automatically.** The proof record is written by the checker as a side effect
   of running. A hand-written proof record is void, and the schema enforces this by fixing
   `capture_mode` to the single value `automatic`.

**S-7(3)** A hook is not CI. `git commit --no-verify` bypasses a local hook, and an author
with write access can edit a gate and re-pin it. Condition S-7(2)(3) is satisfied by a
remote check that re-runs the same deterministic gate on the pushed diff. A local hook
alone satisfies it only as an interim state, and the interim must be recorded.

**S-7(4)** Measured state of this project's two design gates at the time of drafting:
`scripts/design-token-gate.py` and `scripts/design-structure-gate.py` are invoked from
`.githooks/pre-push` at lines 106 and 123, and from 0 of the 10 workflows in
`.github/workflows/`. So they meet S-7(2)(3) on the hook limb and not on the CI limb, and
that is the honest description of where they stand.

**S-7(5)** The proof kinds are a closed registry. A proof of any other kind is not a proof.

| kind | what it establishes |
|---|---|
| `register_completeness` | every component referenced by any declared screen exists in the register |
| `reconciliation` | the register agrees with Figma and with the codebase, both directions, S-5(6) |
| `composition` | no screen uses an unregistered component; the anti-drift proof |
| `contrast` | every registered component's boundaries clear their floors in every theme |
| `states` | every required state of every registered component is drawn |
| `parity` | each registered component's code counterpart matches its props and states contract |
| `token_pin` | the two named records agree where the pin declares them aligned |
| `retirement_drain` | a component proposed for retirement has zero remaining consumers, S-9 |
| `ledger_staleness` | each generated ledger is current with its source, S-4(2) |
| `no_stored_values` | `.vds/**` holds no realisation AND yields none under the S-2(9) recovery test, S-2(8) |
| `screen_parity` | each registered screen's required arrangement is the one its authoritative frame draws, S-5A |
| `geometry` | each registered surface's SHAPE - radius, boundary weight, density, type scale - is the one the design system specifies, and the count of surfaces that do not comply is BOUNDED AND FALLING, S-7A |

The eleventh kind was added by amendment on 2026-07-30, by the route S-7(6) requires. The
first ten all read a COMPONENT; `register_completeness` and `composition` say "screen" and
read a screen's REFERENCES. So the arrangement of a page was checked by nothing, and S-5A(1)
records the defect that made that visible.

The twelfth kind was added by amendment on 2026-07-31, by the same route, and it closes the
same class of hole one level further down. The first eleven answer WHICH components a
surface uses, in WHAT state, and in WHAT arrangement. None of them answers what the surface
LOOKS LIKE. A page can compose only registered components, each in an enforceable status,
arranged exactly as its frame draws, and still read as the outgoing design, because radius,
boundary weight, density and type scale are none of those things.

S-7A records the defect that made it visible, and it is not hypothetical. On the subscriber
project the token layer was migrated to a new palette across six themes, every proof went
green, three separate progress numbers read high - 95.6% adoption, 193/199 routes, 0 owed
column deviations - and the application looked substantially unchanged to the person who
commissioned the work. The paint had moved and the shape had not: 561 hand-rolled
card-geometry containers, 376 legacy rule blocks and 385 files importing the outgoing
component library were all invisible to every proof in the registry.

### S-7A GEOMETRY, AND WHY A PIN IS NOT A PROOF

**S-7A(1)** The defect. A design system is adopted in two parts and only one of them is
easy to measure. The PAINT - which token a surface references - is a name, and a name is
trivially checkable. The SHAPE - corner radius, boundary weight, control density, spacing
step, type scale - is what a person actually sees, and no proof in this registry read it
until this amendment.

The consequence is specific and was observed: swapping one dark-neutral palette for another
dark-neutral palette is a real change that is nearly invisible, while the geometry that
carries the visual identity stays exactly where it was. Every instrument reported progress.
The product looked the same. Both were true.

**S-7A(2)** A geometry proof MUST state a bound and that bound MUST fall. This is the
operative clause and it exists because of how the defect survived after being found.

The subscriber project HAD a shape instrument before this amendment: a ratchet holding the
count of non-compliant containers at its current value so it could not rise. It reported
"561 hand-rolled card-geometry containers, pin 561". That is a floor, and a floor is a
different instrument from a target. A number that may only be held can never fall, and this
one did not: it moved from 667 to 561 through work done for other reasons, then stopped.

So: a `geometry` proof carries a bound and a DIRECTION. Refuse the proof if the bound was
not lowered within the declared window. A ratchet that never tightens is a record of a
defect, presented as a control.

**S-7A(3)** The bound is per SURFACE KIND, not one number for the estate. "561" names no
work: it cannot be assigned, it cannot be finished, and it hides which surfaces are worst.
The proof partitions by the shape being violated - radius, boundary weight, density, type
scale - so each row is a job somebody can pick up and drive to zero.

**S-7A(4)** Geometry is read from the SHIPPED artefact, never from a model of it. A code
model of the intended design is a legitimate design tool and is NOT admissible as the
subject of this proof: it is a third artefact that drifts, and on the subscriber project a
17-page code model of the design drifted so completely that it now models the OUTGOING
system it was built to replace. The proof reads what ships. The frames remain the target.

**S-7(6)** Adding a proof kind is an amendment to this specification and to the invariant
registry, not a script anyone may drop in. The registry is closed for the same reason VJS
closed its predicate registry: an open registry is a free-form script surface, and a
free-form script surface is not a gate.

**S-7(7)** The proof surface is the one that rots. VJS holds 173 decision logs against 3
proof records, measured at drafting. VDS's entire value rests on the proof surface, so
capture is wired into the checker under S-7(2)(5) rather than left to be written by hand,
and S-11(4) makes the ratio itself a checked condition.

---

## S-8 The enforcement lock

**S-8(1)** `.vds/enforcement.lock` pins the proof-script surface by digest, and is held
outside the scripts it witnesses. It moves the integrity witness out of the mutable surface
it guards: a weakening edit bumps a digest and trips a loud, blocking finding rather than
passing under its own possibly weakened logic.

**S-8(2)** Each entry names a path, its sha256, its kind, what invokes it, which proof
kinds it produces, and the test that proves its failing direction. The last field is what
makes S-7(2)(2) structural rather than aspirational.

**S-8(3)** The lock is opt-in. A repository with no lock file produces no drift finding, so
an unpinned project is quiet rather than broken. But a warrant may not cite a proof whose
script is absent from a present lock.

**S-8(4)** Drift is fatal. Re-pinning is deliberate: re-lock only after a recorded gate
change, and self-file the rationale under S-12(3).

**S-8(5)** What the lock cannot do, stated plainly rather than glossed. It cannot bind an
author with full write access who edits a gate and re-locks it in the same act. The
backstops for that residue are non-machine: the Principal's gate and the duty of reasonable
care. VDS records this as a named remainder, not as a solved problem, and no VDS document
may claim the enforcement surface is tamper-proof.

**S-8(6)** The positive direction of the drift check is itself tested: a test edits a
pinned file and asserts a fatal finding. Without it the lock proves only that unmodified
files are unmodified.

---

## S-9 Amendment and retirement of a registered component

**S-9(1)** A register with no exit rule rots, because the only lawful move becomes adding.
This section is the exit rule. A record is never deleted and an identifier is never reused.

### Amendment

**S-9(2)** Any change to a registered component's contract is an amendment, recorded in
`amendments[]` with `at`, `by`, `kind`, `what`, and the proof or warrant that supports it.
`contractVersion` increments on every amendment.

**S-9(3)** A **non-breaking** amendment adds an optional prop, adds a state to `drawn` or
`built`, tightens a contrast floor, or corrects a factual field such as `demand` or a
source path. It requires a decision log under S-12(2) and a passing `reconciliation` proof.
It does not require a warrant.

**S-9(4)** A **breaking** amendment removes or renames a prop, removes a required state,
changes a role or accessible-name source, or **lowers a contrast floor**. It requires a
warrant, because the surface it invalidates is the surface a warrant was granted over.

**S-9(5) Anti-relaxation.** A floor may be tightened by any project. A floor may never be
loosened below the designpack floor it inherits, and an attempt to do so is dropped at load
with a defect recorded, not merely warned about. Where a lower floor is genuinely correct
(for example because SC 1.4.11 does not reach a purely decorative rule), the correct move is
to change the component's **scope**, not its floor: record that the component is decoration
and carries no control boundary, with the basis stated. That is a factual claim a reviewer
can contest, whereas a quietly lowered floor is not.

### Retirement

**S-9(6)** Retirement is three phases and cannot be compressed.

1. **Supersession notice.** The record moves to `status: deprecated`, sets `deprecatedAt`,
   and sets `supersededBy` to the successor's id, or to `null` where the component is
   withdrawn outright with no replacement. From this moment the `composition` proof reports
   every consuming site as a warning, per site, by route. A deprecated component never
   passes silently.
2. **Drain.** `demand.routes` must reach zero, measured by the named command, not asserted.
   A `retirement_drain` proof records the measurement and its digest. While any consumer
   remains, retirement is refused: the proof exits non-zero and no warrant may be granted
   over the register in that state.
3. **Tombstone.** On a passing drain proof the record moves to `status: retired`, sets
   `retiredAt` and `retirementProofId`, and is kept forever. The identifier is never
   reused. The tombstone is what makes a `supersedes` chain readable years later, and it is
   why `supersedes` is an array: a successor may absorb several predecessors.

**S-9(7)** The successor named in `supersededBy` must itself be `registered` or later
before the predecessor may be deprecated. Deprecating toward a component that does not yet
exist is how a library ends up with two incomplete halves and no whole.

**S-9(8)** A retired record remains valid against the schema and remains readable by every
proof. `reconciliation` treats a retired record's absence from the codebase as correct, and
its **presence** in the codebase as a violation. That inversion is the point: after
retirement, the code being there is the defect.

**S-9(9) RESERVED.** Whether a component may ever be retired while a shipped route still
consumes it, on a forced-drain deadline ordered rather than measured, is unsettled.
Referred as `SUBMISSION-VDS-004`. Until answered, S-9(6)(2) is absolute and no deadline
overrides a non-zero demand.

**S-9(10) RESERVED.** Where the primitive floor sits is unsettled: whether a bare HTML
element in a screen counts as an unregistered component for the `composition` proof, or
whether some enumerated set of elements is below the register's floor. The answer changes
how much of the codebase the anti-drift proof reaches, so it must not be settled by
whichever choice makes the first run pass. Referred as `SUBMISSION-VDS-005`. Until
answered, `composition` reports bare elements as informational rows, counted in
`rows_considered` and excluded from `rows_enforced`, and any warrant relying on it says so.

---

## S-10 Referral to VJS

**S-10(1)** Every judgement call routes to VJS. The referral path is a submission under
`schema/submission.schema.json`, filed into `.vds/submissions/filed/` with the VJS
submission id, and answered by a VJS order whose citation and ratio are recorded back onto
the submission.

**S-10(2)** Before filing, the citator is checked. A submission must list every near
authority considered and say why each is not on all fours. A submission that skips the
citator check re-litigates settled law and is defective, and the schema requires at least
one citator entry for exactly this reason.

**S-10(3)** Referral is for the enumerated triggers only: a first-impression point, a
genuine distinction from an existing ratio, a proposal to overrule a precedent, an
instruction that conflicts with the designpack, or a discovered breach. Everything else is
a decision log under S-12(2).

**S-10(4)** Every operative citation in a warrant must resolve to a defined object. The
check is existence-only and never reads what the cited authority says. An unresolved
citation is a fatal finding.

**S-10(5)** A convening record pins the bench and the case file by digest, and the
resulting warrant repeats the same `case_file_digest`, so what was decided on is provable
after the fact.

**S-10(6)** VDS holds no appellate function. An appeal from a VJS order answering a VDS
submission runs in VJS, by VJS's own tiers, and VDS records the outcome.

---

## S-11 Accession, versioning and locks

**S-11(1)** `.vds/designpack.lock` carries `designpack_id`, `designpack_version`, `digest`,
`schema_version`, `generated_at` and `locked_by`. Adoption is vendored, read-only,
digest-pinned and fail-closed. The runtime never fetches doctrine.

**S-11(2)** A loader refuses, loudly and at load time, any designpack whose
`schema_version` exceeds what it understands. A loader that skips clauses it cannot parse
is silently lawless.

**S-11(3)** A digest bump is a deliberate recorded act. No doctrine flows downstream by
silence.

**S-11(4)** `.vds/install.lock` carries `schema_version`, `generated_at`, `config_digest`,
`hooks[]`, `hook_digests[]` and `adapters[]`, so a missing or altered hook is itself a
finding rather than a quiet absence.

**S-11(5)** Two front doors, exactly one wall. Any CLI, editor plugin, MCP verb or Figma
integration is a convenience door over one checker binary. The wall is the gate that runs
whether or not the door was used. "The author used the tool" is never proof of conformance.

---

## S-12 Permits, decisions and breaches

**S-12(1)** A governed write needs a permit: scoped by path glob, carrying its obligations
by id, expiring, and closed by proof. The lifecycle is
`route -> permit -> obligations -> proof -> log -> validate`.

**S-12(2)** A reversible call with low blast radius is a decision log, not a referral. The
log carries `court_required: false` and `why`, which is what records that a fork was
considered and disposed without a sitting. This is what keeps referral cheap enough to
actually use.

**S-12(3)** A self-reported breach is a first-class record with a fixed schema:
`what_happened`, `law_breached[]` with each entry citing an instrument, `discovered_by`,
`containment`, `remedy[]`. Remedy is restorative, not punitive: the work is made good and
the lawful route resumed.

**S-12(4)** The two defects at S-1(4) are the founding breach entries, and they are filed
as breaches rather than described as background, because a system whose first act is to
excuse the failures that motivated it has taught itself the wrong lesson.

---

## S-13 Reserved matters, collected

Every clause that depends on an unsettled point, with its submission. Nothing here may be
implemented until the submission is answered, and no VDS document may state a ruling on
any of them.

| clause | question | submission |
|---|---|---|
| S-6(5) | may W1 be granted provisionally on a greenfield surface | `SUBMISSION-VDS-001` |
| S-6(6) | who may grant W2, given composition is fully machine-checkable | `SUBMISSION-VDS-002` |
| S-3(6) | does a designpack bind one project, a tenant, or the realm | `SUBMISSION-VDS-003` |
| S-9(9) | may a component be retired against a forced-drain deadline while consumers remain | `SUBMISSION-VDS-004` |
| S-9(10) | where the primitive floor sits for the composition proof | `SUBMISSION-VDS-005` |

---

## S-14 Cost, stated honestly

**S-14(1)** The register is the expensive part, and it is expensive whether or not VDS
exists, because it is just "write down every component and what it must do". Anyone
proposing to skip VDS to avoid that cost has not avoided it: they have decided not to write
it down, which is the state that produced both defects at S-1(4).

**S-14(2)** What VDS adds on top of the register is the gating, which is comparatively
cheap, and the proofs, most of which are small scripts of the kind already written in this
project. Two of the twelve proof kinds exist today in some form.

> **Superseded on this point by S-14A(3): all eleven are implemented.** S-14(2) is left as
> written because it is a COST FORECAST made at drafting, and a statute that quietly edits its
> own forecasts to match the outcome cannot be checked against what it predicted. The number
> here is history; S-14A(3) is the current state. Read them in that order.

**S-14(3)** What VDS adds in ongoing cost, stated so nobody is surprised: every new
component needs a record before it may be used; every contract change needs an amendment
entry; every warrant re-runs its proofs when the surface changes; and the reconciliation
proof will fail on day one and keep failing until the register is genuinely complete. That
last one is a feature and will not feel like one.

**S-14(4)** What VDS does not buy: it does not make the design good, it does not reduce the
work of designing, and it does not remove the need for the Principal to look at the screens.
It converts a class of silent failure into a loud one at authoring time. That is the whole
return, and it is worth the cost only because the failures in that class are the ones that
reach production and stay there for months.

---

## S-14A The implementation

**S-14A(1)** VDS is implemented as a Rust workspace producing one binary, `vds`, on a pinned
toolchain. This is not a free choice: VJS is a Rust workspace, VDS refers every judgement call
to VJS (S-1(2)), and two governance systems with one purpose and two toolchains is the
fragmentation the pair exists to avoid.

**S-14A(2)** Every closed set this specification fixes is a type rather than a validated
string. A tenth state (S-5(3)), a status outside the lifecycle (S-5(4)), a proof kind outside
the registry (S-7(5)) and a `capture_mode` other than `automatic` (S-7(2)(5)) are each
unrepresentable rather than invalid. Where a rule can be made structural it is made
structural, because a rule enforced at runtime is a rule that can be reached with the check
disabled.

**S-14A(3)** All twelve proof kinds at S-7(5) are implemented. `unimplemented_because` is kept on
the type rather than deleted: a kind that later has to be withdrawn must record WHY, per kind,
rather than disappearing from a match arm, and the difference between work and a dependency is
what tells a reader which it is. What remains unbuilt is a pin GENERATOR: `token_pin` checks a
pin and nothing in this build produces one, because one of the two records it compares is
behind a network call S-7(2)(1) forbids inside a proof. Its runs are `vacuous` and say so.

---

## S-15 Commencement

**S-15(1)** This specification does not commence on being written. It commences when a
dated, digest-pinned assent event exists in `designpack/v1/provenance/assent/` naming this
document's digest, and `.vds/designpack.lock` pins the pack containing it.

**S-15(2)** Until commencement, no warrant may be granted, because there is nothing to grant
one under.

**S-15(3)** The five reserved matters at S-13 do not block commencement. They block the
clauses that depend on them, and those clauses fail closed in the meantime: strict W1, W2
referred, absolute drain, informational-only bare elements, and single-project binding.
