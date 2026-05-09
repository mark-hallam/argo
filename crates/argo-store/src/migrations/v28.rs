//! Schema migration v28 — per-skill quality score history.
//!
//! Reconciles raw SQL migration `24__skill_quality_score.sql`
//! (Stream `claude-skill-quality-judge`). Single table
//! `skill_quality_score` with one row per (skill_id,
//! computed_at). Operators read the most recent row for the
//! current 0..100 score; older rows back trends and regressions.
//!
//! 24-collision split: this slot pairs with v27 above and v29
//! below (see v27 doc comment for ordering rationale).
//!
//! Idempotent: CREATE TABLE/INDEX IF NOT EXISTS. Rollback drops
//! the table and both of its indexes.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V28_UP {
        exec(tx, "v28 up", sql).await?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V28_DOWN {
        exec(tx, "v28 down", sql).await?;
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

const V28_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS skill_quality_score (
        skill_id        TEXT NOT NULL,
        score           INTEGER NOT NULL,
        inputs_json     TEXT NOT NULL,
        computed_at     INTEGER NOT NULL,
        PRIMARY KEY (skill_id, computed_at)
    )",
    "CREATE INDEX IF NOT EXISTS idx_skill_quality_skill
     ON skill_quality_score(skill_id)",
    "CREATE INDEX IF NOT EXISTS idx_skill_quality_computed
     ON skill_quality_score(computed_at DESC)",
];

const V28_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_skill_quality_computed",
    "DROP INDEX IF EXISTS idx_skill_quality_skill",
    "DROP TABLE IF EXISTS skill_quality_score",
];
