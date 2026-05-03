//! Schema migration v7 — `session_grant` table for per-session
//! human-approval grants (#664). The vertical-template
//! escalation gate (#660) blocks tools listed in the manifest's
//! `[escalation].human_approval_required`; this table is how
//! the operator approves them.
//!
//! Wholly additive: no existing tables are touched. Every
//! existing session keeps working unchanged — sessions with
//! zero grants behave exactly as they did before.
//!
//! ## Schema
//!
//! - `id BLOB PRIMARY KEY` — UUID v7 of the grant row
//! - `session_id BLOB NOT NULL REFERENCES session(id)`
//! - `tool_name TEXT NOT NULL` — exact match against the
//!   `[escalation].human_approval_required` list and against
//!   `ToolCallRequest.name`. (No glob patterns yet — keep the
//!   security boundary tight; future PR can opt-in glob.)
//! - `granted_at INTEGER NOT NULL` — Unix epoch seconds
//! - `source TEXT NOT NULL DEFAULT 'cli'` — provenance hint;
//!   today either `'cli'` or `'gateway'`
//! - `notes TEXT` — operator-supplied free-form context (e.g.
//!   "approved by mark via Slack")
//!
//! Down migration drops the table.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V7_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v7 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V7_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v7 down: {e}")))?;
    }
    Ok(())
}

const V7_UP: &[&str] = &[
    "CREATE TABLE session_grant (
        id          BLOB PRIMARY KEY,
        session_id  BLOB NOT NULL REFERENCES session(id),
        tool_name   TEXT NOT NULL,
        granted_at  INTEGER NOT NULL,
        source      TEXT NOT NULL DEFAULT 'cli',
        notes       TEXT
    )",
    // Per-session grant lookup is the hot path the agent
    // runner walks at every TurnStart — index `session_id`
    // so the load stays O(1) under load.
    "CREATE INDEX idx_session_grant_session_id ON session_grant(session_id)",
];

const V7_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_session_grant_session_id",
    "DROP TABLE IF EXISTS session_grant",
];
