# Mission load-bearing wire — cross-tenant audit isolation

**Owner**: claude. **Depends on**: J14 (MERGED #1419) audit-feed endpoint; `mission-enterprise-audit-full-text-search` strongly preferred MERGED first (migration ordering: 64 before 65).
**Authored against**: origin/main `eb71de86` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-enterprise-cross-tenant-audit`.

> SOURCE: J14 #1419 PR-deferral. The audit-feed endpoint can't return a 403 on cross-tenant access because `audit_record` has no `tenant_id` column — sessions are workspace-derived and the gateway authenticates a single bearer-token. That's a structural single-tenant assumption that breaks the moment a tenant deploys for a team. This brief threads `tenant_id` through the audit chain end-to-end and adds the cross-tenant gate.
>
> **REWRITE NOTE 2026-05-12 (conductor)**: an earlier version of this brief was BLOCKED for premise drift — it cited a non-existent `argo-rbac` crate, a fictional `AuditStore::append`, and a non-existent `TenantStore::tenant_for_operator`. The rewrite below uses actual paths verified against origin/main: `crates/argo-audit/src/sink.rs::AuditSink::write(session_id, kind, payload, principal_id)`, the `argo-capability` crate (not `argo-rbac`), and the workspace-derived tenant convention already documented in `routes_audit_feed.rs` module docs ("Tenant scoping is single-tenant per workspace").

## Scope

Migration 65 adds `tenant_id TEXT NOT NULL DEFAULT '__legacy__'` to `audit_record`. The `__legacy__` sentinel grandfathers every historical row into one bucket — the chain stays intact (the hashed columns `id, session_id, kind, payload_json, parent_hash, self_hash, created_at` are not touched).

New writes derive `tenant_id` from the workspace identity. v1 keeps the existing single-tenant-per-workspace convention: the workspace's `argo.toml` carries an optional `[tenant] id = "..."` field; absent that, derive a stable tenant id from `workspace_root_path_hash[0..16]` so existing single-tenant deployments self-name without operator intervention. The `__legacy__` bucket is readable by the default workspace tenant so existing audit visibility is preserved without an env-flag flip.

Reads enforce: requesting principal in workspace W sees audit rows where `tenant_id` equals W's tenant. Cross-tenant access returns `{items:[], next_cursor:null}` with `200` (NOT 403 — that would leak existence). An explicit `?as_tenant=<other>` admin probe is gated by a new `audit.view.cross_tenant` capability via `argo-capability::CapabilityEngine`; no built-in role grants it.

Specifically:

1. **Migration 65** — `ALTER TABLE audit_record ADD COLUMN tenant_id TEXT NOT NULL DEFAULT '__legacy__'`. Index `(tenant_id, created_at)`. No FK — `tenant_id` is a free-form string. (Slot 63 = J16 `elevated_view_grant`; slot 64 = `mission-enterprise-audit-full-text-search` audit filter columns; slot 65 = this brief.)
2. **Workspace → tenant_id resolver** — new helper `argo_config::workspace_tenant_id(workspace_root: &Path) -> String` reads `[tenant] id` from `argo.toml` if present, else returns `format!("ws-{}", short_hash_of_path(workspace_root))`. Stable across runs. Documented in `RUNBOOK.md`.
3. **`AuditSink::write_with_tenant`** — new method on `AuditSink` that takes an explicit `tenant_id: String`. Existing `AuditSink::write(...)` is preserved and delegates to `write_with_tenant(workspace_tenant_id, ...)` using a tenant id baked into the sink at construction time. New ctor `AuditSink::new_with_tenant(store, tenant_id)`; existing `AuditSink::new(store)` calls `new_with_tenant(store, workspace_tenant_id(cwd()))`. Backward compatible — all ~25 call sites of `sink.write(...)` continue to compile.
4. **`tenant_id` in `payload_json`** — for forward-compat with `mission-enterprise-audit-full-text-search`'s denormalization pattern, also stamp `tenant_id` inside `payload_json` at the publish edge (so the JSON and the column always match for new rows). NOT part of the hash chain inputs (chain hash uses `kind.label() + canonical(payload)`; the tenant denorm column is hashed-payload-derivable but the column itself is denorm). `verify_chain` extension: assert that `payload_json.tenant_id` equals the column when both are present.
5. **Read path** in `crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs` — inject `WHERE tenant_id = ?` unconditionally using the workspace tenant id. The `?as_tenant=<other>` query param overrides only when the principal has the `audit.view.cross_tenant` capability; otherwise treat the override as silently ignored (request behaves as if the param wasn't supplied — no leak via 4xx code).
6. **Capability registration** — `argo-capability::default_roles()` does NOT grant `audit.view.cross_tenant` to any built-in role. Operators add it explicitly via the existing role config path (per CAPABILITY_GUIDE if one exists, else inline in `lib_tests.rs` as a documented example).
7. **2-tenant integration fixture** — `crates/argo-cli/tests/audit_cross_tenant_smoke.rs` (NEW): seed two `argo.toml` workspaces with distinct `[tenant] id`, write audit rows under each, verify the feed scoped to tenant A sees only A's rows and the explicit `?as_tenant=B` probe (a) returns empty without capability, (b) returns B's rows with capability granted.

This is NOT in scope: `tenant_id` on other tables (`memory_entry`, `pending_approval`, etc. — own briefs); a tenant admin UI; tenant invite flow; migrating the `__legacy__` bucket to real tenant_ids (operational concern); per-tenant audit retention policy; tenant deletion / merge.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/migrations/65__audit_record_tenant_id.sql` (NEW, ≤ 40 LOC) — column + index. Verify slot 65 is free at claim time; slots 63 and 64 are taken by J16 elevated_view_grant + mission-enterprise-audit-full-text-search.
- `crates/argo-config/src/lib.rs` (≤ 480 LOC after) — `workspace_tenant_id` helper + `[tenant]` section parsing
- `crates/argo-config/src/lib_tests.rs` (≤ 350 LOC after) — 3 tests: argo.toml override, hash-fallback stability, `__legacy__` sentinel never returned by helper
- `crates/argo-audit/src/sink.rs` (≤ 350 LOC after — currently ~280 LOC) — `new_with_tenant` ctor, `write_with_tenant` method, `write` shim, tenant_id stamping in `payload_json` + column insert
- `crates/argo-audit/src/tests.rs` (extend, ≤ 480 LOC after) — 4 tests: tenant_id stamped on write, legacy bucket isolation, chain hash unchanged for new rows, verify_chain extension catches denorm drift
- `crates/argo-audit/src/verify.rs` (≤ 300 LOC after) — extend chain verification to also assert `payload_json.tenant_id` matches column when both present
- `crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs` (≤ 490 LOC after — currently ~377 LOC per existing brief) — workspace tenant resolution + SQL injection + `?as_tenant=` capability gate
- `crates/argo-cli/src/handlers/gateway/routes_audit_feed_tests.rs` (≤ 490 LOC after — currently ~393 LOC) — 3 tests: tenant scope by default, `?as_tenant=` ignored without capability, `?as_tenant=` honored with capability
- `crates/argo-capability/src/lib.rs` (≤ 400 LOC after) — register `audit.view.cross_tenant` capability constant; NOT added to any default role
- `crates/argo-cli/tests/audit_cross_tenant_smoke.rs` (NEW, ≤ 250 LOC) — 2-tenant integration smoke
- `docs/skycot/RUNBOOK.md` — append section "Multi-tenant audit (v1)" documenting the `[tenant] id` config, the `__legacy__` bucket, and the `audit.view.cross_tenant` capability flag

## Files you must NOT touch

- `crates/argo-store/migrations/0..64__*.sql` — locked (63 = J16, 64 = audit-fts if MERGED first; otherwise this brief uses 64 and the audit-fts brief bumps to 65)
- Audit chain hash inputs (`chain_hash(parent_hash, kind.label(), canonical(payload))` in `argo-audit/src/sink.rs`) — adding tenant_id to the hash inputs would invalidate every historical chain
- `crates/argo-channels/**` — orthogonal
- The capability default-roles list — adding `audit.view.cross_tenant` to a default role would be a security regression

## Acceptance criteria

- [ ] Migration 65 applies forward (new column with `__legacy__` default) AND on a populated DB (existing rows get `__legacy__`)
- [ ] `AuditSink::write_with_tenant` writes `tenant_id` to both column + `payload_json.tenant_id`
- [ ] `AuditSink::write` backward-compatible — all existing call sites continue to compile and behave identically (using the workspace tenant)
- [ ] Hash chain unchanged for pre-migration rows — re-hashing a historical row produces the same `self_hash`
- [ ] `verify_chain` flags drift between `payload_json.tenant_id` and the denorm column (when both present)
- [ ] Audit-feed endpoint scopes by tenant: cross-tenant access returns `{ items: [], next_cursor: null }` with `200`
- [ ] `?as_tenant=<other>` silently ignored without `audit.view.cross_tenant` capability (no 4xx — no existence leak); honored with capability
- [ ] Legacy `__legacy__` rows visible to the default workspace tenant
- [ ] Integration smoke `audit_cross_tenant_smoke.rs` passes: 2-tenant fixture verifies isolation + capability-gated override
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] All touched files < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-enterprise-cross-tenant-audit`
- PR title: `audit: cross-tenant isolation + tenant_id column (mission-enterprise-cross-tenant-audit)`

## Notes for the implementer

- The migration-slot landscape is moving — at claim time, run `ls crates/argo-store/migrations/ | tail -5` and verify slot 65 is free. If J16 (#1426) hasn't merged but `mission-enterprise-audit-full-text-search` has, slot 63 may be free; pick the next free slot in order and document the choice in the migration header comment.
- The `__legacy__` bucket is the most subtle correctness gotcha. Document it as a *deployment-time decision*: operators running a real multi-tenant deployment need to migrate `__legacy__` rows to real tenant_ids before the bucket exits the default-tenant scope. A follow-up brief `mission-audit-legacy-tenant-migrate-cli` will add `argo audit migrate-legacy-tenant <from-tenant> <to-tenant>`.
- The `204-with-empty-items vs 403` distinction on cross-tenant reads is intentional and load-bearing. 403 leaks "this row exists in some tenant". Empty 200 is what an enterprise auditor expects. The capability-gated `?as_tenant=` probe is the explicit admin path.
- **Do NOT add tenant_id to the chain hash inputs.** Doing so retroactively requires re-hashing every historical row, which breaks every external bundle export (see `E5-e-audit-evidence-bundle-exporter` and `E5-h-audit-cross-system-jump-links` downstream consumers that rely on hashes being stable). The denorm column + payload_json stamp + verify_chain sanity-check is the safety pattern.
- `argo-capability` is the actual capability system. The earlier-blocked version of this brief incorrectly cited `argo-rbac` which doesn't exist. `CapabilityEngine::check(&principal, &requested_cap)` is the gate; `grant_dynamic` is how to grant `audit.view.cross_tenant` at test/operator-config time.
- The `workspace_tenant_id` helper's hash-fallback path is the seam that lets fresh single-tenant deployments work without any config. Document it: existing single-tenant deployments will see all their rows under one stable derived tenant id (not `__legacy__`, because the helper never returns the sentinel — only historical rows have it).
- Test the empty-page response shape carefully: a cross-tenant request returning `{ items: [], next_cursor: null }` should be indistinguishable from a legitimate empty query, including timing. Log the same `audit.feed.queried` audit row in both cases.
