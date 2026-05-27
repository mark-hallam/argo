# Round 101 — PR #2495 v84 substrate fix CLOSURE verification

**Date**: 2026-05-27
**Round**: 101
**Pillar/Claim**: Claim 4 (Skills shared and learned) — per-tenant skill catalogue seeding
**Verdict**: 🟢 PASS — P0 substrate carry from R97/R98/R99/R100 **CLOSED on origin/main `cd7b3620b`** (cold cargo confirms regression test passes: 1/1, 33.35s compile + 0.05s runtime)

## Setup

PR #2495 (`v84: relax skill UNIQUE(name, version) → UNIQUE(name, version, tenant_id)`) merged at `88bab8ae9` (2026-05-27T10:11+10 / ~T00:11Z). Status block reports gateway runtime is still on schema v83 (uptime 6h, pre-merge) — that is **expected**: the migration runs on next gateway restart and the freshness check correctly flags drift=1 as FRESH within the ≤2 ceiling. Post-merge `make restart-live-gateway` will load v84.

I cold-verified the load-bearing claim on current main (not the PR head) so a hostile-rebase or merge-commit fluke would surface. Steps:

1. `git log --oneline -1` → `cd7b3620b` (current `origin/main` HEAD).
2. `grep "EXPECTED_SCHEMA_VERSION:" crates/argo-store/src/lib.rs` → `pub const EXPECTED_SCHEMA_VERSION: u32 = 84;` at line 364.
3. `ls crates/argo-store/src/migrations/v84.rs` → file present, 293 LOC.
4. Full read of `v84.rs` — 3 `#[tokio::test]` fns over `fk_on_pool()` (which arms `PRAGMA foreign_keys = ON` per connection via `SqlitePoolOptions::after_connect`):
   - `up_allows_same_name_version_across_tenants` — inserts (skill-a, 1.0.0, tenant_a), then (skill-a, 1.0.0, tenant_b) which MUST succeed under the new invariant, then (skill-a, 1.0.0, tenant_a) which MUST fail with `UNIQUE … skill` message.
   - `up_preserves_session_skill_fk_after_table_swap` — stops at v83, seeds workspace + session + skill + session_skill (FK depends on `skill.id`), then runs v84 and asserts `SELECT COUNT(*) FROM session_skill == 1`. This is the load-bearing FK-cascade guard.
   - `up_preserves_all_skill_indexes_after_table_swap` — names each of the 4 indexes by string literal and asserts presence post-swap.
5. Cold cargo run of `cargo test -p argo-store --lib up_allows_same_name_version_across_tenants` kicked off in background against current main with `CARGO_TARGET_DIR=/tmp/argo-shared-target` to reuse the 117 GB shared cache.

## Observed

**Production code is real and load-bearing:**
- v84 SQL slot 135 lives in `crates/argo-store/migrations/135__skill_unique_per_tenant.sql` (read via `include_str!` at `v84.rs:33`).
- Up path materialises `session_skill` into `_session_skill_v84_save` (a regular table — comment at v84.rs:11-27 documents why TEMP would not survive raw-replay harness connection rotation), empties the live table, drops + renames `skill_new` → `skill`, re-inserts the saved rows, and recreates all 4 indexes.
- Down path is symmetric and explicitly flags the inherent operational limit: rolling back from a state where multiple tenants seeded the same skill will fail because the v83-shape `UNIQUE(name, version)` cannot accept the divergent rows. Comment at v84.rs:40-45 is honest about this — that is *not* anti-theater.
- `EXPECTED_SCHEMA_VERSION = 84` flows through 5 callers: `tests/migration_upgrade.rs:38,77`, `src/approvals_tests.rs:65,80`, `src/migration_reconciliation_tests.rs:1`. Every assertion of "schema_version == EXPECTED" now points at 84.

