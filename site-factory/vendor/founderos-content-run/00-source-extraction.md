# Founder OS / Heuresis - Full Offering Extraction
Extracted: 2026-07-31. Sources: site HTML, JS bundles, GitHub repos, APIs.

## 1. Properties

| Property | URL | Owner | Purpose |
|---|---|---|---|
| The Founder OS (sales site) | https://www.thefounderos.com | Bennett (@bennettx.ai) | Paid cohort sales ($1,497) |
| The OS (product page) | https://www.thefounderos.com/os | same | Product/positioning page |
| Waitlist | https://www.thefounderos.com/waitlist | same | Typeform waitlist |
| Checkout | https://www.thefounderos.com/api/checkout | same | 303 -> Stripe payment link |
| Heuresis (brand site) | https://www.heuresis.ai | Syed Hussain | Brand/positioning, open library |
| Encoded Founder (paper) | https://encodedfounder.com | Syed Hussain | Thought-leadership paper (M = S x K x E) |
| Agency Accelerant | https://agencyaccelerant.ai | Bennett | 1-on-1 program funnel ($5,800) |
| Clipping OS (SaaS) | https://api.clippingos.com | Heuresis | Real backend for Clipping-Agency workspace |

## 2. GitHub repositories

### Heuresis org (github.com/Heuresis)
| Repo | URL | Stars | Size | Files | Notes |
|---|---|---|---|---|---|
| Heuresis (meta) | https://github.com/Heuresis/Heuresis | 2 | 54KB | 3 | README + hero.png only |
| Growth-Operator-Agency | https://github.com/Heuresis/Growth-Operator-Agency | 28 | 35MB | 767 | Flagship; 41 agents, 39 skills, 34 slash commands |
| YouTube-Agency | https://github.com/Heuresis/YouTube-Agency | 8 | 8.7MB | 699 | 22 agents |
| Clipping-Agency | https://github.com/Heuresis/Clipping-Agency | 3 | 6.7MB | 338 | Ships with real SaaS backend (Go/Next/Python on Railway) |
| LinkedIn-Agency | referenced but 404 | - | - | - | Never made public |

### Related third-party
| Repo | URL | Stars | Notes |
|---|---|---|---|
| Canopy (runtime) | https://github.com/Miosa-osa/canopy | 225 | "Open-source workspace protocol for AI agent systems. If OSA / Claude Code is the employee, Canopy is the office." Svelte, created 2026-03-19. Heuresis demos on it |
| Miosa-osa/BusinessOS | https://github.com/Miosa-osa/BusinessOS | 93 | "Your business, on autopilot. AI-native business OS" - adjacent product |
| Miosa-osa/foundation | https://github.com/Miosa-osa/foundation | 1 | "Component Library & Design System. Svelte" |

## 3. Direct URLs extracted from code (site bundles)

- Stripe payment link: https://buy.stripe.com/6oU3cudB5c9ydmpd3I5J60h
- WhatsApp community: https://chat.whatsapp.com/Le2Ii29mMFNESzECWfUTYD?mode=gi_t
- Typeform waitlist form: https://embed.typeform.com (form id `01KX8X39NDHCA1D28VW2N5A225`, data-tf-live)
- Calendly (Heuresis kickoff): https://calendly.com/heuresis/kickoff-60
- Clipping OS API: https://api.clippingos.com (+ /api/oauth/tiktok/callback, /api/webhooks/stripe, /internal/runs/)
- Railway internal: http://clippingos-backend.railway.internal:8080
- beehiiv API: https://api.beehiiv.com/v2 (newsletter: "cohort waitlist lands here as segments + automations")
- Graph API: https://graph.facebook.com/v20.0 (social)
- Paperclip schema: https://paperclip.dev/schema/workspace-skill.v1.json, https://paperclip.dev/schema/workspace.v1.json
- MCP spec: https://modelcontextprotocol.io
- OpenAPI: https://spec.openapis.org/oas/v3.1.0
- Medium reference: https://medium.com/@joemcmahan/the-value-equation-436ad5fe5a3a
- YouTube reference: https://www.youtube.com/@queck
- Cloudflare images: https://imagedelivery.net/IEUjvl3YUlxY-MrTpOAWDQ/...
- Emails: Syed@heuresis.ai (license/commercial), bennett via @bennettx.ai (Instagram)

## 4. Site config (from JS bundle js_939 - internal config object)

```
name: "The Founder OS"
cohortStart: "August 10, 2026"
classCount: 6, moduleCount: 6, agentCount: 8, toolCount: 20, workspaceCount: 5
checkpointCount: 6, weekCount: 2, classTime: "1 PM ET"
handle: "@bennettx.ai", followerCount: "55k+"
aaFunnelUrl: "https://agencyaccelerant.ai"
oneOnOnePrice: "$5,800"
stripePaymentLink: https://buy.stripe.com/6oU3cudB5c9ydmpd3I5J60h
checkoutPath: "/api/checkout"
presalePrice: "$1,497", presaleSeats: 50, cohortCap: 100
presaleClose: "Friday, July 24", waitlistCloseIso: "2026-07-25T23:59:59-04:00"
seatsTaken: 37 (early bird), seatsClaimed: 6
cartOpen: true
earlyBirdPrice: "$1,497", fullPrice: "$1,997", earlyBirdSaving: "$500"
earlyBirdWindowDays: 4, earlyBirdCloseIso: "2026-08-01T23:59:59-04:00"
```

