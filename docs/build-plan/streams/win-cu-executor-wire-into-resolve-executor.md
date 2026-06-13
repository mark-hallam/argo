# win-cu-executor-wire-into-resolve-executor

**Owner**: windows-fleet — **P1 — DEC-054 keystone: make Windows a real executor target.**
**Depends on**: none.
**Authored against**: origin/main `033c9589c` on `2026-06-14`.
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build+verify ON REAL WINDOWS; cross-platform changes → conductor re-verifies unix (RUN tests, not just compile) before merge. Push branch, IN_REVIEW; conductor reviews+merges.

## Goal (DEC-054, operator-locked 2026-06-14)
Wire `argo_executor_computer_use_windows::WindowsCuExecutor` into argo-cli's
`resolve_executor` so a Windows host dispatches computer-use actions to it.
Today it compiles + its primitives are proven (#2906/#2910/#2919) but `run()`
is unreachable from dispatch (noted in executor.rs "not yet wired").

## Fix
In `resolve_executor` (grep `fn resolve_executor` in argo-cli handlers), add a
`#[cfg(target_os = "windows")]` arm that returns the WindowsCuExecutor for the
computer-use action types, mirroring the macOS executor arm. Keep it behind the
SAME opt-in/default-OFF posture as the macOS native executor (no behavior change
for macOS/Linux — conductor re-verifies). Honor the truth-boundary + capability
fence exactly as the macOS path does.

## Verify
On Windows: a computer-use action resolves to WindowsCuExecutor through the real
dispatch path (not a direct construct). Conductor re-verifies on macOS: the
resolve_executor unix/macOS arms are byte-unchanged + argo-cli tests green.
Branch + IN_REVIEW.
