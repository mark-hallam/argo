//! `argo gateway audit` — read per-HTTP-request gateway audit rows.
use crate::cli::cli_gateway_cmds::GatewayAuditCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::gateway_request_audit::{self, GatewayRequestAuditFilter, GatewayRequestAuditRow};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Duration, Utc};

/// Run `argo gateway audit ...`.
pub async fn run(cmd: GatewayAuditCmd) -> ArgoResult<()> {
    let store = open_store().await?;
    match cmd {
        GatewayAuditCmd::List {
            since,
            principal,
            status,
            json,
            limit,
        } => {
            let rows = gateway_request_audit::list(
                store.pool(),
                &GatewayRequestAuditFilter {
                    since_micros: parse_since(since.as_deref())?,
                    principal,
                    status_code: status,
                    limit,
                },
            )
            .await?;
            render_list(&rows, json)
        }
        GatewayAuditCmd::Show { request_id, json } => {
            let Some(row) = gateway_request_audit::get(store.pool(), &request_id).await? else {
                return Err(ArgoError::Provider(format!(
                    "gateway audit: no row for request_id {request_id}"
                )));
            };
            render_show(&row, json)
        }
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

fn parse_since(raw: Option<&str>) -> ArgoResult<Option<i64>> {
    let Some(raw) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    if let Ok(ts) = DateTime::parse_from_rfc3339(raw) {
        return Ok(Some(ts.with_timezone(&Utc).timestamp_micros()));
    }
    let compact = raw.strip_suffix(" ago").unwrap_or(raw).trim();
    let duration = parse_duration(compact)?;
    Ok(Some((Utc::now() - duration).timestamp_micros()))
}

fn parse_duration(raw: &str) -> ArgoResult<Duration> {
    let (digits, unit) = raw.split_at(raw.len().saturating_sub(1));
    let value: i64 = digits
        .parse()
        .map_err(|e| ArgoError::Schema(format!("invalid --since '{raw}': {e}")))?;
    if value <= 0 {
        return Err(ArgoError::Provider(format!(
            "invalid --since '{raw}': must be positive"
        )));
    }
    match unit {
        "s" => Ok(Duration::seconds(value)),
        "m" => Ok(Duration::minutes(value)),
        "h" => Ok(Duration::hours(value)),
        "d" => Ok(Duration::days(value)),
        _ => Err(ArgoError::Provider(format!(
            "invalid --since '{raw}' (expected 30m, 1h ago, 7d, or RFC3339)"
        ))),
    }
}

fn render_list(rows: &[GatewayRequestAuditRow], json: bool) -> ArgoResult<()> {
    if json {
        let body = serde_json::json!({ "rows": rows });
        println!(
            "{}",
            serde_json::to_string(&body)
                .map_err(|e| ArgoError::Provider(format!("gateway audit list json: {e}")))?
        );
        return Ok(());
    }
    if rows.is_empty() {
        println!("argo gateway audit: no rows");
        return Ok(());
    }
    println!("request_id                            status  method  path");
    for r in rows {
        println!(
            "{:<36} {:>6}  {:<6}  {}",
            r.request_id, r.status_code, r.method, r.path
        );
    }
    Ok(())
}

fn render_show(row: &GatewayRequestAuditRow, _json: bool) -> ArgoResult<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(row)
            .map_err(|e| ArgoError::Provider(format!("gateway audit show json: {e}")))?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_since_accepts_relative_with_ago_suffix() {
        assert!(parse_since(Some("1h ago")).unwrap().is_some());
    }

    #[test]
    fn parse_since_accepts_rfc3339() {
        let got = parse_since(Some("2026-05-13T00:00:00Z")).unwrap().unwrap();
        assert_eq!(got, 1_778_630_400_000_000);
    }

    #[test]
    fn parse_since_rejects_garbage() {
        assert!(parse_since(Some("yesterday")).is_err());
    }
}
