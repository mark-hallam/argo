//! Channel-based gateway listener-readiness signal.
//!
//! Replaces the historical 2-second poll loop in `await_listening`
//! that flaked under load (#1057, #1079, #1080, #960). The drain
//! thread reads the gateway's stderr line-by-line and signals via
//! an `mpsc::sync_channel` when the "listening on" log line
//! arrives. The caller blocks on `recv_timeout` with a configurable
//! deadline — default 10s, override with
//! `ARGO_TEST_LISTENER_DEADLINE_MS`.
//!
//! Three failure modes are now distinguishable:
//!   * `Bound` — listener announced; the test can proceed.
//!   * `DeadlineExceeded` — gateway is alive but has not announced
//!     within the deadline. Likely loaded machine; bump the env var.
//!   * `GatewayCrashed` — stderr closed before the announce line.
//!     Surfaces real boot failures (e.g., port-already-bound) that
//!     the polling helper masked as misleading timeouts.
//!
//! After signaling, the drain thread keeps running and discards
//! further stderr lines. This prevents the gateway from blocking
//! on a full pipe buffer mid-test, a latent bug in the previous
//! design where callers held the `BufReader<ChildStderr>` but
//! never read from it.

use std::io::{BufRead, BufReader};
use std::process::ChildStderr;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Default listener-announce deadline if the env var is unset.
pub const DEFAULT_DEADLINE_MS: u64 = 10_000;

/// Env var override for the deadline (milliseconds).
pub const DEADLINE_ENV: &str = "ARGO_TEST_LISTENER_DEADLINE_MS";

/// Drain handle returned on a successful announce. Drop it when
/// the test is done — the underlying thread exits naturally when
/// the child closes its stderr pipe (i.e., on child exit).
pub struct StderrDrain {
    _handle: JoinHandle<()>,
}

#[derive(Debug)]
pub enum ListenerError {
    DeadlineExceeded { deadline_ms: u64 },
    GatewayCrashed,
}

impl std::fmt::Display for ListenerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ListenerError::DeadlineExceeded { deadline_ms } => write!(
                f,
                "listener did not announce in {deadline_ms}ms (set {DEADLINE_ENV} to extend)"
            ),
            ListenerError::GatewayCrashed => write!(
                f,
                "gateway crashed before listener bound (stderr closed without announce)"
            ),
        }
    }
}

impl std::error::Error for ListenerError {}

enum Signal {
    Bound,
    Crashed,
}

/// Resolve the deadline override from the environment, falling
/// back to `DEFAULT_DEADLINE_MS` on missing/invalid values.
pub fn deadline_ms_from_env() -> u64 {
    std::env::var(DEADLINE_ENV)
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(DEFAULT_DEADLINE_MS)
}

/// Spawn the drain thread, watch for the announce line, and block
/// the caller until either the signal arrives, the gateway
/// crashes, or the deadline expires.
pub fn await_listener_signal(stderr: ChildStderr) -> Result<StderrDrain, ListenerError> {
    let deadline_ms = deadline_ms_from_env();
    let (tx, rx) = mpsc::sync_channel::<Signal>(1);

    let handle = thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        let mut line = String::new();
        let mut announced = false;
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    if !announced {
                        let _ = tx.send(Signal::Crashed);
                    }
                    break;
                }
                Ok(_) => {
                    if !announced && line.contains("listening on") {
                        announced = true;
                        let _ = tx.send(Signal::Bound);
                    }
                }
                Err(_) => {
                    if !announced {
                        let _ = tx.send(Signal::Crashed);
                    }
                    break;
                }
            }
        }
    });

    match rx.recv_timeout(Duration::from_millis(deadline_ms)) {
        Ok(Signal::Bound) => Ok(StderrDrain { _handle: handle }),
        Ok(Signal::Crashed) => Err(ListenerError::GatewayCrashed),
        Err(RecvTimeoutError::Timeout) => Err(ListenerError::DeadlineExceeded { deadline_ms }),
        Err(RecvTimeoutError::Disconnected) => Err(ListenerError::GatewayCrashed),
    }
}
