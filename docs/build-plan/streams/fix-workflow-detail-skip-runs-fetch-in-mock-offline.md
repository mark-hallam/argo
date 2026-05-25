# fix-workflow-detail-skip-runs-fetch-in-mock-offline

**Owner**: claude.
**Authored against**: origin/main `5c4abb23e1f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workflow-detail-skip-runs-fetch-in-mock-offline`.
**Source**: e2e-ux-audit (scene-4-and-5-claim-4-and-5) finding F8 — `docs/skycot/ux-audits/2026-05-26-0820/scene-4-and-5-claim-4-and-5.md`.

🟡 **P1 — Workflow detail page always fires `/workflow-runs` query even when source is mock-offline. Guaranteed 404 per page load. Pollutes dev-server logs and adds 200-400ms latency.**

## Root cause

`web/app/(app)/workflows/[verticalId]/[name]/page.tsx:108-132` calls `fetchWorkflowRuns` unconditionally — regardless of whether the workflow's source is `mock-offline` or `gateway`. Against a dev gateway with no run history, this is a guaranteed 404 per workflow detail page open.

Not visible to the camera (the page handles the 404 gracefully and renders the workflow card without runs), but:
1. Pollutes dev-server console with `/api/v1/workflow-runs?…` 404 lines (~1 per page load)
2. Adds 200-400ms of network latency to the page load
3. Erodes the "production-ready" feel during dev sessions
4. Burns gateway CPU on requests that can't possibly return data when source is mock-offline

The fix: short-circuit the `fetchWorkflowRuns` call when `workflow.source === "mock-offline"`, OR have the hook itself return an empty mock-runs envelope without hitting the network.

## Findings this brief closes

- scene-4-and-5-claim-4-and-5 F8 [P1] — Workflow detail hits `/workflow-runs` even with mock data; 404 noise + latency

## Fix recipe

**Option A (preferred — short-circuit at the call site)**:
1. Touch `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:108-132`.
2. Wrap the `useQuery({ queryKey: ['workflow-runs', ...], queryFn: fetchWorkflowRuns })` with a `enabled` guard:
   ```ts
   enabled: workflow.source !== "mock-offline" && workflow.source !== "mock-bypass"
   ```
3. When disabled, the query returns `data === undefined`. The render path already handles the "no runs" branch (verify); just ensure the empty-state copy reads sensibly when source is mock-offline (e.g. "Demo workflow · no run history" rather than "No runs yet").

**Option B (alternative — make the hook itself smarter)**:
1. Touch `web/lib/api/workflow-runs.ts` (or wherever `useWorkflowRuns` lives).
2. Detect mock-offline source via `isDemoMode()` or by checking the workflow's source field passed in.
3. Return a pre-canned empty runs envelope (`{ runs: [], total: 0, source: "mock-offline" }`) without hitting the network.
4. This centralizes the logic but adds coupling between the runs hook and the workflow source — Option A is cleaner.

Pick Option A.

**3. Add a Playwright assertion** at `web/tests/e2e/workflow-detail-no-spurious-fetch.spec.ts`:
- Navigate to `/workflows/sales/qualify_lead` with mock-offline mode
- Use Playwright's network interception to capture all `/api/v1/workflow-runs*` calls during the page load
- Assert 0 such calls
- Repeat for `/workflows/sales/customer_research_followup` (once it ships per `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`)

## Acceptance criteria

- [ ] In mock-offline mode, opening `/workflows/<vertical>/<name>` does NOT trigger a `/api/v1/workflow-runs` fetch.
- [ ] Dev-server console no longer shows `/workflow-runs?…` 404 lines on workflow detail page navigation in mock-offline.
- [ ] Page load latency on workflow detail drops by 200-400ms in mock-offline (measure via Chrome DevTools or Playwright timing).
- [ ] In real-gateway mode (source !== mock-offline), `/workflow-runs` fetch fires as today (no regression).
- [ ] Empty-state copy reads sensibly when source is mock-offline ("Demo workflow · no run history" or similar).
- [ ] New Playwright spec passes.

## May-touch

- `web/app/(app)/workflows/[verticalId]/[name]/page.tsx` (the `useQuery` enabled guard)
- `web/lib/api/workflow-runs.ts` (if Option B chosen)
- `web/tests/e2e/workflow-detail-no-spurious-fetch.spec.ts` (NEW)

## Out of scope

- The 404 on the runs endpoint itself (separate concern — when gateway is up but no runs exist, returning a 200 with empty array is a separate brief)
- Workflow run detail page (different surface)
- Refactoring all mock-offline guards across the SPA (incremental — this brief tackles one specific high-noise call site)

## Effort

~20-40 LOC + 1 test. ~30 minutes. Cheap quality-of-life fix.
