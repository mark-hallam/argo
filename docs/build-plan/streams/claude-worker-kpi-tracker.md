# V1.1 — Worker KPI metric tracker

**Owner**: claude. **Depends on**: claude-worker-charter-storage (charter_md + kpi_json columns) MERGED.

## Why

Mission claim 2: "A worker has duties, KPIs, and a charter." The charter
storage brief queues `kpi_json` (the schema/definition) but there is no
mechanism to **record** observed KPI values over time. Without
time-series points there is no way to ask "is the AR clerk hitting its
weekly DSO target?" — KPI is just a static label.

This brief adds a per-worker time-series KPI store + CLI to record/query.

## Scope

- New `worker_kpi_value(id, worker_id, kpi_name, value, recorded_at,
  source, note)` table. `value` is `REAL`. `source` distinguishes
  manual entry from auto-derived (later wired to anomaly / success
  surfaces).
- CLI:
  - `argo worker kpi record <id> <kpi> <value> [--note <text>]` —
    insert one point
  - `argo worker kpi list <id> [--kpi <name>] [--since <dur>]` — table
  - `argo worker kpi series <id> <kpi> [--bucket day|week|month]
    [--format markdown|json]` — bucketed avg/min/max/count
- KPI names are NOT validated against `kpi_json` in this brief — that
  cross-check is a follow-up so the two PRs can ship independently.

## Approach

1. Migration (next free `38__worker_kpi_value.sql`): table +
   `(worker_id, kpi_name, recorded_at)` index.
2. `crates/argo-store/src/worker_kpi.rs` (NEW) — `record_kpi`,
   `list_kpi`, `series_kpi` (bucketing via SQLite `strftime`).
3. `crates/argo-cli/src/handlers/worker_kpi.rs` (NEW) — handlers +
   markdown formatter (≤ 320 LOC).
4. `cli_worker_cmds.rs` — `Kpi { Record, List, Series }` subenum.

## Files you may touch

- BLANKET CLI ALLOWLIST applies
- EXHAUSTIVE-MATCH amendment applies
- `crates/argo-store/migrations/38__worker_kpi_value.sql` (NEW)
- `crates/argo-store/src/worker_kpi.rs` (NEW)
- `crates/argo-store/src/lib.rs` — narrowly: `pub mod worker_kpi`
- `crates/argo-cli/src/handlers/worker_kpi.rs` (NEW)
- `crates/argo-cli/src/cli_worker_cmds.rs` — `Kpi` variant + subenum
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 pub mod line

## Files you must NOT touch

- `crates/argo-identity/src/types.rs` Worker struct — KPI values are
  separate table, not a struct field
- charter storage migrations — those land in the parent brief
- TUI — out of scope for this brief

## Acceptance criteria

- [ ] `argo worker kpi record` inserts a row
- [ ] `argo worker kpi list --since 7d` filters by recorded_at
- [ ] `argo worker kpi series --bucket week` returns
      avg/min/max/count per bucket in markdown + JSON
- [ ] 8 unit tests (insert, list, bucket arithmetic, since filter)
      + 2 CLI smoke + 1 markdown snapshot
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-worker-kpi-tracker`
- PR title: `worker: per-worker KPI metric time-series store + CLI (claude-worker-kpi-tracker)`

## Notes

- Mirror the `series` formatter on `argo session messages --markdown`
  (see project_argo_state.md "markdown-format family").
- Bucketing in SQLite: `strftime('%Y-%W', recorded_at)` for week,
  `strftime('%Y-%m', recorded_at)` for month.
- Defer auto-emit hooks (anomaly / success) to a follow-up so this
  PR stays narrow.
