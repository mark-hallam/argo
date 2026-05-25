# Scenes 1 + 2 (MISSION Claims 1 + 2) — e2e UX re-audit at 2026-05-25T23:09 Melbourne

- **Auditor**: e2e UX journey agent (read-only)
- **Scope**: v1.0 demo MP4 runbook Scenes 1 + 2 — `/workers` trading-card deck → `/workers/{id}` C2-personality detail → Schedules (a.k.a. Duties) tab → "Run now" → audit drill-in.
- **State of the world**: main at `b1efcf541`. Gateway alive on 7400, schema v81, `/health` returns 200, uptime ~7 h. Dev SPA started on port 60894 (port 3000 occupied by an older `pnpm dev` whose env vars never updated; both servers run the same source).
- **Method**: SPA driven via Claude Preview MCP + curl probes against the gateway. Source-of-truth inspection of code paths via Read / grep. Cross-referenced against the prior audit `docs/skycot/ux-audits/2026-05-25-1100-scene-1-and-2-claim-1-and-2.md`.

## TL;DR

The 13 hours since the 11:00 audit show **no meaningful repair of Scene 1/2's recording path**. Six P0 demo blockers from that audit are still live, one new P0 surfaced (the demo-mint flow returns `needs_onboarding: true` which bounces Scene 1's entry navigation), and a side-effect made the `/team` page render worse than this morning (it now shows the empty-state CTA grid even when the gateway returns Maya). Open ledger shows multiple `fix-*` briefs in the queue (`fix-workers-team-route-ambiguity-and-breadcrumb-drift`, `fix-audit-unknown-worker-and-hipaa-fixture-rows`, `fix-audit-hash-chain-prev-this-hover-and-live-rows`, `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`, `fix-skills-marketplace-search-seed-competitor-pricing`) — all `block`ed, none merged.

- **7 P0** (demo-blockers) · **3 P1** (visible) · **2 P2** (polish)
- Worst single blocker: **Scene 1 cannot start.** Five workers (Mariah / Peta / Priya / Jamal / Nori) do not exist in the gateway — `/api/v1/team` returns one worker named "Maya". The five named in the runbook + the worker-detail mockups are SPA-only fixtures.
- Second worst: **No "Run now" anywhere in the worker UI** and **no Responsibilities/Schedules tab** wired into `WORKER_TAB_IDS`. Scene 2 has no clickable affordance.
- Third worst: **Worker-detail crashes to "Worker not found" against a clean gateway session** because the demo-mint flow returns `needs_onboarding=true`, which leaves the user without a tenant where the real worker lives.

## Headline blockers (top 4)

1. **[P0] Gateway has 1 worker (Maya), not the 5 the runbook names.** Confirmed via `curl /api/v1/team` → `{"total_count":1,"workers":[{"name":"Maya",...}]}`. Scene 1's "Mariah, Peta, Priya, Jamal, Nori — five teammates I hired last week" talk-track is a lie against the real backend. The runbook narrates these names; the camera will see at most one card if the SPA renders gateway data and zero cards if it falls through to the empty state.
2. **[P0] `/team` renders empty-state grid despite gateway returning Maya.** Confirmed via screenshot at `/team`: page shows "Hire your first teammate" hero in dark mode + the "Hire your first AI teammate" 5-starter-prompt grid below it. **No trading-card-deck cards render.** A separate fetch from the same browser session returns the Maya envelope correctly (`/api/v1/team` → 200, 1 worker). Suggests the team page's `useTeam()` -> `fetchTeam()` flow is dropping the response on the first render (auth-bypass-vs-gateway-fetch race after demo-mint), or rendering an empty `workers` array under some other condition.
3. **[P0] No "Run now" affordance exists in the worker UI.** Grep for `Run now|RunNow|run_now` in `web/components/skycot/worker/` returns zero matches. `DutiesCard.tsx:89` titles the section "Responsibilities" (not "Schedules") and only ships an "Add" button (`duties-add` testid). The runbook centerpiece interaction is not implementable today.
4. **[P0] Responsibilities/Schedules tab does not exist.** `WORKER_TAB_IDS` at `web/components/skycot/WorkerTabs.tsx:19-27` is `[overview, activity, memory, skills, trust, escalation, settings]`. The `responsibilities/` directory only contains `new/page.tsx`. A request to `/workers/<id>/responsibilities` falls through to `[tab]/page.tsx`'s `isWorkerTabId()` check, fails, and calls `notFound()`.

## All findings

### F1 [P0] — Gateway team API returns 1 worker (Maya · marketing-intern), not the 5 the runbook narrates

