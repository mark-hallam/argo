# claude-runtime-planner-split — split argo-runtime/src/planner.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main dd9362a0 on 2026-05-10.

## Why

`crates/argo-runtime/src/planner.rs` is at 449/500 LOC. Pre-emptive split keeps headroom for future feature briefs.

## Approach (concrete)

1. **Identify a cohesive sibling group** — typically the planning algorithm core vs supporting types/helpers, or `#[cfg(test)]` mod extraction. Read the file end-to-end before deciding the cut.
2. **Extract to sibling** `planner_<topic>.rs` in the same directory.
3. **Re-export via `pub(crate) use`** (or `pub use` for public API).
4. **Keep public API unchanged** — every existing import must resolve identically.
5. **Test.** `cargo clippy -p argo-runtime --all-targets -- -D warnings` clean. `cargo test -p argo-runtime` green.

## Files you may touch

- `crates/argo-runtime/src/planner.rs` (must drop below 400 LOC)
- `crates/argo-runtime/src/planner_<topic>.rs` (NEW, ≤ 300 LOC)
- `crates/argo-runtime/src/lib.rs` — narrowly: 1 `mod` declaration

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-runtime/`

## Acceptance criteria

- [ ] `planner.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-runtime --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-runtime` green
- [ ] Public API unchanged

## Branch + PR

- Branch: `claude-runtime-planner-split`
- PR title: `argo-runtime: split planner.rs below 500-LOC ceiling (claude-runtime-planner-split)`
