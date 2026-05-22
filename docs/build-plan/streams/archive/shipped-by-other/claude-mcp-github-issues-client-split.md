# claude-mcp-github-issues-client-split — split argo-mcp-connector-github-issues/src/client.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main cd34d607 on 2026-05-10.

## Why

`argo-mcp-connector-github-issues/src/client.rs` is at 465/500 LOC, tight against the hard ceiling. Several active briefs need to add fields/methods here; pre-emptive split unblocks them.

## Approach (concrete)

1. **Identify a cohesive sibling group** — typically one of: a sub-domain of helpers, a struct + its impl block, a `#[cfg(test)]` mod, a stateful type's persistence-layer code, or a renderer/formatter cluster. Read the file end-to-end before deciding the cut line.
2. **Extract to sibling** `client_<topic>.rs` in the same directory.
3. **Re-export via `pub(crate) use`** (or `pub use` if the symbols are part of the crate's public API).
4. **Keep public API unchanged** — every existing `crate::<module>::Name` import must still resolve identically.
5. **Test.** `cargo clippy -p argo-mcp-connector-github-issues --all-targets -- -D warnings` clean. `cargo test -p argo-mcp-connector-github-issues` green (pre-existing flakes outside may-touch don't block per HARD RULE 2).

## Files you may touch

- `crates/argo-mcp-connector-github-issues/src/client.rs` (must drop below 400 LOC after split)
- `crates/argo-mcp-connector-github-issues/src/client_<topic>.rs` (NEW, ≤ 300 LOC)
- `crates/argo-mcp-connector-github-issues/src/lib.rs` — narrowly: 1 `mod` declaration

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-mcp-connector-github-issues/`

## Acceptance criteria

- [ ] `client.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-mcp-connector-github-issues --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-mcp-connector-github-issues` green
- [ ] Public API unchanged (no doc-test breakage)

## Branch + PR

- Branch: `claude-mcp-github-issues-client-split`
- PR title: `argo-mcp-connector-github-issues: split client.rs below 500-LOC ceiling (claude-mcp-github-issues-client-split)`
