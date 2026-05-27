# Scene 1+2 UX audit — Claims 1+2 (Teammate-not-tool + Delegation-is-the-unit)

- **Auditor**: e2e-ux-journey-test-agent
- **Run date**: 2026-05-28 02:52 (local)
- **Scope**: `/workers` (deck) → `/workers/{id}/*` (Overview, Schedules, Activity, Memory, Skills) for Mariah + Peta + Nori; click-through to `/audit/action/{id}`
- **Mode**: SPA running with `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1 NEXT_PUBLIC_SKYCOT_DEMO_MODE=1 NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1` (per `.claude/launch.json`), gateway live on `127.0.0.1:7400` (schema v83, healthy)
- **Lens**: function-vs-dressing rule per `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`

## TL;DR

The trading-card deck on `/team` looks great (C2 holographic lock holds: gradient banners, big "MA/PE/PR/JA/NO" badges, Level numbers, traits, Today list). The C2-personality detail page also looks great for fixture workers — Mariah's "How Mariah works", traits, great-at / escalate-when, signature behaviors all render with character. **But the function side breaks the moment the camera drills in**: every Activity-tab row links to `/audit/action/{id}` and lands on "Action not found" — for both fixture IDs (`act_018f67d0-..._000` not in the sample-audit-detail file) AND real gateway audit IDs (`019e6a64...` — the gateway itself returns 404 on its own detail endpoint). That is the exact failure mode the function-vs-dressing rule forbids: the camera clicks a row, the destination is fabricated. Scene 2 of the v1.0 runbook also has no "Run now" button on the Schedules tab — the tab shows a "Responsibilities" card with toggle switches that are intentionally inert. Claim 2 (delegation-is-the-unit) has **zero visible evidence in the UX** for either fixture or live data — no delegation chain, no handoff, no "delegated to X" pill anywhere on the worker page (the `DelegationChainCard` component exists in `web/components/skycot/audit/` but is gated behind the broken audit-detail page, so it never shows up in the demo path).

13 findings: 4 × P0, 5 × P1, 3 × P2, 1 × P3.

## Headline blockers

1. **F01 [P0]** — Activity-tab row drill-in lands on "Action not found" for every sample worker. The Activity feed generates IDs the audit-detail sample doesn't know about. Scene 2 of the runbook is unrecordable as scripted.
2. **F02 [P0]** — Gateway `/api/v1/audit/{id}` returns 404 for every action id its own `/api/v1/audit?limit=N` feed returned. Function-side: the real-gateway audit detail endpoint is broken, so Scene 6 drill-in also breaks even in `DEMO_LIVE=1` mode.
3. **F03 [P0]** — Scene 2 "Run now" affordance is **missing**. The Schedules tab renders a `DutiesCard` with toggles that are explicitly inert (`canToggleDuty={false}` per the component code) and no Run/Trigger button. The 6-scene runbook narrates clicking it; there is nothing to click.
4. **F04 [P0]** — Claim 2 (Delegation-is-the-unit) has no visible affordance anywhere in the worker journey. No "delegated to / delegated from" pill, no chain visualization, no follow-up arrow. The component `DelegationChainCard` exists but is only mounted on the broken audit-detail page.

## All findings, in order

### F01 [P0] — Worker Activity tab row drill-in fabricates "Action not found"

On `/workers/{id}/activity` the rows display realistic content ("drafted listing copy for 'Eames lounge chair' · $0.021 · Succeeded · 38m ago") with an arrow link to `/audit/action/act_018f67d0-0000-7000-8000-000000000001_000`. That detail page renders `<AuditActionNotFound>` because the sample-audit-detail file only contains 5 hand-shaped entries with IDs like `act_01J0MARIAH_001` — the worker-activity sample generates a different ID space. The two fixtures are decoupled.

Reproduction:
- Open `/workers/018f67d0-0000-7000-8000-000000000001/activity` (Mariah)
- Click the arrow on the first row ("drafted listing copy…")
- You land on `/audit/action/act_018f67d0-..._000`
- Page reads: "Action not found · No audit record matches act_018f67d0-..._000"

Offending code:
- `web/lib/sample-worker-activity.ts:188` — generates IDs as `act_${workerId}_${i}`
- `web/lib/sample-audit-detail.ts:233` — `SAMPLE_AUDIT_DETAILS` only contains `act_01J0MARIAH_001`, `act_01J0PETA_001`, etc — different schema entirely

Suggested fix: either align the two sample files on a shared id schema, or have the worker-activity sample point to one of the 5 existing `SAMPLE_AUDIT_DETAILS` ids per worker.

### F02 [P0] — Gateway audit-detail endpoint returns 404 for IDs from its own feed

