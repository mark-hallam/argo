# Stream C — migration reconciliation v12..v15 with structural compatibility

**Owner**: codex.
**Depends on**: none.
**Authored against**: origin/main `69b929c8` on `2026-05-08`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-migration-reconciliation-v12-15`.
**MVP lane**: unblocks migration-gated skill sharing/canary work without rewriting feature code.

## Scope

Current main has two migration paths: the versioned runner at
`crates/argo-store/src/migrations/v1.rs` through `v11.rs`, and raw SQL files
under `crates/argo-store/migrations/12__*.sql` onward that lazy
`ensure_schema()` callers apply directly. The old brief assumed those lazy
callers wrote legacy marker rows into `schema_version`; they do not.
`schema_version` is a singleton `{id, version, applied_at}`.

Convert the first raw cluster into versioned modules while making each module
structurally idempotent. The versioned `up()` functions should probe actual
tables/columns/indexes and apply only missing pieces, so they work on fresh DBs
and on DBs where lazy raw SQL already created some or all of the v12..v15
schema. Do not add audit plumbing or marker/digest rows.

Cluster:

| Raw file | Versioned module |
| --- | --- |
| `crates/argo-store/migrations/12__worker_identity.sql` | `crates/argo-store/src/migrations/v12.rs` |
| `crates/argo-store/migrations/13__memory_layers.sql` | `crates/argo-store/src/migrations/v13.rs` |
| `crates/argo-store/migrations/14__worker_budget.sql` | `crates/argo-store/src/migrations/v14.rs` |
| `crates/argo-store/migrations/15__shared_team_memory.sql` | `crates/argo-store/src/migrations/v15.rs` |

## Files you may touch

- `crates/argo-store/src/migrations/mod.rs` — register v12..v15 after v11.
- `crates/argo-store/src/migrations/v12.rs` (NEW) — worker identity schema, idempotent.
- `crates/argo-store/src/migrations/v13.rs` (NEW) — memory layer columns/indexes, idempotent column probes.
- `crates/argo-store/src/migrations/v14.rs` (NEW) — worker budget schema, idempotent.
- `crates/argo-store/src/migrations/v15.rs` (NEW) — shared team memory schema, idempotent.
- `crates/argo-store/src/migration_reconciliation_tests.rs` (NEW) — fresh and legacy/partial schema tests.
- `crates/argo-store/src/lib.rs` — narrowly: test module include if needed.

## Files you must NOT touch

- `crates/argo-store/migrations/12__worker_identity.sql` through `15__shared_team_memory.sql` — keep raw files for lazy ensure callers until the full reconciliation series finishes.
- `crates/argo-store/src/migrations/v1.rs` through `v11.rs`.
- `crates/argo-cli/tests/migration_walk_smoke.rs` — no such current-main test exists; do not invent a CLI dependency for this store-only brief.
- Audit crates or audit migrations — marker/audit semantics were removed from this rewrite.
- Anything not on the may-touch list above.

## Acceptance criteria

- [ ] `v12.rs` through `v15.rs` exist and are registered after v11.
- [ ] Fresh `SessionStore::migrate()` reaches schema version 15 and creates worker/persona/role/duty, memory layer columns/indexes, worker_budget, and shared_team_memory.
- [ ] A DB where raw 12..15 SQL was already applied can run the versioned runner without duplicate-column/table errors.
- [ ] A partial legacy DB, for example worker identity present but memory layer columns absent, migrates to the full v15 schema.
- [ ] `roll_back_to(11)` has defined down paths for v12..v15 or explicitly returns a clear unsupported rollback error for non-droppable legacy-safe pieces; document the chosen behavior in code comments and tests.
- [ ] `schema_version` remains the singleton version row; no digest/marker rows are introduced.
- [ ] `cargo clippy -p argo-store --all-targets -- -D warnings` passes.
- [ ] `cargo test -p argo-store` passes.
- [ ] Every modified file stays under 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `codex-mission-migration-reconciliation-v12-15`
- PR title: `store: reconcile raw migrations v12-v15 structurally (mission-migration-reconciliation-v12-15)`

## Notes for the implementer

- Use helper probes local to the new migration modules or a tiny shared helper in `mod.rs`; do not add a broad migration framework in this PR.
- For `ALTER TABLE ... ADD COLUMN`, always check `PRAGMA table_info(...)` first.
- For table/index DDL, prefer `CREATE TABLE IF NOT EXISTS` and `CREATE INDEX IF NOT EXISTS`, matching the raw SQL files.
- The next reconciliation briefs can copy this pattern for collision-heavy ranges.
