# Claude — Action cancel lifecycle

**Owner**: claude. **Depends on**: A-executor-trait (#784),
H-2 (#800) merged.

## Scope

Add `Cancelled` state to V3 action lifecycle + CLI:
- `argo worker actions cancel <id> [--note "..."]`
- Lifecycle rule: only `pending` and `approved` → `cancelled`
- Audit row written on cancel

## Files you may touch
- `crates/argo-cli/src/handlers/worker_actions_transitions.rs`
  — extend
- `crates/argo-cli/src/handlers/worker_actions.rs` — narrowly:
  add cancel dispatch
- `crates/argo-types/src/lib.rs` (or wherever ActionState
  lives) — add Cancelled variant
- `crates/argo-cli/tests/action_cancel.rs` (NEW)
- BLANKET CLI ALLOWLIST applies

## Acceptance criteria
- [ ] ActionState::Cancelled with serde round-trip
- [ ] cancel() only allowed from pending/approved
- [ ] Audit row labelled `action_cancelled` with operator
      + note
- [ ] 3-path test (pending, approved, rejected-from-executing)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Every authored file < 500 LOC

## Branch + PR
- Branch: `claude-action-cancel-flow`
- PR title: `worker: cancel action lifecycle (claude-action-cancel-flow)`
