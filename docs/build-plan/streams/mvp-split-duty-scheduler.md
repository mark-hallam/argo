# mvp-split-duty-scheduler — split duty_scheduler.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 MVP — proven split pattern.**

**Authored against**: origin/main 7fdbbdb8 on 2026-05-10.

## Why

`crates/argo-orchestrator/src/duty_scheduler.rs` is at **672 LOC** — 172 over the 500-LOC ceiling. Blocks duty-scheduler extension briefs and trips clippy when CI re-enables.

## Approach

1. Read `duty_scheduler.rs` end-to-end. Identify a cohesive section (likely cron-tick eval vs. duty-fire persist vs. recovery helpers).
2. Move to `crates/argo-orchestrator/src/duty_scheduler_<topic>.rs`. Two siblings if natural.
3. Re-export via `pub(crate) use`.

Target: `duty_scheduler.rs` ≤ 350 LOC.

## Files you may touch

- `crates/argo-orchestrator/src/duty_scheduler.rs`
- `crates/argo-orchestrator/src/duty_scheduler_<topic>.rs` (NEW, ≤ 250 LOC each)
- `crates/argo-orchestrator/src/lib.rs` — narrowly: 1-2 `mod` declarations if needed

## Files you must NOT touch

- `crates/argo-cli/tests/duty_scheduler_gateway_boot.rs`
- `crates/argo-orchestrator/tests/duty_scheduler_integration.rs`
- Other orchestrator source files

## Acceptance criteria

- [ ] `duty_scheduler.rs` ≤ 350 LOC
- [ ] Each sibling ≤ 250 LOC
- [ ] `cargo clippy -p argo-orchestrator --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-orchestrator` clean
- [ ] No public API changes

## Branch + PR

- Branch: `mvp-split-duty-scheduler`
- PR title: `orchestrator: split duty_scheduler.rs below 500-LOC ceiling (mvp-split-duty-scheduler)`
