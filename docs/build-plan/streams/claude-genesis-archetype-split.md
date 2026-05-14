# claude-genesis-archetype-split — split archetype.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing (pure refactor).
**Authored against**: origin/main `18b7746f` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-genesis-archetype-split`.

## Scope

Pure refactor split for `crates/argo-genesis/src/archetype.rs` (475 LOC). Pre-emptive split before file hits the 500-LOC ceiling.

Mechanical split: identify a sibling cluster (helpers, tests, fixtures), move to a sibling file, preserve public surface, both files end under 400 LOC.

**Not in scope**: changing any function body; renaming pub items.

## Files you may touch

- `crates/argo-genesis/src/archetype.rs` (extract content).
- NEW sibling file under same module.
- Parent declaration file if needed.

## Files you must NOT touch

- Any caller of the file's public items.

## Acceptance criteria

- [ ] Both files end under 400 LOC.
- [ ] Public surface unchanged.
- [ ] `cargo clippy -p <crate> --all-targets -- -D warnings` clean.
- [ ] `cargo test -p <crate>` passes.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-genesis-archetype-split`
- PR title: `refactor: split archetype.rs below 500-LOC ceiling (claude-genesis-archetype-split)`

## Notes for the implementer

- ~30 min mechanical refactor; pure shape change.
