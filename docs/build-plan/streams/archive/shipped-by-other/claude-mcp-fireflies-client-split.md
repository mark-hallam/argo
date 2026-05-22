# claude-mcp-fireflies-client-split — split argo-mcp-connector-fireflies/src/client.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main 7157bae4 on 2026-05-10.

## Why

`crates/argo-mcp-connector-fireflies/src/client.rs` is at 437/500 LOC. Pre-emptive split keeps headroom (mirror of github-issues client split #1176 pattern).

## Approach (concrete)

1. **Identify a cohesive sibling group** — typically by API surface (e.g. transcripts vs meetings vs search) or args-construction helpers.
2. **Extract to sibling** `client_<topic>.rs`.
3. **Re-export via `pub(crate) use`**.
4. **Test.** `cargo clippy -p argo-mcp-connector-fireflies --all-targets -- -D warnings` clean. `cargo test -p argo-mcp-connector-fireflies` green.

## Files you may touch

- `crates/argo-mcp-connector-fireflies/src/client.rs` (must drop below 400 LOC)
- `crates/argo-mcp-connector-fireflies/src/client_<topic>.rs` (NEW, ≤ 300 LOC)
- `crates/argo-mcp-connector-fireflies/src/lib.rs` — narrowly: 1 `mod` declaration

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-mcp-connector-fireflies/`

## Acceptance criteria

- [ ] `client.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-mcp-connector-fireflies --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-mcp-connector-fireflies` green
- [ ] Public API unchanged

## Branch + PR

- Branch: `claude-mcp-fireflies-client-split`
- PR title: `argo-mcp-connector-fireflies: split client.rs below 500-LOC ceiling (claude-mcp-fireflies-client-split)`
