# win-doctor-cross-platform-shellouts

**Owner**: windows-fleet — **P2 CROSS-PLATFORM — `argo doctor` shells out to three unix-only binaries (`sh`, `kill`, `df`) → broken/incomplete diagnostics on native Windows.**
**Depends on**: none.
**Authored against**: origin/main `069d57962` on `2026-06-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh win-doctor-cross-platform-shellouts`
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build+verify on Windows; the FIX is cross-platform (`#[cfg(unix)]` keeps the unix path byte-identical) so the conductor ALSO re-verifies on unix before merge. Push branch, IN_REVIEW; conductor reviews+merges. **Do NOT self-merge.**
Source: docs/skycot/windows/PORTABILITY-AUDIT-2026-06-15.md — findings 3, 8, 9 of 9 (the three remaining un-shipped findings; 1/2/4/5/6/7 already merged via #2944/#2948/#2946/#2947/#2945).

## Problem (three `argo doctor` unix-only shell-outs — all VERIFIED present on origin/main `069d57962`)

`argo doctor` is a shipped production command (#2931 smoke). Three of its probes
shell out to unix-only binaries with no `#[cfg(windows)]` fallback:

1. **Finding 8 (most consequential) — `kill -0` PID-liveness.**
   `crates/argo-cli/src/handlers/doctor_gateway.rs:22` —
   `std::process::Command::new("kill").args(["-0", &pid.to_string()])`.
   On Windows there is no `kill` binary → spawn fails. **Latent**: #2931's smoke
   passed only because no `gateway.pid` existed, so `gateway_status()` returned
   early (line ~15) before reaching line 22. Any run with a live PID file →
   broken gateway-status reporting. (Same PID-liveness concern the gateway's own
   `lifecycle.rs::pid_is_alive` already handles via `#[cfg(not(unix))]` — see the
   sibling brief `win-gateway-stop-portability`; mirror that pattern here.)

2. **Finding 3 — cert summary `sh`.**
   `crates/argo-cli/src/handlers/doctor_network.rs:209` —
   `tokio::process::Command::new("sh").arg("-c").arg(cmd)` runs an `openssl`
   pipeline with `2>/dev/null`. No `/bin/sh` on native Windows → `NotFound`,
   swallowed by `.ok()?` (~line 215) → the TLS probe **silently** downgrades to
   "certificate details unavailable" (expiry + SHA256 fingerprint never
   retrieved). Graceful (no crash) but the diagnostic is silently incomplete.

3. **Finding 9 (lowest) — disk usage `df`.**
   `crates/argo-cli/src/handlers/doctor_storage.rs:266` —
   `Command::new("df").arg("-Pk").arg(path)`. No `df` on Windows → spawn fails,
   `.ok()?` → `None` → `capacity_status()` reports WARN/blank instead of real
   disk diagnostics. Graceful, diagnostic-only. **P3** within this brief.

## Fix (one cohesive PR — same crate `argo-cli`, same class; no parallel abstraction)

For each of the three, gate the unix shell-out behind `#[cfg(unix)]` and add a
`#[cfg(not(unix))]` Windows arm that preserves the function's existing return
type and degrades honestly:

- **`doctor_gateway.rs` `pid_is_alive`/status:** `#[cfg(unix)]` the `kill -0`
  spawn; on Windows use `tasklist /FI "PID eq <pid>" /FO CSV` (parse for the PID)
  OR — preferred long-term — the `sysinfo` crate if already a dep, OR
  conservatively mirror `lifecycle.rs::pid_is_alive`'s non-unix arm (assume
  alive / report `{"running": <pid-file present>}`). Pick the option consistent
  with what `win-gateway-stop-portability` lands so the two PID-liveness sites
  don't diverge — coordinate if both are in flight.
- **`doctor_network.rs` `cert_summary`:** `#[cfg(unix)]` around the `sh`+openssl
  shell-out; `#[cfg(not(unix))]` returns `None` (or a "TLS cert details not
  available on this platform" note) keeping the `Option` signature. **NO
  THEATER:** a Windows stub returning "unavailable" is the honest result — do
  NOT fake cert data. (Stretch / out-of-scope here: porting the cert fetch to
  native Rust TLS — rustls/native-tls — to drop the shell-out on all platforms.)
- **`doctor_storage.rs` `disk_usage`:** `#[cfg(unix)]` the `df` path;
  `#[cfg(not(unix))]` either return `None` (honest gap) or query free space via
  a cross-platform crate (`sysinfo`) / Win32 `GetDiskFreeSpaceEx` if low-cost.
  Keep the `Option<(u64,u64,u64)>` signature.

Each touched file stays < 500 LOC. Add a small unit test per site asserting the
non-unix arm compiles + returns the honest degraded value (cross-platform test,
runs on Windows AND the swarm). Unix path stays byte-identical.

## Verify (RUN it — NO THEATER)

- On Windows: `cargo build -p argo-cli` clean; `cargo test -p argo-cli` for the
  three touched handler modules green (incl. the new non-unix-arm tests); clippy
  `-D warnings` clean on the 3 touched files. Run `argo doctor` on Windows and
  confirm it completes (no spawn panic) with gateway-status/cert/disk degrading
  honestly rather than crashing.
- Conductor re-verifies on unix: `cargo test -p argo-cli` (the `#[cfg(unix)]`
  arms are unchanged → byte-identical behaviour) + `argo doctor` still reports
  full cert/disk/gateway diagnostics on macOS.

## Out of scope
- Native-Rust TLS cert porting (noted as the better long-term fix for finding 3).
- Changing what `argo doctor` reports on unix (behaviour must stay identical).

**LOC budget:** ~60–140 across 3 handler files + 3 small tests.
