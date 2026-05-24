# mission-session-meta-mutators-tenant-pinned

**Owner**: codex.
**Depends on**: nothing.
**Authored against**: origin/main `4312a5ca6` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-session-meta-mutators-tenant-pinned`.

🟡 **P2 LATENT-LEAK** (defense-in-depth). Five session mutators in
`session_meta.rs` plus `store_session_routing::set_routing_policy`
key on `session.id` alone. `SessionId` is UUID-unique, but the
`session` table has `tenant_id` and the SQL should bind it.

## Why

PR #2116 §4 finding 6 named this brief. Re-verified at
`3df52142a` in `eval-argo-store-tenant-bind-sweep-remaining-tables`
§3.2 row 6. Specific SQL statements:

- `session_meta.rs:82`  — `UPDATE session SET title = ? WHERE id = ?`
- `session_meta.rs:96`  — `UPDATE session SET ended_at = ? WHERE id = ?`
- `session_meta.rs:109` — `UPDATE session SET ended_at = NULL WHERE id = ?`
- `session_meta.rs:123` — `UPDATE session SET worker_id = ? WHERE id = ?`
- `session_meta.rs:138` — `DELETE FROM session WHERE id = ?`
- `store_session_routing.rs:31` — `UPDATE session SET routing_policy = ? WHERE id = ?`

## Scope

- Add `tenant_id: TenantId` parameter to each mutator.
- WHERE clauses bind `tenant_id`.
- Thread tenant through callers:
  - `crates/argo-cli/src/handlers/session.rs`
  - `crates/argo-cli/src/handlers/session_routing.rs`
  - `crates/argo-cli/src/handlers/gateway/routes_session_skills_lifecycle.rs`
  - `crates/argo-tui/src/{input_run,actions}.rs`
  - `crates/argo-store/src/session_store.rs` (in-crate re-export at line 247)
- Cross-tenant negative test using a synthetic UUID-clash scenario
  (same pattern as worker-budget-write brief).
- ~250-400 LOC.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/src/session_meta.rs`
- `crates/argo-store/src/store_session_routing.rs`
- `crates/argo-store/src/session_store.rs`
- `crates/argo-store/tests/` (new pin test)
- `crates/argo-cli/src/handlers/session{,_routing}.rs`
- `crates/argo-cli/src/handlers/gateway/routes_session_skills_lifecycle.rs`
- `crates/argo-tui/src/{input_run,actions}.rs`
- `docs/build-plan/BUILD_STATUS.md`

## Acceptance criteria

- [ ] All six mutators reject cross-tenant calls at SQL.
- [ ] Pin test passes.
- [ ] `cargo clippy -p argo-store -p argo-cli -p argo-tui --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
