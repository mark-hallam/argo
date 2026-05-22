# claude-workflow-dag-split — split argo-workflow/src/dag.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main 33fa37a2 on 2026-05-10.

## Why

`crates/argo-workflow/src/dag.rs` is at 453/500 LOC. Pre-emptive split keeps headroom for future feature briefs.

## Approach (concrete)

1. **Identify a cohesive sibling group** — typically the DAG construction/validation logic vs the traversal/topo-sort logic, or `#[cfg(test)]` mod extraction. Read the file end-to-end before deciding the cut.
2. **Extract to sibling** `dag_<topic>.rs` in the same directory.
3. **Re-export via `pub(crate) use`** (or `pub use` for public API).
4. **Keep public API unchanged** — every existing import must resolve identically.
5. **Test.** `cargo clippy -p argo-workflow --all-targets -- -D warnings` clean. `cargo test -p argo-workflow` green.

## Files you may touch

- `crates/argo-workflow/src/dag.rs` (must drop below 400 LOC)
- `crates/argo-workflow/src/dag_<topic>.rs` (NEW, ≤ 300 LOC)
- `crates/argo-workflow/src/lib.rs` — narrowly: 1 `mod` declaration

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-workflow/`

## Acceptance criteria

- [ ] `dag.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-workflow --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-workflow` green
- [ ] Public API unchanged

## Branch + PR

- Branch: `claude-workflow-dag-split`
- PR title: `argo-workflow: split dag.rs below 500-LOC ceiling (claude-workflow-dag-split)`
