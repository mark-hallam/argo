# claude-skill-tests-split — split argo-skill tests.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep — defensive split.**

**Authored against**: origin/main 828e5e1b on 2026-05-10.

## Why

`crates/argo-skill/src/tests.rs` is at 471 LOC. Future skill-API tests (canary picker, promote-pinning, version-resolution) push this over 500. Pure refactor mirrors proven pattern (#1116/#1126).

## Approach

1. Group tests by concern (e.g., promote vs. resolve vs. store).
2. Move one cohesive group to `crates/argo-skill/src/tests_<topic>.rs`.
3. Re-include via `#[cfg(test)] #[path = "..."] mod` (sibling-test convention).

Target: `tests.rs` ≤ 350 LOC.

## Files you may touch

- `crates/argo-skill/src/tests.rs` — extract sibling
- `crates/argo-skill/src/tests_<topic>.rs` (NEW, ≤ 250 LOC)
- `crates/argo-skill/src/lib.rs` — narrowly: 1 declaration if needed

## Acceptance criteria

- [ ] `tests.rs` ≤ 350 LOC
- [ ] `cargo clippy -p argo-skill --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-skill` clean (same test count pre/post split)

## Branch + PR

- Branch: `claude-skill-tests-split`
- PR title: `skill: split tests.rs below 500-LOC ceiling (claude-skill-tests-split)`
