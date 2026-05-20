//! System-level workspace notifications backing the bell-popover API
//! (stream `web-gateway-endpoints-notifications-batch`).
//!
//! Notifications are tenant-scoped events addressed at a specific
//! human/worker/system principal (or broadcast `NULL` principal). The
//! gateway list, dismiss, dismiss-all, and unread-count endpoints all
//! read/write through this module; dismissals stamp `dismissed_at`
//! rather than deleting rows so the audit chain stays intact.
//!
//! Schema is owned by `migrations/125__notifications.sql`; the runtime
//! [`ensure_schema`] helper applies it idempotently, mirroring the
//! same lazy-bootstrap pattern as `magic_link_tokens` and
//! `data_export_job` so crate-scoped tests don't need the full
//! versioned runner.
//!
//! ```no_run
//! # async fn demo() -> argo_types::ArgoResult<()> {
//! use argo_store::notifications::{insert, list_for_principal, ListQuery, NotificationInsert};
//! use sqlx::sqlite::SqlitePoolOptions;
//! let pool = SqlitePoolOptions::new().connect("sqlite::memory:").await.unwrap();
//! insert(&pool, &NotificationInsert {
//!     id: "nt_demo",
//!     tenant_id: "t1",
//!     principal_id: Some("human:demo@example.com"),
//!     kind: "duty-output-ready",
//!     body: "Daily revenue digest is ready.",
//!     created_at: 1_700_000_000,
//!     related_audit_id: None,
//!     severity: "info",
//! }).await?;
//! let _ = list_for_principal(&pool, "t1", Some("human:demo@example.com"), &ListQuery::default()).await?;
//! # Ok(()) }
//! ```

use argo_types::{ArgoError, ArgoResult};
use serde_json::{json, Value};
use sqlx::SqlitePool;

/// Default page size for `GET /api/v1/notifications`. Matches the
/// bell-popover's visible window (50 items above the fold) plus a
/// small safety margin.
pub const DEFAULT_LIMIT: u32 = 50;

/// Hard cap on `?limit=` to prevent unbounded scans; if a caller asks
/// for more, the handler clamps and the response still pages.
pub const MAX_LIMIT: u32 = 200;

const ENSURE_SQL: &str = include_str!("../migrations/125__notifications.sql");

/// Insert payload — every column written when a notification is first
/// emitted. The `id` is caller-minted (uuid-v7 stringified) so tests
/// can pin stable values.
#[derive(Debug, Clone)]
pub struct NotificationInsert<'a> {
    /// Caller-issued id (e.g. `nt_<uuid-v7>`).
    pub id: &'a str,
    /// Tenant scope; required even for broadcast rows.
    pub tenant_id: &'a str,
    /// `None` for broadcast-to-tenant rows; otherwise the recipient
    /// (`human:<email>`, `worker:<uuid>`, `system:<name>`).
    pub principal_id: Option<&'a str>,
    /// Short stable category label (e.g. `budget-approaching`).
    pub kind: &'a str,
    /// Plain-text body the popover renders as the headline.
    pub body: &'a str,
    /// Unix-seconds emit timestamp.
    pub created_at: i64,
    /// Optional cross-link into the audit chain.
    pub related_audit_id: Option<&'a str>,
    /// One of `info`, `warning`, `critical`. Used by the popover for
    /// styling — defaults to `info` at the DB level if omitted.
    pub severity: &'a str,
}

/// One row of `notifications` shaped for the bell-popover wire form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationRow {
    /// Caller-minted id.
    pub id: String,
    /// Tenant scope of the row.
    pub tenant_id: String,
    /// `Some` for addressed rows, `None` for broadcast.
    pub principal_id: Option<String>,
    /// Category label.
    pub kind: String,
    /// Wire body text.
    pub body: String,
    /// Emit timestamp.
    pub created_at: i64,
    /// `Some` once a viewer has marked the row read.
    pub read_at: Option<i64>,
    /// `Some` once a viewer has dismissed the row. Dismissed rows are
    /// excluded from list/unread-count by default.
    pub dismissed_at: Option<i64>,
    /// Optional cross-link into the audit chain.
    pub related_audit_id: Option<String>,
    /// One of `info`, `warning`, `critical`.
    pub severity: String,
}

