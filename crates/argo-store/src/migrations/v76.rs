//! Schema migration v76 — system-level notifications table
//! (Stream `web-gateway-endpoints-notifications-batch`).
//!
//! The schema body lives in `migrations/125__notifications.sql`; this
//! wrapper applies it via the versioned migration runner so existing
//! workspaces upgrade without invoking the runtime `ensure_schema`
//! fast-path. Mirrors the v51 (`trust_observation`) / v43
//! (`skill_canary`) precedents.
//!
//! Slot rationale: v75 (`session_grant_target`) is the highest
//! versioned slot on origin/main. Raw SQL filename slot 124 was taken
//! by `service_token_binding.sql` (#1865, merged 2026-05-21); this
//! brief takes filename slot 125. The two counters move independently
//! per the v55 docstring.
use argo_types::ArgoError;
use sqlx::Sqlite;

const V76_UP_SQL: &str = include_str!("../../migrations/125__notifications.sql");

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    let stripped: String = V76_UP_SQL
        .lines()
        .filter(|line| !line.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n");
    for stmt in stripped.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        sqlx::query(stmt)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v76 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V76_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v76 down: {e}")))?;
    }
    Ok(())
}

const V76_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_notifications_created_desc",
    "DROP INDEX IF EXISTS idx_notifications_inbox",
    "DROP TABLE IF EXISTS notifications",
];
