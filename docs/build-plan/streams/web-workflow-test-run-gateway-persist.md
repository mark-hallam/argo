# web-workflow-test-run-gateway-persist

**Authored against**: `origin/main` `842a6e2e4` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-workflow-test-run-gateway-persist`.

## Summary

#1971 landed NDJSON test-run streaming. #1980 landed full workflow-rollback (UI + backend). The test-run history is currently localStorage-only (tick-24 brief `web-workflow-test-run-history-panel`). Persist test-run results server-side so a workflow's run history survives browser clears + is multi-device.

Pre-condition: **#1971 MERGED ✅** + **#1980 MERGED ✅**.

## Acceptance

- [ ] New gateway endpoint `POST /api/v1/workflows/<vid>/<name>/test-runs` accepts the test-run summary (steps + status + duration + ndjson_blob_ref).
- [ ] New gateway endpoint `GET /api/v1/workflows/<vid>/<name>/test-runs?limit=10` returns the last N runs for the workflow.
- [ ] New table `workflow_test_run` (next free migration slot via `bash scripts/check-migration-slots.sh`) with tenant_id, workflow_id, status, started_at, finished_at, step_count, ndjson_summary fields. Tenant-scoped via canonical pattern.
- [ ] Cross-tenant negative test: tenant B can't fetch tenant A's runs → 404 (per #1983 pattern).
- [ ] When a test run completes (`useTestRun` hook emits "done"), client posts to the new endpoint. Best-effort: persist failure doesn't break the in-memory overlay.
- [ ] `TestRunHistoryPanel` (when it lands from tick-24 brief) reads from the gateway endpoint INSTEAD of localStorage. Fall back to localStorage only if endpoint is unreachable in non-strict-prod mode.
- [ ] At least one integration test covering happy-path + cross-tenant negative.
- [ ] `AuditKind::WorkflowTestRunCompleted` variant emitted on completion (4-site recipe).

## May touch

- New migration under `crates/argo-store/migrations/<slot>__workflow_test_run.sql`
- `crates/argo-store/src/workflow_test_run.rs` (new module + tests)
- `crates/argo-cli/src/handlers/gateway/routes_workflows_test_runs.rs` (new file)
- `crates/argo-cli/src/handlers/gateway/routes_workflows_test_runs_tests.rs` (new)
- `web/lib/api/test-runs.ts` (new)
- `web/lib/hooks/use-test-run.ts` (extend to POST on completion)
- `crates/argo-audit/src/event.rs` (4-site recipe for new AuditKind)

## Must NOT touch

- The NDJSON streaming endpoint (#1971 settled).
- The RunOverlay component (#1971 settled).
- The workflow-rollback machinery (#1980 settled).
- `workflow_definition` schema.

## Notes

- Sizing: 400-700 LOC (backend additive + UI swap + 2 tests).
- Pairs with `web-workflow-test-run-history-panel` (tick-24 brief) — if that lands first, this brief swaps its localStorage to gateway.
- Mission claim-3 leverage: persisted run history is procurement-grade table-stakes.
