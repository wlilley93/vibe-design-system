# 06 - Objection and FAQ Engine

- **Skill:** `/home/jellytot/Documents/agents-final/skill-library/product/skills/objection-and-faq-engine/SKILL.md`
- **Fed:** `/var/tmp/claude/founderos-extraction.md` + outputs 01-05
- **Assumptions made in place of client interaction:**
  - This skill belongs to a launch pack and its first-run check wants `launch-setup` config (`launch.type`, `product.*`, `voice.*`). None exists. Product fields reconstructed from the extraction; `launch.type` assumed "live cohort launch" (measured: fixed start date, early-bird window, waitlist, presale).
  - "The biggest stated objection the creator has heard": no creator to ask. Substituted with the biggest objection derived in 01 ("unverifiable, and partly free anyway"), flagged as derived, not heard.
  - DM conversations assumed part of the launch (the funnel measurably runs through Instagram, @bennettx.ai with 55k+ followers, a WhatsApp community, and a Typeform waitlist), so the DM guide is written. Assumption, not a measured launch plan.
  - Live event Q&A guide: **omitted.** The skill scopes it to "if a webinar or workshop is planned"; the extraction documents no launch webinar. The cohort classes themselves are live, but they are the product, not a launch event. Omission noted rather than a guide invented for an event that may not exist.
  - Quality-bar deviation, flagged: the time-objection answer is required to give "a specific number of hours." Class durations are not in the extraction, so the completion path below is concrete in classes, weeks, and dates but cannot state hours without fabricating them.
  - .docx requirement replaced by markdown per this run's contract.

---

## 1. Objection map (10 objections)

| # | Category | Objection, in the buyer's words |
|---|---|---|
| O1 | Price | "$1,497 is a lot for two weeks." |
| O2 | Price/Skepticism | "The repos are free for solo use. Why would I pay for what I can clone?" |
| O3 | Trust | "The site says $120k in two months and 10x productivity. Where's the evidence?" |
| O4 | Trust | "There are no testimonials anywhere. Has anyone actually done this?" |
| O5 | Trust | "The screenshots look like mockups. Is there real software behind this?" |
| O6 | Fit | "I'm not technical enough to live in a repo and a terminal." |
| O7 | Fit | "I have a small company, not a solo practice. Can I use this at work?" |
| O8 | Time | "Six live classes at 1 PM ET in two weeks? I can't make that schedule." |
| O9 | Urgency | "I'll wait for the next cohort." |
| O10 | Skepticism | "How is this different from Canopy or BusinessOS, which are open source?" |

Covers price, time, fit, trust, urgency, skepticism, plus two product-specific ones (O2, O5) outside the standard categories' usual forms.

## 2. Sales page FAQ (7 questions, buyer's language, honest answers)

**"The repos are free. What am I actually paying for?"**
Correct, and we would rather you clone the flagship first: 767 files, 41 agents, 39 skills, free for solo use. What the free path doesn't include: six live classes, six checkpoints, the five private workspaces (creator-os, crm-os, finance-os, project-management-os, second-brain-os), the community, and a fixed two-week clock aimed at *your* company, not the demo one. You're paying for the build, not the markdown.

**"Where are the testimonials?"**
There aren't any, because this is the first cohort and we won't invent them. What we can offer instead is unusual: the material itself is public, so you can read what you're buying before you pay. The results section of this page will be written by this cohort.

**"What about the '$120k' and '10x' numbers I've seen?"**
That's a fair thing to push on. Those are marketing claims we cannot hand you third-party verification for today, and this page doesn't ask you to buy on them. Buy, if you buy, on what you can check: the public repos, the curriculum, the live format, and the price.

**"Is this real software or a folder of prompts?"**
It's a method with a defined anatomy: a boot file, an 11-compartment encoding schema, 28 invariants, agent definitions with roles and escalation, and install adapters for 11 runtimes. One workspace has been taken all the way to production software (Clipping OS, a live $99/mo SaaS). The dashboard screenshots on the site are demos of the intended experience, not a hosted product you log into; what you build lives in your repo and your runtime.

