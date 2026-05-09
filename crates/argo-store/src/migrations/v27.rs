//! Schema migration v27 — operator-feedback channel.
//!
//! Reconciles raw SQL migration `24__operator_feedback.sql`
//! (Stream `claude-operator-feedback-channel`). Single table
//! `operator_feedback` carrying up/down votes per action; the
//! skill-learning loop reads the most-recent vote when weighting
//! candidate emission.
//!
//! 24-collision split: this slot pairs with v28 and v29 below.
//! The brief's recommended assignment (operator_feedback → v27,
//! skill_quality_score → v28, vertical_archived → v29) does not
//! match git-log oldest-first ordering — git shows
//! vertical_archived (PR #884, 08:20) before skill_quality_score
//! (PR #886, 08:36) before operator_feedback (PR #889, 09:14).
//! We follow the brief's published-contract ordering anyway,
//! consistent with the v19/v23 collision-split rationale.
//!
//! Idempotent: CREATE TABLE/INDEX IF NOT EXISTS. Rollback drops
//! the table and all three indexes.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V27_UP {
        exec(tx, "v27 up", sql).await?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V27_DOWN {
        exec(tx, "v27 down", sql).await?;
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

const V27_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS operator_feedback (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        action_id TEXT NOT NULL,
        vote TEXT NOT NULL,
        comment TEXT NOT NULL DEFAULT '',
        given_at INTEGER NOT NULL,
        worker_id TEXT
    )",
    "CREATE INDEX IF NOT EXISTS idx_operator_feedback_action
     ON operator_feedback(action_id, given_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_operator_feedback_worker
     ON operator_feedback(worker_id, given_at DESC)
     WHERE worker_id IS NOT NULL",
    "CREATE INDEX IF NOT EXISTS idx_operator_feedback_vote
     ON operator_feedback(vote, given_at DESC)",
];

const V27_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_operator_feedback_vote",
    "DROP INDEX IF EXISTS idx_operator_feedback_worker",
    "DROP INDEX IF EXISTS idx_operator_feedback_action",
    "DROP TABLE IF EXISTS operator_feedback",
];
