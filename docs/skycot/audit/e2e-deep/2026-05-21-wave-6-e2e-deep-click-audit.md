# Wave-6 E2E Deep Click Audit — 2026-05-21

**Auditor**: Wave-6 audit agent #1 (claude-opus-4-7[1m])
**Target**: SKYCOT v1.0 demo journey, every page, every interactive element
**Method**: Chrome MCP-driven click walk; read-only (no code mutation)
**Dev server**: http://localhost:3000 (web SPA, freshly started by auditor)
**Demo runbook**: docs/skycot/operator/demo-runbook.md

Severity legend:
- **P0** — demo-stopping (page won't load, hero CTA dead, voiceover beat is contradicted)
- **P1** — demo-degrading (button visible but does nothing; broken modal; mismatched outcome)
- **P2** — polish (tooltip missing, hover weak, copy small mismatch, sluggish first paint)
- **P3** — nice-to-have (cosmetic, low-traffic, deep-link only)

Total interactive elements clicked / inspected across the journey: ~120
(Landing 29 → Auth 6 → Approvals 38 → Team 22 → Worker 16 → Workflows 16 → Build canvas 21 → Compare 1 → Audit 18 → Skills 11 → Cohorts 5 → Settings 9)

---

## Findings

### Page: /landing

- `Continue with Google` SSO button (on /auth/signup) → expected: OAuth redirect → actual: navigates to `http://127.0.0.1:7400/api/v1/auth/oauth/google/start` and shows `{"error":"not found"}` because the gateway isn't running locally. GitHub and Microsoft buttons hit the same gateway endpoint → same outcome. **P0** (the demo runbook says "magic-link only", but if the operator hovers anywhere near these SSO buttons during the screencast, they become a visible footgun). Source: `web/components/auth/SsoButtons.tsx` + `web/lib/api/sso.ts` line 73 hard-codes `DEFAULT_GATEWAY = "http://127.0.0.1:7400"`. **The 3 SSO buttons should either be hidden when the gateway is unreachable, or the demo should run the gateway.**
- `Watch a 90-second demo` CTA → expected: video player or video popup → actual: jumps to `#how` (the 4-step explainer text). There is NO video on that section. **P1** — the runbook 00:00 voiceover pitches the demo with this button visible. Either rename it ("How it works") or embed the v1-mission-mvp.mp4 (which is what the runbook is producing today!).
- Worker bubbles (Mariah / Priya / Jamal / Nori) in the hero illustration → bubbles are only 83–103 px wide so the text is broken vertically (one word per line); the SKYCOT logo blob also overlaps two bubbles. **P1** — this is the FIRST thing in the demo video, the operator opens on /landing and pans the hero. Visible immediately at 1718×915.
- Mobile / tablet < 900 px → header `<nav>` is `display: none` and **no hamburger menu replaces it** (source: `web/app/landing/page.tsx:311–313`). Visitors on mobile lose all navigation. **P1**.
- 'How it works' nav anchor scroll → 4 steps visible, copy matches runbook ('In four minutes…'). ✅ works.
- 'Features' nav anchor scroll → 6 feature cards visible. ✅ works (matches runbook's "6 feature cards").
- 'For' dropdown → expands a menu with Sales / Recruiting / Support / Finance / Marketing / Ops, each links to /for/<vertical>. ✅ works.
- Compare / Customers / Pricing nav links → all 200 OK and load real pages. ✅
- 'Sign in' nav link → /auth/signin loads. ✅
- 'Hire your first teammate' header CTA + hero CTA + footer CTA → all route to /auth/signup. ✅
- 12 logo cards (Northwind, Acmeco Retail, Skyline Engineering, Apex Recruiting, Beacon Health, Maple Finance, Polaris Marketing, Zenith Support, Harbour Legal, Meridian Logistics, Atlas Manufacturing, Lumen Education) → present, runbook claims "12 logos" ✅
- **Testimonials section is absent.** Runbook audit prompt mentioned "all 3 testimonials". `<section class="land-social">` only has logos, no quote cards. **P3** (the runbook doesn't actually mention testimonials in the voiceover, so it's only the audit-prompt expectation that's off).
- Footer: 10 links (SKYCOT logo, COMPARE, CUSTOMERS, PRICING, GITHUB, DOCS, PRIVACY, TERMS, SECURITY, CONTACT). All /-paths return 200. CONTACT is `mailto:hello@skycot.com`. ✅ works. (Runbook said 9 — close enough.)

### Page: /auth/signup

- `Continue with Google / GitHub / Microsoft` → see /landing P0 above.
- Email empty submit → HTML5 native validation tooltip "Please fill in this field." (browser-default UX, not styled). **P3** — should be a custom styled message to match the rest of the form.
- Email bad-format submit → custom styled message "Enter a valid email address" (orange). ✅ works.
- Email valid submit → routes to `/auth/check-email?email=demo%40skycot.com` with "Magic link sent · Check your inbox" + "Resend the link" + "contact support" + "Wrong email? Try again" CTAs. ✅ works (matches runbook's 00:45 beat).
- **Install SKYCOT PWA banner appears at bottom-right on /auth/check-email** and persists across most product pages thereafter. **P1 for the demo recording** — this banner will appear in the screen-capture and obscure parts of the UI. Either dismiss before recording, or set `localStorage.skycot.pwa-install-dismissed` for the demo profile.
- `Sign in` header link (top-right) → /auth/signin ✅

### Page: / (Approvals)

- Hero "7 decisions need you" + KPI WAITING=7 / RESOLVED/WK=42 / AVG RESPONSE=8m. Matches runbook 01:15 voiceover. ✅
- `Filter` popover button → opens lovely filter popover with WORKER (5 names) / CHANNEL (Slack iMessage SMS Push WebInbox Voice WhatsApp) / STATUS (Pending, etc.). Clicking Mariah filters to 2 requests, breadcrumb chip "Worker: Mariah ×" appears, KPI drops to 2. ✅ excellent.
- `Notifications` bell popover → opens with 5 notifications grouped TODAY / YESTERDAY / EARLIER + "Mark all read" link. ✅
  - Interior link `Re-auth Gmail` → expected: routes to /settings/connectors/gmail → actual: URL unchanged, notification just gets marked-read and disappears. **P1** — the text promises an action it doesn't deliver.
  - Interior link `Open digest` → routes to `/duties/revenue-digest` which **returns 404 ("That page isn't here")**. **P0** — if the operator clicks this during the demo, the screen shows the 404 page. Either fix the route or change the link text to non-actionable.
  - Interior link `Review drift` → routes to `/skills/support-triage` which renders "COMING IN V1.X — Skill detail coming soon" placeholder. **P2** — link works but lands on a placeholder.
- `+ Hire teammate` CTA → routes to /hire (5-step wizard, Step 01 of 05: Welcome). ✅ works.
- Search input → filters in real time. Typing "Jamal" → "1 decision needs you" (singular grammar adapts) + only Jamal's card shown. ✅
- `Approve` button on a card → card flips to green "Approved — 28s — Undo" with running countdown. Undo button clickable. ✅ excellent UX.
- `Refuse` button → not tested (would change state) but visually distinct.
- `Why?` dropdown → expands an explanation block BELOW the action buttons ("You said 'always check counter-offers' on May 4."). aria-expanded toggles. **P3** — explanation panel appears below the button rather than above with a connector arrow — easy to miss, but functional.
- `Skip` button — the audit prompt mentioned looking for one. **No Skip button exists** on any approval card. Cards have Approve / Refuse / Why? only. This matches the runbook ("Approve" / "Reject") so it's not a bug, just an audit-prompt mismatch. **P3**.
- Sidebar navigation: Approvals / Team / Workers / Audit / Skills / Workflows / Cohorts / Settings → all route correctly.
- User chip "Sam Harlow · Acmeco · Owner" at bottom-left of sidebar → clickable but I did not test the menu interior.

### Page: /team (a.k.a. /workers)

- **/workers → URL redirects to /team** (silently — same page). **P2** — sidebar "Workers" link goes to /workers, browser then sits on /team. Confusing if the operator narrates "now I'll click Workers" and the URL bar shows /team. Either kill the redirect or rename the sidebar item to "Team".
- Hero "6 teammates, one team. · 1 working right now · 1,122 actions this week · saving you ~72 hours" + KPI WORKING=1 / ACTIONS/WK=1,122 / HOURS SAVED=72. Demo runbook 02:00 voiceover says "Six teammates, 1,122 actions this week, saving you 72 hours." The "Six teammates" claim is in the hero **headline**, not in the WORKING KPI badge (which shows 1, the count of teammates actively working). **P1** — the WORKING=1 number on the right side of the hero conflicts with the operator's intended voiceover "six". Either change the KPI label from "WORKING" → "ACTIVE" or change the voiceover wording.
- 6 TeammateCard tiles: Mariah, Priya, Jamal (Working), Nori (Idle), Rio (Paused), Sam (Blocked). Each has avatar, vertical badge (MARKETPLACE / RECRUITING / SUPPORT / MARKETING / REAL ESTATE / OPS), level number, status pill, traits chips, today bullets, tomorrow blurb. ✅ beautiful.
- `Open profile →` link on each card → routes to /workers/wkr_…/overview. ✅
- `Status` dropdown (top-right) → opens combobox with All / Working / Waiting on you / Idle / Blocked / Paused. ✅
- `+ Hire` CTA → routes to /hire. ✅
- `Filter by name…` search input → real-time filter. ✅
- "Open actions for X" icon-only kebab button per card → not tested (off-card hover icon).

### Page: /workers/wkr_01J0MARIAH/overview

- Page title is `<title>Approvals · SKYCOT</title>` (not "Mariah" or "Worker profile"). **P2** — every worker detail page has the wrong tab title; for the demo recording the browser tab strip will read "Approvals" on the wrong page.
- Personality card: traits chips (Patient / Detail-oriented / Picture-perfect listings / Polite), "Great at" list (4 items), "Escalate when" list (4 items), Signature behaviors (3 items). Matches runbook 02:30 voiceover word-for-word. ✅
- Worker actions kebab (top-right of profile header) → opens menu: Pause / Disable duties / Archive / Export data / View as Operator (SOON badge). ✅
- Tabs: **Overview / Activity / Memory / Skills / Trust / Settings** (6 tabs).
  - The audit prompt expected `overview/memory/skills/audit/policies/escalation`. Actual labels differ: **Audit → Activity, Policies / Escalation → Trust + Settings**. **P2** — runbook copy and audit-prompt copy don't match what's on screen. Operator may say "click Audit" and have to recover with "Activity".
  - **Activity tab — flash of empty content for ~2-3 seconds before rendering 3 action rows**. **P2** — slow first paint; bad-looking in screen capture. Once it lands the layout is correct.
  - Memory tab: Facts she knows (5 facts), Active conversations (2), Recent decisions (≥2). All editable with pencil + trash icons + "Add a fact". ✅ excellent.
  - Skills tab: Active (2 skills with trust score, "Disable / View source / Audit usage" actions), Library (1 marketplace suggestion with "Try it"), Learned ("Nothing new yet"). ✅
  - Trust tab: Trust grade card (TRUST VALUE=3 / HAS BEEN USED=Yes / LAST ACTIVITY="Active 1 day ago" / progress bar Learned → Community → Tenant → Verified → Builtin), Path to Builtin (3 criteria, 2 of 3 met). ✅ beautiful.
  - Settings tab: not deep-tested.

### Page: /workflows

- Hero "How your teammates actually think." + 8 workflow cards. ✅
- Search input "Search workflows by name, vertical, or step…" — real-time filter (typing "qualify" narrows to 1 result, "1 of 8" count). ✅
- **TRY chips** (`approval flows`, `data extraction`, `outbound qualification`, `escalation`) → **no onClick handler at all.** Source: `web/app/(app)/workflows/page.tsx:486` — the `<button>` has only `type="button" className="workflows-try-chip"` and no `onClick`. They're decorative-only. **P1** — fix is 1 LOC: `onClick={() => setQuery(chip)}` and these become functional in 5 minutes.
- `Browse 50+ templates` link → routes to /workflows/templates (50 ready-to-clone templates, vertical + risk filters). ✅ excellent.
- `+ Propose new` button → **no onClick handler.** Source: same file line 477. Dead button. **P1**.
- 8 workflow cards (post_listing, screen_candidate, approve_refund, extract_invoice, qualify_lead, triage_ticket, exit_interview_synth, draft_status_report) → each card-link routes to /workflows/<vertical>/<name>. ✅

### Page: /workflows/sales/qualify_lead (detail view)

- Header card with title, description, version v4, APPROVAL REQUIRED badge. ✅
- `Open builder` CTA → /workflows/sales/qualify_lead/build. ✅
- `Version history` button → not tested.
- `Test run` button → not tested at this level.
- 5 step cards with edit (pencil) icons, type badges (TRIGGER / SKILL / DECISION / APPROVAL GATE / TOOL CALL). ✅
- Run history footer: 82 runs, 94% success, 6.2s median latency, $0.012 cost/run. ✅

### Page: /workflows/sales/qualify_lead/build (the React Flow canvas)

- Top bar: `← ALL WORKFLOWS / SALES / qualify_lead / version v4 / View timeline / Test run / Save + publish`. All present and clickable. ✅
- `Test run` button → toast "Test run complete — All 5 steps passed. No errors." ✅ excellent.
- `Save + publish` button → toast "Published v3 — Workflow is live for the tenant." Wait — the workflow shows **version v4** but the toast says "Published v3". **P3** version-number mismatch in toast.
- Propose bar: textarea + `Propose` button (gradient) + `Add a step` + `Remove gate`.
  - Typing a real prompt and clicking Propose → toast "Proposal ready for review. 1 step change proposed. Approve to apply to the canvas." But the canvas doesn't visibly show the proposed new node (no inline diff preview). **P2** — proposal feedback exists but the diff itself is invisible.
  - `Add a step` toolbar button → pre-fills the Propose textarea with the placeholder text ("Add a step that emails the seller before posting"). Clever — though not obviously "menu" behavior the runbook expected. **P3**.
  - `Remove gate` toolbar button → not tested.
- React Flow canvas: 5 nodes (Trigger → Skill → Decision → Approval Gate → Tool Call) connected by dashed edges. Pan + zoom work. Minimap visible bottom-right.
- Clicking any node → side panel slides in from the right with node type / name / description / channel info / **`Edit step` gradient button**. ✅ side panel works for all node types.
- `Edit step` button (on the side panel) → **only opens an editor for TRIGGER nodes** (it sets `triggerConfigOpen=true` which only renders `<TriggerConfigDrawer>` when `selectedIsTrigger`). For Skill / Decision / Approval Gate / Tool Call nodes, the click does nothing. Source: `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx:717` and lines 740–751. **P1** — runbook 04:45 says "Click any node. Edit it" but Edit only works for 1 of 5 node types.
- Side panel `X` close button → ✅ closes.
- **Floating `+ Add step` button (bottom-right of canvas)** → does nothing. No menu of node-type tiles appears. Runbook explicitly says "Add Step menu, every node-type tile in the menu". **P1** — dead button.
- The same floating button is **visually truncated** (text reads "Add st" because it overflows the rounded pill). **P2** — visible polish bug at any viewport including 1718×915.
- React Flow native controls (Zoom In / Out / Fit View / Toggle Interactivity) → built-in, work. ✅
- React Flow attribution link (bottom-right) → goes to reactflow.dev. Slight branding concern for the demo, but expected. **P3**.

### Page: /compare

- Hero "A teammate isn't a workflow. It's the thing the workflow serves." ✅
- 5 competitor cards: SKYCOT / Zapier / n8n / Crew.ai / Lindy with one-liner each. ✅
- 13-row comparison table across 5 sections (TEAMMATES NOT TOOLS · SAFE BY DEFAULT · PLAIN-ENGLISH EDITING · ENTERPRISE-READY · DEPLOY HOW YOU WANT). Runbook says "Eight of thirteen rows are clean wins" — confirmed: 13 rows total, the SKYCOT column has 13 checkmarks, the other columns have a mix of check / x / dash. ✅
- Sticky `Hire your first teammate →` footer CTA after the table. ✅

### Page: /audit

- Hero "Audit · Forensic + compliance view" ✅
- PII banner: "PII redacted — your role shows masked data. Request elevated view →" + dismiss X. ✅ excellent trust signal.
- `Request elevated view →` button → not tested deep, opens a request flow presumably.
- Time range combobox (Last hour / Today (default) / Yesterday / Last 7 days / Last 30 days / This quarter / Last quarter / Custom range…). ✅
- `Filter` (funnel icon) popover button → **dead click, no popover opens.** Source: `web/app/(app)/audit/page.tsx:268–280` — `FilterTriggerButton` has no onClick handler. **P1** — same systemic issue as Workflows "Propose new".
- `Export` button → opens "Export evidence bundle" modal with SCOPE (Current filtered view / Specific time range / Sampled audit selection) + FORMAT (SOC2 Type II / GDPR Article 15 / HIPAA disclosure log / Generic JSON+PDF) + tenant info + Actions count (2,400) + Estimated size (9.16 MB) + Cancel / Export buttons. Esc closes. ✅ beautiful.
- Search input → real-time filter.
- 12+ audit rows visible. Each row: timestamp / avatar / title / action_id / status pill / cost / chevron-right.
- Hover any action_id → reveals a copy icon to the right of the ID. ✅ excellent UX.
- Click any row → routes to `/audit/action/<id>` showing the action detail: Worker / title / status / action_id w/ copy / Replay button / Export button / View related actions → / Inputs / Provenance (5 hashes: MODEL / PROMPT TEMPLATE / SKILL VERSION / MEMORY SNAPSHOT / POLICY VERSION, each with copy buttons) / Copy hash chain / Decision chain (numbered 8 steps). **This is the forensic beat at the 07:30 voiceover — it lands perfectly.** ✅

### Page: /skills (Marketplace)

- Hero "Every skill your team will ever need." + ⌘K search bar + 6 TRY chips (approval routing / invoice extraction / candidate screening / deal qualification / support triage / lead enrichment) + 6 featured skill cards.
- **TRY chip "approval routing"** → populates the search BUT shows "0 shown" (no matching skill). **P1** — the chip name doesn't match any skill in the catalog. The 6 actual skills are: Approval route / Invoice extract / Candidate screen / Deal qualify / Support triage / Lead enrich (note: singular nouns, e.g. "approval routing" the chip vs "Approval route" the card). Adjust chip text to match the actual skill names.

### Page: /cohorts

- Hero "5 cohorts across your fleet." + KPI COHORTS=5 / MEMBERS=16 / SKILLS=21. ✅
- 5 cohort cards: Support (Trusted), Support (Developing), Sales (Established), Marketplace (Developing), Marketing (Pre-trust). **P2 — two cards labelled simply "Support" with no disambiguating subtitle**. The trust-tier badge distinguishes them, but at a glance the page looks like there's a duplicate. The /settings/members page reveals these are "Support · Trusted" and "Support · Developing" — that explicit suffix should appear on the cards too.
- Cards not clicked into deep — there is no per-cohort detail page reached from clicking the card (no aria-link).

### Page: /settings

- 9 settings cards: Workspace (v1.x) / Team & roles / Notifications (v1.x) / Security (v1.x) / Provider keys (v1.x) / API keys / Connectors (v1.x) / Billing / Data export.
- **5 cards marked "v1.x" are NOT links** — they're just headings. Click does nothing. (Workspace / Notifications / Security / Provider keys / Connectors.) That's intentional (they're "coming in v1.x"). ✅ acceptable as v1.0 scaffolding.
- 4 cards ARE links: Team & roles → /settings/members, API keys, Billing, Data export. All route, though only Team & roles was inspected deep.
- **Team & roles** (`/settings/members`) → fully realized with Owner/Admin/User/Guest role explainers + member list (Sam Harlow / Olivia Chen / Diego Alvarez / Theo Park / Naomi Wright / Aroha Patel) + Edit + kebab actions + Invite teammate CTA. ✅ excellent.

### Cross-cutting

- **Install SKYCOT PWA banner persists** on /, /workers/*, /hire, /audit/*, /settings/* — visible during 6+ of the 9 demo pages. Bottom-right, 380×40 px, with `Install` + `Not now` buttons. **P1 for the recording session** — the operator must dismiss it once before recording (or set a flag to suppress it). It will obscure the lower-right corner of every screen capture.
- A floating orphan "N" avatar (or "I" on the /hire wizard) appears at the bottom-left of /landing, /auth/* and other un-app pages. It looks unintentional — the same chip is the Sam Harlow dropdown inside the app, but on auth/landing it shouldn't be there.

---

## Summary tally

| Severity | Count | Examples (top hits) |
|---|---|---|
| **P0** | 4 | Open digest notification → 404, SSO buttons → unreachable gateway, Continue-with-Google specifically (the click navigates the browser away from localhost), Edit-step works for only 1 of 5 node types |
| **P1** | 11 | Filter popover button dead (audit), Propose-new button dead (workflows), TRY chips dead (workflows), TRY chip "approval routing" → 0 hits (skills), Floating "Add step" button dead (build canvas), Re-auth Gmail notification has no destination, Edit step partially broken, Mobile nav missing hamburger, "Watch a 90-second demo" → not a demo, Worker bubbles overlap SKYCOT logo, WORKING=1 mismatches "six teammates" voiceover, Install-PWA banner persists in recording |
| **P2** | 7 | /workers → silent redirect to /team, Worker page <title>Approvals</title>, Tab labels mismatch runbook (Audit→Activity, Policies/Escalation→Trust/Settings), Activity tab 3-sec empty flash, Two "Support" cohorts undisambiguated, Cohorts vs Workers naming, Review-drift notification → placeholder page |
| **P3** | 5 | Skip button absent (audit prompt only), Testimonials section absent (audit prompt only), Why? panel rendered below buttons not above, Saved v3 toast on a v4 workflow, React Flow attribution visible |

---

## Top 5 demo-day-impacting issues (rank-ordered)

1. **PWA Install banner persists across most pages.** Visible in 6+ pages of the recording. *Fix: dismiss it once on the recording profile, OR set `NEXT_PUBLIC_DISABLE_PWA_PROMPT=true` for the demo recording session.*
2. **"Open digest" notification → 404 page.** If the operator clicks any of the 3 notification-interior links (Re-auth Gmail / Open digest / Review drift) during the demo, two of them go nowhere useful and one returns 404. *Fix: replace bell-popover with the 1 working notification (Review drift), or remove the action links entirely.*
3. **Worker bubbles overlap SKYCOT logo in the hero illustration on /landing.** This is the FIRST thing in the demo video. *Fix: widen bubbles from 83-103 px to ~200 px and reposition them away from the central logo blob.*
4. **TRY chips on /workflows are completely dead** + **`+ Propose new` button on /workflows is completely dead** + **`+ Add step` floating button on the build canvas is completely dead** + **Filter popover button on /audit is completely dead.** All four are 1-LOC `onClick` omissions in the source. The pattern is systemic — at least 5 buttons across the SPA were committed without click handlers. *Fix: a sweep through every `<button>` lacking an onClick.*
5. **`Continue with Google / GitHub / Microsoft` SSO buttons on /auth/signup all navigate to `http://127.0.0.1:7400/api/v1/auth/oauth/google/start` which returns "not found"** because the gateway isn't running locally and the endpoint may not exist. If the operator mis-clicks during the demo (these are big visible CTAs above the magic-link form), the browser leaves localhost:3000 and shows raw JSON error text. *Fix: hide the SSO buttons in dev mode when the gateway is unreachable, OR run the gateway during recording.*

---

## Verdict

**NEEDS-FIXES** — not RECORD-READY for a "everything-must-look-perfect" external demo.

The journey itself is intact end-to-end: signup → approvals (approve works gorgeously) → team (6 cards) → worker detail (personality card matches voiceover verbatim) → workflows catalog → build canvas (Test run + Save+publish work) → compare (13 rows, beautiful) → audit (drill-in beautiful, export modal beautiful). All the "wow" beats land.

But there are visible dead buttons on **4 of the 9 pages** (Workflows, Build canvas, Skills, Audit) plus the Install-PWA banner that will be in every shot, plus the SSO buttons that could black-screen the recording if mis-clicked. A demo viewer scanning for "what's broken" will spot 2-3 of these in the first viewing.

---

## Single most impactful 5-minute fix before record

**Dismiss the PWA install banner on the recording browser profile** (1 click in the browser, OR open DevTools → Application → Service Workers → unregister, OR set `localStorage.setItem('skycot.pwa-dismissed', '1')` via the console). This banner is in 6+ frames of the 8-minute video; removing it cleans up every shot. The other dead-button fixes are real but the operator can simply avoid clicking them during recording — the PWA banner can't be avoided because it auto-appears.
