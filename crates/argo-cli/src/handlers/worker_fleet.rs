//! `argo worker fleet` — presence view over live workers.
//!
//! Stream `claude-worker-presence-tracking`. Sorts by
//! `worker.last_active_at DESC` with NULLs last; renders idle
//! (>30 days since last seen) and never-seen rows distinctly so an
//! operator can answer "which workers are actually working right
//! now?" without joining audit history.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::{worker_presence, SessionStore};
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::Utc;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
struct FleetOut<'a> {
    ok: bool,
    count: usize,
    rows: &'a [worker_presence::FleetRow],
}

/// Run `argo worker fleet [--tenant <id>] [--limit N] [--since 7d] [--json]`.
pub async fn run(
    tenant: Option<&str>,
    limit: u32,
    since: Option<&str>,
    json: bool,
) -> ArgoResult<()> {
    let tenant_id = tenant.map(parse_tenant).transpose()?;
    let tenant_string = tenant_id.map(|t| t.to_string());
    let since_epoch = since.map(since_cutoff).transpose()?;
    let store = open_store().await?;
    worker_presence::ensure_schema(store.pool()).await?;
    let rows =
        worker_presence::fleet(store.pool(), tenant_string.as_deref(), limit, since_epoch).await?;

    if json {
        print_json(&FleetOut {
            ok: true,
            count: rows.len(),
            rows: &rows,
        })
    } else {
        print_table(&rows);
        Ok(())
    }
}

fn since_cutoff(raw: &str) -> ArgoResult<i64> {
    let seconds = parse_duration_secs(raw)?;
    Utc::now()
        .timestamp()
        .checked_sub(seconds)
        .ok_or_else(|| ArgoError::Provider(format!("--since duration '{raw}' is out of range")))
}

pub(crate) fn parse_duration_secs(raw: &str) -> ArgoResult<i64> {
    let trimmed = raw.trim();
    let digits_len = trimmed.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits_len == 0 || digits_len == trimmed.len() {
        return Err(ArgoError::Provider(
            "--since expects a duration like 7d, 12h, 30m, or 60s".into(),
        ));
    }
    let (digits, unit) = trimmed.split_at(digits_len);
    let value: i64 = digits
        .parse()
        .map_err(|e| ArgoError::Provider(format!("--since duration '{raw}': {e}")))?;
    let multiplier = match unit {
        "s" => 1,
        "m" => 60,
        "h" => 60 * 60,
        "d" => 24 * 60 * 60,
        _ => {
            return Err(ArgoError::Provider(format!(
                "--since unit '{unit}' is not supported; use s, m, h, or d"
            )));
        }
    };
    value
        .checked_mul(multiplier)
        .ok_or_else(|| ArgoError::Provider(format!("--since duration '{raw}' is too large")))
}

async fn open_store() -> ArgoResult<SessionStore> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}

fn print_table(rows: &[worker_presence::FleetRow]) {
    if rows.is_empty() {
        println!("(no workers)");
        return;
    }
    println!(
        "| worker_id | persona | last_active | status |"
    );
    println!("| --- | --- | --- | --- |");
    for row in rows {
        let persona = row.persona.as_deref().unwrap_or("-");
        let last = match row.last_active_at_rfc3339.as_deref() {
            Some(rfc) => rfc.to_string(),
            None => "(never seen)".to_string(),
        };
        println!(
            "| `{}` | {} | {} | {} |",
            row.worker_id, persona, last, row.status
        );
    }
}

fn print_json<T: Serialize>(value: &T) -> ArgoResult<()> {
    let body = serde_json::to_string_pretty(value)
        .map_err(|e| ArgoError::Provider(format!("serialize worker fleet JSON: {e}")))?;
    println!("{body}");
    Ok(())
}

fn parse_tenant(value: &str) -> ArgoResult<TenantId> {
    Uuid::parse_str(value)
        .map(TenantId)
        .map_err(|e| ArgoError::Schema(format!("invalid tenant id '{value}': {e}")))
}
