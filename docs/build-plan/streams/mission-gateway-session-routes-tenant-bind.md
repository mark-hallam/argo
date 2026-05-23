# mission-gateway-session-routes-tenant-bind

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `91ca7ace7` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-gateway-session-routes-tenant-bind`.

🔴 **P1 CLAIM-5/6 CROSS-TENANT LEAK CLUSTER.** 11 gateway routes
read `/sessions/<id>/…` data — header, spend, stats, turns, audit
rollups, clone — without binding tenant_id. tenant_b with a
tenant_a session UUID receives tenant_a's row data + can drive
session-clone.

## Why

Sibling sweep brief `eval-gateway-uuid-bound-routes-tenant-
ownership-sweep` (PR pending merge) §3 cluster 2 enumerates 11
handlers that call `SessionStore::get_session(id)` (or per-session
aggregate SQL) with no tenant filter. The shape mirrors `decide_
request` (#2146) and `session_export` (`mission-gateway-session-
export-tenant-bind`). FINDINGS-ONLY pin
(`pin_cluster_2_session_show_route_leaks_across_tenants`) lives at
`crates/argo-cli/tests/gateway_uuid_routes_tenant_ownership.rs`.

## Scope

- Add `tenant_id` filter to `SessionStore::get_session` SQL
  (`crates/argo-store/src/session_query.rs` — likely an
  additive-constructor `get_session_for_tenant(id, tenant_id)`
  pattern per the brief-authoring pre-flight memory; leave the
  current fn delegating to it for callers that don't yet pass
  tenant).
- Thread `tenant_id` through the per-session aggregate helpers
  (`session_tool_call_count`, `session_spend_micros`, the
  `audit_record` rollups in `routes_session_audit_rollups.rs`).
- At each of the 11 gateway callsites in cluster 2, derive
  `tenant_id` via `request_tenant_id(req)` (same helper
  `routes_audit_replay.rs:156` uses) and pass through to the store
  fn.
- Un-ignore + invert
  `pin_cluster_2_session_show_route_leaks_across_tenants`.
- Add 2 same-tenant baseline regressions confirming the
  legitimate path still 200s.
- ~300-450 LOC across argo-store + argo-cli + tests.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-store/src/session_query.rs`
- `crates/argo-store/src/migrations/` (if PK / index needs widening — likely not, `session.tenant_id` already exists)
- `crates/argo-cli/src/handlers/gateway/routes_session.rs`
- `crates/argo-cli/src/handlers/gateway/routes_session_stats.rs`
- `crates/argo-cli/src/handlers/gateway/routes_session_clone.rs`
- `crates/argo-cli/src/handlers/gateway/routes_session_audit_rollups.rs`
- `crates/argo-cli/src/handlers/gateway/routes_audit_session.rs`
- `crates/argo-cli/tests/gateway_uuid_routes_tenant_ownership.rs` (un-ignore + invert cluster-2 pin)
- `crates/argo-cli/tests/` — new same-tenant baseline tests
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- Other UUID-bound route clusters (1, 3, 4, 5, 6). Each has its own
  sibling brief.

## Acceptance criteria

- [ ] `SessionStore::get_session` (or its tenant-aware sibling) binds `tenant_id`.
- [ ] All 11 callsites in cluster 2 thread `tenant_id` from the request.
- [ ] Cluster-2 FINDINGS pin un-ignored + inverted; passing.
- [ ] ≥2 same-tenant baseline tests passing.
- [ ] `cargo clippy -p argo-store -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.
