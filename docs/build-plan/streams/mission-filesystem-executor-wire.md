# mission-filesystem-executor-wire — Wire FilesystemExecutor into runtime dispatch

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `31d8e20e` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-filesystem-executor-wire`.

## Scope

v1.2 capability breadth. `argo-executor-filesystem` crate exists. Wire runtime dispatch: `ExecutorKind::Filesystem` variant + agent dispatcher routing.

LOC: ~250.

## Files you may touch

- `crates/argo-runtime/src/executor.rs` (NEW) — narrowly: add Filesystem variant + dispatch arm. ≤40 modified lines.
- `crates/argo-core/src/agent.rs` (or current dispatch location) — narrowly: route filesystem actions. ≤30 modified lines.
- `crates/argo-executor-filesystem/tests/` — integration test: file create/read/delete in worker root + sandbox-escape rejection. ≤150 LOC.

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

- Branch: `codex-mission-filesystem-executor-wire`
- PR title: `feat: Wire FilesystemExecutor into runtime dispatch (mission-filesystem-executor-wire)`

## Notes for the implementer

- Priority: v1.2 capability.
- Authored as part of post-v1 polish backlog batch 2026-05-15.
