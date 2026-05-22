# mvp-split-work-queue — split work_queue.rs test below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 MVP — proven test-split pattern.**

**Authored against**: origin/main 7fdbbdb8 on 2026-05-10.

## Why

`crates/argo-cli/tests/work_queue.rs` is at **593 LOC** (integration-test file). The 500-LOC ceiling applies to test files too. Refactor mirrors proven test-split pattern (#1116/#1126/#1146).

## Approach

1. Read `work_queue.rs` end-to-end. Group tests by concern (e.g., enqueue vs. claim vs. complete vs. dispatch).
2. Move one cohesive group to `crates/argo-cli/tests/work_queue_<topic>.rs` as a new integration-test file (Rust treats each top-level `tests/*.rs` as its own crate). Re-use any common fixture helpers via a `tests/common/mod.rs` if not already there; if helpers are inlined, hoist them to `tests/common/work_queue_helpers.rs`.
3. NO source-file changes.

Target: `work_queue.rs` ≤ 350 LOC.

## Files you may touch

- `crates/argo-cli/tests/work_queue.rs` — extract group
- `crates/argo-cli/tests/work_queue_<topic>.rs` (NEW, ≤ 250 LOC)
- `crates/argo-cli/tests/common/work_queue_helpers.rs` (NEW only if helpers need hoisting; ≤ 150 LOC)

## Files you must NOT touch

- Any source under `crates/*/src/`
- Other test files

## Acceptance criteria

- [ ] `work_queue.rs` ≤ 350 LOC
- [ ] New test file(s) ≤ 250 LOC each
- [ ] `cargo test -p argo-cli --test work_queue` and the new test(s) pass
- [ ] Same total assertion count pre/post split

## Branch + PR

- Branch: `mvp-split-work-queue`
- PR title: `cli: split tests/work_queue.rs below 500-LOC ceiling (mvp-split-work-queue)`
