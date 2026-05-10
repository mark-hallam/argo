# claude-cli-tests-work-action-transitions-split — split argo-cli/tests/work_action_transitions.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main 33fa37a2 on 2026-05-10.

## Why

`crates/argo-cli/tests/work_action_transitions.rs` is at 458/500 LOC. Pre-emptive split keeps headroom for future feature briefs (proven low-risk pattern, ~30 split-prep PRs merged this session).

## Approach (concrete)

1. **Identify a cohesive sibling group** — group tests by transition class (e.g. happy-path vs error-path, or by ActionState transition). Read the file end-to-end before deciding the cut.
2. **Extract to sibling** `tests/work_action_transitions_<topic>.rs` (Cargo treats each test file as its own binary; or use `tests/common/` shared helpers if needed).
3. **Keep public test surface unchanged** — every existing test name remains discoverable.
4. **Test.** `cargo test -p argo-cli --test work_action_transitions` and `cargo test -p argo-cli --test work_action_transitions_<topic>` both green.

## Files you may touch

- `crates/argo-cli/tests/work_action_transitions.rs` (must drop below 400 LOC)
- `crates/argo-cli/tests/work_action_transitions_<topic>.rs` (NEW, ≤ 300 LOC)
- `crates/argo-cli/tests/common/` — narrowly: shared test helpers if needed

## Files you must NOT touch

- Any source file outside `crates/argo-cli/tests/`

## Acceptance criteria

- [ ] `work_action_transitions.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli --tests` green (pre-existing flakes outside may-touch don't block per HARD RULE 2)

## Branch + PR

- Branch: `claude-cli-tests-work-action-transitions-split`
- PR title: `argo-cli: split tests/work_action_transitions.rs below 500-LOC ceiling (claude-cli-tests-work-action-transitions-split)`
