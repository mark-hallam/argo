# claude-store-lib-split — split argo-store lib.rs at 499-LOC ceiling-1

**Owner**: claude. **Depends on**: nothing (pure refactor).
**Authored against**: origin/main `e048bc8b` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-store-lib-split`.

## Scope

Pure refactor split for `crates/argo-store/src/lib.rs` (499 LOC). 1 line below ceiling. lib.rs is high-traffic (every new store module re-exports from here). Pre-emptive split unblocks future store work.

Mechanical split:

1. Identify natural extraction boundary (a logical sub-cluster of items — usually a sibling group of helpers, or one of: tests, fixtures, builders, parsers). Keep the original file's PUBLIC surface unchanged.
2. Move the sub-cluster to a sibling file (e.g. `lib_helpers.rs` or `lib_tests.rs`).
3. Add `mod lib_helpers;` (or whatever sibling name) to the parent module file.
4. Re-export any pub items that callers expect from the parent (so the public surface is preserved).
5. Verify no `super::` reference now needs adjustment.

Both files should end under 400 LOC each (leaves headroom for future additions).

**Not in scope**: changing any function body; renaming any pub item; introducing new tests.

## Files you may touch

- `crates/argo-store/src/lib.rs` (extract content).
- NEW sibling file (e.g. `lib_helpers.rs` or `lib_tests.rs`).
- The crate's `lib.rs` or `mod.rs` if it declares the module (likely yes).

## Files you must NOT touch

- Any caller of the file's public items — the split preserves the public surface.
- Any unrelated file in the crate.

## Acceptance criteria

- [ ] Both files end under 400 LOC.
- [ ] Public surface unchanged (caller imports still work).
- [ ] `cargo clippy -p <containing-crate> --all-targets -- -D warnings` clean.
- [ ] `cargo test -p <containing-crate>` passes.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-store-lib-split`
- PR title: `refactor: split lib.rs below 500-LOC ceiling (claude-store-lib-split)`

## Notes for the implementer

- ~30 min mechanical refactor.
- Pure shape change; no behavior change.
