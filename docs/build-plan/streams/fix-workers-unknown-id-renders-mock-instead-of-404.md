# fix-workers-unknown-id-renders-mock-instead-of-404

**Owner:** either
**Authored against:** origin/main `410e6ed1240d26f9ab6b48eb94a2fdf177c38270` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-workers-unknown-id-renders-mock-instead-of-404`
**Source:** ux-audit-2026-05-27-2351

🟢 **P2 — `/workers/<arbitrary-uuid>/overview` returns 200 with mock data in demo mode instead of 404. A confused investor poking at the URL bar gets a fabricated worker for any random UUID.**

## Root cause

> I probed `/workers/018f67d0-0000-7000-8000-000000000001/overview` (a fictional sample id) AND `/workers/019e6785-928a-7270-a766-f945b6cd7161/overview` (the real gateway id for Mariah). Both return HTTP 200. Combined with F01/F05, both also render the same SAMPLE Mariah overview content.
> — `scene-1-and-2-claim-1-and-2.md` F12

The SPA's worker-overview fetcher under demo / auth-bypass mode falls through to a DEFAULT mock for any id that doesn't match a sample-persona UUID, instead of throwing `WorkerNotFoundError`. The `WorkerNotFound.tsx` empty-state component exists at `web/components/skycot/` but isn't reached on the unknown-id path.

Specifically:
- `web/lib/api/worker-overview.ts:110-138` (fetchWorkerOverview)
- `web/lib/sample-personas.ts` exports `SAMPLE_PERSONA_IDS` — the canonical set
- The fetcher's auth-bypass branch should `throw new WorkerNotFoundError()` when the id isn't in `SAMPLE_PERSONA_IDS`, not fall back to a default fixture

This is a `feedback_no_theater_in_demo` violation: the SPA fabricates content for a URL the operator never seeded. Not on the recording path, but a curious viewer pasting a URL gets a mock.

## Findings this brief closes

- `scene-1-and-2-claim-1-and-2.md` F12 [P2] — `/workers/<unknown-id>/overview` returns 200 with mock data instead of 404

## Fix recipe

1. **Short-circuit the auth-bypass branch in `fetchWorkerOverview`**:
   - Import `SAMPLE_PERSONA_IDS` from `web/lib/sample-personas.ts`.
   - If `id` is not in the set AND `isAuthBypassed() && !isDemoLive()`, throw `WorkerNotFoundError`.
2. **Mirror in `fetchWorker`** at `web/lib/api/workers.ts:129-166` so the top-level worker fetch also 404s for unknown ids.
3. **Ensure the WorkerNotFound shell renders**:
   - `web/app/(app)/workers/[workerId]/overview/page.tsx` should already catch `WorkerNotFoundError` and render `WorkerNotFound.tsx`. Verify.
4. **Add a Playwright spec** `web/playwright/workers/unknown-id-404.spec.ts`:
   - Navigate to `/workers/00000000-0000-0000-0000-000000000000/overview`
   - Assert the `WorkerNotFound` empty state renders, not the mock Mariah card
5. **In live mode** (gateway-driven), the gateway already returns 404 for unknown ids; no change needed there.

## Acceptance criteria

- [ ] `/workers/<unknown-id>/overview` in demo mode renders the WorkerNotFound empty state, not a fabricated worker.
- [ ] Real sample-persona ids continue to render correctly.
- [ ] In `NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1` live mode, gateway-real worker ids resolve correctly.
- [ ] Playwright spec passes cold.
- [ ] PR body cites finding F12.

## May-touch

- `web/lib/api/worker-overview.ts`
- `web/lib/api/workers.ts`
- `web/lib/sample-personas.ts` (verify `SAMPLE_PERSONA_IDS` export)
- `web/components/skycot/WorkerNotFound.tsx` (verify renders, no change expected)
- `web/playwright/workers/unknown-id-404.spec.ts` (NEW)

## Out of scope

- Fix auth-bypass short-circuit pattern broadly (covered by `fix-spa-demo-live-mode-route-real-gateway-on-camera`)
- Worker overview activity row id mismatch (covered by `fix-worker-overview-activity-row-href-id-mismatch-with-audit-detail`)
- Worker page redirect (covered by `fix-workers-redirect-to-team-url-flicker`)

## Effort

S — ~30-50 LOC + 1 test. ~1-2 hours.
