//! `argo budget anomalies` + `argo budget anomaly-tick` handlers
//! (Stream `claude-spend-anomaly-alerts`). Sibling of `budget.rs`
//! so the dispatcher there stays focused on the read-side rollups.
//!
//! `list` is the read-side surface — query the
//! `worker_spend_anomaly` table with optional `--worker` /
//! `--since` filters.
//!
//! `tick` is the write-side surface intended to run on cron (or
//! a tokio interval — that wiring lands as a v1.x follow-up):
//! walks every active worker, folds the previous-hour
//! USD-per-hour spend into the (worker, hour-of-day) baseline,
//! and writes a `worker_spend_anomaly` row when |z| exceeds the
//! configured threshold.
use argo_store::spend_anomaly::{
    configured_threshold, list_active_worker_ids, list_anomalies, load_baseline,
    observe_recent_hour, record_anomaly, save_baseline, AnomalyFilters, SpendAnomaly,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::json;

/// `argo budget anomalies` — read recent anomaly events.
pub(crate) async fn list(
    store: &SessionStore,
    worker: Option<&str>,
    since: Option<DateTime<Utc>>,
    limit: u32,
    json: bool,
) -> ArgoResult<()> {
    let filters = AnomalyFilters {
        worker_id: worker,
        since_micros: since.map(|t| t.timestamp_micros()),
    };
    let rows = list_anomalies(store.pool(), &filters, limit).await?;
    if json {
        let payload = json!({
            "count": rows.len(),
            "anomalies": rows.iter().map(anomaly_to_json).collect::<Vec<_>>(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
        return Ok(());
    }
    println!(
        "spend anomalies ({} rows; threshold |z| > {:.1})",
        rows.len(),
        configured_threshold()
    );
    if rows.is_empty() {
        println!("  (none — all workers within historical bounds)");
        return Ok(());
    }
    println!(
        "  {:<26} {:<14} {:>8} {:>10} {:>10} {:>10}",
        "id", "worker", "hour_utc", "observed", "mean", "z"
    );
    for row in &rows {
        println!(
            "  {:<26} {:<14} {:>8} ${:>9.4} ${:>9.4} {:>+10.2}",
            short(&row.id),
            short(&row.worker_id),
            row.hour_of_day,
            row.observed_usd_per_hour,
            row.baseline_mean_usd,
            row.z_score,
        );
    }
    Ok(())
}

/// `argo budget anomaly-tick` — run one detection pass.
pub(crate) async fn tick(
    store: &SessionStore,
    now: DateTime<Utc>,
    json: bool,
) -> ArgoResult<()> {
    let pool = store.pool();
    let workers = list_active_worker_ids(pool).await?;
    let threshold = configured_threshold();
    let mut decisions: Vec<TickDecision> = Vec::new();
    for worker_id in &workers {
        let (hour_of_day, observed) = observe_recent_hour(pool, worker_id, now).await?;
        let mut baseline = load_baseline(pool, worker_id, hour_of_day).await?;
        // Compute the z-score against the PRE-update baseline so
        // a single outlier doesn't poison its own detection
        // (the new sample shifts the mean toward itself).
        let z = baseline.z_score_for(observed);
        baseline.update(observed, now.timestamp_micros());
        save_baseline(pool, &baseline).await?;
        let mut decision = TickDecision {
            worker_id: worker_id.clone(),
            hour_of_day,
            observed_usd_per_hour: observed,
            mean_usd_per_hour: baseline.mean_usd_per_hour,
            stddev_usd_per_hour: baseline.stddev(),
            z_score: z,
            anomaly_id: None,
        };
        if let Some(z) = z {
            if z.abs() > threshold {
                let id = record_anomaly(
                    pool,
                    worker_id,
                    observed,
                    decision.mean_usd_per_hour,
                    decision.stddev_usd_per_hour,
                    z,
                    hour_of_day,
                    now.timestamp_micros(),
                )
                .await?;
                // Alert surface — tracing::warn! gets ingested by
                // any log shipper (Datadog, Loki, OpenTelemetry,
                // etc.) the operator already runs, so this IS a
                // notification with no extra wiring. Full
                // NotifyChannel `send()` integration (per the
                // brief's "re-uses existing channels") is deferred
                // to v1.x: it needs channel selection logic
                // (worker.channels first, workspace default
                // fallback) + the gateway to drive the tick (so
                // channel manifest loading isn't repeated per
                // cron call).
                tracing::warn!(
                    target: "argo_cli::budget_anomalies",
                    anomaly_id = %id,
                    worker_id = %worker_id,
                    hour_of_day = hour_of_day,
                    observed_usd_per_hour = observed,
                    baseline_mean_usd = decision.mean_usd_per_hour,
                    baseline_stddev_usd = decision.stddev_usd_per_hour,
                    z_score = z,
                    threshold = threshold,
                    "spend anomaly: worker {} burned ${:.4}/hr (mean ${:.4}, |z|={:.2})",
                    worker_id, observed, decision.mean_usd_per_hour, z.abs(),
                );
                decision.anomaly_id = Some(id);
            }
        }
        decisions.push(decision);
    }
    if json {
        let payload = json!({
            "now_micros": now.timestamp_micros(),
            "threshold": threshold,
            "worker_count": workers.len(),
            "decisions": decisions,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
        return Ok(());
    }
    let triggered = decisions.iter().filter(|d| d.anomaly_id.is_some()).count();
    println!(
        "spend anomaly tick: {} workers checked, {} anomalies recorded (|z| > {:.1})",
        workers.len(),
        triggered,
        threshold
    );
    for d in &decisions {
        let z = d
            .z_score
            .map(|z| format!("{z:+.2}"))
            .unwrap_or_else(|| "(no-baseline)".to_string());
        let marker = if d.anomaly_id.is_some() { "!" } else { " " };
        println!(
            "  {marker} worker={} hour_utc={} observed=${:.4} mean=${:.4} z={z}",
            short(&d.worker_id),
            d.hour_of_day,
            d.observed_usd_per_hour,
            d.mean_usd_per_hour,
        );
    }
    Ok(())
}

fn anomaly_to_json(row: &SpendAnomaly) -> serde_json::Value {
    json!({
        "id": row.id,
        "worker_id": row.worker_id,
        "observed_usd_per_hour": row.observed_usd_per_hour,
        "baseline_mean_usd": row.baseline_mean_usd,
        "baseline_stddev_usd": row.baseline_stddev_usd,
        "z_score": row.z_score,
        "hour_of_day": row.hour_of_day,
        "observed_at_micros": row.observed_at_micros,
    })
}

fn short(s: &str) -> String {
    s.chars().take(12).collect()
}

#[derive(Debug, Clone, Serialize)]
struct TickDecision {
    worker_id: String,
    hour_of_day: u8,
    observed_usd_per_hour: f64,
    mean_usd_per_hour: f64,
    stddev_usd_per_hour: f64,
    z_score: Option<f64>,
    anomaly_id: Option<String>,
}

#[cfg(test)]
#[path = "budget_anomalies_tests.rs"]
mod tests;

/// Replace the [`ArgoError`] mapper if needed in the future. Keeps
/// the symbol public so the v1.x notify-channel hook can reuse it
/// without re-implementing.
#[doc(hidden)]
pub(crate) fn _into_err(msg: String) -> ArgoError {
    ArgoError::Provider(msg)
}
