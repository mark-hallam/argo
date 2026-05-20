//! Periodic service-token-binding cleanup.
//!
//! The store owns row deletion; this task only computes a TTL cutoff,
//! calls the store helper, and writes one audit row for non-zero sweeps.
use argo_audit::{AuditKind, AuditSink, SYSTEM_AUDIT_TENANT_ID};
use argo_store::{service_token_binding, SessionStore};
use argo_types::ArgoResult;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;

pub const DEFAULT_TICK_SECS: u64 = 3_600;
pub const MIN_TICK_SECS: u64 = 60;
pub const DEFAULT_TTL_SECS: u64 = 86_400 * 90;
pub const MIN_TTL_SECS: u64 = 86_400;

const TICK_ENV: &str = "ARGO_SERVICE_TOKEN_BINDING_TTL_TICK_SECS";
const TTL_ENV: &str = "ARGO_SERVICE_TOKEN_BINDING_TTL_SECS";

pub fn spawn_service_token_binding_sweeper(store: Arc<SessionStore>) -> Option<JoinHandle<()>> {
    let interval = Duration::from_secs(tick_secs_from_env());
    Some(tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            if let Err(e) = sweep_once(&store, Utc::now()).await {
                tracing::warn!("service-token-binding sweeper error: {e}");
            }
        }
    }))
}

pub(crate) fn tick_secs_from_env() -> u64 {
    env_secs(TICK_ENV, DEFAULT_TICK_SECS, MIN_TICK_SECS)
}

pub(crate) fn ttl_secs_from_env() -> u64 {
    env_secs(TTL_ENV, DEFAULT_TTL_SECS, MIN_TTL_SECS)
}

fn env_secs(key: &str, default: u64, minimum: u64) -> u64 {
    let Ok(raw) = std::env::var(key) else {
        return default;
    };
    match raw.parse::<i64>() {
        Ok(n) if n >= minimum as i64 => n as u64,
        Ok(n) if n > 0 => minimum,
        _ => {
            tracing::warn!("ignoring invalid {key}={raw}; using {default}s");
            default
        }
    }
}

pub(crate) async fn sweep_once(store: &Arc<SessionStore>, now: DateTime<Utc>) -> ArgoResult<u64> {
    let ttl = ChronoDuration::seconds(ttl_secs_from_env() as i64);
    let cutoff = now - ttl;
    let cutoff_iso = cutoff.to_rfc3339();
    let count = service_token_binding::delete_stale_before(store.pool(), &cutoff_iso).await?;
    if count > 0 {
        let payload = json!({
            "kind": "service_token_binding.expired",
            "count": count,
            "swept_at_iso": now.to_rfc3339(),
            "cutoff_iso": cutoff_iso,
        });
        AuditSink::new_with_tenant(Arc::clone(store), SYSTEM_AUDIT_TENANT_ID.to_string())
            .write(None, AuditKind::ServiceTokenBindingExpired, payload, None)
            .await?;
    }
    Ok(count)
}

#[cfg(test)]
#[path = "service_token_binding_sweeper_tests.rs"]
mod tests;
