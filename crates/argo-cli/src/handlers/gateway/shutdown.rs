//! Graceful-shutdown plumbing for `argo gateway start`.
//!
//! SIGTERM (and SIGINT for dev) flips a shared flag, the accept-loop's
//! `tokio::select!` observes a oneshot channel from the signal task
//! and stops taking new connections, and after the loop exits we
//! drain in-flight requests up to a deadline before closing the
//! session store. A force-exit watchdog guarantees the process
//! unblocks even if a request hangs past the drain window.
//!
//! The previous behavior was a hard exit on signal — in-flight
//! requests got RST and `SessionStore`'s SQLite WAL never got a
//! chance to checkpoint cleanly. This module is the surface area
//! the gateway uses to make that exit ordered.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::oneshot;

/// Default deadline for in-flight drain after SIGTERM, in seconds.
pub(super) const DEFAULT_SHUTDOWN_DEADLINE_SECS: u64 = 30;
/// Force-exit grace period beyond the drain deadline.
pub(super) const FORCE_EXIT_GRACE_SECS: u64 = 5;
/// Status code emitted on hung-drain force-exit (mirrors `timeout(1)`).
pub(super) const FORCE_EXIT_STATUS: i32 = 124;
/// Drain progress log cadence.
const DRAIN_LOG_INTERVAL: Duration = Duration::from_secs(5);
/// Drain poll interval — small so we don't pad the deadline.
const DRAIN_POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Shared shutdown state observed by the accept loop, the per-request
/// drop-guard, and the drain loop. Cheap to clone across tasks.
#[derive(Clone, Default)]
pub(crate) struct ShutdownController {
    inner: Arc<Inner>,
}

#[derive(Default)]
struct Inner {
    flag: AtomicBool,
    in_flight: AtomicUsize,
}

impl ShutdownController {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// True once SIGTERM/SIGINT has been observed.
    pub(crate) fn is_shutting_down(&self) -> bool {
        self.inner.flag.load(Ordering::SeqCst)
    }

    /// Mark the controller as shutting down. Idempotent.
    pub(crate) fn trigger(&self) {
        self.inner.flag.store(true, Ordering::SeqCst);
    }

    /// Number of requests currently inside the per-request guard.
    pub(crate) fn in_flight(&self) -> usize {
        self.inner.in_flight.load(Ordering::SeqCst)
    }

