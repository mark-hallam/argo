//! Graceful shutdown for the long-running `argo work serve`
//! daemon (#770).
//!
//! Intent: when the operator runs `systemctl stop` or
//! `kubectl delete pod`, the worker should finish the current
//! step (job processing or sleep) and exit cleanly with a
//! distinct log line. SIGKILL still wins; this module catches
//! the gentler shutdown paths so the action ledger doesn't end
//! up with a half-written transition row.
//!
//! ## Signals handled
//!
//! - **SIGINT** (Ctrl+C, cross-platform via `tokio::signal::ctrl_c`)
//! - **SIGTERM** (Unix only — what `systemctl` / Docker /
//!   Kubernetes send by default). Cfg-gated so the build still
//!   succeeds on Windows; on Windows only SIGINT applies.
//!
//! ## Mechanics
//!
//! `install_shutdown_watcher()` returns an
//! `Arc<AtomicBool>` set by a background task whenever a
//! shutdown signal lands. The serve loop checks the flag at
//! the top of each iteration AND races it against its sleep,
//! so:
//!
//! - Signal during sleep → exit immediately (don't wait the
//!   full interval).
//! - Signal during job processing → exit after the current job
//!   completes (no half-finished steps).
//! - Signal during signal-watcher's own setup → flag never
//!   flips; falls back to "no graceful shutdown" — the operator
//!   can still SIGKILL.
//!
//! Pure `tokio`-based; no new deps.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Spawn the signal watcher task and return the shared flag.
/// Cheap to call; the task is fire-and-forget.
pub(crate) fn install_shutdown_watcher() -> Arc<AtomicBool> {
    let flag = Arc::new(AtomicBool::new(false));
    let flag_for_task = Arc::clone(&flag);
    tokio::spawn(async move {
        wait_for_any_shutdown_signal().await;
        eprintln!(
            "argo work: shutdown signal received; will exit after the current step"
        );
        flag_for_task.store(true, Ordering::SeqCst);
    });
    flag
}

/// Future that resolves when the shutdown flag is set. Used by
/// the serve loop's `tokio::select!` so a signal during sleep
/// doesn't wait the full interval.
pub(crate) async fn wait_for_shutdown(flag: &Arc<AtomicBool>) {
    // Poll cheaply with short sleeps. The watcher fires this
    // flag in response to a real signal, so the worst-case
    // wake-up latency is the poll interval. 200ms keeps the
    // shutdown observable without burning CPU when idle.
    while !flag.load(Ordering::SeqCst) {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
}

#[cfg(unix)]
async fn wait_for_any_shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    // Both signals are best-effort: if we can't install the
    // SIGTERM handler (would only happen on a bizarre platform),
    // fall back to SIGINT-only.
    let mut sigterm = match signal(SignalKind::terminate()) {
        Ok(s) => Some(s),
        Err(e) => {
            eprintln!(
                "argo work: warning — could not install SIGTERM handler ({e}); \
                 SIGINT (Ctrl+C) still works"
            );
            None
        }
    };
    if let Some(sigterm) = sigterm.as_mut() {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = sigterm.recv() => {}
        }
    } else {
        let _ = tokio::signal::ctrl_c().await;
    }
}

#[cfg(not(unix))]
async fn wait_for_any_shutdown_signal() {
    // Windows: only SIGINT (Ctrl+C) is supported.
    // `kubectl delete pod` etc. send SIGTERM-equivalent
    // signals that don't have a clean Tokio handler on
    // Windows; those processes will still be SIGKILL'd.
    let _ = tokio::signal::ctrl_c().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke: the flag starts `false` and the watcher task can
    /// be installed without panicking. The actual signal-fire
    /// path is tested by integration (operator sends a signal),
    /// not unit tests — `tokio::signal` requires a real signal
    /// handler installed on the process, which is racy under
    /// `cargo test` parallelism.
    #[tokio::test]
    async fn install_returns_unfired_flag_initially() {
        let flag = install_shutdown_watcher();
        // Give the spawned task a moment to install but not
        // long enough to reasonably catch a stray signal.
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        assert!(!flag.load(Ordering::SeqCst));
    }

    /// Lock the polling-future contract: when the flag is
    /// already true, `wait_for_shutdown` resolves on the
    /// first poll cycle (no >200ms hang).
    #[tokio::test]
    async fn wait_for_shutdown_resolves_when_flag_already_set() {
        let flag = Arc::new(AtomicBool::new(true));
        let start = std::time::Instant::now();
        wait_for_shutdown(&flag).await;
        assert!(
            start.elapsed() < std::time::Duration::from_millis(500),
            "wait_for_shutdown took {:?} when flag was already set",
            start.elapsed()
        );
    }

    /// Lock the polling-future contract from the other side:
    /// when the flag is set MID-wait, the future resolves
    /// within roughly the polling interval.
    #[tokio::test]
    async fn wait_for_shutdown_resolves_when_flag_flips_mid_wait() {
        let flag = Arc::new(AtomicBool::new(false));
        let flag_for_setter = Arc::clone(&flag);
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            flag_for_setter.store(true, Ordering::SeqCst);
        });
        let start = std::time::Instant::now();
        wait_for_shutdown(&flag).await;
        assert!(
            start.elapsed() < std::time::Duration::from_millis(800),
            "wait_for_shutdown took {:?} after mid-wait flip",
            start.elapsed()
        );
    }
}
