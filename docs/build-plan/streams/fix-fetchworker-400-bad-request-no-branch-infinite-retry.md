# fix-fetchworker-400-bad-request-no-branch-infinite-retry

**Owner**: claude.
**Authored against**: origin/main `4b3aa443f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-fetchworker-400-bad-request-no-branch-infinite-retry`.
**Source**: UX-AUDIT-T0519 2026-05-27 — `docs/skycot/ux-audits/2026-05-27-0519/onboarding-hire-an-ai-teammate.md` F1 + F10.

🔴 **P0 DEMO-GATE — `fetchWorker` only translates HTTP 404 into `WorkerNotFoundError`. When the gateway returns HTTP 400 (e.g. a non-UUID synthetic worker id from the demo wizard) the raw ApiError bubbles up, React Query retries on its default exponential backoff (5 attempts ~15s total), and `WorkerLayoutShell` stays in `isPending=true` the whole time. The camera sees "Loading worker…" forever, then a friendly error after ~15s.**

## Root cause

`web/lib/api/workers.ts:119-152` — `fetchWorker(workerId)` calls `apiGet('/api/v1/workers/<id>')`. The catch block only checks for HTTP 404 and converts to `WorkerNotFoundError`. Any other status (400, 403, 500, network) re-throws as a raw `ApiError`.

`web/lib/hooks/use-worker.ts` (likely defaults via `useQuery`) — React Query's default `retry: 3` (sometimes 5) with exponential backoff hammers the same broken request multiple times before surfacing.

`web/app/(app)/workers/[workerId]/WorkerLayoutShell.tsx:48` — render-time gate `isPending` traps the spinner.

Trigger path:
1. Wizard's `Hire them` button mints `demo-worker-<verticalId>-<base36-stamp>` (see `web/lib/onboarding/use-first-hire.ts:102-112`)
2. Router pushes to `/workers/demo-worker-marketing-intern-mpn1b10z`
3. Gateway rejects: `HTTP 400 {"error":"bad_request","reason":"invalid worker_id 'demo-worker-marketing-intern-mpn1b10z': invalid character: found 'm' at 3"}`
4. `fetchWorker` re-throws, React Query retries, shell stays `isPending`, camera sees spinner forever (≥15s)

Even after the sibling brief `fix-onboarding-demo-mode-fake-validation-and-synthetic-worker-redirect` flips first-hire to a real UUID, the FE robustness defect remains: any caller that arrives at `/workers/<bad-id>` (deep-link, bookmark, typo) hits the same infinite spinner.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F1 [P0] — Post-hire `/workers/{synthetic-id}` hangs on "Loading worker…" forever
- onboarding-hire-an-ai-teammate F10 [P1] — `WorkerOverviewContent` retries indefinitely on 400 (~15s spinner before friendly error)

## Fix recipe

1. **Extend `fetchWorker` 4xx translation** at `web/lib/api/workers.ts:147-149`:
   - Catch HTTP 400 (`bad_request` / `invalid worker_id`) AND HTTP 404 → throw `WorkerNotFoundError`.
   - Catch HTTP 403 → throw a distinct `WorkerForbiddenError` (renders a different shell).
   - All other statuses still re-throw as `ApiError`.

2. **Disable retries on 4xx** in the React Query hook (`web/lib/hooks/use-worker.ts`):
   - `retry: (failureCount, error) => { if (error instanceof WorkerNotFoundError || error instanceof WorkerForbiddenError) return false; return failureCount < 2; }`
   - This stops the ~15s hammer on a known-bad id.

3. **WorkerLayoutShell renders `WorkerNotFound` on 400** the same way it already does on 404 (`web/app/(app)/workers/[workerId]/WorkerLayoutShell.tsx:52-60`) — no change required if step 1 lands; the `WorkerNotFoundError` thrown by `fetchWorker` flows through the existing `catch`.

4. **Vitest pin** at `web/lib/api/__tests__/workers.test.ts`:
   - `fetchWorker → translates HTTP 400 to WorkerNotFoundError`
   - `fetchWorker → translates HTTP 404 to WorkerNotFoundError`
   - `useWorker → does not retry on WorkerNotFoundError`

## Acceptance criteria

- [ ] Visiting `/workers/demo-worker-anything` renders the friendly `<WorkerNotFound>` shell in <1s (no spinner-then-retry-then-error sequence).
- [ ] `fetchWorker` translates HTTP 400 to `WorkerNotFoundError` (same class as 404).
- [ ] `useWorker` does NOT retry on `WorkerNotFoundError`.
- [ ] Vitest specs above pass.
- [ ] Manual smoke: paste a typo'd URL into the browser, expect the friendly shell, not a 15s spinner.

## May-touch

- `web/lib/api/workers.ts` (extend 4xx branch)
- `web/lib/hooks/use-worker.ts` (retry policy)
- `web/lib/api/__tests__/workers.test.ts` (NEW)

## Out of scope

- The wizard's synthetic id mint (covered by `fix-onboarding-demo-mode-fake-validation-and-synthetic-worker-redirect`)
- WorkerForbiddenError shell design (file the follow-up brief if needed)
- Gateway-side error envelope normalization (gateway already returns `{error,reason}` cleanly)

## Effort

S — ~30-50 LOC + 3 vitest cases. ~30-60 min.
