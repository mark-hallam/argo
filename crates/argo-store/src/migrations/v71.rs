//! Schema migration v71 — approval draft content preview.
//!
//! Adds `approval_request.draft_content` for Stream
//! `mission-pm-approval-content-preview`. Surfaces the actual draft
//! body (PRD text, email body, roadmap delta) alongside the existing
//! intent + consequences so approvers can read what is about to ship
//! before they decide instead of rubberstamping blind.
//!
//! Slot rationale: v70 is the highest registered version on
//! origin/main. Raw SQL slot 116 is the highest committed filename;
//! slot 117 is the next free slot. Filename and version counters
//! advance independently per the v55 convention.
use argo_types::ArgoError;
use sqlx::{Row, Sqlite};

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    if !column_exists(tx, "approval_request", "draft_content").await? {
        exec(
            tx,
            "v71 up",
            "ALTER TABLE approval_request ADD COLUMN draft_content TEXT NULL",
        )
        .await?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    if column_exists(tx, "approval_request", "draft_content").await? {
        exec(
            tx,
            "v71 down",
            "ALTER TABLE approval_request DROP COLUMN draft_content",
        )
        .await?;
    }
    Ok(())
}

async fn exec<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
    ctx: &str,
    sql: &str,
) -> Result<(), ArgoError> {
    sqlx::query(sql)
        .execute(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("{ctx}: {e}")))?;
    Ok(())
}

async fn column_exists<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
    table: &str,
    column: &str,
) -> Result<bool, ArgoError> {
    // audit-allow: identifier substitution from fixed v71 migration probes
    let rows = sqlx::query(&format!("PRAGMA table_info({table})"))
        .fetch_all(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("v71 column probe {table}.{column}: {e}")))?;
    Ok(rows.iter().any(|row| {
        let name: String = row.get("name");
        name == column
    }))
}
