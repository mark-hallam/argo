# fix-worker-activity-tab-poll-for-streaming

**Owner**: claude.
**Authored against**: origin/main `271a3d4cc2a90ad40949400340a0464a19f48d5f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-worker-activity-tab-poll-for-streaming`.
**Source**: UX-AUDIT-T0119MEL `docs/skycot/ux-audits/2026-05-27-0119/scene-1-and-2-claim-1-and-2.md` F8 (P1).

🟠 **P1 — Scene 2 step 3 of the v1.0 runbook says "the activity feed streams in real time: turn start → runner invocation → `WorkerDutyFired` audit row." But `web/lib/hooks/use-worker-activity.ts` has no `refetchInterval`, no SSE, no polling — it's a one-shot React Query. The camera will not see anything stream; rows arrive only on full page reload. Cheap polling (refetchInterval: 3000) passes the camera test and is enough for v1.0.**

## Root cause

`web/lib/hooks/use-worker-activity.ts` is a vanilla `useQuery` with no `refetchInterval` option and no subscription path. The Scene 2 narration ("the activity feed streams in real time") demands either polling or an SSE/WebSocket connection; neither is wired. The activity tab will look static during a recording, even if the gateway is emitting new audit rows in the background.

Cheapest fix that passes the camera test: add `refetchInterval: 3000` to the useQuery options. Cards arrive every 3 seconds; the narrator can say "there's the row" within the polling window without re-loading. SSE / WebSocket is the credible v1.x streaming path; polling is enough for the demo recording.

This compounds with scene-1-2 F3 (no Run-now to fire the duty) and F4 (no `worker_duty.fired` rows exist anyway) — but even when those are resolved, the activity tab won't *stream* without this fix.

## Findings this brief closes

- `docs/skycot/ux-audits/2026-05-27-0119/scene-1-and-2-claim-1-and-2.md` F8 [P1] — Worker activity tab has no live streaming; Scene 2 step 3 "streams in real time" unsupported

## Fix recipe

1. Edit `web/lib/hooks/use-worker-activity.ts` to add `refetchInterval: 3000` to the `useQuery` options. Also add `refetchIntervalInBackground: false` so the polling pauses when the tab is hidden.
2. Add a `staleTime: 1000` so the cache shows fresh data without an immediate refetch on tab focus.
3. (Optional) gate the polling behind a `live` boolean prop on the consumer page — e.g. activity tab opts in, other consumers (audit drill-in) don't. Default: on.
4. Verify no UX regression: open `/workers/<id>/activity`, watch the network tab — `/api/v1/workers/<id>/activity` fires every 3s. No double-rendered rows; the React Query cache dedupes by audit-row id.
5. Update the runbook narration if needed: the polling cadence is 3s, so "watch this — give it a second" is more accurate than "instantly." Worth a doc-only follow-up if narrators trip on the timing.

## Acceptance criteria

- [ ] `useWorkerActivity` polls every 3000 ms when the tab is visible.
- [ ] Polling pauses when the tab is hidden (no background network traffic).
- [ ] An audit row inserted into the gateway becomes visible on the activity tab within 5 seconds without a manual reload.
- [ ] No duplicate rows visible (the React Query cache dedupes by row id).
- [ ] Vitest covers the `refetchInterval` config.

## May-touch

- `web/lib/hooks/use-worker-activity.ts` (~10 LOC config additions)
- `web/lib/hooks/use-worker-activity.test.ts` (~30 LOC vitest)

## Out of scope

- Real SSE / WebSocket streaming (v1.x follow-up).
- The underlying audit-pipeline emission (covered by `fix-audit-feed-missing-skill-invoked-and-workflow-step-executed-rows`).
- The Run-now affordance to fire the duty in the first place (separate brief: a NEW worker-duty-row-run-now brief — see audit F3).

## Effort

XS — ~10 LOC config + test. ~30 minutes. P1 because Scene 2 of the v1.0 demo runbook narrates "real-time streaming" and the camera will catch the static activity tab. Polling at 3s is enough for the recording and matches the user's "watch this" cadence.
