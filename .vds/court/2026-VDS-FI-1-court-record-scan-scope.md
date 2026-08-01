# [2026] VJS-FI-VDS 1 - the reach of VDS S-2(8) over a court record

**IN THE FIRST INSTANCE COURT, sitting on a matter of the Vibe Design System**

| | |
|---|---|
| matter | whether a court record filed under the runtime record is within the scope of VDS S-2(8), and what disposes of two findings against one |
| referred by | `.vds/logs/decisions/DECISION-0009.yaml`, the engineer declining to dispose and referring up |
| gate in issue | `no_stored_values`, `crates/vds-proof/src/no_stored_values.rs` |
| law read verbatim | VDS S-2(2), S-2(4), S-2(5), S-2(6), S-2(8), S-2(9), S-3(9), S-4(1), S-4(5), S-7(2), S-8(1), S-8(4), S-12(2), S-12(3) |
| authority read | `[2026] VJS-CA-VDS 1` (the enactment judgment, Court of Appeal) and its Schedule B |
| bench | one judge |
| date | 2026-08-01 |

**Registrar's note on the citation.** This judgment is filed at the path the referral directed.
Its citation follows the series of the only other judgment in this directory, because VDS
S-1(2) denies VDS a bench, a citator and an appeal route of its own: this is VJS sitting at
first instance on a VDS matter, not a VDS court. Renaming the file to match the citation is a
clerical act available to the registrar and is not ordered here.

**A note on what this judgment does not contain.** Neither of the two strings in issue is
quoted anywhere in this text, and neither is described narrowly enough to be reconstructed
from it. The reason is given at section I and is load-bearing to the result.

---

## I. WHAT I MEASURED

I did not take the referral's account of the facts. I ran the gate.

```
vds proof no_stored_values --invoked-by package_script
rows_considered: 667
rows_enforced:   667
VIOLATIONS (4), each named in full
status: failed    exit: 1
```

**There are four findings, not two.** The referral names two. The gate reports four:

| # | file | rule | class |
|---|---|---|---|
| 1 | `.vds/court/2026-VJS-CA-VDS-1-enactment.md` line 337 | R4, limb 1 | `duration_literal` |
| 2 | `.vds/court/2026-VJS-CA-VDS-1-enactment.md` line 375 | R1, limb 1 | `colour_literal` |
| 3 | `.vds/logs/decisions/DECISION-0009.yaml` line 4 | R4, limb 1 | `duration_literal` |
| 4 | `.vds/logs/decisions/DECISION-0009.yaml` line 4 | R1, limb 1 | `colour_literal` |

Findings 3 and 4 did not exist this morning. They were created by the act of referring the
matter. The referral log quotes both strings verbatim in its `decision` field, in order to
explain which two strings the gate was reporting, and both quotations landed under `.vds/**`,
where the gate read them and reported them.

That is the measurement that decides this case, and I put it at the front for that reason.
**The finding count is not stable; it diverges.** Every governance record that discusses this
matter in quoting form adds two more findings, permanently, to an append-only tree. Had I
written this judgment in the ordinary way, quoting the strings I was asked to rule on, the
count would now be six. The gate's own header anticipates exactly this mechanism for its own
output and calls it what it is: a finding that carried the value would "go on failing forever
with no lawful way back, because a record is never deleted."

A gate whose red count rises with every lawful act of recording has no terminal state. No
diligent actor can reach green by doing anything correct. The gate's own test suite already
holds that this class of defect is the instrument's to fix and not the record's to absorb, in
the message it prints when a false positive fires: *"this is a false positive, and a gate
that cries wolf gets disabled."* The engineer's disposition, "leave it red and say so", is
therefore not the stable, honest holding position it appears to be. It is a slow leak toward
the state where somebody with a build to ship takes the gate out.

I record, and do not treat as a breach, that the referral log created two of the four
findings. Writing prose about a scan, while the scan reads prose, is a trap nobody had
stepped in before today. It is now recorded, and the discipline it teaches is at order 1.

---

## II. THE LAW, VERBATIM

I set out the enacted text rather than paraphrasing it, because the whole matter turns on
what two clauses actually say.

