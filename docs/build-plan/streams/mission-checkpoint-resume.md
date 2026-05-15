# mission-checkpoint-resume — DAG checkpoint + resume across restarts

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `31d8e20e` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-checkpoint-resume`.

## Scope

v1.3 multi-worker orchestration. After each task completes, save checkpoint `{task_id, output, ts, dag_state}`. On resume, load last checkpoint and skip completed tasks. Depends on workflow-dag-runtime.

LOC: ~900.

## Files you may touch

- `crates/argo-orchestrator/src/checkpoint.rs` (NEW) — serialization + persistence. ≤400 LOC.
- NEW `crates/argo-store/migrations/NNNN_dag_checkpoint.sql` — pick next free slot at claim. ≤30 lines.
- `crates/argo-cli/src/handlers/workflow.rs` — narrowly: add resume command. ≤80 modified lines.
- Test sibling — crash + resume integration test. ≤180 LOC.

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

- Branch: `codex-mission-checkpoint-resume`
- PR title: `feat: DAG checkpoint + resume across restarts (mission-checkpoint-resume)`

## Notes for the implementer

- Priority: v1.3 multi-worker (depends on mission-workflow-dag-runtime).
- Authored as part of post-v1 polish backlog batch 2026-05-15.