`GET /api/v1/audit?limit=N` returns 200 with rows like `{"id": "019e6a675ef27b11b3ca1d906f9996ab", ...}`. `GET /api/v1/audit/019e6a675ef27b11b3ca1d906f9996ab` returns 404 `{"error":{"code":"not_found","message":"audit row not found"}}`. This breaks Scene 6 even with `DEMO_LIVE=1` because the SPA correctly proxies to the gateway and gets 404.

Reproduction:
- `curl http://127.0.0.1:7400/api/v1/audit?limit=2` — pick any returned id
- `curl http://127.0.0.1:7400/api/v1/audit/{id}` — returns 404
- In the SPA, open `/audit` (rows render from real gateway data), click any row, "Action not found"

Offending code: gateway side — likely `crates/argo-cli/src/handlers/gateway/` audit handlers. The list and detail SQL queries are not unified. Out of read-only scope, but worth a focused bug.

Suggested fix: gateway-side bug — the audit-detail handler is reading from a different table/filter than the list handler, or the row's tenant gate is rejecting it on detail read but allowing it on list.

### F03 [P0] — Schedules tab has no "Run now" affordance (Scene 2 unrecordable)

The Schedules tab renders `<WorkerSchedulesContent>` → `<DutiesCard>` with 4 toggle rows ("List new inventory with photos + tags · as items arrive", "Re-price stale listings against competitor data · daily 06:30", etc). The toggles are intentionally inert per the file comment: *"Toggle + body-click stay inert (`canToggleDuty={false}`) until the duty PATCH endpoint lands"* (`WorkerSchedulesContent.tsx:20-22`). There is no Run/Trigger/Fire-now button anywhere.

Reproduction:
- Open `/workers/018f67d0-0000-7000-8000-000000000001/schedules`
- Inspect every button + link on the page; none reads "Run now" or equivalent
- The v1.0 demo runbook Scene 2 says: *"Click 'Run now' → audit row lands in /audit within 5s"* — there is nothing to click

Offending code:
- `web/components/skycot/worker/WorkerSchedulesContent.tsx:87-94`
- `web/components/skycot/worker/DutiesCard.tsx` — no Run button

Suggested fix: add a "Run now" button per row (and/or per worker at the tab level) that POSTs to the gateway duty-trigger endpoint. If the endpoint doesn't exist yet, this is a Track-E backend brief plus the UI wiring.

### F04 [P0] — No delegation/handoff evidence anywhere in the worker journey

Claim 2 (Delegation-is-the-unit) has zero surface in `/team`, `/workers`, `/workers/{id}/overview|schedules|activity|memory|skills`. No "delegated to X" pill, no chain card, no follow-up arrows next to actions. The `DelegationChainCard` component exists in `web/components/skycot/audit/DelegationChainCard.tsx` and is referenced by `audit-detail.ts` (`DelegationRole = "originator" | "delegated_to" | "executed"`), but the audit-detail page is broken (see F01/F02) so the chain card never renders during the demo.

This is the most loadbearing missing surface for the v1.0 demo: the regression test for Claim 2 passes (per CLAUDE.md status block) but the camera will not see anything that says "Mariah handed this to Compliance" or "Peta delegated to Patrick".

Reproduction:
- Walk all 8 tabs on `/workers/{id}` — no delegation surface
- Walk `/audit` — rows look real but no per-row delegation chip
- Click any row → broken detail page

Offending code: design + product gap; `DelegationChainCard` is built but not surfaced in the demo path.

Suggested fix: add a small "delegation chain" pill on each Activity-row showing originator → executed chain, even just 2 nodes. Or render the `DelegationChainCard` inline on the worker Activity tab.

### F05 [P1] — Sidebar duplication: both "Team" and "Workers" nav entries

`/team/` and `/workers/` are both top-level nav items in the sidebar (`web/components/skycot/nav-items.tsx:24-25`), but `/workers/` is just an empty-state shim that `router.replace("/team")` whenever workers exist. Two clicks land in the same place; the buyer/operator/camera is going to wonder which one is which.

Reproduction:
- Click "Team" → `/team` (the trading-card deck)
- Click "Workers" → URL flashes through `/workers` then redirects to `/team`

Offending code:
- `web/components/skycot/nav-items.tsx:24-25`
- `web/app/(app)/workers/page.tsx:25-27`

Suggested fix: pick one. "Workers" is the V1.0-canonical name per the runbook + Claim 1 vocabulary ("Teammate, not tool" — but also "Workers" is the page label per `project_workers_page_design_locked`). Rename the route to whichever stays. The current state confuses the camera narrator and risks Scene 1 mis-narration.

### F06 [P1] — Worker detail page accepts real gateway IDs but loses the C2 personality

