# Content audit — SKYCOT web/ — 2026-05-20

**Auditor:** Claude Opus 4.7 (1M context) parallel audit agent
**Voice target:** Plain-English, "teammates not tools," warm-but-confident, second-person ("you"), enterprise-credible.
**Anti-target voice:** Robotic AI-tool jargon ("automation," "agentic workflow," "AI-powered"), corporate hedging ("seamlessly," "leverage," "synergize"), passive voice.
**Surfaces audited:**
- `/landing` (867 LOC)
- `/compare` (473 LOC)
- `/skills` (371 LOC)
- `/workflows` (500 LOC)
- `/workflows/[verticalId]/[name]` (909 LOC)
- `/auth/signup` (102 LOC)
- `components/skycot/worker/WorkerPersonalityCard.tsx` (264 LOC)
- `/settings/members` (587 LOC)

---

## Voice / clarity findings (punch list)

### `/landing` — `web/app/landing/page.tsx`

- **L:36** "Audit-first, enterprise-grade." — Inconsistent capitalization across the page. Line 533 has "Audit-first" (cap-A); line 36 same. Elsewhere "audit-first" lowercase is treated as a generic descriptor. Pick one — recommend lowercase "audit-first" except when starting a sentence.
- **L:469-470** "Don't buy another tool. / Hire a teammate. / (Seriously.)" — Strong hero. The "(Seriously.)" parenthetical lands well; keep it. **No change.**
- **L:475-478** "SKYCOT compiles plain-English job descriptions into AI workers that live in Slack, learn from your feedback, and ask for approval before they act. Audit-first, single-binary, enterprise-grade." — "compiles" is dev jargon. A CEO reading this doesn't compile things. **Suggested rewrite:** "SKYCOT turns a plain-English job description into an AI teammate that lives in Slack, learns from your feedback, and asks before it acts. Audit-first. Enterprise-grade."
- **L:498** "v1.0 shipped · Hiring our first 100 teams" — Strong. Implies scarcity + momentum. **No change.**
- **L:507-518** Trust strip with "SOC 2 TYPE II", "ISO 27001", "GDPR · HIPAA · AFSL", "2,847 teams this month". **PROBLEM:** AFSL = Australian Financial Services Licence — Australian-only, won't mean anything to a US / EU reader. SOC 2 / ISO 27001 / GDPR / HIPAA are the four to lead with. Move AFSL to the security/compliance subpage. Also: "2,847 teams this month" reads as fake-precision marketing. If it's real, source it. If it's not, replace with "Hiring our first 100 teams" (matches L:498).
- **L:531-540** Bubble strings: "Mariah: 3 listings live, 2 buyers waiting on you." / "Priya: 8 candidates screened today." / "Jamal: 12 tickets resolved, median reply 4m." / "Nori: Q2 update draft ready for review." — These are perfect. They show the product working in human voice. **No change.**
- **L:69-71** HOW step "Describe the job" / "I need someone to screen inbound leads, qualify them, and book interviews." — Good. Notice the example uses "someone" not "an automation." Stay on this voice.
- **L:75-78** HOW step "Meet your new teammate" / "SKYCOT spins up a named worker (Priya, Jamal, Mariah, etc.) with the right skills, joins your Slack, and introduces themselves." — "spins up" is dev jargon. **Suggested rewrite:** "SKYCOT introduces you to a named teammate — Priya, Jamal, or Mariah — with the right skills, ready in Slack."
- **L:21-26** FEATURES "Named teammates, not tools" / "Each AI worker has a name, a personality, and a job. They show up in Slack like a colleague." — Strong. **Minor:** "Each AI worker" mixes terminology. The whole page says "teammate" — be consistent. Suggested: "Each teammate has a name, a personality, and a job. They show up in Slack like a colleague."
- **L:33-36** "Asks before it acts" / "Every risky action waits for your yes. Approval-first by default. Trust ratchets up over time." — Strong. Keep.
- **L:39-43** "Edit workflows in chat" / "Describe the change in plain English. Your teammate proposes the new TOML, you approve, it ships." — **PROBLEM:** "TOML" is a file-format acronym. 95% of CEO/operator readers don't know what TOML is. **Suggested rewrite:** "Describe the change in plain English. Your teammate proposes the update, you approve, it ships."
- **L:51-55** "Versioned + reversible" / "Every workflow change versioned like code. Side-by-side diff. One-tap rollback." — "Versioned like code" is dev jargon — but acceptable if the buyer is technical. For a plain-English read: "Every change is saved as a new version. See it side by side. One tap to roll back."
- **L:56-60** "Multi-channel native" / "Slack, iMessage, email, web inbox — all wire in the same approval flow. No extra setup." — "wire in" is dev jargon. **Suggested rewrite:** "Slack, iMessage, email, web inbox — they all share the same approval flow. No extra setup."
- **L:61-65** "Audit-first, enterprise-grade" / "Forensic-grade audit chain on every action. Capability-based RBAC. Encryption-at-rest by default." — **PROBLEM:** "RBAC" (Role-Based Access Control) is an acronym non-developers don't know. **Suggested rewrite:** "A complete record of every action your teammate takes. Granular role permissions. Encryption at rest by default."
- **L:719-720** Integration "logos" are just plain `<div>` text labels (Slack, Gmail, etc.). When real logos land, they need alt text. Today it's accessible (real text), but visually weak — flag for design.
- **L:745** "How do we stack up vs Zapier, n8n, Crew.ai, Lindy?" — Good. Direct comparison energy.
- **L:746-748** "Side-by-side, dimension-by-dimension. Spoiler: 8 of 13 rows are clean wins. Some are close. Some we've decided not to compete on." — **STRONG.** Honest. Self-aware. Keep.
- **L:849** "© 2026 SKYCOT · Made in Melbourne · skycot.com" — Good. Per memory: SKYCOT is Melbourne, not Sydney. Correct.
- **L:778** "Your first teammate is **four minutes away**." — Strong CTA. "Four minutes" appears 3 times on this page (L:54, L:478, L:778) — that's intentional repetition. Keep.