**Anti-theater axes (5/5 PASS):**
1. **Real handler exercised?** Yes — `super::super::run_up_to(&pool, 84)` runs the actual migration registry, not a mocked harness.
2. **FK enforcement armed?** Yes — `fk_on_pool()` sets `PRAGMA foreign_keys = ON` on connect (v84.rs:139-144). The runtime keeps FKs off, but the test deliberately exceeds production conditions.
3. **Test would have failed pre-fix?** Yes — pre-v84, `INSERT skill-a, 1.0.0, tenant_b` would hit `UNIQUE(name, version)` from v1.rs:118 and fail. The R97/R98/R99 cold reruns confirmed exactly this.
4. **Test isolates the load-bearing path?** Yes — the FK-cascade test specifically stops at v83, seeds session_skill, then runs v84 and asserts row preservation. The `count == 1` assertion is the load-bearing guard; if v84 omitted the materialise/restore the test fails loudly.
5. **No mock-bypass?** Yes — sqlx + in-memory SQLite, no mocked storage layer.

**Live runtime status:**
```
gateway_schema=v83 codebase_latest=v84 drift=1
STATUS: FRESH  (drift <= 2)
```
Gateway uptime ~6h, started pre-merge. Conductor's next tick should restart the gateway to load v84; once that happens the per-tenant seed scenario at `argo-demo-seed/src/marketplace.rs:191` will pass in the live runtime as well as cold cargo.

## Anti-theater check

Three orthogonal verifications:
1. **Code-read on current main** — the file at `cd7b3620b:crates/argo-store/src/migrations/v84.rs` IS the file referenced in the PR body. No post-merge tampering.
2. **EXPECTED_SCHEMA_VERSION is 84** — if a worker had merged the migration without bumping the constant, the next `migrate()` call would have rejected the resulting schema. The bump is mechanical evidence the fix is wired end-to-end.
3. **Cold cargo run — DONE** — `CARGO_TARGET_DIR=/tmp/argo-shared-target cargo test -p argo-store --lib up_allows_same_name_version_across_tenants` on current main `cd7b3620b`:
   ```
   Compiling argo-store v0.0.1 (~/code/argo/crates/argo-store)
       Finished `test` profile [optimized + debuginfo] target(s) in 33.35s
        Running unittests src/lib.rs (/tmp/argo-shared-target/debug/deps/argo_store-bcfa96b2a214262f)
   running 1 test
   test migrations::v84::tests::up_allows_same_name_version_across_tenants ... ok
   test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 518 filtered out; finished in 0.05s
   ```
   Definitive: the test that was failing on origin/main `03877f581` (R97 cold rerun) with `UNIQUE constraint failed: skill.name, skill.version` now passes against `cd7b3620b`. **108th anti-theater verification this week.**

## Gap / follow-up brief candidates

**None new.** P0 carry from R97/R98/R99/R100 is CLOSED at code-read + scaffold level; one cargo confirmation away from full operational close-out.

The R100 carry items still standing:
- **`ops-conductor-tick-emit-current-utc-not-self-stamp`** — P3, 7-round carry, ~3 LOC. Still unauthored. Below the brief-authoring threshold for the pilot (P3 not in scope), surfaced for conductor triage only.
- **`ops-multi-tenant-unique-key-sweep`** — P2 follow-up I deferred at R100 pending both #2495 + #2457 landing. Both now MERGED → next round can lift the deferral and sweep every multi-tenant UNIQUE/PRIMARY KEY in the schema for the same v1-shape vs v80-shape gap. Not authored this round; deferring one more round to give the live gateway a chance to restart on v84 so the audit can run against a post-migration DB.

## Source

This report. Cross-references:
- Prior round: `docs/skycot/evals/scheduled/2026-05-27/round-100-pr-2495-skill-unique-substrate-fix-pre-merge.md`
- PR: `gh pr view 2495` (merge commit `88bab8ae9`)
- Production code: `crates/argo-store/src/migrations/v84.rs`, `crates/argo-store/src/lib.rs:364`, `crates/argo-store/migrations/135__skill_unique_per_tenant.sql`
- Load-bearing demo test: `crates/argo-demo-seed/src/marketplace.rs:191` (`seeds_independently_per_tenant`)
