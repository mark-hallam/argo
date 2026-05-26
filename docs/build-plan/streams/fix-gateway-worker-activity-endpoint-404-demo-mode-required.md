# fix-gateway-worker-activity-endpoint-404-demo-mode-required

**Owner**: codex.
**Authored against**: origin/main `4b3aa443f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-gateway-worker-activity-endpoint-404-demo-mode-required`.
**Source**: UX-AUDIT-T0519 2026-05-27 — `docs/skycot/ux-audits/2026-05-27-0519/scene-1-and-2-claim-1-and-2.md` F14.

🟦 **P2 — Activity tab fetches `GET /api/v1/workers/<id>/activity?range=…`. Gateway returns 404 "not found" — endpoint doesn't exist. The page only renders content because `SAMPLE_WORKER_ACTIVITY` fallback fires under demo-mode. If the demo MP4 is recorded against a non-demo build (`NEXT_PUBLIC_SKYCOT_DEMO_MODE` not set), the tab is empty.**

## Root cause

`web/lib/api/worker-activity.ts:246` calls `apiGet('/api/v1/workers/<id>/activity?range=…')`. The gateway has no handler for this route:

```bash
curl http://127.0.0.1:7400/api/v1/workers/018f67d0-0000-7000-8000-000000000001/activity
# → 404 {"error":"not found"}
```

`worker-activity.ts:236-280` catches the 404 and falls back to `SAMPLE_WORKER_ACTIVITY[workerId]` ONLY if `isDemoMode()` returns true. The demo-mode short-circuit is the only thing keeping the Activity tab populated in the current dev/preview/demo configuration. A non-demo-mode production build would render an empty state.

For the v1.0 demo MP4 (POC scope), this works — the recording is against demo-mode. But:
1. The runbook should EXPLICITLY confirm the demo recording happens with `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`.
2. Long-term, the gateway should implement the endpoint so production tenants have a real activity feed.

This is P2 (not P0) because the demo path works today via the fallback. Filing as a real gateway endpoint brief because the hollow surface erodes Claim 1 ("Teammate, not tool") in production.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F14 [P2] — Activity tab gateway endpoint returns 404; relies on demo-mode fallback the demo MP4 must trigger

## Fix recipe

Two parallel paths.

**Path 1 — short-term documentation (10 minutes).**

1. Update `docs/skycot/demos/v1-mission-mvp-runbook.md` Scene 2 section to explicitly require `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` for the recording.
2. Add a pre-record checklist item: "Confirm `process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE` is set in the recording profile."

**Path 2 — long-term gateway endpoint.**

3. Implement `GET /api/v1/workers/<id>/activity?range=…` in `crates/argo-cli/src/handlers/gateway/`:
   - New file `routes_worker_activity.rs` (sibling to `routes_team_list.rs`).
   - Query the audit table for rows with `worker_id = <id>` AND `timestamp >= now - range`.
   - Project into the same shape `SAMPLE_WORKER_ACTIVITY` expects: `{id, ts, verb, status, channel, action_type, ...}`.
   - Honor the same range tokens the SPA sends (`1d / 7d / 30d`).
   - Tenant-scope every query (per the Claim 6 invariant).

4. Wire the route into `routes_dispatch_get_api.rs`'s dispatch table.

5. Vitest pin (SPA-side): `fetchWorkerActivity(<id>)` returns ≥1 row when the gateway is alive AND demo seed has been run.

6. Cargo test (gateway): `tests/worker_activity_route_tenant_scope.rs` — assert cross-tenant isolation (Worker X's activity NOT readable by Tenant Y).

## Acceptance criteria

- [ ] `curl http://127.0.0.1:7400/api/v1/workers/<canonical-id>/activity?range=7d` returns ≥1 row when demo seed has been run.
- [ ] Cross-tenant: a different `x-argo-tenant-id` header returns 404 (not the rows).
- [ ] Activity tab renders the same shape it does today under demo-mode fallback.
- [ ] Vitest pin + cargo cross-tenant pin pass.
- [ ] Runbook updated with the env-var requirement (Path 1).

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_worker_activity.rs` (NEW)
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_get_api.rs` (wire route)
- `crates/argo-cli/tests/worker_activity_route_tenant_scope.rs` (NEW)
- `web/lib/api/__tests__/worker-activity.test.ts` (pin)
- `docs/skycot/demos/v1-mission-mvp-runbook.md` (Path 1 — runbook note)

## Out of scope

- Worker duties / schedules endpoints (covered by `fix-gateway-worker-duties-schedules-routes-404.md`)
- Audit detail fabrication for unknown ids (covered by `fix-audit-action-detail-demo-mode-fallback.md`)
- SAMPLE_WORKER_ACTIVITY fixture content itself (dressing-side)

## Effort

M — ~150-250 LOC across gateway route + dispatch + cross-tenant test. ~2-4 hours.
