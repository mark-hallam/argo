# V1.1 — Worker OKR (objectives + key results) CLI

**Owner**: claude. **Depends on**: claude-worker-kpi-tracker MERGED
(key results reference KPI names).

## Why

Mission claim 2: workers have **goals**, not just charters. KPIs
record observed values; OKRs declare *intent* — what the worker is
trying to achieve in a quarter. Without an OKR layer, "good
performance" has no anchor: the AR clerk's 92% collection rate is
either a win or a miss depending on whether the goal was 90% or 95%.

Quarterly OKRs are the contract between operator and worker.

## Scope

- New `worker_objective(id, worker_id, period, title, description,
  status, created_at)` table. `period` like `2026Q2`. `status` enum:
  `active|achieved|missed|cancelled`.
- New `worker_key_result(id, objective_id, kpi_name, target_value,
  comparison, weight)` table. `comparison` enum: `gte|lte|eq`.
- CLI:
  - `argo worker okr set <worker> --period 2026Q2 --title "..."
    --kr <kpi>:<op>:<target>:<weight> [--kr ...]`
  - `argo worker okr show <worker> [--period 2026Q2]
    [--format markdown|json]`
  - `argo worker okr progress <worker> [--period 2026Q2]` — joins
    against `worker_kpi_value` latest reading per KR
- `progress` returns weighted score 0-100% + per-KR status.

## Approach

1. Migration `39__worker_objective.sql` + `40__worker_key_result.sql`.
2. `crates/argo-store/src/worker_okr.rs` (NEW) — CRUD + progress join
   against `worker_kpi_value` (depends on `claude-worker-kpi-tracker`).
3. `crates/argo-cli/src/handlers/worker_okr.rs` (NEW, ≤ 380 LOC).
4. CLI: nested `Okr { Set, Show, Progress }` under `worker`.

## Files you may touch

- BLANKET CLI ALLOWLIST applies
- EXHAUSTIVE-MATCH amendment applies
- `crates/argo-store/migrations/39__worker_objective.sql` (NEW)
- `crates/argo-store/migrations/40__worker_key_result.sql` (NEW)
- `crates/argo-store/src/worker_okr.rs` (NEW)
- `crates/argo-store/src/lib.rs` — narrowly: pub mod
- `crates/argo-cli/src/handlers/worker_okr.rs` (NEW)
- `crates/argo-cli/src/cli_worker_cmds.rs` — `Okr` variant
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 pub mod line

## Files you must NOT touch

- `worker_kpi.rs` — read-only join, no schema additions
- Worker struct in `argo-identity` — OKRs live in their own table
- TUI — defer to follow-up

## Acceptance criteria

- [ ] `argo worker okr set` parses `--kr kpi:op:target:weight` repeatable
- [ ] `argo worker okr show` markdown lists objectives + nested KRs
- [ ] `argo worker okr progress` joins to latest `worker_kpi_value`
      and emits weighted score + per-KR achieved/missed
- [ ] Period filter `--period 2026Q2` works
- [ ] 10 unit tests (set, show, progress arithmetic, weight=0 rejected,
      empty period returns no rows) + 3 CLI smoke + 1 markdown snapshot
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-worker-okr-cli`
- PR title: `worker: OKR objectives + key-results CLI (claude-worker-okr-cli)`

## Notes

- Period regex: `^(19|20)\d{2}Q[1-4]$`. Reject anything else with a
  clap parser — keeps the data sane.
- Progress = `Σ (weight × kr_achievement) / Σ weight` clamped 0..1.
  KR achievement = 1.0 if comparison passes, else `actual/target`
  (or `target/actual` for `lte`) clamped to [0, 1].
- Markdown output mirrors `argo session messages --markdown` style.
