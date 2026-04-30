//! Mid-run steer channel (Amendment 14, §6.3).
//!
//! `/steer <session-id> "<text>"` — CLI, TUI, gateway, and the
//! plugin protocol all enqueue steers through this channel. The
//! agent loop drains the queue after each tool result and appends
//! a synthesized user message before the next provider call. The
//! key invariant is **prompt-cache safety**: steers come AFTER the
//! cached prefix, so the next provider call still hits cache up
//! through the last cached message.
use argo_types::SessionId;
use parking_lot::Mutex;
use std::collections::HashMap;

/// Thread-safe per-session steer queue.
#[derive(Debug, Default)]
pub struct SteerChannel {
    inner: Mutex<HashMap<SessionId, Vec<String>>>,
}

impl SteerChannel {
    /// Empty channel.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Enqueue `text` for `session_id`. The next loop tick that
    /// drains this session sees it.
    pub fn enqueue(&self, session_id: SessionId, text: String) {
        let mut g = self.inner.lock();
        g.entry(session_id).or_default().push(text);
    }

    /// Drain pending steers for `session_id`, in enqueue order.
    /// The session's slot is removed when empty.
    #[must_use]
    pub fn drain(&self, session_id: SessionId) -> Vec<String> {
        let mut g = self.inner.lock();
        g.remove(&session_id).unwrap_or_default()
    }

    /// Peek the current depth without draining (for telemetry).
    #[must_use]
    pub fn depth(&self, session_id: SessionId) -> usize {
        self.inner.lock().get(&session_id).map_or(0, Vec::len)
    }
}
