# Whole-site MVP claim coverage — load-bearing rollup

**Audit date**: 2026-05-26
**Agent**: e2e-ux-journey-test (whole-site rollup)
**Scope**: every page NOT covered by the 4 other audit agents (marketing, /cohorts, /settings + sub-tabs, /brand, /team, /workflows/templates, theme toggle, viewport responsiveness), then the 6-MISSION-claim rollup.
**Backend state observed**: dev server :3000 up, gateway :7400 up. Seed database has 1 worker ("Maya", marketing-intern), 1 cohort, 0 workflows, 0 approvals, 0 skills, 0 audit rows from real agent turns (only operator-browsing rows with worker="unknown").

---

## TL;DR

- **24 findings**: 7 P0, 8 P1, 7 P2, 2 P3.
- **Methodology constraint**: the Claude Preview browser wedged on the first `/landing` navigation and never recovered, so the report is based on (a) server-rendered HTML via `curl` from the dev server, (b) gateway responses with the authorised `Origin: http://localhost:3000` header, and (c) source-code inspection. No live screenshots / DOM snapshots captured. Viewport-responsiveness spot-checks could not be performed and are flagged as a gap to validate.
- **Biggest blocker**: the gateway CORS (Cross-Origin Resource Sharing — browser rule that controls when a webpage can talk to a server on a different web address) allowlist matches `http://localhost:3000` **exactly** and rejects `http://127.0.0.1:3000` and `http://localhost:53854` (the preview-proxy port). Any operator opening the SPA via a non-`localhost:3000` URL gets perpetual "Loading…" spinners on every gateway-fed page — the demo is recordable only if the operator types the right URL.
- **Demo-readiness verdict**: the SPA renders 200 on 49 / 51 in-scope routes, but the underlying seed data is a desert (1 worker named Maya, 0 cohorts beyond marketing-intern, 0 workflows, 0 approvals, 0 marketplace skills, 0 real agent-turn audit rows). Without seeding the Mariah/Peta/Priya/Jamal/Nori roster + the customer_research_followup workflow + the "competitor pricing scrape" skill + 10+ real audit rows, the 6-scene runbook is **not recordable as scripted**.
- **6-claim SPA story**: 1🟡 (visible-but-thin), 2🔴 (no delegation surface visible), 3🟢 (provider/connector settings ship), 4🔴 (marketplace empty), 5🔴 (no UI evidence of cu-macos), 6🟡 (audit page renders but feed is operator-noise only). See "6-claim coverage rollup" section.

---

## Headline blockers (P0)

1. **F01** — Gateway CORS allowlist is `localhost:3000`-only; `127.0.0.1:3000` and the preview-proxy port both return a 204 preflight with no `Access-Control-Allow-Origin`, so every authenticated browser call fails and the entire in-app UI sits on spinners.
2. **F02** — Seed database has 1 worker (Maya, marketing-intern), not the 5-worker roster the runbook narrates (Mariah/Peta/Priya/Jamal/Nori). Scene 1 is unrecordable.
3. **F03** — Cohorts gateway returns only `marketing-intern::unknown`, so the locked D3 magazine grid will render at most 1 dept tab ("Marketing") + 1 lead tile. The "6 dept tabs + asymmetric magazine" demo can't be staged.
4. **F04** — Skill marketplace gateway returns `{"skills":[],"total_count":0}`. Scene 4's "competitor pricing scrape" search beat returns nothing.
5. **F05** — Workflows gateway returns 0 workflows; the `customer_research_followup` MCP+computer-use combo Scene 5 narrates does not exist as runtime data.
6. **F06** — Audit list contains only operator-browsing rows (`worker.name: "unknown"`). Scene 6 narrates clicking an audit row and seeing "real agent provenance" — the click reveals operator self-spam, not agent work.
7. **F07** — The worker-detail tab strip is Overview / Activity / Memory / Skills / Trust / Escalation / Settings — there is no "Schedules" tab. Scene 2 narration "click Schedules → see 3 duties → Run now" doesn't map to any tab.

---

## All findings, in order

