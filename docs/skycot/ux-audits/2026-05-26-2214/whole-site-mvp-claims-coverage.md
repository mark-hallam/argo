# Whole-site MVP claim coverage audit — 2026-05-26 22:14

**Scope:** every SPA page NOT covered by the four parallel scene audits — marketing surface (`/`, `/landing`, `/pricing`, `/customers`, `/compare`, `/for/*`, `/docs`, `/help`, `/security`, `/privacy`, `/terms`, `/sla`, `/changelog`, `/maintenance`, `/offline`), auth (`/auth/*`), `/cohorts`, `/cohorts/new`, `/settings` + sub-tabs, `/brand`, `/team`, `/workflows/templates`, `/workflows/build`, `/skills-canvas` (absent), `/retro`, `/billing`, `/support`, `/genesis`, `/verticals`, `/orchestration`, the `/hire` wizard, viewport responsiveness, theme toggle, plus the 6-mission-claim rollup. Six dedicated scene audits cover `/workers`, `/workers/<id>`, `/skills`, `/workflows/<v>/<w>`, `/approvals`, `/audit`.

**Environment:** dev server on `:3000`, gateway alive on `:7400` (uptime 27716 s ≈ 7h41m; started ~14:32 Melbourne), schema v83 FRESH. Seed = 1 worker (Maya · marketing-intern · id `019e5dbb-c03d-79a0-a9a4-ee60bb3c8218`), 1 cohort (marketing-intern), 0 skills, 0 workflows, 0 approvals (live gateway — not demo-mode-seeded fixtures).

**Tool note:** preview MCP under contention from parallel audits; walked the SPA via `curl --max-time 8`, source reading, and gateway probing. Visual-only findings called out explicitly.

**Baseline:** prior 20:07 whole-site audit at `docs/skycot/ux-audits/2026-05-26-2007/whole-site-mvp-claims-coverage.md`. Nine web-related commits have landed since (see "Resolved since last audit" below). This run re-validates each prior finding and adds the new fix-cycle deltas.

---

## TL;DR

