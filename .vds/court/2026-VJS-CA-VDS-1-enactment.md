# [2026] VJS-CA-VDS 1 - Enactment of the visual-gap proof kinds

Court of Appeal (Fidelity, Engine, Estate JJ) on SUBMISSION-VDS-012 to 017,
sitting under the binding ratios of [2026] VJS-SC-OPBOX 1. All six ENACTED,
five as amended; the richness-doctrine repeal carried on option 1, unanimous.

---

## Judge Fidelity, Court of Appeal

| S012 ENACT_AMENDED (option 1) | S013 ENACT (option 1) | S014 ENACT_AMENDED (option 1) | S015 ENACT_AMENDED (option 1) | S016 ENACT_AMENDED (option 1) | S017 ENACT_AMENDED (option 1, as amended) |

== JUDGE FIDELITY, COURT OF APPEAL: SUBMISSIONS VDS-012 TO VDS-017 ==

PRELIMINARY. I have read [2026] VJS-SC-OPBOX 1 (orders 1-31, ratio Q1-Q4) and all six submissions in full, and I have read the implementing branch feat/proof-kinds-visual-gap at /home/jellytot/Projects/vibe-design-system. I test each draft against the binding ratios and, where the draft claims to be built, against the build.

The apex court has already decided most of this lane. Four of the six submissions are implementations of its ratios and are enacted without re-litigation under S-11(c). Two of them state, on their face, rules the apex court has already qualified. Where that happens my disposition is amendment and not refusal: a submission that under-states a binding qualification is cured by reading the qualification in, because the qualification binds whether or not the statute repeats it, and a statute that contradicts it would be a second copy of a rule, which is one copy and a disagreement.

Three findings run across the set and I take them first, because two of them are mine to make and all three change dispositions.

FINDING A: A TWO-VALUED AGREEMENT BIT CANNOT EXPRESS A FRAME'S SILENCE. SC-1 order 6 binds a frame "only for what it draws in the states it draws", and order 14 forbids agents to "manufacture conformance from what was never drawn". In crates/vds-core/src/types/geometry.rs the authority snapshot's row is `pub agrees: bool`. There is no value for "the signed frame draws nothing on this dimension". An out-of-band comparator that finds nothing to compare has only one honest-looking bit to write, and it will write `true`, and the proof will then record conform against a signed frame on ground the frame never drew. That is order 14 breached by a datatype. It reaches S-012 directly and S-016 by analogy.

FINDING B: A DEADLINE MEASURED ONLY AGAINST THE INPUT IT GATES IS A DEADLINE THE SUBJECT SETS. Draft S-7C(3) measures the burndown deadline against the reading's own `taken_at`, and crates/vds-proof/src/burndown.rs fires R3 only where `reading.taken_at > deadline`. I grepped for `generated_at` in that file and in the crate root and found nothing: the burndown proof reads no independent freshness witness at all, and `ledger_staleness` does not cover burndown readings. A subject that stops regenerating its reading therefore outlives its undertaking in silence. SC-1 order 12 requires a redraw obligation "enforced by a deadline gate on the model of the 2026-09-15 parity fence", and order 27 keeps that fence as a hard date. A gate whose clock the gated party winds is not that gate.

FINDING C: SCREEN_PARITY HAS NO ACCEPTANCE SURFACE. Both of S-017's options assert one. I measured. `grep -i accept` across crates/vds-proof/src/screen_parity.rs and crates/vds-core/src/types/screen.rs returns a single hit about node-id spelling. The comparison at screen_parity.rs:715 is a bare inequality, `record.arrangement.columns != columns`. So the engine already scores a code side richer than its frame as a difference. The acceptance surface the doctrine actually occupies is in the consuming repository: the Figma richness pin on the opbox-frontend pre-push hook, and the fifteen accepted-over rows that SC-1 order 26 disposes of. This is decisive for S-017 and I return to it there.

-------------------------------------------------------------------

SUBMISSION-VDS-012 (geometry two-sided limb, S-7A(5)). ENACT_AMENDED, OPTION 1.

The submission is on all fours with SC-1 Q2. Order 6 says a frame binds from hash-bound registry entry for what it draws; a snapshot that holds only agreement bits and two input hashes, and that dies visibly when either hash moves, is that order made mechanical. Option 3 (stale only on the authority side) is refused for the reason the draft itself gives and the reason Q4 gives: a binding that keeps citing an agreement measured against a predecessor artefact is a check that cannot fail, which is the 561-pin-561 instrument and the very defect Q4's ratio names. Option 2 refuses the only instrument that can say "what shipped is what was decided" and is rejected.

The implementation is sound in the parts I can test. crates/vds-proof/src/geometry.rs R11 witnesses the snapshot's own content, R12 stales on both sides, R13 fails on disagreement against a signed frame, and W2 resolves an unsigned frame to no_authority. Critically, at geometry.rs:667 a row asserting `agrees` against an Unsigned frame is refused rather than passed. That is order 14 respected at the authority boundary.

But the same order is breached one level down, by Finding A. Three amendments.

AMENDMENT 012-A (three-valued agreement). Insert as S-7A(5A):
"The agreement row is THREE-VALUED, never two. Per surface kind the comparator records `agrees`, `disagrees`, or `not_drawn`. `not_drawn` is a first-class state meaning the signed frame draws nothing on that dimension; it resolves to no_authority for that surface kind and can never contribute conformance. A frame binds only for what it draws in the states it draws, and a comparator that finds nothing to compare and reports agreement manufactures conformance out of silence ([2026] VJS-SC-OPBOX 1, orders 6 and 14). A snapshot whose rows are all `not_drawn` is a binding nothing can fail and is refused under S-7(2)(4)."

AMENDMENT 012-B (commencement). Insert as S-7A(5C):
"S-7A(5) commences only upon the commencement of S-7D(2) and S-7D(3). Until the sign-off register exists in this specification the authority limb has no register to read, every binding it could record would resolve no_authority, and a limb that resolves to one state on every input is not a control (S-7(2)(4))."

AMENDMENT 012-C (no primacy conferred). Insert as S-7A(5D):
"Nothing in this clause confers primacy. Primacy attaches per proof kind only on the terms of [2026] VJS-SC-OPBOX 1 orders 2 to 5. No repo-local gate on the same ground may be retired until the covering proof has run red on a seeded counterexample of the specific incident class that gate was forged against, on the repository concerned; until that demonstration both run, and where they disagree the stricter reading enforces and the disagreement is itself filed."

012-C is declaratory of binding law and is not a re-litigation. I enact it because the consuming repository reads this statute and not this judgment, twenty-eight local gates stand on the ground these kinds cover, and order 3 is the only thing standing between this lane and a retirement that removes a working instrument in favour of an untested one.

-------------------------------------------------------------------

SUBMISSION-VDS-013 (prohibition kind, S-7B). ENACT, OPTION 1.

This one needs no amendment and should not be delayed by the argument in the others. SC-1 order 22 expressly contemplates the kind: "ruling 140 survives strengthened, each new prohibition proof kind being itself a 140 obligation". The apex court has therefore already held that new prohibition proof kinds may exist and has fixed the condition on which they do: each carries a gate and a seeded failing-direction test. S-8(2)'s test field carries that condition structurally, and the submission's fail-closed record states the kind is lock-pinned with its seed-red test.

The draft contradicts no ratio. It is not frame-bound, so Q2 to Q4 do not reach it. It is a Q1 instrument and the order 2-5 conditions attach to it automatically by 012-C's declaratory clause and by the orders themselves.

On the merits, the two silent failure modes it refuses are the right two, and the recorded-expansion rule is the better half. A prohibition without a recorded expansion is the "silently narrowed scope" defect: a pass over a smaller population reads exactly like a pass over the original, and W1's decision to SCAN growth as well as warn on it is correct, because warning without scanning would create an unenforced shadow while the baseline caught up. R4 and R5 refuse the vacuous forms, which is the vacuity discipline applied to the kind's own founding.

Option 3 (regex) is rightly refused as the default. An unescaped dot that silently widens a match is precisely the silent-scope defect the kind exists to refuse, and adopting the expressive form by default would build the defect into the cure. The draft's reservation of an explicit regex flag to a future amendment is the correct disposal.

-------------------------------------------------------------------

SUBMISSION-VDS-014 (burndown kind, S-7C). ENACT_AMENDED, OPTION 1.

The kind is not merely lawful, it is ordered. SC-1 order 24 requires that "all deviation proofs run estate-wide in report-only (burndown) mode from day one, each surface flipping from report to block only at the moment its frame is registered". The court used the word. This submission builds the instrument that word names, and refusing it would leave order 24 with no mechanism.

Option 3 (a plain ratchet) is refused for the reason S-7A(2) already gives and the record already proves: a number that may only be held has invisible headroom below the pin, a metric that fell from 100 to 60 under a pin of 100 can regress forty times in silence, and the subscriber project's 667-to-561-then-stop is that failure observed. The unre-pinned-decrease rule is the whole difference between a burndown and a ratchet and it is the clause worth having.

Amended for Finding B alone.

AMENDMENT 014-A (a deadline may not be deferred by a reading that stopped moving). Insert as S-7C(5):
"A deadline may not be deferred by a reading that stopped moving. A burndown record carrying a deadline MUST also declare a maximum reading age in days. The proof is fatal where the reading's `taken_at` precedes, by more than that many days, the most recent `generatedAt` among the ledgers the run read. The clock is never the wall clock (S-7(2)(1)); it is the run's own freshest independent input, so a subject that stops regenerating the reading cannot thereby outlive the undertaking that reading witnesses. A deadline whose only clock is the input it gates is a deadline the subject sets."

The witness named is one the engine already holds and already uses in this pattern: `register_completeness` measures `directedAt` against `ledger.generated_at` at crates/vds-proof/src/register_completeness.rs:331. The amendment is therefore buildable within the determinism constraint of S-7(2)(1) and needs no new input.

-------------------------------------------------------------------

SUBMISSION-VDS-015 (measurement coverage, S-5(9)). ENACT_AMENDED, OPTION 1.

OPTION 1 (housed in register_completeness) over option 2 (a sixteenth kind). The submission offers the choice honestly and invites the bench to prefer purity. I do not, for three reasons. First, S-7(6) makes adding a kind an amendment to the specification AND to the invariant registry, with an enforcement-lock entry and a seed test, and S-7(7) records that the proof surface is the one that rots: a two-rule kind is a rot surface bought to keep a scope note tidy. Second, the fields live on the component record that `register_completeness` already reads, so a sixteenth kind would put two proofs over one artefact and invite exactly the disagreement order 4 has to arbitrate. Third, the honest cure for an outgrown scope note is to amend the note, not to build a second instrument around it. The submission's own worry is right and is met by writing the widening down rather than by relocating the rule.

The defect is real and measured, and it is the one the register was built to prevent: a rule row registered with nothing measuring it stays green forever, and a row "measured" by pointing at a plan is measured by prose. Extending the docs-bind-nothing principle from warrants and orders to MEASURES is the correct extension: a measure that cannot fail is worse than an absent measure, because it reports.

Two defects in the implementation as written, and one gap.

The first is at register_completeness.rs:302-316. R3 is a DENYLIST: it refuses `.md`, `internal-docs/`, `docs/`, `plans/`, `readme`. It therefore refuses the document paths it happens to know and admits a sentence. `measuredBy: "the enhancement charter"` passes. `measuredBy: "see the migration brief"` passes. A rule measured by a sentence is the prose this clause exists to refuse, arriving through the front door.

The second is at register_completeness.rs:291-294: `has_metadata` gates the whole block and a record carrying neither field is skipped. The clause is therefore opt-in and its opt-out is silent. That is lawful, and the fail-closed record is right that nothing reddens retroactively, but an opt-in rule with a silent opt-out measures the rows somebody remembered to mark and prints a clean pass over the rest.

AMENDMENT 015-A (invert R3 to an allowlist). Insert as S-5(9)(a):
"A measure is well formed only in a recognised form: the name of a proof kind in the closed registry at S-7(5), or a repository-relative path to a file that exists in the subject tree and is not a document. Anything else is refused. The rule is an ALLOWLIST and never a denylist of document-shaped strings: a denylist refuses the paths it happens to know and admits a sentence, and a measure that is a sentence is the prose this clause exists to refuse."

