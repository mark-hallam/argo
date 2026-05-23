# mission-gateway-forge-runs-tenant-bind

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `91ca7ace7` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-gateway-forge-runs-tenant-bind`.

🔴 **P1 CLAIM-5 CROSS-TENANT LEAK CLUSTER.** 6 gateway routes under
`/forge/runs/<id>/…` (promote, requeue, show, payload, audit,
abort) key off `forge_run_id` only — `SessionStore::get_forge_run`,
`requeue_forge_run`, and the audit / payload neighbours. tenant_b
with a tenant_a forge_run UUID can read the run row, promote its
candidate to a skill, requeue it, or abort it.

## Why

Sibling sweep brief `eval-gateway-uuid-bound-routes-tenant-
ownership-sweep` §3 cluster 5. FINDINGS-ONLY pin
(`pin_cluster_5_forge_run_promote_leaks_across_tenants`) lives at
`crates/argo-cli/tests/gateway_uuid_routes_tenant_ownership.rs`.

**Promotion is the worst-case mutation** in this cluster — a
successful cross-tenant promote inserts a row into the **target
tenant's** skill registry (the requesting tenant), keyed off the
**source tenant's** candidate version. That's both a leak AND a
supply-chain insertion: tenant_b can poison tenant_b's own skill
namespace with arbitrary content sourced from tenant_a's forge
runs.

## Scope

- Add `tenant_id` filter to `SessionStore::get_forge_run` SQL +
  Rust signature (additive-constructor pattern preferred).
- Same for `requeue_forge_run`, the `forge_run` payload SELECT,
  and the audit-rollup SQL that reads `forge_run`.
- Audit `forge_run` schema for `tenant_id` column — confirm at
  claim-time (likely present per recent forge audit work).
- At each of the 6 gateway callsites in cluster 5, derive
  `tenant_id` via `request_tenant_id(req)` and pass through.
- Specifically for `forge_promote_response`: the **promote**
  path's `skills.promote(name, version)` already binds against
  the requesting tenant's skill registry — but the row read at the
  top of the handler determines `(skill_name, candidate_version)`.
  A cross-tenant `get_forge_run` returning the source tenant's row
  is the exploit. The fix is upstream: the `get_forge_run` filter.
- Un-ignore + invert
  `pin_cluster_5_forge_run_promote_leaks_across_tenants`.
- Add 2 same-tenant baseline regressions.
- ~200-300 LOC.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-store/src/forge_run_query.rs` (or wherever `get_forge_run` lives — grep at claim-time)
- `crates/argo-store/src/migrations/` (if column-add needed; likely not)
- `crates/argo-cli/src/handlers/gateway/routes_forge.rs`
- `crates/argo-cli/src/handlers/gateway/routes_forge_promote.rs`
- `crates/argo-cli/src/handlers/gateway/routes_forge_rollups.rs`
- `crates/argo-cli/src/handlers/gateway/routes_post_writes.rs` (`forge_abort_response` only)
- `crates/argo-cli/tests/gateway_uuid_routes_tenant_ownership.rs` (un-ignore + invert cluster-5 pin)
- `crates/argo-cli/tests/` — new same-tenant baseline tests
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- Other UUID-bound route clusters (1, 2, 3, 4, 6). Each has its own
  sibling brief.

## Acceptance criteria

- [ ] `SessionStore::get_forge_run`, `requeue_forge_run`, and the audit/payload neighbours all bind `tenant_id`.
- [ ] All 6 callsites in cluster 5 thread `tenant_id` from the request.
- [ ] Cluster-5 FINDINGS pin un-ignored + inverted; passing.
- [ ] ≥2 same-tenant baseline tests passing — particularly the same-tenant promote path stays green.
- [ ] `cargo clippy -p argo-store -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.