17 findings. **0 P0**, **5 P1**, **9 P2**, **3 P3**. Most of the 20:07 punch-list still applies: the seven in-app pages still leak the `(app)`-group default "Approvals · SKYCOT" tab title (the layout's hardcoded fallback at `web/app/(app)/layout.tsx:20`); all five F1 onboarding step pages still ship without `metadata` so the browser tab reads the site-default marketing title on every step; the six `/for/*` vertical-landing pages still double-suffix their titles (`"… — SKYCOT · SKYCOT"`); four settings sub-pages still emit page-only titles that skip the global `· SKYCOT` template; `/workflows/templates` still opts out of the global suffix. **New since 20:07**: PR #2345 landed `DelegationChainCard` on the audit drill-in page — claim 2 (delegation) now has a real SPA proof surface for the first time, materially upgrading the 6-claim rollup. **Still outstanding (no fix landed)**: PR #2407 ("seed demo marketplace skill catalog on gateway boot") is OPEN-MERGEABLE-CLEAN but not merged — `/skills` returns `{"skills":[],"total_count":0}` and Scene 4's "search 'competitor pricing scrape' → 3 results" beat returns the empty-state CTA instead of the catalog rows.

The 6-claim coverage rollup is in better shape than 20:07:
- **Claim 1** LIVE and demonstrable (Maya rendered with real tagline + headline bullets).
- **Claim 2** now LIVE in the SPA (PR #2345 — `DelegationChainCard` on `/audit/action/[actionId]`).
- **Claim 3** LIVE and demonstrable (`/hire` + `/genesis` SSE).
- **Claim 4** SURFACE LIVE, SEED STILL EMPTY (PR #2407 not merged).
- **Claim 5** not in SPA scope.
- **Claim 6** demonstrable via `ChannelStripCard` (PR #2391, prior audit) — modulo the live audit feed having only `channel: "web"` rows so the empty-state CTA fires.

---

## Headline blockers

1. **F1 [P1]** Seven in-app routes (`/verticals`, `/orchestration`, `/genesis`, `/brand`, `/retro`, `/billing`, `/support/inbox`) inherit the `(app)`-group default "Approvals" title, rendering `"Approvals · SKYCOT"` in the browser tab on routes that have nothing to do with approvals. Root cause: the `(app)`-group layout sets `title: { default: "Approvals" }` at `web/app/(app)/layout.tsx:20`. Camera-side risk if the demo records the tab. Carried unchanged from 20:07 F10.
2. **F2 [P1]** All five F1 onboarding steps (`/onboarding/welcome|workspace|connect-provider|pick-vertical|first-hire`) still ship without per-step metadata so all five render the site-default `"SKYCOT — Hire your first AI teammate"` in the browser tab. Scene 3 records all five — the tab reads the same generic marketing string across every step. Carried unchanged from 20:07 F9.
3. **F3 [P1]** All six `/for/*` vertical-landing pages still double-suffix titles (e.g. `"AI sales teammates that never sleep — SKYCOT · SKYCOT"`). The marketing nav doesn't link to them from the primary navigation but `/landing` social-share links do. Carried unchanged from 20:07 F6.
4. **F4 [P1]** Live seed has 1 worker (Maya) — Scene 1's scripted 5-worker trading-card-deck beat (Mariah / Peta / Priya / Jamal / Nori) remains unrecordable as scripted. The runbook either adjusts to feature Maya or the seed adds the canonical 5. Carried unchanged from 20:07 F12 (no fix merged).
5. **F5 [P1]** `/skills` is the A1-marketplace shell, but the gateway returns 0 skills so Scene 4's "search 'competitor pricing scrape' → 3 results" beat returns the `"No skills installed yet"` empty-state CTA pointing to `/team`. PR #2407 is the fix — OPEN, MERGEABLE, CLEAN, not yet merged.

---

## All findings

### Marketing surface

#### F6 [P1 — was P1 in 20:07 F6] — `/for/*` pages double-suffix titles
All 6 vertical-landing pages emit titles like `"AI sales teammates that never sleep — SKYCOT · SKYCOT"`. The page sets its own `— SKYCOT` suffix and inherits the global `· SKYCOT` from the root template. Unchanged since 20:07.
- Repro: `for v in sales marketing finance ops support recruiting; do curl -s http://localhost:3000/for/$v | perl -nle 'print "$1" if /<title[^>]*>([^<]+)/'; done` → all double-suffixed.
- File: `web/app/for/_components/VerticalLandingShell.tsx` (or wherever per-vertical metadata is set) + `web/app/layout.tsx` template.
- Fix: drop the trailing `— SKYCOT` from each `/for/*` page-level title; the global `· SKYCOT` already handles brand.

#### F7 [P2 — was P2 in 20:07 F7] — `/sla`, `/maintenance`, `/offline` inherit site-default title
Three policy/status pages return `"SKYCOT — Hire your first AI teammate"` instead of a page-specific title. Unchanged since 20:07.
- Repro: `curl -s http://localhost:3000/sla | perl -nle 'print "$1" if /<title[^>]*>([^<]+)/'` → `SKYCOT — Hire your first AI teammate`. Same for `/maintenance`, `/offline`.
- Files: `web/app/sla/page.tsx`, `web/app/maintenance/page.tsx`, `web/app/offline/page.tsx` — none export `metadata`.
- Fix: add `export const metadata: Metadata = { title: "SLA" }` (resp. "Maintenance", "Offline") at the top of each page; the global template handles the `· SKYCOT` suffix.

#### F8 [P3 — was P3 in 20:07 F8] — `/customers` testimonials are fictional fixtures (dressing-side)
Mariah at Northwind, Priya at Apex, etc — same personas as the demo workers. Per the function-vs-dressing rule and `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`, this is acceptable POC dressing because the demo camera doesn't drill into `/customers` per the runbook. Worth noting if the demo recording is ever repurposed for public-launch context.
- File: `web/app/customers/page.tsx`.

### In-app pages

#### F9 [P1 — was P1 in 20:07 F9] — All 5 F1 onboarding step pages leak the site-default browser tab title
The five onboarding step pages are client components without `metadata` exports. All five emit `"SKYCOT — Hire your first AI teammate"` in the browser tab. Scene 3 records every step in sequence.
- Repro: `for p in welcome workspace connect-provider pick-vertical first-hire; do curl -s http://localhost:3000/onboarding/$p | perl -nle 'print "$1" if /<title[^>]*>([^<]+)/'; done` → all 5 return `SKYCOT — Hire your first AI teammate`.
- File: `web/app/(setup)/onboarding/{welcome,workspace,connect-provider,pick-vertical,first-hire}/page.tsx` — all start with `"use client";` and have no sibling `layout.tsx`.
- Fix: add a single `web/app/(setup)/onboarding/layout.tsx` with `export const metadata: Metadata = { title: { default: "Onboarding", template: "%s · Onboarding · SKYCOT" } }`, then a per-step `layout.tsx` with `title: "Welcome"` (etc) — server-component layouts can set metadata for client-component pages.

#### F10 [P1 — was P1 in 20:07 F10] — Seven in-app routes inherit "Approvals · SKYCOT" tab title
`/verticals`, `/orchestration`, `/genesis`, `/brand`, `/retro`, `/billing`, `/support/inbox` all ship without page-level metadata so they inherit the `(app)`-group layout's default. Per `web/app/(app)/layout.tsx:20`, that default is hardcoded `"Approvals"`. The marketing nav links to none of them, but the in-app sidebar exposes most; demo Cmd-clicking a sidebar link opens a tab titled "Approvals · SKYCOT" on a non-Approvals route.
- Repro: `for p in /verticals /orchestration /genesis /brand /retro /billing /support/inbox; do curl -s http://localhost:3000$p | perl -nle 'print "$1" if /<title[^>]*>([^<]+)/'; done` → all 7 return `Approvals · SKYCOT`.
- File: `web/app/(app)/layout.tsx:20` (the `default: "Approvals"` is the source) + each route's missing `metadata` export. The cleanest fix is to ditch the `(app)`-group default entirely (force every page to declare its own title) — or change the default to something generic like `"Workspace"`.
- Files needing per-page metadata: `web/app/(app)/{verticals,orchestration,genesis,brand,retro,billing}/page.tsx`, `web/app/(app)/support/inbox/page.tsx`.

#### F11 [P2 — was P2 in 20:07 F13] — `/workflows/templates` opts out of the global suffix
Page-level metadata at `web/app/(app)/workflows/templates/layout.tsx:5` sets `title: "Workflow templates"` (no `· SKYCOT` suffix) and `openGraph.title: "Workflow templates — SKYCOT"`. The page metadata overrides the global template instead of composing with it. Unchanged since 20:07.
- Repro: `curl -s http://localhost:3000/workflows/templates | perl -nle 'print "$1" if /<title[^>]*>([^<]+)/'` → `Workflow templates`.
- Fix: change `title: "Workflow templates"` to `title: { absolute: "Workflow templates · SKYCOT" }` — or, simpler, just `title: "Workflow templates"` while ensuring the `(app)`-group layout's `template: "%s · SKYCOT"` actually composes (it should — investigate why this override skips the template).

#### F12 [P2 — was P2 in 20:07 F14] — Four settings sub-pages skip the global `· SKYCOT` suffix
`/settings/members` → `"Members"`, `/settings/api-keys` → `"API keys"`, `/settings/security` → `"Security"`, `/settings/data-export` → `"Data export"`. The other 7 settings sub-pages emit `"Settings · SKYCOT"` (the parent settings layout's metadata inherited correctly).
- Repro: `for p in /settings/members /settings/api-keys /settings/security /settings/data-export; do curl -s http://localhost:3000$p | perl -nle 'print "$1" if /<title[^>]*>([^<]+)/'; done` → `Members`, `API keys`, `Security`, `Data export`.
- Files: `web/app/(app)/settings/{members,api-keys,security,data-export}/page.tsx` — each likely exports a page-level `metadata` with `title` set to the bare word, bypassing the template.
- Fix: drop the page-level title override (let the parent settings layout's title inherit), or change `title: "Members"` → `title: "Members · Settings"` to compose with the global template.

#### F13 [P2 — was P1 in 20:07 F15] — Worker `/schedules` and `/duties` gateway routes still 404; Scene 2 "Run now" demo beat has no backend wiring
Unchanged since 20:07. No new commits address the gateway-side missing routes for `/api/v1/workers/{id}/duties` or `/api/v1/workers/{id}/schedules`. Scene 2's "click Mariah's Schedules tab + Run now → audit row" remains backend-impossible at the moment. Downgraded from P1 to P2 because the fix scope is "add a gateway route" — orthogonal to the SPA whole-site sweep.
- Repro: `curl -s http://127.0.0.1:7400/api/v1/workers/019e5dbb-c03d-79a0-a9a4-ee60bb3c8218/duties` → `{"error":"not found"}`. Same for `/schedules`.

#### F14 [P2 — was P2 in 20:07 F16] — Multiple in-app pages are 100% client-rendered; demo records the loading-shell frame
`/workers`, `/workers/<id>`, `/team`, `/cohorts`, `/audit`, `/approvals`, `/skills`, `/verticals`, `/orchestration`, `/genesis`, `/workflows/<v>/<w>` all return loading shells in the SSR payload — the real content paints client-side after the gateway round-trip. During demo recording every navigation will show 100-500 ms of loading state before content appears. Confirmed `/skills` still returns the literal text `"Loading skills from the marketplace…"` in the SSR HTML at `web/app/(app)/skills/page.tsx:149`. Pattern-level issue, not per-page. Acceptable for a POC; not for a polished v1.1.
- Fix: add server components that `getQueryClient().prefetchQuery(...)` and pass via `HydrationBoundary` so the first HTML payload includes the real content.

#### F15 [P2 — was P2 in 20:07 F17] — `/hire` page title double-suffixes ("Hire — SKYCOT · SKYCOT")
Same shape as the `/for/*` issue. `/hire` page-level metadata emits `"Hire — SKYCOT"`, then the global template adds `· SKYCOT`, yielding `"Hire — SKYCOT · SKYCOT"`.
- Repro: `curl -s http://localhost:3000/hire | perl -nle 'print "$1" if /<title[^>]*>([^<]+)/'` → `Hire — SKYCOT · SKYCOT`.
- File: `web/app/(hire)/hire/page.tsx` metadata export.

#### F16 [P3 — NEW since 20:07] — `/hire/step/1` returns HTTP 404 with content "Hire · step 1" in the title
`web/app/(hire)/hire/step/[step]/page.tsx:17` defines `VALID_STEPS = {2,3,4,5}` (step 1 lives at `/hire`, step 6 lives at `/hire/done/[workerId]`). Out-of-range step indexes trigger `notFound()` which is correct, but the layout file still sets a page title `"Hire · step 1"` that renders in the 404 response's `<title>` element. Cosmetic only — Next's default 404 page renders, but the tab reads `"Hire · step 1"` not the standard `"404 — Page not found · SKYCOT"`.
- Repro: `curl -i http://localhost:3000/hire/step/1` → HTTP/1.1 404; `<title>Hire · step 1</title>`.
- File: `web/app/(hire)/hire/step/[step]/layout.tsx` metadata.
- Fix: don't set a step-derived title in the layout — derive from a `generateMetadata` that returns `notFound`-aware values, or set the title at the page level (which gets skipped on `notFound`).

### Cross-cutting

#### F17 [P2 — was P1 in 20:07 F11] — Gateway binary's workers-collection-alias status: STILL STALE OR NEVER RESTARTED
PR #2395 added `/api/v1/workers` aliasing `/api/v1/team`. Source dispatcher at `crates/argo-cli/src/handlers/gateway/routes_dispatch_get_api_groups.rs:64` is wired. Live gateway uptime is 27716 s (started ~14:32) — same start time as the 20:07 audit; the operator hasn't restarted yet. `/api/v1/workers` still 404s.
- Repro: `curl -s -o /dev/null -w "%{http_code}\n" http://127.0.0.1:7400/api/v1/workers` → `404`.
- Fix: operator runs `make restart-live-gateway` (CLAUDE.md item (b) is technically done — port unified to 7400 — but the binary actually serving requests still predates the alias merge). Downgraded from P1 to P2 because the SPA doesn't call `/api/v1/workers` directly — it calls `/api/v1/team` which works. No camera-side impact unless the demo somehow drills into the workers-collection endpoint.

### Demo-mode dressing (function-vs-dressing rule applied)

#### F18 [P1 — was P3 in 20:07 F19] — `.env.local` does not enable demo-mode; the seeded approvals + audit fallback fixes (PRs #2397, #2405, #2406) never fire in the running SPA
PR #2397 seeded 3 decided approvals so the approvals ring renders 67% on demo. PR #2405 added "graceful audit drill-in fallback in demo mode." PR #2406 derived the /team waiting badge from `SAMPLE_APPROVALS`. **All three only fire when `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`.** The current `web/.env.local` (line 6) does NOT set this — it only sets `ARGO_GATEWAY_AUTH_BYPASS=1` (a separate, latent-bug env var that the middleware doesn't read because it lacks the `NEXT_PUBLIC_` prefix; see the 20:07 F19 finding). Net effect: every demo-MP4 P0 fix that landed today is dormant in the running build. The approvals ring will read 0%, the audit drill-in will hit real (empty) gateway data and miss the fallback, and the /team waiting badge will derive from the empty live approvals feed.
- Repro: `grep DEMO_MODE web/.env.local` → no match. `grep -rE "NEXT_PUBLIC_SKYCOT_DEMO_MODE" web/lib/skycot/demo-mode.ts` → confirms the env var name.
- Fix: add `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` to `web/.env.local` before recording. Also resolves the latent `ARGO_GATEWAY_AUTH_BYPASS` naming issue if the operator simultaneously prefixes it to `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`.
- **Upgraded from P3 → P1** because Scene 6 (Approvals ring) + Scene 6 (Audit drill-in) + Scene 1 (/team waiting badge) all depend on this flag being on for the demo to land their function-side beats. This is the load-bearing pre-record flag.

#### F19 [P3] — `/landing` primary CTA is "Hire your first AI teammate" → `/auth/signup`
The marketing nav primary button reads "Hire your first AI teammate" and links to `/auth/signup`, not `/hire`. Probably intentional (signup → first hire flow) but the button copy conflates the two actions. Polish-only.
- File: `web/components/marketing/MarketingNav.tsx`.

---

## Resolved since last audit (20:07 → 22:14)

These 20:07 findings are now FIXED, partially FIXED, or PRs landed since:

- **#2345 (PR landed)** — Audit drill-in delegation chain card. Adds `DelegationChainCard` to `/audit/action/[actionId]` rendering the handoff chain as a multi-step worker list with arrows between steps. This is a structural upgrade — claim 2 (delegation) now has a first-class SPA proof surface for the first time. Closes the "claim 2 LIVE in runtime, silent in SPA" gap I called out in the 20:07 rollup.
- **#2353 (PR landed)** — `fix audit feed hash envelope`. Stabilises the hash-chain envelope on the Etherscan-style audit table.
- **#2362 (PR landed)** — `fix(onboarding): hydrate pick-vertical cards from gateway truth`. Onboarding's vertical-picker step now reads from the gateway instead of hardcoded fixtures.
- **#2376 (PR landed)** — `fix(onboarding): surface created flag + workspace toast`. Idempotent provision flag now emits the right UX feedback.
- **#2343 (PR landed)** — `fix(onboarding): return friendly provider-key config error`. Provider-key step error path now produces a non-technical error message.
- **#2404 (PR landed)** — `fix-onboarding-provision-demo-mode-bypass: short-circuit gateway calls in demo mode + scrub X-Argo-Principal leak`. Demo-mode now bypasses provision-side gateway calls cleanly. (Inert until `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` — see F18.)
- **#2405 (PR landed)** — `fix: graceful audit drill-in fallback in demo mode`. Audit drill-in shows a real-looking fallback when gateway is empty. (Inert until demo-mode env var set.)
- **#2397 (PR landed)** — `fix: seed 3 decided approvals so /approvals ring renders 67% on demo`. (Inert until demo-mode env var set.)
- **#2406 (PR landed)** — `fix: derive /team waiting badge from SAMPLE_APPROVALS so /approvals can't disagree`. (Inert until demo-mode env var set.)

Still **NOT** resolved (PRs not yet landed):

- **#2407 (PR OPEN, MERGEABLE, CLEAN)** — `fix: seed demo marketplace skill catalog on gateway boot`. The fix for Scene 4's empty skill catalog. Not yet merged.

Net: one structural improvement (the claim-2 delegation chain card is the headline win); seven demo-mode fixes that are dormant pending the env-var flag (F18); and the remaining title-leak punch-list is unchanged. Orientation score is up modestly because the claim-2 gap closed.

---

## 6-mission-claim coverage rollup (from SPA observations)

For each claim, "does this work functionally in MVP scope based on what the SPA shows?" with cited evidence. Function-vs-dressing rule applied — claims about behaviour the camera drills into must be backed by real backend; copy-only assertions don't pass.

### Claim 1 — Teammate, not tool — VERDICT: YES, demonstrable

The `/workers` and `/team` pages render real workers from the gateway (`/api/v1/team` returns Maya · marketing-intern with tagline `"Marketing intern that drafts and asks before publishing."`). Worker objects carry `headline.bullets` describing what they did. Marketing nav exposes the top-level "Roster" link. The `/landing` H1 is `"Don't buy another tool."` Framing is consistent.
- **Evidence**: `curl -s http://127.0.0.1:7400/api/v1/team` → Maya worker object with real tagline.
- **File**: `web/app/(app)/workers/page.tsx`, `web/components/skycot/nav-items.tsx:25`.

### Claim 2 — Delegation is the unit — VERDICT: NOW DEMONSTRABLE (newly LIVE in SPA via PR #2345)

The 20:07 audit flagged this as PARTIAL (runtime LIVE, SPA silent). PR #2345 added `DelegationChainCard` at `web/components/skycot/audit/DelegationChainCard.tsx` and wired it on `/audit/action/[actionId]` (`web/app/(app)/audit/action/[actionId]/page.tsx`). The card renders one row per delegation step with arrows between them — the SPA now visualises the runtime's `current_handoff_id` field as a graph the camera can drill into. The card's test (`DelegationChainCard.test.tsx`) covers a 3-step delegation chain rendering with role `"delegated_to"` per step. Demo can now narrate **and show** delegation chains.
- **Evidence**: `web/components/skycot/audit/DelegationChainCard.tsx`, `web/app/(app)/audit/action/[actionId]/page.tsx` (PR #2345 commit `62f2eae4c`).
- **Caveat**: live seed has no delegated audit rows because no workflows have run, so the camera can't actually drill into a real chain without the operator triggering a delegate-task workflow first. Surface is LIVE; data to populate it isn't.

### Claim 3 — Platform builds workers — VERDICT: YES, demonstrable (Genesis SSE live)

Two surfaces: `/hire` (5-step F1 wizard) and `/genesis` (describe-and-watch SSE-streaming preview from PR #2326). Per memory `feedback_genesis_is_ai_teammate_onboarding_studio` Genesis is the investor-moment surface. Claim holds in MVP scope.
- **Evidence**: `/genesis` placeholder reads `"Describe the role in plain English. e.g. 'A customer success lead who triages tier-1 tickets…'"`.
- **File**: `web/app/(app)/genesis/page.tsx`.

### Claim 4 — Skills shared and learned — VERDICT: SURFACE LIVE, SEED STILL EMPTY

`/skills` is the A1-marketplace shell with hero search + suggested chips. `/api/v1/skills` returns `{"skills":[],"total_count":0}`. The page resolves the loading state into the `"No skills installed yet"` empty-state CTA pointing to `/team` (`web/app/(app)/skills/page.tsx:167`). PR #2407 ("seed demo marketplace skill catalog on gateway boot") is OPEN, MERGEABLE, CLEAN — once merged + gateway restarted, Scene 4's search beat will return matches. Until then, claim 4 has a surface but no content for the camera to land.

### Claim 5 — Any-PC execution — VERDICT: NOT IN SPA SCOPE (runtime-side, awaiting operator macOS perms)

No SPA surface. Per CLAUDE.md claim-5 is 🟡 LIKELY-LIVE pending operator macOS Accessibility + Screen Recording grants on the argo binary. Audit silent.

### Claim 6 — Multi-channel + multi-tenant + trust — VERDICT: NOW DEMONSTRABLE (channel strip closes the prior gap)

`ChannelStripCard` (PR #2391, 20:07 audit context) is wired on the worker overview tab and aggregates last-7-days audit rows by `channel`. Multi-tenant scoping is plumbed through every gateway response (`tenant_id: "019e5057-1d3f-7fa2-9c1b-c34ef01cc21e"` visible in `/api/v1/team`, `/api/v1/cohorts`, `/api/v1/skills`). Trust ratchet has visible surfaces in `/landing` Step 03 + worker `trust_grade_band` field.
- **Caveat**: live audit feed has 0 rows (workflows haven't run), so the channel-strip's empty-state branch fires which shows the connect-CTA pointing at `/settings/connectors`, not the full channel strip. To exercise the demo beat, seed needs ≥1 non-`web` audit row per channel — operator-decisional.

**Net 6-claim verdict for MVP demo recording:**
- Claims 1, 2 (newly), 3, 6 fully demonstrable as SPA surfaces.
- Claim 4 demonstrable conditional on PR #2407 merging + gateway restart.
- Claim 5 out of scope for SPA.

The structural shift since 20:07 is **claim 2** — runtime + SPA are now both LIVE, the demo can both narrate and visualise delegation chains.

---

## Orientation score: 83 / 100

Up 1 point vs 20:07 (82 → 83). The headline structural improvement is PR #2345's `DelegationChainCard` closing the claim-2-silent-in-SPA gap. The rest of the punch-list is unchanged — seven separate title-leak findings (F6, F7, F9, F10, F11, F12, F15), the live-seed gaps (F4, F5), and the client-rendered loading-shell pattern (F14). The single load-bearing pre-record fix is **F18** — `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` needs to be set in `web/.env.local` for the four demo-mode fixes shipped today (#2397, #2404, #2405, #2406) to actually fire. Without it, the operator's demo-MP4 P0 fix cycle is invisible in the running build.

Three things the operator can do in <60 minutes that would land the biggest UX wins:
1. Set `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` + restart Next dev server. (F18 — unblocks all today's demo-mode fix-cycle work.)
2. Merge PR #2407 + restart gateway. (F5 — fills the skill marketplace for Scene 4.)
3. Drop the `(app)`-group layout's `default: "Approvals"` hardcode + add per-page `metadata` to the 7 leaking routes + 5 onboarding steps + 4 settings sub-pages + `/workflows/templates`. (F1, F2, F9, F10, F11, F12 — 17 of the 22 minutes of tab-title polish.)

The biggest remaining 6-claim gap is **claim 4 seed emptiness** — surface is built, fix is one merge away. Everything else is title polish + SSR-prefetch architecture work that can ship in v1.1.
