# mission-skill-share-tenant-pinned

**Owner**: claude.
**Depends on**: nothing.
**Authored against**: origin/main `3df52142a` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-skill-share-tenant-pinned`.

🔴 **P0 LATENT-BUT-LIVE LEAK (post-PR-#2092 sweep gap).**
`skill_share.rs:71-89` runs:

```sql
UPDATE skill SET shared = ?, shared_with_cluster = ?
WHERE name = ? AND version = ?
```

with no `tenant_id` bind. The caller at
`crates/argo-cli/src/handlers/gateway/marketplace.rs:252` is the
**LIVE** `POST /marketplace/<skill_id>/install` remote route.

## Exploit shape

1. Tenant A locally installs a skill named `payroll` v`1.0.0` via the
   tenant-aware `install_for_tenant` path. `skill` row exists with
   `tenant_id = A`.
2. Tenant B finds `payroll` v`1.0.0` in the marketplace catalog and
   POSTs `/marketplace/<id>/install`.
3. The marketplace handler calls
   `set_shared(store, "payroll", "1.0.0", true, Some(B.cluster_name))`.
4. The UPDATE keys on `(name, version)` only, so it matches tenant A's
   row and flips A's `shared_with_cluster` to tenant B's cluster name.

**Cross-tenant data mutation.** The only thing protecting tenant A
today is the sparsity of the `skill` table population (PR #2092
backfilled existing rows with `__migration_placeholder__`).

## Why

Found in `eval-argo-store-tenant-bind-sweep-remaining-tables` §4.3.

## Scope

- Add `tenant_id: TenantId` parameter to `skill_share::set_shared`.
- WHERE: `WHERE name = ? AND version = ? AND tenant_id = ?`.
- If the marketplace install needs to first INSERT a per-tenant skill
  row (rather than UPDATE the global one), refactor `set_shared` to
  upsert by `(tenant_id, name, version)` — verify the data-model
  intent against `migrations/v79` at claim-time.
- Thread tenant through the marketplace handler:
  `crates/argo-cli/src/handlers/gateway/marketplace.rs:252`.
- Cross-tenant pin test:
  - Tenant A installs `payroll` v1.0.0 with tenant_id = A.
  - Tenant B calls `set_shared("payroll", "1.0.0", true, "B-cluster",
    tenant_id = B)`.
  - Tenant A's row's `shared` + `shared_with_cluster` must be
    unchanged.
- ~200-350 LOC.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/src/skill_share.rs`
- `crates/argo-store/tests/skill_share_tenant_bind_pin.rs` (new)
- `crates/argo-cli/src/handlers/gateway/marketplace.rs`
- `crates/argo-cli/src/handlers/gateway/marketplace_render.rs` (if
  the list-side helper also needs tenant signature changes — read
  at claim-time)
- `docs/build-plan/BUILD_STATUS.md`

## Acceptance criteria

- [ ] `set_shared` rejects or no-ops cross-tenant calls at SQL.
- [ ] Pin test passes.
- [ ] `cargo clippy -p argo-store -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
