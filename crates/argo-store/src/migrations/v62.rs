//! Schema migration v62 — approval no-reply reminder stamp.
//!
//! Adds `approval_request.escalation_sent_at` for Stream
//! `mission-escalation-on-no-reply`. The approval expiry sweeper owns
//! terminal timeout decisions; this column lets the reminder loop send
//! exactly one pre-expiry escalation and retry later when dispatch
//! fails.
//!
//! Slot rationale: v61 is the current versioned tail on main. Raw SQL
//! filename slot 71 is pinned to `mission-approval-bundle-ui`, slot 72
//! is `retro`, and slots 63-70 are reserved/occupied, so the raw
//! counterpart uses slot 73. Filename and version counters advance
//! independently per the v55 convention.
use argo_types::ArgoError;
use sqlx::{Row, Sqlite};

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    if !column_exists(tx, "approval_request", "escalation_sent_at").await? {
        exec(
            tx,
            "v62 up",
            "ALTER TABLE approval_request ADD COLUMN escalation_sent_at INTEGER NULL",
        )
        .await?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    if column_exists(tx, "approval_request", "escalation_sent_at").await? {
        exec(
            tx,
            "v62 down",
            "ALTER TABLE approval_request DROP COLUMN escalation_sent_at",
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
    let rows = sqlx::query(&format!("PRAGMA table_info({table})"))
        .fetch_all(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("v62 column probe {table}.{column}: {e}")))?;
    Ok(rows.iter().any(|row| {
        let name: String = row.get("name");
        name == column
    }))
}
