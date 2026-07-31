# 01 - Offer Clarifier

- **Skill:** `/home/jellytot/Documents/agents-final/skill-library/revenue/skills/offer-clarifier/SKILL.md`
- **Fed:** `/var/tmp/claude/founderos-extraction.md` (the only source; a competitive-intel extraction, not client input)
- **Assumptions made in place of client interaction:**
  - No `config.json` exists at the pack root and `sales-page-setup` has never run. Every field the config would carry (brand voice, tone words, forbidden words, awareness level) is absent. Where the workflow needs one, the substitute is derived from the extraction and flagged inline.
  - There is no client to interview. All "Inputs" questions are answered from the extraction only. Where the extraction is silent, the output says so instead of inventing.
  - Buyer awareness level assumed **solution-aware**: the audience is Bennett's 55k+ Instagram following plus people already using agent runtimes (the workspaces ship adapters for 11 coding-agent runtimes), so they know what an AI agent is; they have not necessarily seen a "business as an encoded repo" offer before.
  - This is a dry run for a hypothetical page. Nothing here is client work.

---

## 1. Raw offer inventory (extract only, no interpretation)

**The Founder OS, a paid live cohort. $1,497 early bird (full price $1,997), cohort starts August 10, 2026.**

| Element | Measured detail |
|---|---|
| Live classes | 6 classes over 2 weeks, at 1 PM ET |
| Modules | 6 |
| Checkpoints | 6 |
| Curriculum | 1. Onboard, Foundation & Fundamentals; 2-3. Encoded Workspaces I & II; 4. Encoded Tools; 5. Go-to-Market, Content & Lead Generation; 6. Automation, Heurestics & Future-Proofing |
| Workspaces | 5 named workspaces (creator-os, crm-os, finance-os, project-management-os, second-brain-os); these five are not public |
| Agents | 8-agent demo roster (Brain Librarian, Inbox Triage, Slack Scout, Payments Pulse, CRM Pulse, Social Pulse, Notion Sync, Studio Monitor) |
| Tools | 20 advertised; 25 defined in the site bundle (attio, gbrain, supabase, comms-feed, notion, stripe, slack, playwright, and 17 more) |
| Community | WhatsApp community (live invite link) |
| Related public material | Growth-Operator-Agency repo (767 files, 41 agents, 39 skills, 34 slash commands, 28 stars), YouTube-Agency (699 files, 22 agents), Clipping-Agency (338 files, ships with a real Go/Next/Python SaaS backend). Every workspace shares one anatomy: SYSTEM.md boot file, ENCODING.md 11-compartment schema, INVARIANTS.md with 28 rules, company.yaml, agents/, skills/, integrations for 11 runtimes |
| Access / license | Heuresis Source License v1.0: source-available, free for solo/learning/testing use; company use requires a commercial license from the licensor |
| Capacity | 50 presale seats, cohort cap 100; early bird closes 2026-08-01; checkout is a verified live Stripe payment link |
| Adjacent tier | Agency Accelerant, a $5,800 1-on-1 program over 8 weeks (separate funnel, same seller) |

Not in the offer (absent from the extraction): a guarantee or refund policy, a payment plan, stated class durations, any bonus items, any alumni.

## 2. The transformation

- **Before:** the founder is the integration layer of their own company. Twenty-plus tools, multiple inboxes, a CRM, payment processors, and social accounts, all reconciled by hand, by one person, whose operating knowledge lives in their head and nowhere an agent can read it.
- **After:** the company exists as an encoded, versioned workspace: a repo with a boot file, an 11-compartment schema, 28 invariants, and a roster of named agents with defined roles, scope, and escalation, runnable from whichever of 11 agent runtimes the founder already uses. Routine reading, triage, and reporting are delegated to agents; the founder reviews instead of routes.

*Flag: this transformation is written in the seller's structural language because no voice-of-customer data exists in the extraction. The skill demands buyer language; the closest available proxy is the demo agent job descriptions (e.g. "37 unread, 5 need you"), and the extraction records that those screenshots are mockups with placeholder records, so they describe intended function, not lived results. `customer-research-engine` output is a named gap.*

**One sentence:** In two weeks of live classes, your business goes from a stack of tools only you can operate to an encoded workspace that agents can read, run, and extend.

## 3. Before/after grid

| Lens | Before | After |
|---|---|---|
| Functional | Founder manually reads every inbox, checks every processor, updates every doc; knowledge is unwritten; nothing is delegatable to software | Company encoded in a repo (schema, invariants, company.yaml); named agents handle triage, monitoring, and sync across the tool stack; work is versioned and inspectable |
| Emotional | *(inferred, no VOC in extraction)* the low-grade dread of being the single point of failure; the sense that stepping away breaks things | *(inferred)* confidence that the routine layer runs and reports without them; the business is legible instead of remembered |
| Social | The team, clients, and collaborators must go through the founder for state ("what's the status of X?") | Anyone (and any agent) can query the encoded workspace; the founder answers with the system, not from memory |
| Status | "Busy founder drowning in tools" | Operator of an agent-run company; part of the early cohort of a visible movement (public repos, published paper, named method) |

