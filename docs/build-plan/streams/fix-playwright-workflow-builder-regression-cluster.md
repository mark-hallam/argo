# fix-playwright-workflow-builder-regression-cluster

**Owner**: codex.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-playwright-workflow-builder-regression-cluster`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/chrome-live-walk-and-playwright.md` Playwright failure cluster: workflow builder / save / publish / rollback (13 specs).

🔴 **P0 — The workflow builder suite has 13 failing specs covering add-step, controlled-state, delete, duplicate (×2), shortcuts, concurrent-edit, publish-real (×3), rollback (×1), rollback-button (×2), save-real (×2). This is the entire authoring surface of Claim 5 — the camera-visible "compose a workflow that combines MCP + computer-use" beat is the headline. The regression suite is fully red.**

## Root cause

The cluster is too large to attribute to one bug — it's likely a coordinated state-machine + persistence regression where:

1. **The builder's controlled-state hook may have regressed** — `builder-controlled-state` failing implies the foundational add/edit/delete contracts aren't holding. This is the substrate failure that cascades to all the per-action specs.
2. **The publish/save/rollback REST paths may be returning unexpected envelopes** — `workflow-save-real × 2`, `workflow-publish-real × 3`, `workflow-rollback × 1+2` all fail. Likely a gateway-response shape mismatch or a CORS / auth-header issue.
3. **The concurrent-edit spec failing** suggests the optimistic-lock or version-clash detection logic has changed.

Per audit "Highest-leverage fix targets" section:
> "Workflow builder regression (closes 13 playwright fails): builder save/publish/rollback/concurrent-edit/duplicate/delete all failing — one brief covers them as a single regression."

## Findings this brief closes

- chrome-live-walk-and-playwright Playwright cluster: workflow builder / save / publish / rollback (13 failed specs)
  - `builder-add-step`
  - `builder-controlled-state`
  - `builder-delete`
  - `builder-duplicate` × 2
  - `builder-shortcuts`
  - `workflow-concurrent-edit`
  - `workflow-publish-real` × 3
  - `workflow-rollback` × 1
  - `workflow-rollback-button` × 2
  - `workflow-save-real` × 2

## Fix recipe

1. **Read the playwright run.log** at `docs/skycot/ux-audits/2026-05-26-2007/playwright/run.log` and group the 13 failures by error shape (timeout / assertion / 404 / 500). Don't guess.

2. **Run `pnpm playwright test builder-controlled-state` first** — it's the foundational spec. If it's broken, fix it before touching any other spec; the others may green up automatically.

3. **Inspect `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` and the builder-handlers / palette / inspector modules** for a recent commit that may have changed the contract. Likely candidates:
   - `web/app/(app)/workflows/[verticalId]/[name]/build/builder-handlers.ts`
   - `web/app/(app)/workflows/[verticalId]/[name]/build/SubWorkflowPicker.tsx`
   - `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts`

4. **For the publish/save/rollback specs**: capture the network traffic during a failing run. The gateway routes:
   - `POST /api/v1/verticals/{vid}/workflows/{name}` (save)
   - `POST /api/v1/verticals/{vid}/workflows/{name}/publish`
   - `POST /api/v1/verticals/{vid}/workflows/{name}/rollback`
   Check the request/response envelope against what the SPA expects.

5. **For the concurrent-edit spec**: check the optimistic-lock header (`If-Match` etag or similar) is still being sent + the gateway still expects it.

6. **Re-run the full cluster** until all 13 specs are green. If some are downstream of an unfixed bug (e.g. workflow seed missing), mark them `test.skip` with a clear TODO ledger reference and document in this brief's "deferred" section.

## Acceptance criteria

- [ ] `pnpm playwright test "builder"` passes 0 failed
- [ ] `pnpm playwright test "workflow-publish"` passes
- [ ] `pnpm playwright test "workflow-rollback"` passes
- [ ] `pnpm playwright test "workflow-save"` passes
- [ ] `pnpm playwright test "workflow-concurrent-edit"` passes
- [ ] The 13 failing specs from the 2026-05-26-2007 run.log are all green or explicitly `.skip`-ed with ledger-tracked reason
- [ ] Test wall-time for the cluster < 8 minutes

## May-touch

- `web/app/(app)/workflows/[verticalId]/[name]/build/*` (builder shell, handlers, types)
- `web/lib/api/workflows.ts` (REST envelopes)
- `web/lib/api/workflow-execute.ts` (run-now path if specs cover it)
- `web/playwright/workflows/builder-*.spec.ts` (selector updates if DOM shifted)
- `web/playwright/workflows/workflow-{save,publish,rollback,concurrent-edit}-*.spec.ts`

## Out of scope

- New MCP / CU step kinds in palette (covered by `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`)
- Workflow templates page (covered by `fix-workflow-templates-page-static-seed-no-gateway-binding`)
- The `/workflows/build` Sales-tile redirect (covered by separate brief `fix-workflows-build-sales-tile-redirect-to-onboarding`)

## Effort

~300-600 LOC across multiple builder files + specs + possibly gateway envelope alignment. ~10-16 hours. P0 because Claim 5 authoring is the central capability for the v1.0 narrative.
