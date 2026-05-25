# fix-pr-2317-bump-expected-schema-version-to-83 — bump `EXPECTED_SCHEMA_VERSION` from 82 to 83 to match v83 migration landed by #2317

**Owner**: claude or codex. **Depends on**: nothing (post-#2317 cleanup).
**Authored against**: origin/main `b4f396f2c` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-pr-2317-bump-expected-schema-version-to-83`.

## Why this is P0 (gateway 503 right now)

PR #2317 (`fix(gateway): deny legacy audit rows by default`, commit `dc6d224eb`, merged 2026-05-25T15:04:51Z) added migration `v83.rs` and registered it via `registry_compliance.rs`, but did **not** bump `EXPECTED_SCHEMA_VERSION` from 82 to 83 in `crates/argo-store/src/lib.rs:364`. Source comment on that line says "bump on every new vNN".

Live impact on `origin/main` right now (verified 2026-05-25T17:11Z):

- `curl http://127.0.0.1:7400/health` → `HTTP 503` body `{"status":"degraded","reason":"schema_version_mismatch"}`. The check-gateway-freshness.sh script reports STATUS: UNREACHABLE (false negative — the gateway is up, /health is just 503).
- `cargo test -p argo-store --test migration_upgrade v22_populated_db_upgrades_to_tip` → **1/1 FAILED** with `assertion left == right failed: post-upgrade schema_version != EXPECTED_SCHEMA_VERSION · left: 83 · right: 82`.
- `cargo test -p argo-store --lib migration_reconciliation` → **6/7 FAILED** (only `apply_migrations_idempotent` passes; all 6 reconciliation scenarios fail with same `left: 83, right: 82`).
- Round-67 evaluator finding S1 P1 documents this in `docs/skycot/evals/scheduled/2026-05-25/round-67-pr-2317-bump-expected-schema-version-post-merge.md`.

Total: 7 broken tests on main right now, plus the live degraded-503 signal blocking SPA workers that need the gateway up (e.g. `mission-genesis-spa-page-describe-and-watch` depends on `/api/v1/genesis` SSE).

## Scope

Pure 1-line constant bump.

- `crates/argo-store/src/lib.rs:364` — change `pub const EXPECTED_SCHEMA_VERSION: u32 = 82;` to `pub const EXPECTED_SCHEMA_VERSION: u32 = 83;`.

No test fixtures need updating — all 9 historic assertion sites in `lib_tests.rs` / `approvals_tests.rs` / `migration_reconciliation_tests.rs` already use the `EXPECTED_SCHEMA_VERSION` constant (refreshed back at #1551 + #1918). The bump cascades automatically.

After the bump, the 7 failing tests should turn green. Verify by running the cargo-test set listed in §Acceptance.

**Not in scope**: changing v83 migration logic, adding a new migration, or refactoring the schema-version check itself.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/src/lib.rs` — single 1-line change at `:364`.

## Files you must NOT touch

- Any `migrations/vN.rs` file — v83 migration is correct as merged in #2317.
- Any `*_tests.rs` file — assertions are already constant-based, no fixture refresh needed.

## Acceptance criteria

- [ ] `crates/argo-store/src/lib.rs:364` reads `pub const EXPECTED_SCHEMA_VERSION: u32 = 83;`.
- [ ] `cargo test -p argo-store --test migration_upgrade v22_populated_db_upgrades_to_tip` → 1/1 PASS.
- [ ] `cargo test -p argo-store --lib migration_reconciliation` → 7/7 PASS.
- [ ] `cargo clippy -p argo-store --all-targets -- -D warnings` clean.
- [ ] `cargo check -p argo-cli` clean (gateway binary depends on argo-store; sanity check).
- [ ] BUILD_STATUS.md updated with PR number on flip to IN_REVIEW.

## Operator perms / overnight authorization

This brief falls under the overnight `fix-pr-22*` --skip-clippy authorization (operator memory `feedback_overnight_genesis_track_p_merge_authorization_2026_05_25`, window expires 2026-05-26T22:00Z AEST). Conductor may admin-merge with `bash scripts/conductor-pre-merge-gate.sh --skip-clippy` once PR opens CLEAN.

## Branch + PR

- Branch: `claude-fix-pr-2317-bump-expected-schema-version-to-83` or `codex-fix-pr-2317-bump-expected-schema-version-to-83`.
- PR title: `store: bump EXPECTED_SCHEMA_VERSION 82 → 83 to match #2317 v83 migration (fix-pr-2317-bump-expected-schema-version-to-83)`.

## Notes for the implementer

- This is the smallest brief in the queue. 1 edit + 2 cargo test runs + 1 cargo clippy run. Should ship in ~5 min.
- Reference: prior pattern of constant-only bumps after migration landings — see `#1551` (v60 fixture refresh) and `#1918` (v76 assertion refresh). Both shipped same-tick.
- Once this lands, the live gateway `/health` will flip 503 → 200 on next restart; current process keeps stale binary so a `bash scripts/restart-live-gateway.sh` follow-up may be needed (operator-side, not part of this brief).