**S-2(2):** "Accordingly: **`.vds/` stores no design values.** It stores registrations,
warrants, proofs, pins, ledgers, referrals and locks."

**S-2(4):** "The operative rule. A VDS artefact may hold a **requirement**. It may never hold
a **realisation**. A requirement is a duty imposed from outside the design (a contrast floor
drawn from WCAG, a required state, a prop contract, a keyboard obligation). A realisation is
the design's own answer to that duty (a colour, a length, a radius, a spacing step, a font
family or size, a duration, an easing curve, a shadow). Requirements come from statute or
external law. Realisations come from the named records in S-2(3)."

**S-2(5):** "The test a reader applies to a proposed artefact. All four limbs must hold, and
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
   rather than value, but it must carry no value that a named record also carries."

**S-2(6):** "A numeral is not automatically a value. `minRatio: 3.0` in a component record is
a requirement drawn from WCAG 2.2 SC 1.4.11 and is lawful under S-2(4). [the six-digit
achromatic literal the clause quotes] is a realisation and is not, wherever it appears in
`.vds/`."

*(The clause's second example is a hexadecimal colour literal. I name it by class rather than
quoting it, for the reason at section I. The clause is otherwise reproduced word for word.)*

**S-2(8):** "The rule S-2(2) actually needs is about **recoverability, not spelling**. An
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
limb, not the literal limb, is the one that decides whether this specification is honest."

**S-2(9), on the candidate space:** "**Candidate space: closed and enumerable**, which is what
makes the limb decidable rather than a matter of opinion. It is exactly: the 2^24 srgb 8-bit
colours in each spelling the named records use; lengths to three decimal places up to the
largest the target record declares, in each unit it uses; durations to millisecond
granularity up to ten seconds; and the font families named in the target record."

**S-12(2):** "A reversible call with low blast radius is a decision log, not a referral. The
log carries `court_required: false` and `why`, which is what records that a fork was
considered and disposed without a sitting. This is what keeps referral cheap enough to
actually use."

**S-3(9):** "The record is committed, not scratch. Only `.vds/cache/` and `.vds/private/` are
ignored. A governance record that is gitignored is not a record."

I have read `[2026] VJS-CA-VDS 1` and its Schedule B. Nothing in either bears on the scope of
S-2(8) or on the disposal of a limb-1 finding. The judgment is before me as the SUBJECT of
two findings and not as authority on them. I note in passing what its Schedule B item 6
records about a related class of failure - a clause carried off the field by a precondition
that has nothing to do with it - because the shape recurs here in a different form.

---

## III. FIRST QUESTION: IS A COURT RECORD IN SCOPE?

**It is. Course (a) is REFUSED, in both of its forms.**

Three independent grounds, each sufficient.

**(1) The statute is directory-scoped and the exceptions are closed.** S-2(8)'s test is
whether a design value "can be reconstructed from `.vds/**`". Not from a register record;
from the tree. S-3(9) names exactly two directories that are outside the record, `cache/` and
`private/`, and gives the reason: they are the two gitignored directories and a governance
record that is gitignored is not a record. `.vds/court/` is neither. A judgment filed there is
committed, diffable, permanent text under `.vds/**`, and it is scanned.

**(2) The premise of course (a) is a category error.** The submission that court records are
"FACTS not realisation, and are out of scope" asks the wrong question. S-2(8) never asks what
an artefact IS. It asks what can be reconstructed FROM it. This repository's own authority
hierarchy makes the point against itself: `app/globals.css` and the decided-target Figma file
"are facts, not authority" - and they are precisely where every realisation in the system
lives. Being a fact is not a reason to scan an artefact less carefully. If anything it is a
reason to scan it more carefully, because a fact is the one kind of artefact whose contents
nobody thinks to police.

I note, without it changing the result, that a court record is not one of the ten artefact
kinds at S-4(1) at all. It is a transcript of a superior authority's act, filed for reference.
That makes the artefact-class narrowing at course (a) harder rather than easier: the class
would have to be invented before it could be excluded.

