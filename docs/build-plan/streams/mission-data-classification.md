# mission-data-classification — PII/PHI/Financial classification tags + per-class handling

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `f4fd2459` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-data-classification`.

## Scope

v2.1 governance. Tag data with classification (PII/PHI/Financial/Public). Per-classification handling rules: redaction in logs, encryption at rest, retention policy, approval-required-on-export.

LOC: ~600.

## Files you may touch

- NEW `crates/argo-classification/src/lib.rs` — tags + handlers. ≤350 LOC.
- NEW `crates/argo-classification/src/lib_tests.rs` — tests. ≤150 LOC.
- NEW `crates/argo-classification/Cargo.toml`.
- Workspace `Cargo.toml` — add member.
- `crates/argo-audit/src/event.rs` — narrowly: emit classification metadata. ≤40 modified lines.
- `crates/argo-store/src/redact.rs` (NEW or extend) — narrowly: per-class redaction. ≤60 modified lines.

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

- Branch: `codex-mission-data-classification`
- PR title: `feat: PII/PHI/Financial classification tags + per-class handling (mission-data-classification)`

## Notes for the implementer

- Priority: v2.1 enterprise (regulated industries).
- Authored as part of post-v1 backlog batch 2026-05-15.
