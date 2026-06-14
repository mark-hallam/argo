# win-gateway-stop-portability

**Owner**: windows-fleet — **P2 CROSS-PLATFORM — `argo gateway stop` cannot terminate the gateway on native Windows (returns an error instead of stopping it).**
**Depends on**: none.
**Authored against**: origin/main `069d57962` on `2026-06-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh win-gateway-stop-portability`
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build+verify on Windows; the FIX is cross-platform (`#[cfg(unix)]` keeps the unix SIGTERM path byte-identical) so the conductor ALSO re-verifies on unix before merge. Push branch, IN_REVIEW; conductor reviews+merges. **Do NOT self-merge.**
Source: the real Windows e2e journey run (#2940 / `win-e2e-full-platform-journey`, MERGED) surfaced this — "NEW BUG: `argo gateway stop` broken on Windows (SIGTERM unsupported)". Confirmed against current main below.

## Problem (VERIFIED present on origin/main `069d57962`)

`crates/argo-cli/src/handlers/gateway/lifecycle.rs` already cfg-splits the two
process primitives, but the **non-unix arm of `signal_term` is a stub that
refuses to work** rather than a real Windows implementation:

```rust
#[cfg(unix)]
pub(super) fn signal_term(pid: i32) -> std::io::Result<()> {
    if unsafe { libc::kill(pid, libc::SIGTERM) } == 0 { Ok(()) }
    else { Err(std::io::Error::last_os_error()) }
}

#[cfg(not(unix))]
pub(super) fn signal_term(_pid: i32) -> std::io::Result<()> {
    Err(std::io::Error::other("SIGTERM not supported on this platform"))   // lifecycle.rs:~45
}
```

So on native Windows `argo gateway stop` calls `signal_term`, gets the
`Err("SIGTERM not supported on this platform")`, and **fails to stop the running
gateway** — the operator must `taskkill` it by hand. Note `boot.rs:~253` also
prints `"argo gateway: sent SIGTERM to pid {pid}"` unconditionally, which on
Windows is misleading (nothing was sent). The companion `pid_is_alive`
(`lifecycle.rs:~22`, `libc::kill(pid, 0)`) is already handled on non-unix
(returns `true` conservatively) — only the **terminate** path is the gap.

This is a real cross-platform defect for the Windows fleet: the gateway is known
to boot+serve on Windows (#2920 GATEWAY-E2E.md), so a working `stop` is part of
the basic lifecycle.

## Fix

Implement the `#[cfg(not(unix))]` (or a dedicated `#[cfg(windows)]`) arm of
`signal_term` to actually terminate the process on Windows. Options, pick the
lowest-risk that cleanly stops the gateway:
- **`taskkill /PID <pid> /T` (graceful) → `/F` (force) fallback** via
  `std::process::Command` — simplest, no new crate; map a non-zero exit to a
  clear `io::Error`. (`/T` also reaps child processes the gateway spawned.)
- OR the Win32 API (`OpenProcess` + `TerminateProcess`, or
  `GenerateConsoleCtrlEvent` for a graceful CTRL_BREAK) via the `windows`/
  `windows-sys` crate **only if already a dependency** — do not add a heavy new
  dep for this if `taskkill` suffices.

Also fix the **misleading `boot.rs` log**: make the "sent SIGTERM" message
platform-accurate (e.g. `#[cfg(windows)]` → "requested stop of pid {pid}
(taskkill)"), or move/guard it so it only prints after `signal_term` returns
`Ok`. Keep the unix SIGTERM path byte-identical.

Consider whether `pid_is_alive`'s non-unix "assume alive" should become a real
Windows liveness check (`tasklist`) for a correct stop→confirm loop — if the
stop handler polls `pid_is_alive` to confirm termination, the always-`true` stub
would hang/false-report. If so, coordinate with the sibling brief
`win-doctor-cross-platform-shellouts` (which also touches PID-liveness in
`doctor_gateway.rs`) so the two PID-liveness implementations don't diverge.

## Verify (RUN it — NO THEATER)

- On Windows: `cargo build -p argo-cli` clean; `cargo test -p argo-cli` for the
  gateway lifecycle module green; clippy `-D warnings` clean on the touched
  file(s). **Live check (the whole point of this fleet):** `argo gateway start`,
  confirm `/health` 200, then `argo gateway stop` → assert the process is
  actually gone (`tasklist /FI "PID eq <pid>"` shows no match) and the command
  returns success, not the "SIGTERM not supported" error. Capture the real
  before/after into the results doc.
- Conductor re-verifies on unix: `cargo test -p argo-cli` (the `#[cfg(unix)]`
  SIGTERM arm is unchanged → byte-identical) + a local `start`/`stop` on macOS
  still stops the gateway via SIGTERM.
- Results → `docs/skycot/windows/E2E-GATEWAY-STOP.md` (the Windows live
  start→stop→confirm-gone evidence). NO THEATER: if a chosen mechanism can't
  fully reap the process, record that honestly + recommend a follow-up.

## Out of scope
- Changing the unix SIGTERM/SIGINT shutdown semantics.
- A full service-manager integration (Windows Service / SCM) — that's a separate,
  larger piece; this brief only makes the existing `stop` command work.

**LOC budget:** ~40–100 (the Windows `signal_term` arm + the boot.rs log guard + a test).
