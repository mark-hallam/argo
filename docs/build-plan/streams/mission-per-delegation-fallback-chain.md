# mission-per-delegation-fallback-chain — Inherit caller's escalation chain on delegate_task

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `31d8e20e` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-per-delegation-fallback-chain`.

## Scope

MISSION_BACKLOG Amendment 7. When `delegate_task` emits a handoff (already shipping via #1572), inherit the caller's `active_escalation_chain` and prepend the callee's WorkerId.

LOC: ~250.

## Files you may touch

- `crates/argo-tool/src/tools/delegate_task.rs` — narrowly: chain-inheritance logic in `emit_handoff()`. ≤60 modified lines.
- `crates/argo-orchestrator/src/handoff.rs` — narrowly: plumb chain into handoff payload. ≤40 modified lines.
- Test sibling — 8 unit tests + 1 smoke. ≤150 LOC.

## Files you must NOT touch

- Anything outside the may-touch list above.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Implementation matches scope above.
- [ ] All new tests pass.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side).
- [ ] No file > 500 LOC.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `codex-mission-per-delegation-fallback-chain`
- PR title: `feat: Inherit caller's escalation chain on delegate_task (mission-per-delegation-fallback-chain)`

## Notes for the implementer

- Priority: claim 2 — delegation as unit.
- Authored as part of post-v1 polish backlog batch 2026-05-15.
