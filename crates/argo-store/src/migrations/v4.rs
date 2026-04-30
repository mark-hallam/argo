//! Schema migration v4 — `session_skill` association table for
//! per-session skill curation. Lets a session opt into a
//! curated subset of skills instead of always seeing every
//! active row in the agent runner's system prompt.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V4_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v4 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V4_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v4 down: {e}")))?;
    }
    Ok(())
}

const V4_UP: &[&str] = &[
    "CREATE TABLE session_skill (
        session_id  BLOB NOT NULL REFERENCES session(id) ON DELETE CASCADE,
        skill_id    BLOB NOT NULL REFERENCES skill(id) ON DELETE CASCADE,
        attached_at INTEGER NOT NULL,
        PRIMARY KEY (session_id, skill_id)
    )",
    "CREATE INDEX idx_session_skill_session ON session_skill(session_id)",
];

const V4_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_session_skill_session",
    "DROP TABLE IF EXISTS session_skill",
];
