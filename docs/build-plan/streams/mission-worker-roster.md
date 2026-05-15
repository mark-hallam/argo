# mission-worker-roster — Worker rosters + team-level EffectivePolicy

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `f4fd2459` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-worker-roster`.

## Scope

v1.3 multi-worker orchestration. Named groups of workers with team-level policy (approval, skill, availability, budget). CLI: `argo roster create/add-worker/remove-worker/set-policy`. Logical grouping for DAG dispatch + delegation.

LOC: ~500.

## Files you may touch

- NEW `crates/argo-identity/src/roster.rs` — Roster struct + storage. ≤250 LOC.
- NEW `crates/argo-identity/src/roster_tests.rs` — unit tests. ≤150 LOC.
- `crates/argo-identity/src/policy.rs` — narrowly: team-level inheritance. ≤30 modified lines.
- NEW `crates/argo-cli/src/handlers/roster.rs` (NEW) — CLI handler. ≤200 LOC.
- `crates/argo-cli/src/cli_command_enum.rs` — Roster variant. ≤15 modified lines.
- NEW `crates/argo-store/migrations/NNNN_roster.sql` (NEW) — pick next free slot. ≤25 lines.

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

- Branch: `codex-mission-worker-roster`
- PR title: `feat: Worker rosters + team-level EffectivePolicy (mission-worker-roster)`

## Notes for the implementer

- Priority: v1.3 multi-worker UX.
- Authored as part of post-v1 backlog batch 2026-05-15.
