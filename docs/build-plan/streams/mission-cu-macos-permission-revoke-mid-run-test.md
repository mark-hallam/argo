# mission-cu-macos-permission-revoke-mid-run-test

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `ba94edd63` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-cu-macos-permission-revoke-mid-run-test`.

🟢 **P3 RESILIENCE — partial-failure audit-chain integrity.** Source: evaluator round-4 (`docs/skycot/evals/scheduled/2026-05-22/computer-use-macos-executor-unit-tests.md`):

> "`mission-cu-macos-permission-revoke-mid-run-test`: a tokio test that simulates the operator revoking screen-recording permission mid-step, asserting the executor returns `MacosCuError::PermissionLost(...)` cleanly without leaking partial screenshots into the audit chain."

Context: round 4 confirmed 37/37 unit tests pass for `argo-executor-computer-use-macos`. But the unit tests don't cover the partial-failure path where macOS revokes screen-recording permission mid-execution (e.g. the operator opens System Settings > Privacy & Security and toggles off the app's access while a long-running run-loop is in progress).

The expected behavior: executor detects the revoke on the NEXT `screencapture` call, returns a typed `MacosCuError::PermissionLost(...)` (or whatever the variant is named), and crucially — does NOT write the partial screenshot data into the audit chain. Half-screenshots in the audit log are worse than no screenshot at all: they imply a successful step that didn't actually complete.

## Scope

Add a tokio integration test that:

1. Constructs a `MacosCuExecutor` with a mock screen-capture provider (or whatever test seam exists — find via `grep ScreenCaptureProvider crates/argo-executor-computer-use-macos/src/`).
2. Drives a multi-step run-loop where step 1 succeeds (real screenshot, real audit row written), step 2's `capture()` call returns a simulated permission-revoke error (the test injects the error via the mock provider).
3. Asserts:
   - The executor returns `MacosCuError::PermissionLost(...)` (or the actual variant — discover from the crate's error.rs).
   - The audit chain after the test has EXACTLY ONE step row (step 1's complete row), NOT two with step 2 in partial state.
   - The run-loop's exit cleanup released any in-progress resources (e.g. no leftover screenshot file at `$ARGO_HOME/cu-macos/runs/<run_id>/step-2.png`).

4. **Paired audit-chain integrity test**: after the partial-failure, call `crate::audit_chain::verify_audit_chain_for_run(run_id)` (or equivalent) and assert it returns `Ok` — the chain isn't broken by the failure.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `NEW: crates/argo-executor-computer-use-macos/tests/permission_revoke_mid_run_test.rs` (~100-150 LOC).
- `crates/argo-executor-computer-use-macos/Cargo.toml` — narrowly scoped: add `[dev-dependencies]` if missing.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- The executor's error variants — this brief is a TEST. If a new variant `PermissionLost` is needed, that's a separate brief.
- `argo-audit` crate — out of scope.

## Acceptance criteria

- [ ] Test file exists with the spec'd path.
- [ ] Test asserts: typed error variant on revoke, audit chain has only completed steps, no partial-state screenshot files left on disk.
- [ ] Test passes with `cargo test -p argo-executor-computer-use-macos --test permission_revoke_mid_run_test`.
- [ ] If the test reveals the executor DOES leak partial screenshots → ship `#[ignore]`'d + FINDINGS-ONLY PR-body note. Conductor authors fix-brief.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `mission-cu-macos-permission-revoke-mid-run-test`
- PR title: `test(cu-macos): permission revoke mid-run audit-chain integrity (mission-cu-macos-permission-revoke-mid-run-test)`

## Notes for the implementer

- ~100-180 LOC expected. Single integration test.
- **The audit-chain integrity assertion is load-bearing.** Without it, this is just "did the executor return an error?" which is a weak contract.
- **Use the existing executor test harness pattern** — clone the setup from one of the 37 passing unit tests.
- Cite the evaluator report verbatim in the PR body.
