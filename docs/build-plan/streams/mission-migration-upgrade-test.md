# Mission load-bearing wire — v0.x → v1.0 schema upgrade integration test

**Owner**: claude. **Depends on**: existing `migration_chain.rs` test SHIPPED (verified at `crates/argo-store/tests/migration_chain.rs`); versioned migration ladder v1..v46 SHIPPED.
**Authored against**: origin/main `3fe6a7b9` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-migration-upgrade-test`.

> SOURCE: MISSION_BACKLOG.md Section 2 #28 ("Add upgrade test (v0.x → v1.0 schema migration) once stable schema declared"). Today `migration_chain.rs` verifies the chain walks cleanly from empty → tip, but there's no test that exercises a **populated** v0.x DB (real rows from an early schema) being upgraded forward. Operators upgrading existing deployments need confidence that data survives.

## Scope

Add `tests/migration_upgrade.rs` integration test that:

1. **Builds a v22-era populated DB** in a temp dir via:
   - Apply migrations v1..v22 manually (stops at v22 — a stable mid-history snapshot post the reconciliation work but before recent additions).
   - INSERT realistic rows: 5 workers, 3 tenants, 50 audit_records, 20 sessions, 10 skills, 30 memory rows, 5 approval_requests.
   - Close the connection cleanly so the DB file is in a "shipped-and-deployed" state.

2. **Re-opens the DB and migrates to tip (v46+)**:
   - Calls `SessionStore::open(path)` which triggers `ensure_schema` and walks all forward migrations.
   - Asserts no error, no data loss, no constraint violations.

3. **Verifies post-upgrade data integrity**:
   - All 5 worker rows still present with original IDs.
   - audit_chain hash chain still valid (no broken links from column additions).
   - skill rows have null `trust_grade`/`last_used_at` (added in v45/v46 with defaults).
   - approval_requests survived migration reshapes.

4. **Verifies idempotency**: second `open()` call is no-op (no migrations re-applied).

The v22 baseline is intentional — it's the stable "reconciliation done" point per BUILD_PLAN_V11 history. If the schema settles further pre-v1.0 release, the baseline can rebase to a later version, but v22 today gives ~24 forward migrations worth of upgrade exercise.

This is NOT in scope: downgrades (no DOWN migrations supported); cross-engine migration (only SQLite); migrating from pre-v1 raw-SQL-only deployments older than v1.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/tests/migration_upgrade.rs` (NEW, ≤ 400 LOC) — the integration test above.
- `crates/argo-store/tests/fixtures/v22_seed.sql` (NEW, ≤ 200 LOC) — the seed-row inserts. Plain SQL kept separate from Rust for easy auditing.
- `crates/argo-store/Cargo.toml` — narrowly scoped: confirm `tempfile` is already a dev-dep (verify; add if absent).

## Files you must NOT touch

- `crates/argo-store/src/migrations/**` — migrations themselves are upstream; this brief only TESTS them
- `crates/argo-store/tests/migration_chain.rs` — sibling test; reuse helpers via `mod common` if needed but don't modify
- Any non-store crate
- `crates/argo-store/migrations/**` — no new migration

## Acceptance criteria

- [ ] `migration_upgrade.rs` integration test exists
- [ ] Test builds v22 DB with realistic seed data (5 workers, 50 audit_records, etc.)
- [ ] Upgrade to tip succeeds without error
- [ ] All seeded rows survive (count preserved per table)
- [ ] Audit chain hash links still valid after upgrade
- [ ] New columns added in v23+ have defaults (NULL or 0) for old rows
- [ ] Second open() is idempotent (no re-application)
- [ ] Test runs in < 5 seconds on a developer laptop (no real workload)
- [ ] No new public API
- [ ] `cargo test -p argo-store --test migration_upgrade` passes
- [ ] `cargo clippy -p argo-store --all-targets -- -D warnings` clean
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-migration-upgrade-test`
- PR title: `store: v22→tip migration upgrade integration test (mission-migration-upgrade-test)`

## Notes for the implementer

- v22 is the baseline because it's post-reconciliation (migration system unified) and gives broad upgrade exercise. If a future brief settles the v1.0 release schema, this test can be rebased to that version.
- The seed SQL should be hand-written, not generated from production dumps — keep it small and auditable.
- Audit chain verification is load-bearing: a migration that broke prev_hash linkage would be catastrophic in production. The test must call `argo-audit::verify_all` (or equivalent) on the upgraded DB.
- If the seed-time migration manual-apply path is awkward, factor out a small `apply_migrations_up_to(version: u32)` helper into the test file — DON'T add it to the production code path.
- The `< 5s` time budget keeps this test in the default `cargo test` set rather than gated behind a flag.
