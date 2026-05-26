# Whole-site MVP audit + 6-claim coverage rollup — 2026-05-27 01:19 AEST

Auditor: e2e-ux-journey-test-agent (whole-site scope)
Environment: Next.js dev on :3000, gateway on :7400 (schema v83, FRESH)
Scope: marketing pages, cohorts, settings, brand, team, workflows/templates, theme toggle, viewport, plus the 6-MISSION-claim rollup based on SPA observations.

## TL;DR

22 findings: **3 P0 (demo-blocking), 9 P1 (camera-might-catch / load-bearing-UX), 8 P2 polish, 2 P3 nits**. One root cause unifies most of the P0 / P1 cluster: **the SPA's typed `fetch` client uses the absolute gateway URL `http://127.0.0.1:7400` directly, which the browser blocks via CORS-without-`Access-Control-Allow-Origin`, even though Next.js already exposes a same-origin rewrite at `/api/v1/*` that works fine.** Every in-app surface that depends on live data is effectively in `mock-offline` fallback; pages without a mock fallback (notably `/audit` and the non-Overview worker sub-tabs) hard-fail to a stuck-loading state or a 500 error shell — both of which the camera reads in scenes 2 and 6.

Headline blockers:
1. **F01 — Cross-origin gateway calls all CORS-fail in the browser.** Every `/api/v1/*` request from the SPA is rejected (`TypeError: Failed to fetch`), so the demo runs entirely on mock data. The same path through Next's rewrite returns 200 + 27 real audit rows.
2. **F02 — `/audit` page is permanently stuck in `audit-loading` state.** `useAuditFeed` never resolves (no network call observed even after 8 s); page body renders only the demo banner. Scene 6 of the v1.0 runbook drills into a row here.
3. **F03 — Worker sub-tabs other than `/overview` render the 500 error shell.** Visiting `/workers/<id>/duties` or `/workers/<id>/responsibilities` lands on "We couldn't load this worker." Scene 2 attempts to open Mariah's "Schedules" tab — that tab does not exist (the actual tab strip is Overview / Activity / Memory / Skills / Trust / Escalation / Settings).

## 6-MISSION-claim coverage rollup (based on whole-site SPA observation)

| Claim | Whole-site evidence | Verdict |
|---|---|---|
| **1 — Teammate, not tool** | Workers page renders 5 named teammates with hero, trait chips, "Great at" / "Escalate when" / signature behaviors (C2 personality), KPIs, channel strip — all wired from `SAMPLE_TEAM` via mock-offline. `/team` + `/workers/<id>/overview` show full personality, role, "Operations · Procurement" framing. **MVP works for the demo** as long as the recording stays on Mariah's Overview tab. | 🟢 functional in MVP |
| **2 — Delegation is the unit** | `/approvals` renders rich seeded inbox with severity tiers (P0/P1/P2), KPIs ($48k at risk, 43m median time, 67 % approval rate), and worker-attributed cards ("Peta wants to escalate the release-blocker thread", "Nori wants to apply a 32 % discount on QT-2024-7283"). Cards look like real delegated decisions. **Approval surface is camera-ready.** Schedule/Run-now affordance for triggering a delegation does NOT exist on Mariah's overview (4 duty rows render, but only schedule/edit/toggle controls — no "Run now"). | 🟡 functional except scene-2 "Run now" beat |
| **3 — Platform builds workers (Genesis)** | `/genesis` route renders the editorial-center "Describe a role" textarea + "Generate worker" button; supports plain-English entry. Cannot verify generation end-to-end because all gateway POSTs CORS-fail from the SPA. **The investor moment is mocked through.** | 🟡 surface exists, real call path broken by F01 |
| **4 — Skills shared + learned** | `/skills` marketplace renders 6 featured skill cards (Approval route, Invoice extract, Candidate screen, Deal qualify, Support triage, Lead enrich) with hero search, library tags, version pills, and a "Skills my workers learned this month" rail. Card design matches the locked A1 marketplace pattern. Click-through to install flow not exercised in this scope. | 🟢 catalog functional in MVP |
| **5 — Any-PC execution** | No SPA surface in scope exposes computer-use or MCP-substrate state directly; can't verify from web. The `/workflows` page shows a `customer_research_followup` template but the demo runbook owns that beat. | ⚪ out of whole-site scope |
| **6 — Multi-channel + multi-tenant** | App shell shows tenant ("Acmeco · Owner") + persona ("Sam Harlow"), every settings page shows "Settings changes are tenant-scoped" copy, audit copy mentions workspace isolation. `/settings/notify-channels` claims "11 approval channels ship with SKYCOT — email, chat, SMS, voice, push, and the web inbox." Channels are described, not interactively configured. **Story is told in copy + chrome, isolation cannot be verified from the SPA alone (function-side belongs to gateway agents).** | 🟢 told in copy; underlying isolation per cold-cargo evidence already 🟢 |

