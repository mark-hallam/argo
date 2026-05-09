//! Schema migration v31 — typed data classification tags.
//!
//! Reconciles raw SQL migration `26__data_classification.sql`
//! (Stream `claude-data-classification`). Single table
//! `data_classification_tag` carrying PII / PHI / financial /
//! secret / public / untagged tags over memory rows, audit
//! payloads, and worker actions. One row per (record_kind,
//! record_id, class).
//!
//! 26-collision split: this slot pairs with v32 and v33 below.
//! The brief's recommended assignment maps data_classification →
//! v31, spend_baseline → v32, worker_budget_alert → v33. Git-log
//! shows spend_baseline (PR #892, 09:31) was authored before
//! data_classification (PR #893, 09:59) before
//! worker_budget_alert (PR #902, 11:48); pure git-log oldest-
//! first would assign spend_baseline → v31 instead. We follow
//! the brief's published-contract ordering, consistent with the
//! v19/v23/v27 collision-split rationale.
//!
//! Idempotent: CREATE TABLE/INDEX IF NOT EXISTS. Rollback drops
//! the table and all three indexes.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V31_UP {
        exec(tx, "v31 up", sql).await?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V31_DOWN {
        exec(tx, "v31 down", sql).await?;
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

const V31_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS data_classification_tag (
        record_kind TEXT NOT NULL,
        record_id TEXT NOT NULL,
        class TEXT NOT NULL,
        tagged_at_micros INTEGER NOT NULL,
        tagged_by TEXT,
        PRIMARY KEY (record_kind, record_id, class)
    )",
    "CREATE INDEX IF NOT EXISTS idx_dc_tag_class
     ON data_classification_tag(class)",
    "CREATE INDEX IF NOT EXISTS idx_dc_tag_record
     ON data_classification_tag(record_kind, record_id)",
    "CREATE INDEX IF NOT EXISTS idx_dc_tag_tagged_at
     ON data_classification_tag(tagged_at_micros)",
];

const V31_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_dc_tag_tagged_at",
    "DROP INDEX IF EXISTS idx_dc_tag_record",
    "DROP INDEX IF EXISTS idx_dc_tag_class",
    "DROP TABLE IF EXISTS data_classification_tag",
];
