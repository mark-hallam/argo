# claude-cli-tests-gtm-engineer-chain-split — split argo-cli/tests/gtm_engineer_chain.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main 61782374 on 2026-05-10.

## Why

`crates/argo-cli/tests/gtm_engineer_chain.rs` is at 481/500 LOC, tight against the hard ceiling. Pre-emptive split keeps headroom for future feature briefs.

## Approach (concrete)

1. **Identify a cohesive sibling group** — the file has shared helpers (`workspace_root`, `run_argo`, `load_gtm_actions`, `assert_chain`, `fresh_workspace_for_chain`, `execute_and_complete_next_with`) and ~7 `#[test]` fns. Cohesive split candidates: pull the chain-validation tests (`every_chained_target_is_actually_declared`, `every_approval_gated_action_stays_approval_gated`, `buyer_touching_actions_have_no_auto_emit`) into a separate sibling integration-test file, leaving the workflow-execution tests in the original.
2. **Extract to sibling** `crates/argo-cli/tests/gtm_engineer_chain_validation.rs`. Tests are independent binaries; duplicate small helpers as needed or factor into `tests/common/gtm.rs`.
3. **Re-export via `pub(crate) use`** (N/A for tests).
4. **Keep public API unchanged.**
5. **Test.** `cargo clippy -p argo-cli --all-targets -- -D warnings` clean. `cargo test -p argo-cli` green.

## Files you may touch

- `crates/argo-cli/tests/gtm_engineer_chain.rs` (must drop below 400 LOC)
- `crates/argo-cli/tests/gtm_engineer_chain_validation.rs` (NEW, ≤ 300 LOC)
- `crates/argo-cli/tests/common/mod.rs` or `crates/argo-cli/tests/common/gtm.rs` (NEW, only if helpers are factored)

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-cli/tests/`

## Acceptance criteria

- [ ] `gtm_engineer_chain.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` green
- [ ] Public API unchanged

## Branch + PR

- Branch: `claude-cli-tests-gtm-engineer-chain-split`
- PR title: `argo-cli: split tests/gtm_engineer_chain.rs below 500-LOC ceiling (claude-cli-tests-gtm-engineer-chain-split)`
