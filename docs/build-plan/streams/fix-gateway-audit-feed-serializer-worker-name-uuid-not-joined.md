# fix-gateway-audit-feed-serializer-worker-name-uuid-not-joined

**Owner:** codex
**Authored against:** origin/main `2a9911c6828c4b37e9313b32dd024457e31d15b8` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-gateway-audit-feed-serializer-worker-name-uuid-not-joined`
**Source:** UX e2e audit 2026-05-28-0252 (scene-6 F7)

🟠 **P1 — In the live `/api/v1/audit?limit=20` response, every row's `worker` field reads `{"id": "<uuid>", "name": "<same uuid>"}` — the worker UUID is being duplicated into the display name slot. Any /audit render that lands on the real gateway feed shows rows labelled with raw UUIDs instead of "Mariah" / "Peta" / "Priya" / "Jamal" / "Nori".**

## Root cause

The audit-feed serializer in `crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs` constructs each row's `worker` envelope by reading the audit row's `worker_id` column but NOT joining against the workers table to pick up the worker's `name`. The current code path effectively returns `{ id: worker_id, name: worker_id }` — a placeholder masquerading as the display name.

Audit reproduction:

```
curl http://127.0.0.1:7400/api/v1/audit?limit=2 | jq '.items[].worker'
→ { "id": "019e6785-928f-7281-bf8d-e062bc277478",
    "name": "019e6785-928f-7281-bf8d-e062bc277478" }   # ← name is the UUID
```

This is a function-side defect that surfaces whenever the SPA's `DEMO_LIVE=1` path takes effect (currently masked by sibling Scene-6 F5 — the SPA short-circuits to fixtures — but once that path lands real feed data, the camera will read UUID strings). Per the function-vs-dressing rule, names the camera reads must be real.

## Findings this brief closes

- scene-6 F7 [P1] — Real audit rows show UUIDs as worker names; serializer not joining against worker registry

## Fix recipe

1. **Join the audit-list SQL against the workers table** (or whichever table carries the worker registry — confirm by reading `crates/argo-store/src/audit/` and `crates/argo-store/src/workers/` for the existing schema):
   ```sql
   SELECT a.*, w.name AS worker_name
   FROM audit_rows a
   LEFT JOIN workers w ON w.id = a.worker_id AND w.tenant_id = a.tenant_id
   WHERE a.tenant_id = ?1
   ORDER BY a.seq DESC
   LIMIT ?2
   ```
   Tenant-bind the join so cross-tenant worker names cannot leak.
2. **Pass the resolved name through the response serializer** at `routes_audit_feed.rs`. Map the row to `WorkerRef { id: worker_id, name: worker_name.unwrap_or_else(|| worker_id.clone()) }`. The fallback to UUID is preserved only when the worker has been deleted or doesn't exist.
3. **Mirror the same join in `routes_audit_detail.rs`** so the detail page also renders worker names not UUIDs (this is a coupled defect — the same root cause).
4. **Add an integration test** at `crates/argo-cli/tests/audit_feed_worker_name_join.rs`:
   - Seed a worker named "Mariah" with id `m1` under tenant `T1`.
   - Insert an audit row referencing `worker_id = m1`.
   - Fetch `/api/v1/audit?limit=10` with tenant `T1`; assert `items[0].worker.name == "Mariah"` and `items[0].worker.id == m1`.
   - Delete the worker; re-fetch; assert `name` falls back to the UUID.
5. **Watch the LOC budget** — these two handler files are each near the 500-line ceiling. If the join requires moving SQL into a helper, place it under `crates/argo-cli/src/handlers/gateway/audit_query.rs` (new file).

## Acceptance criteria

- [ ] `curl http://127.0.0.1:7400/api/v1/audit?limit=2` returns rows whose `worker.name` matches the worker registry, NOT the UUID, for seeded workers.
- [ ] Same join applies to `/api/v1/audit/<id>` detail responses.
- [ ] Cross-tenant test: a row whose `worker_id` exists in tenant `T2` but is fetched under `T1` returns the UUID as fallback (no cross-tenant leak).
- [ ] `cargo test -p argo-cli --test audit_feed_worker_name_join` green.
- [ ] On the running preview, `/audit` rows display worker names (Mariah, Peta, …) under demo-live mode.
- [ ] No LOC budget violations.
- [ ] PR body cites scene-6 F7.

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs`
- `crates/argo-cli/src/handlers/gateway/routes_audit_detail.rs`
- `crates/argo-cli/src/handlers/gateway/audit_query.rs` (NEW if helper extraction is needed)
- `crates/argo-cli/tests/audit_feed_worker_name_join.rs` (NEW)

## Out of scope

- The list↔detail tenant-resolution drift (covered by `fix-gateway-audit-detail-tenant-resolver-drift-vs-feed`)
- The SPA fallback when the feed returns 0 rows (covered by `fix-approvals-audit-fetchers-fallback-on-200-empty-in-demo-mode`)
- Worker registry schema changes — assume `workers.name` already exists per the seeded roster

## Effort

S — ~60 LOC SQL+serializer change × 2 routes + ~100 LOC integration test. ~3-4 hours.
