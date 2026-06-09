# web-wire-schedules-panel-to-live-listduties

**Owner**: claude.
**Authored against**: origin/main `eddcab780` on `2026-06-09`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-wire-schedules-panel-to-live-listduties`.

## Why (NO-THEATER leak, VERIFIED on origin/main `eddcab780`)
The worker-detail **Schedules** tab renders FABRICATED schedule rows while a REAL backend list-GET shipped but is never consumed — a NO-THEATER leak the camera/operator sees (DEC-007 / DEC-029).
- `web/components/skycot/worker/WorkerSchedulesContent.tsx:87-88` renders `<DutiesCard duties={overview.duties} .../>`, where `overview` is `useWorkerOverview(...).data.overview`. The duty list is fabricated sample data sourced from `web/lib/sample-worker-overview.ts` (eight `duties: [...]` blocks, e.g. lines 73, 97, 121...).
- The live fetcher `listDuties` (`web/lib/api/duties.ts:429`, backed by `GET /api/v1/workers/<id>/duties`, persisted `role_duty` rows — NOT fixtures) shipped in #2742 with its NO-THEATER gate already built in (`mockFallbackAllowed()` = `!hasRealSession()`, `duties.ts:25-26`). But `rg -rln 'listDuties' web/components web/app` returns NOTHING — it was never wired to a component. BUILD_STATUS.md notes "Panel-wiring deferred."
- Net effect: a real signed-in / demo-live operator viewing the Schedules tab sees made-up schedules instead of the worker's actual persisted ones (or an honest empty state).
- **User-facing vocab is "Schedule", not "Duty"** (DEC-004 / vocabulary-duty-vs-schedule memory): UI copy stays "Schedule"; code/types/`role_duty`/`listDuties` keep "duty". This brief changes the DATA SOURCE only — no copy change.

## Grep-before-write
- `rg -n 'WorkerSchedulesContent' web/components web/app` — the panel + its route mount; confirm this is the only consumer to rewire.
- `rg -rln 'listDuties' web/components web/app web/lib` — proves the live fetcher has no component consumer today (only `duties.ts` + `duties.test.ts`).
- `rg -n 'export interface ListDutiesResult|export async function listDuties|function mockFallbackAllowed|hasRealSession' web/lib/api/duties.ts` — the live API surface + its existing gate; REUSE it, do not re-implement.
- `rg -n 'export function hasRealSession|export function isDemoLive|export function isGatewayAuthBypassed' web/lib/skycot/demo-mode.ts` — the canonical demo-mode gate helpers (note: it is `isGatewayAuthBypassed`, there is no `isAuthBypassed`). The duties module's `mockFallbackAllowed()` already wraps `!hasRealSession()`; consume that, do not introduce a parallel gate.
- `rg -n 'duties:' web/lib/sample-worker-overview.ts` — the fabricated source being displaced; keep it only as the anonymous-dev fallback.
- New surfaces: none. `rg 'pub (struct|enum|fn) listDuties'` / `rg 'export (function|const) useWorkerSchedules'` — confirm no parallel hook/fetcher exists before adding any thin wrapper hook.

## Fix (additive; names its consumer)
1. In `web/components/skycot/worker/WorkerSchedulesContent.tsx`, stop sourcing `duties` from `overview.duties`. Instead fetch via the live `listDuties(workerId, fallback)` (a small `useQuery` wrapping it, or a new `web/lib/hooks/use-worker-schedules.ts` thin hook that calls `listDuties`). Pass the worker's existing sample duties (from `sample-worker-overview.ts`, keyed by `workerId`) as the `fallback` callback so the **anonymous dev** (no session, not demo-live) path is byte-identical to today.
2. Rely on the fetcher's already-shipped gate: with a real session OR demo-live, `mockFallbackAllowed()` is false, so a 404 / malformed body throws an honest error (no sample leak) and a real empty rule set renders the **honest empty state** (`DutiesCard` already shows an empty branch at `DutiesCard.tsx:106` `duties.length === 0`). Do NOT weaken `mockFallbackAllowed`.
3. Keep the existing loading + error tabpanel branches (`worker-schedules-loading` / `worker-schedules-error` testids) keyed off the new query's `isPending` / `isError`. Keep `canToggleDuty={false}` (duty PATCH-toggle is a separate brief).
4. Do NOT change UI copy: the tab + card stay labelled "Schedule(s)"; only the data path changes. The consumer of `listDuties` is `WorkerSchedulesContent` (+ optional `use-worker-schedules` hook) — no other surface.

## Acceptance (truth-bounded)
1. Vitest (extend `web/components/skycot/worker/*.test.tsx` or add `WorkerSchedulesContent.test.tsx`): mock `@/lib/api/client` `apiGet` + `@/lib/skycot/demo-mode`. With `hasRealSession()` true (or demo-live), the panel renders LIVE gateway duties (assert a gateway-returned label appears, and the SAMPLE label does NOT). With a real session + empty gateway list, it renders the honest empty state (no sample rows). With anonymous dev (`hasRealSession()` false, not demo-live) + gateway 404, it falls back to the sample.
2. `rg -rln 'listDuties' web/components web/app` now returns the panel (the leak-closing wire exists).
3. `cd web && npx vitest run` green for the touched test file; touched files compile (`npx tsc --noEmit`); no file > 500 LOC.

## On ship (same commit)
- Flip the Schedules-panel-wiring row in `docs/build-plan/FEATURE_MATRIX.md` to ✅ Live with this commit SHA + `web/components/skycot/worker/WorkerSchedulesContent.tsx` (add the row if absent; do NOT regenerate `docs/skycot/feature-matrix.html` — it is GITIGNORED per DEC-043).
- Note in the commit body that it closes the NO-THEATER Schedules-tab sample-data leak (DEC-007/DEC-029) by consuming the #2742 `listDuties` read.

## Effort
S (one component rewired to an already-shipped fetcher + one vitest; data-path swap, no new backend, no copy change).