*The emotional row is inference from the functional facts, not from customer quotes. The page should not present those feelings as reported.*

## 4. Ideal buyer profile

A solo founder or one-to-three-person agency operator, already technical enough to live in a coding-agent runtime (Claude Code, Cursor, or one of the other nine supported), most likely already following @bennettx.ai. Their stuck point: the business runs on roughly 20 tools and one overloaded person; they have tried automation apps and productivity systems, and believe the blocker is "not enough hours," when the actual blocker is that nothing about their company is written down in a form software can act on. In 30 days they want to say "my workspace is encoded and my first agents run"; in 90, "the routine layer of my company does not need me daily."

**Bad fit, per the measured facts:** companies (the Heuresis Source License is free only for solo/learning/testing; company use requires a commercial license, contact Syed@heuresis.ai); anyone unwilling to work in a repo and a terminal; anyone who cannot attend live classes at 1 PM ET across two weeks (no stated recording policy in the extraction); anyone shopping for finished plug-and-play software rather than a taught method, since the only conventional SaaS in the estate is Clipping OS at $99/mo, which is a separate product.

## 5. Value equation

| Dimension | Score (1-5) | Notes |
|---|---|---|
| Dream outcome | 4 | "A business that runs itself" is a top-tier desire for this buyer. Strong as stated. |
| Perceived likelihood | **2 (weakest)** | The headline marketing claims are unverifiable from the extraction ("$120k in two months" and "10x more productive" are explicitly unverified; the "98 agents" total includes an 18-agent LinkedIn repo that is a 404; the product screenshots are mockups). The page MUST NOT lean on those. What it CAN lean on is unusually strong and public: 767 files, 41 agents, 39 skills and 34 slash commands in a repo anyone can read before paying, a real shipped SaaS backend in Clipping-Agency, and a live verified checkout. The likelihood story is "inspect the goods yourself," not "trust our numbers." |
| Time to result | 4 | Six classes in two weeks with six checkpoints is a fast, concrete cadence. Cohort date is fixed (Aug 10). |
| Effort & sacrifice | 2 | Real effort: attend 6 live classes, fill company.yaml, run install scripts, encode your own business. The page must be honest that this is a build-it-with-you program, not done-for-you (done-for-you is the $5,800 tier). |

**What the page must do:** put the public repos to work as proof (they are the only verifiable evidence), and reframe the effort as the point (an encoded business you built is one you can maintain).

## 6. Price framings

1. **Against the alternative (measured):** the same seller's 1-on-1 route, Agency Accelerant, is $5,800 over 8 weeks. The cohort delivers the taught method at $1,497, roughly a quarter of the 1-on-1 price. A second honest anchor: the flagship workspace repo is free to clone for solo use under the license; what $1,497 buys is the live teaching, the six checkpoints, the five non-public workspaces, and the cohort itself, not access to markdown.
2. **Per-unit breakdown:** $1,497 across 6 live classes is about $250 per class; across the 14-day cohort, about $107 per day. Early bird saves a measured $500 against the $1,997 full price, with a real, dated close (2026-08-01).

Fit: this buyer is solution-aware and price-comparing against tools and courses, so framing 1 (alternative cost) should lead; framing 2 supports. A cost-of-inaction framing in dollars is NOT available: the extraction contains no measured cost-of-the-problem numbers, and none should be invented.

**High-ticket note:** at $1,497 this needs real transformation evidence. There are no testimonials in the extraction, so the evidence burden falls entirely on the public repos and the live cohort mechanics (see skill 04).

## 7. The single biggest objection

Not "it's too expensive." The specific belief: **"I can't verify any of this works, and part of it I could get free."** Concretely: the flashy claims are unverified, the screenshots are mockups, one advertised repo is missing, there are no alumni, and the flagship repo is free for solo use anyway. The page must answer why the cohort is worth $1,497 when the markdown is cloneable: the answer available from the facts is the live build cadence, checkpoints, the five private workspaces, the community, and the encoding of *your* business rather than a demo one.

## 8. Main alternative

**Do it themselves with the free public repos** (license permits solo/learning use), or with adjacent open-source (Canopy, 225 stars; BusinessOS, 93 stars). Difference: those give the artifact, not the method, the checkpoints, the cohort, or the five private workspaces; and none of them encode *this buyer's* company.

---

### Offer brief (one page)

- **Offer name (working):** The Founder OS, August 2026 cohort
- **Transformation:** two weeks of live classes take your business from a tool stack only you can operate to an encoded workspace agents can read, run, and extend
- **Ideal buyer:** technical solo founder / small agency operator already using an agent runtime; blocked by an unencoded business, not by hours
- **Value equation weak spot:** perceived likelihood; fix with inspectable public repos, never with the unverified revenue claims
- **Price framings:** vs $5,800 1-on-1 (lead); ~$250/class, $500 early-bird saving with a real deadline (support)
- **Biggest objection:** "unverifiable, and partly free anyway"; answer with what the cohort adds over the cloneable repo
- **Main alternative:** DIY on the free repos; different because the cohort encodes *your* company, live, with checkpoints
