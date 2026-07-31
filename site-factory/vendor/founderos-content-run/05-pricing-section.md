# 05 - Pricing Section Designer

- **Skill:** `/home/jellytot/Documents/agents-final/skill-library/revenue/skills/pricing-section-designer/SKILL.md` (plus `references/pricing-presentation.md`: five-layer justification stack, three anchoring methods, single-vs-tiered decision guide, payment plan rules)
- **Fed:** `/var/tmp/claude/founderos-extraction.md` + outputs 01-04
- **Assumptions made in place of client interaction:**
  - No `config.json`. The "guarantee terms (from guarantee-writer)" input is **absent twice over**: guarantee-writer is not in this pack's run order, and the extraction records no guarantee or refund policy anywhere. The skill's step 4 requires a guarantee reminder in the section copy; it is omitted, and the omission is flagged rather than a guarantee invented.
  - Payment plan: none exists (single Stripe payment link). Step 5 (payment plan framing) is therefore not applicable; noted, not fabricated.
  - Canva connector not connected; the layout fallback text is included per the skill's own fallback.
  - .docx requirement replaced by markdown per this run's contract.

---

## 1. Pricing structure choice

**Single price, no tiers, no payment plan**, with a real time-boxed early-bird price. Justification per the reference decision guide: this is a cohort with one clear audience, and the only "tier" in the estate ($5,800 Agency Accelerant, 1-on-1, 8 weeks) is a separate funnel with its own page, better used as an anchor than presented as a tier here; bolting it into a tier card would violate the "describe the ideal buyer for each tier in one sentence" rule, since its buyer and format differ. Manufactured tiers are explicitly warned against, and no payment plan can be presented because none exists at checkout.

The early bird is not a discount gimmick: it has a measured close date (2026-08-01), a measured delta ($500 against the $1,997 full price), and a measured capacity structure (50 presale seats, cohort cap 100). It is presented as exactly what it is.

## 2. Price justification stack (the five layers, as copy)

*(Layer 3, time value, cannot be quantified: the extraction has no measured hours-spent or hours-saved figures. That layer is carried qualitatively and flagged; no numbers invented.)*

> By the end of the two weeks you will have something specific: your company encoded as a versioned workspace, company.yaml filled with your real business, your invariants written, and your first agents defined with roles, scope, and escalation, runnable from the agent runtime you already use. **(Layer 1: the specific result)**
>
> There are two ways to build this with the people who made the method. One-on-one is $5,800 over eight weeks. The other way is this cohort. **(Layer 2: the alternative)**
>
> You could also assemble it alone from the public repos; the license genuinely allows that for solo use, and we would rather you read those 767 files than take our word for anything. What the free path does not include is the thing that gets systems finished: six live classes, six checkpoints, five private workspaces, and a fixed two-week clock. **(Layer 3, qualitative + honesty about the free alternative)**
>
> What you build is yours after the cohort ends: the workspace persists as a repo you keep, under a license that stays free for solo use. This is not a subscription you rent. **(Layer 4: ongoing value)**
>
> The full early-bird investment is $1,497. Across six live classes, that is about $250 a class; across the fourteen days of the cohort, about $107 a day. **(Layer 5: per-unit)**

## 3. Anchoring copy (before the price, honest comparisons)

1. "The 1-on-1 version of this method is $5,800. This cohort is the group-format route to the same body of work."
2. "The one piece of finished software in this estate, Clipping OS, rents at $99 a month for a single function. The cohort is a one-time price for the operating layer of the whole business, and you keep what you build."
3. *(Cost-of-inaction anchor: unavailable in dollars; the extraction contains no measured cost of the buyer's problem, and the reference forbids dishonest calculation. Handled qualitatively in the close instead.)*

## 4. The pricing section copy

> **One cohort. One price.**
>
> The 1-on-1 version of this method costs $5,800. The public half of the material is free on GitHub, and you should read it. This cohort sits exactly between those two facts: the live, checkpointed, two-week build of your own Founder OS.
>
> **The Founder OS, August 10 cohort**
> **$1,497** early bird, until August 1
> ($1,997 after that; early bird saves $500)
>
> One payment, checkout via Stripe. 50 presale seats; the cohort is capped at 100.
>
> Included: 6 live classes (1 PM ET) · 6 checkpoints · 5 private workspaces · the 8-agent starter roster · integrations across the 20-tool stack · the WhatsApp community · a workspace you keep, free for solo use under the source license.
>
> **[Join the August 10 cohort]**

*Flag for the seller, visibly: no guarantee or refund terms exist anywhere in the measured offer. A $1,497 first-run cohort with no testimonials and no stated refund policy stacks all the risk on the buyer; the honest page should state the refund policy whatever it is, and this section has a slot reserved for it under the price line. This run cannot write terms that do not exist.*

## 5. Payment plan framing

Not applicable: no payment plan exists at the measured checkout (single Stripe payment link). Per the skill's rules nothing is framed; if the seller adds one, present it alongside the full price with a descriptive label ("spread over the cohort"), never as a budget option.

## 6. Tier copy

Not applicable: single price chosen in step 1. The $5,800 program remains an anchor mention with its own funnel, not a tier card.

## 7. Canva fallback: recommended layout (text description)

A single centred pricing card, not a tier row: offer name, then the early-bird price large with the strike-through $1,997 and the dated close ("until August 1") directly beneath in small text, then a six-line included list (one line per stack item, no sub-bullets), then the CTA button, then the seat line ("50 presale seats · cohort capped at 100") in muted text. One card, one button, nothing else in the viewport; the reserved refund-policy slot sits between price and included list once terms exist.