Net: 1 / 2 / 4 / 6 are **functionally MVP-ready from the SPA**; 3 is fronted but its end-to-end call CORS-fails; 5 is out of scope. The dominant SPA risk to the demo is F01 cascading into F02 and F03.

## All findings, in order

### F01 [P0] — SPA fetch client uses absolute gateway URL; every cross-origin call CORS-fails in the browser
The typed `apiGet` / `apiPost` wrapper prepends `NEXT_PUBLIC_GATEWAY_URL` (default `http://127.0.0.1:7400`) to every path, so the browser does cross-origin fetches. Curl preflights return 204 cleanly but the actual GET response from the gateway has no `Access-Control-Allow-Origin` header set for the page origin, so the browser rejects the response with `TypeError: Failed to fetch`. Confirmed in-page: `fetch('http://127.0.0.1:7400/api/v1/audit')` → "Failed to fetch", whereas the same-origin `fetch('/api/v1/audit')` (which goes through Next.js's rewrite to the gateway) returns 200 + 27 real audit rows.
- Reproduction:
  - Open `/audit` in Chrome
  - DevTools console → `fetch('http://127.0.0.1:7400/api/v1/audit')`
  - Compare to `fetch('/api/v1/audit')` (same response body, status 200, no CORS error)
- Files: `web/lib/api/client.ts:80-81` (`gatewayBase()` returns absolute URL) and `:179` (prepends it unconditionally); rewrite that should be used lives at `web/next.config.ts:30-32` (`{ source: "/api/v1/:path*", destination: "${gatewayUrl}/api/v1/:path*" }`)
- Suggested fix: make `gatewayBase()` return `""` (empty string → same-origin) for client-side requests, keeping the absolute URL only for SSR/route-handler contexts; alternative is to add CORS headers on the gateway, but the Next rewrite is already shipped and free.

### F02 [P0] — `/audit` page is permanently stuck in `audit-loading` skeleton
After 8 s wait + manual reload, the page renders only the demo banner; `data-testid="audit-loading"` is present, `audit-feed` and `audit-empty` are not. No network request to `/api/v1/audit` is ever made from the page (verified via Chrome network panel). Same-origin curl/fetch to `/api/v1/audit` returns 200 with 27 rows, so the gateway side is healthy. Root cause is F01: the audit fetch CORS-fails silently, and unlike `/workers` and `/cohorts`, the audit hook's `mock-offline` fallback path does not seem to fire (likely because `useInfiniteQuery`'s first page errors before the fallback in `fetchAuditFeed` triggers — needs trace under a fixed F01). Scene 6 of the runbook drills into a row here.
- Reproduction:
  - `open http://127.0.0.1:3000/audit`
  - Wait 10 s. Page stays empty except the banner.
  - DevTools → no fetch to `/api/v1/audit?*`
- Files: `web/app/(app)/audit/page.tsx:73` (`AuditPageContent`), `web/lib/api/audit.ts:345` (`fetchAuditFeed`), `web/lib/hooks/use-audit-feed.ts:28`
- Suggested fix: fixing F01 should unblock this; if not, add the same `isMissingOrUnreachable` + demo-mode mock branch to the `useInfiniteQuery` error path that `fetchAuditFeed` already has.