The v1.0 runbook (Scene 1) names Mariah / Peta / Priya / Jamal / Nori as the trading-card-deck personas. The live gateway returns:

```json
{"total_count": 1, "workers": [{"id": "019e5dbb-c03d-79a0-a9a4-ee60bb3c8218",
 "name": "Maya", "role": {"vertical": "marketing-intern", ...}}]}
```

The five personas live only as SPA-side fixtures (`web/lib/sample-personas.ts`). Per the function-vs-dressing rule, persona names ARE dressing — but Scene 1 immediately drills into a worker card and Scene 2 narrates that worker's duties, so the data on those cards must be real or the camera catches the discontinuity.

- Reproduction: `curl http://127.0.0.1:7400/api/v1/team` → 1 worker.
- File path: gateway team handler (Rust) — but the user-facing fix is to run a demo seed CLI that mints Mariah/Peta/Priya/Jamal/Nori as real worker rows. See `mission-demo-seed-data-bootstrap` brief and the `DEMO_SEED_PERSONAS` constant at `web/lib/sample-personas.ts:171-188`.
- Suggested fix: Land + run the demo seed CLI before recording. Operator memory `feedback_no_theater_in_demo` already locked this as Option B; carried over from the 11:00 audit.

### F2 [P0] — `/team` renders empty-state CTA grid despite the gateway returning Maya

Visually captured (screenshot saved during this run) on `http://127.0.0.1:60894/team` while the user has a fresh `__Host-session` cookie minted via `/api/auth/demo-mint`. Behaviour:

- `fetch('/api/v1/team', { credentials: 'include' })` from the same page returns `{"total_count":1, "workers":[Maya]}` → 200.
- Yet the rendered DOM shows the page header "Team" + "0 working" subtitle + the `<TeamEmptyState>` "Hire your first AI teammate" CTA grid.

So either `useTeam().data?.workers` is briefly `[]` on first render and the empty state captures it, or `fetchTeam` is treating the envelope as empty when no auth-bypass flag is set. Operator does NOT see Maya's trading card. Scene 1's first beat is unrecordable.

- Reproduction: visit `/auth/signup` → mint demo session → navigate `/team` → screenshot. Empty state renders.
- File path: `web/lib/api/team.ts:54-78` (response envelope parsing) and `web/app/(app)/team/page.tsx:171-186` (render branches).
- Suggested fix: Trace whether the response shape `{workers, total_count}` is being unwrapped correctly. Verify `total_count: 1` survives the React Query cache; verify the team page isn't reading from a stale empty cache. Add a `dataSource = "gateway"` chip to the page so the operator can see at-a-glance whether mock or live data is rendering.

### F3 [P0] — No "Run now" anywhere in the worker UI

Grep across `web/components/skycot/worker/`:

```
$ rg 'Run now|RunNow|run_now' web/components/skycot/worker/
# empty
```

The DutiesCard ships an "Add" button (`data-testid="duties-add"`) but no per-duty fire affordance. The runbook Scene 2 says: "Operator clicks 'Run now' on `morning_kickoff`" — there is no button to click.

- File path: `web/components/skycot/worker/DutiesCard.tsx`
- Suggested fix: Author a per-duty `<button>Run now</button>` that POSTs to a yet-to-author gateway endpoint (`POST /api/v1/duties/<id>/run`). Both the SPA control and the gateway route are missing today. Carryover from F11 in the 11:00 audit.

### F4 [P0] — `/workers/<id>/responsibilities` falls through to `notFound()`

`WORKER_TAB_IDS` at `web/components/skycot/WorkerTabs.tsx:19-27` lists 7 tabs: `[overview, activity, memory, skills, trust, escalation, settings]`. The runbook Scene 2 narrates clicking into a "Duties" / "Schedules" tab — that tab is not in the array, not labelled, and the `responsibilities/` route directory only ships `new/page.tsx` (the create form). `responsibilities/page.tsx` does not exist. The `[tab]/page.tsx` catch-all calls `notFound()` for any tab not in `WORKER_TAB_IDS`.

