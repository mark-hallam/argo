# claude-cli-handlers-worker_create-split — split argo-cli/src/handlers/worker_create.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main 3521821d on 2026-05-10.

## Why

`crates/argo-cli/src/handlers/worker_create.rs` is at 439/500 LOC. Pre-emptive split keeps headroom for future feature briefs.

## Approach (concrete)

1. **Identify a cohesive sibling group** — typically by sub-step (e.g. validation vs persistence vs CLI-output) or `#[cfg(test)]` mod extraction. Read end-to-end.
2. **Extract to sibling** `worker_create_<topic>.rs`.
3. **Re-export via `pub(crate) use`**.
4. **Test.** `cargo clippy -p argo-cli --all-targets -- -D warnings` clean. `cargo test -p argo-cli` green.

## Files you may touch

- `crates/argo-cli/src/handlers/worker_create.rs` (must drop below 400 LOC)
- `crates/argo-cli/src/handlers/worker_create_<topic>.rs` (NEW, ≤ 300 LOC)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 `mod` declaration

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-cli/`

## Acceptance criteria

- [ ] `worker_create.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` green
- [ ] Public API unchanged

## Branch + PR

- Branch: `claude-cli-handlers-worker_create-split`
- PR title: `argo-cli: split handlers/worker_create.rs below 500-LOC ceiling (claude-cli-handlers-worker_create-split)`
