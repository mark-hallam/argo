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
    unsafe { libc::kill(pid, 0) == 0 }
}

#[cfg(not(unix))]
pub(super) fn pid_is_alive(_pid: i32) -> bool {
    // Conservative on non-Unix: assume alive if we got this far.
    true
}

#[cfg(unix)]
pub(super) fn signal_term(pid: i32) -> std::io::Result<()> {
    if unsafe { libc::kill(pid, libc::SIGTERM) } == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(unix))]
pub(super) fn signal_term(_pid: i32) -> std::io::Result<()> {
    Err(std::io::Error::other(
        "SIGTERM not supported on this platform",
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
