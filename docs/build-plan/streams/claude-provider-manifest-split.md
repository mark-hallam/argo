# claude-provider-manifest-split — split argo-provider/src/manifest/mod.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main 9d8a18df on 2026-05-10.

## Why

`crates/argo-provider/src/manifest/mod.rs` is at 444/500 LOC. Pre-emptive split keeps headroom for future provider-manifest extensions.

## Approach (concrete)

1. **Identify a cohesive sibling group** — typically by manifest section (e.g. routing block vs auth block vs pricing block) or `#[cfg(test)]` mod extraction. Read end-to-end.
2. **Extract to sibling** within the manifest/ submodule (e.g. `manifest/<topic>.rs`).
3. **Re-export via `pub use`** since manifest is part of provider's public API.
4. **Test.** `cargo clippy -p argo-provider --all-targets -- -D warnings` clean. `cargo test -p argo-provider` green.

## Files you may touch

- `crates/argo-provider/src/manifest/mod.rs` (must drop below 400 LOC)
- `crates/argo-provider/src/manifest/<topic>.rs` (NEW, ≤ 300 LOC)

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-provider/`

## Acceptance criteria

- [ ] `manifest/mod.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-provider --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-provider` green
- [ ] Public API unchanged

## Branch + PR

- Branch: `claude-provider-manifest-split`
- PR title: `argo-provider: split manifest/mod.rs below 500-LOC ceiling (claude-provider-manifest-split)`
