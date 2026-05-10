# claude-cli-tests-vertical-workflow-run-split — split argo-cli/tests/vertical_workflow_run.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main 33fa37a2 on 2026-05-10.

## Why

`crates/argo-cli/tests/vertical_workflow_run.rs` is at 458/500 LOC. Pre-emptive split keeps headroom for future feature briefs.

## Approach (concrete)

1. **Identify a cohesive sibling group** — group tests by workflow phase (e.g. setup/run/teardown) or by vertical scenario. Read the file end-to-end before deciding the cut.
2. **Extract to sibling** `tests/vertical_workflow_run_<topic>.rs`.
3. **Keep public test surface unchanged.**
4. **Test.** `cargo test -p argo-cli --tests` green.

## Files you may touch

- `crates/argo-cli/tests/vertical_workflow_run.rs` (must drop below 400 LOC)
- `crates/argo-cli/tests/vertical_workflow_run_<topic>.rs` (NEW, ≤ 300 LOC)
- `crates/argo-cli/tests/common/` — narrowly: shared test helpers if needed

## Files you must NOT touch

- Any source file outside `crates/argo-cli/tests/`

## Acceptance criteria

- [ ] `vertical_workflow_run.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli --tests` green

## Branch + PR

- Branch: `claude-cli-tests-vertical-workflow-run-split`
- PR title: `argo-cli: split tests/vertical_workflow_run.rs below 500-LOC ceiling (claude-cli-tests-vertical-workflow-run-split)`
