# claude-cli-worker_actions_emit_tests-split — split argo-cli/src/handlers/worker_actions_emit_tests.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main 8889b40c on 2026-05-10.

## Why

`crates/argo-cli/src/handlers/worker_actions_emit_tests.rs` is at 426/500 LOC. Pre-emptive split keeps headroom for future test additions.

## Approach (concrete)

1. **Identify a cohesive sibling group** — typically by emit-action kind (notify vs handoff vs success) or by happy-path vs error-path.
2. **Extract to sibling** `worker_actions_emit_tests_<topic>.rs` with `#[path]` include from parent OR as a separate `mod` declaration.
3. **Keep test surface unchanged** — every existing test name remains discoverable.
4. **Test.** `cargo clippy -p argo-cli --all-targets -- -D warnings` clean. `cargo test -p argo-cli` green.

## Files you may touch

- `crates/argo-cli/src/handlers/worker_actions_emit_tests.rs` (must drop below 400 LOC)
- `crates/argo-cli/src/handlers/worker_actions_emit_tests_<topic>.rs` (NEW, ≤ 300 LOC)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 `mod` declaration if needed

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-cli/`

## Acceptance criteria

- [ ] `worker_actions_emit_tests.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` green
- [ ] All test names preserved

## Branch + PR

- Branch: `claude-cli-worker_actions_emit_tests-split`
- PR title: `argo-cli: split worker_actions_emit_tests.rs below 500-LOC ceiling (claude-cli-worker_actions_emit_tests-split)`
