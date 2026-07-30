# site-factory - the seven-skill writing run

The whole point of the AI/MCP layer is the writing, and the writing is done by skills.
This is that chain actually run, in the order the pack declares, on a real brief. It is
the reference run: what `WRITING-BRIEF.md` is asking for, done once, properly.

Subject: **site-factory itself**. Chosen because every fact about it is checkable in
this repo, so nothing in the copy below had to be invented. A run on a subject I only
half knew would have produced the exact filler this codebase exists to refuse.

Brief in:

> A website factory for people who build sites. A brief goes in and a built,
> token-driven site comes out, with every design decision visible and every unwritten
> line counted. Instead of a page builder that hides its choices, or a template you have
> to reverse-engineer.

Twenty unwritten lines came out, assigned across six skills. What follows works them.

Ban list in force throughout (`copy.js` `BANNED`, the same constant
`tests/copy.test.js` enforces, so the page and the tests cannot drift):
solutions, cutting-edge, empower, seamless, unlock, best-in-class, world-class,
synergy, leverage.

---

## 1. `revenue/skills/offer-clarifier`

Required first. A page can only be as clear as the offer under it.

### Raw inventory

A CLI and a local studio. 35 config fields in 9 layers. 21 block types in 42 variants.
4 style packs. Two routes, which compile differently rather than being one flow with a
toggle. Output is a folder: `dist/home.html`, `home.css`, `blocks/`, `tokens/`,
`manifests/`, plus `COPY-TODO.md`, `copy-brief.json`, `WRITING-BRIEF.md`. 21 matching
Figma component sets bound to one variable library. An optional governance seam. Zero
dependencies. 67 tests.

### The transformation

**Before.** You have an idea and an empty folder. You take a template and spend two days
undoing its opinions - its type scale, its shadows, its idea of a card - and at the end
you still cannot say why the spacing is 24px. Or you use a builder, and what comes out
is a page you cannot read, cannot diff, and cannot hand to anyone.

**After.** One sentence in. A running site whose every value traces to a named token,
whose unwritten lines are listed by name and count, and whose design decisions sit in a
file you can argue with.

In the buyer's words: *"I spend longer fighting the starter than building the thing."*
and *"I don't know why it looks like that."*

### Before/after grid

| Lens | Before | After |
|---|---|---|
| **Functional** | Can produce a page; cannot change it systematically. Every tweak is a search-and-replace. | Change one token, see it everywhere. Hand the folder to someone else and they can read it. |
| **Emotional** | A quiet suspicion the design is arbitrary, because nothing says otherwise. | The decisions are legible, so you can disagree with them. That is the relief. |
| **Social** | "Here is the Figma, here is the repo, they disagree." | The component set and the code ship together, and a test fails when they diverge. |
| **Status** | Assembling something. | Running a system. |

### Ideal buyer

A solo builder or a two-to-five-person studio shipping client sites or internal tools,
already past templates. They have a Figma file and a repo that no longer agree. They
tried a page builder and abandoned the export. They believe the blocker is that they are
"not a designer" - the actual blocker is that nothing they use makes a design decision
explicit enough to argue with. In 30 days they want to be able to say: every number on
this page came from somewhere I chose.

**Bad fit.** Anyone who wants a drag-and-drop canvas and never wants to see a config.
Anyone who needs a library of 200 finished marketing templates.

### Value equation

| Dimension | Score | Note |
|---|---|---|
| Dream outcome | **Medium-high** | "A site in one command" is a crowded promise. The differentiated dream is smaller and sharper: a site whose decisions are legible. Lead with the small one. |
| Perceived likelihood | **LOW - the weak dimension** | "Brief in, site out" is the exact sentence every AI site builder says, and mostly does not deliver. Claims cannot carry this. The page must SHOW the artefact - the real file tree, the real brief, the real count. |
| Time to result | **Very high** | One command, seconds. Strongest dimension and currently underused. |
| Effort and sacrifice | **Medium** | You must be comfortable in a terminal and reading JSON. Say so; do not soften it. |

**Flagged:** perceived likelihood. Every section below is written to carry evidence
rather than adjectives, because that is the dimension that will lose the reader.

### Price framing

**There is no price.** site-factory is not sold. Both framings the skill asks for are
therefore unwritable, and inventing tiers to fill the pricing block would be the
precise failure this whole codebase refuses. See §5 - the honest output of
`pricing-section-designer` here is *do not render this block*, and that is recorded
rather than papered over.

### The single biggest objection

Not "too expensive" (there is no price) and not "I don't trust AI". Precisely:

> **"Generated output is a debt. Yours will be generic, and I will spend longer fixing
> it than I would have spent starting from scratch."**

The belief underneath: a generator must average toward a default look, so what it gives
you is something to fight. The page must meet that belief directly, and the FAQ does.

### Main alternative

