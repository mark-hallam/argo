# Stream B — worker KPI time-series store and CLI

**Owner**: codex.
**Depends on**: `claude-worker-charter-storage` MERGED.
**Authored against**: origin/main `69b929c8` on `2026-05-08`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-worker-kpi-tracker`.
**MVP lane**: closes mission claim 2's "worker has KPIs" runtime evidence path.

## Scope

Worker charters can already carry KPI definitions, but current main has no
time-series record of observed KPI values. Add a small per-worker KPI value
store plus `argo worker kpi ...` commands.

The previous block was caused by `crates/argo-cli/src/cli_worker_cmds.rs`
being at/near the 500 LOC ceiling. Current main has sibling command files for
worker archive and charter. Follow that pattern: put the new KPI subcommand
enum in a new sibling file and add only a small `WorkerCmd::Kpi(...)` variant
to `cli_worker_cmds.rs`.

## Files you may touch

- `crates/argo-store/migrations/39__worker_kpi_value.sql` (NEW) — use this prefix if still free at claim time; if another migration lands first, bump to the next free raw SQL prefix and note it in the PR body.
- `crates/argo-store/src/worker_kpi.rs` (NEW) — record/list/series helpers.
- `crates/argo-store/src/worker_kpi_tests.rs` (NEW) — unit tests for store helpers.
- `crates/argo-store/src/lib.rs` — narrowly: `pub mod worker_kpi;`.
- `crates/argo-cli/src/cli_worker_kpi_cmds.rs` (NEW) — clap enum for record/list/series.
- `crates/argo-cli/src/cli_reexports.rs` — narrowly: module declaration/re-export for `cli_worker_kpi_cmds`.
- `crates/argo-cli/src/cli_worker_cmds.rs` — narrowly: add `WorkerCmd::Kpi(WorkerKpiCmd)` and one `pub use`.
- `crates/argo-cli/src/handlers/worker_kpi.rs` (NEW) — CLI handler.
- `crates/argo-cli/src/handlers/worker_kpi_tests.rs` (NEW) — formatter/parse tests.
- `crates/argo-cli/src/handlers/worker_actions.rs` — narrowly: route `WorkerCmd::Kpi` to the new handler if this remains the worker command dispatcher.
- `crates/argo-cli/src/handlers/mod.rs` — one `pub mod worker_kpi;` line.
- `crates/argo-cli/tests/worker_kpi_smoke.rs` (NEW) — record/list/series smoke.

## Files you must NOT touch

- `crates/argo-identity/src/types.rs` — KPI observations are a separate store table, not a Worker struct field.
- Existing charter migrations — KPI definitions stay in charter storage.
- `docs/skycot/RUNBOOK.md` — over the 500 LOC ceiling.
- TUI files — read-side UI is a follow-up.
- Anything not on the may-touch list above.

## Acceptance criteria

- [ ] `argo worker kpi record <worker_id> <kpi> <value> [--tenant <tenant>] [--note <text>]` inserts one value.
- [ ] `argo worker kpi list <worker_id> [--tenant <tenant>] [--kpi <name>] [--since <rfc3339>]` filters correctly.
- [ ] `argo worker kpi series <worker_id> <kpi> --bucket day|week|month` returns avg/min/max/count buckets.
- [ ] Text output is operator-readable and JSON output is available for every list/show/status-style command.
- [ ] Store tests cover insert, tenant isolation, KPI-name filter, since filter, and bucket math.
- [ ] CLI smoke covers record/list/series.
- [ ] `cargo clippy -p argo-store -p argo-cli --all-targets -- -D warnings` passes.
- [ ] `cargo test -p argo-store -p argo-cli` passes.
- [ ] Every modified file stays under 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `codex-claude-worker-kpi-tracker`
- PR title: `worker: record and query per-worker KPI values (claude-worker-kpi-tracker)`

## Notes for the implementer

- Use `REAL` for KPI values, `TEXT` worker ids, and a tenant boundary column. Keep the schema additive.
- The raw migration namespace currently jumps from `38__session_worker_id.sql`; check `ls crates/argo-store/migrations` before creating the file.
- Do not validate KPI names against charter `kpis_json` in this PR. The first MVP need is observable time-series data.