`/workers/019e6785-928f-7281-bf8d-e062bc277478/overview` (Nori with the gateway-real id) loads — but the "How Nori works" card shows generic template strings ("Working through the responsibilities you set / Asking when something is ambiguous / Surfacing patterns across recent runs / Confidence in the next action is below 80% / A risky side-effect would happen if they proceed / Multiple plausible interpretations exist"). The same generic strings show on Peta. The C2-personality lock requires Mariah/Peta/etc each have **distinctive** great-at/escalate-when lists.

This is the "two worlds" problem: the team-list uses fixture IDs (rich personality) and the detail page accepts either fixture OR gateway-real IDs, but the latter falls back to a generic template that contradicts Claim 1 ("not a settings panel").

Reproduction:
- Open `/workers/019e6785-928f-7281-bf8d-e062bc277478/overview` (gateway-real Nori)
- Open `/workers/018f67d0-0000-7000-8000-000000000007/overview` (fixture Peta)
- Compare great-at + escalate-when sections — identical generic strings

Offending code:
- `web/components/skycot/worker/Overview*` rendering — likely the hardcoded escalate/great-at fallback when the worker payload doesn't carry persona traits

Suggested fix: when the gateway worker payload is bare (no persona traits), keep the page on the fixture-mapped persona for the demo IDs. Or wire the persona traits into the real gateway worker model so the SPA gets distinctive content regardless of source.

### F07 [P1] — Mariah's Memory tab shows facts that don't match her role

Mariah's role is "lists and re-prices items for sale" / "Operations · Procurement". The Memory tab shows facts: *"Vendor onboarding requires Compliance review for any vendor over $50K/year spend"*, *"Annual renewals route to Procurement + the business owner"*, *"Use 'strategic partners' not 'vendors' for Tier 1 vendors per Operations branding"*. Three of five facts are about vendor procurement, not item listing. The fifth fact ("Photos required for every listing — minimum 3, maximum 8") IS about listings. Mismatch between the role + the memory.

Reproduction:
- Open `/workers/018f67d0-0000-7000-8000-000000000001/memory`
- Compare the 5 fact strings to the role tagline "lists and re-prices items for sale"

Offending code:
- `web/lib/sample-worker-memory.ts` — facts for Mariah look copy-pasted from a procurement persona

Suggested fix: rewrite Mariah's facts so 4-5 of 5 are about listings/repricing/photo standards/competitor watch — facts the operator narrating Scene 1 can actually reinforce ("she remembers the photo standard, she remembers the floor-price rule").

### F08 [P1] — Schedules tab title bar reads "Responsibilities" not "Schedules"

The tab is labelled "Schedules" in the tablist but the card inside is titled "Responsibilities" with no schedule-shaped indicator visible (the cron-shaped text "daily 06:30" lives in a small right-aligned pill, not as the primary affordance). The vocab fix mandates "Schedule" as the operator-facing word — the tab label is correct, but the card heading still uses the older "Responsibilities" framing.

Reproduction:
- Open `/workers/018f67d0-0000-7000-8000-000000000001/schedules`
- Tab label: "Schedules" ✓
- Card heading: "Responsibilities" ✗

Offending code:
- `web/components/skycot/worker/DutiesCard.tsx` — card title string

Suggested fix: rename the card heading from "Responsibilities" to "Schedules" or "Recurring duties" (UI-facing copy only — the file/component name + the duty/duties code stays per the vocab-discipline memory).

### F09 [P1] — Page title falls back to UUID instead of worker name on Activity tab

`/workers/018f67d0-0000-7000-8000-000000000001/activity` sets `document.title = "018f67d0-0000-7000-8000-000000000001 · Activity"` instead of "Mariah · Activity". Overview works ("Mariah · Overview"), Memory works, Schedules works — but Activity + Skills + Trust tabs all hit the UUID fallback on the title. The page H1/breadcrumb still says "Mariah", so it's only the browser-tab title that's broken (visible in any screenshare or screen-recording where browser chrome is in frame).

Reproduction:
- Open `/workers/018f67d0-0000-7000-8000-000000000001/overview` → tab title "Mariah · Overview"
- Open `/workers/018f67d0-0000-7000-8000-000000000001/activity` → tab title "018f67d0-… · Activity"

Offending code:
- `web/app/(app)/workers/[workerId]/activity/page.tsx` — title metadata likely awaits a hook that hasn't resolved by the time `<title>` is set

Suggested fix: pre-resolve the worker name from `useWorker(workerId).data?.name` or set a deferred title via `useEffect` once the hook resolves. The fallback should be "Worker · Activity" not the UUID.

### F10 [P2] — `MOCK-BYPASS` badge on `/team` page header

