# E2E demo journey audit — wave 1 (2026-05-20)

Auditor: Wave-1 audit agent #1 (Claude Opus 4.7 1M)
Method: live walk via Chrome MCP at 1440x900 against `http://localhost:3000`
Source-of-truth runbook: `docs/skycot/operator/demo-runbook.md`

---

## TL;DR

The demo is **recordable with caveats** — 9 of the 11 demo-path pages render polished, on-brand UI and the headline Approval interaction (the demo's emotional money shot at 01:15) works flawlessly. **The /workflows/build "Propose in plain English" interaction is BROKEN** (centerpiece of the 04:45-06:00 voiceover beat), and **/compare has a 1-2 second flash-of-unstyled-content on first paint** that will look broken on camera. There's also a ~410-message console error storm (auth.js session fetch failing because of a Chrome-profile-disk issue) that will be horrifying if dev tools are open during recording. With the two specific fixes below the demo can record cleanly.

---

## Page-by-page findings

### 1. `/landing`

- **Page loaded?** Yes, <2s. SKYCOT light theme, brand-loud register.
- **Hero polished?** Yes. "Don't buy another tool. Hire a teammate. (Seriously.)" with Audiowide on the hero, purple→pink gradient on "teammate", "V1.0 SHIPPED · HIRING OUR FIRST 100 TEAMS" pellet, brand-correct trust pellets (NO CREDIT CARD · SOC 2 TYPE II · ISO 27001 · GDPR · HIPAA · AFSL · 2,847 TEAMS THIS MONTH).
- **Demo-script CTAs:**
  - Top-nav "How it works / Features / Compare / Customers / Pricing" — present (Features = broken, see below)
  - Top-right "Sign in" — present
  - Top-right "Hire your first worker" (purple gradient pill) — present
  - Hero "Hire your first worker" (purple gradient pill with arrow) — present, clickable
  - Hero "Watch a 90-second demo" (white outline) — present (runbook says this becomes the embedded video post-record)
  - "How it works" 4-step grid (01 Describe the job / 02 Meet your new teammate / 03 Approve their first decisions / 04 Watch the audit chain) — present, polished
  - 6-tile features grid (Named teammates not tools / Asks before it acts / Edit workflows in chat / Versioned + reversible / Multi-channel native / Audit-first enterprise-grade) — present
  - Customer logos row (Northwind Commerce, Acmeco Retail, Skyline Engineering, Apex Recruiting, Beacon Health Ops, Maple Finance) — present
  - Testimonial cards — present
  - "See all customers" CTA — present
  - Connects-to-your-stack grid (Slack, Gmail, Outlook, Zendesk, Intercom, Salesforce, HubSpot, QuickBooks, Xero, NetSuite, Greenhouse, Lever, Workday, Notion, Linear, GitHub, Stripe, Shopify) — present
  - "How do we stack up vs Zapier, n8n, Crew.ai, Lindy?" gradient banner + "See the comparison" CTA — present
  - "Your first teammate is four minutes away" final CTA — present
  - Footer (SKYCOT · COMPARE · CUSTOMERS · PRICING · GITHUB · DOCS · PRIVACY · TERMS · SECURITY · CONTACT · "© 2026 SKYCOT · MADE IN MELBOURNE · SKYCOT.COM") — present, MELBOURNE attribution correct
- **Anything broken on camera?**
  - The **worker bubbles** (Nori, Jamal, Priya, Mariah quotes) on the right side of the hero are **cramped, overlapping, and the SKYCOT logo bubble is dead-center, covering "Mariah"'s text**. Each quote is also forced into an extremely narrow ~50px column. Looks "AI-generated mock" rather than crafted.

### 2. `/auth/signup`

- **Page loaded?** Yes, instant.
- **Hero polished?** Yes — editorial-center "Hire your first AI teammate." with Audiowide + gradient. "START YOUR FREE TRIAL" pellet. "In four minutes" subtitle.
- **Demo-script CTAs:**
  - Top-right "Already have an account? Sign in" — present
  - 3 OAuth buttons: Continue with Google / GitHub / Microsoft — present **but each labeled `v1.x`** (meaning deferred from v1). On camera the operator must avoid mentioning these or scroll past.
  - "OR WITH EMAIL" divider — present
  - "Work email" field with placeholder "you@yourcompany.com" — present
  - "Send magic link" gradient CTA — present, works
- **Anything broken?** No. Form filled with `demo@skycot.com` + submit landed cleanly at `/auth/check-email`.

### 3. `/auth/check-email`

- **Page loaded?** Yes (redirected by submit).
- **Hero polished?** Yes — "MAGIC LINK SENT" pellet, "Check your inbox." hero, "We've emailed a one-tap sign-in link to demo@skycot.com. It expires in 15 minutes." subtitle.
- **CTAs:** "Resend the link" / "Didn't get the email? Check spam, or contact support." link / trust pellets SOC 2 TYPE II · GDPR · SSO READY — all present.
- **Anything broken?** No.

### 4. `/` (Approvals inbox)

- **Page loaded?** Yes, <2s.
- **Hero polished?** Yes — gradient hero "APPROVALS · LIVE / 7 decisions need you. / Your AI teammates have batched these for your review. One tap each." with WAITING 7 / RESOLVED/WK 42 / AVG RESPONSE 8m KPIs.
- **Demo-script CTAs:**
  - Sidebar nav: Approvals (active) / Team / Workers / Audit / Skills / Workflows / Cohorts / Settings — all present
  - User chip "Sam Harlow · Acmeco · Owner" bottom-left — present, on-brand
  - "+ Hire teammate" CTA — present
  - Mariah approval card matches voiceover exactly: "Mariah wants to send this listing · Slack · 2m ago / Bauer Vapor X5, used 2x · $90. Reply to buyer: 'Yes still available, $90 firm. Pickup in Auckland this weekend.' / If YES → goes to Facebook Marketplace, visible to buyer in 30s. / If NO → I'll wait for your direction. / Approve / Refuse / Why?" — works
  - Clicking **Approve** WORKS PERFECTLY — counter ticks 7→6, hero text re-renders "6 decisions need you", card morphs into green "Approved · 28s · Undo" toast. **This is the demo's emotional money shot and it lands.**
- **Anything broken on camera?**
  - Two pellets beside the H2 read **"connecting..." (with a yellow dot) and "mock (offline)" (with an amber dot)**. Both visible at 1440x900 zoom. The operator's voiceover at 01:15 will say "your inbox … 7 decisions need you", but the viewer's eye will catch "mock (offline)" right next to it. **Looks like fake/non-prod data.** Recommendation: hide both pellets for the demo, or have feature-flag for "demo mode" that hides them.
  - Bottom-right purple "?" help bubble — visible throughout. Could be dismissed before recording.

### 5. `/team`

- **Page loaded?** Yes, <2s.
- **Hero polished?** Excellent. Brand-loud gradient hero "YOUR AI WORKFORCE · LIVE / 6 teammates, one team. / 1 working right now · 1,122 actions this week · saving you ~72 hours." with WORKING 1 / ACTIONS/WK 1,122 / HOURS SAVED 72 KPIs. **Matches voiceover word-for-word.**
- **Demo-script CTAs:**
  - 6 worker cards (Mariah, Priya, Jamal, Nori, Rio, Sam) with avatar circles + status pills (Waiting on you · Working · Idle · Paused · Blocked) + TODAY summaries + TOMORROW upcoming items — present
  - Each card has "Open profile →" CTA — works, routes to `/workers/wkr_01J0MARIAH/overview`
  - "Status" filter dropdown + "+ Hire" CTA + "Filter by name…" search — present
- **Anything broken?** No.

### 6. `/workers/wkr_01J0MARIAH/overview`

- **Page loaded?** Yes, <2s. Tabs: Overview / Activity / Memory / Skills / Trust / Settings.
- **Hero polished?** Yes — Mariah avatar, name, "Waiting" status pill, role descriptor "lists and re-prices items for sale · Operations · Procurement", "3 listings live" KPI badge.
- **Demo-script CTAs:**
  - "How Mariah works" card with personality chips: **Patient · Detail-oriented · Picture-perfect listings · Polite** — matches voiceover EXACTLY
  - **Great at** column (Spotting price gaps against the live market / Cropping + ordering photos for best buyer scroll / Writing tight 60-word listing copy with real specifics / Knowing when a listing needs a category change) — matches script
  - **Escalate when** column (Price drop below the seller's floor / Photos look heavily edited or stock / Buyer asks a question outside the listing scope / Multiple competing offers on the same item) — matches script
  - **Signature behaviors** sub-section (Always confirms the seller before posting publicly / Asks for a single source photo if you give multiples / Refuses to use stock imagery — always your photos) — present
  - "What she does" descriptor card — present
  - Responsibilities toggles (List new inventory with photos + tags · as items arrive / Re-price stale listings against competitor data · daily 06:30 / Respond to buyer questions on active listings · as messages arrive / Flag items dropping below floor price · daily 06:30) — present with green-active toggles
  - "+ Add responsibility" CTA — present
  - Today's KPIs (listings live 3 +33% / buyer replies sent today 11 +6% / items flagged below floor 2 0% / median time-to-list 9m -28%) with sparkline trends — **matches voiceover 3 listings live (+33%), 11 buyer replies, 2 items flagged below floor**
  - Recent activity feed (5 timestamped rows with approved-by-Sam / auto-sent / needs-review badges) — present
- **Anything broken?** A skeleton loading-row visible at the very bottom of the page (last card never finishes loading on first paint). Minor. Probably hits a backend that errored.

### 7. `/workflows`

- **Page loaded?** Yes, <2s.
- **Hero polished?** Yes — "WORKFLOWS · LIVE / How your teammates actually think." with hero search "Search workflows by name, vertical, or step…" + "8 workflows" pill + "Browse 50+ templates" CTA + "Propose new" gradient CTA + TRY-pills (approval flows · data extraction · outbound qualification · escalation).
- **Demo-script CTAs:**
  - 8 workflow cards: post_listing (GTM-Engineer · approval required · 6 steps · by Mariah), screen_candidate (People-Ops · autonomous · 8 steps · by Priya), approve_refund (Support · 5 steps · by Jamal), extract_invoice (Finance · 4 steps · by Sam), **qualify_lead (Sales · 7 steps · by Rio)** — voiceover calls this one out, present, clickable, triage_ticket (Support · 3 steps · by Jamal), exit_interview_synth (People-Ops · 3 steps · by Priya), draft_status_report (Marketing · 4 steps · by Nori)
  - Each card has step-progress bar, version (v1-v6), age, author
- **Anything broken?** Runbook says "Catalog with 50+ templates if Track D landed; otherwise 9" — page shows 8. "Browse 50+ templates" CTA is **promissory copy** but never tested where it goes. If a reviewer clicks it expecting 50, they may hit a stub.

### 8. `/workflows/sales/qualify_lead/build`

This is the centerpiece of the 04:45-06:00 voiceover beat.

- **Page loaded?** Yes, ~3s (React Flow init).
- **Hero polished?** Yes — sticky top bar: "← ALL WORKFLOWS · SALES · qualify_lead · version v4" + "View timeline / Test run / Save + publish" CTAs.
- **Demo-script CTAs:**
  - AI prompt bar (sparkle icon · "Describe a change in plain English — e.g. 'Add a step that emails the seller before posting'" · "Propose" / "Add a step" / "Remove gate") — visible
  - **CRITICAL BUG:** **Typing the demo prompt ("Add a step that checks the lead's company size against our ICP before booking the demo") and clicking "Propose" clears the input but produces NO visible diff, no proposed change, no side panel, no toast — the canvas is unchanged.** This is the moment the voiceover script says "and it proposes the diff. You approve. It ships." On camera this will fail visibly.
  - React Flow vertical canvas (Trigger Lead from web/paid/referral → Skill Enrich+BANT score → Decision Branch on BANT score → Approval gate AE approves outreach → Tool call Send personalized outreach + calendar link) — renders cleanly
  - Click a node → side panel slides in showing "APPROVAL GATE / AE approves outreach / Slack DM with qual summary + one-tap approve. / Channel: Slack / Outreach to enterprise needs AE signoff. / Edit step CTA" — **WORKS**, this lands as the "click any node, edit it" beat
  - Mini-map + zoom controls (+/-/fit/lock) bottom-left — present
- **Anything broken on camera?**
  - **Propose-in-plain-English action is silently broken** (above) — this is the worst issue in the entire audit
  - "Add step" floating button at bottom-right is **partially clipped behind the mini-map on first paint** (only "Add st" visible). After interacting with the canvas it un-clips. Z-index/positioning bug.
  - "React Flow" attribution watermark visible bottom-right corner. Should be hidden via the React Flow `proOptions={{ hideAttribution: true }}` if you have a Pro license, or moved/styled if not.

### 9. `/compare`

- **Page loaded?** Yes, but with FOUC (flash of unstyled content). First ~1-2 seconds the entire page renders as **unstyled vertical text wall** (paragraphs run together, no checkmarks, no columns, no card layout). Then React hydrates and the proper layout snaps in.
- **Hero polished?** Once hydrated: yes. "A teammate isn't a workflow. It's the thing the workflow serves." hero, 5-card competitor strip (SKYCOT highlighted in light purple · Zapier · n8n · Crew.ai · Lindy), then a comparison matrix with green checks / red X's / orange dashes organized under section headers (TEAMMATES NOT TOOLS / SAFE BY DEFAULT / PLAIN-ENGLISH EDITING / ENTERPRISE-READY / DEPLOY HOW YOU WANT).
- **Demo-script CTAs:** Comparison matrix matches voiceover ("8 of 13 rows are clean wins"). Sections work.
- **Anything broken on camera?**
  - **The FOUC on first paint is a recording risk.** If the operator navigates to /compare during recording at 06:30 and the camera catches the unstyled state, the page looks broken. Either prerender static, or pre-load and only swap in once hydrated.

### 10. `/audit`

- **Page loaded?** Yes, <2s.
- **Hero polished?** Sharp Etherscan-style: "Audit · Forensic + compliance view" title + filter button + "Today (default)" picker + "Export" CTA + search bar.
- **Demo-script CTAs:**
  - "PII redacted — your role shows masked data. Request elevated view →" warning banner — present
  - Time-grouped activity stream (TODAY section) with rows: 5m ago · JA · resolved SUP-29382 with KB article SK-413 · act_01J0JAMAL_025 · Succeeded · $0.004; 7m ago · SA · extracted INV-[PHONE] line items · $0.012; etc. — each row has worker chip, action description, action ID, status badge, $ cost, "→" detail arrow
  - Clicking a row routes to `/audit/action/act_01J0JAMAL_025` — pop-out detail page with two-column Inputs + Provenance layout (Model claude-opus-4-7 · Prompt template generic.action · Skill version generic-skill 1.0.0 · Memory snapshot mem_synthetic · Policy version generic-policy 1.0) + "Copy hash chain" CTA + "Replay / Export" buttons + numbered "Decision chain" with 8-step trace. **Beautiful.**
- **Anything broken on camera?**
  - **Action descriptions contain raw placeholder text `INV-[PHONE]` and `QT-[PHONE]`** (e.g. "extracted INV-[PHONE] line items", "validated QT-[PHONE] against pricing rules", "flagged duplicate invoice INV-[PHONE]", "flagged 32% discount on QT-[PHONE] for Deal Desk", "queued $14,250 for CFO approval (INV-[PHONE])"). Looks like unfinished sample-data templating where `${phoneFormat()}` wasn't subbed in. Visible to camera.
  - The browser tab title is "SKYCOT — Hire your first AI teammate" instead of "Audit · SKYCOT". Same issue on /audit/action/* — title leaks from root layout.

### 11. Bonus pages walked

- **`/skills`** — marketplace page. Hero "Every skill your team will ever need." + ⌘K hero search + try-pills (approval routing · invoice extraction · candidate screening · deal qualification · support triage · lead enrichment) + 6 featured skill cards (Approval route, Invoice extract, Candidate screen, Deal qualify, Support triage, Lead enrich). Matches the locked design. Browser title also leaking.
- **`/cohorts`** — 5-tile grid (Support · Trusted, Support · Developing, Sales · Established, Marketplace · Developing, Marketing · Pre-trust) with hero "5 cohorts across your fleet" + COHORTS 5 / MEMBERS 16 / SKILLS 21 KPIs. **Note: locked design memory `project_cohorts_page_design_locked.md` says cohorts should be the "magazine tiles" pattern with 6 dept tabs (Project & Ops being the 7th) + asymmetric 1 lead + 2 stacked + 4 compact layout — current page is a simple grid.** Mismatch with the design lock. Not on the demo path so low-priority for tomorrow, but flagged for follow-up.
- **`/pricing`** — Free $0 / Pro $29 per teammate/month (MOST POPULAR) / Enterprise Custom. Polished.
- **`/customers`** — Case studies for Northwind Commerce, Acmeco Retail, Skyline Engineering, Apex Recruiting, Beacon Health Ops, Maple Finance with HRS/MO SAVED / ACTIONS/MO / SINCE-date stats. Polished.
- **`/features`** — **HARD 404** ("That page isn't here. The link may be out of date, or the page may have moved. Try one of the destinations below. [Approvals] [Workers] [Docs]"). **But `Features` is linked from the landing top-nav** — anyone clicking it from the marketing site hits a dead end. Not on the demo voiceover path but visible in nav during the landing fly-over.

---

## Critical issues (must fix before record)

1. **`/workflows/sales/qualify_lead/build` — "Propose" action is silently broken.** The plain-English-edit moment is the literal heart of the demo (the voiceover beat at ~05:30 is "And here's the magic… describe a change in plain English… and it proposes the diff. You approve. It ships."). Currently typing the prompt + clicking Propose clears the input but produces zero visible reaction (no diff modal, no canvas change, no side panel, no toast, no spinner, no error). **Fix or fake:** wire it to a stub that pops a "Proposed change" modal even if the diff is hardcoded for the demo.

2. **`/compare` — FOUC on first paint.** Hero + matrix render unstyled as a vertical text wall for ~1-2 seconds before React hydrates. Fix with prerender / `loading="eager"` / `suppressHydrationWarning` on the hero, or just navigate there well before the voiceover hits 06:30 to pre-warm.

3. **Auth.js session-fetch error storm in console.** `ClientFetchError: There was a problem with the server configuration` repeating on every page load (caused by Chrome profile leveldb running out of disk: `FILE_ERROR_NO_SPACE`). 410 messages in current session. If dev tools are open in the recording window, viewers will see red errors scrolling. **Fix:** clear Chrome's profile data (or use a fresh profile for the recording) and ensure `NEXTAUTH_URL` + `NEXTAUTH_SECRET` env vars are set so the session endpoint doesn't 5xx. (`/api/auth/session` itself returns 200 — the client-side error is from auth.js's SessionProvider, likely because the disk-full state crashed its internal cache.) **Also:** keep dev tools closed during recording regardless.

4. **`/` Approvals inbox — "connecting…" + "mock (offline)" pellets right next to the H2.** Both pellets render in the screenshot at 1440x900 and will be read by any viewer who pauses on that frame. Tells the story "this is fake data, the real product doesn't work yet." Fix: hide both in demo mode, or replace "mock (offline)" with "demo data" (much more honest framing, the operator can voice over "we're using sample data so you can see the shape — your inbox will look like this from day one").

5. **`/audit` rows show `INV-[PHONE]` / `QT-[PHONE]` raw placeholder text.** Looks like a sample-data templating fix wasn't applied. Visible to camera — viewer will think your invoicing system literally outputs `[PHONE]` placeholders. Fix: substitute real-looking IDs like `INV-39482` / `QT-29381` in the sample data fixtures.

---

## Polish issues (nice-to-have, fix when time)

- **Landing hero — worker bubbles are cramped and overlap the SKYCOT logo bubble.** Quote text forced into 50px-wide columns, looks AI-generated. Either widen each bubble or fan them out so they don't collide with the logo.
- **`/features` — hard 404 but linked from landing top-nav.** Either build the page, redirect to a real anchor on /landing, or remove from nav.
- **`/workflows/.../build` — "Add step" button clipped behind mini-map on first paint.** Z-index fight. Move mini-map up 64px or move the button left of it.
- **`/workflows/.../build` — visible "React Flow" attribution watermark.** Hide via `proOptions={{ hideAttribution: true }}` (requires React Flow Pro license; if no license, restyle position/color to be inconspicuous).
- **Browser tab `<title>` is "SKYCOT — Hire your first AI teammate" on every page.** Page-specific `<title>` not being set. Visible in browser tab during recording. At minimum set "Approvals · SKYCOT", "Team · SKYCOT", "Audit · SKYCOT", etc.
- **`/auth/signup` — OAuth buttons labeled `v1.x`.** Suggests not implemented yet. Either hide them for the demo (only show "OR WITH EMAIL" section), or remove the v1.x badge.
- **Worker-detail page — bottom card stuck in skeleton loading state.** Last "Recent activity" or related card never finishes loading on first paint. Either remove or fix the backend that errored.
- **`/cohorts` page does NOT match the locked design** (operator-locked `project_cohorts_page_design_locked.md` calls for magazine-tile asymmetric layout with 6 dept tabs; current page is a simple grid). Not on demo path, but flagged for the design-implementation track.

---

## Console + network errors summary

- **Total console messages in session:** 410
- **Total network requests captured:** 713
- **Recurring error pattern (estimated >40 occurrences across the session):**
  - `Error: IO error: …/065394.ldb: FILE_ERROR_NO_SPACE` from Chrome's leveldb (the Chrome browser profile is disk-full on the test machine — not a server-side issue)
  - `ClientFetchError: There was a problem with the server configuration` from auth.js's SessionProvider, downstream of the disk-full state
- **Server-side 5xx detected:** `GET /icon?c22e0c76a4212a40 → 503` on every page load (favicon route). Background error, won't be visible unless dev tools open.
- **Server-side 4xx detected:** `GET /features → 404` (covered above).
- **All `/api/auth/session` responses themselves were 200** — so the server-side auth handler is healthy. The client-side ClientFetchError is from auth.js's hydration trying to write a session cache into Chrome's disk-full leveldb.
- **All `_next/static/chunks/…` JS bundles loaded 200.**
- **All `_next/static/media/*.woff2` font files loaded 200.**

---

## Recommendation for tomorrow's record

Recordable with two pre-flight fixes:

1. **Stub or fake the Propose action** so clicking Propose pops a "Proposed change" modal/diff even if it's hardcoded for the qualify_lead example. (~30 min of frontend work — render a static modal with the proposed step inserted; the demo doesn't need it to actually wire to the backend.)
2. **Hide the "connecting…" + "mock (offline)" pellets** on the Approvals page, or rename "mock (offline)" → "demo data". (10 min CSS/copy change.)

Then for the recording itself:

- Fresh Chrome profile (avoids the FILE_ERROR_NO_SPACE storm)
- Dev tools CLOSED throughout (no console-error reveal)
- Pre-warm `/compare` (open it once before recording so the bundle is cached, avoiding the FOUC)
- Avoid clicking the "Features" nav link on /landing (it 404s)

The /workflows/build Propose stub is the only true blocker. Everything else can ship as-is and the operator can voiceover around the rough edges.
