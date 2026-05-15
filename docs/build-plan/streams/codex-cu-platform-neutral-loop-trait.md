# codex-cu-platform-neutral-loop-trait — Extract platform-neutral CU run-loop trait

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `ea8f41d0` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-cu-platform-neutral-loop-trait`.
**Source**: F-001a in `docs/skycot/audit/2026-05-15-codex-full-architecture-audit.md`. Pairs with `codex-cu-macos-run-loop-wiring` (must MERGE first).

## Scope

V1 SHIP-BLOCKER prep. Current `run_loop` in `crates/argo-executor-computer-use/src/executor_step_helpers.rs:36` is ~190 LOC and tightly coupled to E2B-style step primitives. The macOS executor crate ships step primitives (`preflight`, `warm_up`, `execute_step_recorded`) but has no `run()` orchestrator — see `crates/argo-cli/src/handlers/worker_actions_executor_cu_macos.rs:9`. Extract a `CuStepRunner` trait the existing `run_loop` can call into so a per-platform implementation can satisfy it without duplicating the loop logic.

This brief is the PRECURSOR. It must MERGE before `codex-cu-macos-run-loop-wiring` claims, because that brief implements the trait for the macOS executor.

Scope:
- Define a `CuStepRunner` trait in `argo-executor-computer-use` (Or a new `argo-executor-computer-use-traits` module) with methods matching what `run_loop` currently calls: `preflight`, `warm_up`, `execute_step_recorded` (and any other primitives the loop calls).
- Refactor `run_loop` in `executor_step_helpers.rs` to take a `&dyn CuStepRunner` (or generic `R: CuStepRunner`) parameter instead of hardcoded E2B step calls.
- Provide a default impl of `CuStepRunner` for the existing E2B-style executor so behavior is byte-identical post-refactor.
- Add unit tests proving the trait dispatch produces the same step sequence as before.

LOC: ~300 (mostly refactoring + trait + 1 default impl + tests).

## Files you may touch

- `crates/argo-executor-computer-use/src/executor_step_helpers.rs` — refactor `run_loop` to take `&dyn CuStepRunner`. ≤120 modified lines.
- `crates/argo-executor-computer-use/src/executor.rs` — narrowly: update the `run()` callsite to pass the existing E2B impl. ≤25 modified lines.
- `crates/argo-executor-computer-use/src/cu_step_runner.rs` (NEW) — trait definition + default E2B-style impl. ≤180 LOC.
- `crates/argo-executor-computer-use/src/lib.rs` — `pub mod cu_step_runner;` + re-exports. ≤10 modified lines.
- `crates/argo-executor-computer-use/src/cu_step_runner_tests.rs` (NEW) — trait dispatch + behavior-parity tests. ≤120 LOC.

## Files you must NOT touch

- `crates/argo-executor-computer-use-macos/**` — that crate's wire-up is the FOLLOWUP brief `codex-cu-macos-run-loop-wiring`. DO NOT touch macOS in this brief.
- `crates/argo-cli/src/handlers/worker_actions_executor_cu_macos.rs` — adapter wire-up is the followup.
- Anything outside the may-touch list above.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] `CuStepRunner` trait defined and `run_loop` takes it as parameter.
- [ ] Existing E2B-style executor still works byte-identical (default impl).
- [ ] Behavior-parity test passes: identical step sequence for a known input.
- [ ] `cargo test -p argo-executor-computer-use` passes.
- [ ] `cargo clippy -p argo-executor-computer-use --all-targets -- -D warnings` clean.
- [ ] No file > 500 LOC; no fn > 80 LOC.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `codex-cu-platform-neutral-loop-trait`
- PR title: `refactor(executor-computer-use): extract CuStepRunner trait for platform-neutral run loop (codex-cu-platform-neutral-loop-trait)`

## Notes for the implementer

- This is a PURE REFACTOR. Behavior MUST be byte-identical post-extraction.
- The followup `codex-cu-macos-run-loop-wiring` adds the macOS impl + flips the HumanHandoff to Completed.
- If the existing E2B step calls take async closures or context, the trait methods need to match those signatures exactly.
- If `executor_step_helpers.rs` post-refactor still has a fn > 80 LOC, split it further — that's a hard invariant.
