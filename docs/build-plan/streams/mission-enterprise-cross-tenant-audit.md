# Mission load-bearing wire — cross-tenant audit isolation

**Owner**: claude. **Depends on**: J14 (MERGED #1419) audit-feed endpoint; **strongly prefers** `mission-enterprise-audit-full-text-search` MERGED first (migration ordering: 63 before 64).
**Authored against**: origin/main `f1fa23f3` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-enterprise-cross-tenant-audit`.

> SOURCE: J14 #1419 PR-deferral. The audit-feed endpoint can't return a `403` on cross-tenant access because `audit_record` has no `tenant_id` column — sessions are workspace-derived and the bearer-token map happens to be 1:1 with operators today. That's a structural single-tenant assumption that breaks the moment a tenant deploys for a team. This brief threads `tenant_id` through the audit chain end-to-end and adds the cross-tenant 403 gate.

## Scope

Migration 64 adds `tenant_id TEXT NOT NULL DEFAULT '__legacy__'` to `audit_record`. The `__legacy__` sentinel is the v1-grandfather marker — every historical row maps to a single "this was before multi-tenancy" bucket so the chain stays intact. New writes populate the real `tenant_id` from the request context (operator → tenant lookup via the existing `argo-rbac::TenantStore`).

Reads enforce: requesting principal must be in tenant T to see audit rows where `tenant_id = T`. The `__legacy__` bucket is readable by tenant=default-tenant-of-the-operator (configurable per deployment), so existing single-tenant deployments keep their audit visibility without an env-flag flip.

Specifically:

1. **Migration 64** — `ALTER TABLE audit_record ADD COLUMN tenant_id TEXT NOT NULL DEFAULT '__legacy__'`. Index `(tenant_id, created_at)` for the dominant per-tenant range-scan pattern. No FK — `tenant_id` is a free-form string (the tenant store is the source of truth).
2. **Hash chain widening** — `tenant_id` becomes part of `payload_json` (NOT a new hashed column) so the chain stays compatible with `mission-enterprise-audit-full-text-search`'s denorm columns. The `verify_chain` invariant becomes: `tenant_id` in JSON matches the denorm column.
3. **Write path** in `argo-store::AuditStore::append` reads tenant_id from the new `AuditAppendCtx` parameter (replacing the current single-arg signature). Caller threading: every existing call site updated. ~30 call sites — use a sed-grep audit, not a one-pass refactor.
4. **Read path** in `routes_audit_feed.rs` reads `tenant_id` from the bearer-token → tenant lookup (existing in `argo-rbac::TenantStore::tenant_for_operator`). SQL `WHERE tenant_id = ?` injected unconditionally; the 403 path is "tenant lookup failed for this operator" (not "tenant_id mismatch on a row" — that would expose existence). Cross-tenant access returns 404, not 403, to avoid leaking row existence.
5. **403 explicit gate** for the single case where the operator passes `?as_tenant=<other>` (admin override, gated by an `audit.view.cross_tenant` capability that no role has by default).

This is NOT in scope: tenant_id on other tables (`memory_entry`, `pending_approval`, etc. — they have their own briefs); a tenant admin UI; tenant invite flow; migrating the `__legacy__` bucket to real tenant_ids (operational concern); per-tenant audit retention policy.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/migrations/64__audit_record_tenant_id.sql` (NEW, ≤ 40 LOC) — column + index
- `crates/argo-store/src/audit.rs` (≤ 480 LOC after) — `AuditAppendCtx` parameter, `tenant_id` in JSON + denorm, verify_chain extension
- `crates/argo-store/src/audit_tests.rs` (≤ 480 LOC after) — 4 new tests: tenant_id round-trip, legacy bucket isolation, chain integrity across tenant_id, hash unchanged for existing chain
- `crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs` (≤ 490 LOC after) — tenant lookup + SQL injection
- `crates/argo-cli/src/handlers/gateway/routes_audit_feed_tests.rs` (≤ 490 LOC after) — 3 new tests: cross-tenant returns 404 with empty `items`, `?as_tenant=` requires capability (403 without), legacy bucket visible to default-tenant operator
- `crates/argo-rbac/src/capabilities.rs` (≤ 280 LOC after) — register `audit.view.cross_tenant` capability
- ~30 call sites of `AuditStore::append` — updated to pass `AuditAppendCtx` (compile-driven sweep)
- `crates/argo-cli/src/runtime_init.rs` or wherever the audit-write context is built — wire `tenant_id` from operator → tenant lookup

## Files you must NOT touch

- `crates/argo-store/migrations/0..63__*.sql` — locked (this is slot 64)
- The audit chain hash inputs — `tenant_id` stays in `payload_json`, NOT a new hashed column
- `crates/argo-rbac/src/roles.rs` — don't grant the new capability by default to any built-in role

## Acceptance criteria

- [ ] Migration 64 applies forward + on a populated DB (existing rows → `__legacy__`)
- [ ] `AuditAppendCtx { tenant_id, principal_id }` threaded through all `append` call sites
- [ ] Hash chain unchanged for pre-migration rows; new rows hash with `tenant_id` baked into `payload_json`
- [ ] `verify_chain` flags drift between denorm column and JSON
- [ ] Audit-feed endpoint scopes by tenant: cross-tenant access returns `{ items: [], next_cursor: null }` with `200` (not 403 — that would leak row existence)
- [ ] `?as_tenant=<other>` returns `403` without `audit.view.cross_tenant` capability, `200` with
- [ ] Legacy `__legacy__` rows visible to operator's default tenant (deployment config var `ARGO_AUDIT_LEGACY_BUCKET_TENANT`, defaults to first-registered tenant)
- [ ] Integration test: 2-tenant fixture, verify row scoping at the SQL level
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] All touched files < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-enterprise-cross-tenant-audit`
- PR title: `audit: cross-tenant isolation + tenant_id column (mission-enterprise-cross-tenant-audit)`

## Notes for the implementer

- Migration slot 64 follows the slot-63 audit-fts brief. If 63 hasn't landed yet, your migration won't conflict (different columns) — just don't change slot 63's shape.
- The `__legacy__` bucket is the most subtle correctness gotcha. Document it as a *deployment-time decision*: operators who run a multi-tenant deployment need to migrate `__legacy__` rows to real tenant_ids before the bucket exits the default-tenant scope. Ship a `argo audit migrate-legacy-tenant <from-tenant> <to-tenant>` CLI as a follow-up brief.
- The `404 vs 403` distinction on cross-tenant reads is *intentional* and load-bearing. 403 leaks "this row exists in some tenant". 404 with an empty page is what an enterprise auditor expects. The 403 path is reserved for the explicit `?as_tenant=` admin probe.
- The `tenant_id` is denormalized into the new audit-fts columns (slot 63 brief) but not part of the chain hash. Anyone tempted to "fix" this by hashing `tenant_id` should know: doing so retroactively requires re-hashing every historical row, which breaks every external bundle export — see the cross-system jump-links brief (`E5-h`) for downstream consumers that rely on the hashes being stable.
- Bearer-token → tenant resolution lives in `argo-rbac::TenantStore::tenant_for_operator`. Don't add a parallel resolution path in the gateway; reuse.
- Test the empty-page response shape carefully: a cross-tenant request returning `{ items: [], next_cursor: null }` should be indistinguishable from a legitimate empty query, including timing (constant-time-ish — log the same audit row in both cases).
