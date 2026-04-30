//! Chrome / Chromium debug-port auto-detection (§28.AA point 2,
//! Hermes #6780). When the user's already running Chrome with
//! `--remote-debugging-port=9222`, we want to **attach** to that
//! instance instead of spawning a fresh headless one — so users
//! see automation in their familiar window with their cookies
//! and extensions.
//!
//! Strategy: TCP-probe `127.0.0.1:<port>` with a short timeout.
//! No HTTP request, no pulling in reqwest just for this — the
//! port is open iff Chrome bound it. The full
//! `/json/version` round-trip is the bindings PR's job; this is
//! the cheap gate that tells the spawn path which fork to take.
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::time::Duration;

/// Default debug port the spec calls out + Chrome's own default.
pub const DEFAULT_CHROME_DEBUG_PORT: u16 = 9222;

/// Decision the spawn path takes once it knows whether a Chrome is
/// already listening.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeAttachStrategy {
    /// A debug port is open — attach instead of spawning.
    AttachExisting {
        /// Port that responded.
        port: u16,
    },
    /// Nothing listening — the bindings layer will spawn its own
    /// headless Chrome.
    SpawnFresh,
}

/// Probe `127.0.0.1:<port>` with a 200 ms timeout. Returns
/// [`ChromeAttachStrategy::AttachExisting`] iff the TCP connect
/// succeeded.
#[must_use]
pub fn probe_local_chrome(port: u16) -> ChromeAttachStrategy {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
    match TcpStream::connect_timeout(&addr, Duration::from_millis(200)) {
        Ok(_) => ChromeAttachStrategy::AttachExisting { port },
        Err(_) => ChromeAttachStrategy::SpawnFresh,
    }
}

/// Probe the spec default port (`9222`).
#[must_use]
pub fn probe_default_chrome() -> ChromeAttachStrategy {
    probe_local_chrome(DEFAULT_CHROME_DEBUG_PORT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn missing_port_yields_spawn_fresh() {
        // Bind a temporary listener to find a guaranteed-free port,
        // then drop it; the gap between drop and probe is small but
        // real, so we accept either outcome and only assert that
        // the function returns *something*.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        // After drop the port is almost certainly free; result
        // should be SpawnFresh.
        assert_eq!(probe_local_chrome(port), ChromeAttachStrategy::SpawnFresh);
    }

    #[test]
    fn open_port_yields_attach_existing() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let strategy = probe_local_chrome(port);
        // Holding the listener keeps the port open for the duration
        // of the probe.
        drop(listener);
        assert_eq!(strategy, ChromeAttachStrategy::AttachExisting { port });
    }

    #[test]
    fn default_port_constant_matches_chrome_default() {
        assert_eq!(DEFAULT_CHROME_DEBUG_PORT, 9222);
    }
}
