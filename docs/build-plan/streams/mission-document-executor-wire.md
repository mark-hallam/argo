# mission-document-executor-wire — Wire DocumentExecutor (PDF/markdown/csv/gsuite) into runtime dispatch

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `31d8e20e` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-document-executor-wire`.

## Scope

v1.2 capability breadth. `argo-executor-document`, `-csv`, `-gsuite` crates exist. Wire runtime dispatch for `ExecutorKind::DocumentPdf | DocumentMarkdown | DocumentCsv | DocumentGsuite`.

LOC: ~220.

## Files you may touch

- `crates/argo-runtime/src/executor.rs` (NEW) — narrowly: add Document* variants + dispatch arms. ≤50 modified lines.
- `crates/argo-core/src/agent.rs` — narrowly: route document actions. ≤30 modified lines.
- Integration test sibling — PDF + markdown + CSV happy path. ≤130 LOC.

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

- Branch: `codex-mission-document-executor-wire`
- PR title: `feat: Wire DocumentExecutor (PDF/markdown/csv/gsuite) into runtime dispatch (mission-document-executor-wire)`

## Notes for the implementer

- Priority: v1.2 capability.
- Authored as part of post-v1 polish backlog batch 2026-05-15.
