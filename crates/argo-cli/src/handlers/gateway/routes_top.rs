//! Top-level read handlers extracted out of `routes.rs` to keep
//! the dispatcher under the 500-LOC budget. None of these touch
//! the streaming machinery — they're plain `(status_line, body)`
//! responders for the largest GET endpoints.

use super::store_unavailable;
use argo_store::SessionStore;
use std::sync::Arc;

/// `GET /sessions` — list every session row joined with each
/// session's spend total. Optional filters: `since-minutes`,
/// `status=open|ended`, `provider=<id>`, `model=<id>`,
/// `title-contains=<sub>`, `spent-min=<usd_micros>`,
/// `spent-max=<usd_micros>`, `limit=<N>` (default 100).
#[allow(clippy::too_many_arguments)]
pub(super) async fn sessions_response(
    store: Option<&Arc<SessionStore>>,
    since_minutes: Option<i64>,
    status: Option<&str>,
    provider: Option<&str>,
    model: Option<&str>,
    title_contains: Option<&str>,
    spent_min: Option<i64>,
    spent_max: Option<i64>,
    limit: u32,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    match store.list_all_sessions(limit).await {
        Ok(mut rows) => {
            if let Some(minutes) = since_minutes {
                let cutoff = chrono::Utc::now() - chrono::Duration::minutes(minutes);
                rows.retain(|s| s.started_at >= cutoff);
            }
            if let Some(want) = status {
                rows.retain(|s| match want {
                    "open" => s.ended_at.is_none(),
                    "ended" => s.ended_at.is_some(),
                    _ => true,
                });
            }
            if let Some(want) = provider {
                rows.retain(|s| s.provider == want);
            }
            if let Some(want) = model {
                rows.retain(|s| s.model == want);
            }
            if let Some(needle) = title_contains {
                let lower = needle.to_lowercase();
                rows.retain(|s| {
                    s.title
                        .as_deref()
                        .map(|t| t.to_lowercase().contains(&lower))
                        .unwrap_or(false)
                });
            }
            let spend_by_sid = super::spend_per_session(store).await.unwrap_or_default();
            // Spend filters apply after the cheap filters above so
            // we don't pay the lookup unless we still have rows.
            // Treat negative `?spent-min` / `?spent-max` as 0 since
            // usd_micros is non-negative by construction.
            let min_u = spent_min.map(|v| u64::try_from(v).unwrap_or(0));
            let max_u = spent_max.map(|v| u64::try_from(v).unwrap_or(0));
            if min_u.is_some() || max_u.is_some() {
                rows.retain(|s| {
                    let spent = spend_by_sid
                        .get(s.id.0.as_bytes().as_slice())
                        .copied()
                        .unwrap_or(0);
                    if let Some(min) = min_u {
                        if spent < min {
                            return false;
                        }
                    }
                    if let Some(max) = max_u {
                        if spent > max {
                            return false;
                        }
                    }
                    true
                });
            }
            let arr: Vec<serde_json::Value> = rows
                .iter()
                .map(|s| {
                    let spent = spend_by_sid
                        .get(s.id.0.as_bytes().as_slice())
                        .copied()
                        .unwrap_or(0);
                    serde_json::json!({
                        "id": s.id.0.to_string(),
                        "model": s.model,
                        "provider": s.provider,
                        "title": s.title,
                        "started_at": s.started_at.to_rfc3339(),
                        "ended_at": s.ended_at.map(|d| d.to_rfc3339()),
                        "spent_usd_micros": spent,
                    })
                })
                .collect();
            (
                "200 OK",
                serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"list sessions: {e}\"}}"),
        ),
    }
}

/// `GET /doctor` — JSON snapshot of `argo doctor`.
pub(super) async fn doctor_response() -> (&'static str, String) {
    match crate::handlers::doctor::collect().await {
        Ok(v) => (
            "200 OK",
            serde_json::to_string(&v).unwrap_or_else(|_| "{}".to_string()),
        ),
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"doctor: {e}\"}}"),
        ),
    }
}

/// `GET /health/full` — deep health (chain + skill + memory).
/// Mirrors `argo health`: returns 500 if `ok=false`, 200 otherwise.
pub(super) async fn health_full_response() -> (&'static str, String) {
    match crate::handlers::health::collect().await {
        Ok(v) => {
            let status = if v["ok"].as_bool().unwrap_or(false) {
                "200 OK"
            } else {
                "500 Internal Server Error"
            };
            (
                status,
                serde_json::to_string(&v).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"health: {e}\"}}"),
        ),
    }
}

/// `GET /audit` — chain summary: rows, breaks, kind histogram.
/// Mirrors `argo audit verify`. Sorted by descending count then
/// ascending kind label so the response is stable across runs.
pub(super) async fn audit_response(store: Option<&Arc<SessionStore>>) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let report = match argo_audit::verify_all(store).await {
        Ok(r) => r,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"verify_all: {e}\"}}"),
            );
        }
    };
    let mut kinds: Vec<(String, usize)> = report
        .kind_counts
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    kinds.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let kinds_obj: serde_json::Map<String, serde_json::Value> = kinds
        .into_iter()
        .map(|(k, v)| (k, serde_json::Value::from(v)))
        .collect();
    let payload = serde_json::json!({
        "ok": report.ok,
        "rows": report.rows,
        "breaks": report.breaks.len(),
        "kinds": kinds_obj,
    });
    (
        "200 OK",
        serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()),
    )
}
