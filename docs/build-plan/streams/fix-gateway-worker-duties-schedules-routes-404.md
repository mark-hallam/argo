# fix-gateway-worker-duties-schedules-routes-404

**Owner**: codex.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-gateway-worker-duties-schedules-routes-404`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/whole-site-mvp-claims-coverage.md` F15 [P2].

🟡 **P2 — Worker `/duties` and `/schedules` gateway routes still 404; Scene 2's "click Mariah's Schedules tab + Run now → audit row" remains backend-impossible. Currently the SPA's duties card on `/workers/{id}/overview` renders from sample data; the toggle is a console.warn stub.**

## Root cause

```
curl -s http://127.0.0.1:7400/api/v1/workers/<id>/duties → {"error":"not found"}
curl -s http://127.0.0.1:7400/api/v1/workers/<id>/schedules → {"error":"not found"}
```

No gateway routes registered for either path. The SPA's `DutiesCard` component lives at `web/components/skycot/worker/DutiesCard.tsx` and the toggle handler at `WorkerOverviewContent.tsx:74-92` is explicitly a stub:
```ts
const onToggleDuty = useCallback((dutyId, enabled) => {
  console.warn(`[E4-b] duty '${dutyId}' toggled to ${enabled} — mutation lands with E4-h`);
}, []);
```

The worker has a `duties` table per `crates/argo-identity/src/store_hire.rs:151-213` so the data model exists; just no read/write HTTP endpoint.

## Findings this brief closes

- whole-site-mvp-claims-coverage F15 [P2] — Worker /duties + /schedules gateway routes still 404

## Fix recipe

1. **Add `GET /api/v1/workers/{id}/duties`** handler at `crates/argo-cli/src/handlers/gateway/routes_worker_duties.rs` (new file). Returns the worker's duties list with shape:
   ```json
   { "duties": [
     { "id": "d-list", "label": "List new inventory", "cron": "0 9 * * 1-5", "enabled": true },
     ...
   ]}
   ```

2. **Add `PATCH /api/v1/workers/{id}/duties/{duty_id}`** to toggle / edit a duty.

3. **Add `POST /api/v1/workers/{id}/duties`** to create a new duty.

4. **Add `DELETE /api/v1/workers/{id}/duties/{duty_id}`** to remove.

5. **Alias `/schedules`** to `/duties` so the user-facing vocabulary (Schedule per operator-locked memory) matches the URL. Both paths route to the same handler.

6. **Register routes** at `crates/argo-cli/src/handlers/gateway/dispatch.rs` and the per-method group files (`routes_dispatch_get_api_groups.rs`, `routes_dispatch_post_api_groups.rs`, etc).

7. **Update SPA toggle handler** at `web/components/skycot/worker/WorkerOverviewContent.tsx:74-92` to fire a real `PATCH` request via `useMutation` + `mutateDuty()`:
   ```ts
   const { mutateAsync: toggleDuty } = useDutyToggle(workerId);
   const onToggleDuty = useCallback(async (dutyId, enabled) => {
     await toggleDuty({ dutyId, enabled });
   }, [toggleDuty]);
   ```

8. **Implement `useDutyToggle` hook** at `web/lib/api/worker-duties.ts` (NEW).

9. **Add a Run-now per-duty button** that fires the duty's associated workflow via `executeWorkflow` (existing API). Wire it into the DutiesCard component.

10. **Add Playwright spec** at `web/playwright/workers/duties-toggle-and-runs.spec.ts`:
    - Load `/workers/{id}/overview` (or `/schedules` tab)
    - Toggle a duty off → assert UI updates + audit row appears with `duty.toggled`
    - Click Run-now → assert audit row appears with `workflow.executed`

## Acceptance criteria

- [ ] `GET /api/v1/workers/{id}/duties` returns 200 with worker's duties
- [ ] `PATCH /api/v1/workers/{id}/duties/{duty_id}` updates the duty + emits audit row
- [ ] `POST /api/v1/workers/{id}/duties` creates new duty
- [ ] `/api/v1/workers/{id}/schedules` aliases to the same handler
- [ ] SPA toggle fires a real mutation, not console.warn
- [ ] Run-now button works per duty
- [ ] New Playwright spec passes
- [ ] Cold rerun of claim-1 + claim-2 LIVE tests still pass

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_worker_duties.rs` (NEW)
- `crates/argo-cli/src/handlers/gateway/dispatch.rs`
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_get_api_groups.rs`
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post_api_groups.rs`
- `crates/argo-identity/src/duties.rs` (read/write helpers)
- `web/lib/api/worker-duties.ts` (NEW)
- `web/components/skycot/worker/DutiesCard.tsx`
- `web/components/skycot/worker/WorkerOverviewContent.tsx`
- `web/playwright/workers/duties-toggle-and-runs.spec.ts` (NEW)

## Out of scope

- The "Schedules" tab as a separate top-level worker tab (orthogonal UX brief)
- Workflow execute path (already shipped)
- Cron / schedule editor UI (v1.1 polish)

## Effort

~400-700 LOC across gateway + SPA + spec. ~10-15 hours. P2 listed but P1 for Scene 2 demo recording — Run-now beat is unrecordable without this OR the workflow-detail Run-now (covered by separate brief).
