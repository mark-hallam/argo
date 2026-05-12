//! `argo worker restarts [--worker <id>] [--since <duration>]` —
//! read-only listing of `worker.oom_restart` audit-chain rows
//! emitted by the supervisor when a worker subprocess dies
//! (mission-oom-restart-handler / MISSION_BACKLOG §8 #237).
//!
//! Restarts are immutable audit history; this command is purely
//! a viewer. There are intentionally no `--clear` or `--retry-
//! budget-reset` flags — see the brief's "Notes for the
//! implementer" section.
//!
//! Output is JSON-lines: one object per row, sorted by
//! `created_at_micros DESC`. Operators can pipe into `jq` or
//! `argo audit show` for richer rendering.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_runtime::oom_restart::AUDIT_KIND_LABEL;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{Duration, Utc};
use sqlx::{Pool, Sqlite};

/// Top-level entry. `worker_id` is an optional filter; when
/// `Some`, narrows to rows whose `payload_json.worker_id`
/// matches the supplied string. `since` is the same suffix
/// syntax as `argo worker success` (`30d` / `12h` / `45m` /
/// `90s`).
pub async fn run(worker_id: Option<&str>, since: Option<&str>) -> ArgoResult<()> {
    let cutoff_micros = since.map(parse_since).transpose()?;
    let pool = open_pool().await?;
    let rows = fetch_rows(&pool, worker_id, cutoff_micros).await?;
    for row in &rows {
        let line = serde_json::to_string(row)
            .map_err(|e| ArgoError::Provider(format!("serialize restart row: {e}")))?;
        println!("{line}");
    }
    Ok(())
}

/// One emitted JSON-line. Mirrors the supervisor's audit
/// payload plus the row-level `created_at_micros` so consumers
/// can sort/filter on persistence time even when the
/// `recorded_at` field disagrees with wall clock (NTP slew etc).
#[derive(Debug, serde::Serialize)]
pub struct RestartRow {
    pub created_at_micros: i64,
    pub payload: serde_json::Value,
}

async fn fetch_rows(
    pool: &Pool<Sqlite>,
    worker_id: Option<&str>,
    cutoff_micros: Option<i64>,
) -> ArgoResult<Vec<RestartRow>> {
    let raw: Vec<(String, i64)> = if let Some(cutoff) = cutoff_micros {
        sqlx::query_as(
            "SELECT payload_json, created_at FROM audit_record \
             WHERE kind = ? AND created_at >= ? \
             ORDER BY created_at DESC",
        )
        .bind(AUDIT_KIND_LABEL)
        .bind(cutoff)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as(
            "SELECT payload_json, created_at FROM audit_record \
             WHERE kind = ? \
             ORDER BY created_at DESC",
        )
        .bind(AUDIT_KIND_LABEL)
        .fetch_all(pool)
        .await
    }
    .map_err(|e| ArgoError::Provider(format!("worker restarts fetch: {e}")))?;
    let mut out = Vec::with_capacity(raw.len());
    for (payload_str, micros) in raw {
        let Ok(payload) = serde_json::from_str::<serde_json::Value>(&payload_str) else {
            continue;
        };
        if let Some(filter) = worker_id {
            let matches = payload
                .get("worker_id")
                .and_then(|v| v.as_str())
                .is_some_and(|s| s == filter);
            if !matches {
                continue;
            }
        }
        out.push(RestartRow {
            created_at_micros: micros,
            payload,
        });
    }
    Ok(out)
}

fn parse_since(raw: &str) -> ArgoResult<i64> {
    let raw = raw.trim();
    if raw.len() < 2 {
        return Err(ArgoError::Provider(format!(
            "invalid --since '{raw}' (e.g. 30d, 12h, 45m, 90s)"
        )));
    }
    let (digits, unit) = raw.split_at(raw.len() - 1);
    let value: i64 = digits
        .parse()
        .map_err(|e| ArgoError::Provider(format!("invalid --since '{raw}': {e}")))?;
    if value <= 0 {
        return Err(ArgoError::Provider(format!(
            "--since must be positive: {raw}"
        )));
    }
    let dur = match unit {
        "s" => Duration::seconds(value),
        "m" => Duration::minutes(value),
        "h" => Duration::hours(value),
        "d" => Duration::days(value),
        other => {
            return Err(ArgoError::Provider(format!(
                "unsupported --since suffix '{other}' in '{raw}' (expected s/m/h/d)"
            )));
        }
    };
    Ok((Utc::now() - dur).timestamp_micros())
}

async fn open_pool() -> ArgoResult<Pool<Sqlite>> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(store.pool().clone())
}
