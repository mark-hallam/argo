# claude-cli-work_artifacts-split — split argo-cli/src/handlers/work_artifacts/mod.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main c780496b on 2026-05-10.

## Why

`crates/argo-cli/src/handlers/work_artifacts/mod.rs` is at 424/500 LOC. Pre-emptive split keeps headroom.

## Approach (concrete)

1. **Identify a cohesive sibling group** — typically by artifact type (e.g. files vs links vs blobs) or rendering vs persistence helpers.
2. **Extract to sibling** within `work_artifacts/` (e.g. `work_artifacts/<topic>.rs`).
3. **Re-export via `pub(crate) use`** so dispatcher imports stay unchanged.
4. **Test.** `cargo clippy -p argo-cli --all-targets -- -D warnings` clean. `cargo test -p argo-cli` green.

## Files you may touch

- `crates/argo-cli/src/handlers/work_artifacts/mod.rs` (must drop below 400 LOC)
- `crates/argo-cli/src/handlers/work_artifacts/<topic>.rs` (NEW, ≤ 300 LOC)

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-cli/`

## Acceptance criteria

- [ ] `work_artifacts/mod.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` green
- [ ] Public API unchanged

## Branch + PR

- Branch: `claude-cli-work_artifacts-split`
- PR title: `argo-cli: split handlers/work_artifacts/mod.rs below 500-LOC ceiling (claude-cli-work_artifacts-split)`
