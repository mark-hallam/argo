# codex-cu-macos-run-loop-wiring — Wire macOS CU executor through the run-loop trait (V1 SHIP-BLOCKER)

**Owner**: codex. **Depends on**: `codex-cu-platform-neutral-loop-trait` MERGED.
**Authored against**: origin/main `ea8f41d0` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-cu-macos-run-loop-wiring`. ALSO check `codex-cu-platform-neutral-loop-trait` is MERGED.
**Source**: F-001b in `docs/skycot/audit/2026-05-15-codex-full-architecture-audit.md`. The v1 ship-blocker per `CLAUDE.md:103` macOS-first decision and `docs/skycot/V1_DEFINITION.md` demo gate.

## Scope

🔴 V1 SHIP-BLOCKER. Native macOS computer-use currently resolves but does not run the autonomous Anthropic CU loop. The adapter at `crates/argo-cli/src/handlers/worker_actions_executor_cu_macos.rs:9` documents that the macOS crate has step primitives but no `run()` orchestrator, and the executor at `:91` returns `ActionOutcome::HumanHandoff { bundle_path }` after preflight, writing instructions telling the operator to carry the action out manually at `:157`.

With `codex-cu-platform-neutral-loop-trait` shipping the `CuStepRunner` trait, this brief:
- Implements `CuStepRunner` for the macOS executor's step primitives (`preflight`, `warm_up`, `execute_step_recorded`).
- Wires the macOS adapter at `worker_actions_executor_cu_macos.rs` to construct the macOS step runner and call `run_loop` with it.
- Returns `ActionOutcome::Completed` on `stop_success` instead of `HumanHandoff`.
- Preserves the HumanHandoff path as a fallback ONLY when preflight gaps make autonomous run impossible.

LOC: ~600 (trait impl + adapter rewrite + tests).

## Files you may touch

- `crates/argo-executor-computer-use-macos/src/executor.rs` — implement `CuStepRunner` for the macOS step API. ≤180 modified lines.
- `crates/argo-executor-computer-use-macos/src/lib.rs` — re-export the new impl. ≤15 modified lines.
- `crates/argo-executor-computer-use-macos/src/run_loop_tests.rs` (NEW) — autonomous-loop tests with mocked step primitives. ≤200 LOC.
- `crates/argo-cli/src/handlers/worker_actions_executor_cu_macos.rs` — replace HumanHandoff bundle logic with run_loop invocation; preserve HumanHandoff only for preflight-failed case. ≤180 modified lines.
- `crates/argo-cli/src/handlers/worker_actions_executor_cu_macos_tests.rs` (NEW or sibling extend) — adapter-level tests for Completed-vs-HumanHandoff branching. ≤120 LOC.

## Files you must NOT touch

- `crates/argo-executor-computer-use/**` — that crate's surface is set by the precursor brief. Don't refactor it again here.
- Anything outside the may-touch list above.

## Acceptance criteria

- [ ] macOS executor implements `CuStepRunner` (the trait from precursor brief).
- [ ] Adapter constructs macOS step runner + calls `run_loop`.
- [ ] On `stop_success`: returns `ActionOutcome::Completed`.
- [ ] On preflight failure: still returns `HumanHandoff` (preserved fallback).
- [ ] Integration test proves an autonomous macOS CU action completes via mocked step primitives (no HumanHandoff).
- [ ] `cargo test -p argo-executor-computer-use-macos` passes.
- [ ] `cargo test -p argo-cli worker_actions_executor_cu_macos` passes.
- [ ] `cargo clippy -p argo-executor-computer-use-macos -p argo-cli --all-targets -- -D warnings` clean.
- [ ] No file > 500 LOC; no fn > 80 LOC.
- [ ] BUILD_STATUS.md row updated with PR number.
- [ ] PR body explicitly notes this resolves the v1 SHIP-BLOCKER for `V1_DEFINITION.md` macOS demo gate.

## Branch + PR

- Branch: `codex-cu-macos-run-loop-wiring`
- PR title: `feat(executor-cu-macos): wire autonomous CU run loop, return Completed on stop_success (codex-cu-macos-run-loop-wiring) — V1 SHIP-BLOCKER`

## Notes for the implementer

- This is THE v1 ship-blocker. The demo gate at `V1_DEFINITION.md` requires macOS CU to run autonomously, not handoff to operator.
- Operator decision (Q1, 2026-05-15): macOS-first stays AND E2B remains operational. Both paths work; macOS is the primary v1 demo target.
- Preserve the HumanHandoff fallback for preflight-failed case (missing macOS permissions) — that's a legitimate operator-handoff trigger, not a stub.
- The `stop_success` branch is the autonomous-completion signal from the loop. Map it to `ActionOutcome::Completed { ... }` with whatever metadata the existing CU outcome carries.
- If macOS step primitives need additional state (a11y handle, screen recorder), thread it through the trait impl's `&self`, not via globals.