Curl confirms: `GET http://127.0.0.1:60894/workers/019e5dbb-c03d-79a0-a9a4-ee60bb3c8218/responsibilities` → HTTP 200 with `notFound` strings throughout the rendered HTML (Next's not-found shell).

- File path: `web/components/skycot/WorkerTabs.tsx:19-39` + `web/app/(app)/workers/[workerId]/responsibilities/` (missing `page.tsx`).
- Suggested fix: Add `"responsibilities"` (label "Schedules") to the tab IDs/labels arrays and author the page. Reuses `useWorkerOverview(workerId).duties[]`.

Note: `evals/page.tsx` HAS been authored since the 11:00 audit (a partial improvement) but `WORKER_TAB_IDS` still doesn't list `evals`, so the new page is unreachable through the tab strip.

### F5 [P0] — UI section labelled "Responsibilities", not "Schedules" — vocab drift from operator-locked memory

Memory `project_vocabulary_duty_vs_schedule` (operator-locked 2026-05-23) requires the UI vocabulary to say "Schedule" not "Duty". Today: `DutiesCard.tsx:89` renders `<CardTitle>Responsibilities</CardTitle>`. Neither "Duty" nor "Schedule" — a third option, and not the operator-locked one. The Scene 2 talk track says "she has duties" — but the camera-visible label is "Responsibilities". Scene-vocabulary mismatch.

- File path: `web/components/skycot/worker/DutiesCard.tsx:89`
- Suggested fix: Rename CardTitle to "Schedules". Carry the same rename through the missing tab (F4) and any sibling cards.

### F6 [P0] — Worker-detail page renders "Worker not found" against a live gateway worker

While driving the SPA in this run: navigating to `/workers/019e5dbb-c03d-79a0-a9a4-ee60bb3c8218/overview` rendered the `WorkerNotFound` component (`web/components/skycot/WorkerNotFound.tsx`) despite the same browser session being able to GET `/api/v1/workers/<id>` and receive a 200 with Maya's full payload. Root cause is an interaction between server-side `fetchFreezeState` (which silently returns `null` on missing token) and the client `useWorker(workerId)` query firing before the React-Query cache has a valid session token, throwing `WorkerNotFoundError` on the initial 401-converted-to-404. Hard refresh after the session is hot resolves it, but Scene 1's flow (click trading card → land on detail) does NOT have a moment to wait for the cache to warm.

- File path: `web/app/(app)/workers/[workerId]/WorkerLayoutShell.tsx:43-53` (the not-found branch fires on any error with status 404 or `WorkerNotFoundError`), `web/lib/api/workers.ts:97-121`.
- Suggested fix: Wait for the session peek before triggering the worker query; or have `fetchWorker` treat HTTP 401 as auth-not-ready rather than 404.

### F7 [P0] — Demo-mint flow returns `needs_onboarding: true`, bouncing Scene 1's entry navigation to `/onboarding/pick-vertical`

`POST /api/auth/demo-mint` returns `{ ok: true, needs_onboarding: true }` on a fresh session. When the SPA recognizes the onboarding flag, the operator gets bounced to `/onboarding/pick-vertical` instead of the workers roster. The runbook (Scene 1) opens with `/workers` already populated; today's demo-mint flow forces a 4-step onboarding wizard before that surface is reachable.

- Reproduction: `curl -X POST http://127.0.0.1:60894/api/auth/demo-mint -H 'content-type: application/json' -d '{"email":"demo@skycot.com"}'` — body contains `"needs_onboarding":true`.
- File path: `web/app/api/auth/demo-mint/route.ts:155-158` (the flag is hardcoded `needs_onboarding: true`).
- Suggested fix: The recording session should not need onboarding. Either (a) gate `needs_onboarding` to false when a demo seed has already minted the 5 workers, or (b) have `mission-demo-seed-data-bootstrap` flip the tenant's `needs_onboarding` to false as the last step.

### F8 [P0] — Audit `/audit/action/<id>` 404s on IDs the audit list just returned

Confirmed unchanged from the 11:00 audit. `/api/v1/audit?limit=3` returns 3 rows with valid IDs; immediately calling `/api/v1/audit/action/<id>` returns `{"error":"not found"}`. Scene 2's pass cue is "audit row appears with `duty_id`, `session_id`, and `tenant_id` payload intact" — but the camera cannot drill into that row without a 404. Function-side fabrication on a destination the camera lands on = POC blocker.

- Reproduction:
  - `id=$(curl -s 'http://127.0.0.1:7400/api/v1/audit?limit=1' | jq -r '.items[0].id')`
  - `curl -s "http://127.0.0.1:7400/api/v1/audit/action/$id"` → `{"error":"not found"}`
- File path: gateway-side disagreement between the list and action routes (Rust). Brief `fix-audit-hash-chain-prev-this-hover-and-live-rows` is `block`ed in the ledger.

### F9 [P0] — Every audit row has `worker.name = "unknown"` and `action_type = "hipaa_compliance_check"`

The audit timeline today is filled with stray test-fixture rows:

```json
{"action_type":"hipaa_compliance_check","verb":"hipaa_compliance_check",
 "worker":{"id":"","name":"unknown"}, "timestamp":"2026-05-25T06:03Z", ...}
```

A healthcare compliance action on a marketing-intern tenant with an empty worker_id. Scene 2's "audit row lands within 5s tagged with the worker's `tenant_id` and the fired `duty_id`" cannot pass because (a) the worker_id is empty, (b) the action is the wrong vertical, (c) the action_type is wrong. Brief `fix-audit-unknown-worker-and-hipaa-fixture-rows` is `block`ed.

- Reproduction: `curl 'http://127.0.0.1:7400/api/v1/audit?limit=3'` → every row has `worker.name = "unknown"`.

### F10 [P1] — Browser tab title is the raw UUID on every worker-detail route

`document.title` on `/workers/<id>/overview` is `"019e5dbb-c03d-79a0-a9a4-ee60bb3c8218 · Overview"` (and lowercase `019e5dbb-... · responsibilities` when navigating to a missing tab). The Next.js metadata template is using the URL segment instead of the worker name. The camera captures the title bar + cmd-Tab thumbnails during the recording.

- File path: `web/app/(app)/workers/[workerId]/layout.tsx` (no `generateMetadata` defined; falls back to route segment).
- Suggested fix: Add `generateMetadata` that reads the worker name server-side and returns `{ title: '${name} · Overview' }`.

### F11 [P1] — `/workers` redirects to `/team` on hasWorkers, creating a nav-vs-URL contradiction

`/workers/page.tsx` does `router.replace("/team")` when at least one worker exists. The left nav highlights "Workers" but the URL becomes `/team` and the breadcrumb on the detail says "Team". Memory `project_workers_page_design_locked` describes "Workers" as the canonical surface, but the implementation makes "/team" canonical. Demo viewers will catch the URL flicker.

- File path: `web/app/(app)/workers/page.tsx:24-46`
- Suggested fix: Choose ONE canonical URL. Either delete the redirect and build the trading-card-deck UI at `/workers` directly, or relabel the nav to "Team" and remove the breadcrumb mismatch.

### F12 [P1] — Two env-var name mismatches in `.claude/launch.json` keep the demo data path off

`.claude/launch.json` sets `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` + `NEXT_PUBLIC_ARGO_DEMO_MODE=1`. The middleware + `fetchTeam` read `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS` (with `ARGO_` prefix) and `lib/skycot/demo-mode.ts:20` reads `NEXT_PUBLIC_SKYCOT_DEMO_MODE`. Neither env var matches the canonical reader. Net: a fresh `pnpm dev` from this launch.json gives the operator the empty-state CTA grid; an agent re-running this audit hits the same problem.

- File path: `.claude/launch.json:6-13` (unchanged since the 11:00 audit; this is the 14-hour-old finding F14 from that report).
- Suggested fix: Update launch.json to use the canonical names: `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` + `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`. Or expand the env-var readers to accept both shapes with a deprecation log line.

### F13 [P2] — `TeammateCard.tsx` is 582 LOC (exceeds the project's 500-LOC budget)

CI invariant per `CLAUDE.md` ("No file > 500 LOC") is currently violated. Not a demo-blocker on its own; relevant because the trait-map fix (Peta missing) and any other change to this file will trip the LOC gate before merge. Carryover from F6 in the 11:00 audit.

- File path: `web/components/skycot/TeammateCard.tsx` (582 lines)
- Suggested fix: Extract the `TRAITS_BY_NAME` + `LEVEL_BY_NAME` maps + the `<style jsx>` block into sibling files.

### F14 [P2] — `Peta` missing from `TRAITS_BY_NAME` + `LEVEL_BY_NAME` despite being in the runbook persona list

If the demo seed runs and creates Peta as a worker, `TeammateCard.tsx:51-67` will hash-derive her trading-card level and render an empty trait chip row. Memory `project_pm_cohorts_peta_patrick` describes her as Project Coordinator L2 with specific personality traits.

- File path: `web/components/skycot/TeammateCard.tsx:51-67` and `WorkerPersonalityCard.tsx:35-95`.
- Suggested fix: Add Peta to both maps. Tied to F13 (the LOC budget will fail the commit unless that's fixed in parallel).

## Function-vs-dressing classification (per `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`)

| Finding | Function or Dressing | POC blocker? |
|---|---|---|
| F1 (5 workers missing) | **Function** — Scene 1 drills into worker cards | YES |
| F2 (`/team` empty despite Maya) | **Function** — the camera lands here in Scene 1 | YES |
| F3 (no Run now) | **Function** — Scene 2 centerpiece interaction | YES |
| F4 (Schedules tab missing) | **Function** — Scene 2 navigation | YES |
| F5 ("Responsibilities" vocab) | **Function** — talk-track says "duties"; UI says third thing | YES |
| F6 (Worker not found race) | **Function** — Scene 1 click-through | YES |
| F7 (needs_onboarding=true) | **Function** — Scene 1 entry navigation | YES |
| F8 (audit drill-in 404) | **Function** — Scene 2 camera drill-in | YES |
| F9 (unknown worker / hipaa rows) | **Function** — Scene 2 pass cue | YES |
| F10 (UUID in title bar) | **Dressing** — camera-visible but cosmetic | NO (P1 visible) |
| F11 (workers/team URL contradiction) | **Dressing** — camera-visible URL bar | NO (P1 visible) |
| F12 (launch.json env-var names) | **Setup** — blocks demo prep, not the recording itself | NO (P1) |
| F13 (TeammateCard 582 LOC) | **CI gate** — blocks the fix, not the recording | NO (P2) |
| F14 (Peta missing from traits) | **Dressing** — fired only if seed adds Peta | NO (P2) |

**Net for the POC/investor demo**: 9 function-side P0 blockers. Scenes 1 + 2 are unrecordable today.

## Cross-cutting observation

The 11:00 audit's headline blockers are still in place. Five `fix-*` briefs targeting them appear in the ledger (commits `dcc46a46b`, `5d899a9ac`, `2c4d29739`, `74ed65742`, `1e4412794`) but all sit in `block` state — none have shipped. The supervisor + 3 workers are listed as ACTIVE in CLAUDE.md, so the swarm is consuming work but not converging on the recording-blocker list. Operator should either (a) promote these 5 fix-* briefs above the Track P / lint waves currently at the top of `BUILD_STATUS.md`, or (b) record the demo before they're done if dressing-side workarounds are acceptable for the POC audience (but per the function-vs-dressing rule, F1-F9 are NOT dressing-side workaroundable).

## Resolved since the 11:00 audit

- **F8 from 11:00 (UUID in title bar)** → still present, re-filed as F10 here.
- **F1 from 11:00 (Charter object crash)** → no longer the proximate failure: the worker-detail page now renders "Worker not found" before it can crash on the charter shape. The underlying type mismatch (charter:string vs charter:{text,version,…}) likely still exists in `WhatSheDoesCard.tsx:156`, but the page never reaches that render path against today's session-token-race condition. Once F6 is fixed, F1-from-11:00 will surface again.
- **`evals/page.tsx`** → authored since 11:00 (good!), but still not wired into `WORKER_TAB_IDS` so the tab strip can't reach it.

## Orientation score

**52 / 100.**

Same design system (C2 trading cards, C2 personality, gradient brand) — when the data path works the design is beautiful. The lower score vs the 11:00 audit (62) reflects that:

1. `/team` got worse (now shows empty state with real gateway data, where previously the mock path rendered cards),
2. The demo-mint flow now demands onboarding completion before Scene 1 is reachable,
3. The drilled-into worker-detail page renders Not Found against a real worker ID in the same session that can fetch the worker.

The implementation gap (function-side data path + missing pages + missing buttons) widened in the last 14 hours despite the supervisor running. The 9 function-side P0s are the entire pre-record gate; the design system is no longer the bottleneck.

## Reproduction commands (operator can copy-paste)

```bash
# 1. Confirm the team API returns only Maya
curl -sS http://127.0.0.1:7400/api/v1/team | jq '.total_count, .workers[].name'

# 2. Confirm the audit list is full of stray hipaa rows with no worker
curl -sS 'http://127.0.0.1:7400/api/v1/audit?limit=5' | jq '.items[] | {action_type, worker}'

# 3. Confirm audit drill-in 404s on an id the list just returned
id=$(curl -sS 'http://127.0.0.1:7400/api/v1/audit?limit=1' | jq -r '.items[0].id')
curl -sS "http://127.0.0.1:7400/api/v1/audit/action/$id"

# 4. Confirm responsibilities falls through to notFound()
curl -sS http://127.0.0.1:60894/workers/019e5dbb-c03d-79a0-a9a4-ee60bb3c8218/responsibilities | grep -c 'notFound'

# 5. Confirm needs_onboarding=true on demo-mint
curl -sS -X POST http://127.0.0.1:60894/api/auth/demo-mint \
  -H 'content-type: application/json' -d '{"email":"demo@skycot.com"}' | jq '.needs_onboarding'
```