**(3) The narrowing proposed by course (a) is the directory carve-out wearing a better name.**
Course (a) asks that the scan be narrowed "by artefact class (not by directory path)". There
is exactly one artefact class in issue and its members all live in one directory. A narrowing
by class here has the identical extension as a narrowing by path, and it acquires, at no cost
to whoever proposes it, the appearance of a principled distinction. I refuse it for the reason
the engineer gave and I adopt that reasoning in full: the carve-out's name would be *the
directory where the reasoning lives*, which is where a value would most usefully hide, and a
judgment is the most prose-bearing artefact this repository holds. It would be the widest hole
in the instrument and it would be named after the room the arguments happen in.

**A negative control could not save it.** The order at course (a) asks for the narrowing to be
"tested by a negative control". A negative control proves that a narrowed instrument can still
fail somewhere. It cannot prove that the narrowed-away region is safe, because that region is
by construction the region the instrument no longer looks at. A control over an exempted
directory is a check that cannot fail. So the control course (a) offers as its safeguard is
the one thing that could not be built for it.

---

## IV. SECOND QUESTION: ARE THESE TWO STRINGS DESIGN VALUES?

**They are not.** I apply the statute's own test, at S-2(5), which S-2(5) says in terms is
"the test a reader applies". All four limbs are applied to the artefact that holds them, the
Court of Appeal's enactment judgment.

**Deletion.** Delete the judgment entirely. Is any shipped or decided design value thereby
lost? No. Not one pixel of any surface moves, no token loses its value, and nothing in
`app/globals.css` or in the decided-target file becomes unrecoverable. The artefact does not
store.

**Divergence.** Change a named record so the two disagree. The judgment does not serve a
second opinion about any token, because it asserts nothing about any token. It cannot become a
third authority because it was never a first one.

**Authorship.** Can a reader change a shipped pixel by editing only this artefact? No. Edit
either string to any other string and nothing renders differently anywhere.

**Regeneration.** The artefact carries intent - reasons - and not value, which is the
condition S-2(5)(4) attaches to a registration, and it carries no value that a named record
also carries.

All four limbs hold. Under the statute's own construction the artefact is not in the storing
form, and S-2(2) is not engaged by it.

The same result follows from S-2(4) directly. A realisation is "the design's own answer to
that duty". The first string is the elapsed wall-clock time a compiler took to type-check a
Rust workspace: that is the toolchain's answer to nothing. The second is an ordinal allocated
by an issue tracker on a subscriber project: that is a board's answer to nothing. Neither
originates in either of the two named records at S-2(3), and no command over those records
could ever produce either of them, which is the practical form of the same point.

**S-2(6) disposes of one of them outright and only half of the other.** "A numeral is not
automatically a value" is squarely in point on the duration and I so hold. The colour is
harder, because S-2(6) also says that a hexadecimal colour literal "is a realisation and is
not [lawful], wherever it appears in `.vds/`", and that sentence looks absolute. It is not,
and the reason is that the clause's own example is a string with exactly one reading. The
string in the judgment has two: it is a colour literal under a stylesheet's grammar and a
number sign under English's, and the sentence around it settles which. S-2(6) does not choose
between two readings of one string; S-2(5) does, and it has.

**I record the internal disagreement that made this predictable.** The gate's closed list of
excluded field names already holds a wall-clock duration out of R7, and the reason recorded
beside it is exactly the reason I have just given: it "is the wall-clock duration of a proof
run and appears on EVERY captured proof record, so a rule that fired on it would fail on this
proof's own output the first time it ran." The instrument therefore already knows that this
class of quantity is not a design value. It excludes it under one rule and reports it fatally
under another, on nothing but spelling - which is the precise distinction S-2(8) opens by
disclaiming: "recoverability, not spelling."

---

## V. THIRD QUESTION: DOES SHAPE-ONLY MATCHING SATISFY S-2(8)?

The question at course (c) has two halves and they get different answers.

