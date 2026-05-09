//! Worker presence helpers (`last_active_at` + fleet view).
//!
//! Mission claim 1 ("persistent identity over time") needs an
//! observable notion of time-since-active. Presence is stored on
//! the live `worker` row so operators can answer "which of my
//! workers is actually working right now?" without scanning every
//! audit row.
//!
//! Schema is owned by `migrations/42__worker_presence.sql`; the
//! [`ensure_schema`] runtime helper adds the column + index
//! idempotently so audit/session writers can call [`touch_presence`]
//! on first use without depending on test-harness migration order.

use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{Row, SqlitePool};

/// Workers idle for longer than this render with status `"idle"`.
pub const IDLE_THRESHOLD_SECS: i64 = 30 * 24 * 60 * 60;

/// One row in the worker fleet presence view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FleetRow {
    /// Worker id (UUIDv7).
    pub worker_id: String,
    /// Tenant boundary that owns the worker.
    pub tenant_id: String,
    /// Vertical template id this worker instantiates.
    pub vertical_id: String,
    /// Public persona name when the persona row is present.
    pub persona: Option<String>,
    /// Epoch seconds for the last known activity, or `None`
    /// when the worker has never been seen.
    pub last_active_at: Option<i64>,
    /// RFC-3339 rendering of [`Self::last_active_at`].
    pub last_active_at_rfc3339: Option<String>,
    /// `"active"` when seen within `IDLE_THRESHOLD_SECS`, otherwise
    /// `"idle"`. Workers with `last_active_at = NULL` are also `"idle"`.
    pub status: String,
}

/// Ensure the `worker.last_active_at` column and supporting index
/// exist. Safe to call repeatedly; gated on `PRAGMA table_info(worker)`
/// so it never raises on second-run.
pub async fn ensure_schema(pool: &SqlitePool) -> ArgoResult<()> {
    if !table_exists(pool, "worker").await? {
        return Ok(());
    }
    if !column_exists(pool, "worker", "last_active_at").await? {
        sqlx::query("ALTER TABLE worker ADD COLUMN last_active_at INTEGER")
            .execute(pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("worker_presence column: {e}")))?;
    }
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_worker_presence_tenant_last_active
         ON worker(tenant_id, last_active_at DESC)",
    )
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker_presence index: {e}")))?;
    Ok(())
}

/// Mark one worker active at the current time. Unknown workers are
/// a no-op so audit/session writers can call this unconditionally
/// without coupling to identity-store reads.
pub async fn touch_presence(pool: &SqlitePool, worker_id: &str) -> ArgoResult<()> {
    ensure_schema(pool).await?;
    sqlx::query("UPDATE worker SET last_active_at = strftime('%s','now') WHERE id = ?")
        .bind(worker_id)
        .execute(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("worker_presence touch: {e}")))?;
    Ok(())
}

/// List live workers ordered by most recent activity. NULL
/// `last_active_at` rows sort last (per acceptance criterion).
pub async fn fleet(
    pool: &SqlitePool,
    tenant_id: Option<&str>,
    limit: u32,
    since_epoch_secs: Option<i64>,
) -> ArgoResult<Vec<FleetRow>> {
    fleet_at(pool, tenant_id, limit, since_epoch_secs, Utc::now().timestamp()).await
}

async fn fleet_at(
    pool: &SqlitePool,
    tenant_id: Option<&str>,
    limit: u32,
    since_epoch_secs: Option<i64>,
    now_epoch_secs: i64,
) -> ArgoResult<Vec<FleetRow>> {
    ensure_schema(pool).await?;
    if !table_exists(pool, "worker").await? {
        return Ok(Vec::new());
    }

    let mut sql = String::from(
        "SELECT w.id, w.tenant_id, w.vertical_id, p.name AS persona, w.last_active_at \
         FROM worker w \
         LEFT JOIN persona p ON p.id = w.persona_id",
    );
    let mut filters = Vec::new();
    if tenant_id.is_some() {
        filters.push("w.tenant_id = ?");
    }
    if since_epoch_secs.is_some() {
        filters.push("w.last_active_at IS NOT NULL AND w.last_active_at >= ?");
    }
    if !filters.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&filters.join(" AND "));
    }
    sql.push_str(" ORDER BY w.last_active_at IS NULL ASC, w.last_active_at DESC, w.id ASC LIMIT ?");

    let mut query = sqlx::query(&sql);
    if let Some(tenant) = tenant_id {
        query = query.bind(tenant);
    }
    if let Some(since) = since_epoch_secs {
        query = query.bind(since);
    }
    let rows = query
        .bind(i64::from(limit.clamp(1, 500)))
        .fetch_all(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("worker_presence fleet: {e}")))?;

    rows.into_iter()
        .map(|row| {
            let last_active_at: Option<i64> = row.try_get("last_active_at").map_err(row_err)?;
            Ok(FleetRow {
                worker_id: row.try_get("id").map_err(row_err)?,
                tenant_id: row.try_get("tenant_id").map_err(row_err)?,
                vertical_id: row.try_get("vertical_id").map_err(row_err)?,
                persona: row.try_get("persona").map_err(row_err)?,
                last_active_at,
                last_active_at_rfc3339: last_active_at.and_then(format_epoch),
                status: status(last_active_at, now_epoch_secs).to_string(),
            })
        })
        .collect()
}

async fn table_exists(pool: &SqlitePool, table: &str) -> ArgoResult<bool> {
    let found: Option<i64> =
        sqlx::query_scalar("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?")
            .bind(table)
            .fetch_optional(pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("worker_presence table inspect: {e}")))?;
    Ok(found.is_some())
}

async fn column_exists(pool: &SqlitePool, table: &str, column: &str) -> ArgoResult<bool> {
    let rows = sqlx::query(&format!("PRAGMA table_info({table})"))
        .fetch_all(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("worker_presence column inspect: {e}")))?;
    for row in rows {
        let name: String = row
            .try_get("name")
            .map_err(|e| ArgoError::Provider(format!("worker_presence column name: {e}")))?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn status(last_active_at: Option<i64>, now_epoch_secs: i64) -> &'static str {
    match last_active_at {
        Some(last) if now_epoch_secs.saturating_sub(last) <= IDLE_THRESHOLD_SECS => "active",
        _ => "idle",
    }
}

fn format_epoch(value: i64) -> Option<String> {
    DateTime::<Utc>::from_timestamp(value, 0).map(|dt| dt.to_rfc3339())
}

fn row_err(err: sqlx::Error) -> ArgoError {
    ArgoError::Provider(format!("worker_presence row: {err}"))
}

#[cfg(test)]
#[path = "worker_presence_tests.rs"]
mod tests;
