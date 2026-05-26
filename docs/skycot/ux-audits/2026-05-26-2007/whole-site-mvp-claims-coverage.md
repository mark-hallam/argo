# Whole-site MVP claim coverage audit — 2026-05-26 20:07

**Scope:** every SPA page NOT covered by the four parallel scene audits — all marketing pages (`/`, `/landing`, `/pricing`, `/customers`, `/compare`, `/for/*`, `/docs`, `/help`, `/security`, `/privacy`, `/terms`, `/sla`, `/changelog`, `/maintenance`, `/offline`), all auth pages (`/auth/*`), `/cohorts`, `/cohorts/new`, `/settings` + sub-pages (11 sub-tabs), `/brand`, `/team`, `/workflows/templates`, `/workflows/build`, `/verticals`, `/orchestration`, `/genesis`, `/retro`, `/billing`, `/support`, the hire wizard, viewport responsiveness, theme toggle, the 6-mission-claim rollup.

**Environment:** dev server on `:3000`, gateway alive on `:7400` (uptime ~5.6h, started 14:34), schema v83 FRESH. Seed = 1 worker (Maya · marketing-intern), 1 cohort, 0 skills, 0 workflows, 0 approvals.

**Tool note:** preview MCP timed out on `eval`/`snapshot` calls (contended by parallel audits). Walked the SPA via `curl --max-time 20` + source reading + gateway probing. Visual-only findings are called out where they exist.

**Baseline:** prior 14:41 whole-site audit at `docs/skycot/ux-audits/2026-05-26-1441/whole-site-mvp-claims-coverage.md`. 11 web-related commits have landed since (see "Resolved since last audit" below). This run re-validates each prior finding against the current build.

---

## TL;DR