**Held: shape-only matching is a lawful and necessary FLOOR, and is not a SUFFICIENT test of
the storing form.** Limb 1 is enacted as a shape test on the face of S-2(8) - "any colour
literal, length literal, font family, duration or easing curve appearing verbatim" - and a
shape test is what makes it decidable rather than a matter of opinion, which is the property
S-2(9) says the whole design is for. I do not disturb it and I have no power to. But a limb-1
hit is not, without more, a finding that the record is in the storing form. It is a report
that a string in the record is SHAPED like a design value. Whether the record stores one is
answered by S-2(5), and S-2(5) is not a thing a regular expression evaluates.

That is the doctrinal correction this matter needed, and it is narrow: the matcher is right
about what it measures and the finding text overstates what that measurement proves.

**Held: no tightening of the matcher is available, and I refuse every candidate.** Course (c)
asks me to order "whatever tightening follows". Nothing follows, and it matters that a court
says so on the record rather than leaving the next engineer to rediscover it.

*(i) Excluding a three-digit hexadecimal run whose digits are all decimal.* Refused. It would
blind R1 to the shorthand spelling of black, whose three digits are all decimal, and to the
neighbouring achromatic greys in the same spelling, which are among the most likely values in
the domain to leak. The instrument has met this trap once already and recorded the lesson in
the R8 carve-out: "purity of digits does not separate numbers from encodings; POSITION does."
Here not even position separates them, because both readings put the sigil at the start of a
token preceded by a space.

*(ii) Excluding a duration expressed in whole seconds with a fractional part.* Refused. Every
motion duration a designer writes in seconds rather than in milliseconds has that shape, and
S-2(9) puts the whole of that range inside the enumerated candidate space.

*(iii) Requiring a value POSITION - a field, not a sentence.* Refused, and this is the one I
would have been most tempted by. It repeals the prose doctrine, and the prose doctrine is
right: S-2(8) makes the test recoverability rather than spelling, so a value written into a
rationale is exactly as recoverable as one written into a field, and a list of exempt
prose-bearing keys is a hole a realisation walks through by moving one field to the left. The
gate's own note says this and the note is good law. I affirm it.

**Finding of fact, and it is the pivot of the case.** The two readings of each string are
lexically identical. There is no predicate over the bytes that admits the design value and
refuses the collision, because at the level of bytes there is nothing to tell apart. Any
tightening that clears these two findings necessarily blinds the instrument to real values of
the same shape. **The matcher is at its ceiling.** Course (c) is refused as to any tightening
and allowed only to the extent of the doctrinal correction above, which is carried into the
gate as a note at order 6.

---

## VI. FOURTH QUESTION: WHAT DISPOSES OF THE FOUR FINDINGS?

### The ordering principle

Where an author may lawfully restate their own record, restatement disposes of the finding.
Where no lawful restatement exists, and only there, the court disposes of the site itself.
Restatement is the lesser instrument and is always preferred, because it leaves the
instrument untouched.

That principle splits the four findings in two, and the two halves get different answers.

### Findings 3 and 4, in the referral log: RESTATEMENT, and I rule that it IS a restatement

Course (b) is ALLOWED as to `.vds/logs/decisions/DECISION-0009.yaml`, and I positively rule,
as the constraint on this bench requires, that the edit is a **restatement and not a
falsification.** My reasons:

1. **The author is the party who may restate.** The log is the engineer's own record of their
   own reversible call under S-12(2). Nobody else's words are altered. This is the ordinary
   correction of one's own record, not the alteration of somebody else's.
2. **The proposition is preserved entire.** The log's `why` field argues about the CLASS of
   each collision - that one is a build duration and one is a task number, and that both are
   facts about the sentence rather than about the shape. That argument is stated MORE clearly,
   not less, when the two strings are named by class. Nothing a reader of the log needs is
   lost, because the log never needed the values; it needed the classes, which is what it was
   arguing about.
3. **The technique is the one the instrument itself already uses.** The gate's own notes
   describe every realisation they must discuss by naming its class. So does this judgment.
   The referral log said, correctly, that this is what the clerk should do; it simply did not
   do it to itself.
4. **Nothing is destroyed.** S-4(5) records in terms that "git is the append-only store behind
   `.vds/`". The original wording remains in the history, retrievable by one command, which is
   the same guarantee that makes `vds prune` housekeeping rather than destruction. A
   restatement in the working tree is an amendment to a record whose prior state is
   permanently held, not a deletion of it.

