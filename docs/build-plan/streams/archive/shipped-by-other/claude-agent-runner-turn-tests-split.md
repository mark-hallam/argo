# claude-agent-runner-turn-tests-split — split turn_tests.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep — defensive split.**

**Authored against**: origin/main 6a753bae on 2026-05-10.

## Why

`crates/argo-agent-runner/src/turn_tests.rs` is at 497 LOC — 3 LOC of headroom against the 500-LOC HARD RULE 6 ceiling. Future turn-runner test additions (canary routing, vertical-aware hooks, etc.) will hit the wall. Pure refactor; mirrors proven pattern (#851/#882/#909/#910/#914/#915/#932/#935/#1116/#1121/#1125/#1126/#1130/#1131/#1132/#1133/#1134/#1135/#1136/#1137).

## Approach

1. Group tests by concern (e.g., `turn_run_*` vs. `turn_persist_*` vs. `turn_error_*`).
2. Move one cohesive group to `crates/argo-agent-runner/src/turn_tests_<topic>.rs`.
3. Re-include via `#[cfg(test)] #[path = "..."] mod` (sibling-test convention).

Target: `turn_tests.rs` ≤ 350 LOC.

## Files you may touch

- `crates/argo-agent-runner/src/turn_tests.rs` — extract sibling
- `crates/argo-agent-runner/src/turn_tests_<topic>.rs` (NEW, ≤ 250 LOC)
- `crates/argo-agent-runner/src/lib.rs` — narrowly: 1 declaration if needed

## Acceptance criteria

- [ ] `turn_tests.rs` ≤ 350 LOC
- [ ] `cargo clippy -p argo-agent-runner --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-agent-runner` clean (same test count pre/post split)

## Branch + PR

- Branch: `claude-agent-runner-turn-tests-split`
- PR title: `agent-runner: split turn_tests.rs below 500-LOC ceiling (claude-agent-runner-turn-tests-split)`