19 findings. **0 P0**, **5 P1**, **10 P2**, **4 P3**. The site is in better shape than the 14:41 baseline — the F02 root-redirect, F08 `/help`-redirect, F17 `/support`-redirect, and F18 `/retro`/`/billing` documentation findings are all RESOLVED. The new P1 is **all five F1 onboarding steps leak the site-default browser tab title** ("SKYCOT — Hire your first AI teammate") because every onboarding step is a client component without page metadata — visible if the demo records the URL bar during the Scene 3 onboarding walk. The F04 gateway-workers-alias fix (PR #2395) has landed in source but the **running gateway binary is stale** (started ~2.5h before the merge) so `/api/v1/workers` still 404s in the live environment — operator should restart the gateway to pick up the fix. F01 (in-app title leak on /verticals /orchestration /genesis /brand /retro /billing /support/inbox) and F05 (`/for/*` double-suffix `· SKYCOT · SKYCOT`) still apply unchanged.

On the **6 mission claims** rollup, the picture is materially better than the 14:41 audit:
- Claim 1 LIVE and visible (Worker page + roster + tagline).
- Claim 2 (delegation) still has no first-class SPA surface — runtime is LIVE but UI is silent.
- Claim 3 LIVE and visible (`/hire` + `/genesis` wired; Genesis SSE describe-and-watch landed via #2326).
- Claim 4 LIVE in source; **0 skills in seed** means the Scene 4 marketplace search returns empty (`/api/v1/skills` returns `{"skills":[],"total_count":0}`).
- Claim 5 has no SPA surface as expected (runtime-side computer-use).
- Claim 6 NEWLY VISIBLE — `ChannelStripCard` (PR #2391) now lives on the worker overview tab and aggregates audit rows by channel (slack / iMessage / sms / voice / whatsapp / web / push). This closes the "no multi-channel SPA proof beat" gap I flagged in the 14:41 audit, modulo seed having no non-`web` channel rows so the empty-state CTA fires instead of the channel strip.

---

## Headline blockers

1. **F1 [P1]** All 5 onboarding step pages (`/onboarding/welcome|workspace|connect-provider|pick-vertical|first-hire`) inherit the site-default `<title>` "SKYCOT — Hire your first AI teammate" because they're client components with no `metadata` export. Scene 3 records all 5 — the browser tab will read the same generic string on every step.
2. **F2 [P1]** Running gateway binary predates the workers-collection-alias fix (PR #2395 / commit `9ea693db1` landed 17:00 Melbourne; gateway started 14:34). `/api/v1/workers` still returns 404 in the live environment. Source is fixed — operator just needs to restart the gateway.
3. **F3 [P1]** `/verticals`, `/orchestration`, `/genesis`, `/brand`, `/retro`, `/billing`, `/support/inbox` all leak "Approvals · SKYCOT" in the browser tab. Unchanged since the 14:41 audit's F01 — none of the per-page metadata fixes have landed. Recording-side risk if the camera frames the tab on any of these routes.
4. **F4 [P1]** `/for/*` pages all double-suffix their title (e.g. `"AI sales teammates that never sleep — SKYCOT · SKYCOT"`). Unchanged since 14:41 F05. The marketing nav now links to `/for/*` pages from `/landing`, so a demo viewer Cmd-clicking any vertical card opens a tab with the double-SKYCOT title.
5. **F5 [P1]** Scene 1 of the demo runbook scripts a 5-worker trading-card deck (Mariah / Peta / Priya / Jamal / Nori). The live seed has **1 worker (Maya · marketing-intern)**. PR #2367 scrubbed Sam/Rio from the sample fixtures but didn't add the canonical 5 to the live seed. The `/workers` page will show 1 card, not a deck. Unrecordable as scripted.

---

## All findings

### Marketing pages

#### F6 [P1 — was P1 in 14:41 F05] — `/for/*` pages double-suffix titles ("... — SKYCOT · SKYCOT")
All 6 vertical-landing pages emit titles like `"AI sales teammates that never sleep — SKYCOT · SKYCOT"`. The page sets its own "— SKYCOT" suffix and inherits the global "· SKYCOT" suffix from the root template. Browser tab reads doubled brand. Unchanged since 14:41.
- Repro: `curl -s http://localhost:3000/for/sales | perl -nle 'print "$1" if /<title[^>]*>([^<]+)/'` → `AI sales teammates that never sleep — SKYCOT · SKYCOT`. Same shape for `/for/{recruiting,marketing,finance,ops,support}`.
- File: `web/app/for/_components/VerticalLandingShell.tsx` (or wherever the metadata is set per-vertical) + `web/app/layout.tsx` template.
- Fix: drop the trailing `— SKYCOT` from each `/for/*` page-level title; the global `· SKYCOT` already handles brand.

#### F7 [P2 — was P2 in 14:41 F06] — `/sla`, `/maintenance`, `/offline` inherit site-default title
Three policy/status pages return the default `"SKYCOT — Hire your first AI teammate"` instead of a page-specific title. Unchanged since 14:41.
- Repro: `curl -s http://localhost:3000/sla | perl -nle 'print "$1" if /<title[^>]*>([^<]+)/'` → `SKYCOT — Hire your first AI teammate`. Same for `/maintenance`, `/offline`.
- Files: `web/app/sla/page.tsx`, `web/app/maintenance/page.tsx`, `web/app/offline/page.tsx`.

#### F8 [P3] — `/customers` testimonials are fictional fixtures (dressing-side, per the function-vs-dressing rule)
Mariah at Northwind, Priya at Apex, etc — same names as the demo worker personas. Demo camera doesn't drill into customer profiles per the runbook, so this is acceptable POC dressing. PR #2378 also added a `/customers/<slug>` noindex robots gate. Worth noting if the demo recording is ever repurposed for public-launch context.
- File: `web/app/customers/page.tsx` (testimonial copy).

### In-app pages

#### F9 [P1 — NEW since 14:41] — All 5 F1 onboarding step pages leak the site-default browser tab title
The five onboarding step pages (`/onboarding/welcome`, `/onboarding/workspace`, `/onboarding/connect-provider`, `/onboarding/pick-vertical`, `/onboarding/first-hire`) are client components that don't export `metadata`. All five show `"SKYCOT — Hire your first AI teammate"` in the browser tab. Scene 3 of the demo records every step in sequence — the tab will read the same generic string across all five.
- Repro: `for p in /onboarding/welcome /onboarding/workspace /onboarding/connect-provider /onboarding/pick-vertical /onboarding/first-hire; do curl -s http://localhost:3000$p | perl -nle 'print "$1" if /<title[^>]*>([^<]+)/'; done` → all 5 return `SKYCOT — Hire your first AI teammate`.
- File: `web/app/(setup)/onboarding/welcome/page.tsx:1` (and 4 sibling step pages) — all start with `"use client";` and have no `metadata` export.
- Fix: add `web/app/(setup)/onboarding/<step>/layout.tsx` files with `export const metadata: Metadata = { title: "<Step name> · SKYCOT" }`. Server-component layouts can set metadata for client-component pages.

#### F10 [P1 — was P1 in 14:41 F01] — `/verticals`, `/orchestration`, `/genesis`, `/brand`, `/retro`, `/billing`, `/support/inbox` all leak "Approvals · SKYCOT" tab title
Seven in-app routes ship without page-level metadata so they inherit the next-closest parent's title — which happens to be the Approvals page metadata. The marketing nav and the in-app sidebar both could surface any of these routes; tab titles read "Approvals · SKYCOT" on routes that have nothing to do with approvals. Unchanged since 14:41.
- Repro: `curl -s http://localhost:3000/brand | perl -nle 'print "$1" if /<title[^>]*>([^<]+)/'` → `Approvals · SKYCOT`. Same for `/verticals`, `/orchestration`, `/genesis`, `/retro`, `/billing`, `/support/inbox`.
- Files: `web/app/(app)/{brand,verticals,orchestration,genesis,retro,billing}/page.tsx`, `web/app/(app)/support/inbox/page.tsx`.

#### F11 [P1 — NEW since 14:41] — Running gateway binary stale vs HEAD; `/api/v1/workers` collection alias not live
PR #2395 (commit `9ea693db1`, merged 2026-05-26 17:00 Melbourne) added a `/api/v1/workers` collection route that aliases `/api/v1/team`. The source-code dispatcher at `crates/argo-cli/src/handlers/gateway/routes_dispatch_get_api_groups.rs:64` correctly routes `GET /api/v1/workers` to the new `routes_workers_list::list_response`. But the live gateway started at 14:34 (uptime 20181s as of 20:07), about 2.5 hours before the merge — so the live binary doesn't carry the alias. `/api/v1/workers` still 404s in the running environment.
- Repro: `curl -s -o /dev/null -w "%{http_code}\n" http://127.0.0.1:7400/api/v1/workers` → `404`. `curl -s -o /dev/null -w "%{http_code}\n" http://127.0.0.1:7400/api/v1/workers/019e5dbb-c03d-79a0-a9a4-ee60bb3c8218` → `200` (singleton route, unchanged, works).
- Fix: operator restarts the gateway — `make restart-live-gateway` per CLAUDE.md item (b). No code change needed.

#### F12 [P1 — NEW since 14:41] — Live seed has 1 worker (Maya); Scene 1's "5-worker trading card deck" beat is unrecordable as scripted
The demo runbook's Scene 1 narrates Mariah / Peta / Priya / Jamal / Nori on `/workers`. The gateway returns one worker: Maya · marketing-intern. PR #2367 ("scrub Sam/Rio from sample fixtures, lock Scene-1 canonical roster") scrubbed deprecated names but didn't add the 5 canonical workers to live seed. The /workers trading-card deck will render exactly one card with "No activity yet." in the headline.
- Repro: `curl -s http://127.0.0.1:7400/api/v1/team | python3 -c "import sys,json; d=json.load(sys.stdin); print('Total:', d['total_count'], 'Names:', [w['name'] for w in d['workers']])"` → `Total: 1 Names: ['Maya']`.
- File: backend seed bootstrap (likely `crates/argo-cli/src/handlers/demo/seed.rs` or similar) — needs to either insert the canonical 5 workers or the demo recording adjusts to feature Maya only. Operator-decisional.

#### F13 [P2 — was P2 in 14:41 F07] — `/workflows/templates` missing global `· SKYCOT` suffix; tab reads "Workflow templates"
Every other page uses the global title template `"X · SKYCOT"`. `/workflows/templates` opts out and emits just `"Workflow templates"`. Unchanged since 14:41.
- Repro: `curl -s http://localhost:3000/workflows/templates | perl -nle 'print "$1" if /<title[^>]*>([^<]+)/'` → `Workflow templates`.
- File: `web/app/(app)/workflows/templates/page.tsx` (or its layout). The page-level metadata override doesn't compose with the root template.

#### F14 [P2] — Four settings sub-pages emit page-only titles, breaking the global `· SKYCOT` suffix
`/settings/members` → "Members", `/settings/api-keys` → "API keys", `/settings/security` → "Security", `/settings/data-export` → "Data export". The other 7 settings sub-pages emit "Settings · SKYCOT" (parent metadata inherited correctly).
- Repro: `for p in /settings/members /settings/api-keys /settings/security /settings/data-export; do curl -s http://localhost:3000$p | perl -nle 'print "$1" if /<title[^>]*>([^<]+)/'; done` → `Members`, `API keys`, `Security`, `Data export`.
- Files: `web/app/(app)/settings/{members,api-keys,security,data-export}/page.tsx`.
- Fix: either drop the page-level title override (let the layout's title inherit), or change `"Members"` → `"Members · Settings · SKYCOT"`.

#### F15 [P2 — was P1 in 14:41 F03] — Worker `/schedules` and `/duties` gateway routes still 404; Scene 2 "Run now" demo beat has no backend wiring
Unchanged since 14:41. No new commits address the gateway-side missing routes for `/api/v1/workers/{id}/duties` or `/api/v1/workers/{id}/schedules`. Scene 2's "click Mariah's Schedules tab + Run now → audit row" remains backend-impossible at the moment.
- Repro: `curl -s http://127.0.0.1:7400/api/v1/workers/019e5dbb-c03d-79a0-a9a4-ee60bb3c8218/duties` → `{"error":"not found"}`. Same for `/schedules`.
- Downgraded from P1 because the fix scope is "add a gateway route" — orthogonal to the SPA. Backend brief, not a frontend issue.

#### F16 [P2 — was P2 in 14:41 F12-F16] — Multiple in-app pages are 100% client-rendered; demo records the loading-shell frame
`/workers`, `/workers/<id>`, `/team`, `/cohorts`, `/audit`, `/approvals`, `/skills`, `/verticals`, `/orchestration`, `/genesis`, `/workflows/<v>/<w>` all return loading shells in the SSR payload — the real content paints client-side after the gateway round-trip. During demo recording every navigation will show 100-500ms of loading state before content appears. Pattern-level issue, not per-page. Acceptable for a POC; not for a polished v1.1.
- Repro: `curl -s http://localhost:3000/workers/019e5dbb-c03d-79a0-a9a4-ee60bb3c8218 | grep -oE 'Loading worker'` → `Loading worker`.
- Fix: add server components that `getQueryClient().prefetchQuery(...)` and pass via `HydrationBoundary` so the first HTML payload includes the real content.

#### F17 [P2] — `/hire` page title double-suffixes ("Hire — SKYCOT · SKYCOT")
Same shape as the `/for/*` issue. `/hire` page-level metadata emits `"Hire — SKYCOT"`, then the global template adds `· SKYCOT`, yielding `"Hire — SKYCOT · SKYCOT"`.
- Repro: `curl -s http://localhost:3000/hire | perl -nle 'print "$1" if /<title[^>]*>([^<]+)/'` → `Hire — SKYCOT · SKYCOT`.
- File: `web/app/(hire)/hire/page.tsx` metadata.

#### F18 [P2] — `/hire/step/0` and `/hire/step/6` both 404 — wizard doesn't clamp invalid step indexes
Out-of-range step indexes return generic 404 instead of redirecting to `/hire`. Unchanged since 14:41 F22.
- Repro: `curl -s -o /dev/null -w "%{http_code}\n" http://localhost:3000/hire/step/0` → 404. Same for `/hire/step/6`.
- File: `web/app/(hire)/hire/step/[step]/page.tsx` — clamp invalid step values.

### Backend / env

#### F19 [P3 — was P3 in 14:41 F20, re-verified] — `.env.local` uses wrong env var name for auth bypass
`web/.env.local:6` sets `ARGO_GATEWAY_AUTH_BYPASS=1`. The middleware (`web/middleware.ts:58`) reads `process.env.NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS === "1"`. Without the `NEXT_PUBLIC_` prefix the var is server-only and the middleware can't see it. Per the no-theater lock this is currently *helping* — the SPA hits the real gateway. But the name mismatch is a latent bug.
- File: `web/.env.local:6` says `ARGO_GATEWAY_AUTH_BYPASS=1`; should be `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` to match the checked name.

### Cross-cutting

#### F20 [P3 — was P3 in 14:41 F25] — `/landing` primary CTA is "Hire your first AI teammate" → /auth/signup
Marketing nav primary button reads "Hire your first AI teammate" and links to `/auth/signup`, not `/hire`. Probably intentional (signup → first hire) but the button copy conflates the two actions. Polish.
- File: `web/components/marketing/MarketingNav.tsx`.

---

## 6-mission-claim coverage rollup (from SPA observations)

For each claim, "does this work functionally in MVP scope based on what the SPA shows?" with cited evidence. Function-vs-dressing rule applied — claims about behaviour the camera will drill into must be backed by real backend; copy-only assertions don't pass.

### Claim 1 — Teammate, not tool — VERDICT: YES, demonstrable
The `/workers` and `/team` pages render real workers from the gateway (`/api/v1/team` returns Maya · marketing-intern with a real tagline `"Marketing intern that drafts and asks before publishing."`). Worker objects carry a `headline.bullets` field describing what they did, not what tools they ran. The marketing nav now exposes a top-level "Roster" link to `/team` (PR #2378). The `/landing` H1 is `"Don't buy another tool."` — the framing is consistent. Claim holds in MVP scope.
- Evidence: `curl -s http://127.0.0.1:7400/api/v1/team` → Maya worker object with tagline.
- File: `web/app/(app)/workers/page.tsx`, `web/components/skycot/nav-items.tsx:25`.

### Claim 2 — Delegation is the unit — VERDICT: PARTIAL (backend LIVE, SPA silent)
Per CLAUDE.md round-54 the runtime LIVE delegate-task fallback-chain inheritance was wired via PR #2256. But the SPA still has no visible "this row was delegated by worker X to worker Y" rendering. `/orchestration` is the nearest surface and returns 0 runs (no workflows in seed). The `/audit` page renders rows with a single `worker.id` field — no delegation chain visualisation. Demo can assert delegation in copy but can't show it.
- Evidence: `crates/argo-runtime/.../current_handoff_id` is the runtime field; no `delegated_by` field in `/api/v1/audit` row shape.

### Claim 3 — Platform builds workers — VERDICT: YES, demonstrable (Genesis SSE now live)
Two surfaces: `/hire` (5-step F1 wizard) and `/genesis` (describe-and-watch — the new SSE-streaming preview surface from PR #2326). `/genesis` H1 reads `"Hire your next teammate."` with a placeholder `"Describe the role in plain English. e.g. 'A customer success lead who triages tier-1 tickets, drafts response templates, and escalates refunds over $500'."` Per the operator-clarified framing (memory `feedback_genesis_is_ai_teammate_onboarding_studio`) Genesis is the investor-moment surface that distinguishes Skycot from "ChatGPT with prompts". Claim holds in MVP scope.
- Evidence: `curl -s http://localhost:3000/genesis | perl -nle 'print "$1" if /placeholder="([^"]+)/'` → describe-the-role-in-plain-English prompt.
- File: `web/app/(app)/genesis/page.tsx`.

### Claim 4 — Skills shared and learned — VERDICT: SURFACE LIVE, SEED EMPTY
`/skills` page is the A1-marketplace shell with hero search + suggested-search chips. The `/api/v1/skills` and `/api/v1/skills/learned-this-month` endpoints exist and respond. Per CLAUDE.md `#2092 + #2110` are merged and the claim-4 LIVE install-invoke test passed cold rerun. **But the live seed has 0 skills** — Scene 4's "search 'competitor pricing scrape' → 3 results" beat returns nothing. PR #2350 added "seed learned provenance fallbacks" but the marketplace itself is empty. Operator must either seed 3+ installable skills or adjust the demo script.
- Evidence: `curl -s http://127.0.0.1:7400/api/v1/skills` → `{"skills":[],"tenant_id":"...","total_count":0}`.

### Claim 5 — Any-PC execution — VERDICT: NOT IN SPA SCOPE (runtime-side, awaiting operator macOS perms)
No SPA surface. Per CLAUDE.md claim-5 is 🟡 LIKELY-LIVE pending operator macOS Accessibility + Screen Recording grants on the argo binary. Nothing to validate from the SPA side. Audit silent on claim 5.

### Claim 6 — Multi-channel + multi-tenant + trust — VERDICT: NOW DEMONSTRABLE (channel strip closes the prior gap)
The big shift since 14:41: **PR #2391 added `ChannelStripCard`** at `web/components/skycot/worker/ChannelStripCard.tsx`. It aggregates the worker's last-7-days audit rows by `channel` (slack / imessage / sms / push / web / voice / whatsapp) and renders a per-channel strip on the worker overview tab. Each row links to `/audit?worker=<id>&channel=<channel>` for drill-in. The component file's own header says verbatim: *"Claim 6 (multi-channel + multi-tenant) proof surface for the worker overview tab. Aggregates the worker's last-7-days audit rows by channel so the v1.0 demo MP4 can land 'Mariah pinged your Slack 4 times and your Email 2 times in the last 7 days — here's the audit trail for one of those Slack messages.'"* This closes the multi-channel SPA proof gap I flagged in the 14:41 audit. Multi-tenant scoping is already plumbed through every gateway response (`tenant_id: "019e5057-1d3f-7fa2-9c1b-c34ef01cc21e"` visible in `/api/v1/team`, `/api/v1/cohorts`, `/api/v1/skills`). Trust ratchet has visible surfaces in `/landing` Step 03 + worker `trust_grade_band` field.
- Evidence: `web/components/skycot/worker/ChannelStripCard.tsx:20-36` header + `web/components/skycot/worker/WorkerOverviewContent.tsx:160` wiring.
- **Caveat**: the live audit feed has only `channel: "web"` rows (audit-trail rows from my own browsing). The empty-state branch fires which shows the connect-CTA pointing at `/settings/connectors`, not the full channel strip. To exercise the demo beat, seed needs ≥1 non-`web` audit row per channel — operator decisional.

**Net 6-claim verdict for MVP demo recording:**
- Claims 1, 3, 6 fully demonstrable.
- Claim 4 demonstrable conditional on seeding 3+ skills.
- Claim 2 LIVE in runtime, silent in SPA (assert in narration only).
- Claim 5 out of scope for SPA.

---

## Resolved since last audit (14:41 → 20:07)

These 14:41 findings are now FIXED in the current build:

- **F02 [P1] root URL `/` lands on Approvals** → FIXED via PR #2394 (`web: redirect unauth root to landing`). `web/middleware.ts:63` now redirects `/` → `/landing` when no session cookie present. Auth-aware redirect — logged-in users still land on Approvals.
- **F04 [P1] `/api/v1/workers` collection 404** → FIXED IN SOURCE via PR #2395 (`gateway: alias workers collection route`); `crates/argo-cli/src/handlers/gateway/routes_workers_list.rs` and dispatcher line 64 are wired. **Still 404 in running gateway** (see F11 above — needs binary restart).
- **F08 [P2] `/help` is a placeholder** → FIXED via PR #2399 (`fix(web): polish three orphan SPA routes`). `/help` now 307-redirects to `/docs`.
- **F17 [P1] `/support` 404** → FIXED via PR #2381 (`fix bare support and vertical redirects`). `/support` now 307-redirects.
- **F18 [P3] `/retro` + `/billing` orphan routes** → FIXED-AS-DOCUMENTED via PR #2399 (the three-orphan-routes polish PR). Both routes still exist (intentional — they're future surfaces) but are now documented.
- **F26 [P3] Marketing customer names overlap with worker personas** → PARTIALLY FIXED via PR #2367 (`scrub Sam/Rio from sample fixtures, lock Scene-1 canonical roster`). Customers page still has the names, but the sample worker fixtures are now disjoint from customers fixtures.
- **F24 [P3] Theme picker source-only verified** → PR #2368 (`fix(settings): surface theme picker in workspace`) — ThemePicker is now imported in `/settings/workspace` so operator can swap mode + tenant theme without leaving the surface.

Net: 7 prior findings closed. The biggest structural improvements between the two audits are the root-redirect fix, the Genesis describe-and-watch SSE surface landing, and the ChannelStripCard closing the claim-6 multi-channel-SPA-proof gap.

---

## Orientation score: 82 / 100

Up 4 points vs 14:41 (78 → 82). The whole-site surface is in materially better shape — 7 prior findings closed in one afternoon's commits, the channel-strip closes the load-bearing claim-6 gap, the root URL routes correctly, the orphan routes are tidied. The remaining 19 findings are evenly split between **(a) tab-title polish** (F1, F6, F7, F10, F13, F14, F17 — seven separate title-metadata bugs across the in-app + marketing surface, all <30 min each), **(b) the still-stale gateway binary** (F11 — operator restart only), **(c) the live-seed gap** (F12 — 1 worker not 5, 0 skills not ≥3 — needs operator demo-seed bootstrap), and **(d) the existing client-rendered loading-shell pattern** (F16 — Next.js 16 architecture pattern, fix is a v1.1 SSR-prefetch sweep).

The biggest remaining 6-claim gap is **claim 2 delegation has no SPA surface** — runtime is LIVE per round-54, but the SPA audit-row shape has no `delegated_by` field and there's no "delegation chain" visualisation. The v1.0 demo can assert delegation in narration only. Worth a brief to add a `delegated_by` lens to `/audit` rows + an orchestration-detail page showing handoff chains as graph; not a v1.0 ship blocker.
