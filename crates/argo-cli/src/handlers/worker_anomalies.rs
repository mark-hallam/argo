//! `argo worker anomalies <id>` (Stream
//! `claude-anomaly-detector`).
//!
//! Lists recent behaviour anomaly events for one worker. Each
//! event shows the dimension that drifted (`latency_ms` /
//! `retry_count`), the observed value, the prior baseline mean +
//! stddev, the z-score that fired, and when it was recorded.
//!
//! Operationally this complements the circuit breaker: the
//! breaker trips on consecutive failures; the anomaly detector
//! catches subtler drift the breaker would miss (sudden latency
//! spikes that don't yet produce errors, retry-count creep, etc.).
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::anomaly::{list_for_worker, AnomalyEvent};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::sync::Arc;

/// Top-level entry.
pub async fn run(
    worker_id: &str,
    since_minutes: Option<i64>,
    limit: Option<u32>,
    json: bool,
) -> ArgoResult<()> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = Arc::new(SessionStore::open(&db_path(&home)).await?);
    store.migrate().await?;
    let since_micros = since_minutes
        .map(|m| (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros());
    let events = list_for_worker(store.pool(), worker_id, since_micros, limit).await?;
    if json {
        for e in &events {
            println!(
                "{}",
                serde_json::to_string(e).unwrap_or_else(|_| "{}".to_string())
            );
        }
    } else {
        render_text(worker_id, &events);
    }
    Ok(())
}

fn render_text(worker_id: &str, events: &[AnomalyEvent]) {
    if events.is_empty() {
        println!("worker {worker_id}: (no anomalies)");
        return;
    }
    println!(
        "worker {worker_id}: {} recent anomaly event(s):",
        events.len()
    );
    println!(
        "  {:<22} {:<14} {:>10} {:>10} {:>8} {:<22}",
        "observed_at", "dimension", "observed", "mean", "z", "action"
    );
    for e in events {
        let observed_at = e.observed_at.format("%Y-%m-%d %H:%M:%S").to_string();
        let action_short = if e.action_id.len() > 22 {
            e.action_id[..22].to_string()
        } else {
            e.action_id.clone()
        };
        println!(
            "  {:<22} {:<14} {:>10.3} {:>10.3} {:>+8.3} {action_short}",
            observed_at, e.dimension, e.observed, e.baseline_mean, e.z_score
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_store::anomaly::{record_sample, BehaviorSample};
    use argo_store::SessionStore;
    use chrono::Utc;

    async fn fresh_pool() -> sqlx::SqlitePool {
        // worker_anomaly_event + worker_behavior_baseline now ship via
        // the migration ladder (v19); a migrated SessionStore replaces
        // the old anomaly::ensure_schema bootstrap.
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        store.pool().clone()
    }

    #[tokio::test]
    async fn list_returns_events_in_descending_order() {
        let pool = fresh_pool().await;
        // Seed 6 baseline samples + drop a spike.
        for v in [100, 102, 98, 101, 99, 100] {
            record_sample(
                &pool,
                "w-1",
                "seed",
                "post_listing",
                BehaviorSample {
                    latency_ms: v,
                    retry_count: 0,
                },
                3.0,
                Utc::now(),
            )
            .await
            .unwrap();
        }
        record_sample(
            &pool,
            "w-1",
            "spike",
            "post_listing",
            BehaviorSample {
                latency_ms: 5_000,
                retry_count: 0,
            },
            3.0,
            Utc::now(),
        )
        .await
        .unwrap();
        let evts = argo_store::anomaly::list_for_worker(&pool, "w-1", None, None)
            .await
            .unwrap();
        assert!(!evts.is_empty(), "spike should produce an anomaly row");
        // Descending by observed_at — most recent first.
        for w in evts.windows(2) {
            assert!(w[0].observed_at >= w[1].observed_at);
        }
    }

    #[test]
    fn render_text_handles_empty_list() {
        // smoke: just ensure no panic.
        render_text("w-1", &[]);
    }
}
