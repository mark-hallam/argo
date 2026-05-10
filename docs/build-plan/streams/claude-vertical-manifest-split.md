# claude-vertical-manifest-split — split argo-vertical/src/manifest.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main 9d8a18df on 2026-05-10.

## Why

`crates/argo-vertical/src/manifest.rs` is at 430/500 LOC. Pre-emptive split keeps headroom for future vertical-template extensions.

## Approach (concrete)

1. **Identify a cohesive sibling group** — typically by manifest section (e.g. persona vs actions vs eval-config) or `#[cfg(test)]` mod extraction. Read end-to-end.
2. **Extract to sibling** `manifest_<topic>.rs`.
3. **Re-export via `pub use`** since manifest is part of vertical's public API.
4. **Test.** `cargo clippy -p argo-vertical --all-targets -- -D warnings` clean. `cargo test -p argo-vertical` green.

## Files you may touch

- `crates/argo-vertical/src/manifest.rs` (must drop below 400 LOC)
- `crates/argo-vertical/src/manifest_<topic>.rs` (NEW, ≤ 300 LOC)
- `crates/argo-vertical/src/lib.rs` — narrowly: 1 `mod` declaration

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-vertical/`
- Any vertical's TOML/skill files (those are inputs, not implementation)

## Acceptance criteria

- [ ] `manifest.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-vertical --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-vertical` green
- [ ] Public API unchanged

## Branch + PR

- Branch: `claude-vertical-manifest-split`
- PR title: `argo-vertical: split manifest.rs below 500-LOC ceiling (claude-vertical-manifest-split)`
