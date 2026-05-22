# claude-cli-worker_actions_transitions-split — split argo-cli/src/handlers/worker_actions_transitions.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main 15214e3e on 2026-05-10.

## Why

`crates/argo-cli/src/handlers/worker_actions_transitions.rs` is at 410/500 LOC. Pre-emptive split keeps headroom for future state-transition logic.

## Approach (concrete)

1. **Identify a cohesive sibling group** — typically by transition kind (e.g. happy-path vs revert vs timeout) or by `#[cfg(test)]` mod extraction.
2. **Extract to sibling** `worker_actions_transitions_<topic>.rs`.
3. **Re-export via `pub(crate) use`**.
4. **Test.** `cargo clippy -p argo-cli --all-targets -- -D warnings` clean. `cargo test -p argo-cli` green.

## Files you may touch

- `crates/argo-cli/src/handlers/worker_actions_transitions.rs` (must drop below 400 LOC)
- `crates/argo-cli/src/handlers/worker_actions_transitions_<topic>.rs` (NEW, ≤ 300 LOC)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 `mod` declaration

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-cli/`

## Acceptance criteria

- [ ] `worker_actions_transitions.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` green
- [ ] Public API unchanged

## Branch + PR

- Branch: `claude-cli-worker_actions_transitions-split`
- PR title: `argo-cli: split handlers/worker_actions_transitions.rs below 500-LOC ceiling (claude-cli-worker_actions_transitions-split)`
