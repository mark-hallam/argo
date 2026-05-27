# Whole-site MVP claims-coverage rollup — 2026-05-27 09:49 Melbourne (T0819 batch)

**Scope**: every SPA route NOT in the 4 sibling agents' hot paths. Marketing (`/landing`, `/pricing`, `/customers`, `/compare`, `/security`, `/sla`, `/changelog`, `/privacy`, `/terms`, `/for/*`, `/docs`, `/help`), in-app surfaces outside the 6-scene runbook (`/cohorts`, `/cohorts/new`, `/settings/*` 11 sub-pages, `/brand`, `/brand/components`, `/team`, `/workflows/templates`, `/verticals`, `/genesis`, `/retro`, `/orchestration`, `/support/inbox`, `/billing`, `/billing/subscribe`, `/skills-canvas`), theme provider wiring, viewport-responsiveness markers, plus the load-bearing **6-MISSION-claim coverage rollup**.

**Method**: gateway probes (`curl http://127.0.0.1:7400`), Next.js dev-server probes (`curl http://localhost:3000`), source-of-truth `.tsx` + brief-ledger inspection (read-only — no cargo, no Next.js prod build). The preview headless browser locked up early in the run and the dev server has been compiling routes 30-200s per cold hit, so a chunk of evidence is curl + `.tsx`-source rather than DOM-snapshot.

**Working tree**: branch `main`, gateway 7400 schema **v83** (unchanged from T0623), HEAD `d9455b091` "fix(web): T0519MEL P0s — fetchworker 400 + duty toggle E4-h leak" (operator-direct fix landed at T0913MEL, ~36 min before this audit window opened).

**Time since prior whole-site audit**: 86m (T0623 → T0819). Since T0623: 1 SPA fix-commit landed (`d9455b091` closed 2 of the 3 T0519MEL P0 demo-blockers — fetchworker 400 retry-storm + the duty-toggle E4-h refactor); 1 backend ops commit confirmed Claim 5 LIVE (`a2642ac02` claim-5 cold rerun mcp+cu post macos perms PASS).

---

## TL;DR