### F03 [P0] — Worker sub-tabs that aren't `overview` / `activity` / `memory` / `skills` / `trust` / `escalation` / `settings` render the 500 error shell
Visiting `/workers/<id>/duties` or `/workers/<id>/responsibilities` shows the worker hero, the tab strip, then a "500 — We couldn't load this worker." panel from `error.tsx`. The catch-all `[tab]/page.tsx` calls `notFound()` for unknown tab IDs, but Next.js routes that through the worker-detail error boundary instead of the not-found shell because the layout's `WorkerLayoutShell` is already mid-render. This matters because the v1.0 runbook (Scene 2) instructs the operator to open Mariah's "Schedules" tab — that tab name does not exist; "Schedules" / duties are surfaced as a card on the Overview tab.
- Reproduction:
  - `open http://127.0.0.1:3000/workers/018f67d0-0000-7000-8000-000000000001/duties`
  - Page renders hero + tabs + "500 — We couldn't load this worker."
- Files: `web/app/(app)/workers/[workerId]/[tab]/page.tsx:33` (calls `notFound()` for invalid tabs), `web/app/(app)/workers/[workerId]/error.tsx:8-30`, `web/components/skycot/WorkerTabs.ts` (defines `WORKER_TAB_LABELS`)
- Suggested fix: update the v1.0 runbook to navigate to Mariah's **Overview tab** (which already shows 4 duty rows with edit / toggle / schedule controls) and stop saying "click the Schedules tab"; OR add a "Schedules" or "Duties" tab as an alias under `WORKER_TAB_IDS` rendering the existing `card-duties` cluster as the panel content.

### F04 [P1] — 6 in-app routes inherit "Approvals" as their browser-tab title because the `(app)` route group's default title leaks
The `(app)` layout sets `metadata.title.default = "Approvals"` so routes that don't have their own `layout.tsx` metadata override (`/support`, `/retro`, `/orchestration`, `/verticals`, `/genesis`, `/billing`, `/brand`) all render `<title>Approvals · SKYCOT</title>`. Camera unlikely to read the OS tab bar in the demo, but it's nakedly broken for any screenshot, share, or browser-tab pinning. The `/brand` route is particularly bad because the page is operator-facing brand documentation; even worse, `/genesis` is the v1.0 demo headline per memory `project_pitch_agent_v1_demo`.
- Reproduction:
  - `for r in /support /retro /orchestration /verticals /genesis /billing /brand; do echo "$r → $(curl -sf -L http://127.0.0.1:3000$r | grep -oE '<title[^>]*>[^<]*</title>' | head -1)"; done`
  - All 7 print `<title>Approvals · SKYCOT</title>`
- Files: `web/app/(app)/layout.tsx:18-23` (sets `title.default = "Approvals"`); each affected route has no `layout.tsx` with its own metadata
- Suggested fix: change the default to `"SKYCOT"` (so the title template renders `<verb-or-page> · SKYCOT`) and let each route ship its own `metadata.title`; or add per-route layouts for the 6 affected pages.

### F05 [P1] — Worker detail browser-tab title is the raw UUID instead of the worker name
`<title>018f67d0-0000-7000-8000-000000000001 · Overview</title>` rather than `Mariah · Overview`. The actual page body shows "Mariah" correctly. Tab-title is fed from the URL slug rather than the hydrated worker name. Cosmetic for the camera but ugly for share-card / browser-history affordances and the user-menu's recently-visited list.
- Reproduction:
  - `open http://127.0.0.1:3000/workers/018f67d0-0000-7000-8000-000000000001/overview`
  - Browser tab title shows the UUID
- Files: `web/app/(app)/workers/[workerId]/layout.tsx` (no `generateMetadata`); could derive from the worker fetch the layout already does
- Suggested fix: add `generateMetadata({ params })` to that layout that calls `findSampleWorker(workerId).name` (with gateway as primary, mock as fallback) and renders `${name} · ${tabLabel}`.

### F06 [P1] — `/workers/{id}/duties` is the demo runbook's "click the Schedules tab" target but that tab doesn't exist
The v1.0 runbook (`docs/skycot/demos/v1-mission-mvp-runbook.md` Scene 2) tells the operator to open Mariah's "Schedules" tab. The actual tabs available are: Overview, Activity, Memory, Skills, Trust, Escalation, Settings. The duties / schedules data renders as a CARD ON THE OVERVIEW TAB (`data-testid="card-duties"` with 4 `duty-row-*` entries plus `duty-schedule-*` and `duty-edit-*` controls). No "Run now" affordance is wired — the controls are toggle / edit / schedule only. Compounded with F03 (the wrong-route hits 500), this is a documentation × implementation mismatch the demo will trip on.
- Reproduction:
  - Open Mariah's overview → 4 duty rows visible under "WHAT MARIAH OWNS" / "card-duties"
  - No "Run now" / "Trigger" button on any duty
  - The runbook's "Schedules tab → Run now → audit row lands in /audit within 5s" sequence cannot be executed
