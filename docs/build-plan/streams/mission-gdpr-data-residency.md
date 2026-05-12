# Mission load-bearing wire — GDPR data residency (per-tenant DB region pinning)

**Owner**: claude. **Depends on**: `mission-enterprise-cross-tenant-audit` MERGED (tenant_id threading).
**Authored against**: origin/main `d454edb3` on `2026-05-12`.

> SOURCE: MISSION_BACKLOG.md Section 7 #234. EU tenants need data-residency guarantees: their data lives in an EU-region DB, never crosses to US-region. Today: single-DB, single-region. Build the multi-DB routing layer.

## Scope

Extend the store layer to route tenant-scoped reads/writes to a per-tenant DB pool:

1. **Config** `[storage.tenants.<tenant_id>] db_url = "..." region = "eu-west-1"` — operator-defined per-tenant DB connection
2. **`MultiTenantStore`** wrapper around the existing `SessionStore` — looks up the per-tenant pool by tenant_id (from request context), falls back to the default pool if none configured (backward compat)
3. **Cross-region write refusal** — if a request's tenant_id resolves to an EU-region pool but the server is running in US-region (per `ARGO_DEPLOYMENT_REGION`), refuse with 451 (Unavailable for Legal Reasons) + audit row
4. **Migration runner** runs per-tenant (each pool gets its own schema; idempotent)
5. **Operator CLI** `argo storage list-tenants` + `argo storage migrate-tenant <id>` + `argo storage health` per-tenant

This is NOT in scope: live cross-region replication (operator's DB infra responsibility); data-export for tenant offboarding (separate brief — `mission-gdpr-tenant-data-export`); right-to-be-forgotten purge (sibling brief covers per-row delete; this is per-tenant region routing only).

## Files you may touch

- `crates/argo-store/src/multi_tenant_pool.rs` (NEW, ≤ 400 LOC) — pool registry + tenant→pool resolution
- `crates/argo-store/src/multi_tenant_pool_tests.rs` (NEW, sibling, ≤ 300 LOC) — 7 tests
- `crates/argo-store/src/lib.rs` — narrowly scoped: `MultiTenantStore` export
- `crates/argo-cli/src/handlers/gateway/region_gate.rs` (NEW, ≤ 180 LOC) — middleware enforcing region match
- `crates/argo-cli/src/handlers/storage_cmds.rs` (NEW or extend, ≤ 350 LOC) — CLI subcommands
- `argo-config/src/sections.rs` — `[storage.tenants.<id>]` map parsing
- `docs/skycot/RUNBOOK.md` — "Data residency" subsection with EU/US deployment recipe

## Files you must NOT touch

- Existing call sites of the default `SessionStore` (they continue to compile with `MultiTenantStore::default_pool()` for now); per-tenant migration ordering (each pool migrates independently)

## Acceptance criteria

- [ ] `MultiTenantStore` resolves tenant_id → pool with fallback to default
- [ ] Cross-region request returns 451 + audit row (kind: `security.cross_region_blocked`)
- [ ] `argo storage list-tenants` shows region + health per tenant
- [ ] Migration runner is per-tenant + idempotent
- [ ] Backward compat: single-tenant deployments without `[storage.tenants]` config see unchanged behavior
- [ ] 7 unit tests + 1 integration test (2-tenant fixture, distinct regions)
- [ ] `cargo clippy -p argo-store -p argo-cli --all-targets -- -D warnings` clean
- [ ] All touched files < 500 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-gdpr-data-residency`
- PR title: `store+gateway: per-tenant DB region pinning (mission-gdpr-data-residency)`

## Notes

- The 451 status code is the right answer here — operator+regulator distinguish "this request is illegal under the applicable law for the resource" from "you don't have permission".
- Don't try to do cross-region replication in this brief. That's downstream of operator-side DB infra (RDS Global Cluster / Cloud SQL cross-region replicas / etc.).
- The audit row on cross-region block is the visibility seam — compliance auditors want evidence the gate actually fired.