AMENDMENT 015-B (coverage is part of the result). Insert as S-5(9)(b):
"The run reports, per run, how many register records carry a measurement-coverage field and how many carry neither, on the S-5A(7) principle that coverage is part of the result. A record carrying neither field is outside the clause and is not a failure, but it is COVERAGE OWED and is counted where the verdict is read. An opt-in rule whose opt-out is silent measures the rows somebody remembered."

AMENDMENT 015-C (record the widened scope). Insert as S-5(9)(c):
"The scope note of `register_completeness` is amended to read: existence, and the measurement coverage of a directed record. The widening is recorded in this specification and in the enforcement-lock entry for that proof, and is never left standing in a scope note the proof has outgrown."

-------------------------------------------------------------------

SUBMISSION-VDS-016 (visual_review kind, S-7D(6)). ENACT_AMENDED, OPTION 1.

This submission implements a binding ratio almost verbatim and is enacted without re-litigation on its central question. SC-1 order 16: "Machine visual_review classifies against the registered record; it never adjudicates taste and its verdicts create no authority." The apex court named this kind, in these words, and settled that it may exist and on what terms. S-11(c) forbids re-opening it.

The implementation honours the order in the place it matters most. At crates/vds-proof/src/visual_review.rs:262 the proof DERIVES authority itself from the sign-off register rather than trusting the record's own verdict field, and at :268 a recorded `conform` against an unsigned frame is refused. A verdict therefore creates no authority: it is classified against the registered record, exactly as order 16 requires. The staleness discipline is three-sided (shipped source, frame content, and the authority itself) and any of the three moving ends the verdict rather than degrading it, which is the right shape.

I also record that per-surface activation, which SC-1 orders 20, 23 and 24 make a condition precedent, falls out of the construction: a deviate finding arises only in the `(Signed, Deviate)` arm at :315, and an unsigned frame takes the `Skipped("no_authority_frame_unsigned")` arm at :283. There is no estate-wide flag day in this code and there is no scoring of unregistered ground as deviate. That is order 20 satisfied by construction and I say so rather than leaving it to be re-checked.

Option 3 is refused: the pipeline inside the engine violates S-7(2)(1) on both limbs, as the draft concedes. Option 2 re-invents the staleness discipline per repository, which is this lane's founding failure repeated.

One breach, at the remedy line. At visual_review.rs:326-336 the R5 finding tells the reader: "The resolution path is a proposed redraw closed by a NEW sign-off, then a re-review; there is no engine-side excusal." SC-1 order 19 gives THREE resolution routes, not one, and order 18 makes the deviation remedially inert as to removal. A remedy line that names one route and says "there is no excusal" will be read by an agent as an instruction to make the surface match, and 155 O7 is the exclusive law of removal. A frame's silence can flag code; it can never kill it.

AMENDMENT 016-A (remedial inertness on the face of the finding). Insert as S-7D(6A):
"A deviate verdict is a classification and never an execution order. The finding's remedy line states, in terms, that the difference remains live pending disposition, that no instrument may auto-remove, auto-hide or unrender a shipped surface on the strength of it, and that its sole automatic consequence is a proposed redraw. It further states the three routes by which the deviation resolves: a covering sign-off adopting it, an express registered direction parking it, or a deletion that independently discharges the removal burden of [2026] VJS-CC-OPBOX 155 O7 by an informed-deletion signature reciting the live function destroyed or proving it dead or homed ([2026] VJS-SC-OPBOX 1, orders 18, 19 and 21). A remedy line that reads as an instruction to delete is itself the defect: the engine classifies, and O7 disposes."

AMENDMENT 016-B (commencement). Insert as S-7D(6B):
"S-7D(6) commences with S-7D(2) and S-7D(3) and not before."

-------------------------------------------------------------------

SUBMISSION-VDS-017 (sign-off register, authority states, redraw primitive; repeal of the acceptance doctrine). ENACT_AMENDED, OPTION 1 AS AMENDED.

THE OPTION. Option 2 (new kinds only, two regimes coexist) cannot stand. Its stated virtue is that "no enacted behaviour changes without its own ruling", and that virtue is spent: SC-1 order 17 IS that ruling. The apex court has overruled the richness doctrine prospectively in its automatic-acceptance limb, and it did not confine the overrule to kinds not yet built. An enacted instrument that keeps an automatic-acceptance limb after order 17 is an instrument enforcing a rule that has ceased to exist, and Q4's ratio names it: a rule that accepts every addition is a check that cannot fail. Two regimes cannot coexist once one of them has been overruled; what would coexist is the law and a survival of it.

So option 1 in its direction. But option 1's consequence clause, "screen_parity's acceptance surface is removed in a follow-up amendment", is wrong twice over, and I amend rather than adopt it.

It is wrong as fact. Finding C above: I measured the branch and `screen_parity` holds no acceptance surface. Its comparison is a bare inequality at screen_parity.rs:715 and it already scores a richer code side as a difference. Enacting option 1 as drafted would order a follow-up amendment against a surface that does not exist, which is a direction reaching nothing, and the follow-up would report success having changed no behaviour anywhere.

It is wrong as law. The acceptance data is not disposable. SC-1 order 26: the fifteen accepted-over rows "convert to redraw proposals on paper only, each carrying its original acceptance's 155 O2 direction and magnitude as the redraw brief; they retain render rights and no gate may report them as deviations pending disposition by covering sign-off". Deleting an acceptance surface would destroy the direction and magnitude that the successor regime runs on. Acceptance is repealed as a VERDICT. It is preserved as a RECORD.

WHAT ACTUALLY HAPPENS TO SCREEN_PARITY'S ACCEPTANCE SURFACE, on either option, stated plainly because the bench was asked: nothing is removed from `screen_parity`, because there is nothing there. What `screen_parity` is missing is the other half of order 17, and that is the half that must be enacted. Today the proof scores a column mismatch as a finding against ANY registered screen, signed frame or not. Under orders 20, 23 and 24 the regime binds a surface only from that surface's registry sign-off, and before then the surface is no_authority which no parity gate may score as deviate. So `screen_parity` must become authority-aware, exactly as `visual_review` and the geometry limb already are. The acceptance surface the doctrine really occupies is the consuming repository's Figma richness pin and its fifteen accepted-over rows, and those CONVERT to parked redraws under order 26.

The draft also contradicts order 19 on its face. S-7D(4) as drafted: "The resolution path is a new signed frame version, never an engine-side excusal." Order 19 gives three. The draft's single route is the apex court's route (i) alone, and it silently drops route (ii), an express registered direction parking the addition, and route (iii), a deletion that independently discharges O7. That cannot be enacted as drafted.

THE BUILDABILITY GAP THIS EXPOSES, and it is the most consequential thing in this judgment. SC-1 order 30 makes every Principal direction that disposes of a surface's conformance a registrable sign-off act "hash-bound to its logged decision", and order 31 orders the four directions of 2026-08-01 backfilled as the register's founding entries. The implemented `SignOff` struct at crates/vds-core/src/types/signoff.rs requires `file_key`, `node_id` and `frame_digest`, and `frame_authority()` grants authority only where the frame's CURRENT content digest equals a row's. A direction does not change the frame. It therefore cannot produce a covering frame digest, and there is no field in which to record the log id and the decision digest that order 30 binds it to. As implemented, S-017 CANNOT register the four rows order 31 mandates. Likewise `RedrawStatus` is closed at proposed, drawn, signed, withdrawn, with `withdrawn` documented as "the deviation stands and stays red": there is no state meaning "parked under a registered direction, render rights retained, not reportable as a deviation", which orders 26, 28 and 29 require and which the band itself is currently sitting in.

One amendment closes all of it. Five amendments follow.

AMENDMENT 017-A (S-7D(4) restated). S-7D(4) is enacted in the following terms, replacing the drafted text:
"NO ACCEPTANCE STATE. The direction-blind and direction-carrying acceptance concepts do not exist in this engine, for any frame-bound kind, new or enacted. On ground covered by a signed, registered frame, an addition the frame does not draw is a deviation exactly like a missing element, and the richness-is-a-floor acceptance doctrine is repealed as an AUTOMATIC ACCEPTANCE, prospectively, in conformity with [2026] VJS-SC-OPBOX 1 order 17. The repeal is of the automatic acceptance and of nothing else. A deviation-by-addition is remedially inert as to removal; its sole automatic consequence is a proposed redraw, the addition remaining live pending disposition; and it resolves by one of exactly three routes: (i) a covering sign-off adopting it, (ii) an express registered direction parking it under S-7D(2A), or (iii) a deletion that independently discharges [2026] VJS-CC-OPBOX 155 O7. A signed frame's silence is never a deletion order."

AMENDMENT 017-B (direction acts). Insert as S-7D(2A):
"DIRECTION ACTS. A Principal direction that disposes of a surface's conformance is itself a sign-off act and enters the same register, hash-bound to its LOGGED DECISION rather than to a frame's content ([2026] VJS-SC-OPBOX 1, orders 15, 30 and 31). A direction row records the log id, the digest of the logged decision, the surface it touches (file key and node id where it names one, the route where it does not), its direction and magnitude in the 155 O2 form, the signer, the date, and a redraw-by date. A direction row confers authority for its own terms only, later in time, and carries a live duty to redraw so the frame record converges on the directed state. While a direction row stands and its redraw-by date has not passed, a deviation it covers is REPORTED and never fatal; past that date the proof is fatal on the REDRAW DUTY and never on the shipped surface. An unregistered direction is not instrument-readable authority: authority the instruments cannot read is authority the estate does not have."

AMENDMENT 017-C (parked redraws; the order 26 conversion). Insert as S-7D(5A):
"RedrawStatus admits a fifth value, `parked`, lawful ONLY with a `directedBy` naming a direction row under S-7D(2A). A parked redraw retains its subject's render rights, is reported and never fatal, and carries the direction's own words as its brief. Rows converted from a prior acceptance record migrate to `parked` and MUST carry the original acceptance's direction and magnitude; the conversion preserves that data and never discards it ([2026] VJS-SC-OPBOX 1, order 26). Acceptance is repealed as a VERDICT and preserved as a RECORD."