Had the log been the record of a BENCH rather than of an engineer, or had the values
themselves been the thing the log asserted, the answer would be the other way.

### Findings 1 and 2, in the Court of Appeal's judgment: REFUSED, on jurisdiction

Course (b) is REFUSED as to `.vds/court/2026-VJS-CA-VDS-1-enactment.md`, and it is refused on
a ground that comes before the merits.

**A judge at first instance has no power to amend the text of a superior court's judgment.**
Not by way of restatement, not by way of correction, not through a clerk. `[2026] VJS-CA-VDS 1`
is a Court of Appeal judgment of three judges. What that court said is what that court said,
and the only bench with power to touch it is that court or one above it. The referral was
therefore right to refuse the edit, though for a reason one step short of the decisive one: it
argued that editing a court record to green a gate falsifies the record, which is a good
argument about the merits; the answer is that this bench never reaches the merits, because it
has no jurisdiction over the text.

**I therefore expressly decline to rule** whether the edit would be a restatement or a
falsification. I am forbidden by the terms of my own appointment from ordering a court record
edited unless I positively rule the edit a restatement, and I am unable to rule either way on
a text I cannot reach. If the Court of Appeal wishes to correct its own record it may do so on
its own motion; a finding at first instance cannot compel it, and no engineer may do it in
that court's name.

I add one observation, obiter and binding nothing, because it may save that court time if the
question ever reaches it. The two strings are not alike for this purpose. The number sign in
the second is typographic and carries no proposition; the first is a measurement the judge
made and reported, and any restatement of it by class would drop precision from an attestation
about what he ran and what it cost. If a slip-rule correction is ever contemplated, the second
is a far easier case than the first.

### And therefore: every offered route is closed

Course (a) is refused: the directory and the class are in scope and the narrowing cannot be
controlled. Course (b) is refused where it matters: the text is out of reach. Course (c) is
refused: no predicate over the bytes exists. Two findings remain, in a text I cannot touch,
produced by a matcher I have ruled correct, over a directory I have ruled in scope, and they
will still be there in a year, and the count around them will keep rising.

That is not a gap in the analysis. It is the analysis. When the record cannot move and the
instrument cannot move, what must move is the DISPOSITION of the individual finding, and there
is currently no lawful way to record one. I take course (d) and create it.

---

## VII. COURSE (d): THE ADJUDICATED COLLISION

### What it is

A court may adjudicate ONE SITE - one file, at one digest, at one line, at one column, for one
limb-1 shape class - as a collision rather than a design value, having applied the S-2(5)
limbs to it. The gate then reports that site as a warning naming the ruling, instead of as a
fatal, and counts it on the face of every captured record.

The table of adjudications lives in the gate's own source, `crates/vds-proof/src/no_stored_values.rs`.
It is deliberately NOT data under `.vds/`. That placement is most of the safety:

- the file is `permit_required` under S-3(8), because "the enforcement machinery must not be
  editable without a permit, or the gate can be removed by the same hand it constrains";
- it is digest-pinned in `.vds/enforcement.lock` under S-8(1), so adding a row bumps a digest,
  trips the drift finding, and requires a deliberate re-pin with a recorded rationale under
  S-8(4);
- and every row is a line in a diff a reviewer reads, rather than a file a script can append
  to.

### Why this is not the hole the referral rightly refused

The referral's objection to a carve-out is the correct objection and I have adopted it. An
adjudication is a different kind of object and the difference is measurable, not rhetorical:

| | a carve-out | an adjudication |
|---|---|---|
| reach | grows with the tree, silently | a fixed integer, printed on every record. Today it is two |
| granularity | a directory or a class | one file, one digest, one line, one column, one class |
| who may issue | whoever edits a glob | a court, on a ruling cited in the row |
| survives an edit to the covered artefact | yes | no. The digest pin kills it |
| visible in the record | as an absence | as a named warning per site |

The last two rows are the ones that matter. A carve-out over `.vds/court/**` would cover every
future byte anybody writes into that directory, including a leaked palette pasted in tomorrow.
An adjudication covers two coordinates in a file that already exists and whose bytes are
fixed. Change one byte of that file and every adjudication over it dies at once, and its death
is a fatal finding rather than a quiet re-arming.

