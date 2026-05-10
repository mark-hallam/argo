# claude-cli-cu_replay-split — split argo-cli/src/handlers/cu_replay.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main a7e516d4 on 2026-05-10.

## Why

`crates/argo-cli/src/handlers/cu_replay.rs` is at 416/500 LOC. Pre-emptive split keeps headroom for future CU replay format additions.

## Approach (concrete)

1. **Identify a cohesive sibling group** — typically by output format (markdown vs gif vs ndjson) or by frame-loading helpers.
2. **Extract to sibling** `cu_replay_<topic>.rs`.
3. **Re-export via `pub(crate) use`**.
4. **Test.** `cargo clippy -p argo-cli --all-targets -- -D warnings` clean. `cargo test -p argo-cli` green.

## Files you may touch

- `crates/argo-cli/src/handlers/cu_replay.rs` (must drop below 400 LOC)
- `crates/argo-cli/src/handlers/cu_replay_<topic>.rs` (NEW, ≤ 300 LOC)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 `mod` declaration

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-cli/`
- `crates/argo-cli/src/handlers/cu_replay_tests.rs` (already a sibling)

## Acceptance criteria

- [ ] `cu_replay.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` green
- [ ] Public API unchanged

## Branch + PR

- Branch: `claude-cli-cu_replay-split`
- PR title: `argo-cli: split handlers/cu_replay.rs below 500-LOC ceiling (claude-cli-cu_replay-split)`
