//! Schema migration v23 — worker rosters.
//!
//! Reconciles raw SQL migration `22__roster.sql` (Stream
//! `claude-worker-roster`). Two tables — `roster` carrying the
//! shared policy/channels/budget cap, and `roster_member` joining
//! workers to rosters (M:N).
//!
//! 22-collision split: this slot pairs with v24 below. The raw
//! `migrations/` directory ships two files prefixed `22__`:
//! `22__roster.sql` and `22__tenant_archived.sql`. The brief's
//! recommended assignment maps roster → v23 and tenant_archived →
//! v24. `git log --diff-filter=A` shows `22__tenant_archived.sql`
//! (PR #877, 07:08) was authored ~26 minutes BEFORE
//! `22__roster.sql` (PR #880, 07:34); pure git-log creation order
//! would assign tenant_archived first. We follow the brief's
//! mapping anyway because it is the published contract that the
//! v28-37 sub-brief and downstream legacy-marker bookkeeping
//! build on.
//!
//! Idempotent: CREATE TABLE/INDEX IF NOT EXISTS. Rollback drops
//! both tables and their indexes.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V23_UP {
        exec(tx, "v23 up", sql).await?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V23_DOWN {
        exec(tx, "v23 down", sql).await?;
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

const V23_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS roster (
        id                      TEXT PRIMARY KEY,
        tenant_id               TEXT NOT NULL,
        name                    TEXT NOT NULL,
        policy_json             TEXT NOT NULL,
        channels_json           TEXT NOT NULL,
        budget_cap_usd_micros   INTEGER,
        created_at              INTEGER NOT NULL,
        UNIQUE(tenant_id, name)
    )",
    "CREATE INDEX IF NOT EXISTS idx_roster_tenant ON roster(tenant_id)",
    "CREATE TABLE IF NOT EXISTS roster_member (
        roster_id   TEXT NOT NULL,
        worker_id   TEXT NOT NULL,
        added_at    INTEGER NOT NULL,
        PRIMARY KEY (roster_id, worker_id)
    )",
    "CREATE INDEX IF NOT EXISTS idx_roster_member_worker ON roster_member(worker_id)",
];

const V23_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_roster_member_worker",
    "DROP TABLE IF EXISTS roster_member",
    "DROP INDEX IF EXISTS idx_roster_tenant",
    "DROP TABLE IF EXISTS roster",
];
