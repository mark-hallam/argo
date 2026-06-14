# win-sandbox-localsandbox-bash-cfg-gate

**Owner**: windows-fleet — **P2 CROSS-PLATFORM — code-exec E2B fallback fails `NotFound` on native Windows.**
**Depends on**: none.
**Authored against**: origin/main `20fbc6b8c` on `2026-06-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh win-sandbox-localsandbox-bash-cfg-gate`
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build+verify on Windows; the FIX is cross-platform so the conductor ALSO re-verifies on unix before merge. Push branch, IN_REVIEW; conductor reviews+merges.
Source: docs/skycot/windows/PORTABILITY-AUDIT-2026-06-15.md (finding 2 of 9).

## Problem
`crates/argo-sandbox/src/local.rs:30` hardcoded `tokio::process::Command::new("bash")`
in the production `impl Sandbox for LocalSandbox` (NOT cfg-gated — only the
tests were `#[cfg(all(test, unix))]`). `LocalSandbox` is the E2B fallback the
code-execution executor uses when `ARGO_E2B_TOKEN` is unset
(`argo-cli/src/handlers/worker_actions_executor_code.rs:258` constructs it
unconditionally on all platforms). On native Windows `bash` is not guaranteed →
`NotFound` at runtime.

## Fix
The caller constructs `LocalSandbox` unconditionally, so cfg-gating the `impl`
away would break argo-cli's Windows COMPILE. Instead route through the platform
shell (same approach as #2915 / win-terminal-streaming-bash-portability): a
local `shell_command` helper (`cmd /C` on Windows, `bash -c` on unix) — a
sibling copy of `argo_tool::process_registry::shell_command` because argo-tool
depends on argo-sandbox (sharing the seam would cycle). Command syntax stays
caller-responsibility per platform. Unix byte-identical. New cross-platform
`echo` smoke test (runs on Windows AND the swarm).

## Verify
On Windows: `cargo test -p argo-sandbox --lib` green (incl. the new portable
`runs_through_platform_shell`) + clippy clean + local.rs < 500 LOC. Conductor
re-verifies unix (bash -c path unchanged).

## NOTE — pre-existing RED on main (NOT this change)
`cargo test -p argo-sandbox --test v1_deferred_status_pin` FAILS on
origin/main `20fbc6b8c` independent of this diff: the deferred-status pin
(snapshot vs `140db7c03`, 2026-05-24) flags
`crates/argo-cli/src/handlers/worker_actions_executor_cu_windows_tests.rs` as a
new `argo_executor_computer_use_windows::` caller not in its allowlist (4 refs
on origin/main; my diff touches only argo-sandbox/src). Tracked + fixed
separately as `win-deferred-status-pin-refresh-cu-windows-test-caller`. Flagged
because a RED argo-sandbox suite on main may be feeding a DEC-055 merge freeze.
