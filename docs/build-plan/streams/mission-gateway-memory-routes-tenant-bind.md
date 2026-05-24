# mission-gateway-memory-routes-tenant-bind

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `91ca7ace7` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-gateway-memory-routes-tenant-bind`.

🔴 **P1 CLAIM-5/6 CROSS-TENANT LEAK CLUSTER.** 8 gateway routes
under `/memory/<id>/…` (lock, unlock, edit, forget, versions, show)
key off `entry_id` only — `argo_memory::MemoryStore::set_lock`,
`supersede`, `revert`, `history` and the two direct `WHERE id = ?`
SELECTs in `routes_lists_memory.rs` + `routes_meta.rs`. tenant_b
with a tenant_a memory UUID can mutate or read tenant_a's row.

## Why

Sibling sweep brief `eval-gateway-uuid-bound-routes-tenant-
ownership-sweep` §3 cluster 4. FINDINGS-ONLY pin
(`pin_cluster_4_memory_lock_route_leaks_across_tenants`) lives at
`crates/argo-cli/tests/gateway_uuid_routes_tenant_ownership.rs`.

The memory cluster is unique among the sweep clusters in that 4
of the 8 handlers (`memory_lock`, `memory_unlock`, `memory_edit`,
`memory_forget`) are **mutating** — meaning a cross-tenant leak
here is not just read-disclosure but unauthorised mutation. P0-ish
in severity; ranked P1 only because no in-the-wild caller has been
observed exploiting it.

## Scope

- Add `tenant_id` parameter to `MemoryStore::set_lock`,
  `supersede`, `revert`, and `history` SQL + Rust signatures.
- Audit `memory_entry` schema for `tenant_id` column — likely
  present already (added v13 ALTER per
  `eval-argo-store-tenant-bind-sweep` row); confirm at claim-time
  and add migration if absent.
- At each of the 8 gateway callsites in cluster 4, derive
  `tenant_id` via `request_tenant_id(req)` and pass through.
- Un-ignore + invert
  `pin_cluster_4_memory_lock_route_leaks_across_tenants`.
- Add 2 same-tenant baseline regressions.
- ~250-350 LOC.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-memory/src/store.rs`
- `crates/argo-memory/src/` — any sibling files for `history` if it lives elsewhere
- `crates/argo-store/src/migrations/` (if migration needed)
- `crates/argo-cli/src/handlers/gateway/routes_post_admin.rs`
- `crates/argo-cli/src/handlers/gateway/routes_lists_memory.rs`
- `crates/argo-cli/src/handlers/gateway/routes_meta.rs`
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_get_catalog_groups.rs`
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post_memory.rs`
- `crates/argo-cli/src/handlers/gateway/routes_post_writes.rs` (append-memory's `parent` field is exploitable cross-tenant; same fix scope)
- `crates/argo-cli/tests/gateway_uuid_routes_tenant_ownership.rs` (un-ignore + invert cluster-4 pin)
- `crates/argo-cli/tests/` — new same-tenant baseline tests
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- Other UUID-bound route clusters (1, 2, 3, 5, 6). Each has its own
  sibling brief.

## Acceptance criteria

- [ ] `MemoryStore::set_lock`/`supersede`/`revert`/`history` all bind `tenant_id`.
- [ ] Both direct `WHERE id = ?` SELECTs in `routes_lists_memory.rs` + `routes_meta.rs` gain `AND tenant_id = ?`.
- [ ] All 8 callsites in cluster 4 thread `tenant_id` from the request.
- [ ] Cluster-4 FINDINGS pin un-ignored + inverted; passing.
- [ ] ≥2 same-tenant baseline tests passing.
- [ ] `cargo clippy -p argo-memory -p argo-store -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.
