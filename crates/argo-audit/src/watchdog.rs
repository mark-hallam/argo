//! Background audit-chain watchdog (mvp-audit-tamper-watchdog).
//!
//! Periodically runs [`crate::verify_all`] against the open
//! [`SessionStore`] and invokes a caller-supplied `on_break`
//! callback for every [`ChainBreak`] it finds. Counts consecutive
//! ticks that fail to verify and emits a structured
//! `tracing::error!` once the streak reaches
//! `max_consecutive_failures`. Counter resets on the next clean
//! verify so transient breaks don't pin the alarm on.
//!
//! ```
//! # async fn doc() -> argo_types::ArgoResult<()> {
//! use std::sync::Arc;
//! use argo_audit::{spawn_watchdog, WatchdogConfig, ChainBreak};
//! use argo_store::SessionStore;
//! let store = Arc::new(SessionStore::open(":memory:").await?);
//! store.migrate().await?;
//! let cb: Arc<dyn Fn(ChainBreak) + Send + Sync> = Arc::new(|_| {});
//! spawn_watchdog(store, WatchdogConfig::default(), cb).abort();
//! # Ok(()) }
//! ```
use crate::verify::{verify_all, ChainBreak};
use argo_store::SessionStore;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;

/// Tuning knobs for the watchdog loop.
#[derive(Debug, Clone)]
pub struct WatchdogConfig {
    /// How often the watchdog calls [`verify_all`]. Default 5 min.
    pub interval: Duration,
    /// Consecutive failing ticks before a `tracing::error!` alarm
    /// fires in addition to the per-break callback. Default 3.
    pub max_consecutive_failures: u32,
}

impl Default for WatchdogConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(300),
            max_consecutive_failures: 3,
        }
    }
}

/// Spawn the watchdog as a tokio task and return its `JoinHandle`.
/// The task runs until aborted.
#[must_use]
pub fn spawn_watchdog(
    store: Arc<SessionStore>,
    cfg: WatchdogConfig,
    on_break: Arc<dyn Fn(ChainBreak) + Send + Sync>,
) -> JoinHandle<()> {
    tracing::info!(
        target: "audit::watchdog",
        interval_secs = cfg.interval.as_secs(),
        max_consecutive_failures = cfg.max_consecutive_failures,
        "audit chain watchdog enabled"
    );
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(cfg.interval);
        // First tick fires immediately; skip it so verify doesn't
        // race with whatever just spawned us.
        ticker.tick().await;
        let mut consecutive = 0u32;
        loop {
            ticker.tick().await;
            consecutive =
                run_tick(&store, cfg.max_consecutive_failures, consecutive, &on_break).await;
        }
    })
}

/// One iteration of the watchdog loop, factored out so unit tests
/// don't need to wait on `tokio::time::interval`. Returns the
/// updated consecutive-failure counter.
pub(crate) async fn run_tick(
    store: &SessionStore,
    threshold: u32,
    consecutive: u32,
    on_break: &Arc<dyn Fn(ChainBreak) + Send + Sync>,
) -> u32 {
    match verify_all(store).await {
        Ok(report) if report.ok => 0,
        Ok(report) => {
            let break_count = report.breaks.len();
            for b in report.breaks {
                on_break(b);
            }
            let next = consecutive.saturating_add(1);
            if next >= threshold {
                tracing::error!(
                    target: "audit::watchdog",
                    consecutive_failures = next,
                    threshold,
                    break_count,
                    "audit chain tamper alarm: consecutive failure threshold reached"
                );
            }
            next
        }
        Err(e) => {
            tracing::warn!(
                target: "audit::watchdog",
                error = %e,
                "verify_all failed; will retry on next tick"
            );
            consecutive
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuditKind, AuditSink};
    use argo_types::{SessionId, TenantId, WorkspaceId};
    use std::sync::atomic::{AtomicUsize, Ordering};

    async fn fresh() -> (Arc<SessionStore>, SessionId) {
        let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
        store.migrate().await.unwrap();
        let tenant = TenantId::new();
        let ws = WorkspaceId::new();
        sqlx::query(
            "INSERT INTO workspace (id, tenant_id, name, root_path, created_at) \
             VALUES (?, ?, 'test', '.', strftime('%s','now'))",
        )
        .bind(ws.0.as_bytes().as_slice())
        .bind(tenant.0.as_bytes().as_slice())
        .execute(store.pool())
        .await
        .unwrap();
        let session = store
            .create_session(tenant, ws, "m", "p", Some("t"), None)
            .await
            .unwrap();
        (store, session)
    }

    fn counting_cb() -> (Arc<AtomicUsize>, Arc<dyn Fn(ChainBreak) + Send + Sync>) {
        let counter = Arc::new(AtomicUsize::new(0));
        let c = Arc::clone(&counter);
        let cb: Arc<dyn Fn(ChainBreak) + Send + Sync> = Arc::new(move |_| {
            c.fetch_add(1, Ordering::SeqCst);
        });
        (counter, cb)
    }

    #[tokio::test]
    async fn clean_chain_no_alarm() {
        let (store, session) = fresh().await;
        let sink = AuditSink::new(store.clone());
        for _ in 0..3 {
            sink.write(Some(session), AuditKind::TurnStart, serde_json::json!({}), None)
                .await
                .unwrap();
        }
        let (counter, cb) = counting_cb();
        let mut consecutive = 0u32;
        for _ in 0..3 {
            consecutive = run_tick(&store, 3, consecutive, &cb).await;
        }
        assert_eq!(consecutive, 0);
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn tampered_chain_fires_callback() {
        let (store, session) = fresh().await;
        let sink = AuditSink::new(store.clone());
        sink.write(Some(session), AuditKind::TurnStart, serde_json::json!({"a": 1}), None)
            .await
            .unwrap();
        sink.write(Some(session), AuditKind::TurnEnd, serde_json::json!({}), None)
            .await
            .unwrap();
        sqlx::query("UPDATE audit_record SET payload_json = '{\"a\": 999}' WHERE seq = 1")
            .execute(store.pool())
            .await
            .unwrap();
        let (counter, cb) = counting_cb();
        let consecutive = run_tick(&store, 3, 0, &cb).await;
        assert_eq!(consecutive, 1);
        assert!(counter.load(Ordering::SeqCst) > 0);
    }

    #[tokio::test]
    async fn consecutive_failure_threshold() {
        let (store, session) = fresh().await;
        let sink = AuditSink::new(store.clone());
        sink.write(Some(session), AuditKind::TurnStart, serde_json::json!({"a": 1}), None)
            .await
            .unwrap();
        sink.write(Some(session), AuditKind::TurnEnd, serde_json::json!({}), None)
            .await
            .unwrap();
        let original: String =
            sqlx::query_scalar("SELECT payload_json FROM audit_record WHERE seq = 1")
                .fetch_one(store.pool())
                .await
                .unwrap();
        sqlx::query("UPDATE audit_record SET payload_json = '{\"a\": 999}' WHERE seq = 1")
            .execute(store.pool())
            .await
            .unwrap();
        let (_counter, cb) = counting_cb();
        let mut consecutive = 0u32;
        for _ in 0..3 {
            consecutive = run_tick(&store, 3, consecutive, &cb).await;
        }
        assert_eq!(consecutive, 3, "three tampered ticks raise to threshold");
        sqlx::query("UPDATE audit_record SET payload_json = ? WHERE seq = 1")
            .bind(&original)
            .execute(store.pool())
            .await
            .unwrap();
        consecutive = run_tick(&store, 3, consecutive, &cb).await;
        assert_eq!(consecutive, 0, "clean tick resets counter");
    }
}
