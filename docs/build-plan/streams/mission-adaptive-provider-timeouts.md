# mission-adaptive-provider-timeouts — Wire heartbeat-event timeout extension in provider transport

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `31d8e20e` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-adaptive-provider-timeouts`.

## Scope

MISSION_BACKLOG Amendment 6. `TransportSpec` already has `heartbeat_event` + `heartbeat_grace_ms` fields. Wire the runtime: on heartbeat arrival, reset the read-timeout clock + apply grace extension. Add integration test against mock SSE provider verifying timeout doesn't fire when heartbeats tick.

LOC: ~300.

## Files you may touch

- `crates/argo-provider/src/transport.rs` — narrowly: timeout reset on heartbeat. ≤80 modified lines.
- `crates/argo-provider/src/transport_tests.rs` (sibling) — integration test with mock SSE. ≤150 LOC.

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

- Branch: `codex-mission-adaptive-provider-timeouts`
- PR title: `feat: Wire heartbeat-event timeout extension in provider transport (mission-adaptive-provider-timeouts)`

## Notes for the implementer

- Priority: v1.1 production hardening.
- Authored as part of post-v1 polish backlog batch 2026-05-15.