### `/compare` — `web/app/(app)/compare/page.tsx`

- **L:386** "A teammate isn't a workflow. / It's the **thing the workflow serves**." — Excellent positioning headline. Keep.
- **L:392-395** "Zapier, n8n, Crew.ai, Lindy — each one is good at part of the job. SKYCOT picks up where they leave off: named teammates that ask before they act, earn trust over time, and ship a full audit trail of every decision." — Strong. Keep.
- **L:121** "Crew.ai · Multi-agent framework for developers. Code-first, no operator surface." — Honest, accurate. Good.
- **L:124** "Lindy · AI assistant builder. Strong UX, light on multi-channel + audit." — Crisp. Keep.
- **L:140-142** "If 8+ of those rows landed for you, this is your team." — **GREAT.** Self-confident. Doesn't apologize.
- **L:48** "Do they feel like real teammates (named, with personality + responsibilities)?" — Slightly informal "+" — better as "and." Suggested: "Do they feel like real teammates — with names, personalities, and clear responsibilities?"
- **L:54** "Do they show up in Slack like a colleague would?" — Keep. Good.
- **L:60** "Do they earn trust over time + get promoted?" — Same "+" → "and" nit. Suggested: "Do they earn trust over time and get promoted?"
- **L:66** "Do they ask for approval before doing risky things?" — Plain. Strong.
- **L:78** "Can you edit a workflow by chatting in plain English?" — Good.
- **L:99** "Multi-tenant by construction (not bolted on)?" — "Multi-tenant" is dev jargon. **Suggested rewrite:** "Built for multiple teams from day one (not bolted on)?"
- **L:105** "Capability-based RBAC across every action?" — Same RBAC issue. **Suggested rewrite:** "Granular role-and-permission control on every action?"
- **L:111** "Encryption-at-rest with operator-controlled keys?" — Technical, but enterprise buyers expect this language. Keep.
- **L:117** "Single binary you can self-host?" — Audience-correct (the technical buyer's question). Keep.
- **L:118** "Air-gapped deploys for regulated environments?" — Good.

### `/skills` — `web/app/(app)/skills/page.tsx`

- **L:319-321** "Every skill your team / will ever **need**." — Strong. The "ever" is bold; lean into it.
- **L:323-325** "Drop in a pre-built skill — approval routing, invoice extraction, candidate screening, and dozens more — and your AI teammates start using it the same shift." — **GREAT** voice. "The same shift" is human language. Keep.
- **L:31** "Route approvals to the right human based on risk, dollar, and seniority rules." — Strong, plain.
- **L:38** "Pull structured line items from PDF or scanned invoices with vendor reconciliation." — Good.
- **L:45** "First-pass candidate screen against a JD with skill-match scoring and red-flag flags." — **PROBLEM:** "red-flag flags" is awkward redundancy. **Suggested rewrite:** "First-pass candidate screen against a JD — skill-match scoring, plus flags for any red flags." Or just: "First-pass candidate screen against a JD with skill-match scoring."
- **L:52** "BANT-style outbound qualification, calendar-link insertion, CRM update on accept." — **PROBLEM:** "BANT" (Budget Authority Need Timeline) is sales jargon — half the buyer audience won't know it. **Suggested rewrite:** "Outbound qualification using budget/authority/need/timeline scoring, calendar-link insertion, CRM update on accept." Or just: "Outbound lead qualification with calendar booking + CRM updates."
- **L:59** "Classify tickets, deflect with KB articles, escalate the rest with full context bundle." — "KB" is fine (Knowledge Base, widely understood). Keep.
- **L:66** "Append firmographic + technographic context to inbound leads from public sources." — **PROBLEM:** "firmographic + technographic" is B2B-sales jargon. CEO outside that world won't parse it. **Suggested rewrite:** "Append company + tech-stack details to inbound leads, sourced from public data."
- **L:330** "Search skills, cohorts, or workers…" — Good. Plain.
- **L:333** "132 total" — Real count? Or placeholder? If placeholder, mark clearly. Demo recordings expose fake numbers.

### `/workflows` — `web/app/(app)/workflows/page.tsx`

- **L:417** "How your teammates / actually **think**." — **GREAT.** Doesn't say "workflows" in the H1, even though it's the workflows page. Sells the meaning, not the noun. Keep.
- **L:421-423** "Workflows are the plain-English step-by-step playbooks your AI teammates follow. Edit them in chat, version them like code, ship to production in one click." — **PROBLEM:** "ship to production in one click" assumes dev vocabulary. **Suggested rewrite:** "Workflows are the step-by-step playbooks your teammates follow. Edit them in chat. Roll back any version with one click."
- **L:447** "Propose new" button — **PROBLEM:** "Propose" is the gateway's verb (skill name: `propose_workflow_from_plain_english`). For an operator UI button, "New workflow" or "Add workflow" is the obvious-action CTA. **Suggested rewrite:** Button text = "New workflow".
- **L:451** "TRY · approval flows · data extraction · outbound qualification · escalation" — Good chip set. Keep.
- **L:35** SEED workflow "approve_refund" appears TWICE in the seed list (lines 32-40 and 76-84). Both are "support / approve_refund" with different lastEdited values. **Bug:** Duplicate entry in mock data. Fix the seed.
- **L:106** RISK_LABEL "approval-required" / "Autonomous" — Good. Plain.
- **L:481** "v3" / "{wf.versions} steps" — `wf.versions` in the Clock context says "3 days ago" but the meta line shows "v3". Make sure version + edited semantics are clear: version is the version number, edited is the date. Currently visually conflated.

### `/workflows/[verticalId]/[name]` — `web/app/(app)/workflows/[verticalId]/[name]/page.tsx`

- **L:66** "Median resolution under 4 minutes." — Good claim.
- **L:73** "Skill `support.classify` tags severity (P0..P3) + intent (refund / how-to / bug / billing)." — "P0..P3" is dev jargon for severity. For an operator viewer, **suggested rewrite:** "Skill `support.classify` tags how urgent (critical / high / medium / low) and what (refund / how-to / bug / billing) the ticket is about."
- **L:103** "Skill `lead.enrich` pulls firmographic + technographic data; BANT-scores 0-100 across Budget, Authority, Need, Timeline." — Same as `/skills` — BANT is sales jargon. Here it's slightly more acceptable because the user has clicked into a sales workflow, so the audience self-selected. Borderline keep.
- **L:147** "vendor portal upload." — Plain. Good.
- **L:170** "BrowserExecutor opens FB, fills the form, uploads photos, submits." — **PROBLEM:** "BrowserExecutor" is the internal class name. Customers don't need to know that. **Suggested rewrite:** "Your teammate opens Facebook Marketplace in a browser, fills the form, uploads photos, and submits."
- **L:218** "Skill `feedback.synthesize` finds themes (manager, compensation, growth, culture) without naming individuals." — Strong privacy reassurance. Keep.
- **L:248** "Skill `narrative.compose` writes the report in the client's voice (length, tone, vocabulary all match prior drafts)." — Good.

### `/auth/signup` — `web/app/auth/signup/page.tsx`

- **L:50** "Hire your first / **AI teammate**." — Strong + matches the landing CTA. Keep.
- **L:53-55** "In four minutes, you'll set up your workspace, link a provider, and have a teammate working alongside you. No credit card required." — **GREAT.** Removes friction. The emphasis on "four minutes" + "no credit card" lowers the activation barrier. Keep.
- **L:57** AuthSsoButtons — Already documented as v1.x placeholders. Make sure their disabled state is visually obvious — operators will assume "Google" works.
- **L:88** "{pending ? 'Sending magic link…' : 'Send magic link'}" — Clear. Good.
- **L:97** "We'll email you a one-tap sign-in link. No password to remember." — **GREAT.** Removes anxiety about "another password."

### `components/skycot/worker/WorkerPersonalityCard.tsx`

- **L:35-37** Mariah traits: "Patient", "Detail-oriented", "Picture-perfect listings", "Polite" — "Picture-perfect listings" is a job description, not a trait. **Suggested:** Replace with "Perfectionist" or "Visual" or "Precise."
- **L:39-44** Mariah greatAt — All in second-person/observer voice. Strong. Keep.
- **L:47-50** Mariah escalateWhen — "Photos look heavily edited or stock" / "Buyer asks a question outside the listing scope" / "Multiple competing offers on the same item" — Clear escalation signals. Good.
- **L:74-77** Priya signature: "Always sends the rejection within 48h of decline" / "Books a buffer week before a senior interview" / "Asks for clarification before discounting a strong yes" — **GREAT.** These read like a real teammate's signature moves. Sells the "teammate" frame harder than any marketing copy.
- **L:80** Jamal: "Calm under pressure", "Empathic", "Quick", "Honest" — "Empathic" should be "Empathetic" (US/UK both prefer it for adjective meaning "feeling empathy"; "empathic" is mostly clinical). **Suggested:** "Empathetic".
- **L:88-91** Jamal escalateWhen — "Customer mentions churn or 'cancel' in any reply" / "Same customer opens 3+ tickets in 24h" / "Issue involves billing or a refund over $200" / "Customer is on a VIP or enterprise tier" — Clear. Operationally useful. Good.
- **L:97** Default personality: "Confidence in the next action is below 80%" — A number escalation rule is brittle as a placeholder. **Suggested:** "When the teammate isn't sure what to do next" (plain-English version).
- **L:209** Section label "HOW {name.toUpperCase()} WORKS" — Good. "HOW MARIAH WORKS" is warmer than "WORKER OVERVIEW."

### `/settings/members` — `web/app/(app)/settings/members/page.tsx`

- **L:445** "Team & roles" page title — Good.
- **L:446-448** "Invite humans onto your workspace. Workers (AI teammates) live on the Team page." — **GREAT clarification.** Disambiguates human-vs-AI in one sentence. Keep.
- **L:81-85** ROLE_DESCRIPTIONS.owner: "Everything an Admin can do" — "Admin" capitalized inconsistently. Below, "Owner" is title-cased. Lowercase role names throughout, OR title-case throughout. Pick one.
- **L:91-97** ROLE_DESCRIPTIONS.admin.can: "Hire / pause / archive workers" / "Edit workflows + skills" / "Invite new members" / "Approve actions on any worker" / "View full audit log" — Plain, clear. Good.
- **L:99-105** ROLE_DESCRIPTIONS.user — "Day-to-day operator. Uses workers + approves their decisions." — Good. The "uses workers" framing is human-natural.
- **L:107-113** ROLE_DESCRIPTIONS.guest: "Read-only. Good for auditors, observers, prospective hires." — **GREAT.** The "prospective hires" use case is unexpected + clever sales hook. Keep.
- **L:471** "Invite a teammate" modal title — Good.
- **L:472** "They'll get a magic-link email and land directly on your workspace." — Plain. Good.
- **L:18** SeedMember `lastSeen: "online now"` — Real-time freshness in a demo seed is good, but if the demo recording captures "online now" and the screen lingers, it's a tell. Consider "Active just now" or include a relative-time clock.
- **L:11-65** Mock members: "Sam Harlow", "Olivia Chen", "Diego Alvarez", "Theo Park", "Naomi Wright", "Aroha Patel" — Diverse, plausible. Good.
- **L:36** "naomi@auditor.gov" — Government domain in mock data is a smart enterprise signal but could read as fake. Consider `naomi@external-auditor.com` to soften.

---

## Cross-page consistency issues

### Voice consistency

- "**Teammates**" is the canonical term and appears 40+ times. "**Workers**" appears almost as often (the gateway entity name). The product story is "teammates." When talking ABOUT the entity in marketing/UI surfaces, use "teammates." Use "workers" only where the technical noun matters (API responses, internal admin views, capability docs). Today the mix is unintentional in places — e.g. `/skills` line 30 says "AI teammates" but the workflow card on `/workflows` uses "lastEditor" + worker names. That's fine. The cross-page sweep: **on `/landing`, replace all standalone "AI worker(s)" with "AI teammate(s)" or "teammate(s)".**

### Acronym discipline (operator is not a developer)

Per the operator's standing instructions, every dev acronym should expand on first use. Currently violated:

- "TOML" — `/landing` L:43, `/workflows/[v]/[n]` extensively. Replace with "workflow file" or "config" in customer-facing copy.
- "RBAC" — `/landing` L:66, `/compare` L:105. Replace with "role and permission control."
- "BANT" — `/skills` L:52, `/workflows/[v]/[n]` L:103. Expand to "budget/authority/need/timeline" or replace.
- "JD" — `/skills` L:45, `/workflows/[v]/[n]` L:103. "JD" is widely understood in HR contexts (Job Description), so context-dependent — keep on people-ops pages, expand on the landing page.
- "KB" — `/skills` L:59. Keep (knowledge base, widely understood).
- "AFSL" — `/landing` L:516. Australian-only. Drop from landing.
- "SOC 2" / "ISO 27001" / "GDPR" / "HIPAA" — All keep. Compliance acronyms are expected.
- "P0..P3" — `/workflows/[v]/[n]` L:73. Customer-facing severity should be plain ("critical/high/medium/low").

### CTA clarity

Each CTA on each surface should make the next action OBVIOUS without context. Today:

- **Landing top-right pill "Hire your first worker"** — Should be "Hire your first teammate" (per voice rule).
- **`/workflows` button "Propose new"** — Operators don't propose; they create. Change to "New workflow."
- **`/compare` final CTA "Hire your first teammate →"** — Good.
- **Signup submit "Send magic link"** — Good.
- **`/settings/members` "Invite teammate"** — Good.

### Internationalization risks (en-US idioms)

We serve global. Today:

- "**Spoiler:**" (`/landing` L:746) — Pop-culture en-US idiom. Most readers understand it, but if SKYCOT lands in a translation pipeline this is a non-trivial idiom. Consider "**Heads-up:**" or "**Quick note:**".
- "**One-tap**" everywhere — Mostly works globally. Some markets prefer "one-click" — but "tap" reads mobile-native, so keep.
- "**Buffer week**" (`WorkerPersonalityCard` Priya signature) — US business idiom for "slack time before something important." Borderline. Replace with "extra time" if explicit.
- "**ship to production**" (`/workflows` L:423) — Dev idiom. Already flagged for plain-English rewrite.
- "**stack up**" (`/landing` L:745, "How do we stack up vs Zapier...") — Borderline informal. Acceptable on a marketing page.
- Currency display: all `$` symbols are USD. When SKYCOT sells in EUR / GBP / AUD, this needs locale-aware formatting. Not a v1.0 demo blocker; flag for v1.1.

### Typos / grammar

- **L:80 `WorkerPersonalityCard`** — "Empathic" → "Empathetic" (style preference, not strict error).
- **L:35 `WorkerPersonalityCard`** — "Picture-perfect listings" as a trait is a noun-phrase, not an adjective. Replace.
- **L:45 `/skills`** — "red-flag flags" is awkward redundancy. Fix.
- **L:34 `/workflows`** — Duplicate "approve_refund" entry in `SEED_WORKFLOWS`. Bug.
- No spelling errors found in the audited files.

---

## Differentiation check

The brand promise (from `CLAUDE.md` / memory): "AI teammates that ask before they act, earn trust, ship an audit trail." Audit pages against competitor copy patterns:

- **Zapier voice** — "Connect your apps. Automate workflows." SKYCOT avoids this. ✓
- **n8n voice** — "Build powerful workflows fast." SKYCOT avoids this. ✓
- **Lindy voice** — "Build AI agents." SKYCOT calls them "teammates," not "agents." ✓
- **Crew.ai voice** — Developer-focused, code-first. SKYCOT is operator-first. ✓
- **Generic AI tool voice** — "AI-powered," "intelligent automation," "agentic workflows." SKYCOT uses NONE of these terms. ✓

The pages I audited reliably stay in the "teammate" frame. The biggest voice slips are dev-jargon spillover (TOML, RBAC, BANT, BrowserExecutor) — not competitor-voice contamination. That's a great place to be.

---

## Top 3 content priorities for the demo MP4

1. **Strip dev jargon from `/landing` and `/workflows`**. Specifically: drop "TOML", "RBAC", "compile", "ship to production", "BrowserExecutor". Each is a 5-minute fix. The demo viewer is your buyer's CEO — they should never need to ask "what does RBAC mean?"

2. **Fix the duplicate `approve_refund` seed in `/workflows`**. Real bug — both rows render in the demo, visible side by side.

3. **Standardize "teammates" vs "workers"** on `/landing`. Both terms appear, but "teammates" is the brand-frame and "workers" is the gateway-frame. On any customer-facing surface, "teammates" wins. One global find-replace in `app/landing/page.tsx`.

Optional 4th if there's time: **replace "2,847 teams this month"** with "Hiring our first 100 teams" (matches the hero eyebrow at L:498, removes fake-precision smell).
