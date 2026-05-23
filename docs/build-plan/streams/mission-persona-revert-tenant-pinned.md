# mission-persona-revert-tenant-pinned

**Owner**: claude.
**Depends on**: nothing.
**Authored against**: origin/main `3df52142a` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-persona-revert-tenant-pinned`.

🟡 **P2 LATENT-LEAK** (defense-in-depth). Two persona helpers in
`persona_revision.rs` key on `worker_id` alone (not `tenant_id`):

- `persona_revision.rs:133` —
  `SELECT COALESCE({column}, '') FROM persona WHERE worker_id = ?`
- `persona_revision.rs:148` —
  `UPDATE persona SET {column} = ? WHERE worker_id = ?`

`persona` has a `tenant_id` column. `worker_id` is UUID-unique, so
cross-tenant collision is structurally impossible, but the SQL
should bind `tenant_id` for defense-in-depth.

## Why

PR #2116 §4 finding 7 named this brief. Re-verified at
`3df52142a` in `eval-argo-store-tenant-bind-sweep-remaining-tables`
§3.2 row 7.

## Scope

- Add `tenant_id: TenantId` parameter to
  `persona_revision::get_column_for_revert` (the SELECT) +
  `apply_revert` (the UPDATE).
- WHERE: `WHERE worker_id = ? AND tenant_id = ?`.
- Thread tenant through callers:
  - `crates/argo-cli/src/handlers/persona_revert.rs`
  - `crates/argo-cli/src/handlers/persona_history.rs`
- Cross-tenant pin test.
- ~100-150 LOC.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/src/persona_revision.rs`
- `crates/argo-store/tests/` (new pin test)
- `crates/argo-cli/src/handlers/persona_revert.rs`
- `crates/argo-cli/src/handlers/persona_history.rs`
- `docs/build-plan/BUILD_STATUS.md`

## Acceptance criteria

- [ ] Both helpers reject cross-tenant calls at SQL.
- [ ] Pin test passes.
- [ ] `cargo clippy -p argo-store -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
