# mission-scim-user-update-tenant-pinned

**Owner**: claude.
**Depends on**: nothing.
**Authored against**: origin/main `3df52142a` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-scim-user-update-tenant-pinned`.

🟡 **P1 LATENT-LEAK.** `scim_user.rs:226,271` runs
`UPDATE scim_user SET … WHERE id = ?` (patch + deactivate) — no
`tenant_id` bind. SCIM ids are issued by upstream IDPs and globally
unique within an IDP, but cross-IDP collisions are not structurally
prevented.

## Why

PR #2116 §4 finding 3 named this brief. Production callers exist
(SCIM webhook routes), so the trust boundary is upstream-IDP
trust today. Re-verified at `3df52142a` in
`eval-argo-store-tenant-bind-sweep-remaining-tables` §3.2 row 3.

## Scope

- Add `tenant_id: TenantId` parameter to
  `scim_user::patch_user` + `scim_user::deactivate_user`.
- WHERE: `WHERE id = ? AND tenant_id = ?`.
- Thread tenant through routes:
  - `crates/argo-cli/src/handlers/gateway/routes_scim_users_envelope.rs`
  - `crates/argo-cli/src/handlers/gateway/routes_scim_users.rs`
- Cross-tenant negative test (two tenants, two SCIM users with
  colliding IDP ids, patch tenant_b's user from tenant_a context →
  expect no rows updated).
- ~150-250 LOC.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/src/scim_user.rs`
- `crates/argo-store/tests/` (new pin test)
- `crates/argo-cli/src/handlers/gateway/routes_scim_users{,_envelope}.rs`
- `docs/build-plan/BUILD_STATUS.md`

## Acceptance criteria

- [ ] `patch_user` + `deactivate_user` reject cross-tenant calls at SQL.
- [ ] Pin test passes.
- [ ] `cargo clippy -p argo-store -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
