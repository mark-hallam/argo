# claude-tool-send-message-split — split send_message.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing (pure refactor).
**Authored against**: origin/main `45c21803` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-tool-send-message-split`.

## Scope

Pure refactor split for `crates/argo-tool/src/tools/send_message.rs` (483 LOC). send_message.rs at 483 LOC.

Mechanical split: identify a sibling cluster (helpers, tests, fixtures), move to a sibling file, preserve public surface, both files end under 400 LOC.

**Not in scope**: changing any function body; renaming pub items.

## Files you may touch

- `crates/argo-tool/src/tools/send_message.rs` (extract content).
- NEW sibling file under same module.
- Parent  if it declares the module.

## Files you must NOT touch

- Any caller of the file's public items.

## Acceptance criteria

- [ ] Both files end under 400 LOC.
- [ ] Public surface unchanged.
- [ ] `cargo clippy -p <crate> --all-targets -- -D warnings` clean.
- [ ] `cargo test -p <crate>` passes.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-tool-send-message-split`
- PR title: `refactor: split send_message.rs below 500-LOC ceiling (claude-tool-send-message-split)`

## Notes for the implementer

- ~30 min mechanical refactor; pure shape change.
