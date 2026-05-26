# Scenes 1 + 2 — Claims 1 + 2 (Teammate-not-tool, Delegation-is-the-unit)

**Date:** 2026-05-26 20:07 AEST
**Scope:** `/workers` deck + `/workers/{id}` tabs (all 7 tabs) + Schedules/Duties affordance + "Run now" + audit-row landing.
**Tester:** e2e-ux-journey-test-agent (preview MCP + curl against gateway).
**Lens:** function-vs-dressing per `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`.
**Prior baseline:** `docs/skycot/ux-audits/2026-05-26-1441/scene-1-and-2-claim-1-and-2.md` (13 findings, 38/100). Significant deltas: #2367 (scrub Sam/Rio, lock canonical roster) MERGED + #2403 (gate freeze button on permissions) MERGED + #2366 (render empty chain fallbacks) MERGED.
**Servers used:** gateway `http://127.0.0.1:7400` (schema_version 83, uptime ~5h), Next dev on :3000.

---

## TL;DR

**3 P0, 4 P1, 3 P2 — 10 findings total.** State vs the 14:41 audit: **the 5-card deck now renders correctly** (Mariah / Peta / Priya / Jamal / Nori) — the F1 roster fix from #2367 landed and the canonical SAMPLE_PERSONA_IDS now matches the runbook. **Audit drill-in empty-state from #2366 also landed** — the SPA now shows a graceful "decision chain unavailable" message instead of 8 rows of empty stage names. **However the two structural Scene-2 blockers persist unchanged**: there is no Schedules tab on the worker profile (tab strip is still overview/activity/memory/skills/trust/escalation/settings), there is no Run-now affordance anywhere on /workers/{id}, the duty toggle is still a no-op stub that console.warns `[E4-b] duty 'd-list' toggled to false — mutation lands with E4-h`, and clicking it produces zero HTTP traffic. The Scene-2 fallback path (Run-now on /workflows/{vertical}/{name}) is **also dead** because the live gateway has **zero workflows published across all 6 verticals** (gtm-engineer/people-ops/support/sales/finance/marketplace all return `{count: 0, items: []}`), so the Run-now button is rendered DISABLED with tooltip "Available once this workflow is published to the gateway." So between the worker profile and the workflow detail page, there is currently **no SPA-driven path to fire a real agent turn against the gateway.**

**The one-line summary for the operator:** Scene 1 is now camera-ready (5 cards, correct roster, traits chips). Scene 2 still has nothing to fire — no Schedules tab, no Run-now on the worker profile, and the fallback /workflows Run-now button is disabled because the gateway has zero workflow definitions. To unblock Scene 2 you need either (a) publish a workflow definition for one of the demo verticals so /workflows Run-now lights up, then rewrite the runbook to navigate /workflows instead of clicking from /workers/{id}/schedules, OR (b) ship the Schedules tab + Run-now on the worker profile and wire it to `executeWorkflow`. Path (a) is one CLI command + a one-paragraph runbook edit; path (b) is several days of UI + wiring.

---

## Headline blockers (top 3 P0s)

| # | Severity | Finding | Scene impact |
|---|---|---|---|
| F1 | P0 | No Schedules tab; no Run-now button anywhere on /workers/{id}; duty toggle is a no-op stub (`mutation lands with E4-h`). | Scene 2 step 1 "click Schedules" impossible; step 2 "Run-now" impossible from the worker profile. |
| F2 | P0 | Gateway has **zero workflows** across all 6 verticals → /workflows Run-now button is DISABLED with tooltip "Available once this workflow is published to the gateway." | Scene 2 fallback path also dead. No SPA path to fire a real agent turn. |
| F3 | P0 | Audit feed still has only `audit.detail.viewed` HTTP rows with `worker = {id:"", name:"unknown"}`; drill-in returns 0 decision-chain stages. | Claim 2 "delegation is the unit" still has no provenance for the camera to drill into. |

---

## All findings

### F1 [P0] — No Schedules tab + no Run-now affordance + duty toggle is a stub (unchanged from prior audits)

