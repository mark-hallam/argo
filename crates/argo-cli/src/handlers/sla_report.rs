//! `argo sla report` — availability + latency-percentile rollup over
//! `audit_record kind='http.request'` rows. Mirrors the gateway-side
//! `GET /api/v1/dashboards/sla` envelope so CLI and SPA share the
//! same SLO numbers.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::audit_record_query::{
    availability_in_window_filtered, latency_percentiles_in_window_filtered, AvailabilitySnapshot,
    ErrorRateFilter, LatencyPercentiles,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;

pub async fn run(window: &str, tenant: Option<&str>, json_output: bool) -> ArgoResult<()> {
    let store = open_store().await?;
    let (avail, pcts) = load(&store, window, tenant).await?;
    if json_output {
        println!("{}", envelope(&avail, &pcts, tenant));
    } else {
        render_table(&avail, &pcts, tenant);
    }
    Ok(())
}

pub async fn gateway_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return (
            "503 Service Unavailable",
            json!({"error": "store unavailable"}).to_string(),
        );
    };
    let window = query_param(req, "window").unwrap_or_else(|| "24h".to_string());
    let tenant = query_param(req, "tenant");
    match load(store, &window, tenant.as_deref()).await {
        Ok((avail, pcts)) => ("200 OK", envelope(&avail, &pcts, tenant.as_deref())),
        Err(e) => (
            "400 Bad Request",
            json!({"error": e.to_string()}).to_string(),
        ),
    }
}

async fn open_store() -> ArgoResult<SessionStore> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}

async fn load(
    store: &SessionStore,
    window: &str,
    tenant: Option<&str>,
) -> ArgoResult<(AvailabilitySnapshot, LatencyPercentiles)> {
    let secs = parse_window_secs(window)?;
    let end = Utc::now().timestamp_micros();
    let start = end.saturating_sub((secs as i64).saturating_mul(1_000_000));
    let filter = ErrorRateFilter {
        tenant_id: tenant.map(str::to_string),
        route: None,
    };
    let avail = availability_in_window_filtered(store, start, end, &filter).await?;
    let pcts = latency_percentiles_in_window_filtered(store, start, end, &filter).await?;
    Ok((avail, pcts))
}

fn parse_window_secs(raw: &str) -> ArgoResult<u64> {
    let raw = raw.trim();
    if raw.len() < 2 || !raw.is_ascii() {
        return Err(ArgoError::Provider(format!(
            "invalid --window '{raw}' (expected 30m, 1h, or 7d)"
        )));
    }
    let (digits, unit) = raw.split_at(raw.len() - 1);
    let value: u64 = digits
        .parse()
        .map_err(|e| ArgoError::Schema(format!("invalid --window '{raw}': {e}")))?;
    if value == 0 {
        return Err(ArgoError::Provider(format!(
            "invalid --window '{raw}': must be positive"
        )));
    }
    match unit {
        "s" => Ok(value),
        "m" => Ok(value.saturating_mul(60)),
        "h" => Ok(value.saturating_mul(3600)),
        "d" => Ok(value.saturating_mul(86_400)),
        _ => Err(ArgoError::Provider(format!(
            "invalid --window '{raw}' (expected 30m, 1h, or 7d)"
        ))),
    }
}

fn envelope(avail: &AvailabilitySnapshot, pcts: &LatencyPercentiles, tenant: Option<&str>) -> String {
    json!({
        "availability": avail,
        "success_pct": avail.success_pct(),
        "latency": pcts,
        "filters": { "tenant": tenant },
    })
    .to_string()
}

fn render_table(avail: &AvailabilitySnapshot, pcts: &LatencyPercentiles, tenant: Option<&str>) {
    println!("sla report");
    println!(
        "window: {}..{} tenant={}",
        avail.start_ts,
        avail.end_ts,
        tenant.unwrap_or("*")
    );
    println!(
        "total={} success={} success_pct={:.3}",
        avail.total,
        avail.success_count,
        avail.success_pct()
    );
    println!(
        "latency_us  p50={} p95={} p99={} (samples={})",
        pcts.p50_us, pcts.p95_us, pcts.p99_us, pcts.total
    );
}

fn query_param(req: &str, key: &str) -> Option<String> {
    let target = req.strip_prefix("GET ").unwrap_or(req);
    let q = target.split_once('?')?.1;
    let q = q.split_once(' ').map(|(q, _)| q).unwrap_or(q);
    for kv in q.split('&') {
        if let Some((k, v)) = kv.split_once('=') {
            if k == key {
                return Some(v.to_string());
            }
        }
    }
    None
}
