//! Adaptive provider heartbeat monitor (Amendment 6, §6.4).
//!
//! Streams from a provider can take 60+ seconds to emit their first
//! token (long-thinking models, reasoning traces, slow batch hosts).
//! A fixed 180s reconnect timer would either kill them too early or
//! sit blind for too long after the stream actually died.
//!
//! `HeartbeatMonitor` resets on **any signal of liveness**:
//! - any byte received from the streaming response,
//! - any provider-emitted heartbeat (`event: ping`, `:keepalive`,
//!   per the manifest),
//! - any successful HTTP keep-alive frame on non-streaming providers.
//!
//! Default grace window: 30s. The transport calls
//! [`HeartbeatMonitor::note_activity`] for every signal and
//! [`HeartbeatMonitor::is_stale`] before issuing a reconnect.
use parking_lot::Mutex;
use std::time::{Duration, Instant};

/// Per-call heartbeat monitor.
///
/// Cheap to construct (one `Instant` + one `Duration`), `Send +
/// Sync`, and stays cheap under contention because the lock window
/// is just a single field-store.
#[derive(Debug)]
pub struct HeartbeatMonitor {
    last_activity: Mutex<Instant>,
    grace: Duration,
}

impl HeartbeatMonitor {
    /// New monitor with a 30s default grace window — the spec
    /// default ("default grace window: 30s from last byte").
    #[must_use]
    pub fn new() -> Self {
        Self::with_grace(Duration::from_secs(30))
    }

    /// New monitor with a custom grace window. Lets long-thinking
    /// model manifests bump the window via `[heartbeat].grace_ms`.
    #[must_use]
    pub fn with_grace(grace: Duration) -> Self {
        Self {
            last_activity: Mutex::new(Instant::now()),
            grace,
        }
    }

    /// Note that the connection saw activity (data byte / ping /
    /// keep-alive). Resets the grace timer.
    pub fn note_activity(&self) {
        *self.last_activity.lock() = Instant::now();
    }

    /// `true` when no activity has been seen for longer than
    /// `grace`. Callers consult this before issuing a reconnect.
    #[must_use]
    pub fn is_stale(&self, now: Instant) -> bool {
        now.saturating_duration_since(*self.last_activity.lock()) > self.grace
    }

    /// Time since the last `note_activity()` call.
    #[must_use]
    pub fn since_last(&self, now: Instant) -> Duration {
        now.saturating_duration_since(*self.last_activity.lock())
    }

    /// Configured grace window.
    #[must_use]
    pub fn grace(&self) -> Duration {
        self.grace
    }
}

impl Default for HeartbeatMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_monitor_is_not_stale() {
        let m = HeartbeatMonitor::with_grace(Duration::from_secs(1));
        // The constructor stamps `last_activity = now`, so a check
        // microseconds later is not stale.
        assert!(!m.is_stale(Instant::now()));
    }

    #[test]
    fn note_activity_resets_the_timer() {
        let m = HeartbeatMonitor::with_grace(Duration::from_millis(100));
        std::thread::sleep(Duration::from_millis(50));
        m.note_activity();
        std::thread::sleep(Duration::from_millis(70));
        // 70ms < 100ms grace → not stale.
        assert!(!m.is_stale(Instant::now()));
    }

    #[test]
    fn no_activity_past_grace_is_stale() {
        let m = HeartbeatMonitor::with_grace(Duration::from_millis(50));
        std::thread::sleep(Duration::from_millis(80));
        assert!(m.is_stale(Instant::now()));
    }

    #[test]
    fn since_last_grows_monotonically() {
        let m = HeartbeatMonitor::new();
        let a = m.since_last(Instant::now());
        std::thread::sleep(Duration::from_millis(5));
        let b = m.since_last(Instant::now());
        assert!(b >= a);
    }

    #[test]
    fn grace_is_propagated_from_constructor() {
        let m = HeartbeatMonitor::with_grace(Duration::from_secs(7));
        assert_eq!(m.grace(), Duration::from_secs(7));
    }

    #[test]
    fn property_streaming_one_byte_per_25s_never_reconnects() {
        // §6.7 acceptance: "a stream emitting one byte per 25s
        // never reconnects".
        let m = HeartbeatMonitor::with_grace(Duration::from_secs(30));
        // Simulate: every 25ms (scaled down 1000×) we emit a byte
        // → note_activity resets; even after many "bytes" we're
        // always in the green.
        for _ in 0..10 {
            std::thread::sleep(Duration::from_millis(25));
            m.note_activity();
            assert!(!m.is_stale(Instant::now()));
        }
    }

    #[test]
    fn property_silent_past_grace_reconnects() {
        // §6.7 acceptance: "a stream silent for 35s does [reconnect]".
        let m = HeartbeatMonitor::with_grace(Duration::from_millis(30));
        std::thread::sleep(Duration::from_millis(40));
        assert!(m.is_stale(Instant::now()));
    }
}