impl NotificationRow {
    /// JSON wire form used by `GET /api/v1/notifications` items.
    /// Matches the web `NotificationItem` shape (camelCase keys, `read`
    /// boolean rather than the raw `read_at` timestamp).
    #[must_use]
    pub fn to_wire_json(&self) -> Value {
        json!({
            "id": self.id,
            "kind": self.kind,
            "title": self.body,
            "createdAt": iso8601_seconds(self.created_at),
            "read": self.read_at.is_some(),
            "severity": self.severity,
            "relatedAuditId": self.related_audit_id,
        })
    }
}

/// Pagination + filter input for [`list_for_principal`].
#[derive(Debug, Clone, Default)]
pub struct ListQuery {
    /// 1..=`MAX_LIMIT`; clamped at the caller boundary.
    pub limit: Option<u32>,
    /// `created_at` upper bound for descending pagination (exclusive).
    pub before: Option<i64>,
    /// When `true`, dismissed rows are excluded (default behaviour).
    pub exclude_dismissed: bool,
}

/// Result envelope for [`list_for_principal`]. The handler maps this
/// 1:1 into the JSON response body so the cursor + has_more contract
/// stays consistent across all callers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListPage {
    /// Page rows newest-first.
    pub items: Vec<NotificationRow>,
    /// `true` when at least one more row exists at `created_at < cursor`.
    pub has_more: bool,
    /// Pass this back as `?before=` to fetch the next page.
    pub cursor: Option<i64>,
}

/// Ensure the `notifications` table + supporting indices exist.
/// Idempotent; safe to call from every entrypoint.
pub async fn ensure_schema(pool: &SqlitePool) -> ArgoResult<()> {
    let stripped: String = ENSURE_SQL
        .lines()
        .filter(|line| !line.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n");
    for stmt in stripped.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        sqlx::query(stmt)
            .execute(pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("notifications ensure: {e}")))?;
    }
    Ok(())
}

/// Insert one notification. Use `INSERT OR IGNORE` so retries on the
/// same caller-minted id are idempotent (the emitter is responsible
/// for stable ids when it wants dedup, otherwise mint a fresh uuid).
pub async fn insert(pool: &SqlitePool, ins: &NotificationInsert<'_>) -> ArgoResult<()> {
    ensure_schema(pool).await?;
    sqlx::query(
        "INSERT OR IGNORE INTO notifications \
         (id, tenant_id, principal_id, kind, body, created_at, related_audit_id, severity) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(ins.id)
    .bind(ins.tenant_id)
    .bind(ins.principal_id)
    .bind(ins.kind)
    .bind(ins.body)
    .bind(ins.created_at)
    .bind(ins.related_audit_id)
    .bind(ins.severity)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("notifications insert: {e}")))?;
    Ok(())
}

/// List notifications visible to `principal_id` within `tenant_id`.
///
/// Visibility = the row's `principal_id` matches OR is `NULL`
/// (broadcast). Dismissed rows are excluded by default. Rows are
/// returned newest-first; pagination uses `?before=<created_at>`.
pub async fn list_for_principal(
    pool: &SqlitePool,
    tenant_id: &str,
    principal_id: Option<&str>,
    query: &ListQuery,
) -> ArgoResult<ListPage> {
    ensure_schema(pool).await?;
    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let before = query.before.unwrap_or(i64::MAX);
    // Pull `limit + 1` to detect `has_more` without a second round trip.
    let probe_limit = i64::from(limit) + 1;
    let rows: Vec<RawRow> = sqlx::query_as(
        "SELECT id, tenant_id, principal_id, kind, body, created_at, \
         read_at, dismissed_at, related_audit_id, severity \
         FROM notifications \
         WHERE tenant_id = ? \
           AND (principal_id IS NULL OR principal_id = ?) \
           AND (? = 0 OR dismissed_at IS NULL) \
           AND created_at < ? \
         ORDER BY created_at DESC \
         LIMIT ?",
    )
    .bind(tenant_id)
    .bind(principal_id)
    .bind(i64::from(query.exclude_dismissed))
    .bind(before)
    .bind(probe_limit)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("notifications list: {e}")))?;

    let has_more = rows.len() as i64 > i64::from(limit);
    let visible: Vec<NotificationRow> = rows
        .into_iter()
        .take(limit as usize)
        .map(decode_row)
        .collect();
    let cursor = if has_more {
        visible.last().map(|r| r.created_at)
    } else {
        None
    };
    Ok(ListPage {
        items: visible,
        has_more,
        cursor,
    })
}

