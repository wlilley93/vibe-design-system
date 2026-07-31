# 07 - CTA and Close Writer

- **Skill:** `/home/jellytot/Documents/agents-final/skill-library/revenue/skills/cta-and-close-writer/SKILL.md` (plus `references/cta-and-close-patterns.md`, read in full: CTA bank + rubric, six-component close, ethical urgency guide, two-path formula)
- **Fed:** `/var/tmp/claude/founderos-extraction.md` + outputs 01-06
- **Assumptions made in place of client interaction:**
  - Guarantee terms (input "from guarantee-writer"): **absent**. guarantee-writer is not in this run order and no guarantee exists in the extraction. The guarantee-callback component of the close is omitted (it is one of the optional three; the required three are all present) and the gap is flagged to the seller.
  - Urgency inputs are real and measured: early bird $1,497 closes 2026-08-01 with a stated rise to $1,997; the cohort starts August 10; 50 presale seats, cap 100. The bundle config also records `seatsTaken: 37`; that is the site's own counter, not independently verified, so the close quotes the seat *structure* (50/100) as fact and treats the live count as the seller's number to display, not this copy's claim.
  - The two-path contrast is written at the 90-day horizon the skill specifies, with the honest caveat that no measured client outcomes exist; Path A is therefore written as what the buyer will have *built and be running* (structural, defensible), not what they will have *earned* (unmeasurable).
  - .docx requirement replaced by markdown per this run's contract.

---

## 1. CTA button variants, scored (Specificity /3, Benefit direction /3, Active voice /2; 7-8 excellent)

| # | Button copy | Spec | Benefit | Active | Total | Note |
|---|---|---|---|---|---|---|
| C1 | **Join the August 10 cohort** | 3 | 2 | 2 | **7** | Names the exact thing and its date; benefit implicit rather than stated |
| C2 | **Start encoding my business** | 2 | 3 | 2 | **7** | Names the transformation verb itself; slightly less concrete about what happens next |
| C3 | Save my seat for $1,497 | 3 | 1 | 2 | 6 | Very concrete, but names the payment, not the outcome |
| C4 | Get the Founder OS + all 5 workspaces | 3 | 2 | 1 | 6 | Specific to the stack; "get" is flat as a decision verb |
| C5 | Enroll now | 1 | 0 | 2 | 3 | The floor from the generic bank; kept for contrast, rewrite threshold per rubric |

**Top 2: C1 and C2.** Recommended deployment: C1 everywhere the date and logistics are nearby (pricing card, close); C2 in the hero where the transformation is the frame.

## 2. Close structure (which of the six components, in order)

1. Transformation restatement (required)
2. Two-path contrast
3. Cost of inaction (required)
4. Price and options, final time
5. ~~Guarantee callback~~ omitted: no guarantee exists to call back; seller flag repeated
6. Final CTA (required)

## 3. Two-path contrast

> Ninety days from now, one of two things is true.
>
> You did the two weeks. Your company lives in a repo now: schema written, 28 invariants set, company.yaml filled with your actual business. The agents you defined in the cohort read your inboxes, your CRM, and your books through your own stack, and Monday starts with you reviewing their output instead of producing it. When something breaks, you read the workspace, because there is a workspace to read.
>
> Or it is ninety days from now and the operating layer of your company is still you. Same twenty tools, same four inboxes, same routing of everything through one keyboard, plus one more quarter of it. Nothing got worse, exactly. Nothing got legible either.
>
> Both paths start from the same place. One of them starts August 10.

## 4. Cost of inaction

> Here is the honest accounting, without invented numbers: the extraction of your working week is the cost. Every process that lives only in your head is a process only you can run, which means the ceiling on the business is your calendar, and staying the router is a decision that renews itself silently every week you don't replace it. We can't tell you what that costs you in dollars; you can, and you already have a number in mind or you wouldn't have read this far.

*(Deliberately unquantified: the extraction contains no measured cost figures for the buyer's problem, and the reference forbids dishonest math. The buyer-supplies-the-number move is the defensible form.)*

## 5. The full close

> **You've seen the whole thing. Here's the decision.**
>
> By the end of the two weeks you will have your business encoded: a versioned workspace with your schema, your invariants, and your first agents defined and running from the runtime you already use. That is what this cohort exists to leave you with; everything in it serves that.
>
> [Two-path contrast, section 3]
>
> [Cost of inaction, section 4]
>
> **The Founder OS, August 10 cohort: $1,497** early bird until August 1, then $1,997. One payment, via Stripe. 50 presale seats; the cohort is capped at 100.
>
> **[Join the August 10 cohort]**
>
> Checkout goes straight to Stripe. Class one is at 1 PM ET on August 10, and the encoding starts there.

## 6. Urgency and scarcity copy (all of it real, per the ethical-urgency guide)

> Three dates and one number, all of them literal:
>
> **August 1** is when the early-bird price ends. It's $1,497 now and $1,997 after; the $500 difference is stated on the checkout, not conjured by a countdown.
>
> **August 10** is when the cohort starts, at 1 PM ET. This isn't an evergreen page; the room convenes once, on that date.
>
> **50 presale seats** exist, inside a cohort capped at **100**, because this is a live, checkpointed build, not a broadcast.
>
> If those constraints don't move you, they shouldn't; join because the two weeks are worth it. But they are real, which is why they're the only urgency on this page.

Every claim maps to a measured config value (earlyBirdCloseIso, fullPrice, cohortStart, classTime, presaleSeats, cohortCap). No fake scarcity patterns used: no timers, no unexplained "spots left", no undated "soon".
