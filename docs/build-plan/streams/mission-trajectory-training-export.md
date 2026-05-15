# mission-trajectory-training-export — Export successful trajectories as jsonl for fine-tuning

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `31d8e20e` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-trajectory-training-export`.

## Scope

v2.0 self-improvement. Aggregate successful session trajectories into jsonl format (instruction + input + expected_output per action). CLI: `argo training export --vertical X --start-date Y` (new top-level `Training` enum variant, distinct from skill management). Filter + format. Output consumable by Claude/etc fine-tuning APIs.

LOC: ~450.

## Files you may touch

- NEW `crates/argo-cli/src/handlers/training.rs` (NEW) — exporter handler. ≤250 LOC.
- `crates/argo-audit/src/export.rs` — narrowly: trajectory aggregation. ≤80 modified lines.
- `crates/argo-cli/src/cli_command_enum.rs` — narrowly: `Training` variant. ≤20 modified lines.
- Test sibling — round-trip + format validation. ≤100 LOC.

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

- Branch: `codex-mission-trajectory-training-export`
- PR title: `feat: Export successful trajectories as jsonl for fine-tuning (mission-trajectory-training-export)`

## Notes for the implementer

- Priority: v2.0 self-improvement.
- Authored as part of post-v1 polish backlog batch 2026-05-15.
