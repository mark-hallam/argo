//! Schema migration v70 — inbound support tickets + reply thread
//! (Stream `claude-support-inbox-spa-page`).
//!
//! Source-of-truth storage for tickets created via
//! `POST /api/support/ticket` (Next.js route added in #1762) plus the
//! operator-side reply thread surfaced through `/api/v1/support/...`.
//! `tenant_id` is nullable because the marketing-site contact form is
//! unauthenticated; the triage UI assigns one later.

use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V70_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v70 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V70_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v70 down: {e}")))?;
    }
    Ok(())
}

const V70_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS support_ticket (
        id              BLOB PRIMARY KEY,
        tenant_id       BLOB,
        user_email      TEXT NOT NULL,
        subject         TEXT NOT NULL,
        body            TEXT NOT NULL,
        status          TEXT NOT NULL DEFAULT 'open',
        priority        TEXT NOT NULL DEFAULT 'normal',
        created_at      INTEGER NOT NULL,
        updated_at      INTEGER NOT NULL,
        assigned_to     BLOB
    )",
    "CREATE INDEX IF NOT EXISTS idx_support_ticket_status \
     ON support_ticket(status)",
    "CREATE INDEX IF NOT EXISTS idx_support_ticket_tenant \
     ON support_ticket(tenant_id)",
    "CREATE TABLE IF NOT EXISTS support_ticket_reply (
        id              BLOB PRIMARY KEY,
        ticket_id       BLOB NOT NULL,
        author_kind     TEXT NOT NULL,
        author_id       BLOB,
        body            TEXT NOT NULL,
        created_at      INTEGER NOT NULL,
        FOREIGN KEY (ticket_id) REFERENCES support_ticket(id)
    )",
    "CREATE INDEX IF NOT EXISTS idx_support_ticket_reply_ticket \
     ON support_ticket_reply(ticket_id)",
];

const V70_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_support_ticket_reply_ticket",
    "DROP TABLE IF EXISTS support_ticket_reply",
    "DROP INDEX IF EXISTS idx_support_ticket_tenant",
    "DROP INDEX IF EXISTS idx_support_ticket_status",
    "DROP TABLE IF EXISTS support_ticket",
];
