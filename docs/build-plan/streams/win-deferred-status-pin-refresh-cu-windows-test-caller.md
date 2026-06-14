# win-deferred-status-pin-refresh-cu-windows-test-caller

**Owner**: windows-fleet — **P1 RED-ON-MAIN — `cargo test -p argo-sandbox` fails on origin/main; possible DEC-055 merge-freeze cause.**
**Depends on**: none.
**Authored against**: origin/main `9d8ba18c0` on `2026-06-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh win-deferred-status-pin-refresh-cu-windows-test-caller`
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). The fix is platform-agnostic (a workspace-pin allowlist edit); verified on Windows, conductor re-verifies on unix. Push branch, IN_REVIEW; conductor reviews+merges.
Discovered while verifying `win-sandbox-localsandbox-bash-cfg-gate` (#2948).

## Problem
`crates/argo-sandbox/tests/v1_deferred_status_pin.rs::deferred_crates_have_no_surprise_callers`
FAILS on origin/main itself (reproduced on `20fbc6b8c` and `9d8ba18c0`). The pin
snapshots the deferred-substrate caller set vs `140db7c03` (2026-05-24); since
then the Windows CU executor work added
`crates/argo-cli/src/handlers/worker_actions_executor_cu_windows_tests.rs` as a
referencer of `argo_executor_computer_use_windows::` (4 substring refs, inside
`#[cfg(target_os = "windows")]` test blocks). The pin walks `src/` (which
includes co-located `*_tests.rs` modules) and that file is not in the
`argo-executor-computer-use-windows` allowlist → drift → test panics.

This is a STALE PIN, not a real promotion: the reference is a TEST module, which
the pin's own docstring says is allowed ("the only references are tests and
capability declarations"). A RED `argo-sandbox` suite on main is a strong
candidate for the DEC-055 merge freeze that is currently stalling the
Windows-fleet PR queue.

## Fix
Add `worker_actions_executor_cu_windows_tests.rs` to the
`argo-executor-computer-use-windows` `allowed_callers` list — exactly the
convention already used for `cu_replay_tests.rs` in the
`argo-executor-computer-use` entry (test files ARE listed in allowlists). No
deferral flip (production caller set unchanged), so CLAUDE.md `v1.0 strategic
decisions` is untouched. Minimal + convention-consistent.

## Verify
On Windows: `cargo test -p argo-sandbox` fully green (`v1_deferred_status_pin`
4/4 + lib + other integration). Conductor re-verifies on unix. Branch + IN_REVIEW.
