# mission-circuit-breaker-work-loop — Auto-pause work loop on N consecutive action errors

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `31d8e20e` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-circuit-breaker-work-loop`.

## Scope

v1.1 production hardening. Add per-session error counter to work loop. On each action failure increment; on success reset. At configurable threshold (default 3), emit NotifyChannel alert (risk=critical) and gate next action. Operator must `argo worker resume` to unlock.

LOC: ~250.

## Files you may touch

- `crates/argo-cli/src/handlers/work_loop.rs` — narrowly: error-counter + pause-gate. ≤80 modified lines.
- `crates/argo-config/src/sections.rs` — `[work_loop.circuit_breaker]` block. ≤25 modified lines.
- Test sibling — counter reset, threshold firing, gate behavior. ≤140 LOC.

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

- Branch: `codex-mission-circuit-breaker-work-loop`
- PR title: `feat: Auto-pause work loop on N consecutive action errors (mission-circuit-breaker-work-loop)`

## Notes for the implementer

- Priority: v1.1 production hardening.
- Authored as part of post-v1 polish backlog batch 2026-05-15.