- Files: `docs/skycot/demos/v1-mission-mvp-runbook.md` (Scene 2 script); `web/components/skycot/worker-detail/cards/DutyCard.tsx` (or similar — duty-row testids); `web/components/skycot/WorkerTabs.ts` (tab definitions)
- Suggested fix: either (a) rewrite the runbook Scene 2 to stay on the Overview tab and use a different beat (e.g. "click 'Edit' on the re-price duty"), or (b) add a "Run now" button to each duty row that triggers a `/api/v1/workers/<id>/duties/<duty-id>/run` mutation.

### F07 [P1] — Hydration mismatch on every (app)-group page (Radix popover IDs)
React reports `aria-controls="radix-_R_..."` server-rendered ID differs from the client value on `appshell-user-menu-trigger` and `help-widget-trigger`. The mismatch fires on every page in the (app) group at load. React's console error is shown to anyone with DevTools open during the screen recording; it also "won't be patched up" per React's wording, leaving the buttons with mismatched `aria-controls` for assistive tech.
- Reproduction:
  - DevTools open → `open http://127.0.0.1:3000/workers`
  - Console shows `A tree hydrated but some attributes of the server rendered HTML didn't match the client properties` with `+ aria-controls="radix-_R_7lmitmrlb_" / - aria-controls="radix-_R_7lqitmrlb_"`
- Files: `web/components/skycot/AppShell.tsx` (renders the `appshell-user-menu-trigger` `<Popover>`), `web/components/ui/HelpWidget.tsx` (the `help-widget-trigger`)
- Suggested fix: wrap the trigger contents in a `useId()`-based wrapper, OR mark the AppShell user-menu and help widget triggers as client-only via `dynamic(() => ..., { ssr: false })` so the server doesn't allocate Radix IDs.

### F08 [P1] — `.env.local` sets `ARGO_GATEWAY_AUTH_BYPASS=1` but the bypass key is `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`
The `.env.local` ships with `ARGO_GATEWAY_AUTH_BYPASS=1`, missing the `NEXT_PUBLIC_` prefix, so the dev `isAuthBypassed()` check across `web/lib/api/*.ts` returns false. The implicit consequence: every page that has a mock-bypass branch quietly falls through to the real network call, then to the mock-offline fallback on CORS error, then renders mock data. Works by accident in dev because of the cascade, but it means the "auth-bypassed dev mode" the briefs describe is never the active path.
- Reproduction:
  - `grep BYPASS web/.env.local` → `ARGO_GATEWAY_AUTH_BYPASS=1` (no `NEXT_PUBLIC_` prefix)
  - `grep -n "NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS" web/lib/api/*.ts` → 8 hits, all of which never trigger
- Files: `web/.env.local:5`, the bypass checks at `web/lib/api/audit.ts:64`, `web/lib/api/team.ts:49`, `web/lib/api/workers.ts:103`, etc.
- Suggested fix: rename in `.env.local` to `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`.

### F09 [P1] — `/workflows/templates` browser-tab title drops the SKYCOT suffix
`<title>Workflow templates</title>` (no "· SKYCOT") because the parent `/workflows/layout.tsx` sets `title: "Workflows"` as a literal string, which Next.js's metadata composition interprets as closing the title template chain for descendants without their own template. Compare: `/workflows` itself renders `Workflows · SKYCOT`, `/workflows/templates` renders `Workflow templates`.
- Reproduction:
  - `curl -sf -L "http://127.0.0.1:3000/workflows/templates" | grep -oE '<title[^>]*>[^<]*</title>'`
  - Returns `<title>Workflow templates</title>`
- Files: `web/app/(app)/workflows/layout.tsx:5` (`title: "Workflows"` — should be `title: { default: "Workflows", template: "%s · SKYCOT" }`), `web/app/(app)/workflows/templates/layout.tsx:5`
- Suggested fix: in `workflows/layout.tsx`, change to `title: { default: "Workflows", template: "%s · SKYCOT" }` so the sub-route inherits the template.

