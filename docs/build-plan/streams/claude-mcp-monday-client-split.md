# claude-mcp-monday-client-split — split argo-mcp-connector-monday/src/client.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main 0dcb8cff on 2026-05-10.

## Why

`crates/argo-mcp-connector-monday/src/client.rs` is at 408/500 LOC. Pre-emptive split keeps headroom (mirror of github-issues #1176 and fireflies #1211 client-split pattern).

## Approach (concrete)

1. **Identify a cohesive sibling group** — typically by API surface or args-construction helpers.
2. **Extract to sibling** `client_<topic>.rs`.
3. **Re-export via `pub(crate) use`**.
4. **Test.** `cargo clippy -p argo-mcp-connector-monday --all-targets -- -D warnings` clean. `cargo test -p argo-mcp-connector-monday` green.

## Files you may touch

- `crates/argo-mcp-connector-monday/src/client.rs` (must drop below 400 LOC)
- `crates/argo-mcp-connector-monday/src/client_<topic>.rs` (NEW, ≤ 300 LOC)
- `crates/argo-mcp-connector-monday/src/lib.rs` — narrowly: 1 `mod` declaration

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-mcp-connector-monday/`

## Acceptance criteria

- [ ] `client.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-mcp-connector-monday --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-mcp-connector-monday` green
- [ ] Public API unchanged

## Branch + PR

- Branch: `claude-mcp-monday-client-split`
- PR title: `argo-mcp-connector-monday: split client.rs below 500-LOC ceiling (claude-mcp-monday-client-split)`
