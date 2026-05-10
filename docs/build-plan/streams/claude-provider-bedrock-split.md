# claude-provider-bedrock-split — split argo-provider/src/bedrock/mod.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main 7157bae4 on 2026-05-10.

## Why

`crates/argo-provider/src/bedrock/mod.rs` is at 437/500 LOC. Pre-emptive split keeps headroom for future bedrock model additions.

## Approach (concrete)

1. **Identify a cohesive sibling group** — typically by request/response shape vs streaming vs auth, or `#[cfg(test)]` mod extraction.
2. **Extract to sibling** within `bedrock/` (e.g. `bedrock/<topic>.rs`).
3. **Re-export via `pub(crate) use`** (or `pub use` for public API).
4. **Test.** `cargo clippy -p argo-provider --all-targets -- -D warnings` clean. `cargo test -p argo-provider` green.

## Files you may touch

- `crates/argo-provider/src/bedrock/mod.rs` (must drop below 400 LOC)
- `crates/argo-provider/src/bedrock/<topic>.rs` (NEW, ≤ 300 LOC)

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-provider/`

## Acceptance criteria

- [ ] `bedrock/mod.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-provider --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-provider` green
- [ ] Public API unchanged

## Branch + PR

- Branch: `claude-provider-bedrock-split`
- PR title: `argo-provider: split bedrock/mod.rs below 500-LOC ceiling (claude-provider-bedrock-split)`
