# mission-task-distribution — Task routing: round-robin + skill-auction

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `f4fd2459` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-task-distribution`.

## Scope

v1.3 multi-worker. When DAG task is ready, route to a worker via round-robin OR skill-auction. Operator configures mode per DAG/task. Depends on workflow-dag-runtime + worker-roster.

LOC: ~800.

## Files you may touch

- `crates/argo-orchestrator/src/routing.rs` — narrowly: round_robin() + skill_auction() impls. ≤200 modified lines.
- `crates/argo-skill/src/skill_match.rs` — narrowly: real impl (current returns None). ≤120 modified lines.
- Test sibling — both modes with synthetic workers. ≤200 LOC.

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

- Branch: `codex-mission-task-distribution`
- PR title: `feat: Task routing: round-robin + skill-auction (mission-task-distribution)`

## Notes for the implementer

- Priority: v1.3 multi-worker (depends on workflow-dag-runtime + worker-roster).
- Authored as part of post-v1 backlog batch 2026-05-15.