**"I'm not a developer. Can I do this?"**
Honestly: this assumes you're comfortable working in a coding-agent runtime like Claude Code or Cursor. If you've never touched one, the first class covers foundations, but if a terminal is a hard no for you, this is not your program, and we'd rather say that here than after you've paid.

**"I run a company, not a solo practice. Can I use it?"**
The included license is free for solo, learning, and testing use only. Company use needs a commercial license: email Syed@heuresis.ai before enrolling, not after.

**"I can't make 1 PM ET. Should I still join?"**
The cohort is six live classes over two weeks at 1 PM ET, with a checkpoint after each stage; the live room is where your specific business gets worked on. The extraction of this offer records no recordings policy, so if you can't attend live, ask that question before you enroll. *(Seller: publish the recordings policy; this is a losable sale that one sentence would fix.)*

## 3. Objection emails

**Dedicated objection email (targets O2, the biggest one)**

Subject: **The Founder OS repos are free. So why does the cohort cost $1,497?**

The objection some of you have, and it's a good one: the flagship workspace is public. 767 files, 41 agents, 39 skills, free for solo use. If you have the discipline to encode your whole company from a cloned repo alone, genuinely, go do that; the license was written so you can.

Here's what we've noticed about self-serve systems, though, and you've probably noticed it too: they stall. Not from lack of material, from lack of a clock and a checkpoint.

The cohort is the clock. Six live classes in two weeks, starting August 10. Six checkpoints so gaps surface while we're in the room, not three weeks after. Five workspaces that aren't in the public repos. And the thing no clone gives you: the encoding of *your* business, live, with the people who built the method.

$1,497 early bird until August 1, then $1,997. The 1-on-1 version of this is $5,800.

Read the repo first if you like. Then: [Join the August 10 cohort]

**Last-day email, objection paragraph (3-4 sentences)**

If you've been circling this for a week, the hesitation is probably one of two things. If it's "can I verify this?", you can: the flagship repo is public, read it tonight. If it's "can I do this alone later?", you can try, and the material will still be free, but the cohort, the checkpoints, and the $1,497 price all close today; the next chance is the full $1,997 price, and this exact room won't reconvene.

## 4. DM conversation guide *(assumed channel; see header)*

Rules honoured throughout: answer first, product second; no manufactured pressure; the deadline is quoted only because it is real.

| Message type | Suggested response | Route |
|---|---|---|
| "Is this right for me?" | "Quick honest filter: do you already use Claude Code, Cursor, or anything like them? If yes, you're the person this was built for. If no, tell me what your stack looks like and I'll tell you straight if it fits." | Continue until the filter question is answered, then page |
| "How's this different from Canopy / BusinessOS?" | "Good pull, you've done your research. Those are open-source runtimes and platforms; this is a live two-week cohort where you encode your own company on our workspace method. Different layer. The best comparison is our own public repo, honestly, read it and see what the cohort adds." | Page, with the repo link |
| "Why is it $1,497 if the repos are free?" | "Because the repos are the material and the cohort is the build: six live classes, checkpoints, five private workspaces, your business not the demo one. If you'd genuinely do it solo, the free path is real and I won't talk you out of it." | Page only if they engage further |
| "Is there a guarantee?" | Answer with the real policy only. No refund policy exists in the measured offer, so the honest reply is "let me get you the exact terms" followed by the actual terms. Do not improvise one. | Hold until answered truthfully |
| "Can my company use it?" | "The included license is solo-use. For company use, email Syed@heuresis.ai and sort the commercial license before you buy, it'll save you pain." | Email, then page |

## 5. Live event Q&A guide

Omitted: no launch webinar or workshop is documented in the extraction. (See header note; the skill's own scoping makes this section conditional.)
