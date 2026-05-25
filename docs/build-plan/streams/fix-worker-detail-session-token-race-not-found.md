# fix-worker-detail-session-token-race-not-found

**Owner**: claude.
**Authored against**: origin/main `d15461a3a` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-worker-detail-session-token-race-not-found`.
**Source**: e2e-ux-audit Agent (scene-1-and-2-claim-1-and-2) finding F6 (`docs/skycot/ux-audits/2026-05-25-2309/scene-1-and-2-claim-1-and-2.md`).

🔴 **P0 — `/workers/<id>/overview` renders "Worker not found" against a real worker ID + a live gateway session, because the client query fires before the session token is hot. Scene 1 click-through is unrecordable.**

## Root cause

Reproduction (from the audit run):
- Same browser session can `GET /api/v1/workers/019e5dbb-c03d-79a0-a9a4-ee60bb3c8218` → 200 with Maya's full payload.
- But navigating `/workers/019e5dbb-c03d-79a0-a9a4-ee60bb3c8218/overview` renders `<WorkerNotFound>` (`web/components/skycot/WorkerNotFound.tsx`).
- Hard refresh after the React-Query cache warms resolves it. Scene 1's click-from-card flow does NOT have that pause.

Root cause is the interaction between server-side `fetchFreezeState` (silently returns `null` on missing token) and the client `useWorker(workerId)` query firing before the React-Query cache has a valid session token. The first request goes out unauthenticated → gateway returns 401 → SPA's fetcher (`web/lib/api/workers.ts:97-121`) coerces 401 to `WorkerNotFoundError` → layout (`web/app/(app)/workers/[workerId]/WorkerLayoutShell.tsx:43-53`) renders the not-found branch.

So "worker not found" is actually "your session wasn't ready yet". A buyer clicking a trading card mid-demo lands on Not Found instead of the C2-personality detail page.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F6 [P0] — Worker-detail page renders "Worker not found" against a live gateway worker due to session-token-race on first navigation

## Fix recipe

Two fixes, pick (a) for the smaller patch:

**(a) Treat HTTP 401 as "auth-not-ready", not "not found"**:
1. Touch site: `web/lib/api/workers.ts:97-121` — the `fetchWorker` function.
2. When the fetch returns 401, throw a distinct `AuthNotReadyError` (or return `Promise.reject(new Error("auth-not-ready"))`).
3. React-Query is configured to retry on this error 1-2 times with a short delay so the session has time to hydrate.
4. Only coerce to `WorkerNotFoundError` on 404 (the actual not-found case).
5. Update `WorkerLayoutShell.tsx:43-53` to keep showing the skeleton (not the not-found UI) while `AuthNotReadyError` is in flight.

**(b) Wait for the session peek before triggering the worker query**:
1. In the layout shell, await `useSession()` or `useSessionToken()` returning a truthy value before enabling `useWorker(workerId)`.
2. Render a skeleton while the session is unresolved.
3. Drop the 401-as-not-found coercion (no longer needed because the query never fires too early).

Either approach removes the race; (a) is more defensive (catches genuine session loss mid-session), (b) is cleaner (prevents the bad request entirely). Author picks based on the existing query-client config.

## Acceptance criteria

- [ ] Fresh navigation to `/workers/<real-id>/overview` (no warm cache) renders the C2-personality detail page within 1-2 seconds, NEVER the "Worker not found" shell.
- [ ] `/workers/<actually-bogus-id>/overview` still renders "Worker not found" within a reasonable time (404 path preserved).
- [ ] No new console errors on the happy path.
- [ ] Pin with a Playwright spec at `web/tests/worker-detail-cold-nav.spec.ts`: cold-load the route, assert C2 page renders, assert no "Worker not found" text.

## May-touch

- `web/lib/api/workers.ts`
- `web/app/(app)/workers/[workerId]/WorkerLayoutShell.tsx`
- `web/lib/hooks/use-worker.ts`
- `web/tests/worker-detail-cold-nav.spec.ts` (NEW)

## Out of scope

- The wider session/token race fixes for other queries (`useTeam`, `useAudit`, etc.) — could be follow-up
- The hardcoded `WorkerPersonalityCard` personalities (separate brief `fix-worker-personality-and-level-derive-from-real-data`)
- Worker page title showing UUID (separate brief `fix-worker-detail-page-title-and-server-client-tab-helper`)

## Effort

~50-120 LOC + 1 Playwright spec. ~1-2 hours. Demo-recording-blocker.
