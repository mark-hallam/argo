# argo-store-migration-assertion-refresh-v76 — refresh 7 hardcoded `== 74` test assertions + bump `EXPECTED_SCHEMA_VERSION` to 76 after PR #1906 lands v76

**Owner**: claude. **Depends on**: PR #1906 (`web-gateway-endpoints-notifications-batch`) merged to `origin/main`.
**Authored against**: origin/main `ad28379f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh argo-store-migration-assertion-refresh-v76`.

## Scope

Pure assertion + constant refresh. PR #1906 registers migration `v76` (`crates/argo-store/migrations/124__notifications.sql` + `crates/argo-store/src/migrations/v76.rs`) but leaves `pub const EXPECTED_SCHEMA_VERSION: u32 = 75` in `crates/argo-store/src/lib.rs:338` unchanged, and the 6 pre-existing `argo-store` test failures (called out as a HARD RULE 2 boundary note on #1906) still hardcode `assert_eq!(store.schema_version().await.unwrap(), 74)`. Once #1906 merges, both surfaces are stale: the constant lags the registered head, and the tests assert two versions behind it.

Before the fix on post-merge `origin/main`, `cargo test -p argo-store --lib` produces 7 failures — the 6 listed in the #1906 description plus `partial_legacy_19_collision_pair_migrates_to_full_v40` (which asserts `== 74` at the same drift level). The 7th was missed in the #1906 boundary note; this brief covers it too so the refresh is complete.

