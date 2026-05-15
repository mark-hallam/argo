use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::audit_record_query::{
    error_rate_in_window_filtered, ErrorRateFilter, ErrorRateSnapshot,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;

pub async fn run(
    window: &str,
    tenant: Option<&str>,
    route: Option<&str>,
    json_output: bool,
) -> ArgoResult<()> {
    let store = open_store().await?;
    let snapshot = load_snapshot(&store, window, tenant, route).await?;
    if json_output {
        println!("{}", envelope(&snapshot, tenant, route));
    } else {
        render_table(&snapshot, tenant, route);
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
    let window = query_param(req, "window").unwrap_or_else(|| "1h".to_string());
    let tenant = query_param(req, "tenant");
    let route = query_param(req, "route").map(|s| s.replace('+', " "));
    match load_snapshot(store, &window, tenant.as_deref(), route.as_deref()).await {
        Ok(snapshot) => (
            "200 OK",
            envelope(&snapshot, tenant.as_deref(), route.as_deref()),
        ),
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

async fn load_snapshot(
    store: &SessionStore,
    window: &str,
    tenant: Option<&str>,
    route: Option<&str>,
) -> ArgoResult<ErrorRateSnapshot> {
    let secs = parse_window_secs(window)?;
    let end = Utc::now().timestamp_micros();
    let start = end.saturating_sub((secs as i64).saturating_mul(1_000_000));
    let filter = ErrorRateFilter {
        tenant_id: tenant.map(str::to_string),
        route: route.map(str::to_string),
    };
    error_rate_in_window_filtered(store, start, end, &filter).await
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

fn envelope(snapshot: &ErrorRateSnapshot, tenant: Option<&str>, route: Option<&str>) -> String {
    json!({
        "snapshot": snapshot,
        "error_rate_pct": snapshot.error_rate_pct(),
        "filters": {
            "tenant": tenant,
            "route": route,
        }
    })
    .to_string()
}

fn render_table(snapshot: &ErrorRateSnapshot, tenant: Option<&str>, route: Option<&str>) {
    println!("error-rate dashboard");
    println!(
        "window: {}..{} tenant={} route={}",
        snapshot.start_ts,
        snapshot.end_ts,
        tenant.unwrap_or("*"),
        route.unwrap_or("*")
    );
    println!(
        "total={} errors={} error_rate_pct={:.2}",
        snapshot.total,
        snapshot.errors,
        snapshot.error_rate_pct()
    );
    println!("\nstatus  count");
    for (status, count) in &snapshot.per_status_histogram {
        println!("{status:>6}  {count}");
    }
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
