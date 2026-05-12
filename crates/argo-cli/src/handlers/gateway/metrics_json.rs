//! `GET /metrics/json` — JSON variant of `/metrics` for clients
//! that don't speak Prometheus text exposition (TUI dashboards,
//! log-shippers, ad-hoc curl scripts). Sibling of `metrics.rs`
//! so neither file breaches the per-file 500-LOC budget.
//!
//! Returns a single object keyed by metric name with the same
//! numbers `/metrics` emits. Errors from individual SQL queries
//! surface as `{metric_name: null, _errors: [...]}` so a partial
//! DB never breaks the whole response.

use argo_store::SessionStore;
use std::sync::Arc;

/// Render the JSON exposition body. Mirrors the contract of
/// `metrics::render`: scrape-counter increments, partial-DB
/// errors surface in `_errors`, store-unavailable still returns
/// a parseable object.
pub(super) async fn render_json(store: Option<&Arc<SessionStore>>) -> String {
    let scrape_n = super::metrics::next_scrape_n();
    let mut payload = serde_json::Map::new();
    payload.insert(
        "argo_build_info".into(),
        serde_json::json!({"version": env!("CARGO_PKG_VERSION")}),
    );
    payload.insert(
        "argo_uptime_seconds".into(),
        serde_json::json!(super::metrics::uptime_seconds()),
    );
    payload.insert(
        "argo_metrics_scrapes_total".into(),
        serde_json::json!(scrape_n),
    );
    let mut errors: Vec<String> = Vec::new();
    let Some(store) = store else {
        payload.insert("argo_store_available".into(), serde_json::json!(0));
        return finalize_json(payload, errors);
    };
    payload.insert("argo_store_available".into(), serde_json::json!(1));
    let pool = store.pool();
    insert_count(
        &mut payload,
        &mut errors,
        pool,
        "argo_sessions_total",
        "SELECT COUNT(*) FROM session",
    )
    .await;
    insert_count(
        &mut payload,
        &mut errors,
        pool,
        "argo_sessions_open",
        "SELECT COUNT(*) FROM session WHERE ended_at IS NULL",
    )
    .await;
    insert_count(
        &mut payload,
        &mut errors,
        pool,
        "argo_messages_total",
        "SELECT COUNT(*) FROM message",
    )
    .await;
    insert_count(
        &mut payload,
        &mut errors,
        pool,
        "argo_audit_rows_total",
        "SELECT COUNT(*) FROM audit_record",
    )
    .await;
    insert_count(
        &mut payload,
        &mut errors,
        pool,
        "argo_skills_active_total",
        "SELECT COUNT(*) FROM skill WHERE superseded_by IS NULL",
    )
    .await;
    insert_count(
        &mut payload,
        &mut errors,
        pool,
        "argo_workspace_artifacts_total",
        "SELECT COUNT(*) FROM marketplace_artifact",
    )
    .await;
    insert_count(
        &mut payload,
        &mut errors,
        pool,
        "argo_forge_runs_running",
        "SELECT COUNT(*) FROM forge_run WHERE status = 'running'",
    )
    .await;
    insert_count(
        &mut payload,
        &mut errors,
        pool,
        "argo_workspaces_total",
        "SELECT COUNT(*) FROM workspace",
    )
    .await;
    insert_count(
        &mut payload,
        &mut errors,
        pool,
        "argo_memory_entries_total",
        "SELECT COUNT(*) FROM memory_entry WHERE superseded_by IS NULL",
    )
    .await;
    insert_count(
        &mut payload,
        &mut errors,
        pool,
        "argo_judge_records_total",
        "SELECT COUNT(*) FROM judge_record",
    )
    .await;
    insert_count(
        &mut payload,
        &mut errors,
        pool,
        "argo_spend_usd_micros_total",
        "SELECT COALESCE(SUM(amount_usd_micros), 0) FROM spend_ledger",
    )
    .await;
    let by_status: Result<Vec<(String, i64)>, _> =
        sqlx::query_as("SELECT status, COUNT(*) FROM forge_run GROUP BY status")
            .fetch_all(pool)
            .await;
    let mut status_map = serde_json::Map::new();
    match by_status {
        Ok(rows) => {
            for (s, n) in rows {
                status_map.insert(s, serde_json::json!(n));
            }
        }
        Err(e) => {
            errors.push(format!("forge_runs_by_status: {e}"));
        }
    }
    payload.insert(
        "argo_forge_runs_total".into(),
        serde_json::Value::Object(status_map),
    );
    // Worker action ledger gauges (#757) — same shape as the
    // text exposition's action block, packaged into the JSON
    // payload so consumers don't have to parse the Prometheus
    // text format.
    let action_counts = super::metrics_actions::gather(store).await;
    super::metrics_actions::render_json(&mut payload, &action_counts);
    // Worker-pool elasticity gauges (mission-worker-pool-elasticity).
    let latest = argo_runtime::latest_tick();
    let (active, depth, target) = latest.as_ref().map_or((0, 0, 0), |t| {
        (
            t.snapshot.active_workers,
            t.snapshot.queue_depth,
            t.recommendation.target_or(t.snapshot.active_workers),
        )
    });
    payload.insert("argo_worker_pool_active_total".into(), serde_json::json!(active));
    payload.insert("argo_worker_pool_queue_depth".into(), serde_json::json!(depth));
    payload.insert("argo_worker_pool_target".into(), serde_json::json!(target));
    payload.insert(
        "argo_worker_pool_scale_events_total".into(),
        serde_json::json!({
            "up": argo_runtime::scale_up_total(),
            "down": argo_runtime::scale_down_total(),
        }),
    );
    match argo_audit::verify_all(store).await {
        Ok(report) => {
            payload.insert(
                "argo_audit_chain_ok".into(),
                serde_json::json!(if report.ok { 1 } else { 0 }),
            );
            payload.insert(
                "argo_audit_chain_breaks".into(),
                serde_json::json!(report.breaks.len()),
            );
        }
        Err(e) => {
            errors.push(format!("verify_all: {e}"));
            payload.insert("argo_audit_chain_ok".into(), serde_json::json!(0));
            payload.insert("argo_audit_chain_breaks".into(), serde_json::json!(0));
        }
    }
    finalize_json(payload, errors)
}

fn finalize_json(
    mut payload: serde_json::Map<String, serde_json::Value>,
    errors: Vec<String>,
) -> String {
    if !errors.is_empty() {
        payload.insert("_errors".into(), serde_json::json!(errors));
    }
    serde_json::to_string(&serde_json::Value::Object(payload)).unwrap_or_else(|_| "{}".to_string())
}

async fn insert_count(
    payload: &mut serde_json::Map<String, serde_json::Value>,
    errors: &mut Vec<String>,
    pool: &sqlx::SqlitePool,
    name: &str,
    sql: &str,
) {
    match sqlx::query_scalar::<_, i64>(sql).fetch_one(pool).await {
        Ok(n) => {
            payload.insert(name.into(), serde_json::json!(n));
        }
        Err(e) => {
            errors.push(format!("{name}: {e}"));
            payload.insert(name.into(), serde_json::Value::Null);
        }
    }
}
