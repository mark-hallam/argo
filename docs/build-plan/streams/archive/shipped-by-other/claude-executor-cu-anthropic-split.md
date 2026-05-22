# claude-executor-cu-anthropic-split — split argo-executor-computer-use/src/anthropic.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main c780496b on 2026-05-10.

## Why

`crates/argo-executor-computer-use/src/anthropic.rs` is at 416/500 LOC. Pre-emptive split keeps headroom for future Anthropic CU API extensions.

## Approach (concrete)

1. **Identify a cohesive sibling group** — typically by request/response shape vs streaming handling vs auth, or `#[cfg(test)]` mod extraction.
2. **Extract to sibling** `anthropic_<topic>.rs`.
3. **Re-export via `pub(crate) use`**.
4. **Test.** `cargo clippy -p argo-executor-computer-use --all-targets -- -D warnings` clean. `cargo test -p argo-executor-computer-use` green.

## Files you may touch

- `crates/argo-executor-computer-use/src/anthropic.rs` (must drop below 400 LOC)
- `crates/argo-executor-computer-use/src/anthropic_<topic>.rs` (NEW, ≤ 300 LOC)
- `crates/argo-executor-computer-use/src/lib.rs` — narrowly: 1 `mod` declaration

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-executor-computer-use/`

## Acceptance criteria

- [ ] `anthropic.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-executor-computer-use --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-executor-computer-use` green
- [ ] Public API unchanged

## Branch + PR

- Branch: `claude-executor-cu-anthropic-split`
- PR title: `argo-executor-computer-use: split anthropic.rs below 500-LOC ceiling (claude-executor-cu-anthropic-split)`
