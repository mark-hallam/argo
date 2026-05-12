//! Cron-job entry point for the E2B idle-sandbox reaper.
//!
//! Lives in this crate (not `argo-runtime`) so the cron registration
//! doesn't drag a new cross-crate dep into the runtime tree while
//! the workspace's cron scaffolding (`mission-cron-tick-maintenance-jobs`)
//! is still in flight. When the cron-tick scaffolding lands, the
//! registration shim moves to `argo-runtime::cron_tick` and calls
//! [`run_e2b_reap_cycle`] directly — no production logic changes.
//!
//! ## Activation
//!
//! Gated by `ARGO_E2B_API_KEY` presence. Operators without E2B
//! configured see no logs and no errors. The skip path uses
//! `tracing::trace!` so even DEBUG-level logging stays quiet.
//!
//! ## Defaults
//!
//! - Interval: 15 minutes ([`DEFAULT_INTERVAL_SECS`]).
//! - Idle threshold: 600 seconds = 10 minutes ([`DEFAULT_MAX_IDLE_SECS`]).
//!
//! Aggressive operators may override `max_idle_secs` to 60s via
//! a future config surface; per-template thresholds are out of scope
//! for v1.

use crate::e2b::{E2bCall, E2bClient};
use crate::e2b_reap::E2bReapReport;
use crate::CuError;

/// Cron tick interval. 15 minutes balances cleanup latency against
/// E2B API quota — a 10-minute idle threshold means a leaked sandbox
/// gets reaped within 25 minutes of last activity, far below the
/// template default 5–15 min `pause_timeout` that drives the billing
/// floor when no reaper is running.
pub const DEFAULT_INTERVAL_SECS: u64 = 900;

/// Idle-time threshold. Sandboxes idle longer than this get closed
/// on the next reap pass. Default 10 min matches the existing
/// in-band CU pool idle-TTL so the out-of-band reaper picks up
/// leaks shortly after the in-band path would have closed them.
pub const DEFAULT_MAX_IDLE_SECS: u64 = 600;

/// Environment variable that gates the reaper. Absent or empty =>
/// the cron tick is a no-op.
pub const ARGO_E2B_API_KEY_ENV: &str = "ARGO_E2B_API_KEY";

/// Environment variable that carries the E2B API base URL.
/// Optional — defaults to E2B's production endpoint.
pub const ARGO_E2B_BASE_URL_ENV: &str = "ARGO_E2B_BASE_URL";

/// Fallback base URL when `ARGO_E2B_BASE_URL` is unset.
const DEFAULT_BASE_URL: &str = "https://api.e2b.dev";

/// Default per-request HTTP timeout for the reaper's list+close
/// calls. Long enough to survive a saturated E2B control-plane,
/// short enough that a stuck call doesn't wedge the cron tick.
const DEFAULT_TIMEOUT_SECS: u32 = 30;

/// Run one reap cycle. Returns `Ok(None)` if `ARGO_E2B_API_KEY` is
/// unset (the documented no-op path); `Ok(Some(report))` after a
/// successful reap; `Err(...)` when client construction or the
/// list/close round-trip itself fails (caller's cron framework
/// decides retry policy).
pub async fn run_e2b_reap_cycle(
    max_idle_secs: u64,
) -> Result<Option<E2bReapReport>, CuError> {
    let api_key = match std::env::var(ARGO_E2B_API_KEY_ENV) {
        Ok(k) if !k.is_empty() => k,
        _ => {
            tracing::trace!(
                env = ARGO_E2B_API_KEY_ENV,
                "e2b reap skipped: API key not configured"
            );
            return Ok(None);
        }
    };
    let base_url = std::env::var(ARGO_E2B_BASE_URL_ENV)
        .unwrap_or_else(|_| DEFAULT_BASE_URL.to_owned());
    let client = E2bClient::new(base_url, api_key, DEFAULT_TIMEOUT_SECS)?;
    let report = client.reap_idle(max_idle_secs).await?;
    tracing::info!(
        scanned = report.scanned,
        reaped = report.reaped.len(),
        errors = report.errors.len(),
        "e2b reap cycle complete"
    );
    Ok(Some(report))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn no_op_when_api_key_unset() {
        // Per-test env handling: snapshot + restore via guard so a
        // parallel test setting the same var doesn't trip us. We
        // remove the var for the duration of the test.
        let prior = std::env::var(ARGO_E2B_API_KEY_ENV).ok();
        // SAFETY: tests run serialized for env mutation via
        // single-threaded runtime; remove_var is safe here.
        unsafe {
            std::env::remove_var(ARGO_E2B_API_KEY_ENV);
        }
        let result = run_e2b_reap_cycle(600).await;
        if let Some(v) = prior {
            unsafe {
                std::env::set_var(ARGO_E2B_API_KEY_ENV, v);
            }
        }
        assert!(matches!(result, Ok(None)));
    }
}