18 findings: **3 P0** (gateway skills + verticals + workflows seed still empty 162m after #2407 — same root-cause persists; audit feed is 10/10 SPA self-traffic rows the camera will drill into during Scene 6; default "Approvals" tab-title still leaks across 8+ in-app pages including the investor-moment `/genesis`), **6 P1** (`/workers` collection 404 forces SPA fictional fallback; `/team` 2 H1s; 7 of 11 settings sub-pages still missing metadata; `/billing/subscribe` H1 only on the error path not success; `/sla` no metadata; `/for/*` titles double-suffix `— SKYCOT · SKYCOT`), **6 P2** (settings panels read-only-no-onChange across workspace/providers/notifications; `/workflows/templates` title breaks `· SKYCOT` chain; `/cohorts` SSR shows empty-preview pre-hydrate; `/genesis` H1 mismatches Studio scope; `/retro` SSR shows "Loading retros…"; team hero KPI numbers are `workers.length × 187/12` made-up arithmetic), **3 P3** (`/skills-canvas` flat 404 from stale references; turbopack `inner_of_upper_lost_followers` panic in dev logs; `/cohorts/new` 92s first-compile in dev).

**Direction vs T0623**: 2 prior P0s RESOLVED (`/docs` 6 dead subroutes — all 6 now exist with `page.tsx` + 4 of the 4 featured-card nested pages exist; `/help` placeholder dead-link page replaced with server-side 307 → `/docs`). 1 prior P1 RESOLVED (worker fetch 400 retry-storm per `d9455b091`). 1 prior P0 NET-NEW PROMOTION (the audit-feed self-traffic ratio is now 10/10 not 5/5 — slightly worse, same root cause). All other prior findings unchanged.

**6-MISSION-claim functional verdict from the SPA, T0819**:
- 1 = 🟡 **LIKELY-LIVE** (1 real worker "Maya" via `/api/v1/team`; `/api/v1/workers` collection STILL 404 → SPA fixture-roster fallback)
- 2 = ❓ **NOT-OBSERVABLE-FROM-SPA** (no `/handoffs` UI; audit drill-in is the only surface and feed is 100% SPA self-traffic)
- 3 = 🟡 **LIKELY-LIVE** (5 onboarding steps SSR; `/api/v1/verticals` empty so `/onboarding/pick-vertical` reads SEED_VERTICALS fixture; `/genesis` Studio H1 still says "Hire your next teammate")
- 4 = 🔴 **BROKEN-IN-SPA** (skills catalog STILL empty 162 minutes after #2407 — same root-cause brief `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution` queued P0, still TODO)
- 5 = 🟡 **LIKELY-LIVE-via-fixtures** (claim-5 substrate cold PASS via #2434 post-macOS-perms; `/workflows/templates` renders 50+ static cards from `web/lib/workflow-templates.ts`; `/api/v1/workflows?vertical=sales` returns 0 — runtime engine has no live workflows; computer-use end-to-end no longer gated on perms but the SPA-side surface still has nothing to drill into)
- 6 = 🟡 **LIKELY-LIVE** (`/settings/connectors` + `/settings/notify-channels` sources render; audit endpoint serves rows but 10/10 are SPA self-traffic — function-side trust signal **degraded vs T0623 5/5**)

**Direction vs T0623** (86m prior): Claim 5 flipped 🟡→🟡-better (cold-rerun PASSED via #2434; substrate now LIVE — but SPA-side observability didn't improve). Claim 4 flat 🔴 (the root-cause boot-seed brief is still TODO — has been TODO 162m). Claims 1/2/3/6 flat. The d9455b091 patch is a real-fix that closed a 15s spinner on a worker-detail navigation — the camera now has one fewer footgun in Scenes 1-2.

**Single biggest unlock for the demo**: **STILL** claim the P0 `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution` brief. It's been TODO 162 minutes across two audit cycles. Same root-cause unblocks F1 (Scene 4) AND F8 (Scene 3 verticals) AND F17 (Scene 5 workflows). One claim, three claims unblocked.

---

## Headline blockers

- **F1 [P0] — `/skills` catalog STILL EMPTY at T0819, 162m after #2407** — `curl -s http://127.0.0.1:7400/api/v1/skills` → `{"skills":[],"tenant_id":"019e5057-1d3f-7fa2-9c1b-c34ef01cc21e","total_count":0}`. Same for `?q=competitor`, `?q=pricing`, `?q=research`, `?q=email`. Scene 4 unrecordable. Same root-cause brief queued by T0519 synthesis (`fix-gateway-skills-and-verticals-boot-seed-tenant-resolution`), still TODO after 2.5 hours.
- **F2 [P0] — `/api/v1/audit` returns 10 of 10 rows = SPA self-traffic** (was 5/5 at T0623, now 10/10 = degraded). Every row: `verb=http.request`, `worker.name=unknown`, `action_type` is `audit.detail.viewed` or `audit.replay`. Scene 6 is the audit drill-in; the camera will click a row and read "audit.detail.viewed by unknown". This is theatre the camera sees — function-side fabrication, **P0 for the demo**. The brief `fix-worker-activity-endpoint-filter-spa-self-traffic` was claimed (per `b48ac31c3` commit) but the audit-feed parallel brief was not — the audit endpoint still bleeds the SPA's own auth/replay pings.
- **F3 [P0] — `(app)/layout.tsx:20` default title "Approvals" STILL leaks across 8+ in-app pages** — confirmed unchanged at T0819 via direct file read. The (app) group metadata still declares `title: { default: "Approvals", template: "%s · SKYCOT" }`. 8 confirmed leak surfaces: `/brand`, `/brand/components`, `/retro`, `/genesis`, `/orchestration`, `/verticals`, `/support/inbox`, `/billing`, `/billing/subscribe`. Worst case: **`/genesis` (the investor-headline Studio per memory `project_pitch_agent_v1_demo`) reads "Approvals · SKYCOT" in the tab bar.** The fix is mechanical (8 × 4-line layout.tsx additions + change default to "SKYCOT").

---

## All findings, in order

### Public marketing pages

#### F4 [P1] — every `/for/<vertical>` page has a duplicate `· SKYCOT` suffix in `<title>` (NET-NEW vs T0623)
The root `web/app/layout.tsx` metadata declares `title: { default: SKYCOT_TITLE_DEFAULT, template: "%s · SKYCOT" }`. Each `/for/<vertical>/layout.tsx` then exports `title: V.title` where V.title already ends in "— SKYCOT" (e.g. "AI sales teammates that never sleep — SKYCOT"). Next.js applies the template on top, producing "AI sales teammates that never sleep — SKYCOT · SKYCOT".

Verified live via curl:
- `/for/sales` → `AI sales teammates that never sleep — SKYCOT · SKYCOT`
- `/for/support` → `AI customer support teammates — SKYCOT · SKYCOT`
- `/for/ops` → `AI ops teammates — Run the back office on autopilot — SKYCOT · SKYCOT`
- `/for/finance` → `AI finance teammates — Close the books faster — SKYCOT · SKYCOT`
- `/for/recruiting` → `AI recruiting teammates — Hire faster, screen better — SKYCOT · SKYCOT`
- `/for/marketing` → `AI marketing teammates — Always-on, never off-brand — SKYCOT · SKYCOT`

Public marketing trust signal: every vertical landing page has a malformed title. Bad for SEO and for any browser-tab QA shot.

Repro: `for v in sales support ops finance recruiting marketing; do curl -s "http://localhost:3000/for/$v" | grep -oE '<title>[^<]+</title>'; done`.

Fix: drop the "— SKYCOT" suffix from each entry in `web/app/for/_data/verticals.ts` (the template will append it automatically). 1-line × 6 places.

File: `web/app/for/_data/verticals.ts`.

#### F5 [P1] — `/sla` STILL falls back to global default title (T0623 F5 unresolved)
`web/app/sla/page.tsx` has no `export const metadata`. The HTML `<title>` reads the root layout default "SKYCOT — Hire your first AI teammate" rather than `SLA · SKYCOT`. Same trivial fix queued at T0519 and again at T0623, still unfixed. 4-line addition.

File: `web/app/sla/page.tsx`.

#### F6 [P0→RESOLVED] — `/docs` no longer fans out to 6 dead 404 sub-pages (T0623 F2 RESOLVED)
At T0623 the prior agent reported `/docs` linked 6 cards to subroutes that 404'd. Verified at T0819: all 6 subroute pages exist (`/docs/getting-started/page.tsx`, `/docs/workflows/page.tsx`, `/docs/skills/page.tsx`, `/docs/audit/page.tsx`, `/docs/api/page.tsx`, `/docs/integrations/page.tsx`), plus 4 featured-card nested pages (`hire-first-teammate`, `visual-canvas`, `trust-ratchet`, `slack-approvals`). No longer a blocker.

#### F7 [P0→RESOLVED] — `/help` placeholder dead-link page replaced with 307 → `/docs` (T0623 F08 RESOLVED)
`web/app/help/page.tsx` is now a server-side `redirect("/docs")` with a comment block explicitly closing the "F08" finding from a prior round. Clean fix.

#### F8 [P3] — `/customers` logos still stub avatars (T0623 F6 unresolved)
Per memory `feedback_logos_from_svgl_app`: customer logos must be SVGs from svgl.app. Still placeholder badges for the 5 fictional customers on `/customers`. Per function-vs-dressing, customer logos are pure dressing — the POC demo never shows `/customers`. Flag-only.

### In-app pages (outside 6-scene hot path)

#### F9 [P1] — `/api/v1/workers` collection endpoint STILL 404 (T0623 F7 unresolved)
- `curl -s --max-time 30 http://127.0.0.1:7400/api/v1/team` → 1 real worker "Maya" (id `019e5dbb-c03d-79a0-a9a4-ee60bb3c8218`, vertical `marketing-intern`)
- `curl -s http://127.0.0.1:7400/api/v1/workers` → `{"error":"not found"}` (instant)

`d9455b091` fixed the fetchWorker 400 retry-storm on the per-worker endpoint, but the COLLECTION endpoint is still 404. SPA `/workers` grid falls back to `web/lib/sample-workers-detail.ts` (the 5 personas Mariah/Peta/Priya/Jamal/Nori), while `/team` separately reads `/api/v1/team` (Maya). Two parallel rosters in the same product surface.

File: `crates/argo-cli/src/handlers/gateway/routes_workers_list.rs` (or the team-vs-workers alias resolver).

#### F10 [P1] — `/team/page.tsx` STILL has 2 `<h1>` elements on one page (T0623 F10 unresolved)
`grep -n '<h1' web/app/(app)/team/page.tsx` → 2 H1s present: line 55 in the `TeamPageFallback` (the Suspense fallback) and line 250 in `PageHeader`. Plus the page renders a `<BrandLoudHero>` above which itself renders an `<h1>` at `web/components/skycot/BrandLoudHero.tsx:227` — so the rendered page has the BrandLoudHero H1 + the sticky-PageHeader H1, two H1s in the DOM.

Repro: `grep -n '<h1' web/app/(app)/team/page.tsx; grep -n '<h1' web/components/skycot/BrandLoudHero.tsx` → 3 matches across the rendering paths, 2 of which collide on `/team`.

Fix: drop the secondary "Team" H1 in `PageHeader` OR demote to `<h2>`. Mechanical.

File: `web/app/(app)/team/page.tsx:250-252`.

#### F11 [P1] — 7 of 11 `/settings/*` sub-pages STILL have no metadata export (T0623 F11 unresolved)
Verified at T0819 via `grep -l "export const metadata" web/app/(app)/settings/*/layout.tsx`:
- Metadata exports present: `/settings/api-keys`, `/settings/data-export`, `/settings/members`, `/settings/security` (4)
- Missing: `/settings/billing`, `/settings/connectors`, `/settings/knowledge`, `/settings/notifications`, `/settings/notify-channels`, `/settings/providers`, `/settings/workspace` (7)

Each missing-metadata page renders "Settings · SKYCOT" (the `/settings/page.tsx` re-exports WorkspaceSettingsPage so the root `/settings` title leaks via its own layout) OR falls through to the (app) "Approvals" default for sub-routes. Mixed bag depending on which fall-through hits first.

Fix: 7 × 4-line `layout.tsx` additions.

#### F12 [P2] — `/workflows/templates` title doesn't get `· SKYCOT` suffix (T0623 F12 unresolved)
Verified via `curl -s http://localhost:3000/workflows/templates | grep title` → `<title>Workflow templates</title>` (no suffix). The (app)/layout.tsx template `"%s · SKYCOT"` only applies when the child layout uses `title: { default: ... }` or a bare string that the template wraps. The `workflows/templates/layout.tsx:5` uses a bare string `"Workflow templates"` — should be `title: { absolute: "Workflow templates · SKYCOT" }` OR just rely on the parent template (verify behavior).

File: `web/app/(app)/workflows/templates/layout.tsx:5`.

#### F13 [P2] — Settings panels are read-only-with-hardcoded-values; no onClick/onChange on 3 of 4 user-editable surfaces
`/settings/workspace/page.tsx`, `/settings/providers/page.tsx`, `/settings/notifications/page.tsx`: zero `useState`, zero `onClick`, zero `onChange`. Values are hardcoded React strings: "SKYCOT Demo", "Marketing operations", "Australia/Melbourne", "Anthropic". The page is presented as if these are configurable settings; the operator can't actually change anything. **Per function-vs-dressing, settings is dressing for the POC demo (camera doesn't drill in)** but it's a customer-onboarding gap that will hit the moment a real customer signs up.

The 4 settings sub-pages that ARE functional: `/settings/api-keys` (real CRUD via `useApiKeys`/`useCreateApiKey`/`useRevokeApiKey`), `/settings/data-export`, `/settings/members`, `/settings/security`.

Fix scope: each of the 3 read-only panels needs a real form + a wired gateway endpoint. Not a demo blocker; tag for post-v1.0 polish.

File: `web/app/(app)/settings/{workspace,providers,notifications}/page.tsx`.

#### F14 [P2] — `/cohorts` SSR shows `CohortsEmptyPreview` even with 1 real cohort (T0623 F13 partially-improved)
T0623 noted the body now gates on `cohortsQuery.isPending` (improvement), but the **hero subtitle pluralisation and `total === 0` hero-title check** still trigger on the SSR pass where `cohorts.length === 0`. So the first paint of `/cohorts` shows "0 cohorts across the fleet" before hydrating to "1 cohort". 200-500ms FOUC.

Fix: gate hero copy on `cohortsQuery.isSuccess && cohorts.length === N` rather than the raw count.

File: `web/app/(app)/cohorts/page.tsx:80-100`.

#### F15 [P2] — `/genesis` hero H1 mismatches operator-clarified Studio scope (T0623 F16 unresolved)
Per memory `feedback_genesis_is_ai_teammate_onboarding_studio` (operator-clarified 2026-05-25T11:40Z): Genesis is the AI Teammate Onboarding Studio (voice/text role + SOP-PDF/wiki/Slack uploads + access-controls + comprehension verification). Current H1 at `web/app/(app)/genesis/page.tsx:250` renders `<BrandLoudHero title="Hire your next teammate.">` → DOM emits `<h1>Hire your next teammate.</h1>`. The H1 reads as hire-CTA, not Studio. For the investor-demo narrative this is a 2-word mismatch.

Plus the same page leaks "Approvals · SKYCOT" in the tab title (per F3) — the investor-headline page has the wrong tab title.

Fix: change `title="Hire your next teammate."` → `title="Onboard a new teammate."` AND add a layout.tsx exporting `metadata.title = "Genesis"`.

File: `web/app/(app)/genesis/page.tsx:250`.

#### F16 [P1] — `/billing/subscribe` has H1 only on the error path, not the success path (T0623 F9 partially-revised)
`grep -n '<h1' web/app/(app)/billing/subscribe/page.tsx` → 1 match at line 64: `<h1>Couldn't start checkout</h1>`. The success path (the page renders before the Stripe redirect fires) has no H1. Accessibility violation on the happy path; tab title also reads "Approvals · SKYCOT" per F3.

Fix: add a top-of-document H1 on the loading/success path ("Redirecting to checkout…") + add the missing `export const metadata`.

File: `web/app/(app)/billing/subscribe/page.tsx`.

#### F17 [P2] — `/api/v1/workflows?vertical=sales` returns 0 workflows; `/workflows/templates` renders 50+ static cards anyway
The runtime workflows endpoint is empty (same root-cause cluster as F1 + F22), but `/workflows/templates` reads from `web/lib/workflow-templates.ts` — a hardcoded list of 50+ template entries. Scene 5 narration depends on the template gallery as a discovery surface and the "Use template" CTA hitting `from_template=<id>` which IS wired to the builder (`resolve-workflow.ts:10`). So the template gallery itself is dressing (decorative seed) but the click-through is real function.

For Scene 5: the demo can record the click-through. The risk is that runbook narration says `customer_research_followup` while the static template list calls it `competitor_intel_digest` (per T0623 F17 sub-finding) — needs runbook/code reconciliation.

Cross-ref: T0519 brief `fix-workflow-execute-csrf-token-wiring-verify-before-demo` (P1, TODO since T0519) is the closest queued mitigation but addresses the run-now POST not the template list.

File: `web/lib/workflow-templates.ts`.

#### F18 [P2] — Team hero shows fake KPIs: `weeklyActions = workers.length × 187`, `hoursSaved = workers.length × 12`
`web/app/(app)/team/page.tsx:151-152`: the brand-loud hero on `/team` shows "Actions/wk" and "Hours saved" KPIs computed from a multiplication on the worker count rather than any real activity data. With 1 real worker (Maya) the hero will read "187 actions this week · 12 hours saved" — both fictional. Per function-vs-dressing, KPI numbers on a card the camera doesn't drill into are dressing-OK; **the camera doesn't drill into `/team` in any of the 6 scenes** so this is P2 polish not a demo blocker.

Fix: either read real activity from the gateway or label as `Sample —` until backed.

File: `web/app/(app)/team/page.tsx:151-152`.

#### F19 [P2] — `/retro` SSR renders "Loading retros…" with no skeleton (T0623 F18 unresolved)
`web/app/(app)/retro/page.tsx:142` has a literal "Loading retros…" string with no skeleton or empty-state fallback. Out-of-runbook path; polish.

#### F20 [P3] — `/skills-canvas` flat route 404 (T0623 F19 unresolved)
Flat route doesn't exist; only `/workers/[workerId]/skills/[skillId]/canvas`. The e2e agent prompt + a couple of stream briefs reference `/skills-canvas`. Either redirect or update refs.

#### F21 [P3] — Turbopack panic in dev-server logs during this audit window
Captured in preview_logs:
```
thread 'tokio-runtime-worker' panicked at .../turbopack/.../aggregation_update.rs:1572:17:
inner_of_upper_lost_followers is not able to remove followers ... from TaskId { id: 867886 } as they don't exist as upper or follower edges
```
This is a Next.js / Turbopack 16 dev-mode bug, not an app bug, but it caused at least one route compile to wedge (the route eventually completed). Not a production concern. Flag only.

#### F22 [P3] — `/cohorts/new` cold-compile took 92.2s and routinely takes 60-86s per request in dev
Dev-mode only; `preview_logs` shows `Compiled /cohorts/new in 92.2s` then `GET /cohorts/new 200 in 65994ms / 85992ms / 57578ms`. This is Turbopack dev cold-compile pain, not a real perf issue. Production build is unaffected. Mentioned because it materially impedes pre-record QA — operators clicking around the SPA hit 60-90s waits.

### Theme + viewport + console hygiene

- **Theme provider wiring** — `web/app/layout.tsx` mounts `<ThemeProvider>` from `next-themes`; the wrapper at `web/components/skycot/ThemeProvider.tsx` uses `"use client"` + reads/writes `theme` storage key. Wiring is correct. Interactive toggle smoke test not done this run (preview locked).
- **Mobile responsiveness markers** — `MarketingNav.tsx:108-110` has the burger button with `data-testid="marketing-nav-open"` + `aria-label="Open menu"` + Radix Dialog overlay. Marketing hamburger is wired. Interactive 375px smoke not done this run.
- **CSP + security headers** — `curl -sI http://localhost:3000/` confirms HSTS preload, X-Frame-Options DENY, X-Content-Type-Options nosniff, restrictive Referrer-Policy. Clean.

---

## 6-MISSION-claim coverage rollup (the load-bearing summary)

Reminder per the function-vs-dressing rule (`docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`): "LIVE in the SPA" means a path the demo camera traverses produces real (non-fixture) data AND renders the locked design. "LIKELY-LIVE" means the page renders, backend cold tests pass, but at least one wire defect prevents end-to-end verification from the browser.

### Claim 1 — Teammate, not tool 🟡 LIKELY-LIVE (flat vs T0623 🟡)

**Evidence**:
- `/api/v1/team` returns 1 real worker "Maya" (`role.vertical = marketing-intern`, tagline "Marketing intern that drafts and asks before publishing"). Timestamp `019e5dbb-c03d-79a0-a9a4-ee60bb3c8218`.
- `/api/v1/workers` still returns `{"error":"not found"}` — same defect as T0519 F4 + T0623 F7 + T0819 F9.
- The `d9455b091` patch closed the fetchworker 400 retry-storm that previously masked this for individual-worker fetches; the collection-endpoint 404 remains.
- Backend cold tests pass (round-19 evaluator + round-54).

**Citation**: `curl -s --max-time 30 http://127.0.0.1:7400/api/v1/team` → `{"total_count":1,"workers":[{"name":"Maya",...}]}`. `curl -s http://127.0.0.1:7400/api/v1/workers` → `{"error":"not found"}`. `git show d9455b091 --stat` → fix lives in `web/lib/api/workers.ts`.

**Demo-recording implication**: Scene 1 narrates "Mariah, Peta, Priya, Jamal, Nori" — the SPA workers grid falls back to that fictional roster while `/team` separately reads Maya. Either the runbook re-scripts ("Maya, your marketing intern") or the demo-seed CLI gets run with the 5-persona roster (CLAUDE.md remaining-for-launch item (e), still TODO).

### Claim 2 — Delegation is the unit ❓ NOT-OBSERVABLE-FROM-SPA (flat vs T0623)

**Evidence**:
- No `/handoffs` or `/delegations` UI surface in the SPA route tree — `find web/app -name 'page.tsx' | grep -i handoff` → 0 hits.
- Audit page IS the only delegation-visualisation surface. But the audit feed is now 10/10 SPA self-traffic (was 5/5 at T0623; degraded by 5 net rows). The row-detail drawer can't render handoff hashes for rows that have none.
- Backend round-54 confirms claim 2 LIVE (delegate-task fallback-chain inheritance wired via #2256).

**Citation**: `curl -s 'http://127.0.0.1:7400/api/v1/audit?limit=10'` → 10 rows, every `verb=http.request`, every `worker.name=unknown`, every `action_type` ∈ {`audit.detail.viewed`, `audit.replay`}.

**Demo-recording implication**: Scene 6 audit drill-in cannot demonstrate delegation chains. Resolving F2 (filter self-traffic) is a prerequisite to making claim 2 observable from the SPA.

### Claim 3 — Platform builds workers 🟡 LIKELY-LIVE (flat vs T0623 🟡)

**Evidence**:
- 5 onboarding steps SSR cleanly (per T0519/T0623 walks; same code paths).
- `/genesis` Studio shell present with `<HeroInput>` + Generate button — function is in place; H1 copy mismatched per F15.
- `/api/v1/verticals` STILL returns `{"verticals":[]}` — `/onboarding/pick-vertical` AND `/verticals` both fall back to `SEED_VERTICALS` fixture.
- Backend round-54 confirms claim 3 LIVE.

**Citation**: `curl -s http://127.0.0.1:7400/api/v1/verticals` → `{"verticals":[]}`. `grep -n SEED_VERTICALS web/app/(app)/verticals/page.tsx` → fallback path at line 27 comment "seed fallback in dev when the gateway is empty".

**Demo-recording implication**: Scene 3 IS recordable (pages SSR, F1 wizard flow renders) but the pick-vertical step reads fixtures, not real `/api/v1/verticals`. Same root-cause as Claim 4 — one brief unblocks both.

### Claim 4 — Skills shared + learned 🔴 BROKEN-IN-SPA (flat vs T0623 🔴)

**Evidence**:
- `/skills` catalog API STILL returns `{"skills":[],"total_count":0}` 162 minutes after #2407 merged + 1 cosmetic chip-rewrite (#2431) + 2 conductor ticks + 2 prior synthesis batches queuing the root-cause P0 brief.
- Root-cause brief `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution` queued by T0519 synthesis, still TODO at T0819 (162 minutes).
- Backend cold test `claim_4_live_install_invoke` 1m53s PASS (2026-05-25).

**Citation**: `curl -s 'http://127.0.0.1:7400/api/v1/skills?q=competitor'` → empty for the chip-rewrite stems (`research`, `email`, `draft`, `competitor`).

**Demo-recording implication**: Scene 4 ("search competitor pricing scrape → 3 results") **STILL UNRECORDABLE**. Same single highest-leverage unlock for the demo MP4 as in the prior two audit cycles.

### Claim 5 — Any-PC execution 🟡 LIKELY-LIVE-via-fixtures (improvement on T0623 ❓→🟡)

**Evidence**:
- Backend: claim 5 cold rerun PASS via `a2642ac02` (operator's macOS perms granted; `claim_5_live_mcp_and_cu` cold cargo PASS). Claim 5 is now 🟢 LIVE at the backend layer per CLAUDE.md header.
- SPA-side: `/workflows/templates` SSR renders 50+ template cards from `web/lib/workflow-templates.ts` (hardcoded). The "Use template" CTA at `[templateId]/page.tsx:93` → `/workflows/{vertical}/{name}/build?from_template={id}` IS wired to the builder via `resolve-workflow.ts:10`. So click-through is real function; the template list is dressing.
- `/api/v1/workflows?vertical=sales` returns 0 workflows. Runtime engine has no live workflows seeded.

**Citation**: `curl -s 'http://127.0.0.1:7400/api/v1/workflows?vertical=sales'` → `{"total_count":0,"workflows":[]}`. `git log -1 a2642ac02 --pretty='%s'` → "ops: claim-5 LIVE — cold rerun mcp+cu post macos perms".

**Demo-recording implication**: Scene 5 ("the camera clicks Run on `customer_research_followup`") needs runbook reconciliation — that exact template name isn't in the static `workflow-templates.ts` list (which has `competitor_intel_digest` and similar). Either re-script Scene 5 to a real template name OR seed `customer_research_followup` into the static list AND into the gateway runtime workflows.

### Claim 6 — Multi-channel + multi-tenant 🟡 LIKELY-LIVE-degraded (flat vs T0623 🟡, audit-feed pollution worsened)

**Evidence**:
- `/settings/connectors` + `/settings/notify-channels` source files exist with channel/connector UI shells. Source-read only — interactive verification deferred.
- Audit feed serves real rows but 10 of 10 are SPA self-traffic (was 5/5 at T0623). Cross-tenant negative test backend cold PASS 52.42s (2026-05-25), but the SPA-side display surface is theatre-empty for any agent-level audit drill-in.
- `/settings/security` exists with metadata + content; no current evidence of RBAC enforcement-not-just-copy.

**Citation**: `curl -s http://127.0.0.1:7400/api/v1/audit?limit=10` → 10 self-traffic rows, all `verb=http.request`, all `worker=unknown`. Backend test names per round-54: `claim_6_live_cross_tenant`.

**Demo-recording implication**: Scene 6 the audit drill-in is theatre-empty (clicking any row shows "audit.detail.viewed by unknown"). Resolving F2 (filter self-traffic) is the prerequisite to making claim 6 visible in the demo MP4. Function-side trust signal **degraded vs T0623** (10/10 worse than 5/5).

---

## Resolved since prior audit (T0623 → T0819, 86m)

- **T0623 F2 (P0) `/docs` fans out to 6 dead 404 subroutes** — RESOLVED. All 6 subroute pages exist (`getting-started`, `workflows`, `skills`, `audit`, `api`, `integrations`) plus 4 featured nested pages. Public marketing trust signal restored.
- **T0623 F08 (P0) `/help` placeholder dead-link page** — RESOLVED via server-side 307 → `/docs`. Comment block explicitly closes the prior round's F08.
- **Worker fetch 400 retry-storm** — RESOLVED via `d9455b091`. `fetchWorker` now translates HTTP 400 to `WorkerNotFoundError` same lane as HTTP 404. Closes the 15s spinner the camera caught after onboarding navigations.
- **Claim 5 cold rerun PASS** — RESOLVED via `a2642ac02` post-macOS-perms. CLAUDE.md `Remaining for v1.0 launch` item (c) marked DONE.

Persisting unfixed from T0623:
- F1 (skills+verticals+workflows seed empty) — same root-cause brief, P0, **TODO 162m and counting**
- F2 (audit feed 5/5 → now 10/10 self-traffic) — **WORSENED**, no consolidated brief
- F3 ((app) layout default-title leak) — no brief
- F9/F11 (workers route 404, settings sub-page metadata) — partial briefs
- F4 (`/for/*` double-suffix titles) — **NEWLY OBSERVED**, no brief
- F5 (`/sla` no metadata) — no brief
- F10 (`/team` 2 H1s) — no brief
- F12-F19 (mostly polish; no briefs except F17 partial)

---

## Orientation score

**63 / 100.** (vs T0623 = 61, T0519 = 63, T0415 = 62)

Reasoning: +2 from T0623 because (a) 2 prior P0s genuinely RESOLVED (`/docs` subroutes + `/help` placeholder), which materially de-risks the public-trust polish (b) `d9455b091` closed the 15s retry-storm footgun in Scenes 1-2 (c) Claim 5 backend flipped 🟡→🟢 via cold rerun. -2 from those gains because the audit-feed self-traffic ratio worsened (5/5 → 10/10) — the Scene 6 drill-in is now demonstrably theatre-empty in a way that's worsening over time as the SPA does its own auth/replay pings without backpressure on the audit feed filter.

Net: small improvement on margins, no progress on the load-bearing P0 (skills+verticals+workflows seed empty), and a new P1 surface (the `/for/*` double-suffix titles, which had been overlooked in prior rounds but is plainly visible in every vertical landing page tab).

**One-bullet recommendation for the operator**: claim the P0 `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution` brief NOW. It's been TODO 162 minutes across three audit cycles. Same root-cause unblocks F1 (Scene 4 skills) AND F8/Claim-3 (Scene 3 verticals) AND F17 (Scene 5 workflows runtime). After that, queue ONE consolidated mechanical brief for F3 (default title leak) + F11 (settings metadata) + F4 (`/for/*` double-suffix) — they're all the same shape of fix (metadata exports on layouts) and would knock 3 P1s down to zero in one PR.