### F10 [P1] — `/onboarding/welcome` uses the root marketing title instead of a wizard-step title
Browser tab shows `SKYCOT — Hire your first AI teammate` (root layout default) rather than `Welcome · SKYCOT` or step-aware `Step 1 of 5 · Welcome · SKYCOT`. The onboarding wizard sits outside the `(app)` group so it doesn't inherit the in-app default; it also doesn't override. Minor for the camera but noticeable when an operator pins the tab during a real signup flow.
- Reproduction:
  - `open http://127.0.0.1:3000/onboarding/welcome`
  - Tab title is the marketing default
- Files: `web/app/(setup)/onboarding/welcome/page.tsx` (no `metadata` export), `web/app/(setup)/onboarding/layout.tsx` if it exists
- Suggested fix: add `export const metadata = { title: "Welcome · Onboarding" }` (and similar per step), or set the title via the wizard shell that already knows the step.

### F11 [P1] — `/cohorts` SSR flash shows "Loading cohorts…" before mock-fallback hydration paints 8 cohorts
First paint shows the empty-state hero ("Build your first cohort.") with subtitle "Loading cohorts…" + repeated "Loading cohorts…" in the grid. ~700 ms later the client-side TanStack query falls back to `SAMPLE_COHORT_SUMMARIES` and the magazine grid (8 cohorts, 5 dept tabs) snaps in. Camera operator will catch the swap on slow networks. The empty-state component does the right thing for never-onboarded tenants but the brief loading flash undermines "wow, look at all these cohorts I have" framing in scene 5.
- Reproduction:
  - Hard reload `/cohorts`
  - Observe brief flash of "Loading cohorts…" between SSR and hydration
- Files: `web/app/(app)/cohorts/page.tsx:80-86` (the subtitle ternary that renders "Loading cohorts…" while `cohortsQuery.isPending`), `web/components/skycot/cohorts/CohortMagazineSkeleton.tsx`
- Suggested fix: when `isPending` AND demo-mode is on, pre-render with `SAMPLE_COHORT_SUMMARIES` so the SSR pass already shows the magazine grid. Or short-circuit to the empty-preview groups on SSR.

