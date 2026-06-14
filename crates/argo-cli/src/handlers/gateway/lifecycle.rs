//! PID file + signal helpers for the gateway daemon. Extracted
//! from the parent `mod.rs` to keep both files under the 500-LOC
//! ceiling.
use std::path::PathBuf;

/// RAII helper: best-effort delete the PID file on drop so a
/// graceful exit (or panic) doesn't leave a stale file behind.
pub(super) struct PidGuard(pub(super) PathBuf);

impl Drop for PidGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[cfg(unix)]
pub(super) fn pid_is_alive(pid: i32) -> bool {
    // signal 0 = "do not actually send a signal, just check that
    // the target process exists and we have permission to signal it".
    // SAFETY: callers pass pid values parsed as i32 from our PID file;
    // signal 0 is the POSIX existence probe and does not alias memory.
    unsafe { libc::kill(pid, 0) == 0 }
}

#[cfg(windows)]
pub(super) fn pid_is_alive(pid: i32) -> bool {
    // No POSIX `kill(pid, 0)` on Windows; the built-in `tasklist` is the
    // liveness probe. `/FI "PID eq <pid>"` prints a CSV row (with the pid as a
    // quoted field) for a live process, else `INFO: No tasks...`. This gives the
    // stop path a correct stale-PID check + stop→confirm instead of the prior
    // always-`true` stub. See `win-gateway-stop-portability`.
    std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains(&format!("\"{pid}\"")))
        .unwrap_or(false)
}

#[cfg(not(any(unix, windows)))]
pub(super) fn pid_is_alive(_pid: i32) -> bool {
    // Conservative on other targets: assume alive if we got this far.
    true
}

#[cfg(unix)]
pub(super) fn signal_term(pid: i32) -> std::io::Result<()> {
    // SAFETY: callers pass pid values parsed as i32 from our PID file;
    // SIGTERM is a valid libc signal constant and does not alias memory.
    if unsafe { libc::kill(pid, libc::SIGTERM) } == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(windows)]
pub(super) fn signal_term(pid: i32) -> std::io::Result<()> {
    use std::process::Command;
    // No POSIX signals on Windows. Honour the unix SIGTERM intent ("stop it")
    // via the built-in `taskkill`. Try a graceful tree-stop first
    // (`taskkill /PID <pid> /T`); a console daemon with no window handler can
    // only be terminated forcefully, so fall back to `/F`. `/T` includes any
    // child processes the daemon spawned. `.output()` swallows taskkill's stdout
    // so the graceful attempt's "could only be terminated forcefully" line does
    // not leak to the operator. See `win-gateway-stop-portability`.
    let pid_s = pid.to_string();
    let graceful = Command::new("taskkill").args(["/PID", &pid_s, "/T"]).output();
    if matches!(&graceful, Ok(o) if o.status.success()) {
        return Ok(());
    }
    let forced = Command::new("taskkill")
        .args(["/PID", &pid_s, "/T", "/F"])
        .output()?;
    if forced.status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "taskkill /PID {pid} /F failed: {}",
            String::from_utf8_lossy(&forced.stderr).trim()
        )))
    }
}

#[cfg(not(any(unix, windows)))]
pub(super) fn signal_term(_pid: i32) -> std::io::Result<()> {
    Err(std::io::Error::other(
        "process termination not supported on this platform",
    ))
}

/// Future that resolves when SIGTERM or SIGINT (Ctrl-C) arrives.
/// On non-Unix targets we only watch Ctrl-C.
pub(super) async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut term = match signal(SignalKind::terminate()) {
            Ok(s) => s,
            Err(_) => return std::future::pending().await,
        };
        let mut int = match signal(SignalKind::interrupt()) {
            Ok(s) => s,
            Err(_) => return std::future::pending().await,
        };
        tokio::select! {
            _ = term.recv() => {}
            _ = int.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

#[cfg(test)]
mod tests {
    use super::pid_is_alive;

    /// The running test process is, by definition, alive. Exercises the real
    /// probe on BOTH platforms (`tasklist` on Windows, `kill(pid,0)` on unix) —
    /// the regression guard for the previously-always-`true` Windows stub
    /// (`win-gateway-stop-portability`).
    #[test]
    fn current_process_reads_as_alive() {
        let pid = std::process::id() as i32;
        assert!(
            pid_is_alive(pid),
            "the running test process (pid {pid}) must read as alive"
        );
    }

    /// A PID this high is never assigned on either platform → must read dead, so
    /// the stop path's stale-PID cleanup can actually fire on Windows (it never
    /// could under the old `true` stub).
    #[test]
    fn improbable_pid_reads_as_dead() {
        assert!(!pid_is_alive(2_147_483_646));
    }
}
