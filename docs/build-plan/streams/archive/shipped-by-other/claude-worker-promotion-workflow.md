# V1.1 — Worker level + promotion workflow CLI

**Owner**: claude. **Depends on**: claude-worker-kpi-tracker MERGED,
claude-worker-success-metrics-vis MERGED.

## Why

Mission claim 2 implies workers are **employees**, not commodities —
there's a tenure arc, not just spawn-and-discard. Today every worker
is at the same conceptual rank. Operators have no native way to
say "this AR clerk has earned senior status" or to gate riskier
auto-approval policies behind seniority.

## Scope

- Migration: add `level: TEXT NOT NULL DEFAULT 'entry'` column to
  `worker` table (or whatever the canonical worker table is called —
  see migration 1 to confirm). Levels: `entry|junior|senior|lead`.
- `Worker` struct in `argo-identity/src/types.rs:384` gains a `level:
  WorkerLevel` enum field. **This is the one out-of-allowlist edit
  this brief needs**; the whole point is the level-on-Worker.
- Promotion criteria gate (configurable defaults):
  - entry → junior: 25 sessions + 80% success rate + 14d tenure
  - junior → senior: 100 sessions + 90% success rate + 60d tenure
  - senior → lead: 500 sessions + 92% success rate + 180d tenure +
    operator confirmation flag
- CLI:
  - `argo worker promote <id> [--to <level>] [--force]
    [--reason <text>]`
  - `argo worker level <id>` — show current + criteria gap to next
- Auto-approval downstream consumption deferred to follow-up.

## Approach

1. Migration `41__worker_level.sql` — `ALTER TABLE worker ADD COLUMN
   level TEXT NOT NULL DEFAULT 'entry'` + index on level.
2. `argo-identity/src/types.rs` — narrowly: add `WorkerLevel` enum +
   field on `Worker` struct + `Default` impl returning `Entry`.
3. `crates/argo-cli/src/handlers/worker_promote.rs` (NEW, ≤ 380 LOC) —
   eligibility check + transition + audit row.
4. Promotion writes a row to existing `audit_event` table
   (`kind = 'worker_promotion'`).

## Files you may touch

- BLANKET CLI ALLOWLIST applies
- EXHAUSTIVE-MATCH amendment applies
- `crates/argo-store/migrations/41__worker_level.sql` (NEW)
- `crates/argo-identity/src/types.rs` — narrowly: add WorkerLevel
  enum + field. THIS is the explicit allowlist exception.
- `crates/argo-cli/src/handlers/worker_promote.rs` (NEW)
- `crates/argo-cli/src/cli_worker_cmds.rs` — `Promote` + `Level`
  variants
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 pub mod line

## Files you must NOT touch

- auto-approval engine — downstream consumption is a follow-up
- TUI — out of scope
- worker creation flow — defaults to `entry`, that's it

## Acceptance criteria

- [ ] `argo worker level <id>` shows current level + numeric gap to
      next (e.g. "23 sessions short, 4d tenure short")
- [ ] `argo worker promote <id> --to junior` succeeds when criteria
      met, writes audit row, updates worker.level
- [ ] `argo worker promote --to senior` blocked when criteria miss;
      `--force` overrides with audit `force_reason` recorded
- [ ] `--to lead` requires `--force` always (operator confirm)
- [ ] WorkerLevel enum implements `FromStr` + `Display` for CLI parse
- [ ] 10 unit tests (each level transition, force path, gap calc)
      + 2 CLI smoke
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-worker-promotion-workflow`
- PR title: `worker: level + promotion workflow with criteria gates (claude-worker-promotion-workflow)`

## Notes

- The `argo-identity` types.rs touch is narrow — strictly adding the
  enum + one field. Anything else in that file is out of bounds.
- Default level is `entry`; existing rows get `entry` via the migration
  default. No backfill needed.
- Criteria thresholds live as `pub const` in `worker_promote.rs` so
  follow-up brief can env-override them.
