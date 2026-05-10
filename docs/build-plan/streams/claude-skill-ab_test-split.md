# claude-skill-ab_test-split — split argo-skill/src/ab_test.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main 04b7d2e0 on 2026-05-10.

## Why

`crates/argo-skill/src/ab_test.rs` is at 462/500 LOC, tight against the hard ceiling. Pre-emptive split keeps headroom for future feature briefs (auto-promote tuning, chi-square variant tests, traffic-split policies).

## Approach (concrete)

1. **Identify a cohesive sibling group** — sub-domain helpers, struct + impl block, `#[cfg(test)]` mod, persistence layer, renderer cluster. Read the file end-to-end before deciding the cut. Likely candidates: the `#[cfg(test)]` mod (extract to `ab_test_tests.rs` via `#[path]`) or the chi-square / SHA-256-split statistical helpers (extract to `ab_test_stats.rs`).
2. **Extract to sibling** `ab_test_<topic>.rs` in the same directory.
3. **Re-export via `pub(crate) use`** (or `pub use` for public API).
4. **Keep public API unchanged** — every existing import must resolve identically.
5. **Test.** `cargo clippy -p argo-skill --all-targets -- -D warnings` clean. `cargo test -p argo-skill` green.

## Files you may touch

- `crates/argo-skill/src/ab_test.rs` (must drop below 400 LOC)
- `crates/argo-skill/src/ab_test_<topic>.rs` (NEW, ≤ 300 LOC)
- `crates/argo-skill/src/lib.rs` — narrowly: 1 `mod` declaration

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-skill/`

## Acceptance criteria

- [ ] `ab_test.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-skill --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-skill` green
- [ ] Public API unchanged

## Branch + PR

- Branch: `claude-skill-ab_test-split`
- PR title: `argo-skill: split ab_test.rs below 500-LOC ceiling (claude-skill-ab_test-split)`