/// Count unread, non-dismissed notifications visible to `principal_id`.
/// Used by the `/unread-count` endpoint that feeds the bell badge.
pub async fn unread_count(
    pool: &SqlitePool,
    tenant_id: &str,
    principal_id: Option<&str>,
) -> ArgoResult<u32> {
    ensure_schema(pool).await?;
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notifications \
         WHERE tenant_id = ? \
           AND (principal_id IS NULL OR principal_id = ?) \
           AND dismissed_at IS NULL \
           AND read_at IS NULL",
    )
    .bind(tenant_id)
    .bind(principal_id)
    .fetch_one(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("notifications unread-count: {e}")))?;
    Ok(u32::try_from(n).unwrap_or(u32::MAX))
}

/// Stamp `dismissed_at` on a single row. Returns `Ok(true)` when the
/// row exists and is visible to `principal_id`; `Ok(false)` when no
/// row matched (idempotent — repeat calls also return `Ok(false)`
/// after the first).
pub async fn dismiss_one(
    pool: &SqlitePool,
    tenant_id: &str,
    principal_id: Option<&str>,
    id: &str,
    now: i64,
) -> ArgoResult<bool> {
    ensure_schema(pool).await?;
    let result = sqlx::query(
        "UPDATE notifications SET dismissed_at = ? \
         WHERE id = ? \
           AND tenant_id = ? \
           AND (principal_id IS NULL OR principal_id = ?) \
           AND dismissed_at IS NULL",
    )
    .bind(now)
    .bind(id)
    .bind(tenant_id)
    .bind(principal_id)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("notifications dismiss: {e}")))?;
    Ok(result.rows_affected() > 0)
}

/// Stamp `dismissed_at` on every unread row visible to `principal_id`.
/// Returns the number of rows touched.
pub async fn dismiss_all_for_principal(
    pool: &SqlitePool,
    tenant_id: &str,
    principal_id: Option<&str>,
    now: i64,
) -> ArgoResult<u32> {
    ensure_schema(pool).await?;
    let result = sqlx::query(
        "UPDATE notifications SET dismissed_at = ? \
         WHERE tenant_id = ? \
           AND (principal_id IS NULL OR principal_id = ?) \
           AND dismissed_at IS NULL",
    )
    .bind(now)
    .bind(tenant_id)
    .bind(principal_id)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("notifications dismiss-all: {e}")))?;
    Ok(u32::try_from(result.rows_affected()).unwrap_or(u32::MAX))
}

type RawRow = (
    String,
    String,
    Option<String>,
    String,
    String,
    i64,
    Option<i64>,
    Option<i64>,
    Option<String>,
    String,
);

fn decode_row(
    (
        id,
        tenant_id,
        principal_id,
        kind,
        body,
        created_at,
        read_at,
        dismissed_at,
        related_audit_id,
        severity,
    ): RawRow,
) -> NotificationRow {
    NotificationRow {
        id,
        tenant_id,
        principal_id,
        kind,
        body,
        created_at,
        read_at,
        dismissed_at,
        related_audit_id,
        severity,
    }
}

/// Render a Unix-seconds timestamp as a Z-suffixed ISO-8601 string.
/// Falls back to the raw epoch string on the (impossible) overflow
/// path so the wire response never panics.
fn iso8601_seconds(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|dt| dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        .unwrap_or_else(|| ts.to_string())
}

#[cfg(test)]
#[path = "notifications_tests.rs"]
mod tests;
