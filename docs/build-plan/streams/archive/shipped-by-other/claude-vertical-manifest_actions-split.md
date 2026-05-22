# claude-vertical-manifest_actions-split — split argo-vertical/src/manifest_actions.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main a7e516d4 on 2026-05-10.

## Why

`crates/argo-vertical/src/manifest_actions.rs` is at 412/500 LOC. Pre-emptive split keeps headroom for future vertical action-class additions.

## Approach (concrete)

1. **Identify a cohesive sibling group** — typically by action category (e.g. notify vs approval vs execute) or `#[cfg(test)]` mod extraction.
2. **Extract to sibling** `manifest_actions_<topic>.rs`.
3. **Re-export via `pub use`** since it's part of vertical's public surface.
4. **Test.** `cargo clippy -p argo-vertical --all-targets -- -D warnings` clean. `cargo test -p argo-vertical` green.

## Files you may touch

- `crates/argo-vertical/src/manifest_actions.rs` (must drop below 400 LOC)
- `crates/argo-vertical/src/manifest_actions_<topic>.rs` (NEW, ≤ 300 LOC)
- `crates/argo-vertical/src/lib.rs` — narrowly: 1 `mod` declaration

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-vertical/`

## Acceptance criteria

- [ ] `manifest_actions.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-vertical --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-vertical` green
- [ ] Public API unchanged

## Branch + PR

- Branch: `claude-vertical-manifest_actions-split`
- PR title: `argo-vertical: split manifest_actions.rs below 500-LOC ceiling (claude-vertical-manifest_actions-split)`