`WORKER_TAB_IDS` at `web/components/skycot/WorkerTabs.tsx:19-27` is still:
```
overview · activity · memory · skills · trust · escalation · settings
```
A `DutiesCard` titled "Responsibilities" renders inside Overview (`web/components/skycot/worker/WorkerOverviewContent.tsx:151-159`). On Mariah's `/overview` the card surfaces 4 duties (List new inventory, Re-price stale listings, Respond to buyer questions, Flag items dropping below floor price) with per-row toggles labeled aria-label="Disable schedule" — **UI vocab is correctly "schedule" per the operator-locked vocabulary memory.** Per-row affordances:
- toggle (aria "Disable schedule", testid `duty-toggle-d-list`)
- click-to-edit (testid `duty-edit-d-list`)
- "+ Add responsibility" footer button (testid `duties-add`)

**No "Run now" button anywhere on the worker profile.** `rg 'Run now' web/app/(app)/workers/ web/components/skycot/worker/` → zero matches.

Worse, the existing toggle is a console.warn stub. I clicked `duty-toggle-d-list` in the live preview; console captured:
```
[E4-b] duty 'd-list' toggled to false — mutation lands with E4-h
```
Network panel showed zero new POST/PUT requests after the click — only the static GET 404 chain to `/api/v1/workers/<sample-id>` (the SPA falls back to mock data when the gateway has no matching worker).

The `WorkerOverviewContent.tsx:74-92` callbacks are still explicit stubs:
```ts
const onAddDuty = useCallback(() => setAnnouncement("Add responsibility dialog ships in E4-h"), []);
const onEditDuty = useCallback(() => setAnnouncement("Edit schedule ships in E4-h"), []);
const onToggleDuty = useCallback((dutyId, enabled) => {
  console.warn(`[E4-b] duty '${dutyId}' toggled to ${enabled} — mutation lands with E4-h`);
}, []);
```

Reproduction:
- Open http://localhost:3000/workers/018f67d0-0000-7000-8000-000000000001/overview (Mariah)
- Inspect tab strip — 7 chips, no Schedules
- Click `[data-testid="duty-toggle-d-list"]` — console shows `[E4-b] duty 'd-list' toggled to false`; no network call
- `rg 'Run now' web/app/(app)/workers/ web/components/skycot/worker/` returns nothing

Suggested fix: either (a) re-author the Scene 2 talk-track to navigate /workflows/{vertical}/{name} and Run-now from there (cheapest), OR (b) ship a `schedules` tab with a `Run now` button per duty wired to `executeWorkflow`. Path (a) is currently blocked by F2 below.

**Files:** `web/components/skycot/WorkerTabs.tsx:19-39`; `web/components/skycot/worker/WorkerOverviewContent.tsx:74-92`; `web/components/skycot/worker/DutiesCard.tsx`.

---

### F2 [P0] — Gateway has zero workflows; the only Run-now button in the SPA is disabled (NEW since prior audit)

The fallback Scene-2 path is /workflows/{vertical}/{name}. That page has a real Run-now button wired to `executeWorkflow` → `POST /api/v1/verticals/{vid}/workflows/{name}/execute` (`web/lib/api/workflow-execute.ts:34-44`, called from `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:159-174`). The wiring is correct.

But the button is rendered DISABLED with tooltip "Available once this workflow is published to the gateway":
```
runBtnDisabled: true
runBtnTitle: "Available once this workflow is published to the gateway"
```
Confirmed via preview snapshot at `/workflows/gtm-engineer/post_listing`.

The cause: `curl -H "X-Argo-Principal: human:demo@acmeco.com" -H "Authorization: Bearer <tok>" http://127.0.0.1:7400/api/v1/verticals/<v>/workflows` returns `{"count": 0, "items": []}` for every demo vertical:
- gtm-engineer → 0
- people-ops → 0
- support → 0
- sales → 0
- finance → 0
- marketplace → 0

The /workflows page renders from a static mock fixture (so the cards exist visually), but the disable-flag `isLive` is computed from a gateway lookup that returns nothing, so the button stays grey.

