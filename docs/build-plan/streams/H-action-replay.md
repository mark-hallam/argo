# Stream H (follow-up) — `argo action replay`

**Owner**: codex. **Depends on**: B-computer-use-executor
(#787) merged.

## Scope

A debugging tool for v1: re-run a completed action against a
fresh executor + fresh sandbox. Useful when:
- A skill-learning candidate looks wrong → operator wants to
  reproduce the trajectory step-by-step
- An action's result_json has unexpected fields → reproduce
  with verbose logging
- The vertical's manifest changed → re-test the action against
  the new system prompt

Subcommand:
- `argo action replay <action_id> [--executor human|api|computer_use]
  [--dry-run] [--bundle-dir <path>]` →
  - reads the original action's payload
  - constructs a fresh executor (default: same kind as
    the original)
  - re-emits the action against the new executor
  - writes result to a sibling bundle dir labelled `replay-<timestamp>`

## Files you may touch
- `crates/argo-cli/src/cli_action_cmds.rs` — narrowly: add
  `Replay { ... }` subcommand
- `crates/argo-cli/src/handlers/action_replay.rs` (NEW)
- `crates/argo-cli/src/cli.rs` — narrowly: register the
  Replay arm
- `crates/argo-cli/src/main.rs` — narrowly: dispatch

## Files you must NOT touch
- The original action's audit trail
- `argo-executor-computer-use/**` (use the existing API)
- The original action's bundle dir

## Acceptance criteria
- [ ] `argo action replay <id>` produces a new bundle in
      `<cwd>/.skycot/outputs/<job_id>/actions/<action_id>/replay-<timestamp>/`
- [ ] `--dry-run` prints what would happen without
      executing
- [ ] `--executor` override lets the operator try a different
      executor kind (human → fills out the bundle without
      auto-execution)
- [ ] Integration test using a mock executor, asserts the
      replay bundle exists + contains the result
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Every authored file < 500 LOC

## Branch + PR
- Branch: `codex-H-action-replay`
- PR title: `cli: argo action replay for debugging completed actions (H-action-replay)`

## Notes
- The replay does NOT update the original action's lifecycle
  state — it's a separate debug operation.
- For computer_use, requires E2B_API_KEY at replay time
  (skip the integration test in CI; keep an offline unit
  test).