### F12 [P1] — `/settings/{branding,integrations,danger}` return 404 despite being listed in the operator's scope
The operator's audit brief listed `branding`, `integrations`, `danger` as expected sub-tabs; none exist. The Settings layout DOES render a "Danger zone" copy block as a static aside on every sub-page, but no `/settings/danger` route. The closest analog to "integrations" is `/settings/connectors`, but the operator's mental model expects both. Either reroute or rename. Not a demo blocker (scene 6 doesn't go here) but a polish gap for the post-demo product surface.
- Reproduction:
  - `curl -sf -L -o /dev/null -w "%{http_code}\n" http://127.0.0.1:3000/settings/branding` → 404
- Files: `web/components/skycot/settings/SettingsTabs.tsx:31-71` (tab definitions); no danger / branding / integrations directories
- Suggested fix: either add the three pages (placeholder OK), or rewire `/settings/integrations` → `/settings/connectors` and call branding "appearance" inline under `/settings/workspace` (which is where the appearance section already lives per the SSR text).

### F13 [P2] — `/changelog` "this week" header reads `May 18 – May 20, 2026` even though today is May 27
The page-leading rail says "What we shipped this week" with date range `May 18 – May 20, 2026`, which is ~7 days stale. Investors and SOC 2 reviewers tend to read changelog freshness as a proxy for product velocity. Auto-update would be the right answer but a manual refresh before recording is also fine.
- Reproduction: open `/changelog`, eyeball the top date rail
- Files: `web/app/changelog/page.tsx` or a feeder JSON / MDX inventory file under `web/content/changelog/` (couldn't grep specifically without diving deeper)
- Suggested fix: pull the latest entry's date dynamically and label the rail "Latest" or "This sprint" instead of "This week".

### F14 [P2] — Marketing nav says "Roster" but the link goes to `/team` (in-app surface), not a marketing roster page
The marketing header anchors include "Roster" → `/team`. Click-through lands a signed-out marketing visitor on the in-app `/team` shell ("YOUR AI WORKFORCE · LIVE", "Hire your first teammate. No teammates yet."). It's an awkward jump from marketing register to in-app empty-state. Either gate `/team` behind auth, or build a `/roster` marketing page that mirrors the v1 worker roster aesthetically.
- Reproduction: open `/landing`, click "Roster", land on `/team`
- Files: `web/components/skycot/marketing/MarketingHeader.tsx` (or similar; the `<a href="/team">Roster</a>` line)
- Suggested fix: change the link to `/customers` or a dedicated `/roster` marketing page; OR gate `/team` so it bounces unauthenticated traffic to `/auth/signin?returnTo=/team`.

### F15 [P2] — `/workflows` shows "Loading workflows…" footer simultaneously with rendered workflow cards
Visible body includes both seven workflow cards (post_listing, screen_candidates, etc.) AND a "Loading workflows…" subtitle. The skeleton state and the populated state are mounted at the same time. Cosmetic but it looks like a stuck spinner overlay on a working page.
- Reproduction:
  - `open http://127.0.0.1:3000/workflows`
  - Search the visible text — "Loading workflows…" appears between header and the workflow grid
- Files: `web/app/(app)/workflows/page.tsx` (search "Loading workflows")
- Suggested fix: gate the "Loading workflows…" element on `isPending && data == null`; once mock-offline data arrives, hide the loader.

### F16 [P2] — `/compare` and `/changelog` ship a single `@media` breakpoint each; behavior at 375 / 768 / 1920 unverified
Both pages have only one `@media` block in their CSS file (`/compare/page.css` and `/changelog/page.css`), suggesting they were built desktop-first with a single mobile-breakpoint pass. Investor demo recording stays at 1440 so this isn't blocking, but customer-facing share links on phones will hit awkward layouts.
- Reproduction: `grep -c "@media" web/app/compare/page.css web/app/changelog/page.css` → 1 each
- Files: `web/app/compare/page.css`, `web/app/changelog/page.css`
- Suggested fix: add 768 + 480 breakpoints to both (these are marketing pages, the social share is the largest source of mobile traffic).

### F17 [P2] — Hardcoded "2,847 companies this month" + "60 hours saved" / "935 actions/wk" stats render alongside actual data
Onboarding welcome ends with `"2,847 COMPANIES THIS MONTH"`; workers page header carries `"0 working right now · 935 actions this week · saving you ~60 hours."`. Per the function-vs-dressing rule, dressing-side stats that the camera doesn't drill into are FINE. Flagging because (a) "0 working right now" combined with "935 actions/wk" reads inconsistent (zero workers can't have generated 935 actions), and (b) the "2,847 companies" rounds well above the actual sign-up base, which is awkward at any investor-facing depth-of-engagement. Not a POC blocker because the camera never drills these.
- Reproduction: open `/workers` and `/onboarding/welcome`, look at the KPI strip
- Files: `web/app/(setup)/onboarding/welcome/page.tsx` (the `2,847` block), `web/app/(app)/team/page.tsx` (the `935` / `60` block)
- Suggested fix: make the workers strip respond to the actual sample data (sum the actions, count of statuses), or drop the strip entirely so the trading-card-deck does the heavy lifting.

### F18 [P2] — `/help` SEO-redirects to `/docs` (intentional) but the marketing nav still anchors to `/docs` directly, not `/help`
Per `web/app/help/page.tsx`, the `/help` route 307s to `/docs` until the in-product help center ships. The marketing header already links to `/docs`, so the redirect mostly exists as a hedge for inbound third-party links. Worth noting only because the audit brief listed `/help` as an expected page — it works (via redirect) but doesn't have its own content.
- Reproduction: `curl -sI http://127.0.0.1:3000/help` → 307 to `/docs`
- Files: `web/app/help/page.tsx:18`
- No suggested fix — this is the operator-intended behavior per the doc-comment.

### F19 [P2] — Mariah's overview "duties" cards use the word "duty" / "duties" in test IDs but the operator-locked UI vocab is "Schedule"
Per memory `project_vocabulary_duty_vs_schedule`, "duty" stays in code/SQL/audit/brief filenames, "Schedule" is operator-facing. The `card-duties`, `duty-row-*`, `duty-toggle-*`, `duty-edit-*`, `duty-schedule-*` test IDs are code, so they're allowed to stay "duty". But the visible card title at runtime needs verification — I confirmed the visible label on the worker overview reads `"WHAT MARIAH OWNS"` (good — neither "Duties" nor "Schedule"), but the card subtitle / column header may still leak "Duties". Couldn't confirm without unblocking the redacted page text from Chrome.
- Reproduction: open Mariah's overview, look at the card heading + sub-elements; check for "Duties" / "Duty" strings
- Files: `web/components/skycot/worker-detail/cards/DutiesCard.tsx` or similar
- Suggested fix: grep the duty card components for visible "Duty" / "Duties" strings, replace with "Schedule" per the locked vocab.

### F20 [P3] — Demo banner sticky bar dominates the viewport top on `/onboarding/welcome`, conflicting with the F1 editorial-center hero
The demo banner (`"Demo mode — showing sample data. Sign in to see your own workspace."`) sits above the F1 editorial-center hero on most app-group routes. It's branded (purple→pink gradient) and well-designed, but it cuts ~60 px out of the editorial whitespace the F1 design depends on. The (setup) group at `/onboarding/welcome` doesn't render this banner (good — the hero is unmolested there); for the app group, consider hiding the demo banner during scene-recording.
- Reproduction: open `/workers`, `/team`, `/skills`, `/audit` — all carry the banner; `/onboarding/welcome` doesn't
- Files: `web/components/skycot/DemoBanner.tsx`
- Suggested fix: dismissible already (`demo-banner-dismiss` testid). Add `localStorage.setItem('skycot-demo-banner-dismissed','1')` to the pre-record checklist.

### F21 [P3] — Marketing `/changelog` has 96 entries but no pagination / "older entries" affordance — scroll-only
The page renders 96 timeline entries inline, which makes the scroll fairly long for a long-tail SEO surface. Not blocking; cosmetic and search-engine-relevant.
- Reproduction: `open /changelog`, page-up / page-down through the 96 entries
- Files: `web/app/changelog/page.tsx`
- Suggested fix: paginate at 20 per page with `?page=2` query strings — useful for both perf and search-result link variety.

### F22 [P3] — `/customers` 5-card grid is consistent but customer SVG logos sometimes degrade to initial-only avatars when the logo file is missing
Spot-checked /customers — saw "NC", "BH", "MF", "AR", "SE" initial-avatars rather than logo SVGs (`<img>` tags would have rendered if the files were present). Per memory `feedback_logos_from_svgl_app`, customer/integration logos should be sourced from svgl.app and stored at `web/public/logos/`. The five customer companies in the carousel are fictional (per the locked function-vs-dressing rule), so initials are arguably the more honest visual. Flagging only as a future-polish nit.
- Reproduction: open `/customers`, look at customer logo placeholders
- Files: `web/app/customers/page.tsx`, `web/public/logos/`
- Suggested fix: keep initials for the v1.0 demo (honest about fictional logos); replace with svgl.app sourcings once we have real signed customers.

## Resolved since last audit

No prior whole-site audit exists at this path. The 2026-05-25 pre-record-ux-audit was operator-rejected ("it's shit") and not used as a source.

## Orientation score

**72 / 100**.

Breakdown: marketing surface is camera-ready (8/10), in-app teammate / cohort / skills / approvals surfaces are visually locked + mock-fallback-functional (8/10), but the F01-F03 cluster wipes 25 points off the load-bearing demo-recording capability. The audit page being permanently empty (F02) is the single biggest hit — Scene 6 of the runbook lands there and clicks a row. Worker-detail Schedules-tab doesn't exist (F06) requiring runbook revision before Scene 2 records. Fixing F01 (which is a 1-line code change in `gatewayBase()`) most likely cascades-resolves F02 and parts of F03 simultaneously, lifting the score to ~85.

Recommended pre-record actions in priority order:
1. Fix F01 (gateway URL prefix) — single point of failure for the whole live-data spine
2. Revisit F02 if F01 didn't auto-resolve it
3. Rewrite Scene 2 of the v1.0 runbook to stay on Mariah's Overview tab (F06)
4. Patch F04 + F05 (browser tab titles — quick wins, camera-adjacent)
5. Dismiss the demo banner during recording (F20)
