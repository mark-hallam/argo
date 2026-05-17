//! Stale-worker probe for `argo doctor`.

use crate::home::db_path;
use argo_store::SessionStore;
use serde_json::{json, Value};
use std::path::Path;

pub(super) async fn collect(home: &Path, cwd: &Path) -> Value {
    let db = db_path(home);
    if !db.exists() {
        return json!({"status": "OK", "stale_count": 0, "detail": "no sessions.db yet"});
    }
    let store = match SessionStore::open(&db).await {
        Ok(s) => s,
        Err(e) => return fail(format!("open store: {e}")),
    };
    if let Err(e) = store.migrate().await {
        return fail(format!("migrate: {e}"));
    }
    if let Err(e) = argo_store::worker_presence::ensure_schema(store.pool()).await {
        return fail(format!("worker presence schema: {e}"));
    }
    let stale_hours = doctor_u64(cwd, "worker_stale_hours", 24);
    let cutoff = chrono::Utc::now()
        .timestamp()
        .saturating_sub((stale_hours.saturating_mul(3600)) as i64);
    let row = sqlx::query_as::<_, (i64, i64)>(
        "SELECT COUNT(*), COALESCE(MIN(last_active_at), 0) FROM worker \
         WHERE archived_at IS NULL AND (last_active_at IS NULL OR last_active_at < ?)",
    )
    .bind(cutoff)
    .fetch_one(store.pool())
    .await;
    let (stale, oldest) = match row {
        Ok(v) => v,
        Err(e) => return fail(format!("worker staleness query: {e}")),
    };
    json!({
        "status": if stale == 0 { "OK" } else { "WARN" },
        "stale_count": stale,
        "threshold_hours": stale_hours,
        "oldest_last_active_at": if oldest == 0 { Value::Null } else { json!(oldest) },
        "detail": if stale == 0 { "no stale active workers" } else { "active workers stale beyond threshold" }
    })
}

pub(super) fn print_text(v: &Value) {
    println!("  Stale workers:");
    println!(
        "    {:<4} {} (threshold={}h)",
        v["status"].as_str().unwrap_or("WARN"),
        v["detail"].as_str().unwrap_or(""),
        v["threshold_hours"].as_u64().unwrap_or(24)
    );
}

fn fail(detail: impl Into<String>) -> Value {
    json!({"status": "FAIL", "detail": detail.into()})
}

fn doctor_u64(cwd: &Path, key: &str, default: u64) -> u64 {
    std::fs::read_to_string(cwd.join("argo.toml"))
        .ok()
        .and_then(|body| toml::from_str::<toml::Value>(&body).ok())
        .and_then(|v| v.get("doctor")?.get(key)?.as_integer())
        .and_then(|n| u64::try_from(n).ok())
        .unwrap_or(default)
}