Reproduction:
- Open http://localhost:3000/workflows/gtm-engineer/post_listing
- Inspect `[data-testid="wfd-run-workflow"]` — `disabled=true`, hover tooltip shows the message
- `curl -s -H "X-Argo-Principal: human:demo@acmeco.com" -H "Authorization: Bearer <session-token>" http://127.0.0.1:7400/api/v1/verticals/gtm-engineer/workflows` → empty items

Suggested fix: run the workflow seed command on the gateway (look at `crates/argo-demo-seed` for a Pass-C step that registers a `workflow_definition` per persona). Even one published workflow per demo vertical unblocks Scene 2 + Scene 5.

**Files:** gateway side — `crates/argo-demo-seed/`; SPA side — already correctly gated, no change needed.

---

### F3 [P0] — Audit feed: only HTTP meta-events; drill-in returns 0 decision-chain stages (mostly unchanged; partially mitigated by #2366)

Updated state vs prior audit: gateway now returns **0 decision-chain stages** instead of 8 empty-bodied ones (good — less footgun). The SPA renders an explicit empty-state via `DecisionChainCard` (`web/components/skycot/audit/DecisionChainCard.tsx:44-49`):
> "Decision chain unavailable — This row records a meta-event, not an agent turn. Drill into an agent-attributed row to see the full chain."

This is the #2366 fix landing. The SPA copy is honest and the empty-state is graceful. **But the underlying problem is unchanged:** there are still **no agent-attributed rows in the audit feed**. Every row is `action_type: audit.detail.viewed`, `worker: {id:"", name:"unknown"}`, `channel: web`, `verb: http.request`. So if Scene 2 lands on /audit and clicks any row, the camera reads "Decision chain unavailable" — better than 8 rows of empty templates, but still doesn't prove the Claim 2 talk-track.

`provenance.signature` is null, `provenance.model.id` is null, `provenance.skill.id` is null — there's literally no agent turn anywhere in the gateway. This is downstream of F1+F2: no path fires a turn, so no audit row exists to drill into.

Reproduction:
- `curl -H "X-Argo-Principal: human:demo@acmeco.com" -H "Authorization: Bearer <tok>" 'http://127.0.0.1:7400/api/v1/audit?limit=5'` → all 5 rows are `audit.detail.viewed`
- Drill-in: `curl -H ... http://127.0.0.1:7400/api/v1/audit/<id>` → `decision_chain: []`, `provenance.signature: null`

Suggested fix: same root cause as F2 — publish workflow definitions + fire one execution per persona at seed time so there's at least one agent-attributed audit row with a populated decision chain for the camera to drill into.

**Files:** gateway side — `crates/argo-demo-seed/`; SPA side — already correctly handling empty state.

---

### F4 [P1] — Peta is missing from `TRAITS_BY_NAME` + `LEVEL_BY_NAME` + `PERSONALITIES` despite being in the canonical roster (regression introduced by partial #2367 fix)

This is a sibling-table problem the prior audit flagged. #2367 added Peta to `SAMPLE_PERSONAS` and `SAMPLE_PERSONA_IDS` so the deck card renders her, but the three personality-lookup tables it depends on were not updated in the same PR. Verified live:

**On `/team` (the deck card):**
- Mariah card shows TRAITS section (Patient / Detail-oriented / Picture-perfect listings / Polite)
- Priya card shows TRAITS section (Thorough / Respectful / Fast / Calibrated)
- Jamal card shows TRAITS section (Calm under pressure / Empathic / Quick / Honest)
- Nori card shows TRAITS section (Creative / On-brand / Concise / Curious)
- **Peta card has NO TRAITS section at all** — the entire chip strip is omitted because `TRAITS_BY_NAME[worker.name] ?? []` returns empty
- Peta's level renders as `18` from `hashLevel("Peta")` instead of a curated number (other workers have hand-tuned levels 5, 9, 12, 15)

