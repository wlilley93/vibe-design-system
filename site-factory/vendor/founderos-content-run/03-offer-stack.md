# 03 - Offer Stack Builder

- **Skill:** `/home/jellytot/Documents/agents-final/skill-library/revenue/skills/offer-stack-builder/SKILL.md` (plus `references/offer-stack-and-bonuses.md`: four-line deliverable frame, stack sequencing, six bonus archetypes, anchor types)
- **Fed:** `/var/tmp/claude/founderos-extraction.md` + `01-offer-clarifier.md` + `02-headline-lab.md`
- **Assumptions made in place of client interaction:**
  - No `config.json`; `preferences.page_length` absent, long-form assumed (standard for a $1,497 offer).
  - The "any bonuses already planned" input: **the extraction contains no bonuses.** The skill's workflow says to design bonuses from scratch, but designing bonuses here would fabricate deliverables the offer does not contain, which the run's no-fabrication constraint forbids. Compromise honoured visibly: step 3 names the archetype gaps a real seller would need to fill, clearly labelled as recommendations for deliverables that DO NOT currently exist, with no value claims attached.
  - Payment structure: one-time payment only. A single Stripe payment link is the only checkout in the extraction; no payment plan exists to present.
  - .docx output requirement replaced by markdown per this run's contract (same note as 02).

---

## 1. Deliverable inventory (measured)

6 live classes (2 weeks, 1 PM ET) · 6 modules · 6 checkpoints · 5 workspaces (creator-os, crm-os, finance-os, project-management-os, second-brain-os) · 8-agent starter roster · integrations for a 20-tool stack (25 defined in the bundle) · 11-runtime adapters · the workspace anatomy (SYSTEM.md, ENCODING.md 11-compartment schema, INVARIANTS.md 28 rules, company.yaml, install.sh, boot.sh) · WhatsApp community · source-available license for solo use.

Absent from the offer as measured: recordings policy, support terms, guarantee, bonuses, alumni community duration.

## 2. The stack (four-line frame, sequenced core → structure → support → accelerator)

**The core: 6 live classes across two weeks (starting August 10, 1 PM ET)**
What it is: Six live sessions walking the full curriculum, from foundation through encoded workspaces, encoded tools, go-to-market, and automation.
What it does: So that you encode your actual company as the cohort runs, instead of collecting a course to watch later.
Why it matters: The alternative is the free public repo and no forcing function, which is exactly how most self-serve system-building stalls.

**The structure: the Encoded Workspace anatomy**
What it is: The schema every workspace shares: a SYSTEM.md boot file, an 11-compartment ENCODING.md, 28 written invariants, and a company.yaml you fill in once with your business's real data.
What it does: Which means your company stops living in your head and starts living in a versioned repo that any of 11 agent runtimes can read and act on.
Why it matters: This is the difference between "I use AI tools" and "my business is legible to software"; it is the specific thing no productivity app produces.

**The structure, part two: 5 workspaces**
What it is: creator-os, crm-os, finance-os, project-management-os, and second-brain-os, the five operating surfaces of the method. Unlike the agency repos, these five are not public.
What it does: So that each functional area of your business gets a ready workspace shape rather than a blank folder.
Why it matters: These are the part of the material you cannot clone from GitHub; they exist only inside the program.

**The support: 6 checkpoints + the WhatsApp community**
What it is: A checkpoint after each stage, and a live WhatsApp group alongside the cohort.
What it does: So that gaps get caught while the cohort is running, not discovered three weeks after it ends.
Why it matters: Checkpoints are the mechanism that separates "attended a cohort" from "left with a working workspace."

**The accelerator: the 8-agent starter roster + 20-tool integration set**
What it is: Eight pre-defined agents (inbox triage, CRM pulse, payments pulse, social pulse, brain librarian, Slack scout, Notion sync, studio monitor) and integration definitions across the 20-tool stack, from CRM to email to Stripe to social.
What it does: Which means your first agents are configured against real roles with defined scope and escalation, not written from scratch.
Why it matters: Empty-editor paralysis is the most common failure mode of "build your own agents"; the roster removes it. (Honesty note for the page: the site's screenshots of these agents are demo mockups; what ships is the definitions, not a hosted product.)

## 3. Bonuses

**None exist in the measured offer.** Rather than invent any, here is the archetype analysis a real seller would act on (recommendations only, not deliverables):

| Archetype gap | What would fill it | Status |
|---|---|---|
| Objection Neutralizer | Something addressing "the big claims aren't verifiable" (e.g. a recorded end-to-end build on a real business) | Does not exist; do not list on page |
| Quick Win | A first-48-hours artifact (e.g. one agent running before class 2) | Class 1 may serve this, but content per class is not measured |
| Next Step | A post-cohort maintenance protocol | Curriculum class 6 ("Future-Proofing") may partially serve this |

The page should simply not have a bonus section. An honest stack beats a padded one.

## 4. Value summary (reasoned accounting, no invented "value: $X,000" figures)

- The taught method, 1-on-1, from the same seller costs **$5,800** (Agency Accelerant, 8 weeks). The cohort is the group-format route to the same body of work.
- The public half of the material is genuinely free for solo use (Growth-Operator-Agency alone: 767 files, 41 agents, 39 skills, 34 slash commands). What the price buys is everything the clone does not include: the live classes, the checkpoints, the five private workspaces, the community, and the encoding of *your* company.
- The only comparable per-seat software in the same estate is Clipping OS at $99/mo, a single-function SaaS; the cohort's scope is the whole operating layer.

## 5. Price framing sequence (three beats, in order)

1. **The comparison.** "There are two ways to get this method with the people who built it. The 1-on-1 route is $5,800 over eight weeks. Or you build it live, in a cohort, with checkpoints."
2. **The reframe.** "The cohort is $1,497 early bird, roughly a quarter of the 1-on-1 price, about $250 per live class. And you can read the public half of the material on GitHub, all 767 files of the flagship, before you decide."
3. **The announcement.** "**The Founder OS, August 10 cohort: $1,497** (early bird, until August 1; then $1,997). One payment, via Stripe. 50 presale seats; the cohort is capped at 100."

## 6. Who this is for, and who it isn't for

**For you if:** you run a solo or very small operation; you already work in an agent runtime (Claude Code, Cursor, or any of the 11 supported); you can attend live at 1 PM ET across two weeks; and you want to build your own operating layer, not buy a finished app.

**Not for you if:** you're a company (the license is free for solo use only; company use needs a commercial license, Syed@heuresis.ai); you won't touch a repo or a terminal; you want done-for-you (that's the $5,800 1-on-1, not this); or you need hosted software with a login (the one SaaS here, Clipping OS, is a separate $99/mo product).

## 7. Full offer section copy (ready to drop in)

> **The Founder OS: August 10 cohort**
>
> Two weeks. Six live classes. You leave with your business encoded: a versioned workspace your agents can read, run, and extend.
>
> Here's everything in it:
>
> [The five stack blocks from section 2, in order]
>
> Two ways to get this method exist: 1-on-1 at $5,800 over eight weeks, or this cohort. The cohort is $1,497 early bird, about $250 per live class, and the public half of the material is on GitHub right now, all 767 files of the flagship workspace, so you can read exactly what you're buying first.
>
> **$1,497 early bird until August 1** (then $1,997). One payment. 50 presale seats, cohort capped at 100.
>
> This is for the technical solo founder who will do the work live. It is not for companies (separate license), and it is not done-for-you.
>
> **[Join the August 10 cohort]**

**Suggested CTA button label:** "Join the August 10 cohort" (scored properly in skill 07).
