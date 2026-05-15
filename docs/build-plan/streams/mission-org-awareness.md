# mission-org-awareness — Org chart + addressable worker names for delegation

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `f4fd2459` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-org-awareness`.

## Scope

v1.3 multi-worker. Workers can address each other by name ('delegate to Alice in sales'). OrgChart stores manager_chain + names. Name resolution in delegate_task.

LOC: ~500.

## Files you may touch

- NEW `crates/argo-identity/src/org.rs` — OrgChart storage + manager-chain traversal. ≤200 LOC.
- NEW `crates/argo-identity/src/org_tests.rs` — unit tests. ≤120 LOC.
- `crates/argo-tool/src/tools/delegate_task.rs` — narrowly: name resolution. ≤40 modified lines.
- `crates/argo-cli/src/handlers/org.rs` (NEW) — `argo org set-manager`, `set-name`. ≤120 LOC.
- `crates/argo-cli/src/cli_command_enum.rs` — Org variant. ≤15 modified lines.

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

- Branch: `codex-mission-org-awareness`
- PR title: `feat: Org chart + addressable worker names for delegation (mission-org-awareness)`

## Notes for the implementer

- Priority: v1.3 multi-worker UX.
- Authored as part of post-v1 backlog batch 2026-05-15.
