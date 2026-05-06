# Stream H — `argo handoff cancel`

**Owner**: codex. **Depends on**: H-orchestrator-cli (#824),
H-orchestrator-persist (#806) merged.

## Scope

- `argo handoff cancel <id> [--note "..."]` → flips
  Pending/Acknowledged → Cancelled with audit row
- Add `Cancelled` to HandoffStatus
- `cancel_handoff(id, note)` method on persist API

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-orchestrator/src/handoff.rs` — narrowly: add
  Cancelled variant + cancel() method
- `crates/argo-orchestrator/src/persist.rs` — narrowly: add
  cancel_handoff method
- `crates/argo-cli/src/cli_session_cmds.rs` — narrowly:
  extend with handoff cancel variant
- `crates/argo-cli/src/handlers/session.rs` — narrowly:
  extend with cancel handler
- `crates/argo-cli/tests/handoff_cancel.rs` (NEW)

## Acceptance criteria
- [ ] cancel() rejects Settled/Expired transitions
- [ ] CLI emits audit row on cancel
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Every authored file < 500 LOC

## Branch + PR
- Branch: `codex-H-handoff-cancel`
- PR title: `orchestrator: argo handoff cancel + Cancelled lifecycle (H-handoff-cancel)`
