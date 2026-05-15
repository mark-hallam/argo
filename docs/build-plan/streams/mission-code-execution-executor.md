# mission-code-execution-executor — CodeExecutor — sandboxed Python/JS code execution (E2B-backed)

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `f4fd2459` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-code-execution-executor`.

## Scope

v1.2 capability breadth. New executor kind that accepts `{ language: 'python'|'javascript', code: '...', timeout_sec: 30 }`, spawns sandbox (via existing argo-executor-computer-use E2B infra or docker fallback), executes with wall-clock + memory limits, returns ExecutorResult.

LOC: ~900.

## Files you may touch

- NEW `crates/argo-executor-code/src/lib.rs` (NEW) — executor impl. ≤500 LOC.
- NEW `crates/argo-executor-code/src/lib_tests.rs` (NEW) — language/timeout/memory tests. ≤250 LOC.
- NEW `crates/argo-executor-code/Cargo.toml` (NEW).
- Workspace `Cargo.toml` — narrowly: add member. ≤3 lines.
- `crates/argo-runtime/src/executor.rs` (NEW) — narrowly: register `ExecutorKind::Code` variant. ≤30 modified lines.
- `crates/argo-config/src/sections.rs` — narrowly: `[executor.code]` block. ≤25 modified lines.

## Files you must NOT touch

- Anything outside the may-touch list above.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Implementation matches scope above.
- [ ] All new tests pass.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] No file > 500 LOC.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `codex-mission-code-execution-executor`
- PR title: `feat: CodeExecutor — sandboxed Python/JS code execution (E2B-backed) (mission-code-execution-executor)`

## Notes for the implementer

- Priority: v1.2 — claim 5 PC execution breadth.
- Authored as part of post-v1 backlog batch 2026-05-15.