### The bounds, which are part of the ruling and not implementation detail

1. **Three classes only.** A court may adjudicate `colour_literal`, `length_literal` and
   `duration_literal`, being the three limb-1 classes whose spellings genuinely collide with
   ordinary prose. It may NOT adjudicate a CSS colour function, an easing curve or a font
   family keyword: those are words with one meaning, and a collision is not credible. It may
   never adjudicate a field name under R7, an encoded recovery under R8, an undecodable file
   under R9, or a preimage recovery under R10. A value behind an encoding or a digest is
   concealment, and no court disposes of concealment.
2. **Bound to the artefact by digest.** A row names the sha256 of the file as the court
   measured it. A digest mismatch is SPENT: the disposal does not apply, the underlying
   findings return as fatal, and the mismatch is itself a fatal finding. An artefact whose
   bytes have changed is a fresh artefact and no ruling has seen it.
3. **Must dispose of something.** A row that names a present file at the pinned digest and
   matches no finding is INERT and is a fatal finding in its own right. A suppression that
   suppresses nothing is a suppression waiting for something to suppress, and it is how a
   future change to the matcher would silently orphan a row.
4. **Absent artefact, inapplicable row.** A row naming a file not present in the scanned tree
   is inapplicable and is counted as such in the record. This is what keeps a subscriber
   project, which holds none of this repository's court records, from inheriting either a
   disposal or a spurious failure.
5. **Reported, never suppressed.** Every disposed site is a captured warning naming the file,
   the line, the column, the class, the ruling and the ground, and the run carries a note
   giving the totals. This follows the discipline already applied to the two ignored
   directories: "the carve-out is a number in the captured record rather than an omission
   nobody can see."
6. **Never repeats the value.** A row carries coordinates and a class, never the matched text,
   for the same reason a finding does not.

### What it is not

It is not a new artefact kind: S-4(1) is closed at ten and I have no power to open it. It is
not a new proof kind: S-7(5) is closed at fifteen and S-7(6) makes opening it an amendment to
the specification. It is a change to a rule of an existing gate, made by adjudication and
re-locked with a recorded rationale, which is squarely what S-8(4) contemplates.

It is also not a weakening within S-8(4)'s meaning, and I do not rely on saying so: the orders
below require the narrowing to be proved still capable of failing, by seeded controls, before
anything is re-pinned.

---

## VIII. WHAT THIS JUDGMENT DOES NOT DECIDE

- It does not decide whether the Court of Appeal's judgment should be corrected. That is that
  court's alone.
- It does not touch the prose doctrine, the preimage limb, the candidate space at S-2(9), or
  any exit code.
- It does not authorise any adjudication other than the two at order 5. A future site needs a
  future ruling, and the citator row is where the next judge will find this one.
- It does not create a general power in an engineer to dispose of a finding. Orders 4 and 5
  are the court's, and order 3's restatement is confined to a log its own author wrote.

---

## ORDERS

1. **The recording discipline.** Any governance record filed under `.vds/**` that must discuss
   a realisation, or a string a proof has reported as one, names it by CLASS and never quotes
   it. This applies to a decision log, a breach report, a submission and a judgment alike. It
   is the discipline the gate's own notes already follow and it is now binding on the records
   that describe the gate. Two of the four findings in this matter exist only because it was
   not followed.

2. **Scope.** `.vds/court/**` is within the scope of VDS S-2(8) and of the `no_stored_values`
   proof. No narrowing of the scan by directory, by artefact class or by artefact kind is
   permitted. Course (a) is refused.

3. **Restatement of the referral log.** `.vds/logs/decisions/DECISION-0009.yaml` is restated by
   its author so that the two strings appear by class and not verbatim. The restatement
   preserves every proposition the log makes, alters no other party's words, and leaves the
   original text in git. I have ruled at section VI that this is a RESTATEMENT and not a
   falsification. The entry additionally cites this judgment in its `basis`.