### F01 [P0] — Gateway CORS allowlist is single-origin; non-localhost:3000 origins die silently

**What I saw**: `OPTIONS http://127.0.0.1:7400/api/v1/skills` with `Origin: http://localhost:3000` returns `Access-Control-Allow-Origin: http://localhost:3000` and the call succeeds. The same OPTIONS with `Origin: http://127.0.0.1:3000` or `Origin: http://localhost:53854` (the Claude-Preview proxy port) returns 204 with **no `Access-Control-Allow-Origin` header at all**. The browser silently blocks every actual fetch, producing the "Loading approvals…" spinner state I observed on `/`, `/skills`, `/workers`, `/cohorts`, etc.
**Why it's a problem**: any operator who types `127.0.0.1:3000` instead of `localhost:3000`, or anyone running through a port-rewriting proxy (which the Claude Preview tool does), gets a UI that looks broken. For the demo recording this is a hidden footgun — the operator must hit the correct origin or the camera captures spinners.
**Reproduction**:
- `/usr/bin/curl -s -i -X OPTIONS -H "Origin: http://127.0.0.1:3000" -H "Access-Control-Request-Method: GET" http://127.0.0.1:7400/api/v1/skills`
- Compare against the same with `Origin: http://localhost:3000` — only the latter returns `Access-Control-Allow-Origin`.
**Likely fix location**: gateway CORS middleware — `crates/argo-gateway/src/middleware/cors.rs` or equivalent. Add `http://127.0.0.1:3000` to the allowlist alongside `http://localhost:3000`, or pattern-match on host+port irrespective of `localhost` vs `127.0.0.1`.
**Suggested fix**: extend allowlist to both name and IP forms of localhost; document the expected dev URL prominently in the demo runbook.

### F02 [P0] — Seed worker roster is 1 worker (Maya), not the 5-name runbook roster

