# ops-fix-migration-llm-input-tokens-duplicate-column

**Authored against**: origin/main `9d9f8fd40` on `2026-05-24`.
**Validate before claim**: bash scripts/check-brief-freshness.sh ops-fix-migration-llm-input-tokens-duplicate-column

## Why this brief exists

Surfaced as a HARD RULE 2 boundary note on PR #2146 (`mission-approval-decide-request-tenant-pinned`): `cargo test -p argo-store` fails one test on clean `origin/main`:

```
migration_chain_applies_and_h_stack_tables_accept_inserts:
  duplicate column `llm_input_tokens` between
  crates/argo-store/migrations/53__forge_run_cost.sql:17 and
  crates/argo-store/src/migrations/v49.rs:89-90
```

Both migrations issue `ALTER TABLE forge_run ADD COLUMN llm_input_tokens INTEGER NOT NULL DEFAULT 0` — the second one panics on apply because the column already exists. Confirmed pre-existing on origin/main 9d9f8fd40:

```
$ rg -nF 'llm_input_tokens' crates/argo-store/migrations/53__forge_run_cost.sql crates/argo-store/src/migrations/v49.rs
crates/argo-store/migrations/53__forge_run_cost.sql:17:ALTER TABLE forge_run ADD COLUMN llm_input_tokens INTEGER NOT NULL DEFAULT 0;
crates/argo-store/src/migrations/v49.rs:89:        "llm_input_tokens",
crates/argo-store/src/migrations/v49.rs:90:        "ALTER TABLE forge_run ADD COLUMN llm_input_tokens INTEGER NOT NULL DEFAULT 0",
```

This blocks `cargo test -p argo-store` on clean main and surfaces as a HARD RULE 2 boundary note on every PR that touches the crate. Has been carried as a "TODO follow-up" boundary note across at least PR #2146 already.

## Acceptance criteria

1. Decide which migration is canonical (likely `53__forge_run_cost.sql` — slot 53 is the additive-migration registry path per `docs/skycot/operator/migrations.md`).
2. Remove the duplicate ADD from the non-canonical site, OR rewrite it to be idempotent (`ALTER TABLE ... ADD COLUMN ... IF NOT EXISTS` semantics — SQLite needs the `PRAGMA table_info` guard pattern that other v* migrations use).
3. `cargo test -p argo-store --lib --test migration_chain_applies_and_h_stack_tables_accept_inserts` green on a clean DB.
4. `cargo test -p argo-store --lib` 514/0 (the count from PR #2146's verification block).
5. `cargo clippy -p argo-store --all-targets -- -D warnings` clean.
6. Migration slot registry (`crates/argo-store/src/migrations/registry.rs`) still passes `scripts/check-migration-slots.sh`.
7. Cross-tenant tests in `argo-store` still green (the `audit_detail` + `approvals::decide_request` pins from #1992 + #2146 must not regress).

## Parallel-surface declarations

- No new pub surface. Migration-table edit only.
- Migration slot 53 is already used by `forge_run_cost.sql` (don't reassign).

## Investigation hint

`crates/argo-store/src/migrations/v49.rs` may have been authored at the same time as `53__forge_run_cost.sql` by two parallel briefs that both intended to add the `llm_input_tokens` column. Whichever lands later is the duplicate. The fix is to identify which migration registers in `registry.rs` and which is dead code, then delete the dead one. If both register but at different slots, the v49.rs path is more likely the dead one (the file-numbered .sql path is the newer convention per `docs/skycot/operator/migrations.md`).

## May-touch

- `crates/argo-store/src/migrations/v49.rs` (likely deletion of the duplicate ADD COLUMN line)
- `crates/argo-store/migrations/53__forge_run_cost.sql` (keep as canonical)
- `crates/argo-store/src/migrations/registry.rs` (if the dead migration was registered, deregister it)
- `crates/argo-store/tests/migration_chain_*.rs` (test asserts the chain applies cleanly — may need a regression case)

## Must NOT touch

- The migration slot of either file. Slot stability is load-bearing.
- Any other `v*.rs` migration or `*.sql` migration file beyond the duplicate-ADD line.
- The `forge_run` table schema beyond the duplicate-ADD removal.

## LOC budget

~10-30 LOC (small surgical fix).

## Priority

🟡 P1 — Crate test gate failing on clean main is a real reliability problem. Not demo-blocking but every PR touching `argo-store` currently has to footnote this in HARD RULE 2.

## Source

PR #2146 (`mission-approval-decide-request-tenant-pinned`) verification block:

> Boundary note (HARD RULE 2): pre-existing `migration_chain_applies_and_h_stack_tables_accept_inserts` fail (duplicate column `llm_input_tokens` between `crates/argo-store/migrations/53__forge_run_cost.sql` and `crates/argo-store/src/migrations/v49.rs`) — outside may-touch, needs a separate migration-reconciliation brief.
