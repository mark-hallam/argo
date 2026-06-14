# win-cli-doctor-cross-platform-sweep

**Owner**: windows-fleet — **P2 — make `argo doctor` honest + non-erroring on native Windows (3 shell-out fixes).**
**Depends on**: none.
**Authored against**: origin/main `7f98b30cf` on `2026-06-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh win-cli-doctor-cross-platform-sweep`
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build+verify on Windows; the FIX is cross-platform so the conductor ALSO re-verifies on unix before merge. Push branch, IN_REVIEW; conductor reviews+merges.
Source: docs/skycot/windows/PORTABILITY-AUDIT-2026-06-15.md (findings 3 + 8 + 9 of 9 — the audit's recommended "doctor cross-platform sweep").

Covers brief-ids `win-cli-doctor-gateway-kill-portability` (8),
`win-cli-doctor-cert-summary-sh-portability` (3),
`win-doctor-disk-usage-df-portability` (9) in one PR (one argo-cli build).

## Problem
Three `argo doctor` handlers spawn POSIX-only programs with no Windows path:
- **doctor_gateway.rs:22** — `kill -0 <pid>` liveness probe. No `kill` on
  Windows → `NotFound`. **Latent ERROR**: #2931's smoke passed only because no
  `gateway.pid` existed; any run with a live pid file breaks gateway-status.
- **doctor_network.rs:209** — `sh -c "openssl … 2>/dev/null | openssl …"` for
  cert expiry/fingerprint. No `/bin/sh` on Windows; `.ok()?` swallows the
  spawn error → "certificate details unavailable" (graceful but a doomed spawn).
- **doctor_storage.rs:266** — `df -Pk` for disk capacity. No `df` on Windows →
  `.ok()?` → `None` → misleading capacity `WARN` (graceful but a doomed spawn).

## Fix
- **kill (8) — REAL fix**: extract `pid_is_alive(pid)`; unix `kill -0`
  (unchanged), Windows built-in `tasklist /FI "PID eq <pid>" /NH /FO CSV` (live
  process → quoted pid in stdout). `argo doctor` gateway status now WORKS on
  Windows.
- **cert (3) / df (9) — clean cfg-gate**: on Windows return `None` directly
  (same user-visible "unavailable"/`WARN` as today, but no doomed spawn) and
  document the native follow-up (rustls cert inspection; Win32
  `GetDiskFreeSpaceExW`/`sysinfo`). `Command` import in doctor_storage cfg-gated
  to `not(windows)` to keep the Windows build warning-free. Unix byte-identical.

## Verify
On Windows: `cargo build -p argo-cli --bin argo` clean; `argo doctor` runs (incl.
gateway-status with a pid file present → no spawn error); clippy clean on the 3
touched files (NB: `clippy -p argo-cli --all-targets -D warnings` is blocked by
PRE-EXISTING argo-executor-computer-use-macos dead-code on Windows — not this
change; tracked `win-cu-macos-crate-deadcode-cfg-gate`). Conductor re-verifies unix.
Branch + IN_REVIEW.
