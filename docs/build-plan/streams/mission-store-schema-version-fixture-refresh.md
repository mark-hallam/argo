# mission-store-schema-version-fixture-refresh — refresh 9 hardcoded `== 59` test assertions to match `EXPECTED_SCHEMA_VERSION = 60`

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `8d54a401` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-store-schema-version-fixture-refresh`.

## Scope

Pure test-fixture refresh. `crates/argo-store/src/lib.rs:226` declares `EXPECTED_SCHEMA_VERSION: u32 = 60`, but 9 test assertions across 3 files still hardcode `== 59`. On clean `origin/main`, `cargo test -p argo-store --lib` produces 8 test failures (the 9 sites collapse to 8 distinct test functions). Surfaced as a HARD RULE 2 boundary note on the PR #1543 (mission-prompt-cache-affinity-wire-mints) but not addressed.

Failing tests:
- `tests::open_and_migrate` (`lib_tests.rs:11`)
- `approvals::tests::down_migration_drops_table` (`approvals_tests.rs:58`, `:73`)
- `migration_reconciliation_tests::fresh_migrate_reaches_v40_and_creates_reconciled_schema` (`:15`)
- `migration_reconciliation_tests::partial_legacy_19_collision_pair_migrates_to_full_v40` (`:122`)
- `migration_reconciliation_tests::partial_legacy_28_collision_trio_migrates_to_full_v40` (`:183`)
- `migration_reconciliation_tests::raw_v12_to_v29_legacy_schema_migrates_without_duplicate_errors` (`:236`)
- `migration_reconciliation_tests::rollback_to_v16_drops_v17_through_v40_tables` (`:261`)
- `migration_reconciliation_tests::rollback_to_v22_drops_v23_through_v40_tables` (`:338`)

The fix is mechanical: change each `assert_eq!(store.schema_version().await.unwrap(), 59)` to `assert_eq!(store.schema_version().await.unwrap(), 60)` (or import `EXPECTED_SCHEMA_VERSION` and assert against the constant — implementer's call; the constant import is slightly preferred for future-proofing but adds an import line).

For the rollback tests (`rollback_to_v16_drops_v17_through_v40_tables`, `rollback_to_v22_drops_v23_through_v40_tables`): the rollback target stays at v16/v22 respectively (the brief title is about the rollback start, which is the most recent version → 60). Update only the start-version assertion, not the rollback target.

**Not in scope**: changing the rollback semantics, adding new migrations, changing `EXPECTED_SCHEMA_VERSION` itself. This brief just refreshes the test fixtures to match what's already on main.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/src/lib_tests.rs` — narrowly scoped: 1 site at `:11`.
- `crates/argo-store/src/approvals_tests.rs` — narrowly scoped: 2 sites at `:58` + `:73`.
- `crates/argo-store/src/migration_reconciliation_tests.rs` — narrowly scoped: 6 sites at `:15` + `:122` + `:183` + `:236` + `:261` + `:338`.

## Files you must NOT touch

- `crates/argo-store/src/lib.rs` — `EXPECTED_SCHEMA_VERSION` is correct at 60.
- Any `migrations/vN.rs` file — migration logic is correct; only the test assertions are stale.
- Any test that asserts `schema_version() == <other-than-59>` — those are testing rollback targets and stay put.

## Acceptance criteria

- [ ] All 9 `== 59` assertions updated to `== 60` (or `== EXPECTED_SCHEMA_VERSION`).
- [ ] `cargo test -p argo-store --lib` produces 0 schema-version failures (the 8 listed tests all pass).
- [ ] `cargo clippy -p argo-store --all-targets -- -D warnings` clean.
- [ ] No new test added; no migration changed; no production code touched.
- [ ] Every modified file < 500 LOC (all 3 should be well under).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-store-schema-version-fixture-refresh`
- PR title: `store(tests): refresh hardcoded schema_version 59 -> 60 (mission-store-schema-version-fixture-refresh)`

## Notes for the implementer

- This is the smallest claude brief in the queue. 9 sed-style edits + 1 cargo test run + 1 cargo clippy run. Should ship in ~10 min.
- Future-proofing: prefer `use argo_store::EXPECTED_SCHEMA_VERSION;` + `assert_eq!(..., EXPECTED_SCHEMA_VERSION as u32)` over a hardcoded `60`. But not required by acceptance.
- Run the 8 failing tests individually after the edit to confirm each passes: `cargo test -p argo-store --lib <test_name>`.
- The `migration_reconciliation_tests` file is the biggest — make sure to update only the 6 cited line numbers (not other assertions that may test mid-migration versions).
