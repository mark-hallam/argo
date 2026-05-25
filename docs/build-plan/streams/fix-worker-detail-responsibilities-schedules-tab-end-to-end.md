# fix-worker-detail-responsibilities-schedules-tab-end-to-end

**Owner**: claude.
**Authored against**: origin/main `0653c478d` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-worker-detail-responsibilities-schedules-tab-end-to-end`.
**Source**: e2e-ux-audit Agent 2 findings F9 + F10 + F11 (`docs/skycot/ux-audits/2026-05-25-1100-scene-1-and-2-claim-1-and-2.md`).

🔴 **P0 DEMO-GATE — Scene 2 centerpiece interaction has no UI. Scene cannot start.**

## Root cause

Three coupled defects, single feature:

1. **`WORKER_TAB_IDS` omits "responsibilities" (label "Schedules") and "evals".** `web/components/skycot/WorkerTabs.tsx:19-27` contains only `[overview, activity, memory, skills, trust, escalation, settings]`. The runbook narrates 9 tabs (these 7 + `evals` + `responsibilities`). Neither extra tab is wired.

2. **`/workers/[workerId]/responsibilities/page.tsx` does not exist.** The directory contains only `new/page.tsx` (the create form). Even if the tab is added, clicking it falls through to the catch-all `[tab]/page.tsx` and lands in `notFound()`.

3. **No "Run now" button anywhere in the worker UI.** `rg "Run now|RunNow|run_now"` across `web/components/skycot/worker/` returns zero matches. The only related API surface is `lib/api/orchestration.ts` (run cancel/reset, not duty trigger). Scene 2's "Click Run now on listing_health_check" interaction has no UI implementation.

Note: per memory `project_vocabulary_duty_vs_schedule` (operator-locked 2026-05-23), the user-facing label is **"Schedule"** (or "Schedules" plural) — the directory + code stay `duty` / `responsibilities`, but every visible label says "Schedules".

## Findings this brief closes

- Agent 2 F9 [P0] — Responsibilities tab is missing from the worker-detail tab list
- Agent 2 F10 [P0] — `/workers/[workerId]/responsibilities/page.tsx` does not exist
- Agent 2 F11 [P0] — No "Run now" button anywhere in the worker UI

## Fix recipe

This is a single coordinated feature. The data shape `duties: []` is already returned by the gateway on `/api/v1/workers/<id>/overview`.

1. **Add "responsibilities" + "evals" to `WORKER_TAB_IDS`** at `web/components/skycot/WorkerTabs.tsx:19-39`:
   ```ts
   export const WORKER_TAB_IDS = [
     "overview", "activity", "responsibilities", "memory",
     "skills", "trust", "evals", "escalation", "settings",
   ] as const;
   ```
   Labels:
   ```ts
   responsibilities: "Schedules",  // per project_vocabulary_duty_vs_schedule
   evals: "Evals",
   ```

2. **Author `web/app/(app)/workers/[workerId]/responsibilities/page.tsx`**:
   - Uses `useWorkerOverview(workerId)` and renders the `duties: []` array as a list of cron-shaped rows.
   - Each row shows: name (e.g. `listing_health_check`), cron expression, last-run timestamp, last-run result pill, **a "Run now" button**, an "Edit" affordance.
   - Empty state: "No schedules yet. Add one." with the existing `+ New responsibility` CTA pointed at `responsibilities/new/page.tsx`.

3. **Implement Run now end-to-end**:
   - Gateway: add `POST /api/v1/workers/<id>/duties/<duty-id>/run` (returns the new audit row id immediately, writes a real audit row, triggers the duty asynchronously).
   - SPA: add `runDutyNow(workerId, dutyId)` to `web/lib/api/duties.ts` (already exists per the directory listing).
   - Wire the "Run now" button to the API call, show a brief toast "Running…", then on success show "Started — see Activity tab" with a deep-link to the new audit row.

4. **Author `web/app/(app)/workers/[workerId]/evals/page.tsx`** (lightweight — the runbook narrates evals but Scene 2's centerpiece is Schedules; can be a thin placeholder showing "No eval cases yet" + CTA if the data isn't wired).

## Acceptance criteria

- [ ] `/workers/<id>/responsibilities` (404-free) renders a list of the worker's scheduled duties, labelled "Schedules" in the tab strip.
- [ ] Each duty row has a visible "Run now" button.
- [ ] Clicking "Run now" hits `POST /api/v1/workers/<id>/duties/<duty-id>/run` and writes a real audit row.
- [ ] Within ~5s, the new audit row is visible at `/audit` with the real `worker_id` and the duty name as `verb`.
- [ ] Tab strip shows 9 tabs in the runbook's documented order.
- [ ] `/workers/<id>/evals` does not 404 (even if it's a thin placeholder).
- [ ] Curl-test: `POST /api/v1/workers/<id>/duties/<duty-id>/run` returns 200 + new audit row id.

## May-touch

- `web/components/skycot/WorkerTabs.tsx` (tab IDs + labels)
- `web/app/(app)/workers/[workerId]/responsibilities/page.tsx` (NEW)
- `web/app/(app)/workers/[workerId]/evals/page.tsx` (NEW, lightweight)
- `web/lib/api/duties.ts` (add `runDutyNow`)
- `crates/argo-cli/src/handlers/gateway/routes_duties*.rs` (new run-now endpoint)
- `crates/argo-duty/src/scheduler.rs` (manual-trigger entry point)

## Out of scope

- Editing duties (the existing `responsibilities/new/page.tsx` covers create)
- Per-duty pause/resume (follow-up)
- Vocabulary rename of internal `duty` symbols (per memory `project_vocabulary_duty_vs_schedule`, code stays `duty`)

## Effort

~250-450 LOC (gateway endpoint + SPA page + tab wiring + tests). ~3-4 hours.
