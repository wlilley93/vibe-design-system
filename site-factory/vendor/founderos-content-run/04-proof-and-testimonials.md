# 04 - Proof and Testimonial Engine

- **Skill:** `/home/jellytot/Documents/agents-final/skill-library/revenue/skills/proof-and-testimonial-engine/SKILL.md` (plus `references/proof-types-and-formatting.md`, read in full: seven proof types, believable-testimonial formula, thin-proof situations)
- **Fed:** `/var/tmp/claude/founderos-extraction.md` + outputs 01-03
- **Assumptions made in place of client interaction:**
  - No `config.json`; `proof.has_testimonials` / `proof.key_results` / `proof.credentials` do not exist and were reconstructed from the extraction.
  - The skill says to run after `sales-page-architect` has defined which sections need proof. That skill is not in this run order; a standard long-form anatomy is assumed (hero → problem → mechanism → offer → price → FAQ → close) and proof is assigned against it.
  - There is no client to send follow-up questions to, so the reference library's "extract the five elements by asking" step cannot run; the thin-proof path is used instead.
  - Notion and Canva connectors: not connected. Per the skill's own fallbacks, output stays in this document and a text description of the recommended callout-card layout is included at the end.
  - .docx requirement replaced by markdown per this run's contract.

---

## 1. Proof audit

| Bucket | Contents | Verdict |
|---|---|---|
| (A) Before/after testimonials | **None exist in the extraction.** | Absent. Nothing may be written here; inventing one is forbidden by this skill itself. |
| (B) Feeling/experience testimonials | **None.** | Absent. |
| (C) Credentials & background | Bennett: @bennettx.ai, 55k+ followers; runs a second, higher-tier program ($5,800 Agency Accelerant). Syed Hussain: Heuresis brand, author of the Encoded Founder paper (M = S x K x E), licensor of the source license. The estate ships one real revenue-bearing SaaS: Clipping OS, $99/mo, Go/Next/Python on Railway with Stripe webhooks and a working TikTok OAuth pipeline. | Usable. The strongest credential is shipped software, not follower count. |
| (D) Raw data & numbers | Public repos: Growth-Operator-Agency (767 files, 41 agents, 39 skills, 34 slash commands, 28 stars), YouTube-Agency (699 files, 22 agents, 8 stars), Clipping-Agency (338 files, ships a real SaaS backend, 3 stars). Live checkout verified (303 → Stripe). 37 early-bird seats recorded as taken in the site's own config. Third party: the Canopy runtime Heuresis demos on has 225 stars (someone else's project; attribute if used). | Usable with care. Star counts are modest; do not dress them up. |
| **Unusable** | "$120k additional revenue in two months" and "10x more productive" (explicitly unverified marketing); "98 agents" (the count includes an 18-agent LinkedIn repo that is a 404; the defensible public number is 63 agents across the three live agency repos, 41+22 plus Clipping's set); product screenshots (mockups with placeholder records). | **Must not appear on the page as proof.** Each fails the "if a claim cannot be defended, it is not on the page" bar. |

**This is thin-proof Situation 1 (new offer, no testimonials) blended with Situation 4 (data without stories).** The reference's order of preference applies: real aggregate data first, credentials as proxy, a defensible confidence statement, and honesty about newness.

## 2. Proof assigned to moments

| Proof | Type (of the seven) | Page moment | Doubt it resolves |
|---|---|---|---|
| "767 files, 41 agents, 39 skills, all public" | Result callout (repo data) | Hero / trust bar | "Is there anything real behind this?" |
| Repo anatomy walkthrough (SYSTEM.md, 28 invariants, 11 runtimes) | Aggregate/inspection proof | Mechanism section | "Is this just prompt packs?" |
| Clipping OS mini-story | Mini case study (seller's own build) | Just before the offer | "Do these people actually ship?" |
| Bennett + Syed credentials | Credentials | Just before the price (warm audience placement per reference) | "Why trust them with $1,497?" |
| Confidence statement | Confidence statement | Where testimonials would live | "Nobody has done this before me" |
| "First cohort" honesty line | Newness disclosure | FAQ / close | Pre-empts the missing-alumni discovery |

## 3. Testimonials

None can be written. There are no raw quotes to polish, and the formula's five elements cannot be sourced. **The page should carry no testimonial section at all for cohort one**, and collect five-element testimonials from this cohort (the reference's Situation 1, option 1: treat cohort one as the beta that produces them).

## 4. Mini case study (the only story the facts support: the seller's own shipped product)

> **The proof they shipped: Clipping OS**
>
> Before this was a cohort, the same team encoded one of their own workspaces all the way into production software. Clipping-Agency is the public workspace; behind it runs Clipping OS, a live $99/month SaaS: a Go backend, a Next.js front end, and a Python worker on Railway, with Stripe billing and a TikTok integration, turning long video into scored, captioned clips automatically.
>
> That is the method at full depth: workspace first, then agents, then working software. The cohort teaches the same path, starting from your business instead of theirs.

(196 words with title; every technical claim is in the extraction. Note the honest scope: this is seller proof, not client proof, and the page should not present it otherwise.)

## 5. Thin proof handled ethically: the confidence statement

> Here is what we can show you instead of testimonials, because this is the first cohort and we won't invent any: the material itself. The flagship workspace is public on GitHub, 767 files, 41 agents, 39 skills, 34 commands, under a license that lets you use it solo for free. Read all of it before you pay. If the method were empty, you would be able to tell from your own screen.

Every clause is defensible from the extraction. Note what this statement deliberately does not say: it makes no result claim ("clients typically report X" is unavailable, there being no clients yet).

## 6. Proof section copy (assembled)

> **No testimonials. On purpose.**
>
> This is cohort one. We could paper this section with vague praise; instead, here is proof you can check yourself:
>
> [Result callout: **767** files · **41** agents · **39** skills, in one public repo]
>
> [Confidence statement, section 5]
>
> [Mini case study: "The proof they shipped: Clipping OS", section 4]
>
> And that's the honest inventory. The rest gets proven live, starting August 10.

Framing sentence in, transition out, per the quality bar.

## 7. Proof placement map

| # | Element | Type | Location | Objection addressed |
|---|---|---|---|---|
| 1 | Repo numbers trust bar | Result callout | Hero | "anything real here?" |
| 2 | Anatomy walkthrough | Inspection | Mechanism | "just prompt packs?" |
| 3 | No-testimonials framing + confidence statement | Confidence | Proof section | "too good to be true" |
| 4 | Clipping OS story | Mini case study | Before offer | "do they ship?" |
| 5 | Credentials (Bennett 55k+, Syed/paper) | Credentials | Before price | "why these people?" |
| 6 | First-cohort disclosure | Newness | FAQ + close | "where are the alumni?" |

## 8. Canva fallback: recommended callout card layout (text description)

One horizontal trust bar directly under the hero subhead: three number blocks (767 files / 41 agents / 39 skills), number in large weight, one-line caption beneath each, all three linking to the public repo; monochrome, no decoration, styled identically so readers learn them as measured facts. Maximum three such callouts on the whole page, per the reference's formatting rule.