**On `/workers/018f67d0-0000-7000-8000-000000000007/overview` (Peta's detail page):**
- Personality card header reads "HOW PETA WORKS" then shows generic chips: **Reliable / Curious / Helpful** (from `DEFAULT_PERSONALITY` at `web/components/skycot/worker/WorkerPersonalityCard.tsx:98`)
- "Great at" defaults to "Working through the responsibilities you set / Asking when something is ambiguous / Surfacing patterns across recent runs"
- "Signature behaviors" defaults to "Always confirms before a public or irreversible action"
- Peta's *actual* traits (Cadence-true / Follow-through / Calm escalator / Context keeper) are defined at `web/lib/sample-personas.ts:136` but never consumed by `WorkerPersonalityCard` or `TeammateCard`

Camera impact: on Scene 1, the Peta card is visibly less rich than the other 4 — the trait-chip strip is the most distinctive C2-holographic ornament and Peta is the only worker missing it. On Scene 2 if the runbook clicks into Peta, the camera reads bland generic boilerplate that doesn't match her quoted personality on the deck.

Reproduction:
- Open http://localhost:3000/team — visually compare Peta's card to the other 4
- `rg 'Peta' web/components/skycot/TeammateCard.tsx web/components/skycot/worker/WorkerPersonalityCard.tsx` → zero matches
- Open http://localhost:3000/workers/018f67d0-0000-7000-8000-000000000007/overview — body text contains "Reliable / Curious / Helpful" not "Cadence-true / Follow-through"

Suggested fix: add Peta to all three maps. Better: refactor all three to consume `findSamplePersona(workerId).traits` from `web/lib/sample-personas.ts` so there's one source of truth instead of four parallel name-keyed lookup tables. The sibling-table sprawl is the bug pattern that produced this regression in the first place.

**Files:** `web/components/skycot/TeammateCard.tsx:51-67`; `web/components/skycot/worker/WorkerPersonalityCard.tsx:35-111`.

---

### F5 [P1] — Duplicate nav entries "Team" + "Workers" both resolve to the same surface (unchanged)

`web/components/skycot/nav-items.tsx:22-30` still ships both `/team` and `/workers` in `NAV_ITEMS`. `/workers` is a redirect (`web/app/(app)/workers/page.tsx:25-27`: `if (hasWorkers) router.replace("/team")`). On the recorded demo, the viewer sees two nav items stacked: "Team · Workers" — implies two distinct screens. Click either, land on /team. Looks like an unfinished feature.

Reproduction: load the SPA, observe left nav. Click "Workers" — URL flips to /team after the spinner beat (see F8).

Suggested fix: drop `/workers` from `NAV_ITEMS` (keep the route accessible for empty-tenant first-touch via the onboarding flow). Or invert and make /workers canonical.

**Files:** `web/components/skycot/nav-items.tsx:22-30`.

---

### F6 [P1] — Demo banner copy ("showing sample data, sign in to see your own workspace") visible at top of /team during the recorded session (unchanged)

Per the previous audit. Banner renders at the top of /team via `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`. Reads:
> "Demo mode — showing sample data. Sign in to see your own workspace."

Has a "Dismiss demo banner" close button (testid `demo-banner-dismiss`). The Scene-1 narration ("here are your five teammates") fights with banner copy ("actually it's sample data") on camera.

Reproduction: load /team in any session with demo-mode on; observe top-of-page banner.

Suggested fix: pre-record operator step "dismiss banner first" + remember the dismiss across the session (currently re-renders on reload). OR auto-hide after 2s in demo-mode. OR replace full-width banner with a tiny corner pill so the camera doesn't read it.

**Files:** find `<DemoBanner>` component (referenced in `web/lib/skycot/demo-mode.ts`).

---

### F7 [P1] — Spinner-then-redirect flash on /workers (unchanged)

`web/app/(app)/workers/page.tsx:25-35` renders a Loader2 spinner first, then the `useEffect` checks team-data, then `router.replace("/team")`. Clicking the Workers nav item shows a brief spinner for 200-800ms before the URL flips. On a recorded demo it looks like a slow page rather than a deliberate redirect.

Suggested fix: server-side redirect via `redirect('/team')` from a server component, or middleware rewrite. Removes the spinner.

**Files:** `web/app/(app)/workers/page.tsx:19-35`.

---

### F8 [P2] — Worker detail page title bar shows the UUID, not the worker name (polish)

Browser tab title for `/workers/018f67d0-0000-7000-8000-000000000001/overview` is:
> "018f67d0-0000-7000-8000-000000000001 · Overview"

Should read "Mariah · Overview". The H1 in-page is correct ("Mariah"); only the document.title is broken. Recording a screencast with browser chrome visible (or showing the tab list) will expose the UUID.

Reproduction: navigate to any worker overview; check `document.title` or the browser tab.

Suggested fix: in the worker layout, set `metadata.title` from the resolved worker name instead of the URL parameter.

**Files:** `web/app/(app)/workers/[workerId]/layout.tsx`.

---

### F9 [P2] — Worker pronouns are inconsistent — Peta's overview narration calls her "she" twice (could be brand-policy decision)

Peta's overview body text at `/workers/018f67d0-0000-7000-8000-000000000007/overview` contains:
- "What she does"
- "Peta hasn't picked up Slack or email yet."
- "Connect a channel so Peta can respond where your team already works."

The "she" pronoun appears in the personality card header ("Always confirms before a public or irreversible action") + in the card section title ("What she does"). The other personas may have similar gendered copy with different pronouns. On a recorded demo + an investor audience, this risks reading as "we hardcoded gender per name" rather than "the operator picks pronouns at hire time." Worth a brand-policy decision.

Reproduction: open `/workers/018f67d0-0000-7000-8000-000000000007/overview`, search for "she".

Suggested fix: (a) settle on "they" for all sample personas (lowest-friction inclusive default), OR (b) ship a pronoun field on `SamplePersona` so it's explicit + per-persona, OR (c) accept gendered copy as part of the persona design. Operator decision.

**Files:** `web/components/skycot/worker/WhatSheDoesCard.tsx` (likely component name leaks the bias); persona copy throughout `web/lib/sample-worker-overview.ts`.

---

### F10 [P2] — Preview MCP browser keeps redirecting to /onboarding/welcome / /workflows during a /workers navigation (tooling note, not a SKYCOT bug)

During this audit, navigations to `/team` and `/workers/{id}/overview` repeatedly bounced to `/onboarding/welcome` or `/workflows` mid-test. Cause: the long-lived dev server on :3000 plus the preview-MCP's own short-lived Chromium each issue independent navigations on the same origin, and the preview's hot-reload race makes them collide. Not a product bug — it's the same harness flake as F12 of the 14:41 audit.

Workaround: stop the long-lived dev server before invoking preview-MCP, or use `?stay=1` query-string trick so navigations don't dedupe via URL.

---

## Resolved since prior audit (2026-05-26-1441)

Cross-checked against the prior baseline's 13 findings:

- **F1 (1 worker "Maya"; SAMPLE_PERSONA_IDS ships Sam/Rio) — RESOLVED via #2367.** Five-card deck now renders Mariah / Peta / Priya / Jamal / Nori. SAMPLE_PERSONA_IDS at `web/lib/sample-personas.ts:148-154` is correct.
- **F4 (audit drill-in returns 8 empty-bodied stages) — RESOLVED via #2366.** Now returns 0 stages + the SPA renders a graceful "Decision chain unavailable" empty state.
- **F7 (`NEXT_PUBLIC_GATEWAY_AUTH_BYPASS` doesn't match `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`) — STILL TRUE but no longer load-bearing.** The mock-bypass path is dead, but the mock-offline path catches the 404 and serves SAMPLE_TEAM anyway. Renamed in launch.json would be cleaner but optional.
- **F11 (Maya's permissions all false, FreezeButton greyed) — PARTIALLY RESOLVED via #2403.** `FreezeButton.tsx:54` now defaults to `true` when permissions are undefined, so the mock-path (where permissions arrive null) shows the button enabled. Maya the live worker still has all permissions false, but Scene 1 doesn't navigate to Maya in the runbook.
- **F2 (no Schedules tab + no Run-now) — UNCHANGED.** Still the load-bearing Scene-2 blocker; promoted to F1 of this report.
- **F3 (Maya zero duties + locked permissions) — UNCHANGED but irrelevant to Scene 1/2 because the runbook narrates the 5 demo personas, not Maya.** Mariah's duties render from `SAMPLE_WORKER_OVERVIEW` fixture with 4 populated entries.
- **F6 (Peta missing from PERSONALITIES + TRAITS_BY_NAME + LEVEL_BY_NAME) — UNCHANGED.** Renumbered as F4 in this report; with the deck-card now visibly rendering 5 cards, the missing-Peta-traits bug is more glaring than before.

No regressions introduced this tick.

---

## Orientation score

**Scenes 1 + 2 ship-readiness: 52 / 100.** (Up from 38/100 at 14:41.)

Scene 1 is now camera-ready: the 5-card deck shows the canonical roster with personality chips on 4 of 5 cards. The one Peta-traits-missing regression (F4) is a 10-line PR — add Peta to three lookup tables (or refactor them to consume sample-personas.ts as single source of truth).

Scene 2 is still not recordable. Two independent paths are broken:
1. The worker profile has no Schedules tab + no Run-now button + the duty toggle is a no-op stub (F1)
2. The fallback /workflows Run-now button is disabled because the gateway has zero workflows published (F2)

Until at least one of those paths is fixed, there is **no SPA-driven way to fire a real agent turn** against the gateway. Without a turn, F3 (audit drill-in has no real chain) stays. Claim 2 ("delegation is the unit") is wired in the runtime (per #2256, `current_handoff_id` is plumbed) but **the SPA has no surface that triggers a delegated_task fanout** that a viewer could see.

**Recommended sequence to unblock Scene 2:**
1. Run the workflow-publish step of the demo seed (or a one-shot `argo workflow publish` CLI) to register at least one `workflow_definition` per demo vertical → /workflows Run-now lights up + Scene-2 narration becomes "click into /workflows, pick Mariah's daily re-price, click Run now" — that's all that needs to change in the runbook
2. Rewrite Scene-2 talk-track to navigate to /workflows instead of /workers/{id}/schedules
3. Verify the resulting audit row has populated `decision_chain`, `provenance.skill.id`, `provenance.signature`, AND ideally `delegated_by` / `handoff_id` to prove Claim 2 in the same camera move

Step (1) is one CLI command + a check. Steps (2)+(3) are runbook edits. The deeper fix (ship a real Schedules tab + Run-now on /workers/{id}) is a separate engineering project.

---

## Answers to the critical questions

**Q1 — Claim 1 LIVE: does the worker feel like a teammate with persistent identity, not a tool with a name?**
**YES, on Scene 1.** The 5-card deck reads as people: each card has personality (traits chips, level number, today/tomorrow narrative bullets), each card name links to a detail page with a charter ("Mariah helps the Procurement team..."), responsibilities, channels, KPIs, recent activity. The C2-holographic-showcase locked design is implemented (gradient banners, weight-900 level numbers, large initials, trait chips). **Caveat: Peta's card and detail page degrade to generic boilerplate (F4)** — she's the only one of the five who reads as a placeholder rather than a colleague, because the trait/personality lookup tables don't know about her. Fix is small.

**Q2 — Claim 2 LIVE: does delegation actually invoke `delegate-task` with fallback-chain inheritance (PR #2256), or is the UI button a sonner-toast stub?**
**NEITHER in a Scene-2-recordable way.** The runtime IS wired (#2256 merged; `ToolContext.current_handoff_id` is plumbed; `delegate_task_inherits_active_inbound_handoff_chain` test exists at `crates/argo-agent-runner/tests/delegate_task_handoff_chain_runtime.rs:155`). The SPA's workflow-execute path (`web/lib/api/workflow-execute.ts → POST /api/v1/verticals/{vid}/workflows/{name}/execute`) is correctly wired to the gateway. **But there is no live trigger surface that produces a delegation chain:** the worker-profile Run-now button does not exist (F1), the workflow-page Run-now button is disabled because the gateway has zero workflows (F2), and the duty toggle is a console.warn stub (F1). So while Claim 2 is "LIVE" per CLAUDE.md's backend-cold-test definition, it is **not demonstrable through the SPA** in the camera's view. The first thing that would land in the audit feed if you DID get Run-now to fire would be a `WorkflowExecuteDispatched` row; until then the audit feed is HTTP meta-events only.
