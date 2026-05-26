# fix-onboarding-demo-mode-fake-validation-and-synthetic-worker-redirect

**Owner**: claude.
**Authored against**: origin/main `78f162a5` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-demo-mode-fake-validation-and-synthetic-worker-redirect`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2152/onboarding-hire-an-ai-teammate.md` F1 [P0] + F2 [P0].

🔴 **P0 DEMO-BLOCKER — Two stacked demo-mode short-circuits violate the operator-locked NO-THEATER rule:**
**(a)** Provider-key step in demo mode renders "Key validated · $0.001 test call confirms the key works" without any network round-trip — the camera sees a lie live on screen.
**(b)** First-hire in demo mode mints a synthetic `demo-worker-${verticalId}-${ts}` id and pushes to `/workers/{synthId}` which immediately renders the **WorkerNotFound** error shell because the synthetic id matches no gateway worker and no sample fallback. The headline beat of Scene 3 ends on an error page.

## Root cause

Two coupled client-side short-circuits land the operator at a not-found shell while pretending the flow succeeded:

1. **`web/lib/onboarding/use-provider-key.ts:36-42`** — `validateProviderKey()` in demo mode returns `{ validated: true, testCost: 0.001 }` synchronously without calling `/api/v1/onboarding/provider-key`. The connect-provider page at `web/app/(setup)/onboarding/connect-provider/page.tsx:138-141` advertises a `$0.001 test call` and renders a "Key validated" banner (line 188); the camera sees both promises fire on a fake.

2. **`web/lib/onboarding/use-first-hire.ts:102-122`** — `firstHire()` short-circuits in demo mode and returns `worker_id: demo-worker-${verticalId}-${ts.toString(36)}` with no gateway call. Step 5 then `router.push("/workers/" + synthId + "/overview")`. The destination layout at `web/app/(app)/workers/[workerId]/layout.tsx` runs `useWorker(synthId)` → `fetchWorker(synthId)` (`web/lib/api/workers.ts:119-152`); gateway 404 + sample fallback miss (sample worker ids in `web/lib/sample-worker-overview.ts:172-185` are canonical UUIDs not synthetic), raising `WorkerNotFoundError`. `WorkerLayoutShell.tsx:52-60` renders `<WorkerNotFound>` with "Worker not found · Back to team".

Per the function-vs-dressing rule both are FUNCTION-side because the camera drills directly into the validation banner (F2) and the post-hire destination (F1). Per `feedback_no_theater_in_demo` (operator-locked 2026-05-24) the demo path must be real backend, not mock-bypass.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F1 [P0] — Demo-mode hire short-circuit lands on Worker not found
- onboarding-hire-an-ai-teammate F2 [P0] — Provider-key validation is fake in demo mode

## Fix recipe

### Provider-key fix (F2)

1. Replace the demo-mode short-circuit in `web/lib/onboarding/use-provider-key.ts:36-42` with one of:
   - **(preferred)** Call `/api/v1/onboarding/provider-key` with `{ provider: "mock", api_key: input }` and have the gateway route at `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs` recognise demo-mode + return `{ validated: true, mock_provider: true, cost_usd: 0.0 }` after a realistic ~400ms delay. Surface `mock_provider: true` to the SPA.
   - **(fallback)** If the gateway path cannot land in time, change the demo-mode short-circuit to delay 400ms and set `mockProvider: true` in the result.
2. Update `web/app/(setup)/onboarding/connect-provider/page.tsx:138-141` + :186-191 so the hero copy AND the success banner read **"Mock provider connected — workers run with simulated responses for the demo"** whenever the result carries `mock_provider: true` (or whenever demo mode is on). Do not show `$0.001 test call` copy unless a real test call was made.

### First-hire fix (F1)

1. Change `web/lib/onboarding/use-first-hire.ts:102-122` to one of:
   - **(preferred — no theater)** Route through `/api/v1/onboarding/first-hire` even in demo mode, with the gateway honoring an `X-Argo-Demo-Mode: 1` header that triggers `demo-seed` to ensure a real demo worker exists in the tenant and returns its actual UUID. The redirect now lands on a real-rendered C2 card.
   - **(fallback)** Have `buildDemoFirstHireResult` return ONE of the canonical sample-worker UUIDs (Mariah `018f67d0-0000-7000-8000-000000000001` by default; vertical-keyed lookup if the operator picked a different vertical). The synthetic-id design is the root bug — drop it.
2. If neither lands in time, add a wildcard match to `findSampleWorker` + `findSampleWorkerOverview` at `web/lib/sample-worker-overview.ts:172-185` that, for any `demo-worker-` prefix id, returns a templated "Newly hired teammate" placeholder card so the destination at least renders.

### Tests

3. Add `web/playwright/onboarding/demo-mode-first-hire-lands-on-worker.spec.ts`:
   - Walk steps 1-5 in demo mode.
   - Click "Hire them" and assert the destination URL matches `/workers/[a-f0-9-]{36}/overview` (a real UUID), not `demo-worker-*`.
   - Assert the page body does NOT contain "Worker not found".
   - Assert it DOES contain the worker name as `<h1>`.
4. Add `web/playwright/onboarding/demo-mode-provider-key-honest-copy.spec.ts`:
   - On step 3 in demo mode, type `sk-ant-anything` and click validate.
   - Assert the success banner does NOT contain `$0.001` OR contains the string `Mock provider`.

## Acceptance criteria

- [ ] Provider-key step in demo mode displays mock-provider copy, not "$0.001 test call".
- [ ] First-hire in demo mode redirects to a real worker UUID (sample or seeded), not `demo-worker-*`.
- [ ] `/workers/{returnedId}/overview` renders a C2 personality card (not the WorkerNotFound shell).
- [ ] Two new Playwright specs pass.
- [ ] Cold rerun of claim-1 LIVE test still passes.

## May-touch

- `web/lib/onboarding/use-provider-key.ts` (demo-mode return shape)
- `web/lib/onboarding/use-first-hire.ts` (drop synthetic-id factory)
- `web/lib/sample-worker-overview.ts` (optional wildcard fallback)
- `web/app/(setup)/onboarding/connect-provider/page.tsx` (honest copy)
- `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs` (demo-mode honor on first-hire + provider-key — if going preferred path)
- Two new Playwright specs under `web/playwright/onboarding/`.

## Out of scope

- Genesis Studio multi-modal upload (covered by `fix-onboarding-welcome-step-genesis-studio-framing`).
- Gateway-binary freshness (covered by `fix-gateway-binary-freshness-auto-restart-or-warn`).
- Welcome pillar Genesis copy mismatch (covered by `fix-onboarding-welcome-step-genesis-studio-framing`).
- Workspace celebration toast (operator-locked to `created:false` per `fix-pr-2404-use-provision-builddemoprovisionresult-set-created-false`).

## Effort

~300-500 LOC across SPA + Rust + 2 specs. ~6-10 hours. P0 because the operator-locked NO-THEATER rule fails on both surfaces and the Scene 3 final beat lands on an error.
