# mission-approval-bundle-decide-tenant-pinned

**Owner**: codex.
**Depends on**: nothing.
**Authored against**: origin/main `4312a5ca6` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-approval-bundle-decide-tenant-pinned`.

🟡 **P1 LATENT-LEAK.** `approval_bundle::decide_bundle` keys
`UPDATE approval_bundle SET status = ?, decided_at = ?, decided_by = ?,
decision = ? WHERE id = ?` on `id` alone — analogue of the
`decide_request` fix in #2146.

## Why

PR #2116 §4 finding 1(d) explicitly names "the analogous change on
`approval_bundle::decide_bundle` (same SQL shape, same caller
trust-boundary today)." PR #2146 did not include this change.
`approval_bundle` is on the same data-shape as `approval_request`.
Found in
`eval-argo-store-tenant-bind-sweep-remaining-tables` §3.2 row 1c.

## Scope

- Add `tenant_id: TenantId` parameter to
  `approval_bundle::decide_bundle`.
- WHERE clause:
  `WHERE id = ? AND tenant_id = ? AND status = 'pending'`.
- Thread tenant through the route caller:
  `crates/argo-cli/src/handlers/gateway/routes_approval_bundles.rs`.
- Add a `cross_tenant_decide_bundle_rejects` pin test.
- Also consider: `approval_bundle_member` SELECT at line 297 keys on
  `bundle_id`; caller-trust today. Document but defer fix unless this
  brief naturally subsumes it.
- ~120-200 LOC.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/src/approval_bundle.rs`
- `crates/argo-store/tests/approval_bundle_tenant_bind_pin.rs` (new)
- `crates/argo-cli/src/handlers/gateway/routes_approval_bundles.rs`
- `docs/build-plan/BUILD_STATUS.md`

## Acceptance criteria

- [ ] `decide_bundle` rejects cross-tenant calls at SQL.
- [ ] Pin test passes.
- [ ] `cargo clippy -p argo-store -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
