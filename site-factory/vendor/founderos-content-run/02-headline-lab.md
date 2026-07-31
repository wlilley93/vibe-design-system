# 02 - Headline Lab

- **Skill:** `/home/jellytot/Documents/agents-final/skill-library/revenue/skills/headline-lab/SKILL.md` (plus `references/headline-formulas.md`, read in full for the 12 formulas and the scoring walk-through)
- **Fed:** `/var/tmp/claude/founderos-extraction.md` + `01-offer-clarifier.md`
- **Assumptions made in place of client interaction:**
  - No `config.json`: `market.awareness_level` assumed solution-aware (carried from 01); `voice.tone_words` and `voice.we_never_say` do not exist, so the "we_never_say" automatic deduction can never fire. Noted per headline instead: any headline leaning on the unverified claims ($120k, 10x) would take the "-2 promises what the offer cannot deliver" deduction, so none were written that way.
  - The skill says to run after `customer-research-engine`. That skill is not in this pack's run order and no VOC exists. The "2-3 golden pain phrases" input is therefore **absent**; pain wording below is inferred from the measured before-state (20+ tools, 4 inboxes, founder as router) and flagged as inferred, not quoted.
  - No existing headlines were supplied to grade (the extraction does not capture the site's current hero copy).
  - File output: the skill demands a .docx deliverable. This run's declared output contract is markdown into the run directory, and the SKILL.md itself states markdown is the format for passing work to the next skill in the pack, which is what happens here. Deviation noted.

---

## 1. Fifteen headlines (9 formulas represented)

| # | Formula | Headline |
|---|---|---|
| H1 | Outcome | Leave with your business encoded: a versioned workspace your agents can run, built live in six classes over two weeks. |
| H2 | Outcome | Walk out of the cohort with company.yaml filled, your invariants written, and your first agents running. |
| H3 | Pain | Every inbox, every deal, every payment still routes through one person: you. |
| H4 | Pain | Your company runs on twenty tools and one overloaded founder, and only one of those ever gets a day off. |
| H5 | Transformation | From being the API between your own tools to a repo of agents that runs the routine layer with you, in two weeks. |
| H6 | Callout | For the founder already living in Claude Code who wants the rest of the business to work the way their codebase does. |
| H7 | Callout | If your company's operating knowledge lives in your head and nowhere an agent can read it, this cohort is for you. |
| H8 | Contrarian | Stop buying tools. Your business doesn't need app number twenty-one; it needs to be encoded. |
| H9 | Question | What would Monday look like if agents had already read your inboxes, your CRM, and your books before you sat down? |
| H10 | Mechanism | The Encoded Workspace method: your company as a versioned repo, with a boot file, 28 invariants, and agents with defined roles. |
| H11 | Mechanism | Founder OS: six live classes that turn your business into a workspace any of 11 agent runtimes can operate. |
| H12 | Aggregate proof | The flagship workspace is public: 767 files, 41 agents, 39 skills. Read every line before you pay a dollar. |
| H13 | Cost of inaction | Every week the business runs through your keyboard is another week it can't run without you. |
| H14 | Clarity/simplicity | Six live classes. Two weeks. Six checkpoints. One encoded business you keep. |
| H15 | Destination | A business you can ask questions, and it answers. |

*No social-proof result headlines (formula 8's usual form) were possible: there are no verified client results in the extraction, and the reference library is explicit that invented social proof destroys credibility. H12 uses the only aggregate numbers that are real: the public repo's own stats.*

## 2. Five pre-headlines

| Type | Pre-headline |
|---|---|
| Names the audience | For technical solo founders and small agency operators |
| Names the situation | When you are the only integration layer your company has |
| Creates intrigue | Your business, as a repo |
| States a big claim | The cohort where your company becomes something agents can run |
| Names the category | A live 2-week cohort, starting August 10 |

## 3. Five subheads

| # | Subhead |
|---|---|
| S1 | Six live classes over two weeks, starting August 10. You encode your own company as you go: schema, invariants, agents, tools. |
| S2 | Built on the same workspace anatomy that is public on GitHub, so you can inspect exactly what you're learning before you enroll. |
| S3 | Works with the runtime you already use: adapters ship for 11 of them, Claude Code included. |
| S4 | Early bird is $1,497 until August 1, then $1,997. Fifty presale seats, cohort capped at 100. |
| S5 | Not done-for-you software. You build it, live, with six checkpoints so nothing gets skipped. |

## 4. Scores (honest, per the rubric: Specificity /3, Resonance /3, Clarity /2, Pull /2, max 10)

| # | Spec | Res | Clar | Pull | Deductions | Total | Note |
|---|---|---|---|---|---|---|---|
| H1 | 3 | 2 | 1 | 2 | 0 | 8 | Very specific; slightly long, "versioned workspace" costs a clarity point for the non-initiated edge of the audience |
| H2 | 3 | 1 | 1 | 1 | 0 | 6 | Insider-specific (company.yaml) to a fault; resonates only with buyers who already read the repo |
| H3 | 2 | 3 | 2 | 2 | 0 | 9 | The lived situation, named plainly; strong open loop into "so what do I do" |
| H4 | 3 | 3 | 2 | 1 | 0 | 9 | "Twenty tools and one overloaded founder" is measured and felt; the joke lands but slightly softens pull |
| H5 | 3 | 3 | 1 | 2 | 0 | 9 | "The API between your own tools" is the sharpest resonance line in the set; the sentence runs long |
| H6 | 3 | 3 | 2 | 1 | 0 | 9 | Perfect callout for the core buyer; declarative, so pull is the weak dimension (mirrors reference Sample B) |
| H7 | 3 | 3 | 2 | 1 | 0 | 9 | Situation-callout per the reference guidance; same declarative pull limit |
| H8 | 2 | 2 | 2 | 2 | 0 | 8 | Good tension; "be encoded" needs the subhead to land immediately, per contrarian formula notes |
| H9 | 3 | 3 | 2 | 2 | 0 | 10 | Specific (inboxes, CRM, books mirror the real agent roster), genuinely interesting question, instant to parse, opens a loop |
| H10 | 3 | 1 | 1 | 1 | 0 | 6 | Mechanism-rich but reads like documentation; better as a section headline, as the formula notes warn |
| H11 | 3 | 2 | 2 | 1 | 0 | 8 | Clean category + mechanism; low tension |
| H12 | 3 | 2 | 2 | 2 | 0 | 9 | The only proof-led headline that is fully defensible; "read every line before you pay" is a real differentiator |
| H13 | 2 | 3 | 2 | 2 | 0 | 9 | Honest cost-of-inaction; matter-of-fact, not scare tactic; strong closer candidate per formula 10 notes |
| H14 | 3 | 1 | 2 | 1 | 0 | 7 | All facts, no felt pain; workable support headline |
| H15 | 1 | 2 | 2 | 2 | 0 | 7 | Beautiful destination line but could sit on several adjacent products; fails the only-this-offer test on specificity |

**Top 3: H9 (10), then H3 and H12 (9s, chosen over the other 9s for complementary angles: H3 pain, H12 proof).** Nothing was rounded up; six headlines sit at 9 or better because the extraction is unusually rich in concrete nouns, which is what the specificity dimension pays for.

## 5. Refinements of the top 3

| Before | After | What changed |
|---|---|---|
| H9: What would Monday look like if agents had already read your inboxes, your CRM, and your books before you sat down? | **What if your agents had already read your inboxes, your CRM, and your books before you sat down on Monday?** | Moved "Monday" to the end so the picture completes on a concrete moment; cut "What would... look like" for directness; 2 words shorter |
| H3: Every inbox, every deal, every payment still routes through one person: you. | **Every inbox, every deal, every payment in your business routes through one person: you.** | Dropped "still" (assumed context the reader doesn't have yet); added "in your business" so the callout is unmissable |
| H12: The flagship workspace is public: 767 files, 41 agents, 39 skills. Read every line before you pay. | **767 files, 41 agents, 39 skills, all public. Read every line of what you're buying before you spend a dollar.** | Led with the numbers (the proof) instead of the label; sharpened the promise to "what you're buying" |

## 6. Recommended combination

- **Pre-headline:** For technical solo founders and small agency operators
- **Headline:** What if your agents had already read your inboxes, your CRM, and your books before you sat down on Monday?
- **Subhead:** Six live classes over two weeks, starting August 10. You encode your own company as you go, on the same workspace anatomy that is public on GitHub.

Why: the pre-headline qualifies, the headline opens the desire loop with a concrete scene, and the subhead answers it with the mechanism, the date, and the inspectable proof, with zero repetition between the three pieces.
