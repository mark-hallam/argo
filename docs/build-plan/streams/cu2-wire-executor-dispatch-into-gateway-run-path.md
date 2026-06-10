# cu2-wire-executor-dispatch-into-gateway-run-path

**Owner**: claude or codex — **load-bearing run path → conductor review before merge.**
**Depends on**: none (independent of cu1; together they deliver "real work").
**Authored against**: origin/main `5f81ae776` on `2026-06-11`.
**Validate before claim**: bash scripts/check-brief-freshness.sh cu2-wire-executor-dispatch-into-gateway-run-path

## Goal (DEC-050 G2)

Make the SPA "Run now" path actually EXECUTE a teammate's declared actions
through their executors — today it emits + classifies actions then stops, so
nothing real happens (the core gap behind "teammate did 0 actions").

## Why now (audit 2026-06-11)

`work_dispatch.rs::process_job()` → `run_job_steps()` → `emit_actions()` →
`work_emission.rs::emit_and_persist_actions()` persists ActionRecords as
`pending`/`blocked` and **returns — no executor is ever invoked**. Executor
dispatch exists only in the CLI (`argo worker actions execute` →
`worker_actions_dispatch.rs` + `resolve_executor`). So a `computer_use` /
`document_creation` / MCP action emitted on Run-now is inert.

## Fix

1. New `work_dispatch_executor_inline.rs`: after `emit_actions()`, iterate the
   returned `Vec<ActionRecord>`; for each that is approved/auto-eligible (risk
   tier + `ARGO_EXECUTION_GRADUATION`, DEC-041), call the SAME
   `resolve_executor()` + `Executor::dispatch()` the CLI uses, and persist the
   `executing→completed|failed` transition to the worker_action ledger.
2. Thread `tenant_id`, vertical manifest, `cwd`/fs_root through `process_job` so
   `resolve_executor` can load the vertical + executor kind.
3. Honor every gate: approval-required + probation + DEC-028 red lines stay
   gated (a held action is NOT auto-executed). Reuse `decide_gate` semantics.
4. `finalize_job()` takes the records so the inline dispatcher has actions +
   completed_jobs evidence in one place.

## May touch

- `crates/argo-cli/src/handlers/work_dispatch.rs`, `work_dispatch_phases.rs`,
  `work_emission.rs`, new `work_dispatch_executor_inline.rs` + tests
- reuse (do NOT fork) `worker_actions_dispatch.rs::resolve_executor` + `Executor`

## Does NOT touch (reuse-only — boundary note for the freshness heuristic)

This brief does NOT modify the CLI worker-action surface
(`handlers/worker_actions.rs`, `cli_worker_cmds.rs`); it REUSES
`worker_actions_dispatch.rs::resolve_executor` + the `Executor` trait from the
gateway run path. No `argo worker actions` command behavior changes.

## Acceptance / verifier

1. Integration (mock executor, no API keys): emit an action with
   `executor='computer_use'`, assert the inline dispatcher calls
   `executor.dispatch()` and writes `completed`. An approval-required action is
   NOT dispatched (stays held). 
2. Low-risk graduated action auto-executes only with `ARGO_EXECUTION_GRADUATION=1`.
3. `cargo test -p argo-cli` green; clippy; the CLI execute path stays
   byte-identical (shared resolver).

## Size / owner / priority

~200-350 LOC incl. tests. **🔴 P1 — DEC-050 G2, the core "actions actually
run" wire.** Pairs with cu1 (file work) and cu3 (cu-macos desktop work).
