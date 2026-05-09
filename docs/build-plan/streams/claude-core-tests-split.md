# claude-core-tests-split — split argo-core/src/tests.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep — defensive split.**

**Authored against**: origin/main 1b1d498c on 2026-05-09 (conductor-authored after observing argo-core/src/tests.rs at 499/500 LOC).

## Why

`crates/argo-core/src/tests.rs` is at 499 LOC — 1 LOC of headroom against the ceiling. Future core-API tests would hit the wall. Pure refactor; mirrors proven pattern.

## Approach

1. Group tests by concern (e.g., session vs. message vs. error tests).
2. Move one cohesive group to `crates/argo-core/src/tests_<topic>.rs`.
3. Re-include via `#[cfg(test)] #[path = "..."] mod`.

Target: `tests.rs` ≤ 350 LOC.

## Files you may touch

- `crates/argo-core/src/tests.rs` — extract sibling
- `crates/argo-core/src/tests_<topic>.rs` (NEW, ≤ 250 LOC)
- `crates/argo-core/src/lib.rs` — narrowly: 1 declaration if needed

## Acceptance criteria

- [ ] `tests.rs` ≤ 350 LOC
- [ ] `cargo clippy -p argo-core --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-core` clean (same test count pre/post split)

## Branch + PR

- Branch: `claude-core-tests-split`
- PR title: `core: split tests.rs below 500-LOC ceiling (claude-core-tests-split)`
