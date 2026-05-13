# claude-wizard-onboard-split — split 482-LOC `wizard_onboard.rs` step functions to siblings

**Owner**: claude. **Depends on**: nothing (pure refactor).
**Authored against**: origin/main `f252a46b` on `2026-05-14`.

> **URGENT split-prep — blocker for `K4-deploy-first-run-setup-wizard-v2`.** `crates/argo-cli/src/handlers/wizard_onboard.rs` is at 482 LOC (over the 480 pre-claim threshold). Currently holds 5 step functions + helper types + tests.

## Scope

Extract the 5 step functions into a sibling `wizard_onboard_steps.rs`:
- `step_detect` (~30 LOC)
- `step_provider_key` (~50 LOC)
- `step_operator` (~60 LOC)
- `step_vertical` (~30 LOC)
- `step_worker` (~50 LOC)
- Plus `existing_operator_block` helper

`wizard_onboard.rs` keeps `Answers` struct, `PromptIo` trait, `KeyVerifier` trait, `run_steps` orchestrator, helper validators (`is_valid_worker_name`, `is_plausible_email`), and the top-level `run` entry. Target: ~250 LOC.

## Files you may touch

- `crates/argo-cli/src/handlers/wizard_onboard.rs` — slim from 482 → ~250 LOC.
- `<NEW: crates/argo-cli/src/handlers/wizard_onboard_steps.rs>` (~250 LOC).
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: ONE `pub(crate) mod wizard_onboard_steps;` line.

## Files you must NOT touch

- `wizard_onboard_tests.rs` — locked (siblings reference functions; verify after split).
- `argo-cli/src/handlers/onboard.rs` + `setup.rs` — locked (separate handlers).
- Step contract (signatures + behavior) — verbatim move.

## Acceptance

- [ ] `wizard_onboard.rs` ≤ 300 LOC.
- [ ] `wizard_onboard_steps.rs` ≤ 300 LOC.
- [ ] `existing_operator_block` exposed as `pub` (needed by K4-v2 browser route).
- [ ] `cargo test -p argo-cli wizard_onboard` passes.
- [ ] Workspace clippy clean.

## Branch
- `claude-wizard-onboard-split`

## Notes
- `-split` suffix → ceiling-check auto-skips.
- Mechanical; follow `boot.rs` split pattern.
