# mvp-split-work-multi-step — split work_multi_step.rs test below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 MVP — proven test-split pattern.**

**Authored against**: origin/main 7fdbbdb8 on 2026-05-10.

## Why

`crates/argo-cli/tests/work_multi_step.rs` is at **528 LOC** (integration-test file). 28 over the 500-LOC ceiling.

## Approach

1. Read `work_multi_step.rs` end-to-end. Group tests by concern (e.g., happy-path multi-step vs. failure-recovery vs. parallel-fanout).
2. Move one cohesive group to `crates/argo-cli/tests/work_multi_step_<topic>.rs`.
3. NO source-file changes.

Target: `work_multi_step.rs` ≤ 350 LOC.

## Files you may touch

- `crates/argo-cli/tests/work_multi_step.rs`
- `crates/argo-cli/tests/work_multi_step_<topic>.rs` (NEW, ≤ 250 LOC)

## Files you must NOT touch

- Any source under `crates/*/src/`

## Acceptance criteria

- [ ] `work_multi_step.rs` ≤ 350 LOC
- [ ] Sibling ≤ 250 LOC
- [ ] Both test binaries compile and pass
- [ ] Same assertion count pre/post split

## Branch + PR

- Branch: `mvp-split-work-multi-step`
- PR title: `cli: split tests/work_multi_step.rs below 500-LOC ceiling (mvp-split-work-multi-step)`
