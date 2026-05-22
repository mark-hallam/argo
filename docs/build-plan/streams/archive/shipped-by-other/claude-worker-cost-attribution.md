# Stream B — per-worker cost attribution CLI

**Owner**: codex.
**Depends on**: `claude-session-worker-id-schema` MERGED.
**Authored against**: origin/main `69b929c8` on `2026-05-08`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-worker-cost-attribution`.
**MVP lane**: gives the operator worker-level economics for delegation decisions.

## Scope

The stale brief assumed messages/actions needed new worker attribution. Current
main already has the durable cost source for MVP: `worker_budget(tenant_id,
worker_id, action_id, usd_micros, charged_at)` from
`14__worker_budget.sql`, plus `session.worker_id` from #1013 for future joins.

Add `argo worker costs <worker_id>` as a read-side CLI over the existing
`worker_budget` ledger. Do not add schema. Do not duplicate the whole
`argo budget` command; this is a worker command because operators reviewing a
worker's charter/KPIs should see cost in the same command family.

## Files you may touch

- `crates/argo-cli/src/cli_worker_costs_cmds.rs` (NEW) — clap enum/options if the shape is large enough to keep out of `cli_worker_cmds.rs`.
- `crates/argo-cli/src/cli_reexports.rs` — narrowly: module declaration/re-export if using the sibling enum.
- `crates/argo-cli/src/cli_worker_cmds.rs` — narrowly: add `WorkerCmd::Costs(...)` or a compact `Costs { ... }` variant only.
- `crates/argo-cli/src/handlers/worker_costs.rs` (NEW) — query/render cost attribution.
- `crates/argo-cli/src/handlers/worker_costs_tests.rs` (NEW) — query/render tests.
- `crates/argo-cli/src/handlers/worker_actions.rs` — narrowly: route the worker costs command if this remains the worker dispatcher.
- `crates/argo-cli/src/handlers/mod.rs` — one `pub mod worker_costs;` line.
- `crates/argo-cli/tests/worker_costs_smoke.rs` (NEW) — CLI smoke over a temporary workspace DB.
- `crates/argo-cli/src/handlers/budget.rs` — narrowly: expose small formatting/since helpers only if reuse avoids duplication; do not grow this file past 500 LOC.

## Files you must NOT touch

- `crates/argo-store/migrations/**` — existing `worker_budget` schema is enough.
- `crates/argo-store/src/budget.rs` — skill-spend helpers are unrelated to this CLI.
- `docs/skycot/RUNBOOK.md` — over the 500 LOC ceiling.
- TUI files — read-side UI is a follow-up.
- Anything not on the may-touch list above.

## Acceptance criteria

- [ ] `argo worker costs <worker_id>` prints total cost and recent charges.
- [ ] `--since <rfc3339>` filters charges by `charged_at`.
- [ ] `--breakdown day` groups by UTC day.
- [ ] `--breakdown action` groups by `action_id`.
- [ ] `--json` emits a stable JSON envelope with total USD micros, row count, and breakdown rows.
- [ ] Worker with zero charges returns total `0` and no error.
- [ ] Query remains tenant-bound when a tenant id is supplied; cross-tenant rows for the same worker id do not leak.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` passes.
- [ ] `cargo test -p argo-cli` passes.
- [ ] Every modified file stays under 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `codex-claude-worker-cost-attribution`
- PR title: `worker: show cost attribution from worker budget ledger (claude-worker-cost-attribution)`

## Notes for the implementer

- Pattern-match `crates/argo-cli/src/handlers/budget.rs` for DB opening and money formatting.
- Keep the first version read-only over `worker_budget`; action-outcome joins can be a later brief once the worker action ledger has a stable query helper.
- If `cli_worker_cmds.rs` would exceed 500 LOC, move the full costs subcommand shape into `cli_worker_costs_cmds.rs` and add only the small enum variant in `cli_worker_cmds.rs`.
