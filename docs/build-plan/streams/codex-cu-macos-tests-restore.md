# codex-cu-macos-tests-restore — Restore 4 cu_macos unit tests broken by #1722 macOS run-loop wiring

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `65a12698` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-cu-macos-tests-restore`.
**Source**: Pre-existing failures flagged in #1727 (codex-toolcontext-tenant) PR body. Codex verified the failures reproduce on current `origin/main` but did not author them — likely regressed by #1722 (codex-cu-macos-run-loop-wiring) which added autonomous CU loop to `MacosCuExecutor::run` and changed adapter return shape.

## Scope

P3 cleanup. Four unit tests in `crates/argo-cli/src/handlers/worker_actions_executor_cu_macos.rs` (test module) fail on current main:

- `cu_macos_tests::dispatch_on_macos_writes_bundle`
- `cu_macos_tests::manifest_hyphen_alias_resolves_to_macos_kind`
- `cu_macos_tests::manifest_underscore_form_resolves_to_macos_kind`
- `cu_macos_tests::unknown_kind_with_macos_in_name_does_not_resolve`

The `notes_smoke` integration test (scenario #20 in `scripts/smoke-skycot.sh`) PASSES, so the autonomous CU loop is working end-to-end. The unit tests likely use old fixture patterns that didn't survive #1722's adapter rewrite (which now calls `exec.run(req)` and maps `stop_success → Completed`).

The Linux + Windows sibling test files (`worker_actions_executor_cu_linux_tests.rs`, `worker_actions_executor_cu_windows_tests.rs`) still have the analogous tests that pass. Use them as the structural reference: the macOS tests should follow the same fixture pattern but for the macOS executor.

Scope:
- Read the 3 sibling test files (cu_linux_tests, cu_windows_tests, cu_macos_tests/inline-test-module) to identify the structural template the tests should follow.
- Read `worker_actions_executor_cu_macos.rs` production code to see what changed in #1722 (new `run()` callsite, different outcome mapping).
- Update the 4 failing tests so they match the new production surface:
  - `dispatch_on_macos_writes_bundle` — verify the bundle is still written on `HumanHandoff` (preflight-failure path), since `Completed` no longer writes a bundle (autonomous-success path).
  - 3 `manifest_*_resolves_to_macos_kind` tests — likely just need the executor kind/dispatch shape updated.
  - `unknown_kind_with_macos_in_name_does_not_resolve` — negative test; check the resolver still rejects unknown kinds even when "macos" is in the name.
- Run the tests locally to confirm they pass.

LOC: ~80.

## Files you may touch

- `crates/argo-cli/src/handlers/worker_actions_executor_cu_macos.rs` — narrowly: the inline `#[cfg(test)] mod cu_macos_tests { ... }` block. ≤80 modified lines. DO NOT modify production code unless a test failure surfaces an actual bug in production.

## Files you must NOT touch

- Production `worker_actions_executor_cu_macos.rs` non-test code.
- `crates/argo-executor-computer-use-macos/**` — the macOS executor crate itself.
- The `cu_linux_tests.rs` / `cu_windows_tests.rs` sibling files (those are the working reference, don't change them).
- Anything outside the may-touch list above.

## Acceptance criteria

- [ ] All 4 named tests pass: `cargo test -p argo-cli cu_macos_tests` returns 4 passed / 0 failed.
- [ ] No production code changes — confirm with `git diff --stat` showing only the test block touched.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Scenario #20 of `scripts/smoke-skycot.sh` (macOS notes_smoke) still passes (was passing before; should stay passing).
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `codex-cu-macos-tests-restore`
- PR title: `test(cli): restore 4 cu_macos unit tests after #1722 adapter rewrite (codex-cu-macos-tests-restore)`

## Notes for the implementer

- The `notes_smoke` integration test passing means the autonomous CU loop works end-to-end. These unit tests are about the dispatch RESOLUTION + adapter wiring, not the loop itself. Updating them should be straightforward.
- If you find that a test was testing behavior that #1722 LEGITIMATELY removed (e.g., "always returns HumanHandoff" — which #1722 explicitly changed to return Completed on success), update the test to reflect the new contract, not to restore the old behavior.
- The Linux + Windows sibling tests should be the structural template; the macOS test module should match their shape.
