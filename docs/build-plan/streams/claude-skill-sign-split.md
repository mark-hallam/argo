# claude-skill-sign-split — split sign.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing (pure refactor).
**Authored against**: origin/main `18b7746f` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-skill-sign-split`.

## Scope

Pure refactor split for `crates/argo-skill/src/sign.rs` (474 LOC). Pre-emptive split before file hits the 500-LOC ceiling.

Mechanical split: identify a sibling cluster (helpers, tests, fixtures), move to a sibling file, preserve public surface, both files end under 400 LOC.

**Not in scope**: changing any function body; renaming pub items.

## Files you may touch

- `crates/argo-skill/src/sign.rs` (extract content).
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

- Branch: `claude-skill-sign-split`
- PR title: `refactor: split sign.rs below 500-LOC ceiling (claude-skill-sign-split)`

## Notes for the implementer

- ~30 min mechanical refactor; pure shape change.