A page builder that hides its choices, or a template you reverse-engineer. The
difference to draw is not *better output* - it is **legible** output.

---

## 2. `revenue/skills/headline-lab`

Writes `page[1] hero-1.h1`. Register A (institutional authority): a fact, no verb
needed, no ask.

### 15 headlines, scored

Rubric: Specificity /3, Resonance /3, Clarity /2, Pull /2. Honest, not generous.

| # | Formula | Headline | S | R | C | P | Total |
|---|---|---|---|---|---|---|---|
| 1 | Flat declarative | A website factory that shows its working. | 2 | 2 | 2 | 2 | **8** |
| 2 | Before/after | One sentence in. A running site out, with every decision named. | 3 | 2 | 2 | 2 | **9** |
| 3 | Enemy-naming | Every generated site is a debt. This one comes with the receipts. | 2 | 3 | 1 | 2 | **8** |
| 4 | Mechanism-led | 35 decisions, 21 blocks, 4 packs. All of them visible. | 3 | 1 | 2 | 1 | **7** |
| 5 | Audience-named | For people who have stopped trusting starters. | 2 | 3 | 2 | 2 | **9** |
| 6 | Negative promise | It will not write the lines it cannot write. It counts them instead. | 3 | 3 | 1 | 2 | **9** |
| 7 | Question | Can you say why the spacing is 24px? | 2 | 3 | 2 | 2 | **9** |
| 8 | Big claim | The only site generator that tells you what it did not do. | 2 | 2 | 2 | 2 | **8** |
| 9 | Time-to-value | A built site in one command. The reasoning in a file next to it. | 3 | 2 | 2 | 2 | **9** |
| 10 | Category-naming | A site factory, not a page builder. | 2 | 2 | 2 | 1 | **7** |
| 11 | Contrarian | Templates hide their opinions. This one hands them to you. | 2 | 2 | 2 | 2 | **8** |
| 12 | Specificity flex | 21 block types. 21 Figma component sets. A test that fails if they drift. | 3 | 2 | 2 | 1 | **8** |
| 13 | Pain-led | The two days you spend undoing a template's opinions. | 2 | 3 | 2 | 1 | **8** |
| 14 | Proof-led | 67 tests, and every one of them was proved by breaking it first. | 3 | 1 | 2 | 1 | **7** |
| 15 | Plain fact | Every value on the page traces to a token you chose. | 3 | 2 | 2 | 1 | **8** |

Nothing scored 10, and nothing should have: the offer's weak dimension is
believability, and a headline cannot fix believability on its own. The 9s all work by
being *narrower* than the category promise, which is the correct move.

**Top 3: #6, #7, #9.**

### Refinements

**#6** - *"It will not write the lines it cannot write. It counts them instead."*
→ **"It will not write the lines it cannot write. It counts them."**
Cut "instead" - the contrast is already carried by the full stop, and three fewer
syllables lands the second beat harder.

**#7** - *"Can you say why the spacing is 24px?"*
→ **"Can you say why the spacing is 24px?"** - unchanged. Tested against
"Do you know why the spacing is 24px?" and lost: *can you say* implicates the reader in
explaining it to someone else, which is the actual sting. Left alone deliberately.

**#9** - *"A built site in one command. The reasoning in a file next to it."*
→ **"A built site in one command, and the reasoning in a file beside it."**
One sentence rather than two: the comma keeps the second half subordinate, which is
right, because the reasoning is the payload and the command is the setup. "Beside"
over "next to" for register.

### Recommended combination