Failing tests + assertion sites:
- `tests::open_and_migrate` — `crates/argo-store/src/lib_tests.rs:11`
- `migration_reconciliation_tests::fresh_migrate_reaches_v40_and_creates_reconciled_schema` — `:15`
- `migration_reconciliation_tests::raw_v12_to_v29_legacy_schema_migrates_without_duplicate_errors` — `:122`
- `migration_reconciliation_tests::partial_legacy_19_collision_pair_migrates_to_full_v40` — `:179` (missed by the #1906 boundary note; same drift)
- `migration_reconciliation_tests::partial_legacy_28_collision_trio_migrates_to_full_v40` — `:228`
- `migration_reconciliation_tests::rollback_to_v16_drops_v17_through_v40_tables` — `:253` (start-version assertion only; the rollback target on `:257` stays at 16)
- `migration_reconciliation_tests::rollback_to_v22_drops_v23_through_v40_tables` — `:330` (start-version assertion only; the rollback target on `:334` stays at 22)

The fix is two-step and mechanical:

1. **Bump `EXPECTED_SCHEMA_VERSION`** in `crates/argo-store/src/lib.rs:338` from `75` to `76`. This is the binary's declared head-of-schema and is already consulted by `store_open.rs:279` (refuse to bind when on-disk diverges), `routes_health.rs` (readiness gate), `routes_openmetrics.rs:173` (metric), `boot_serve.rs:333` (gauge), and the `tests/migration_upgrade.rs` integration test (`:38`, `:77`). Leaving it at 75 after #1906 lands would make the readiness check reject a freshly-migrated DB.

2. **Refresh the 7 hardcoded `== 74` assertion sites** to use `EXPECTED_SCHEMA_VERSION` (the canonical constant). Add `use argo_store::EXPECTED_SCHEMA_VERSION;` to `migration_reconciliation_tests.rs` (a `#[path = "..."] mod` inside `lib.rs`, so prefer `use crate::EXPECTED_SCHEMA_VERSION;`); `lib_tests.rs` already does `use super::*;` so `EXPECTED_SCHEMA_VERSION` is already in scope.

The constant-import pattern is preferred over a bare `76` so the next migration only needs a one-line bump in `lib.rs`, not a chase across 7 assertion sites. Precedent: `crates/argo-store/tests/migration_upgrade.rs:38` already follows this pattern; `mission-store-schema-version-fixture-refresh` (the v59→v60 refresh, 2026-05-13) deferred the same simplification — this brief finishes it.

**Not in scope**: changing migration logic, adding new migrations, touching `notifications.rs` or `v76.rs` from #1906, refactoring the rollback targets, or unrelated test fixtures. This brief only refreshes 1 constant + 7 assertion sites + 1 import line.

## Pre-flight checks (run before authoring code)

- Confirm PR #1906 has merged to `origin/main`: `git log --oneline origin/main | grep -i "notifications.*\(#1906\)" | head -3`. If not merged, this brief is blocked — do **not** claim.
- Confirm `crates/argo-store/src/migrations/v76.rs` exists on `origin/main` and `registry_compliance.rs` registers `version: 76`.
- Confirm `EXPECTED_SCHEMA_VERSION` is still `75` on post-merge `origin/main` (i.e. #1906 did not retroactively bump it).
- Reproduce the 7 failures locally: `cargo test -p argo-store --lib 2>&1 | grep -E "FAILED|test result" | head -20` — expect 7 schema-version failures.

If `EXPECTED_SCHEMA_VERSION` is already `76` when the brief is claimed (someone else bumped it first), drop step 1 and only update the test assertions; if a separate `current_version()` public helper has been introduced, prefer that over the constant.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/src/lib.rs` — exactly 1 line: `pub const EXPECTED_SCHEMA_VERSION: u32 = 75;` → `= 76;` at `:338`.
- `crates/argo-store/src/lib_tests.rs` — 1 assertion at `:11`. `EXPECTED_SCHEMA_VERSION` already in scope via `use super::*;`.
- `crates/argo-store/src/migration_reconciliation_tests.rs` — 6 assertions at `:15`, `:122`, `:179`, `:228`, `:253`, `:330`. Add `use crate::EXPECTED_SCHEMA_VERSION;` (this file is included via `#[path = "migration_reconciliation_tests.rs"] mod ...` from `lib.rs:344`, so `crate::` resolves correctly).

## Files you must NOT touch

- Any `crates/argo-store/src/migrations/vN.rs` file — migration logic is correct; only the test assertions are stale.
- `crates/argo-store/migrations/124__notifications.sql` or `v76.rs` — owned by #1906; do not modify.
- `crates/argo-store/src/notifications.rs` or `notifications_tests.rs` — owned by #1906.
- `crates/argo-store/tests/migration_upgrade.rs` — already uses `EXPECTED_SCHEMA_VERSION`; bumping the constant in step 1 is sufficient.
- Rollback-target assertions inside `rollback_to_v16_*` (line 257: `… == 16`) and `rollback_to_v22_*` (line 334: `… == 22`) — those test the rollback floor and stay put.
- Any other assertion testing a mid-migration version (e.g. `migration_reconciliation_tests.rs:118` asserts `== 11` for the legacy v12 baseline raw apply — leave alone).
- `crates/argo-cli/src/handlers/gateway/*` — health/readiness tests already use `EXPECTED_SCHEMA_VERSION` correctly; the constant bump in step 1 propagates without touching them.

## Acceptance criteria

- [ ] `EXPECTED_SCHEMA_VERSION` bumped from 75 → 76 in `crates/argo-store/src/lib.rs:338`.
- [ ] All 7 `assert_eq!(store.schema_version().await.unwrap(), 74)` sites updated to `assert_eq!(store.schema_version().await.unwrap(), EXPECTED_SCHEMA_VERSION)` (or `EXPECTED_SCHEMA_VERSION as u32` if a coercion warning appears — but `EXPECTED_SCHEMA_VERSION` is already `u32`, so no cast needed).
- [ ] `cargo test -p argo-store --lib` — all 6 listed tests + the 7th (`partial_legacy_19_collision_pair_migrates_to_full_v40`) pass; no regressions elsewhere in the crate.
- [ ] `cargo test -p argo-store --test migration_upgrade` — the v22-to-tip upgrade test still passes (it auto-targets the bumped `EXPECTED_SCHEMA_VERSION`).
- [ ] `cargo test -p argo-cli --lib routes_health` — the readiness gate still passes (it checks `EXPECTED_SCHEMA_VERSION` matches the binary's expected head).
- [ ] `cargo clippy -p argo-store --all-targets -- -D warnings` clean.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean (the gateway crate also imports `EXPECTED_SCHEMA_VERSION`; double-check no caller hardcodes `75`).
- [ ] No new test added; no migration changed; no production code touched beyond the single-line constant bump.
- [ ] Every modified file < 500 LOC (all 3 should be well under; `lib.rs` and `migration_reconciliation_tests.rs` are the largest — check pre-edit LOC, the edits add at most 1 import line).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-argo-store-migration-assertion-refresh-v76`
- PR title: `argo-store(tests): refresh hardcoded schema_version 74 -> EXPECTED_SCHEMA_VERSION (=76) (argo-store-migration-assertion-refresh-v76)`

## Notes for the implementer

- This is among the smallest claude briefs in the queue. 1 constant edit + 1 import addition + 7 assertion-site edits + 1 cargo test run + 1 cargo clippy run. Should ship in ~10 min once #1906 is merged.
- Prefer the import-the-constant pattern over a hardcoded `76`. The constant becomes the single source of truth and the next migration just bumps `lib.rs:338` — no chase across test files.
- After the edits, run the 7 failing tests individually to confirm each passes:
  ```
  cargo test -p argo-store --lib open_and_migrate
  cargo test -p argo-store --lib fresh_migrate_reaches_v40_and_creates_reconciled_schema
  cargo test -p argo-store --lib raw_v12_to_v29_legacy_schema_migrates_without_duplicate_errors
  cargo test -p argo-store --lib partial_legacy_19_collision_pair_migrates_to_full_v40
  cargo test -p argo-store --lib partial_legacy_28_collision_trio_migrates_to_full_v40
  cargo test -p argo-store --lib rollback_to_v16_drops_v17_through_v40_tables
  cargo test -p argo-store --lib rollback_to_v22_drops_v23_through_v40_tables
  ```
- Sanity-check that the `migration_upgrade` integration test (`tests/migration_upgrade.rs`) still passes — it auto-tracks `EXPECTED_SCHEMA_VERSION` and is the canonical "v22 → tip" upgrade gate, so a green run here confirms the v76 migration is actually wired into `all_migrations()` and reachable from a fresh DB.
- This brief is the spiritual sequel to `mission-store-schema-version-fixture-refresh` (v59→v60, 2026-05-13). That brief left the hardcoded literals in place; this one finally swaps them for the canonical constant so the pattern never drifts again.
