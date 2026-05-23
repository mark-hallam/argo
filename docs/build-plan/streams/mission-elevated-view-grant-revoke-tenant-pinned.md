# mission-elevated-view-grant-revoke-tenant-pinned

**Owner**: claude.
**Depends on**: nothing.
**Authored against**: origin/main `3df52142a` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-elevated-view-grant-revoke-tenant-pinned`.

🟡 **P1 LATENT-LEAK.** `elevated_view_grant.rs:154` runs
`UPDATE elevated_view_grant SET revoked_at = ? WHERE id = ? AND
revoked_at IS NULL` — no `tenant_id` bind. UUID-unique grant id covers
practical attack, but the table carries `tenant_id` and the SQL
should bind it.

## Why

PR #2116 §4 finding 2 named this brief. The
re-verification pass in
`eval-argo-store-tenant-bind-sweep-remaining-tables` §3.2 row 2
confirms the leak is still present at `3df52142a`.

## Scope

- Add `tenant_id: TenantId` parameter to
  `elevated_view_grant::revoke`.
- WHERE: `WHERE id = ? AND tenant_id = ? AND revoked_at IS NULL`.
- Thread tenant through callers:
  - `crates/argo-cli/src/handlers/gateway/routes_elevated_view.rs`
  - `crates/argo-cli/src/handlers/gateway/routes_dispatch_post_audit.rs`
  - `crates/argo-redact/src/grants.rs`
- Cross-tenant negative test.
- ~120-200 LOC.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/src/elevated_view_grant.rs`
- `crates/argo-store/tests/` (new pin test)
- `crates/argo-cli/src/handlers/gateway/routes_elevated_view.rs`
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post_audit.rs`
- `crates/argo-redact/src/grants.rs`
- `docs/build-plan/BUILD_STATUS.md`

## Acceptance criteria

- [ ] `revoke` rejects cross-tenant calls at SQL.
- [ ] Pin test passes.
- [ ] `cargo clippy -p argo-store -p argo-cli -p argo-redact --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
