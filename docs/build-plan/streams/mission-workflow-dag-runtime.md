# mission-workflow-dag-runtime — Declarative DAG of tasks + scheduler + retry/skip policy

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `31d8e20e` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-workflow-dag-runtime`.

## Scope

v1.3 multi-worker orchestration. Operator-defined DAG (YAML/TOML): tasks are workers or primitives (wait/branch/fan-out/fan-in), edges declare deps, retry/skip per task. Scheduler topologically executes ready nodes. CLI: `argo workflow run <file>`.

LOC: ~1500.

## Files you may touch

- `crates/argo-orchestrator/src/dag.rs` (NEW) — DAG IR + parser. ≤500 LOC.
- `crates/argo-orchestrator/src/scheduler.rs` (NEW) — scheduler loop. ≤500 LOC.
- `crates/argo-cli/src/handlers/workflow.rs` (NEW) — CLI. ≤200 LOC.
- `crates/argo-cli/src/cli_command_enum.rs` — narrowly: add Workflow variant. ≤15 modified lines.
- Test sibling — 3-task DAG fan-out/fan-in + error injection. ≤250 LOC.

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

- Branch: `codex-mission-workflow-dag-runtime`
- PR title: `feat: Declarative DAG of tasks + scheduler + retry/skip policy (mission-workflow-dag-runtime)`

## Notes for the implementer

- Priority: v1.3 multi-worker.
- Authored as part of post-v1 polish backlog batch 2026-05-15.