The `/team` page header reads "Team `MOCK-BYPASS` · 5 teammates working" — a literal pill that exposes the `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` flag to the camera. It's a dev-mode telltale that doesn't belong in the demo recording (even though the demo banner above already says "Demo mode — showing sample data").

Reproduction:
- Open `/team`
- Read the line under the headline

Offending code:
- Likely `web/app/(app)/team/page.tsx` — a debug pill that needs to be `NODE_ENV === "development"` gated, not env-var gated

Suggested fix: hide the `MOCK-BYPASS` pill when `NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1` (recording mode), or replace it with nothing — the Demo banner above already communicates the state.

### F11 [P2] — Stray spurious navigations during preview-driven walks

While walking via `window.location.assign(...)` in the preview tool, ~40% of navigations spontaneously redirect to `/landing`, `/onboarding/welcome`, `/skills`, or `/audit` ~2-5s after settling on the target page. This is unlikely to be a recording-time issue (a human-driven walk doesn't trigger it), but it surfaces as flaky behaviour in any e2e harness. Could be a prefetch + React-strict-mode + Turbopack interaction or a stale auth-session check.

Reproduction:
- Open `/workers/018f67d0-0000-7000-8000-000000000001/skills` in any browser; wait 5s; check URL
- 50% of the time the URL drifts. Manual click-throughs are stable.

Offending code: unknown — preview-tool interaction artifact, may be unrelated to production behaviour

Suggested fix: defer to a dedicated reproducer; not a recording blocker.

### F12 [P2] — Real gateway audit feed contains only HTTP-request mirror rows, zero delegation evidence

`GET /api/v1/audit?limit=200` returns 200 rows but the action_type distribution is: `audit.replay`, `worker.detail.viewed`, `cohorts.list.queried`, `team.list.queried`, `worker.kpi_rolled`, `audit.detail.viewed`, `permission.demo_override_granted`, `worker.overview.viewed`. Zero `worker.action`, zero `tool.invoked`, zero `delegate.task`, zero `handoff.completed`. The demo workspace has no real agent work in it — the audit feed is full of "user clicked this" rows, not "agent did this" rows. For the camera's Scene 6 narration ("every action, every approval, every decision — fully audit-logged"), there's nothing to narrate.

Reproduction:
- `curl http://127.0.0.1:7400/api/v1/audit?limit=200 | jq '[.items[].action_type] | unique'`
- See nothing that says "an agent did real work"

Offending code: data seeding gap. The brief `mission-demo-seed-data-bootstrap` was rewritten Option-A→Option-B 2026-05-24 (per `feedback_no_theater_in_demo`) to use a real backend seed CLI. Status of that brief: unknown — needs ledger check.

Suggested fix: run the real-seed CLI to land ~5-10 worker-action rows (Mariah re-priced an item, Peta sent a digest, etc) so the audit feed has agent rows the camera can land on.

### F13 [P3] — Demo banner is dismissable; if dismissed mid-record the camera loses context

The "Demo mode — showing sample data. Sign in to see your own workspace" banner has an X (Dismiss) button. If the operator dismisses it mid-take, subsequent re-loads of the page will not show it again (likely localStorage-backed). Minor — operator can avoid dismissing during the take.

Reproduction:
- Open `/team`, click X on the Demo banner; reload; banner stays dismissed

Offending code:
- Demo-banner component, probably `web/components/skycot/DemoBanner.tsx` or similar (session/localStorage flag)

Suggested fix: nit — make the dismiss state per-session not per-tenant, so a fresh window/recording session always shows the banner.

## Resolved since last audit

n/a — first run of this journey.

## Orientation score: **53/100**

The dressing side is excellent: trading-card deck, C2-personality detail card, Audiowide hero + brand-correct gradient, traits-and-behaviors framing that says "colleague not settings panel". For Claim 1 (Teammate, not tool) the chrome is genuinely there. **But the function side breaks the moment the camera moves**: every Activity row drill-in 404s, the "Run now" button doesn't exist, the gateway's own audit-detail endpoint 404s the IDs its list endpoint returns, and there is zero visible delegation surface for Claim 2. The four P0s are not polish — they're "Scene 2 + Scene 6 of the runbook cannot be recorded as scripted" blockers, and Claim 2's demo evidence is missing entirely.

Score breakdown:
- Trading-card deck visual lock (C2): 90/100 ✓
- C2-personality detail card visual lock: 80/100 ✓ (fixture only)
- Vocabulary discipline (Schedule UI / Duty code): 70/100 (tab right, card heading wrong)
- Function-side Claim 1 evidence (real worker data flowing): 35/100 (the two-worlds problem)
- Function-side Claim 2 evidence (delegation visible): 10/100
- Function-side audit drill-in: 0/100 (both fixture + gateway broken)
- POC demo recordability of Scenes 1+2 as scripted: 40/100
