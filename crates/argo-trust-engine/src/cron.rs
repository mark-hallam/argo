//! Periodic trust-grade recomputation loop. Mirrors
//! [`argo_forge::cron`] in shape: env-var-driven config + a
//! `tokio::spawn` that drives `tick()` on an interval.

use argo_store::SessionStore;
use argo_types::ArgoResult;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;

const DEFAULT_INTERVAL_SECS: u64 = 1_800;

/// Controls the trust-engine cron cadence and dry-run mode.
#[derive(Debug, Clone, PartialEq)]
pub struct TrustEngineConfig {
    /// Delay between ticks. Defaults to 30 minutes.
    pub interval_secs: u64,
    /// When true, log the ratchet decision the engine WOULD take
    /// but skip the write + audit-emit side effects.
    pub dry_run: bool,
}

impl Default for TrustEngineConfig {
    fn default() -> Self {
        Self {
            interval_secs: DEFAULT_INTERVAL_SECS,
            dry_run: false,
        }
    }
}

impl TrustEngineConfig {
    /// Build config from `ARGO_TRUST_ENGINE_*` environment variables.
    #[must_use]
    pub fn from_env() -> Self {
        Self::from_vars(|name| std::env::var(name).ok())
    }

    /// Build config from a supplied lookup function. Exists so unit
    /// tests can drive the parser without mutating process env.
    #[must_use]
    pub fn from_vars(mut get: impl FnMut(&str) -> Option<String>) -> Self {
        let default = Self::default();
        Self {
            interval_secs: parse_u64(
                get("ARGO_TRUST_ENGINE_INTERVAL_SECS"),
                default.interval_secs,
            ),
            dry_run: parse_bool(get("ARGO_TRUST_ENGINE_DRY_RUN"), default.dry_run),
        }
    }
}

/// Spawn the periodic trust-engine cron loop. The first tick fires
/// immediately, then repeats every `cfg.interval_secs`.
///
/// Returns a [`JoinHandle`] the caller can drop to abort the loop
/// (e.g. on gateway shutdown via the standard `AbortOnDrop` wrapper).
pub fn spawn_trust_engine_cron(
    store: Arc<SessionStore>,
    cfg: TrustEngineConfig,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(cfg.interval_secs.max(1)));
        loop {
            ticker.tick().await;
            if let Err(e) = tick(Arc::clone(&store), &cfg).await {
                tracing::warn!(target: "trust_engine::cron", error = %e, "trust-engine tick failed");
            }
        }
    })
}

/// Run one engine tick. Iterates every `(tenant_id, worker_id)`
/// pair in the `worker` table and recomputes each. Returns the
/// number of workers whose grade actually changed. Exposed for
/// integration tests so they can drive ticks deterministically
/// without spawning the cron loop.
///
/// # Errors
/// Returns [`argo_types::ArgoError::Provider`] when the underlying
/// worker scan or per-row recompute fails. Per-worker errors do not
/// abort the whole tick — they're logged and counted.
pub async fn tick(store: Arc<SessionStore>, cfg: &TrustEngineConfig) -> ArgoResult<usize> {
    if cfg.dry_run {
        crate::recompute::tick_all_workers_dry_run(store).await
    } else {
        crate::recompute::tick_all_workers(store).await
    }
}

fn parse_u64(v: Option<String>, default: u64) -> u64 {
    v.and_then(|s| s.parse().ok())
        .filter(|n| *n > 0)
        .unwrap_or(default)
}

fn parse_bool(v: Option<String>, default: bool) -> bool {
    v.map(|s| matches!(s.as_str(), "1" | "true" | "TRUE" | "yes" | "on"))
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_interval_is_thirty_minutes() {
        let cfg = TrustEngineConfig::default();
        assert_eq!(cfg.interval_secs, 1_800);
        assert!(!cfg.dry_run);
    }

    #[test]
    fn from_vars_reads_interval_override() {
        let lookup = |name: &str| match name {
            "ARGO_TRUST_ENGINE_INTERVAL_SECS" => Some("60".to_string()),
            _ => None,
        };
        let cfg = TrustEngineConfig::from_vars(lookup);
        assert_eq!(cfg.interval_secs, 60);
        assert!(!cfg.dry_run);
    }

    #[test]
    fn from_vars_rejects_zero_interval() {
        let lookup = |name: &str| match name {
            "ARGO_TRUST_ENGINE_INTERVAL_SECS" => Some("0".to_string()),
            _ => None,
        };
        let cfg = TrustEngineConfig::from_vars(lookup);
        assert_eq!(cfg.interval_secs, 1_800);
    }

    #[test]
    fn from_vars_parses_truthy_dry_run_strings() {
        for raw in ["1", "true", "TRUE", "yes", "on"] {
            let lookup = move |name: &str| match name {
                "ARGO_TRUST_ENGINE_DRY_RUN" => Some(raw.to_string()),
                _ => None,
            };
            let cfg = TrustEngineConfig::from_vars(lookup);
            assert!(cfg.dry_run, "expected dry_run for {raw}");
        }
    }

    #[test]
    fn from_vars_rejects_garbage_dry_run() {
        let lookup = |name: &str| match name {
            "ARGO_TRUST_ENGINE_DRY_RUN" => Some("maybe".to_string()),
            _ => None,
        };
        assert!(!TrustEngineConfig::from_vars(lookup).dry_run);
    }
}
