# mission-support-ticket-update-tenant-pinned

**Owner**: codex.
**Depends on**: nothing.
**Authored against**: origin/main `4312a5ca6` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-support-ticket-update-tenant-pinned`.

🟡 **P1 LATENT-LEAK + doc-comment-admits-the-gap.** `support_query.rs`
ships an UPDATE keyed on `id` only AND a doc-comment saying:

```rust
/// Apply a partial update to a ticket. … Caller is responsible for
/// any RBAC check; the storage layer only enforces schema.
```

The doc-comment is the RBAC contract — promote it to SQL.

## Why

PR #2116 §4 finding 4 named this brief. Re-verified at
`3df52142a` in `eval-argo-store-tenant-bind-sweep-remaining-tables`
§3.2 row 4.

## Scope

- Add `tenant_id: TenantId` parameter to
  `support_query::update_ticket` + `reply_ticket`.
- WHERE: `WHERE id = ? AND tenant_id = ?`.
- Thread tenant through:
  `crates/argo-cli/src/handlers/gateway/routes_support_tickets.rs`.
- Update the doc-comment: remove "Caller is responsible for any
  RBAC check; the storage layer only enforces schema" — that's no
  longer true.
- Cross-tenant negative test.
- ~150-250 LOC.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/src/support_query.rs`
- `crates/argo-store/tests/` (new pin test)
- `crates/argo-cli/src/handlers/gateway/routes_support_tickets.rs`
- `docs/build-plan/BUILD_STATUS.md`

## Acceptance criteria

- [ ] `update_ticket` + `reply_ticket` reject cross-tenant calls at SQL.
- [ ] Doc-comment updated to reflect new storage-layer enforcement.
- [ ] Pin test passes.
- [ ] `cargo clippy -p argo-store -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