**What I saw**: `GET /api/v1/workers/019e5dbb-c03d-79a0-a9a4-ee60bb3c8218` returns a worker named **Maya** with `vertical: marketing-intern`, `last_active_at: null`, `headline.bullets: ["No activity yet."]`. The runbook (`docs/skycot/demos/v1-mission-mvp-runbook.md`) narrates Scene 1 as 5 cards labelled Mariah / Peta / Priya / Jamal / Nori.
**Why it's a problem**: function-side fabrication — Scene 1's whole narration is "here are your 5 hires, click Mariah" and the database has 1 hire named Maya. Demo unrecordable.
**Reproduction**:
- `/usr/bin/curl -s -H "Origin: http://localhost:3000" http://127.0.0.1:7400/api/v1/workers/019e5dbb-c03d-79a0-a9a4-ee60bb3c8218`
- Hit `/workers` in browser → only "Maya" card visible (when CORS isn't blocking).
**Suggested fix**: run the demo-seed CLI (per memory `feedback_no_theater_in_demo` — the "Option B real backend seed" path) to insert the 5 canon workers before recording.

### F03 [P0] — Cohorts gateway returns 1 vertical; D3 magazine layout collapses to a single "Marketing" tab

**What I saw**: `GET /api/v1/cohorts` returns `{cohorts: [{vertical: "marketing-intern", member_count: 1, ...}], total_count: 1}`. The locked D3 design (`project_cohorts_page_design_locked` memory) requires 6 dept tabs + asymmetric lead/stacked/compact layout. The page source (`web/components/skycot/cohorts/dept-registry.ts:75-90`) drops empty departments from the tab strip, so only "Marketing" will render.
**Why it's a problem**: the page that is supposed to be the editorial magazine showcase will look like a single empty card. Not a Scene 1/2 blocker on the runbook script, but if the camera ever pans to `/cohorts` the value-prop dies on landing.
**Reproduction**:
- `/usr/bin/curl -s -H "Origin: http://localhost:3000" http://127.0.0.1:7400/api/v1/cohorts | python3 -m json.tool`
**File reference**: `web/components/skycot/cohorts/dept-registry.ts:75-90` (the empty-dept-drop logic in `groupCohortsByDepartment`).
**Suggested fix**: seed 6+ verticals (support, sales, marketing, finance, project-ops, people, it-devex) with at least 1 cohort each so the magazine layout has content.

### F04 [P0] — Skill marketplace gateway returns 0 skills

**What I saw**: `GET /api/v1/skills` returns `{"skills":[], "total_count":0}`. `GET /api/v1/skills/learned-this-month` returns `{"error":"not found"}`. `GET /api/v1/marketplace/skills` returns `{"error":"not found"}`.
**Why it's a problem**: Scene 4 narrates "search 'competitor pricing scrape' → 3 results → click → install flow". With 0 skills, the search returns empty.
**Reproduction**:
- `/usr/bin/curl -s -H "Origin: http://localhost:3000" http://127.0.0.1:7400/api/v1/skills`
**Suggested fix**: seed skill rows including a "competitor pricing scrape" labelled item before recording, OR pivot Scene 4 narration to a skill the seed actually contains.

### F05 [P0] — Workflows gateway returns 0 workflows; customer_research_followup absent from templates seed

**What I saw**: `GET /api/v1/workflows` returns `{"tenant_id":"...", "total_count":0, "workflows":[]}`. The static `/workflows/templates` page renders 50+ template cards from `web/lib/workflow-templates.ts`, but **does not include** a `customer_research_followup` template — the closest match is `competitor_intel_digest`.
**Why it's a problem**: Scene 5 narrates clicking "customer_research_followup → run → both MCP and computer-use steps land in audit." With no live workflow and no matching template, the entire MCP+CU combo demo is unstaged.
**Reproduction**:
- `/usr/bin/curl -s -H "Origin: http://localhost:3000" http://127.0.0.1:7400/api/v1/workflows`
- `/usr/bin/curl -s http://127.0.0.1:3000/workflows/templates | grep -oE 'template-card-name[^>]*>[^<]+' | head -20`
**File reference**: `web/lib/workflow-templates.ts` (static seed) — needs a `customer_research_followup` entry, or Scene 5 needs to rename to an existing template.
**Suggested fix**: add the canon template to the seed; also wire `/workflows` to surface the template before the live runtime catches up.

### F06 [P0] — Audit list contains only operator-browsing rows (worker="unknown"), no real agent provenance

**What I saw**: `GET /api/v1/audit` returns rows where every `worker.name` is `"unknown"` and the verbs are `http.request` / `audit.detail.viewed` / `audit.replay` — i.e. me (the audit agent) browsing the gateway, not agents executing work.
**Why it's a problem**: Scene 6's load-bearing beat is "click row → real provenance shows up." Per `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md` the drill-in test says: "if the camera clicks a row, the destination must be real." Today the destination is "user-agent: curl/8.7.1" or equivalent — theatre the camera sees.
**Reproduction**:
- `/usr/bin/curl -s -H "Origin: http://localhost:3000" 'http://127.0.0.1:7400/api/v1/audit?limit=10' | python3 -m json.tool`
**Suggested fix**: as part of seed-bootstrap, run a small batch of real agent turns (5-10 cassette-replays) so the audit table contains worker-attributed entries before the camera clicks in.

### F07 [P0] — Worker-detail has no "Schedules" tab; runbook Scene 2 narration mismatches code

**What I saw**: `web/components/skycot/WorkerTabs.tsx:19-39` declares `WORKER_TAB_IDS = [overview, activity, memory, skills, trust, escalation, settings]` with labels Overview / Activity / Memory / Skills / Trust / Escalation / Settings. Duties (now relabelled "Responsibilities" in `web/components/skycot/worker/DutiesCard.tsx:89`) live inside the Overview tab.
**Why it's a problem**: the runbook says Scene 2 is "click the Schedules tab and see 3 cron-shaped duties." With no Schedules tab, the operator either has to ad-lib over the discrepancy or the recording shows the operator clicking around looking for it.
**Reproduction**:
- `find web/app/(app)/workers/[workerId] -type d -maxdepth 1` → confirms `activity / escalation / evals / memory / overview / settings / skills / trust` and no `schedules`.
- `web/components/skycot/WorkerTabs.tsx:19-39` is the canonical tab list.
**Suggested fix**: either add a "Schedules" tab that surfaces the Responsibilities cron summaries with a "Run now" affordance, or rewrite Scene 2 to "click Overview → Responsibilities card → Run now."

### F08 [P1] — Multiple in-app pages leak the default "Approvals · SKYCOT" `<title>` because they don't export `metadata.title`

**What I saw**: `web/app/(app)/layout.tsx:18-23` sets `title: { default: "Approvals", template: "%s · SKYCOT" }`. Pages that are `"use client"` (and thus can't export `metadata`) inherit the default. Concretely:
- `/brand` → `<title>Approvals · SKYCOT</title>`
- `/verticals` → `<title>Approvals · SKYCOT</title>`
- `/orchestration` → `<title>Approvals · SKYCOT</title>`
- `/retro` → `<title>Approvals · SKYCOT</title>`
- `/billing` → `<title>Approvals · SKYCOT</title>`
**Why it's a problem**: browser-tab title, share-card title, and on-camera tab-strip all say "Approvals" while the user is on a different page. The runbook camera doesn't drill into these, so it's not a P0 — but it's visible at a glance and damages perceived polish.
**Reproduction**:
- `for p in /brand /verticals /orchestration /retro /billing; do /usr/bin/curl -s "http://127.0.0.1:3000$p" | grep -oE '<title>[^<]*</title>'; done`
**Suggested fix**: split each into a server-component page-shim that exports `export const metadata = { title: "Brand" }` and re-exports the client component, OR populate `<title>` via `document.title` in a client-side effect in each page.

### F09 [P1] — `/auth/signup` `<title>` is "Hire your first AI teammate · SKYCOT" (marketing title) while `/auth/signin` and `/auth/verify` use product titles

**What I saw**:
- `/auth/signup` → `<title>Hire your first AI teammate · SKYCOT</title>`
- `/auth/signin` → `<title>Sign in · SKYCOT</title>`
- `/auth/verify` → `<title>Verifying your magic link · SKYCOT</title>`
**Why it's a problem**: inconsistent. Either signin/verify should also use the marketing line for cohesion, or signup should drop the marketing line. The drift suggests one page's metadata never got updated when the others were refined.
**Reproduction**: curl those three paths and read the `<title>`.
**File reference**: likely `web/app/auth/signup/page.tsx` or its layout.
**Suggested fix**: align to one of "Sign up · SKYCOT" (consistent with signin) or "Hire your first AI teammate · SKYCOT" across all three.

### F10 [P1] — `/maintenance` and `/sla` page metadata `<title>` falls through to root default ("SKYCOT — Hire your first AI teammate")

**What I saw**: `/maintenance` and `/sla` both render with `<title>SKYCOT — Hire your first AI teammate</title>` — the global default, not a page-specific title. (Unlike F08, these are outside the (app) layout so the issue is at the root layout level — they never declare a title at all.)
**Why it's a problem**: similar to F08 — share cards and browser tabs are wrong. Less likely to be on-camera but still public surfaces visible to investors who navigate the marketing site.
**Reproduction**: `for p in /maintenance /sla; do /usr/bin/curl -s "http://127.0.0.1:3000$p" | grep -oE '<title>[^<]*</title>'; done`
**Suggested fix**: add `export const metadata = { title: "Maintenance · SKYCOT" }` to each route's page or layout.

### F11 [P1] — `/team` is in the sidebar nav but is **not** linked from any marketing page; investors who land on `/landing` cannot reach the team-roster value-prop

**What I saw**: the in-app `/team` page is the teammate-roster surface (TeammateGrid + BrandLoudHero + filter chips) — a value-prop demo surface. Marketing pages (`/landing`, `/pricing`, `/customers`, `/compare`) have no `/team` link. The only top-level shell entry is the sidebar in the (app) layout.
**Why it's a problem**: anyone who hasn't signed in can't see the team page from the public site. Even if the demo MP4 doesn't navigate here, an investor opening the landing in a separate tab can't reach the surface.
**Reproduction**: `grep -o 'href="/team"' /tmp/landing.html /tmp/pricing.html /tmp/customers.html /tmp/compare.html` → no matches.
**Suggested fix**: add `/team` (or a marketing equivalent at `/teammates` or `/roster`) to the MarketingNav. Public-release blocker, not POC blocker.

### F12 [P1] — Dept registry has 8 departments but locked design (`project_cohorts_page_design_locked`) says "6 dept tabs"

**What I saw**: `web/components/skycot/cohorts/dept-registry.ts:18-50` declares 8 departments: Support / Finance & Ops / Sales / Project & Ops / People & HR / IT & Dev-ex / Marketing / Legal. The locked-design memory says "6 dept tabs at the top."
**Why it's a problem**: spec/code drift. With current seed data only "Marketing" renders, so the drift doesn't yet surface; but once seed expands, the design will deviate from the lock.
**Reproduction**: `cat web/components/skycot/cohorts/dept-registry.ts | head -55`
**Suggested fix**: confirm the operator-intended set of 6 (probably drop Legal + IT-Devex or merge People+Project-Ops) and align registry to the lock, OR amend the design lock to 8.

### F13 [P1] — Theme picker is only rendered on `/approvals`, not in Settings or AppShell

**What I saw**: `grep -rn '<ThemePicker' web/` → single match at `web/app/(app)/page.tsx:360`. There is no theme picker in `/settings/workspace` or any other settings sub-tab, and the AppShell sidebar doesn't expose mode-toggling.
**Why it's a problem**: per global instructions and `project_multi_tenant_theming` memory, multi-tenant theming and light/dark are first-class. An operator wanting to switch theme has to navigate to /approvals to find the toggle. Not a demo blocker because the demo can pre-set the theme, but it's a UX gap users will hit on day 1.
**Reproduction**: `grep -rn '<ThemePicker' ~/code/argo/web/` → only `~/code/argo/web/app/(app)/page.tsx:360`.
**Suggested fix**: also render `<ThemePicker />` inside `/settings/workspace` and/or the AppShell header / user-menu drawer. Per the locked Settings design memory, the workspace tab is the natural home.

### F14 [P1] — `/docs` has no `<h1>` element — only `<h3>` card titles — and the page rules itself out of the page-heading hierarchy

**What I saw**: `grep -oE '<h[1234][^>]*>[^<]+</h[1234]>' /tmp/docs.html` returns only `<h3 class="dh-card-title">Getting Started</h3>` etc. There is no `<h1>` on the page.
**Why it's a problem**: SEO and accessibility regression. Screen readers landing on /docs can't find the document title. Plus the "Docs" wordmark in the head doesn't appear in the body.
**Reproduction**: `/usr/bin/curl -s http://127.0.0.1:3000/docs | grep -oE '<h1[^>]*>'` → empty.
**Suggested fix**: add an `<h1>` ("Docs" or "SKYCOT Documentation") above the card grid.

### F15 [P1] — Workflow templates are 100% static seed (`web/lib/workflow-templates.ts`); no binding to gateway

**What I saw**: `web/app/(app)/workflows/templates/page.tsx:1-30` imports `WORKFLOW_TEMPLATES` from `@/lib/workflow-templates`. There is no `useWorkflowTemplates` hook fetching from the gateway.
**Why it's a problem**: if the operator builds a new vertical or template in their tenant, it cannot surface here. Page comment on line 26-28 admits "Real binding to gateway-stored templates lands in a follow-up brief." For a POC demo this is fine (dressing), but for any post-MVP customer it's not what the page implies.
**Reproduction**: read `web/app/(app)/workflows/templates/page.tsx:1-50` + `web/lib/workflow-templates.ts`.
**Suggested fix**: ship the follow-up brief that wires `useWorkflowTemplates()` to `GET /api/v1/workflows/templates`. Until then, add a small "Showing curated seed" pill in the page header so it's clear this isn't tenant data.

### F16 [P2] — Workflow template seed contains 50+ entries but **does not contain** the runbook's `customer_research_followup` name

**What I saw**: `grep template-card-name /tmp/wftpl.html | head -20` lists `qualify_lead, draft_outreach, book_demo, handle_inbound_form, dunning_followup, deal_alert_summary, competitor_intel_digest, renewal_prep_brief, triage_ticket, refund_eligibility, churn_signal_alert, kb_freshness_check, escalate_vip, weekly_report, csat_drop_alert, ...`. No `customer_research_followup`.
**Why it's a problem**: even if Scene 5 ships its own seed via the demo-bootstrap CLI, the template gallery doesn't reinforce the narration. A viewer who notices the missing template in the gallery would catch the drift.
**File reference**: `web/lib/workflow-templates.ts`.
**Suggested fix**: add `customer_research_followup` to the seed in v1.0.

### F17 [P2] — `/brand` page leaks the (app) default "Approvals · SKYCOT" `<title>` but is otherwise content-rich and intentionally internal (per locked design memories, /brand is a designer reference doc, not customer-facing)

**What I saw**: page renders with proper headings (200+ "brand", 93 "Color", 88 "Brand", 60 "color", 28 "gradient" occurrences) but title is wrong (F08 sub-case).
**Why it's a problem**: same as F08, but per `v1-demo-positioning-poc-vs-public.md` and the operator note "/brand reference page uses wrong tokens — NOT a blocker" for POC. So this is a P2 polish item.
**Suggested fix**: same as F08.

### F18 [P2] — `/team` JSON-LD schema embeds "Melbourne, Australia" hardcoded; multi-tenant SaaS positioning may surface this on every customer's deployment

**What I saw**: `/team` HTML contains JSON-LD `{"@type":"Place","name":"Melbourne, Australia"}` baked into the schema metadata. This is correct for the SKYCOT brand (per `project_skycot_melbourne_australia` memory) but on a multi-tenant SPA every tenant inherits Melbourne.
**Why it's a problem**: when SKYCOT goes white-labeled (multi-tenant theming per `project_multi_tenant_theming` memory), the embedded Melbourne address shouldn't propagate to tenant deployments. Not a v1.0 blocker; flag for the white-label work.
**Reproduction**: `grep 'Melbourne' /tmp/team.html`
**Suggested fix**: move the SKYCOT brand metadata into a separate "About SKYCOT" surface and have `/team` render only tenant-specific schema.

### F19 [P2] — `/cohorts/new` reachable but no link from `/cohorts` or anywhere else I could find in the in-app shell

**What I saw**: `GET /cohorts/new → 200`. The file `web/app/(app)/cohorts/new/page.tsx` exists. But searching for `href="/cohorts/new"` in the codebase finds it only in the `CohortCreateForm.tsx` and `/cohorts/page.tsx` empty-state CTA.
**Why it's a problem**: discoverable only via empty-state preview. Operators with non-empty cohorts can't add another via the page.
**Reproduction**: `grep -rn 'href="/cohorts/new"' web/components web/app 2>/dev/null` → limited to empty-state.
**Suggested fix**: add a "+ New cohort" button in the cohorts page header alongside the dept tabs, visible regardless of cohort count.

### F20 [P2] — `/support` (404) and `/for` (404) bare paths return 404 but `/support/inbox` and `/for/sales` etc. work

**What I saw**: directory paths `web/app/(app)/support` and `web/app/for` exist as routing trees with no top-level `page.tsx`. Bare `/support` and `/for` therefore 404. The intended landing pages are `/support/inbox` and `/for/{vertical}`.
**Why it's a problem**: any link to `/support` or `/for` in marketing or docs (or any user-typed URL guess) lands on 404. Not a runbook blocker since the runbook doesn't visit these.
**Reproduction**: `for p in /support /for; do /usr/bin/curl -s -o /dev/null -w "%{http_code} %s\n" "http://127.0.0.1:3000$p"; done`
**Suggested fix**: add `web/app/(app)/support/page.tsx` redirecting to `/support/inbox` (and similarly for `/for` → most-popular vertical). Or add explicit `next.config.js` rewrites.

### F21 [P2] — Customer detail slugs `/customers/{apex-recruiting,beacon-health-ops,maple-finance,northwind-commerce,skyline-engineering}` are fabricated logos / case studies — fine for POC dressing per `feedback_no_theater_in_demo`, but they're indexable today (/customers links to them and `robots.txt` allows /customers)

**What I saw**: `robots.txt: Allow: /customers` + `/customers` links to all 5 fabricated detail pages. Public surface.
**Why it's a problem**: per the function-vs-dressing rule, fabricated customer logos are OK for POC dressing. But they are currently indexable; an investor doing search-engine due-diligence may surface them. For v1.0 POC this is acceptable (not a blocker); for the public release (a separate later artefact) it must be addressed.
**Reproduction**: `/usr/bin/curl -s http://127.0.0.1:3000/robots.txt`
**Suggested fix**: keep as-is for POC; before public release, either replace with real customers or disallow `/customers/*` in robots.txt until they are.

### F22 [P3] — Hard-coded "Hire your first AI teammate" tagline appears in 3 separate places (root layout title, /landing meta, signup metadata) — small drift means future copy edits need 3-way sync

**What I saw**: root document title default, `/landing` title, `/auth/signup` title all carry variations of "Hire your first AI teammate." Three separate sources of truth.
**Why it's a problem**: when the operator iterates on tagline copy (which will happen post-investor-feedback), three files need touching.
**Suggested fix**: extract to one constant in `web/lib/marketing-copy.ts` and re-use.

### F23 [P3] — Sidebar nav ordering puts "Cohorts" last (after Workflows) even though the locked design memory positions cohorts as a peer of Workers/Workflows

**What I saw**: `web/components/skycot/nav-items.tsx:22-30` orders: Approvals / Team / Workers / Audit / Skills / Workflows / Cohorts.
**Why it's a problem**: discoverability nit. Putting Cohorts at the bottom signals "secondary surface" when the locked D3 design implies it's a primary value-prop. Not a demo blocker.
**Suggested fix**: consider Workers → Cohorts → Workflows ordering so the cohort tile-grid feels first-class.

### F24 [P1] — Methodology constraint: viewport-responsiveness (375 / 768 / 1440 / 1920) could not be validated; Claude Preview browser wedged on the first /landing navigation and did not recover

**What I saw**: after navigating the preview to `/landing` and triggering the chunked turbopack build, every subsequent `preview_eval`, `preview_snapshot`, `preview_inspect`, `preview_screenshot`, and `preview_click` call timed out at 30s. The dev-server itself remained healthy (curl probes returned 200 throughout). The wedge appears to be a hung Chromium renderer that did not recover during the audit window.
**Why it's a problem**: the audit scope asked for spot-checks at 4 breakpoints. I have static HTML evidence but no rendered-breakpoint snapshots. Marketing-side responsiveness gaps (text wrap, button overlap, nav collapse) cannot be ruled out from the data I have.
**Suggested fix**: re-run a focused viewport-responsiveness audit once the preview tool is unwedged, or use Playwright / Chromium-headless directly. Three pages to spot-check at minimum: `/landing` (hero + integration strip), `/pricing` (plan grid), `/cohorts` (D3 magazine tiles).

---

## 6-MISSION-claim coverage rollup

| # | Claim | Current code claim | SPA observation | Demo verdict |
|---|---|---|---|---|
| 1 | Teammate, not tool | 🟢 LIVE | 🟡 **VISIBLE-BUT-THIN.** `/workers` page exists, `WorkerHero` + personality cards exist, `/workers/{id}/overview` renders. But the seed has 1 worker (Maya, marketing-intern) with `headline.bullets: ["No activity yet."]` and `last_active_at: null`. The "teammate-feel" requires the worker to *look* like a teammate with activity, schedules, skills — Maya doesn't. **The claim is enforced in code, but the SPA cannot demonstrate it.** Fix: seed Mariah/Peta/Priya/Jamal/Nori with real activity. |
| 2 | Delegation is the unit of work | 🟢 LIVE | 🔴 **NOT VISIBLE.** The runtime's `delegate-task` fallback-chain (per CLAUDE.md round-54) is wired in the cargo-side dispatcher, but I found **no SPA surface** that exposes a delegation graph, a handoff chain, or a "this delegated to X" trace. Audit rows surface verbs but no chain visualisation. Scene 2 narrates "Run now → audit row lands" — a single audit event, not delegation. **The code-side claim is real; the SPA does not tell the delegation story.** Fix: add a delegation/handoff visualiser to the audit detail modal (per `project_audit_page_design_locked` — pop-out modal already designed but does it render the chain?). |
| 3 | Platform builds workers | 🟢 LIVE | 🟢 **VISIBLE.** `/settings/providers`, `/settings/connectors`, `/settings/api-keys`, `/onboarding/connect-provider` all 200 and render. `/cohorts/new` exists. The hire wizard (`web/app/(hire)/`) and onboarding flow (5 steps + first-hire) exist. **The platform-builds-workers story has full UI surface.** Minor: only marketing-intern vertical is seeded so the actual builder feels narrow. |
| 4 | Skills are shared + learned | 🟢 LIVE | 🔴 **NOT VISIBLE.** Skill marketplace returns 0 skills. `/skills/learned-this-month` returns 404. There is no UI evidence of a cross-tenant or cross-worker skill share. The page renders but its content payload is empty. **The code-side claim (cross-tenant pinning via #2092 + #2110) holds; the SPA visibly shows no skills at all.** Fix: seed >0 skills and a "learned-this-month" feed before recording. |
| 5 | Any-PC execution (computer-use) | 🟡 LIKELY-LIVE-pending-perms | 🔴 **NOT VISIBLE in SPA.** No page or panel surfaces "computer-use is connected" / "this worker has macOS access" / "screen-recording permission granted." `/settings/connectors` lists MCP-style integrations but no native-OS executor. `/workflows/build` doesn't visually distinguish a computer-use step from an MCP step. **The strategic decision is macOS-first (per `project_v1_strategic_decisions_2026_05_14`); the SPA has zero affordance to confirm it's connected.** Fix: add an "Executor status" pill or settings sub-tab that says "computer-use (macos): connected / not granted accessibility / not granted screen-recording" so operators can self-verify before recording. |
| 6 | Multi-channel + multi-tenant + audit-grade trust | 🟢 LIVE | 🟡 **PARTIAL.** The audit page (B3 Etherscan-style per locked design) is present and renders. But the audit feed I observed contained only operator-browsing rows (`worker.name: "unknown"`, `verb: http.request`, `action_type: audit.detail.viewed`). No agent-turn rows, no hash-chain entries I could verify, no multi-tenant negative test surfacing. `/settings/notify-channels` exists (multi-channel positioning). **Code-side cross-tenant pins shipped (#2257); SPA doesn't yet show a cross-tenant denial event the way it should.** Fix: seed agent-turn audit rows + at least one cross-tenant denial row so Scene 6's drill-in lands somewhere meaningful. |

---

## Resolved since last audit

No prior whole-site audit on this branch; this is the first run of the rollup. Earlier scene-1/2, scene-4/5, scene-6, and onboarding audits in the same `2026-05-26-1028` directory cover specific journeys.

---

## Orientation score

**52 / 100**

The page surface is broad and well-organised — almost every in-scope route returns 200, locked designs are visibly implemented (Cohorts D3, Workers C2 trading-cards, Audit B3 Etherscan, Settings D top-tabs, Onboarding F1 editorial-center), and the marketing site is coherent.

The score is mid-range because the SPA's value is gated by data that doesn't exist yet: 1 worker named Maya, 0 workflows, 0 skills, 0 marketplace, 0 agent-turn audit rows, 1 cohort vertical. Of the 6 MISSION claims, only **3** (1 thin, 3 strong, 6 partial) tell a visible story in the SPA today; **2, 4, and 5** are functionally claimed in code but invisible in the product surface.

Plus the CORS wedge (F01) silently disables every gateway-fed page for any operator who hits the SPA via the "wrong" localhost form. That's the single highest-leverage fix.

Once the demo-bootstrap CLI is run (per `feedback_no_theater_in_demo` Option B) and CORS is widened, the same surface should score in the 75-85 range without code changes.