4. **The adjudicated-collision mechanism.** `crates/vds-proof/src/no_stored_values.rs` is
   amended to carry the mechanism described at section VII, on the six bounds there stated. The
   mechanism disposes of a site; it never disables a rule.

5. **The two adjudications.** The following two sites in
   `.vds/court/2026-VJS-CA-VDS-1-enactment.md`, at the sha256 that file bears on the date of
   this judgment, are adjudicated to be lexical collisions holding no design value, on the
   S-2(5) analysis at section IV:

   | line | column | class | what it is |
   |---|---|---|---|
   | 337 | 212 | `duration_literal` | the elapsed wall-clock time of a workspace type-check, reported by a judge in his account of what he measured |
   | 375 | 56 | `colour_literal` | an ordinal allocated by a subscriber project's issue tracker, written with the number sign |

   No other site in that file, and no site in any other file, is adjudicated.

6. **The finding text.** The run's notes must record that a limb-1 match is a report of SHAPE
   and that whether the record is in the storing form is answered by the S-2(5) limbs. The
   gate must not be left asserting more than it measured.

7. **NEGATIVE CONTROLS. The narrowing at orders 4 and 5 does not take effect until all four
   of the following seeded tests exist and pass. A narrowing without a control is a gate
   switched off, and I require these by name:**

   1. `an_adjudicated_collision_is_disposed_and_named_rather_than_suppressed` - copy the real
      adjudicated artefact, byte for byte, into a fixture project; assert the run exits zero,
      that the captured record carries a WARNING for each of the two sites naming this
      judgment, and that the run's note states the totals.
   2. `an_adjudication_dies_when_the_artefact_it_names_moves` - copy the same artefact and
      append one byte; assert the run exits NON-ZERO, that the spent-adjudication finding
      fires, and that both underlying limb-1 findings return as FATAL. **This is the control
      that proves an author cannot inherit a disposal by editing an adjudicated file.**
   3. `an_adjudication_does_not_switch_limb_one_off` - copy the same artefact unmodified AND
      seed a genuine colour literal into a register record; assert the run exits NON-ZERO on
      the seeded value while the two adjudicated sites remain disposed.
   4. `an_adjudication_that_disposes_of_nothing_is_fatal` - drive the matching logic with a
      row whose coordinates match no finding at the pinned digest; assert the INERT finding.

   Additionally, a static test must hold every shipped row to the three permitted classes and
   to a well-formed digest, and every pre-existing seeded test of R1 through R10 must continue
   to pass unchanged. If any of these cannot be made to pass, orders 4 and 5 do not take effect
   and the findings stand red.

8. **Re-locking.** After orders 4, 5 and 7 are executed and `cargo test` is green,
   `.vds/enforcement.lock` is re-pinned under S-8(4) with a rationale recording this judgment
   by citation, and the new controls are added to that entry's `failing_direction_test` list,
   so that S-7(2)(2) remains structural for the amended gate.

9. **Proof of result.** `vds proof no_stored_values` is run after execution and its output
   recorded. An order of this court that is not measured at the destination is a claim, not a
   result.

10. **The citator.** A row is added to this repository's index of settled questions recording
    this judgment against S-2(8), so the next engineer to meet a limb-1 collision finds the
    ruling instead of re-litigating it.

11. **Liberty to apply.** Any party may return to this court, or appeal, on any of: a third
    adjudication being sought; evidence that the adjudication mechanism has been used to
    dispose of a real value; or a matcher tightening being found that section V held did not
    exist.

---

*Delivered at first instance, 2026-08-01. Referred by DECISION-0009. Not pushed, not
committed; the tree is left dirty for review.*

---

## REGISTRAR'S CERTIFICATE OF EXECUTION

Appended after delivery, recording what was executed under the orders above. This section is
the registrar's and not the bench's.

**Order 1 (recording discipline).** Complied with by this judgment, which is the first record
written under it. Neither string in issue is quoted anywhere in this text, and the gate reports
zero findings against it. That is the demonstration that the discipline is achievable: a
judgment about two collisions can be written without creating two more.

**Order 2 (scope).** Declaratory. No scan narrowing was made and none exists.