AMENDMENT 017-D (the repeal's reach, and screen_parity). Insert as S-7D(4A):
"The repeal reaches every frame-bound kind, `screen_parity` included; there is no two-regime interval. As measured on the implementing branch, `screen_parity` holds no acceptance surface to remove: its comparison is a bare inequality and it already scores a richer code side as a difference. What it lacks is the other half of order 17, and this clause supplies it. `screen_parity` becomes AUTHORITY-AWARE: a screen whose frame carries no sign-off row matching the frame's current content digest resolves `no_authority`, is reported as coverage owed, and may NOT be scored `deviate`. The regime binds a surface only from that surface's registry sign-off, and until then the proof reports without blocking ([2026] VJS-SC-OPBOX 1, orders 20, 23 and 24). The acceptance surface the doctrine occupies in fact lives in the consuming repository, being the Figma richness pin and the fifteen accepted-over rows, and it is CONVERTED under S-7D(5A) and never deleted."

AMENDMENT 017-E (commencement and the founding entries). Insert as S-7D(7):
"S-7D commences on enactment, and the register is a condition precedent to any frame-bound proof running in blocking mode. Only frames labelled CURRENT SOURCE are registrable; LEGACY/REFERENCE, TARGET/proposal, and self-disclaiming frames are no_authority per se and are registrable only after redraw. The four directions of 2026-08-01 (flat containers; no floating on the dotmatrix; sidebar to the frames' shell; band off-screen, LOG ids 103751, 103951, 104325, 104739) are the register's founding entries and are backfilled as direction rows under S-7D(2A) before any frame-bound proof runs in blocking mode."

THE INTERIM IS UNDISTURBED. The submission's fail-closed record is correct and I endorse it: the enum is closed at three, `accepted` does not parse, and an empty register makes every frame-bound verdict no_authority, which is coverage owed and never green. That is the fail-closed direction and it is the right posture between this judgment and the register's first row.

-------------------------------------------------------------------

== ORDER OF DISPOSITION ==

S-012 ENACT_AMENDED, option 1, with amendments 012-A, 012-B, 012-C.
S-013 ENACT, option 1, unamended.
S-014 ENACT_AMENDED, option 1, with amendment 014-A.
S-015 ENACT_AMENDED, option 1, with amendments 015-A, 015-B, 015-C.
S-016 ENACT_AMENDED, option 1, with amendments 016-A, 016-B.
S-017 ENACT_AMENDED, option 1 as amended, with amendments 017-A to 017-E.

DEPENDENCY, recorded so the enactment order cannot go wrong: S-012's authority limb and S-016's proof both call the sign-off register (geometry.rs:643, visual_review.rs:110). Neither commences unless S-017's S-7D(2) and S-7D(3) commence. S-013 and S-014 are independent and may commence at once.

== RATIO DECIDENDI (Fidelity J) ==

FIRST. Where a submission implements a binding ratio it is enacted without re-litigation; where its drafted text states a rule the apex court has already qualified, the qualification is read into the enactment and the submission is amended, never refused. A statute that under-states a binding qualification is cured by amendment, because a statute that contradicts one is a second copy of a rule, and a second copy of a rule is one copy and a disagreement.

SECOND. A two-valued agreement cannot express a frame's silence. Every instrument that binds a shipped reading to a signed frame must carry a third state for a dimension the frame does not draw, that state resolving to no_authority and never to conformance, and a binding all of whose rows are undrawn is a control that cannot fail and is refused. This follows from [2026] VJS-SC-OPBOX 1 orders 6 and 14 and is their consequence at the level of the datatype: an order that forbids manufacturing conformance from silence is defeated by a field with nowhere to record that the frame was silent.

THIRD. A deadline measured only against the input it gates is a deadline the subject sets. A deadline clause must take its clock from the run's freshest independent input, so that a reading which stops moving cannot outlive the undertaking it witnesses. Determinism forbids the wall clock; it does not license the gated party to wind the clock.

FOURTH. An overruled doctrine is repealed as a VERDICT and preserved as a RECORD. Where an acceptance regime is overruled, the direction and magnitude that carried each acceptance become the successor regime's redraw brief, and a repeal that deletes them destroys the evidence the successor runs on. The repeal reaches every instrument on the overruled ground at once, there being no lawful interval in which an overruled rule survives in an enacted instrument; but a repeal is not an erasure, and the instrument's duty on the day of the repeal is to convert, to become authority-aware, and to report where it may no longer block.

---

## Engine

| S012 ENACT_AMENDED (option 1) | S013 ENACT_AMENDED (option 1) | S014 ENACT_AMENDED (option 1) | S015 ENACT_AMENDED (option 2) | S016 ENACT_AMENDED (option 1) | S017 ENACT_AMENDED (option 1) |

JUDGE ENGINE, COURT OF APPEAL. On six enactment submissions of the visual-gap lane.

PRELIMINARY: WHAT I READ AND WHAT I VERIFIED

I read [2026] VJS-SC-OPBOX 1 in full at its ORDER and RATIO, and all six submissions in full. I then read the implementing code on feat/proof-kinds-visual-gap: prohibition.rs, burndown.rs, visual_review.rs, the S-7A(5) limb of geometry.rs, the S-5(9) limb of register_completeness.rs, the four new core types, ledger_staleness.rs, screen_parity.rs, config.rs, and .vds/enforcement.lock. I recomputed every case_file_digest with scripts/case-file-digest.sh; all six match the tree. I certify that I read the artefacts and did not certify from the submissions' descriptions of them, which matters here, because on two submissions the description and the artefact do not agree.

My remit is buildability and falsifiability. I ask four questions of each draft: can the kind fail, and is the failing direction seeded; is the rule expressible in terms the instrument can actually read; does the record carry every input its rules need and no field no rule reads; and can the kind be switched off by something other than the fact it measures. On that last question I found three real holes and one mis-description, and they drive most of what follows.

SUBMISSION-VDS-012, THE TWO-SIDED GEOMETRY BINDING. ENACT AS AMENDED, on option 1.

Option 3 is refused for the drafters' own reason, which is correct: staling only on the authority side rebuilds the 561-pin-561 instrument. Option 2 is refused because it leaves the commissioned defect class unmeasurable. Option 1 is right in shape and the staleness rules are right: R12 stales on both input hashes, R11 refuses an edited snapshot before anything is compared against it, and the fail-closed resolution in frame_authority (a frame with no measurable current digest is UNSIGNED, not signed) is the correct direction and is pinned by its own test.

Three defects in the artefact, none fatal to the shape.

First, the agreement bit is unpinned trust. The snapshot's rows are produced by an out-of-band comparator the engine neither names by digest nor pins. A comparator that writes agrees: true unconditionally produces a green two-sided binding forever, and nothing in this engine can tell. That is not a hypothetical objection to an out-of-band generator in general: the reading and the capture are both hash-witnessed precisely so that the engine does not have to trust them, and the one input that decides the verdict is the one input carrying no witness at all.

Second, AuthorityAgreement.because is documented as "Required by the proof when agrees is false" and is required by nothing. R13 interpolates it if present and proceeds if absent. A docstring promise is not behaviour, and a bare false bit names no work, which is the very thing S-7B(3) refuses two sections later.

Third, an absent snapshot is silent. The limb is entered behind `if let Some(snapshot)`, and the only vacuity note on the run covers bounds.is_empty(). A project with geometry bounds and no snapshot gets a fully green geometry run with nothing on its face saying the binding was never measured. Under SC-OPBOX 1 order 14, coverage owed must be reported, not inferred.

Fourth, and this is a buildability blocker for adopters rather than a design fault: the snapshot defaults to .vds/ledgers/geometry-authority.yaml, which is inside the configured ledgers directory. ledger_staleness enumerates every *.yaml under that directory and carries arms only for the screens, figma, frames, geometry-reading and ci ledgers; everything else falls to R2 and fails as "a ledger with no staleness test in this build". Enacting S-7A(5) as built therefore turns an enacted proof red, for a reason that is not a defect, in every project that generates a snapshot.

AMENDMENT 012 (enactable verbatim):

"S-7A(5) is amended by adding: The snapshot NAMES the comparator that produced its agreement bits (`comparator`, repository-relative) and that file's digest at comparison time (`comparatorDigest`). Where the project carries an enforcement lock (S-8), the named comparator must appear in that lock with a failing-direction test; a snapshot whose comparator is absent from a present lock carries no authority, and every row of it reads no_authority, coverage owed, never green. Where no lock is present the snapshot is read as it stands (S-8(3)). An agreement row whose `agrees` is false and whose `because` is empty is REFUSED (R14): a bare bit names no work, on S-7B(3)'s reasoning. Where no authority snapshot exists, the geometry run states on its face that the BOUND was measured and the BINDING was not, so that a green geometry run is never read as evidence that what shipped is what was decided."

"S-4(2) is amended by adding to the ledger_staleness registry: R7. The geometry authority snapshot (per `[geometry] authority_ledger`) is a ledger with a staleness test in this build. Its content digest is verified against its content, and the run states, as R5 does for the geometry reading, that self-consistency is settled and CURRENCY is not."

SUBMISSION-VDS-013, PROHIBITION. ENACT AS AMENDED, on option 1.

Option 3 is refused, and the submission refuses it for the right reason: a regex's failure modes are the silent-scope defects the kind exists to prevent. Option 2 is refused; a bespoke grep per repo re-invents the recorded expansion or omits it.

This is the strongest of the six as built. R3 is ordered before R1 so a narrowed scope fails even where every remaining file is clean; W1 scans the grown population rather than deferring it, so growth cannot open an unenforced shadow; R4 and R5 refuse the two shapes that cannot fail; findings name file and line. Eight tests, including the narrowing seed and both vacuity refusals. I would enact this unamended but for one gap.

The gap is per-row falsifiability. The kind carries a seed-red; a ROW does not. A record whose pattern is misspelt (`rounded_` for `rounded-`) passes green over a thousand files forever, and reads identically to a directive that has been met. SC-OPBOX 1 order 2 attaches primacy "only upon demonstrated falsifiability... by negative control against a seeded violation on the repository concerned", which is a per-repository test, and a row that never matched anything on that repository has had no negative control at all. The front door already expands the globs, so counting matches at registration is free.

AMENDMENT 013 (enactable verbatim):

"S-7B is amended by adding: S-7B(5). The record carries `sitesAtRegistration`: the number of matching lines across the recorded expansion at the moment the record was written, counted by the front door and never by hand. Where `sitesAtRegistration` is zero the record must carry `prophylactic: true` and a reason in `because`; a record claiming neither is REFUSED (R6). A pattern that matched nothing in its own scope at registration is indistinguishable from a misspelt one, and a misspelt pattern is a check that cannot fail on the repository concerned ([2026] VJS-SC-OPBOX 1, order 2). Every run reports, per row, `sitesAtRegistration` against the surviving count, so a prohibition's progress is a number somebody can drive to zero and not a pass or fail word."

SUBMISSION-VDS-014, BURNDOWN. ENACT AS AMENDED, on option 1.

Option 3 is refused; a plain ratchet is the instrument S-7A(2) already refused. Option 2 is refused on consolidation. Option 1's distinguishing clause, red on an unre-pinned decrease, is right and is correctly implemented, and the finding names the one-command cure, which is the correct treatment of good news mis-filed.

Two defects and one wording fault.

The reading does not witness what it measured. BurndownReading carries takenAt, generatedBy, rows and a digest over itself. R6 catches a HAND-EDITED reading. Nothing catches an UNREGENERATED one. A reading taken once, at pin 60, keeps every row green while the metric climbs to 100, because the pin equals the reading and the reading equals its own digest. That is precisely the 561-pin-561 instrument, inside the kind commissioned to retire it. The cure exists elsewhere in this same lane: visual_review binds its verdict to the route's source digest and stales when it moves.

The deadline can be suspended by declining to act. S-7C(3) measures the deadline against the reading's takenAt, which is right as against the wall clock (S-7(2)(1)) and wrong as a sole clock: a project that stops regenerating the reading never reaches its own deadline. The lane already has a determinate clock that is gated: register_completeness R2 uses the screens ledger's generatedAt, and that ledger is the one artefact carrying a REGENERATING staleness test.

The statute says "a decrease not re-pinned in the same change is red". No proof can see a change boundary. What is built is: any reading below the pin is red until the pin is lowered to it. The words and the instrument must agree or two readers will read the clause differently.

And the same ledger_staleness R2 collision as 012: the reading defaults to .vds/ledgers/burndown.yaml, inside the ledgers directory, with no arm.

AMENDMENT 014 (enactable verbatim):

"S-7C is amended by adding: S-7C(5). The reading witnesses WHAT it measured and not only its own bytes. It carries `inputs` (the globs the generator read) and `sourceDigest` (the digest of those files at generation). R9: where the current digest of `inputs` differs from `sourceDigest`, the reading is STALE, NO row is enforced against it, and the finding names the metric and the command that regenerates it. A reading nobody regenerated reports the estate as it WAS, and a pin compared against it is a green that means nothing moved when everything did."

"S-7C(3) is amended to read: A deadline, where directed, is measured against the LATER of the reading's `takenAt` and the screens ledger's `generatedAt`, and never the wall clock (S-7(2)(1)), so that declining to regenerate the reading cannot suspend a deadline. Past it, a non-zero metric is red until zero or until the undertaking is renegotiated as a NEW record with the reason on it."

"S-7C(2) is amended by striking the words 'in the same change'. The rule as measured is: any reading above the pin is red, and any reading below the pin is red until the pin is lowered to it."

"S-4(2) is amended by adding to the ledger_staleness registry: R8. The burndown reading (per `[burndown] reading_ledger`) is a ledger with a staleness test in this build: its content digest is verified against its content and its `sourceDigest` against its `inputs`. Without this arm the enacted `ledger_staleness` proof fails every project that adopts S-7C, under R2, for a reason that is not a defect."

SUBMISSION-VDS-015, MEASUREMENT COVERAGE. ENACT AS AMENDED, on OPTION 2, not the drafters' option 1.

I depart from the draft's default and adopt option 2, a sixteenth kind, for a reason the drafters supply themselves three sections earlier: green must mean exactly one thing. register_completeness's own SCOPE_NOTE says it "establishes EXISTENCE and nothing else". A kind whose pass asserts two unrelated propositions is not merely untidy; it is switchable off by the preconditions of whichever proposition it is not measuring, and that is not speculative here. register_completeness opens with `load_fresh(project)?` and `RegisterIndex::build(&ctx.store())?`, both of which abort the run as a precondition failure before the drafted R2/R3 block is ever reached. Both have fired on the subscriber estate: a stale screens ledger and two register records claiming one code coordinate each produced PRECONDITION FAILED with rows_enforced = 0. Neither has anything to do with measurement coverage, and either would carry this clause off the field silently, on the very estate the clause was written for. That settles the housing question.

Two further defects, which travel with the clause into whichever home it takes.

R2 is discharged by any non-empty string. `measuredBy: ["we check this"]` satisfies it, and R3's doc-like heuristic (.md, docs/, plans/, readme) does not catch prose that omits a file extension. A rule measured by a word is measured by prose, which is exactly what R3 exists to refuse.

The clause is opt-in by the party it measures. Activation is `directedAt.is_some() || !measuredBy.is_empty()`; a record carrying neither is outside the clause entirely. The submission presents this as the fail-closed interim, and as an interim it is correct. As a permanent rule it is a check the defect can decline: the row class that shipped structurally-green pages is precisely the row class whose author does not fill in directedAt.

AMENDMENT 015 (enactable verbatim):

"S-5(9) is enacted as a SIXTEENTH proof kind, `measurement_coverage`, and not as rules of `register_completeness`. Reason: register_completeness establishes existence and nothing else, and a proof whose green means two things is switched off by the preconditions of whichever thing it is not measuring. register_completeness refuses to run on a stale screens ledger and on two records claiming one code coordinate, both measured on the subscriber estate, and neither bears on measurement coverage."

"`measurement_coverage` reads the register RECORDS and does not build the code-coordinate index, so a coordinate collision cannot switch it off. Its clock is the LATER of the screens ledger's `generatedAt` and the newest `directedAt` in the register, never the wall clock; the run states which clock it used and states plainly that a clock nobody advances delays R2 rather than firing it."

"R4: each `measuredBy` entry must RESOLVE, to a path that exists in the repository, to a proof kind in the S-7(5) registry, or to a path pinned in the enforcement lock. An entry resolving to nothing is refused. Without resolution, R2 is discharged by typing a word."

"R5: every run reports the CENSUS of register records in an enforceable status carrying neither `directedAt` nor `measuredBy`, naming them, and states that this kind is not evidence of measurement coverage while that count is non-zero. A clause a record joins by carrying an optional field is a clause the defect it was written for can decline."

SUBMISSION-VDS-016, VISUAL REVIEW. ENACT AS AMENDED, on option 1.

Option 3 is refused on S-7(2)(1), as the submission concedes. Option 2 is refused: it re-invents the stale-verdict defect per repo, which is this lane's founding failure. Option 1's core is sound and, in the three-hash staleness rule, better than anything else in the lane: R2, R3 and the authority resolution each END the verdict rather than degrading it, R4 refuses green-against-nothing, R6 refuses a verdict that disagrees with its own evidence, and the closed three-state enum is pinned by a test that asserts "accepted" does not parse.

Four defects.

The delta is untyped prose, and that is not a cosmetic point: it makes SC-OPBOX 1 orders 18 and 19 UNEXECUTABLE. Those orders give deviation-by-addition a materially different disposition from every other deviation, and the engine cannot tell the two apart, because deltas is a Vec<String>. As built, every deviation against a signed frame is a blocking red, which is a stronger automatic consequence than order 18 permits ("its sole automatic consequence is the creation of a redraw proposal") and than ratio Q4 permits ("a docket entry, never an execution order").

shippedScreenshotDigest and frameImageDigest are read by no rule. They appear only in message text. [2026] VJS-CC-VIBE-DESIGN-SYSTEM 7 holds that a recorded field needs a proof limb, and SUBMISSION-VDS-014 cites that authority against itself and satisfies it; this submission does not.

R7 is stated but not completed. Supersession is resolved by comparing reviewed_at strings; two live records for one route bearing the SAME timestamp are both enforced, and the rule that says "two live verdicts for one subject is two answers" never fires. Burndown R7 refuses the analogous duplicate outright and is the model.

Nothing counts a signed frame with no live review. No findings and nothing looked print the same.

AMENDMENT 016 (enactable verbatim):

"S-7D(6) is amended: a delta is a TYPED record and not a line of prose, `{ kind: addition | omission | difference, what: <string> }`. Without a direction on the delta the engine cannot execute [2026] VJS-SC-OPBOX 1 orders 18 and 19, which give deviation-by-addition a different disposition from every other deviation."

"R5 is split. A deviation ALL of whose deltas are `addition` REPORTS and does not block, per SC-OPBOX 1 order 18: the run fails only where no redraw record in `proposed`, `drawn` or `signed` covers that review, and the addition remains live, no instrument removing or hiding it. A deviation carrying any `omission` or `difference` delta against a signed frame is FATAL as drafted. The amendment's failing direction is a review recording an addition delta with no covering redraw record, and it shall be seeded red before the gate is re-pinned (S-8(2))."

"R9: where a review names `shippedScreenshotPath` or `frameImagePath`, the artefact must exist and digest to the recorded hash; W3 where neither is named, stating that the verdict's evidence cannot be retrieved."

"R7 is completed: two live review records for one route bearing the same `reviewedAt` are FATAL, on burndown R7's reasoning, because nothing says which governs."

"W4: every route carrying a signed frame and no live review is NAMED as coverage owed."

SUBMISSION-VDS-017, THE REPEAL. ENACT AS AMENDED, on OPTION 1.

The bench is asked which option better executes SC-OPBOX 1 order 17. Option 1, and I hold it is not close, but the reason differs from the one the submission gives, because the submission mis-describes the estate.

Order 13 makes conform, deviate and no_authority exhaustive "for every enforcement purpose". Order 17 overrules the doctrine, not the doctrine-as-applied-to-new-kinds. A doctrine repealed for some instruments and retained for others is one rule and a disagreement, and this jurisdiction has already paid for that shape once. Option 2 is a standing invitation to cite whichever regime suits.

Now the correction, and it goes to what I was appointed to test. Option 1's stated consequence is that "screen_parity's acceptance surface is removed in a follow-up amendment", and option 2's is that "screen_parity keeps its enacted acceptance behaviour". I read screen_parity.rs and screen.rs. There is no acceptance surface. The kind's rules R1 to R7 and W1 measure the RECORD's required arrangement against the FRAME; there is no acceptance state, no excusal, no richness state, and the strings "accept", "excus", "richness" and "surplus" appear nowhere in the kind. screen_parity does not implement the richness doctrine in either direction, because it never reads the shipped page against the frame at all. The acceptance doctrine and the fifteen accepted-over rows live in the SUBSCRIBER's repo-local instruments and in the caselaw, not in this engine.

So in this engine, option 1 costs nothing and owes no follow-up amendment, and the choice between the options is very nearly a choice between two descriptions of the same no-op. That must be recorded, because a court should not enact a proposition premised on an artefact that does not exist, and a submission that describes an acceptance surface it did not open is the same class of fault as a check that certifies without reading. The real work of the repeal falls on the subscriber, and under SC-OPBOX 1 orders 1 and 5 the subscriber's local instrument is a convergent implementation of the register's rule, not an independent authority, so it is reached by the repeal without any further amendment to this engine.

What happens to that acceptance surface either way is then the operative question, and the answer is fixed by SC-OPBOX 1 order 26: the fifteen accepted-over rows convert to redraw proposals ON PAPER, carry their original acceptance's direction and magnitude as the redraw brief, retain render rights, and no gate may report them as deviations pending disposition. A repeal executed by deleting the state would turn fifteen accepted rows into fifteen deviations overnight, which order 26 forbids in terms. The repeal is therefore a MIGRATION, and the interim the submission describes (a closed enum in which "accepted" does not parse, an empty sign-off register, everything no_authority) is exactly right and satisfies order 23's condition precedent by construction.

AMENDMENT 017 (enactable verbatim):

"Option 1 is adopted. The acceptance doctrine is REPEALED for every frame-bound instrument, new and enacted, wherever the state is expressed, including repo-local convergent instruments in subscriber projects ([2026] VJS-SC-OPBOX 1, orders 1, 5, 13 and 17)."

"Option 1's stated consequence is CORRECTED on the record: `screen_parity` as enacted in this engine carries no acceptance surface. Its rules measure the record's required arrangement against the frame and contain no acceptance, excusal or richness state. No follow-up amendment to `screen_parity` is owed and no enacted behaviour in this engine changes on enactment."

"The repeal is executed as a MIGRATION and never as a deletion. Each existing acceptance row converts to a redraw record in `proposed`, carrying the original acceptance's direction and magnitude as its brief (SC-OPBOX 1 order 26; VJS-CC-OPBOX 155 O2). Until a covering sign-off or an express registered direction disposes of it, no instrument may score a converted row `deviate`, and the row retains its render rights."

"The acceptance state is closed PROSPECTIVELY at the parser: from enactment no instrument may construct an acceptance row. Existing rows remain readable for exactly one purpose, their conversion under the preceding clause, and the count of unconverted rows is a `burndown` metric under S-7C, pinned at its measurement, so the migration is a falling number and not a promise."

"S-7D(1) to (5) are enacted as drafted, subject to AMENDMENT 016."

GENERAL ORDER, binding on all six

"No warrant may cite any kind enacted or amended by this judgment until that kind has been shown to fail by NEGATIVE CONTROL against a seeded violation ON THE SUBSCRIBER REPOSITORY, and not only in the engine's synthetic harness ([2026] VJS-SC-OPBOX 1, order 2). Every failing-direction test now pinned in .vds/enforcement.lock runs against `Harness`, a fabricated tree; that discharges S-8(2) and does not discharge order 2, which attaches primacy per kind and per repository. Each such seeded run shall be recorded, and each retired repo-local gate's incident shall migrate into the register as the covering proof's seed fixture (order 5)."

RATIO

A drafted proof kind is enactable only where three things are true of the ARTEFACT and not merely of the rule. First, the record carries every input its rules read and no field that no rule reads: a fact the rule needs and the record does not hold makes a binding order unexecutable, and a field nothing reads is a field that rots. Second, the reading the kind depends on is bound by hash to the SUBJECT it measured and not only to its own bytes, so that a reading nobody regenerated expires visibly instead of reporting the past as the present. Third, the kind's activation is not in the hands of the party it measures: a clause a record joins by carrying an optional field is a check the defect can decline, and a clock only the subject advances is a deadline the subject can suspend. Falsifiability is proved per kind by a seed-red, per row by a registration witness, and per repository by negative control before any warrant cites it. And a kind that adds a second proposition to an existing kind's green must be its own kind, because a proof whose pass means two things is switched off by the preconditions of whichever thing it is not measuring.

---

## Estate

| S012 ENACT_AMENDED (option 1) | S013 ENACT (option 1) | S014 ENACT (option 1) | S015 ENACT_AMENDED (option 1) | S016 ENACT_AMENDED (option 1) | S017 ENACT_AMENDED (option 2) |

IN THE COURT OF APPEAL - JUDGE ESTATE
On six enactment submissions of the Vibe Design System, tested against [2026] VJS-SC-OPBOX 1 (The Design Constitution)

PRELIMINARY: WHAT I MEASURED

I read the Council's Order and Ratio in full, the six drafts in full, and the implementing code on branch feat/proof-kinds-visual-gap. I type-checked the workspace (cargo check --workspace --all-targets, clean, 6.69s), so every draft before me is buildable as written. I then went to the subscriber and measured the morning after rather than imagining it. Four facts govern most of what follows.

(1) Every new limb is VACUOUS in opbox-frontend on day one. There is no frames ledger (/home/jellytot/Projects/opbox-prod/opbox-frontend/.vds/ledgers holds only POINTERS.md, geometry.yaml, screens.yaml), no signoffs directory, no geometry authority snapshot, no review records, no prohibition and no burndown. All 114 register records parse with directedAt null and measuredBy empty. So enacting all six changes nothing that runs tomorrow. That is precisely what Council orders 20, 23 and 24 require, and it is the strongest single argument for enacting rather than deferring: the queue becomes visible before it is binding.

(2) The condition precedent is UNMET AND CURRENTLY UNMEETABLE. Order 23 makes the sign-off registry a condition precedent to enforcement of Q2-Q4. The front door, crates/vds-cli/src/signoff.rs:63, requires a frames ledger and refuses without one; opbox has none. Worse, order 31 directs that the four directions of 2026-08-01 (LOG ids 103751, 103951, 104325, 104739) be backfilled as the registry's FOUNDING entries, and a direction has no fileKey/nodeId, so it cannot pass through that front door at all. The registry as drafted cannot execute an order of the Council. That is the single largest defect across the six and it is why S-017 cannot be enacted unamended.

(3) The acceptance surface S-017 proposes to repeal DOES NOT EXIST IN THIS ENGINE. I grepped screen_parity.rs, ScreenRecord and ArrangementContract: there is no accepted state, no excusal, nothing. The acceptance surface is in the SUBSCRIBER, at /home/jellytot/Projects/opbox-prod/opbox-frontend/scripts/frame-code-parity.py, whose baseline carries an `accepted` map with a reason per row. That is where the 15 accepted-over rows live.

(4) Report-only is NOT honoured by S-016 as written. In crates/vds-proof/src/visual_review.rs the authority resolution happens at line ~260, AFTER R2 (shipped stale) and R3 (frame stale) have already been able to run fatal at lines ~213 and ~240. On day one the registry is empty, so every surface is no_authority, and a stale verdict on a no_authority surface would fail the gate. Order 24 says a surface flips from report to block only at the moment its frame is registered. Credit where due: the DEVIATE limb (R5) is correctly gated on FrameAuthority::Signed, and a deviate against an unsigned frame is skipped and warned. The draft got the hard half right and left the easy half wrong.

=====================================================================
SUBMISSION-VDS-012 - the two-sided geometry binding. ENACT_AMENDED, option 1.
=====================================================================

Option 3 is self-refuting and the drafter says so: staling only on the authority side reproduces 561-pin-561 with a hash on it. Option 2 forfeits the only instrument in the estate that can say "what shipped is what was decided", which is the exact defect this lane was commissioned against. Option 1 it is.

The seam is better built than I expected. `vds ledger geometry-authority --from <json>` (crates/vds-cli/src/ledger.rs:284) refuses a snapshot born stale on BOTH input hashes before it will write, and computes contentDigest itself, so my initial concern about a hand-authored self-digesting file is answered. R11 refuses an edited snapshot; R12 expires the binding visibly on either side; R13 refuses an agreement CLAIM against an unsigned frame and warns no_authority otherwise. That last is order 14 and order 16 implemented, not merely recited.

But there is a third input and it is the only one the engine does not hash. The `agrees` boolean is produced by an out-of-band comparator, and the engine cannot re-derive it (re-deriving needs the values, which S-2(2) and [2026] VJS-CC-OPBOX 3 forbid it to hold). GeometryAuthority carries `generated_by: String` - a NAME. A comparator rewritten to return `agrees: true` unconditionally produces an identical name, regenerates cleanly through the front door, satisfies R11 and R12, and turns the whole two-sided limb green with nothing measured. That is S-015's own disease - a measure that points at prose - contracted by S-012's measure. And under Council order 2 primacy attaches per proof kind only upon DEMONSTRATED FALSIFIABILITY; a limb whose central assertion the engine cannot falsify cannot carry primacy over any of opbox's 28 gates.

AMENDMENT A1 (enactable verbatim):

"In `GeometryAuthority` insert two required fields: `comparator`, the repository-relative path of the out-of-band program that produced the agreement rows, and `comparatorDigest`, that file's content digest at the moment the comparison ran. Both enter `computeContentDigest`. `vds ledger geometry-authority` shall refuse a snapshot whose `comparator` cannot be read, or whose `comparatorDigest` differs from the file on disk, in the same terms in which it already refuses a stale capture or a stale reading. Add to the geometry proof:

R14 - the COMPARATOR side is stale: the file named by `comparator` cannot be read, or no longer digests to `comparatorDigest`. Fatal, no row enforced, reported alongside R12's two sides. The agreement bits are the comparator's assertion and the engine cannot re-derive them; a comparator that moved after the comparison is a third expired input, and a snapshot that outlives it is an agreement measured by a program that no longer exists.

W3 - the file named by `comparator` does not appear in the project's enforcement lock carrying a `failingDirectionTest`. Warned on every run and recited on the run's record: until it is pinned with a seed that makes it run red, `agrees` is an assertion the engine cannot falsify, and no warrant may claim primacy for this limb over a repo-local gate on the same ground ([2026] VJS-SC-OPBOX 1 orders 2 and 3)."

Further, and this is a narrow point that applies equally to S-015: geometry.rs is ALREADY a blocking gate in .vds/enforcement.lock, and its single `failingDirectionTest` slot pins `geometry_fails_when_the_bound_only_ever_held` - the R3 seed, not an R11-R13 seed. The relock rationale asserts "every new rule carries a seed-red test" and unit tests do exist in source, but the lock records only one. I do not make that a bar to enactment, because no gate is being retired today. I make it a bar to retirement: see the ratio.

=====================================================================
SUBMISSION-VDS-013 - the prohibition kind. ENACT, option 1, unamended.
=====================================================================

This is the best-built of the six and I would not touch it. Option 3 (regex) is correctly refused in the draft's own words: an unescaped dot silently widening a match IS the silent-scope defect the kind exists to refuse, and adopting the more expressive engine would have made the instrument's failure mode identical to its subject. Option 2 leaves the estate with per-repo greps, each re-inventing scope recording or omitting it.

The design decision that earns enactment is the RECORDED EXPANSION. R3 makes a narrowed scope fatal, so a file renamed out of the glob taking its violations with it becomes a finding instead of a disappearance; W1 makes growth a warning while still scanning the new files under R1, so growth cannot create an unenforced shadow. R4 and R5 refuse the vacuous prohibition and do not enforce the row, which is [2026] VJS-CC-OPBOX 3 D3 applied to itself. R1 names every surviving site as file:line, capped at twelve NAMED and all COUNTED - a named site is a job, a count is a mood.

Day-after note, obiter and not a condition. Opbox task #112 (de-carding and de-dotfield) is in progress and is exactly the directive class this kind is for. Registering prohibitions over a scope that is actively being flattened will trip R3 routinely, because deleting a carded file removes it from the recorded expansion. That is correct behaviour and the cure is named in the finding text ("Deliberate removals re-record the expansion through the front door"), and `vds prohibition re-expand` exists at crates/vds-cli/src/prohibition.rs:122. A gate that names its own cure is a gate people will use rather than bypass. Council order 24 does not reach this kind: a prohibition is a directive proof, not a frame-bound deviation proof, and nothing in Q2-Q4 makes registry sign-off a precondition to asserting that a pattern is absent from a place.

=====================================================================
SUBMISSION-VDS-014 - the burndown kind. ENACT, option 1, unamended.
=====================================================================

Option 3 - a plain ratchet without the unre-pinned-decrease rule - is the instrument the geometry amendment already refused and the Council's own Q1 reasoning refuses again: a number that may only be held is a record of a defect presented as a control, and it went 667 to 561 and then stopped for good. Option 2 leaves N bespoke scripts drifting, against the standing consolidation steer.

Two things earn clean enactment. First, R1 and R2 together: red on any increase AND on a decrease not re-pinned in the same change, so the pin sits ON the truth and there is no invisible headroom below it. Second, and more important for this estate, R3's deadline is measured against `reading.taken_at`, never the system clock (crates/vds-proof/src/burndown.rs:257-259). That is not fastidiousness. The obvious first burndown in opbox is the 2026-09-15 parity fence over the 56 parked frames, preserved unchanged by Council order 27. A fence that read the wall clock would produce different findings from identical inputs, would go red overnight with nobody having changed anything, and would be a check people re-run until green. Measured against the reading, re-running against yesterday's reading gives yesterday's answer and `ledger_staleness` is the kind that says the reading is old. That is the correct division of labour and it is already built.

I checked the finding texts for removal language, because order 27 holds that after the fence the gate blocks builds, merges and conform status but never unrenders. R3 says the cure is that "the undertaking is met, or the deadline is re-negotiated as a NEW record with the reason on it". No removal is suggested and the engine has no removal power. Compliant.

=====================================================================
SUBMISSION-VDS-015 - register measurement coverage. ENACT_AMENDED, option 1.
=====================================================================

I adopt option 1, housing the clause in register_completeness rather than in a sixteenth kind. Option 2 is tidier on paper and I considered it seriously, but a kind with two rules and no independent subject would count the same register rows a second time, and this proof family has already been burned by a `rows_enforced` figure that double-counted (screen_parity W1 is deliberately NOT a row for exactly this reason). Widening register_completeness's scope note openly, which the draft already does, is the honest move: the note is the instrument's own statement of reach, and reach is what this whole submission is about. The docs-bind-nothing extension in R3 is right and I would go further than the draft's diffidence about it: a measure that points at a plan document is a measure by prose, and prose is not enforcement whether it is cited by a warrant or by a measure.

The grace mechanism is well built - measured against `ledger.generated_at`, not the clock, same discipline as S-014 - and the serde defaults mean nothing reddens retroactively. I verified that claim rather than accepting it: all 114 opbox register records carry directedAt null and measuredBy empty, so R2 and R3 are inert on day one. Zero exposure. The fail_closed_interim is accurate.

And that is the problem. R2 fires only on records that carry `directedAt`. The population S-015 was written about - "rule rows registered with nothing measuring them stayed green forever" - is 114 records, and every one of them is outside the clause, permanently, unless someone volunteers to arm it. A rule that fires only on the rows that opted in has a denominator, and this one does not report it. That is the shape of a toggle for behaviour that does not exist: the clause will run green forever over the exact rows it was drafted against, and the run will look identical to a run over a fully-measured register.

I do not cure this by making it retroactively fatal. There is no direction date to measure grace from, and a clause that reddened 114 records on enactment morning would be a flag day of the kind order 20 forbids in the neighbouring regime and prudence forbids here. I cure it by making the reach visible.

AMENDMENT A2 (enactable verbatim):

"Add to `register_completeness`, as an informational class counted and named on every run:

I4 - an enforceable register record carrying neither `measuredBy` nor `directedAt`. Such a record is outside draft S-5(9) R2 entirely. The run shall print the count of these records, and shall name their ids where there are twelve or fewer.

And add to `BOUND_NOTE`: 'R2 reaches only records that carry `directedAt`. A record naming no direction and no measure is outside this clause, and the count of such records is printed on every run: a rule that fires only on the rows that armed it has a denominator, and an unreported denominator is the defect this clause was written against.'"

=====================================================================
SUBMISSION-VDS-016 - the visual_review kind. ENACT_AMENDED, option 1.
=====================================================================

Option 3 is refused on the statute's face (S-7(2)(1), network and model, both limbs). Option 2 re-invents the stale-verdict defect per repo, which is this lane's founding failure repeated. Option 1, amended.

What the draft gets right is not small. The verdict is a RECORD and the pipeline stays outside, so the proof reads no network and calls no model. Three staleness sides are hashed: shipped source, frame content, and authority. R4 refuses conform against an unsigned frame, which is the one combination that could smuggle taste back downstream - green against nothing - and it is refused at validation rather than left to judgment. R7 makes the newest record per route govern structurally. R8 refuses the word "signed" without a sign-off row whose hash is the frame's CURRENT hash. And, most creditably, R5 fires only against FrameAuthority::Signed: a deviate verdict on an unsigned frame is skipped and warned as coverage owed. That is Council orders 14, 16 and 20 implemented in code, not recited in a comment.

Two defects.

FIRST, and it is a breach of order 24 as the code stands: R2 and R3 fire fatal BEFORE the authority is resolved. On enactment morning the registry is empty, so every surface in the estate is no_authority; the day the contact sheet lands (opbox task #84, in progress) it will produce review records; and the first time a route's source or frame moves after review, the gate goes red on a surface the Council has held nothing may block on. Note that pre-registration these two rules protect nothing: their whole purpose is to stop a stale GREEN, and no green is available pre-registration because R4 refuses conform against an unsigned frame. So fatal R2/R3 before registration is pure blocking cost at zero protective value.

SECOND, the finding text. Order 18 holds that a deviation-by-addition is remedially inert as to removal and its sole automatic consequence is a redraw proposal; order 21 makes 155 O7 the exclusive law of removal. The engine has no removal power, so the orders are satisfied by construction - but the finding is the interface an agent acts on, and R5 currently reads "an addition the frame omits is a deviation exactly like a missing element", which to an agent clearing a queue reads as licence to delete the addition. The severance must be on the face of the finding, not merely true of the engine.

AMENDMENT A3 (enactable verbatim):

"In `visual_review`, resolve the frame's authority BEFORE rules R2 and R3. Where the resolved authority is `Unsigned`, R2 and R3 shall warn and skip the row (`no_authority_verdict_stale`) rather than fail it, reciting that the verdict is stale and the surface carries no authority, so the staleness is coverage owed and not a breach. R1 (unknown route) and R6 (incoherent record) remain fatal in every authority state, being defects in the record, curable by the recorder, and not facts about any surface.

Add to `RESERVED_NOTE`: 'Before a surface's frame is entered in the sign-off register this kind reports and never blocks ([2026] VJS-SC-OPBOX 1 orders 20, 23 and 24). Registration is the moment a surface flips from report to block, and there is no estate-wide flag day.'

Add to the R5 finding, after "there is no engine-side excusal": 'This finding CLASSIFIES and does not dispose. It is a docket entry and never an execution order: removal of a live surface is governed exclusively by [2026] VJS-CC-OPBOX 155 O7, unimpaired ([2026] VJS-SC-OPBOX 1 order 21), and nothing in this finding licenses removing, hiding or unrendering the addition it names.'"

=====================================================================
SUBMISSION-VDS-017 - sign-off registry, three states, redraw, and the repeal. ENACT_AMENDED, option 2.
=====================================================================

OPTION 2, AND WHY. The Council overruled the richness doctrine PROSPECTIVELY (order 17). Prospective overruling means the new rule binds going forward and does not reach back to disturb what was already disposed under the old one, and order 26 then says so expressly: the 15 accepted-over rows convert to redraw proposals ON PAPER ONLY, carry their original 155 O2 direction and magnitude as the redraw brief, retain render rights, and NO GATE MAY REPORT THEM AS DEVIATIONS pending disposition by covering sign-off. Option 1 would repeal "everywhere including enacted screen_parity", with screen_parity's acceptance surface "removed in a follow-up amendment" - an unenacted, unspecified future change that would flip a whole population from excused to reported in one step. That is an estate-wide flag day wearing a follow-up amendment's clothes, and order 20 forbids it. Option 3 is, as the drafter honestly says, a referral of the Principal's direction back to the Principal, and I will not make the bench do that when the Council has already ruled on the substance.

WHAT HAPPENS TO SCREEN_PARITY'S ACCEPTANCE SURFACE, EITHER WAY. Nothing, because it does not have one. I searched crates/vds-proof/src/screen_parity.rs, crates/vds-core/src/types/screen.rs (ScreenRecord, ArrangementContract, FigmaFrame): there is no acceptance state, no excusal concept, no accepted variant anywhere in the enacted engine. The acceptance surface is a SUBSCRIBER artefact: the `accepted` map in the baseline read by /home/jellytot/Projects/opbox-prod/opbox-frontend/scripts/frame-code-parity.py, which records a superseded target as an excused code with a reason under 155.

This makes option 1 worse than merely premature. It is drafted against an object that does not exist, and it has only two possible readings, both bad. Read as an engine change it is a repeal with no object - a purported repeal of behaviour that is not there, which is the reverse of the toggle-for-behaviour-that-does-not-exist defect and would report as accomplished while accomplishing nothing. Read as reaching into the subscriber, it is an order to strip `accepted` out of frame-code-parity.py, which would immediately convert the 15 rows into reported deviations in direct breach of order 26, and would in any event be ultra vires: a VDS statute governs the VDS engine and its register, not a subscriber's bespoke Python gate. Council order 5 points the other way in any case - a stricter local gate is a register defect to be merged UPWARD, never disabled downward - and here the local gate is not even stricter; it is the custodian of the 15 rows' render rights.

Under option 2 the correct outcome follows with no engine change at all: the new frame-bound kinds carry no acceptance state (AuthorityVerdict is closed at three and "accepted" does not deserialise - there is a test for it at signoff.rs:275), screen_parity's behaviour is untouched because there is nothing to touch, and the 15 rows convert to RedrawRecords with their 155 O2 direction and magnitude carried in `proposed` and `basis` while frame-code-parity.py continues to hold them as accepted until a covering sign-off disposes of each. Two regimes coexist visibly, each citing its instrument, which is what order 26 describes.

Now the two amendments, and they are conditions of enactment rather than improvements.

DEFECT ONE: the signing front door will sign a frame that disclaims itself. Order 25 could not be plainer - only frames labelled CURRENT SOURCE are registrable; LEGACY/REFERENCE, TARGET/proposal and self-disclaiming frames are no_authority per se and eligible only after redraw. `vds signoff record` (crates/vds-cli/src/signoff.rs:63) checks that a frames ledger exists, that the file key matches, that the node is present, and that the row has a content digest. It never looks at the label - and the ledger already carries what it needs, `disclaimed: bool` and `authority_by: AuthorityBy` (crates/vds-figma/src/frames.rs:112, 148). Opbox has 18 self-disclaiming frames (task #81) and 56 parked ones. The registry is the condition precedent to the entire Q2-Q4 regime; a register that can be founded on a frame which says in its own authoritative layer that it is NOT SOURCE CURRENT is poisoned at genesis, and everything downstream inherits the poison.

DEFECT TWO, and the more serious: the registry cannot execute order 31. Council order 30 makes every Principal direction that disposes of a surface's conformance a sign-off act requiring hash-bound registry entry, and order 31 directs that the four directions of 2026-08-01 be backfilled as the registry's FOUNDING entries. A direction is not a frame: it has no fileKey, no nodeId and no frame content hash, and `SignOff` as typed is nothing but a frame content hash. Nothing in S-017 as drafted can record one. The consequence is order 29 becoming unrepresentable: the band is off-screen under registered direction LOG-2026-08-01-104739, and the Council held unanimously that "while the registered direction stands no gate may count the band a violation" - but there is no state in RedrawStatus that says so. `Withdrawn` is documented as "the deviation stands and stays red", which is the opposite. So the day the band's frame is signed, the engine has exactly one thing it can say about a surface the Council has held lawful: deviate.

Let me be clear about what I am NOT doing. I am not re-importing acceptance. Order 19(ii) makes an express registered direction a lawful RESOLUTION of a deviation, and order 15 makes a post-signature direction itself a sign-off act - so a direction is taste exercised AT the registry, hash-bound, by the only person entitled to exercise it, not taste exercised downstream by an engine. That is the whole distinction the constitutional direction turns on, and the drafts have simply not built the half of the registry that carries it.

AMENDMENT A4 (enactable verbatim):

"`vds signoff record` shall refuse, as a precondition, any frame whose frames-ledger row is `disclaimed`, whose `authorityBy` is `Unlabelled`, or whose authority layer is not a CURRENT SOURCE label under `[figma] authority`. The refusal shall name the label it found and direct the signer to resolve the label - redraw and re-capture - before signing. A registry entry over a frame that disclaims its own currency is a signed contradiction, and the register is the condition precedent to the whole regime ([2026] VJS-SC-OPBOX 1 orders 23 and 25); poisoning it at genesis poisons everything downstream of it. This refusal operates at the front door only: it creates no proof rule and reddens nothing already recorded."

AMENDMENT A5 (enactable verbatim):

"The sign-off register shall carry a second row kind, `DirectionRecord`, with fields: `id`; `logId`, the decision-log entry the direction was given in; `decisionDigest`, that entry's content digest when the direction was registered; `surface`, a route or a `fileKey`/`nodeId` pair; `direction` and `magnitude`, the [2026] VJS-CC-OPBOX 155 O2 form requirement preserved by [2026] VJS-SC-OPBOX 1 order 22; and `directedAt`. A `DirectionRecord` carries authority while, and only while, `decisionDigest` equals the log entry's current digest - staleness by hash, never by trust, on the same terms as a `SignOff`.

`RedrawStatus` shall grow a fifth variant, `Parked`, lawful ONLY where `resolvedBy` names a `DirectionRecord` whose `decisionDigest` still matches. `visual_review` R8 shall refuse `parked` without a covering direction row in the terms in which it already refuses `signed` without a covering sign-off; and where a live `Parked` redraw covers a surface, the frame-bound proofs shall SKIP that surface, reciting that while the registered direction stands no gate may count it a violation ([2026] VJS-SC-OPBOX 1 order 29). `RedrawStatus::Withdrawn` shall be documented as what it is - the proposal abandoned, the deviation standing - and shall not be used to record a direction.

This limb is part of what enactment of S-7D(1)-(5) means and is not severable from it: without it the register cannot record the four founding entries [2026] VJS-SC-OPBOX 1 order 31 directs, and a registry that cannot execute the order that founds it is not the condition precedent order 23 requires."

CONSEQUENTIAL, to the subscriber: on enactment, opbox-frontend's 15 accepted-over rows are to be opened as `RedrawRecord`s in status `proposed`, each carrying its original 155 O2 direction and magnitude in `proposed` and `basis`, per order 26. They stay in frame-code-parity.py's `accepted` map and continue to render until disposed by covering sign-off. Opening the redraw is the paper conversion order 26 describes; nothing about their runtime state changes, and W2 naming them as open redraws on every run is a warning, not a deviation report, so order 26's "no gate may report them as deviations" is honoured.

=====================================================================
RATIO
=====================================================================

An instrument's REACH is part of its result, and an unreported reach is a pass over an unknown denominator. Where a proof's verdict rests on an input the engine does not hash, or a rule fires only on the records that opted into it, or a class of subject sits outside the clause by construction, the proof must COUNT and NAME that input and that population on every run, in the same breath as the verdict. A green over a denominator nobody stated is indistinguishable from a green over the whole estate, and the difference between them is the only thing the reader wanted to know.

It follows, and I so hold, that under [2026] VJS-SC-OPBOX 1 order 2 the demonstrated falsifiability on which primacy depends must extend to EVERY input the verdict rests on, not merely to the rule under test. A limb whose central assertion is produced by a program the engine neither hashes nor pins is not falsifiable by the engine, however red its own unit test runs; and under order 3 no repo-local gate may be retired in reliance on such a limb until the seed for THAT limb - and for the comparator that feeds it - is recorded in the enforcement lock of the repository concerned, where the enforcement instrument reads it. A seed that lives only in a source comment is a negative control nothing performs.

I note for the record, as obiter, that geometry.rs and register_completeness.rs are already blocking gates whose enforcement lock carries a single `failingDirectionTest` slot, still pinning their original rules. Enacting S-012 and S-015 adds live limbs to gates whose recorded negative control does not cover them. That is not a bar to enactment today, because nothing is being retired today. It is an absolute bar to retiring any of opbox-frontend's 28 gates on the strength of those limbs, and the lock will need a second slot or a list before that day comes.


---

## THE ENACTMENT ORDER (clerk-assembled)

# IN THE COURT OF APPEAL OF THE VIBE DESIGN SYSTEM
## CLERK'S ENACTMENT ORDER, RATIO AND DISPOSITION
### On SUBMISSION-VDS-012 to SUBMISSION-VDS-017, tested against [2026] VJS-SC-OPBOX 1
### Bench: Fidelity J, Engine J, Estate J. Implementing branch: `feat/proof-kinds-visual-gap` at /home/jellytot/Projects/vibe-design-system

---

# PART I - THE ENACTMENT ORDER

## Section A - Rules of computation and general orders

**ORDER 1.** An amendment proposed in the opinions is CARRIED into this enactment only where two or more judges require the same substantive correction on the same ground and that correction is compatible with the option the majority adopted, and where those judges' formulations of one ground differ in breadth the NARROWEST formulation controls; every other proposed amendment is recorded in Schedule B, binds nothing, and is available to a future submission.

**ORDER 2.** (Unanimous; Fidelity 012-C, Engine General Order, Estate ratio.) No warrant may claim primacy for any proof kind enacted or amended by this judgment, and no repo-local gate may be retired in reliance on one, until that kind has been shown to run RED by negative control against a seeded violation of the specific incident class the gate was forged against, ON THE REPOSITORY CONCERNED and not only in the engine's synthetic `Harness`, with the seed recorded in that repository's `.vds/enforcement.lock` where the enforcement instrument reads it; until that demonstration both instruments run, the stricter reading enforces, and any disagreement between them is itself filed.

**ORDER 3.** (Administrative, giving effect to Order 2.) `.vds/enforcement.lock` shall carry a LIST of `failingDirectionTest` entries per gate rather than a single slot, `geometry.rs` and `register_completeness.rs` being blocking gates whose recorded negative control pins only their original rule and which acquire live limbs under Orders 5 to 8 and 14 to 16 that the present lock does not cover.

**ORDER 4.** No submission before the court is refused, a drafted text that under-states a binding qualification of [2026] VJS-SC-OPBOX 1 being cured by reading the qualification in rather than by refusal, since a statute that contradicts a binding qualification is a second copy of a rule and therefore one copy and a disagreement.

## Section B - SUBMISSION-VDS-012, the two-sided geometry binding (S-7A(5))

**ORDER 5.** SUBMISSION-VDS-012 is ENACTED AS AMENDED on OPTION 1 (unanimous), options 2 and 3 being refused on the ground that a binding stale only on the authority side reproduces the 561-pin-561 instrument and that abandoning the limb forfeits the only instrument in the estate that can say what shipped is what was decided.

**ORDER 6.** `GeometryAuthority` shall carry two further REQUIRED fields, `comparator` (the repository-relative path of the out-of-band program that produced the agreement rows) and `comparatorDigest` (that file's content digest at the moment the comparison ran), both entering `computeContentDigest`.

**ORDER 7.** `vds ledger geometry-authority` shall refuse to write a snapshot whose `comparator` cannot be read or whose `comparatorDigest` differs from the file on disk, in the same terms in which it already refuses a snapshot born stale on the capture or reading side.

**ORDER 8.** R14 is added to the geometry proof: the COMPARATOR side is stale where the file named by `comparator` cannot be read or no longer digests to `comparatorDigest`, and that condition is fatal, enforces no row, and is reported alongside R12's two sides, the agreement bits being the comparator's assertion which the engine cannot re-derive without holding values that S-2(2) and [2026] VJS-CC-OPBOX 3 forbid it to hold.

**ORDER 9.** W3 is added: where the file named by `comparator` does not appear in the project's enforcement lock carrying a `failingDirectionTest`, every run warns and recites on its record that `agrees` is an assertion the engine cannot falsify and that no warrant may claim primacy for this limb over a repo-local gate on the same ground ([2026] VJS-SC-OPBOX 1 orders 2 and 3), this warning form being carried in preference to the broader form that would void the snapshot's authority outright.

## Section C - SUBMISSION-VDS-013, the prohibition kind (S-7B)

**ORDER 10.** SUBMISSION-VDS-013 is ENACTED UNAMENDED on OPTION 1, two of three judges requiring no amendment so that the narrowest position commanding a majority is enactment as drafted, the recorded-expansion rule (R3 fatal on narrowing, W1 scanning growth rather than deferring it) and the vacuity refusals R4 and R5 standing as drafted, and the regex option being refused because an unescaped dot that silently widens a match is the very silent-scope defect the kind exists to refuse.

**ORDER 11.** Order 2 attaches to `prohibition` as to every other kind, and [2026] VJS-SC-OPBOX 1 order 24 does not reach it, a prohibition being a directive proof and not a frame-bound deviation proof, so that registry sign-off is no precondition to asserting that a pattern is absent from a place.

## Section D - SUBMISSION-VDS-014, the burndown kind (S-7C)

**ORDER 12.** SUBMISSION-VDS-014 is ENACTED AS AMENDED on OPTION 1 (unanimous on enactment; two of three requiring the amendment at Order 13), the plain-ratchet option being refused because a number that may only be held has invisible headroom below the pin, which is the 667-to-561-then-stop failure observed on the subscriber estate.

**ORDER 13.** S-7C(5) is inserted: a burndown record carrying a deadline MUST also declare a maximum reading age in days, and the proof is fatal where the reading's `taken_at` precedes, by more than that many days, the most recent `generatedAt` among the ledgers the run read, the clock never being the wall clock (S-7(2)(1)) but the run's own freshest independent input, so that a subject which stops regenerating the reading cannot thereby outlive the undertaking that reading witnesses; the witness is the one `register_completeness` already uses at `register_completeness.rs:331`, and the broader formulation that would re-base every deadline on the screens ledger's `generatedAt` is not carried.

**ORDER 14.** S-7C(3)'s measurement against `reading.taken_at` is otherwise UNDISTURBED, that discipline being what keeps the 2026-09-15 parity fence preserved by [2026] VJS-SC-OPBOX 1 order 27 from producing different findings from identical inputs.

## Section E - SUBMISSION-VDS-015, register measurement coverage (S-5(9))

**ORDER 15.** SUBMISSION-VDS-015 is ENACTED AS AMENDED on OPTION 1, housed in `register_completeness` and not as a sixteenth proof kind, two of three judges preferring the existing home on the ground that a two-rule kind over the same register rows buys a rot surface to keep a scope note tidy and would put two proofs over one artefact.

**ORDER 16.** R4 is added: every `measuredBy` entry must RESOLVE, to the name of a proof kind in the closed registry at S-7(5), to a repository-relative path that exists in the subject tree, or to a path pinned in the enforcement lock, and an entry resolving to nothing is refused, R3's refusal of document paths standing unamended alongside it, because without resolution R2 is discharged by typing a word and a rule measured by a word is measured by prose.

**ORDER 17.** I4 is added as an INFORMATIONAL class, counted and named on every run (ids named where there are twelve or fewer): an enforceable register record carrying neither `measuredBy` nor `directedAt`, such a record being outside S-5(9) R2 entirely and not a failure, and `BOUND_NOTE` shall recite that R2 reaches only records carrying `directedAt`, that a rule which fires only on the rows that armed it has a denominator, and that an unreported denominator is the defect this clause was written against; the narrower informational form is carried in preference to counting the class where the verdict is read.

**ORDER 18.** The scope note of `register_completeness` is amended to read "existence, and the measurement coverage of a directed record", and the widening is recorded both in this specification and in that proof's enforcement-lock entry rather than left standing in a scope note the proof has outgrown.

## Section F - SUBMISSION-VDS-016, the visual_review kind (S-7D(6))

**ORDER 19.** SUBMISSION-VDS-016 is ENACTED AS AMENDED on OPTION 1 (unanimous), the in-engine pipeline option being refused on S-7(2)(1) on both the network and model limbs and the per-repository option being refused as this lane's founding failure repeated.

**ORDER 20.** The frame's authority shall be resolved BEFORE rules R2 and R3, and where the resolved authority is `Unsigned` those rules shall WARN and SKIP the row (`no_authority_verdict_stale`) rather than fail it, reciting that the verdict is stale and the surface carries no authority so that the staleness is coverage owed and not a breach, while R1 (unknown route) and R6 (incoherent record) remain fatal in every authority state as defects in the record and not facts about any surface; this per-surface degrade is carried in preference to the broader course of deferring the kind's commencement entirely, and it satisfies that concern in substance because an empty register renders every surface `no_authority`.

**ORDER 21.** `RESERVED_NOTE` shall recite that before a surface's frame is entered in the sign-off register this kind reports and never blocks ([2026] VJS-SC-OPBOX 1 orders 20, 23 and 24), that registration is the moment a surface flips from report to block, and that there is no estate-wide flag day.

**ORDER 22.** The R5 finding shall state, after its existing text, that the finding CLASSIFIES and does not dispose, that it is a docket entry and never an execution order, that removal of a live surface is governed exclusively by [2026] VJS-CC-OPBOX 155 O7 unimpaired ([2026] VJS-SC-OPBOX 1 order 21), and that nothing in the finding licenses removing, hiding or unrendering the addition it names.

## Section G - SUBMISSION-VDS-017, sign-off register, authority states, redraw, and the repeal

**ORDER 23.** SUBMISSION-VDS-017 is ENACTED AS AMENDED on OPTION 1 by a majority of two to one, Estate J dissenting for option 2, on the ground that [2026] VJS-SC-OPBOX 1 order 17 is itself the ruling option 2 waits for and that a doctrine repealed for some instruments and retained for others is one rule and a disagreement.

**ORDER 24.** It is RECORDED ON THE RECORD, on the concurrent measurement of all three judges, that `screen_parity` as enacted carries NO acceptance surface (no acceptance state, no excusal, no richness state, its comparison at `screen_parity.rs:715` being a bare inequality that already scores a richer code side as a difference), and option 1's consequence clause directing that `screen_parity`'s acceptance surface be "removed in a follow-up amendment" is STRUCK as a direction reaching nothing.

**ORDER 25.** S-7D(4) is amended by striking "the resolution path is a new signed frame version, never an engine-side excusal" and substituting that a deviation resolves by one of exactly three routes, being (i) a covering sign-off adopting it, (ii) an express registered direction parking it under Order 26, or (iii) a deletion that independently discharges [2026] VJS-CC-OPBOX 155 O7, the drafted single route being route (i) alone and therefore contrary to order 19 on its face.

**ORDER 26.** The sign-off register shall carry a second row kind, `DirectionRecord`, with fields `id`, `logId`, `decisionDigest`, `surface` (a route, or a `fileKey`/`nodeId` pair), `direction` and `magnitude` in the [2026] VJS-CC-OPBOX 155 O2 form, and `directedAt`, carrying authority while and only while `decisionDigest` equals the log entry's current digest (staleness by hash, never by trust, on the same terms as a `SignOff`), a direction being taste exercised AT the register by the only person entitled to exercise it and not taste exercised downstream by an engine; the broader form adding a redraw-by date and a fatal redraw-duty limb is not carried.

**ORDER 27.** `RedrawStatus` shall grow a fifth variant, `Parked`, lawful ONLY where `resolvedBy` names a `DirectionRecord` whose `decisionDigest` still matches; `visual_review` R8 shall refuse `parked` without a covering direction row in the terms in which it already refuses `signed` without a covering sign-off; where a live `Parked` redraw covers a surface the frame-bound proofs shall SKIP that surface reciting that while the registered direction stands no gate may count it a violation ([2026] VJS-SC-OPBOX 1 order 29); and `Withdrawn` shall be documented as what it is, the proposal abandoned and the deviation standing, and shall never be used to record a direction.

**ORDER 28.** The repeal is executed as a MIGRATION and never as a deletion: each existing acceptance row converts to a `RedrawRecord` in status `proposed` carrying the original acceptance's [2026] VJS-CC-OPBOX 155 O2 direction and magnitude as its brief, retains its render rights, and may not be scored or reported as a deviation by any instrument pending disposition by covering sign-off or express registered direction ([2026] VJS-SC-OPBOX 1 order 26); the `proposed` status is carried for these rows in preference to `parked`, `parked` being reserved to rows covered by a live direction row.

**ORDER 29.** The acceptance state is closed PROSPECTIVELY at the parser, no instrument being able to construct an acceptance row from enactment (a condition the enacted `AuthorityVerdict`, closed at three with a test at `signoff.rs:275` asserting that "accepted" does not deserialise, already satisfies), and existing rows remain readable for exactly one purpose, their conversion under Order 28.

**ORDER 30.** The repeal reaches every FRAME-BOUND instrument, new and enacted, wherever an acceptance VERDICT is expressed, including repo-local convergent instruments in subscriber projects, which under [2026] VJS-SC-OPBOX 1 orders 1 and 5 are convergent implementations of the register's rule and not independent authorities; the repeal is of the automatic acceptance verdict and of nothing else, and no instrument's acceptance RECORD is deleted.

**ORDER 31.** `vds signoff record` shall REFUSE, as a front-door precondition, any frame whose frames-ledger row is `disclaimed`, whose `authorityBy` is `Unlabelled`, or whose authority layer is not a CURRENT SOURCE label under `[figma] authority`, naming the label it found and directing the signer to resolve the label by redraw and re-capture before signing, a registry entry over a frame that disclaims its own currency being a signed contradiction at the genesis of the condition precedent ([2026] VJS-SC-OPBOX 1 orders 23 and 25); this refusal operates at the front door only, creates no proof rule, and reddens nothing already recorded.

**ORDER 32.** The four directions of 2026-08-01 (flat containers; no floating on the dotmatrix; sidebar to the frames' shell; band off-screen; LOG ids 103751, 103951, 104325, 104739) shall be backfilled as `DirectionRecord` founding entries of the register under Order 26 before any frame-bound proof runs in blocking mode, a registry that cannot execute the order that founds it not being the condition precedent [2026] VJS-SC-OPBOX 1 order 23 requires.

**ORDER 33.** S-7D commences on enactment and the register is a CONDITION PRECEDENT to any frame-bound proof running in blocking mode, the interim posture described in the submission's fail-closed record (empty register, every frame-bound verdict `no_authority`, coverage owed and never green) being endorsed as the correct posture between this judgment and the register's first row.

## Section H - Commencement, dependency, statute status, lock entries, adoption

**ORDER 34.** S-013 (S-7B) and S-014 (S-7C) are independent of the register and commence on enactment.

**ORDER 35.** S-012's authority limb (`geometry.rs:643`) and S-016 (`visual_review.rs:110`) both call the sign-off register and have NO blocking effect until S-7D(2) and S-7D(3) commence and the surface concerned is registered, the flip from report to block being per surface at the moment of its frame's registration and never estate-wide.

**ORDER 36.** S-015 commences on enactment and is inert until register records carry `directedAt` or `measuredBy`, all 114 subscriber records presently carrying neither, so that nothing reddens retroactively and the I4 census at Order 17 is the instrument that keeps that inertness visible.

**ORDER 37.** The implementing repository shall record the following STATUTE STATUS CHANGES in the specification: S-7A(5) ENACTED AS AMENDED with new required fields, R14 and W3; S-7B ENACTED as drafted; S-7C ENACTED AS AMENDED with S-7C(5); S-5(9) ENACTED AS AMENDED with R4, I4, an amended `BOUND_NOTE` and an amended `register_completeness` scope note; S-7D(6) ENACTED AS AMENDED with the authority-first ordering, the amended `RESERVED_NOTE` and the amended R5 finding; S-7D(1)-(5) ENACTED AS AMENDED with the restated S-7D(4), the `DirectionRecord` row kind, `RedrawStatus::Parked` and the front-door label precondition; and the S-7(5) registry remains CLOSED AT FIFTEEN KINDS, no sixteenth kind being created.

**ORDER 38.** The implementing repository shall write ENFORCEMENT-LOCK entries, as a list and not a single slot (Order 3), pinning a seeded failing-direction test for each of: geometry R11, R12, R13, R14 and W3; the whole of `prohibition`; burndown R1, R2, R3, R6, R7 and the S-7C(5) reading-age rule; `register_completeness` R4 and the I4 census; `visual_review` R1 to R8 including the authority-first skip; and the S-7D front-door refusals at Orders 31 and 27, together with the named comparator required by Order 6 for any project generating a geometry authority snapshot.

**ORDER 39.** The implementing repository shall not adopt S-7A(5) or S-7C in any project until it has resolved the uncontradicted measurement that the geometry authority snapshot and the burndown reading default to paths inside the configured ledgers directory for which `ledger_staleness` carries no arm, so that R2 fails an enacted proof for a reason that is not a defect, the resolution being at the implementer's election either to add the arms or to move the defaults, and this order carrying no amendment to the statute.

**ORDER 40.** The subscriber repository /home/jellytot/Projects/opbox-prod/opbox-frontend shall, on adoption: open its fifteen accepted-over rows as `RedrawRecord`s in `proposed` carrying each row's original 155 O2 direction and magnitude (Order 28); retain the `accepted` map in `scripts/frame-code-parity.py` as the RECORD of those rows, which continue to render and may not be reported as deviations, that map ceasing to be readable as a VERDICT (Orders 29 and 30); generate a frames ledger, without which `crates/vds-cli/src/signoff.rs:63` refuses every registration; backfill the four founding direction rows (Order 32); and retire NONE of its twenty-eight repo-local gates until Order 2 is discharged for the covering kind on that repository, each retired gate's incident migrating into the register as the covering proof's seed fixture ([2026] VJS-SC-OPBOX 1 order 5).

---

# PART II - THE RATIO

**HOLDING 1 (unanimous in practice; articulated by Fidelity J).** Where a submission implements a binding ratio it is enacted without re-litigation under S-11(c), and where its drafted text under-states a qualification the apex court has already imposed the qualification is READ IN and the submission amended, never refused, because a statute that under-states a binding qualification is cured by amendment while a statute that contradicts one is a second copy of a rule and therefore one copy and a disagreement.

**HOLDING 2 (unanimous).** The demonstrated falsifiability on which primacy depends under [2026] VJS-SC-OPBOX 1 order 2 extends to EVERY INPUT the verdict rests on and not merely to the rule under test, so that a limb whose central assertion is produced by a program the engine neither hashes nor pins is not falsifiable by the engine however red its own unit test runs; and falsifiability is proved per kind by a seed-red and per repository by negative control recorded in the enforcement lock OF THAT REPOSITORY, a seed that lives only in a source comment or in a synthetic harness being a negative control nothing performs.

**HOLDING 3 (unanimous).** An instrument's REACH is part of its result: where a proof's verdict rests on an input the engine does not hash, or a rule fires only on the records that opted into it, or a class of subject sits outside the clause by construction, the proof must COUNT and NAME that input and that population on every run in the same breath as the verdict, because a green over a denominator nobody stated is indistinguishable from a green over the whole estate and the difference between them is the only thing the reader wanted to know.

**HOLDING 4 (Fidelity J and Engine J; Estate J not dissenting on the principle).** A deadline measured only against the input it gates is a deadline the subject sets, so a deadline clause must take its clock from the run's freshest INDEPENDENT input and a reading that stops moving must expire visibly rather than suspend the undertaking it witnesses; determinism (S-7(2)(1)) forbids the wall clock, it does not license the gated party to wind the clock.

**HOLDING 5 (unanimous).** An overruled doctrine is repealed as a VERDICT and preserved as a RECORD: the repeal reaches every instrument on the overruled ground at once, there being no lawful interval in which an overruled rule survives in an enacted instrument, but it is executed as a MIGRATION and never as a deletion, because the direction and magnitude that carried each acceptance are the successor regime's redraw brief and a repeal that deletes them destroys the evidence the successor runs on.

**HOLDING 6 (Fidelity J and Estate J).** Authority the instruments cannot read is authority the estate does not have: a Principal direction that disposes of a surface's conformance is itself a registrable sign-off act, hash-bound to its LOGGED DECISION rather than to a frame's content, and a register with no row kind able to record one is not the condition precedent [2026] VJS-SC-OPBOX 1 order 23 requires and cannot execute order 31 that founds it.

**HOLDING 7 (unanimous).** A frame-bound finding CLASSIFIES and never disposes: it is a docket entry and not an execution order, no instrument may auto-remove, auto-hide or unrender a shipped surface on the strength of one, [2026] VJS-CC-OPBOX 155 O7 remains the exclusive law of removal, and a remedy line that reads to an agent as an instruction to delete is itself the defect.

---

# PART III - DISPOSITION TABLE

| Sub | Fidelity J | Engine J | Estate J | MAJORITY DISPOSITION | Option | Vote | Amendments carried |
|---|---|---|---|---|---|---|---|
| **S-012** | EA (opt 1) | EA (opt 1) | EA (opt 1) | **ENACT AS AMENDED** | 1 | 3-0 on option, 3-0 on amendment | Orders 6-9: `comparator` + `comparatorDigest` required and digested; front-door refusal; R14 comparator-stale fatal; W3 unpinned-comparator warning + no primacy (narrowest of Engine/Estate) |
| **S-013** | ENACT (opt 1) | EA (opt 1) | ENACT (opt 1) | **ENACT, UNAMENDED** | 1 | 3-0 on enactment; 2-1 for no amendment | None (Order 10). Engine J's `sitesAtRegistration` recorded, not carried |
| **S-014** | EA (opt 1) | EA (opt 1) | ENACT (opt 1) | **ENACT AS AMENDED** | 1 | 3-0 on option, 2-1 on amendment | Order 13: S-7C(5) declared maximum reading age, fatal against the run's freshest independent input (Fidelity's narrower form over Engine's re-based clock) |
| **S-015** | EA (opt 1) | EA (opt 2) | EA (opt 1) | **ENACT AS AMENDED** | **1** (housed in `register_completeness`) | 2-1 on option, 3-0 on enactment | Orders 16-18: R4 resolution requirement (Fidelity + Engine, narrowest merge, R3 retained); I4 informational census + `BOUND_NOTE` denominator recital (Estate's narrower form over Fidelity's); scope-note widening recorded in spec and lock |
| **S-016** | EA (opt 1) | EA (opt 1) | EA (opt 1) | **ENACT AS AMENDED** | 1 | 3-0 on option, 3-0 on amendment | Orders 20-22: authority resolved before R2/R3, unsigned warns and skips, R1/R6 stay fatal (Estate's per-surface form over Fidelity's deferred commencement); `RESERVED_NOTE`; R5 remedial-inertness finding text |
| **S-017** | EA (opt 1 as amended) | EA (opt 1) | EA (opt 2) | **ENACT AS AMENDED** | **1** | 2-1 on option (Estate J dissenting), 3-0 on enactment | Orders 24-33: `screen_parity` acceptance-surface correction and struck consequence clause (3-0); three resolution routes in S-7D(4); `DirectionRecord` (Estate's narrower form); `RedrawStatus::Parked`; migration-not-deletion of the fifteen rows into `proposed` (Engine + Estate's narrower form); prospective parser closure; front-door label refusal (Estate's front-door-only form); founding backfill; register as condition precedent |

EA = ENACT_AMENDED.

---

# SCHEDULE B - RECORDED, NOT CARRIED

Each entry commanded one judge only and binds nothing; each is available to a future submission on its own case file. Recorded so the "narrowest amendment" computation is auditable.

1. **Fidelity J, 012-A (three-valued agreement).** `agrees: bool` cannot express a frame's silence; a per-surface-kind `not_drawn` state resolving to `no_authority`, and refusal of an all-undrawn snapshot under S-7(2)(4). Ground touched by no other judge. Recorded as the strongest uncarried finding in the set.
2. **Fidelity J, 012-B (commencement of S-7A(5) deferred to S-7D(2)/(3)).** Not carried; its practical effect is preserved by Order 35, the limb resolving `no_authority` against an empty register.
3. **Engine J, 012 (R14 refusing `agrees: false` with an empty `because`; the "bound measured, binding not measured" recital where no snapshot exists; `ledger_staleness` R7).** The `ledger_staleness` collision is nevertheless addressed administratively at Order 39.
4. **Engine J, 013 (`sitesAtRegistration` and the `prophylactic` flag; R6 refusing a row claiming neither).** Not carried; two judges enacted S-013 unamended.
5. **Engine J, 014 (`inputs` + `sourceDigest` with R9; deadline re-based on the screens ledger; striking "in the same change" from S-7C(2); `ledger_staleness` R8).**
6. **Engine J, 015 (housing the clause as a sixteenth kind `measurement_coverage`).** The reasoning is recorded in full for the record: `register_completeness` opens with `load_fresh(project)?` and `RegisterIndex::build(&ctx.store())?`, both of which abort as precondition failures before the S-5(9) block is reached, and both have fired on the subscriber estate. The majority housed the clause there notwithstanding; the exposure is recorded and not cured by this judgment.
7. **Engine J, 016 (typed deltas `{kind, what}`; the R5 split making addition-only deviations report rather than block; R9 evidence-artefact digests; completing R7 against equal `reviewedAt`; W4 naming signed frames with no live review).**
8. **Fidelity J, 017-D (making `screen_parity` authority-aware so an unregistered screen resolves `no_authority` and cannot be scored `deviate`).** Not carried; the acceptance-surface correction at Order 24 is carried, the positive authority-awareness limb is not.
9. **Fidelity J, 017-B in part (a direction row's redraw-by date and the fatal redraw-duty limb past it).**
10. **Engine J, 017 (pinning the count of unconverted acceptance rows as a `burndown` metric under S-7C).** Recommended to the subscriber under Order 40 but not enacted.
11. **Estate J, dissent on S-017 option 2** and its consequential reasoning that a repeal reaching subscriber instruments is ultra vires; recorded, and answered on the majority's terms by Orders 28 to 30, which repeal the verdict while preserving the record and the fifteen rows' render rights.