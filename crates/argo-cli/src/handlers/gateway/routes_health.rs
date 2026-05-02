//! Health-probe handlers: legacy `/health` (combined) plus
//! K8s-style `/health/live` (cheap liveness; always 200 if the
//! process responds) and `/health/ready` (readiness; 200 only
//! when the SQLite store is reachable).
//!
//! `/health/full` (deep check including audit chain + bundle
//! integrity) still lives in `routes_meta.rs`. This file picks up
//! the cheap probes so they don't bloat the main dispatcher.
use argo_store::SessionStore;
use std::sync::Arc;

/// `GET /health` — combined liveness + timestamp. The legacy
/// shape callers depend on. Pre-dates the K8s probe split.
pub(super) fn health_response() -> (&'static str, String) {
    let now = chrono::Utc::now();
    (
        "200 OK",
        format!(
            "{{\"status\":\"ok\",\"time\":\"{}\",\"epoch_micros\":{}}}",
            now.to_rfc3339(),
            now.timestamp_micros()
        ),
    )
}

/// `GET /health/live` — minimal liveness signal. Returns 200
/// immediately with `{"status":"alive"}`. Suitable for a K8s
/// `livenessProbe` — failure on this should restart the pod.
pub(super) fn live_response() -> (&'static str, String) {
    let now = chrono::Utc::now();
    (
        "200 OK",
        format!(
            "{{\"status\":\"alive\",\"epoch_micros\":{}}}",
            now.timestamp_micros()
        ),
    )
}

/// `GET /health/ready` — readiness signal. Pings the SQLite
/// store with `SELECT 1`; returns 200 + `{"status":"ready"}`
/// when the store responds, 503 + `{"status":"not_ready",
/// "reason":"…"}` otherwise. Suitable for a K8s
/// `readinessProbe` — failure on this should remove the pod
/// from rotation but NOT restart it.
pub(super) async fn ready_response(
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return (
            "503 Service Unavailable",
            "{\"status\":\"not_ready\",\"reason\":\"store_unavailable\"}".to_string(),
        );
    };
    match sqlx::query_scalar::<_, i64>("SELECT 1").fetch_one(store.pool()).await {
        Ok(_) => {
            let now = chrono::Utc::now();
            (
                "200 OK",
                format!(
                    "{{\"status\":\"ready\",\"epoch_micros\":{}}}",
                    now.timestamp_micros()
                ),
            )
        }
        Err(e) => (
            "503 Service Unavailable",
            format!("{{\"status\":\"not_ready\",\"reason\":\"db_unreachable: {e}\"}}"),
        ),
    }
}