- **Pre-headline** (audience-named): `For people who have stopped trusting starters.`
- **Headline** (#6, refined): `It will not write the lines it cannot write. It counts them.`
- **Subhead** (grounds the drama in specifics): `A brief in, a built site out: 21 block
  types, four style packs, every value traced to a token you chose, and every unwritten
  line listed by name.`

The three do different jobs. The pre-headline qualifies, the headline makes a claim
that sounds like a limitation and is actually the differentiator, and the subhead
supplies the concrete detail the headline deliberately withholds. Register A holds
throughout: facts stated, nothing asked for.

**`page[1] hero-1.h1` = "It will not write the lines it cannot write. It counts them."**

---

## 3. `revenue/skills/offer-stack-builder`

Writes `page[2]` - the heading, three capabilities with what each means for the reader,
and a comparison row. Eight lines.

The discipline the skill asks for: a capability is not a feature name, it is a feature
plus what it changes for the person reading. Each body line below is a consequence, not
a restatement.

**Heading:** `Three things it does that a template cannot.`

**1. Title:** `Every value traces to a token.`
**Body:** `The stylesheet contains no hex literals and no magic numbers - each one is a
var() or a multiple of the space unit you set. Change the unit and the whole page
re-spaces, because there was never a hard-coded 24px to hunt.`

**2. Title:** `It counts what it did not write.`
**Body:** `Lines it cannot honestly write are marked and totalled, then assigned to the
skill that writes them. You get a work queue with names on it, instead of finished-
looking filler you will never go back for.`

**3. Title:** `The Figma file and the code cannot drift.`
**Body:** `Every block type ships with its component set, bound to the same variable
library, and a test fails the build if one gains a type the other does not have. The
handoff stops being a promise about process.`

**Comparison row:** `rows[0].label` = `What happens when you change your mind`

That last one is the row that matters. Templates and builders both do the first
five minutes well; the difference shows on the fourth change.

---

## 4. `revenue/skills/proof-and-testimonial-engine`

Writes two quotes: `testimonials-1.items[0].quote` and `testimonials-1.featured.quote`.

**This skill returns nothing, and that is its correct output.**

site-factory has no users. There are no past buyers, no results to claim, no
credentials that bear on this. The skill's own quality bar is that results be honestly
claimable with numbers and timelines; there are none, so writing two plausible quotes
would fabricate the single most load-bearing evidence on a page whose weak dimension is
already believability. A fabricated testimonial is not a placeholder - it is a lie that
reads finished.

**Recommendation to the composer:** drop `testimonials-1` from the sitemap. Where proof
is genuinely needed (the perceived-likelihood gap the offer brief flagged), carry it
with artefacts instead: the real file tree, the real `WRITING-BRIEF.md`, the real count
of unwritten lines. Those are checkable, which is more than a quote would be.

Both lines stay marked. **2 of 20 lines deliberately unwritten.**

---

## 5. `revenue/skills/pricing-section-designer`

Writes three lines in `pricing-1`.

**Also returns nothing, for a different reason.** There is no price. §1 established that
both framings this skill produces - cost-of-inaction and per-unit - need a number that
does not exist. Two invented tiers would look ordinary and be entirely fictional.

**Recommendation:** drop `pricing-1`, or repurpose the block as the comparison table the
offer-stack row is already reaching for. The block type supports rows; the content is
a comparison, not a price.

**3 of 20 lines deliberately unwritten.** Five in total across §4 and §5 - a quarter of
the page, refused on purpose, which is the number the page's own audit will report.

---

## 6. `product/skills/objection-and-faq-engine`

Writes two question/answer pairs. The skill's rule: write from the objections that
actually stop buyers, and concede the easy thing first.

**Q1 (the objection named in §1):**
`Will I spend longer fixing the output than I would starting from scratch?`

**A1:** `Sometimes, yes - if what you want is a finished page. What comes out is a
structure with every decision named and a list of what it did not write, and if you
wanted the copy done you will still be doing the copy. What it removes is the part
where you undo someone else's opinions before you can start having your own.`

Concedes the easy thing in the first four words, then re-frames rather than argues.

**Q2 (the objection heard most, and the honest one):**
`Is this AI-generated?`

**A2:** `The suggestion step is rule-based, not a model call, and the file says so in
its own header - there is no API key wired into it. The writing is the part that goes
to a model, through skills built for it, and the pipeline marks every line it hands
over so you can see exactly which words were yours and which were not.`

Both answers carry the caveat the skill asks for. Neither claims a capability the
repo does not have.

---

## 7. `revenue/skills/cta-and-close-writer`

Writes `cta-1.heading` and, outside the skill's remit but on the same page,
`footer-a.tagline`.

The close must repeat the single claim the site makes - not summarise the page.
The single claim is: **the decisions are visible.**

**`cta-1.heading`** = `Run it once and read what it decided.`

Register A does not ask, it states a fact - but a CTA has to point somewhere, and this
one resolves the tension by making the instruction *and* the evidence the same act.
The reader is not asked to trust the claim, they are told how to check it, which is
the only close available to an offer whose weak dimension is believability.

**`footer-a.tagline`** = `A brief in, a built site out, and a list of what it would not
write for you.`

---

## What the run produced

| Skill | Lines | Written | Refused |
|---|---|---|---|
| offer-clarifier | - | offer brief (feeds all below) | - |
| headline-lab | 1 | 1 | 0 |
| offer-stack-builder | 8 | 8 | 0 |
| proof-and-testimonial-engine | 2 | 0 | **2** |
| pricing-section-designer | 3 | 0 | **3** |
| objection-and-faq-engine | 4 | 4 | 0 |
| cta-and-close-writer | 2 | 2 | 0 |
| **Total** | **20** | **15** | **5** |

Fifteen of twenty written. Five refused with a reason, and two whole blocks recommended
for removal rather than filling.

That ratio is the finding. A pipeline that reported 20 of 20 written would have
fabricated two testimonials and two price tiers, and the page would have read finished
while carrying its two most load-bearing claims as fiction. The refusals are not the
pipeline failing to do its job - on this brief they are the job.
