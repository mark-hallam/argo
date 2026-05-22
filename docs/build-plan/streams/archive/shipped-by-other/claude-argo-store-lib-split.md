# V1.x — Architectural prep: split argo-store/src/lib.rs

**Owner**: claude. **Depends on**: nothing (pure refactor).

## Scope

`crates/argo-store/src/lib.rs` is at **504 LOC** — already 4 lines
over the project's 500-LOC ceiling. Two recent codex briefs bumped
into this:

- **codex-action-history-cli (#907)**: brief allowed adding
  `tenant_query.rs` re-export through `lib.rs`. Tab worked around
  by keeping query path in CLI handlers — noted as deviation.
- **codex-tenant-list-show-cli (BLOCKED)**: needs to add
  `tenant_query.rs` re-export through `lib.rs`. Not viable without
  this split.

Future briefs adding any new argo-store sub-module will hit the
same wall. Pure refactor — same pattern as #851 (worker_actions),
#882 (cli_subcommands), #909 (skill handlers), #910 (routes_channels).

## Approach

Inspect `argo-store/src/lib.rs` and identify the natural cleavage
points. Likely candidates:
- The crate's `pub use` re-export block (probably the bulk of the
  504 LOC) — could split into `lib.rs` (top-level + small re-exports)
  + `prelude.rs` (bulk re-exports)
- Or extract module declarations into focused sibling files

Goal: `lib.rs` ≤ 300 LOC with headroom for 5-10 new sub-module
re-exports before any sibling approaches the ceiling.

## Files you may touch
- `crates/argo-store/src/lib.rs` — reduce to ≤300 LOC; keep
  public API surface stable via `pub use`
- `crates/argo-store/src/prelude.rs` (NEW, optional) — or
  whatever sibling files the split chooses
- Any consumer of argo-store types — narrowly: update imports
  ONLY if a symbol moved (mechanical; should be zero edits if
  re-exports are right)

## Acceptance criteria
- [ ] `argo-store/src/lib.rs` ≤ 300 LOC
- [ ] No public-API change (every existing call site still
      compiles unchanged)
- [ ] `cargo build --workspace` clean
- [ ] `cargo test --workspace` — same set of tests pass as
      before (no test deletions; no skips)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Headroom for next 5+ sub-module additions before the
      central file approaches 500 ceiling

## Branch + PR
- Branch: `claude-argo-store-lib-split`
- PR title: `refactor: split argo-store/src/lib.rs under 500 LOC budget (claude-argo-store-lib-split)`
