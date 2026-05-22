# Stream B — dry-run preview for `argo worker actions execute`

**Owner**: codex.
**Depends on**: `mission-wire-mcp-action-executor` MERGED and `mission-wire-api-action-executor` MERGED.
**Authored against**: origin/main `69b929c8` on `2026-05-08`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-action-dry-run-mode`.
**MVP lane**: gives operators a safe preview path before side-effecting action execution.

## Scope

The original brief targeted a nonexistent `argo action invoke` command and
stale `crates/argo-cli/src/commands.rs`. Current main executes worker actions
through `argo worker actions execute <action_id>`, with clap definitions in
`cli_worker_cmds.rs` and dispatch in `worker_actions_dispatch.rs`.

Add `--dry-run` to `argo worker actions execute`. In dry-run mode, the CLI
loads the latest action record, runs the same approval/circuit preflight checks
that a real execute would run, resolves the declared executor kind when a
vertical manifest is available, and prints a would-execute preview. It must not
transition the action to `executing`, call an executor, write an output bundle,
or append a completion row.

## Files you may touch

- `crates/argo-cli/src/cli_worker_cmds.rs` — add `--dry-run` to `WorkerActionsCmd::Execute` only.
- `crates/argo-cli/src/handlers/worker_actions.rs` — thread the new flag to dispatch.
- `crates/argo-cli/src/handlers/worker_actions_dispatch.rs` — branch before state mutation when dry-run is set.
- `crates/argo-cli/src/handlers/worker_actions_dry_run.rs` (NEW) — preview helper and text/JSON rendering.
- `crates/argo-cli/src/handlers/worker_actions_dry_run_tests.rs` (NEW) — unit tests for preview behavior.
- `crates/argo-cli/src/handlers/mod.rs` — one `pub mod worker_actions_dry_run;` line.
- `crates/argo-cli/tests/worker_actions_dry_run_smoke.rs` (NEW) — CLI smoke for help + no state transition.

## Files you must NOT touch

- `crates/argo-cli/src/commands.rs` — this file does not exist on current main.
- `crates/argo-cli/src/handlers/worker_actions_executor.rs` — currently 480 LOC; avoid adding to it for this preview-only PR.
- `crates/argo-cli/src/handlers/worker_actions_executor_api.rs`, `worker_actions_executor_mcp.rs`, and `worker_actions_executor_stubs.rs` — no executor refactors in this brief.
- `argo-types::ActionOutcome` — dry-run is CLI preview output, not a new runtime outcome.
- Anything not on the may-touch list above.

## Acceptance criteria

- [ ] `argo worker actions execute --help` shows `--dry-run`.
- [ ] `--dry-run --json` emits `{ "dry_run": true, "action_id": ..., "would_execute": ... }`.
- [ ] Dry-run does not append a new worker-action memory row.
- [ ] Dry-run does not create a `.skycot/outputs/.../actions/<id>/` bundle.
- [ ] Approval-gated actions without approval still fail the same way a real execute would fail.
- [ ] Autonomous-safe pending actions can be previewed without moving to `executing`.
- [ ] Preview includes executor kind when it can be resolved, and a clear `"unknown"`/error field when the vertical manifest is unavailable.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` passes.
- [ ] `cargo test -p argo-cli` passes.
- [ ] Every modified file stays under 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `codex-claude-action-dry-run-mode`
- PR title: `actions: preview worker action execution without state mutation (claude-action-dry-run-mode)`

## Notes for the implementer

- Pattern-match `worker_actions_dispatch::execute`, but branch before `worker_actions_transitions::execute`.
- Use existing query helpers rather than re-parsing MEMORY.md manually.
- This is intentionally CLI dry-run only. Manifest-level `dry_run = true` can be a later runtime contract after the preview surface proves useful.
