# win-terminal-streaming-bash-portability

**Owner**: windows-fleet — **P1 CROSS-PLATFORM — complete feature break on native Windows.**
**Depends on**: none.
**Authored against**: origin/main `6298e118d` on `2026-06-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh win-terminal-streaming-bash-portability`
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build+verify on Windows; the FIX is cross-platform so the conductor ALSO re-verifies on unix before merge. Push branch, IN_REVIEW; conductor reviews+merges.
Source: docs/skycot/windows/PORTABILITY-AUDIT-2026-06-15.md (finding 1 of 9).

## Problem
`crates/argo-tool/src/tools/terminal_streaming.rs:39` hardcodes
`tokio::process::Command::new("bash").arg("-c").arg(command)` in production
(`pub(crate) async fn run_streaming`, the streaming bypass taken when
`ToolContext::progress_tx` is `Some`, i.e. whenever an agent uses `terminal`
with a progress callback). Windows has no guaranteed `bash` (ships only with
Git/WSL/MSYS2), so `Command::new("bash")` fails `NotFound` at runtime on
`x86_64-pc-windows-msvc` → the streaming `terminal` path is a **complete
feature break** on native Windows (no graceful fallback).

This is the SAME bug #2915 fixed in `ProcessRegistry::spawn` — but
`terminal_streaming` was missed (it spawns through its own hardcoded `bash`,
not the shared `shell_command` seam).

## Fix
Reuse the existing, already-merged shell-selection seam instead of adding a
second copy (no parallel abstraction): promote
`crate::process_registry::shell_command` to `pub(crate)` and call it from
`run_streaming` (`cmd /C` on Windows, `bash -c` on unix; command *syntax* stays
caller-responsibility per platform). Unix behaviour is byte-identical. Add a
cross-platform regression test (`echo <token>` is portable across the seam) so
the swarm covers it on macOS/Linux too.

## Verify
On Windows: `cargo test -p argo-tool --lib run_streaming_uses_platform_shell_and_streams`
passes + full `cargo test -p argo-tool --lib` green (no regression, incl.
`process_registry` spawn round-trip after the visibility bump) + clippy clean on
the two touched files. Conductor re-verifies unix (`bash -c` path unchanged).
Branch + IN_REVIEW.