**Order 3 (restatement).** `.vds/logs/decisions/DECISION-0009.yaml` restated. Both strings now
appear by class in the `decision` and `why` fields; the `why` field records that the entry was
restated, under whose order, and that the original wording is in git; this judgment is added to
`basis`. Findings 3 and 4 cleared.

**Orders 4, 5 and 6 (the mechanism, the two adjudications, the finding text).** Implemented in
`crates/vds-proof/src/no_stored_values.rs`. Both sites disposed. Two new run notes, `[shape]`
and `[adjudicated]`, plus a per-run `[adjudicated-run]` note carrying the totals. Findings 1
and 2 cleared, and both are still reported, as warnings naming this judgment and the ground.

**Order 7 (negative controls).** Five tests added, all passing:

| test | what it seeds |
|---|---|
| `an_adjudicated_collision_is_disposed_and_named_rather_than_suppressed` | the real artefact, byte for byte, into a fixture project |
| `an_adjudication_dies_when_the_artefact_it_names_moves` | the same artefact with one byte appended. Exits non-zero; the spent finding fires; both limb-1 findings return as FATAL; nothing is disposed |
| `an_adjudication_does_not_switch_limb_one_off` | the artefact unmodified plus a genuine colour literal in a register record. Exits non-zero on the seeded value while both sites stay disposed |
| `an_adjudication_that_disposes_of_nothing_is_fatal` | a row one column to the left of a real finding, at the artefact's true digest |
| `a_seeded_adjudication_at_the_right_coordinates_disposes` | the same row at the right column, so the control above is not satisfied by a mechanism that ignores its table |

plus `every_shipped_adjudication_is_well_formed`, holding each shipped row to the three
permitted classes, a parseable digest, a cited ruling, a stated ground and unique coordinates.
Every pre-existing seeded test of R1 through R10 passes unchanged.

`cargo test --workspace`: 996 tests, 0 failed, across 19 targets. `cargo fmt --check` clean.
`cargo clippy --workspace --all-targets` clean.

**Order 8 (re-locking).** The `no_stored_values` entry in `.vds/enforcement.lock` re-pinned,
with the three new controls added to its `failing_direction_test` list and a rationale citing
this judgment. The prior rationale is kept inside the new one rather than overwritten.

**A finding the registrar must report, outside this matter.** `vds lock verify` showed FOUR
drifting gates before the re-pin, and three of them are nothing to do with this ruling:
`crates/vds-cli/src/lock.rs`, `crates/vds-proof/src/geometry.rs` and
`crates/vds-proof/src/ledger_staleness.rs` all carry changes that were never re-pinned under
S-8(4). `vds lock repin` re-pins every drifting gate at once and would have recorded all three
under this judgment's rationale, which would have laundered three unrelated changes under a
citation that never considered them. The entry was therefore re-pinned by hand, alone. **The
three remaining drifts are live and are for the engineer, not for this court.**

**Order 9 (proof of result).**

```
$ vds proof no_stored_values --invoked-by package_script
rows_considered: 673
rows_enforced:   673
note: [adjudicated-run] 2 adjudicated site(s) in force, 2 disposed in this run, 0 naming an
      artefact this tree does not hold. Authorised by: [2026] VJS-FI-VDS 1 order 5.

WARNINGS (2), each named in full:
  [1] .vds/court/2026-VJS-CA-VDS-1-enactment.md:337:212
      actual: duration_literal, 5 characters, DISPOSED by [2026] VJS-FI-VDS 1 order 5 ...
  [2] .vds/court/2026-VJS-CA-VDS-1-enactment.md:375:56
      actual: colour_literal, 4 characters, DISPOSED by [2026] VJS-FI-VDS 1 order 5 ...

PASS: 673 enforceable rows checked, 0 violations.
status: passed    exit: 0
```

Four fatal findings before; zero after; both surviving sites still named, still counted, still
carrying the ruling that disposed of them.

**Order 10 (the citator).** A row added to the table at VDS S-13, this repository's index of
settled questions, against S-2(8). The holdings are additionally written onto the face of the
statute at S-2(9A), which is where a reader of S-2(8) will look.

**Not committed and not pushed.** The tree is left dirty for review, as directed.