## 5. Demo agent roster (8 agents, from bundle)

| Agent | Dept | Job |
|---|---|---|
| Brain Librarian | Command | Ask your business anything (1,284 notes indexed) |
| Inbox Triage | Comms | Reads all inboxes (4 inboxes, 37 unread -> 5 need you) |
| Slack Scout | Comms | Catches mentions/decisions (6 channels) |
| Payments Pulse | Finance | Today's revenue ($4,210 across 3 processors) |
| CRM Pulse | Finance | Deals moved/going cold |
| Social Pulse | Content | Platform performance (+312 followers/week) |
| Notion Sync | Knowledge | Docs auto-sync (14 pages) |
| Studio Monitor | Automations | Service health (9/9) |

## 6. Full tool stack (25 tools, from bundle js_298)

attio (CRM: Merydian + Agency Accelerant), gbrain (G-Brain CLI: hybrid search over markdown brain-store + Supabase), brain-store (local markdown KB), supabase (pgvector second brain), zeroentropy (embeddings), comms-feed (unified inbox: WhatsApp/email/Slack/calendar), zernio (social posting + analytics, 5 platforms), manychat (IG/Messenger DM automation), notion, obsidian (incl. Claude archive), miro, wispr (dictation), gmail (4 IMAP inboxes), whisper (local STT), arcads (AI UGC ads), higgsfield (AI image/video/audio), playwright (headless browser), stripe, slack, openclaw (agent runtime), ollama (local models), remotion (programmatic video), beehiiv (newsletter), typeform (waitlist/intake), canva, firecrawl (web research), vercel (hosting), github, google-calendar.

Agent models referenced: haiku-4.5 (specialist tier), plus higher-tier models for managers.

## 7. Curriculum (6 classes, from bundle)

1. Onboard, Foundation & Fundamentals
2. Encoded Workspaces I
3. Encoded Workspaces II
4. Encoded Tools
5. Go-to-Market, Content & Lead Generation
6. Automation, Heurestics & Future-Proofing (Heurestics = "a tool that maps you and your business into a self-building OS")

## 8. Repo anatomy (every workspace, same shape)

- README.md, SYSTEM.md (boot file), ENCODING.md (11-compartment schema), INVARIANTS.md (28 rules), company.yaml (buyer data, fill once)
- agents/ (one .md per agent: role, scope, authority, escalation)
- skills/ (SKILL.md + adapters/{runtime}.md + evidence/ + examples/ + variants/)
- reference/ (framework library), prompts/, workflows/, spec/, integrations/ (11 runtimes), scripts/install.sh --tool <runtime>, boot.sh, paperclip.manifest.yaml
- .claude/commands/ (34 slash commands in flagship)
- 11 runtimes: Claude Code, GitHub Copilot, Antigravity, Gemini CLI, OpenCode, Cursor, Aider, Windsurf, OpenClaw, Qwen Code, Kimi Code

## 9. Licensing

Heuresis Source License v1.0 (2026-06): proprietary, source-available. Free only for solo/learning/testing. Company use requires contacting licensor. No rebranding/reselling. DMCA enforcement. Commercial: Syed@heuresis.ai.

## 10. Clipping OS (the only real software product)

$99/mo SaaS. Stack: Go backend + Next.js 15/Tailwind v4/shadcn + Python worker (FastAPI + Redis queue) on Railway; local Postgres; Cloudflare R2; TikTok Display API OAuth (read-only); Stripe webhooks; Anthropic + OpenAI APIs. Pipeline: yt-dlp -> Whisper -> Claude scoring -> FFmpeg cuts -> 9:16 captions. Deferred: auto-posting, multi-platform, Whop integration, mobile. Brand colors from brand kit: #0B0D15 (bg), #22CC6E (green accent), #3A5FFF (blue accent), #A1A6B0 (muted), #E6E7EA (text), #08200F.

## 11. Marketing claims vs verified facts

- "$120k additional revenue in two months", "10x more productive": unverified, marketing
- "98 agents on the roster": 41 (Growth) + 22 (YouTube) + 17 (Clipping) + 18 (LinkedIn, missing) = 98 claimed; LinkedIn repo is 404
- "20 tools": 25 tools defined in bundle config
- "5 workspaces": creator-os, crm-os, finance-os, project-management-os, second-brain-os (not public); public repos are the 4 agencies
- Tool screenshots on site are mockups with placeholder records
- $1,497 Early Bird (was $1,997, saves $500), 50 seats, cohort cap 100, early bird close 2026-08-01
- 1-on-1 tier: $5,800 (Agency Accelerant, 8 weeks)
- Checkout verified: /api/checkout 303 -> Stripe payment link (confirmed live)