    /// Increment the in-flight counter and return a drop-guard that
    /// decrements on drop (exception-safe — no path leaks the count).
    pub(crate) fn request_guard(&self) -> RequestGuard {
        self.inner.in_flight.fetch_add(1, Ordering::SeqCst);
        RequestGuard {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// Drop-guard that decrements the controller's in-flight counter on
/// `Drop`. Created at the top of `dispatch_route` so every early
/// return / panic-unwind / `?` short-circuit still releases the slot.
pub(crate) struct RequestGuard {
    inner: Arc<Inner>,
}

impl Drop for RequestGuard {
    fn drop(&mut self) {
        self.inner.in_flight.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Read `ARGO_SHUTDOWN_DEADLINE_SECS`, falling back to the 30s default.
pub(super) fn deadline_secs_from_env() -> u64 {
    std::env::var("ARGO_SHUTDOWN_DEADLINE_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(DEFAULT_SHUTDOWN_DEADLINE_SECS)
}

/// Spawn a task that watches SIGTERM (and SIGINT in dev) and flips the
/// controller flag. Returns a oneshot receiver the accept loop awaits
/// in its `tokio::select!` so connections stop being accepted as soon
/// as the trigger fires. The receiver delivers the force-exit
/// watchdog `JoinHandle` so the caller can abort it after a clean
/// drain+close. The watchdog timer starts ticking from the moment the
/// signal lands — that way even a slow client that hangs the accept
/// arm in `read()` can't keep the process alive past `deadline + 5s`.
pub(super) fn install_signal_handler(
    ctl: ShutdownController,
) -> oneshot::Receiver<tokio::task::JoinHandle<()>> {
    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        super::lifecycle::shutdown_signal().await;
        eprintln!("argo gateway: shutdown signal received; draining in-flight requests");
        ctl.trigger();
        let deadline = Duration::from_secs(deadline_secs_from_env());
        let watchdog = spawn_force_exit_watchdog(deadline);
        // Best-effort wake — receiver may already be dropped if we
        // raced startup teardown.
        let _ = tx.send(watchdog);
    });
    rx
}

/// Spawn a watchdog task that force-exits with `FORCE_EXIT_STATUS`
/// after `deadline + FORCE_EXIT_GRACE_SECS` elapses. Aborted by the
/// caller when drain completes cleanly.
pub(super) fn spawn_force_exit_watchdog(deadline: Duration) -> tokio::task::JoinHandle<()> {
    let total = deadline + Duration::from_secs(FORCE_EXIT_GRACE_SECS);
    tokio::spawn(async move {
        tokio::time::sleep(total).await;
        eprintln!(
            "argo gateway: drain hung past {}s + {}s grace; force-exit status {}",
            deadline.as_secs(),
            FORCE_EXIT_GRACE_SECS,
            FORCE_EXIT_STATUS
        );
        std::process::exit(FORCE_EXIT_STATUS);
    })
}

/// Block until the in-flight counter reaches zero or `deadline`
/// elapses. Logs the live count every 5s so operators can correlate
/// drains with k8s pod-termination grace periods.
pub(super) async fn drain_in_flight(ctl: &ShutdownController, deadline: Duration) {
    let start = Instant::now();
    let mut next_log = start + DRAIN_LOG_INTERVAL;
    loop {
        let n = ctl.in_flight();
        if n == 0 {
            eprintln!("argo gateway: drain complete (0 in-flight)");
            return;
        }
        if start.elapsed() >= deadline {
            eprintln!(
                "argo gateway: drain deadline reached ({}s) with {n} in-flight; closing store anyway",
                deadline.as_secs()
            );
            return;
        }
        let now = Instant::now();
        if now >= next_log {
            eprintln!("argo gateway: draining ({n} in-flight)");
            next_log = now + DRAIN_LOG_INTERVAL;
        }
        tokio::time::sleep(DRAIN_POLL_INTERVAL).await;
    }
}

/// 503 response returned to any request that reaches `dispatch_route`
/// after the shutdown flag has flipped. The body is documented as
/// `{"error":"shutting_down"}` — external clients use it to back off
/// during rolling deploys.
pub(crate) fn shutting_down_response() -> (&'static str, String) {
    (
        "503 Service Unavailable",
        "{\"error\":\"shutting_down\"}".to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_flips_flag() {
        let ctl = ShutdownController::new();
        assert!(!ctl.is_shutting_down());
        ctl.trigger();
        assert!(ctl.is_shutting_down());
        // Idempotent.
        ctl.trigger();
        assert!(ctl.is_shutting_down());
    }

    #[tokio::test]
    async fn request_guard_increments_then_decrements() {
        let ctl = ShutdownController::new();
        assert_eq!(ctl.in_flight(), 0);
        {
            let _g1 = ctl.request_guard();
            let _g2 = ctl.request_guard();
            assert_eq!(ctl.in_flight(), 2);
        }
        assert_eq!(ctl.in_flight(), 0);
    }

    #[tokio::test]
    async fn drain_returns_immediately_when_nothing_in_flight() {
        let ctl = ShutdownController::new();
        let started = Instant::now();
        drain_in_flight(&ctl, Duration::from_secs(30)).await;
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn drain_respects_deadline_when_request_hangs() {
        let ctl = ShutdownController::new();
        let _guard = ctl.request_guard();
        let started = Instant::now();
        drain_in_flight(&ctl, Duration::from_millis(150)).await;
        let elapsed = started.elapsed();
        assert!(
            elapsed >= Duration::from_millis(150),
            "drain returned too early: {elapsed:?}"
        );
        assert!(
            elapsed < Duration::from_secs(2),
            "drain did not honour deadline: {elapsed:?}"
        );
        assert_eq!(ctl.in_flight(), 1);
    }

    #[tokio::test]
    async fn drain_completes_when_guard_drops_mid_flight() {
        let ctl = ShutdownController::new();
        let guard = ctl.request_guard();
        let ctl_clone = ctl.clone();
        let drainer = tokio::spawn(async move {
            drain_in_flight(&ctl_clone, Duration::from_secs(5)).await;
        });
        tokio::time::sleep(Duration::from_millis(80)).await;
        drop(guard);
        // Drainer should now resolve quickly.
        tokio::time::timeout(Duration::from_secs(2), drainer)
            .await
            .expect("drain timed out")
            .expect("drainer task");
        assert_eq!(ctl.in_flight(), 0);
    }

    #[test]
    fn deadline_env_parses_positive_only() {
        let prev = std::env::var("ARGO_SHUTDOWN_DEADLINE_SECS").ok();
        std::env::set_var("ARGO_SHUTDOWN_DEADLINE_SECS", "12");
        assert_eq!(deadline_secs_from_env(), 12);
        std::env::set_var("ARGO_SHUTDOWN_DEADLINE_SECS", "0");
        assert_eq!(deadline_secs_from_env(), DEFAULT_SHUTDOWN_DEADLINE_SECS);
        std::env::set_var("ARGO_SHUTDOWN_DEADLINE_SECS", "not-a-number");
        assert_eq!(deadline_secs_from_env(), DEFAULT_SHUTDOWN_DEADLINE_SECS);
        std::env::remove_var("ARGO_SHUTDOWN_DEADLINE_SECS");
        assert_eq!(deadline_secs_from_env(), DEFAULT_SHUTDOWN_DEADLINE_SECS);
        if let Some(v) = prev {
            std::env::set_var("ARGO_SHUTDOWN_DEADLINE_SECS", v);
        }
    }
}
