# mission-policy-as-code — Declarative org policies — policy.toml as code

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `f4fd2459` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-policy-as-code`.

## Scope

v2.1 governance. Org-level policies expressed declaratively in `policy.toml` (approval rules, budget caps, skill restrictions, data-classification gates). Loader + runtime gate + audit.

LOC: ~700.

## Files you may touch

- NEW `crates/argo-policy/src/lib.rs` — Policy IR + loader. ≤400 LOC.
- NEW `crates/argo-policy/src/lib_tests.rs` — parsing + evaluation. ≤200 LOC.
- NEW `crates/argo-policy/Cargo.toml`.
- Workspace `Cargo.toml` — add member.
- `crates/argo-cli/src/handlers/work_loop.rs` — narrowly: policy-gate hook. ≤40 modified lines.

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

- Branch: `codex-mission-policy-as-code`
- PR title: `feat: Declarative org policies — policy.toml as code (mission-policy-as-code)`

## Notes for the implementer

- Priority: v2.1 enterprise gating.
- Authored as part of post-v1 backlog batch 2026-05-15.
