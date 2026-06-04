//! Schema migration v88 — the `waitlist_signups` table backing the public
//! early-access waitlist (`POST /api/v1/waitlist`, WS6).
//!
//! The landing page's waitlist form posts an email (+ optional name/company/
//! source) to capture organic top-of-funnel signups. This is a
//! **platform-level** surface — it sits in the same un-gated, pre-tenant lane
//! as magic-link signup, so the row carries NO `tenant_id` (mirrors how
//! `magic_link` persists a pre-tenant email):
//!
//! ```text
//! waitlist_signups(
//!   id         TEXT PRIMARY KEY,   -- uuid v4, minted by the handler
//!   email      TEXT NOT NULL UNIQUE, -- lower-cased + trimmed dedupe key
//!   name       TEXT,               -- optional
//!   company    TEXT,               -- optional
//!   source     TEXT,               -- e.g. "landing-hero", "landing-final"
//!   created_at INTEGER NOT NULL    -- unix seconds
//! )
//! ```
//!
//! The `UNIQUE(email)` constraint makes a repeat signup idempotent: the
//! handler uses `INSERT ... ON CONFLICT(email) DO NOTHING` so a second
//! submission is a no-op `202`, never a 500.
//!
//! Additive + idempotent: `CREATE TABLE IF NOT EXISTS` only, no existing
//! column touched, so the migration is safe to re-run and safe to roll
//! forward over any prior schema version. Inline SQL mirrors the v85/v86/v87
//! pattern (no `migrations/NNN__*.sql` slot file, so no filesystem slot to
//! collide with — the `duplicate_slot_validator` that runs at boot stays
//! happy).

use argo_types::ArgoError;
use sqlx::Sqlite;

const V88_UP_TABLE_SQL: &str = "CREATE TABLE IF NOT EXISTS waitlist_signups (\
    id         TEXT PRIMARY KEY,\
    email      TEXT NOT NULL UNIQUE,\
    name       TEXT,\
    company    TEXT,\
    source     TEXT,\
    created_at INTEGER NOT NULL\
)";

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    sqlx::query(V88_UP_TABLE_SQL)
        .execute(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("v88 up table: {e}")))?;
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    sqlx::query("DROP TABLE IF EXISTS waitlist_signups")
        .execute(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("v88 down: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn up_creates_waitlist_signups_table() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("pool");
        super::super::run_up_to(&pool, 88).await.expect("migrations");

        sqlx::query(
            "INSERT INTO waitlist_signups (id, email, name, company, source, created_at) \
             VALUES ('w1', 'alice@example.com', 'Alice', 'Acme', 'landing-hero', 1717200000)",
        )
        .execute(&pool)
        .await
        .expect("insert into waitlist_signups");

        let email: String =
            sqlx::query_scalar("SELECT email FROM waitlist_signups WHERE id = 'w1'")
                .fetch_one(&pool)
                .await
                .expect("read back row");
        assert_eq!(email, "alice@example.com");
    }

    #[tokio::test]
    async fn email_is_unique_dedupe_key() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("pool");
        super::super::run_up_to(&pool, 88).await.expect("migrations");

        sqlx::query(
            "INSERT INTO waitlist_signups (id, email, created_at) \
             VALUES ('w1', 'dup@example.com', 1717200000)",
        )
        .execute(&pool)
        .await
        .expect("first insert");

        // ON CONFLICT(email) DO NOTHING — the dedupe path the handler relies on.
        let res = sqlx::query(
            "INSERT INTO waitlist_signups (id, email, created_at) \
             VALUES ('w2', 'dup@example.com', 1717200001) \
             ON CONFLICT(email) DO NOTHING",
        )
        .execute(&pool)
        .await
        .expect("conflicting insert is a no-op");
        assert_eq!(res.rows_affected(), 0, "conflict must insert no row");

        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM waitlist_signups WHERE email = 'dup@example.com'")
                .fetch_one(&pool)
                .await
                .expect("count rows");
        assert_eq!(count, 1, "dedupe must leave exactly one row");
    }

    #[tokio::test]
    async fn optional_columns_are_nullable() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("pool");
        super::super::run_up_to(&pool, 88).await.expect("migrations");

        // name / company / source omitted — must be accepted (columns nullable).
        sqlx::query(
            "INSERT INTO waitlist_signups (id, email, created_at) \
             VALUES ('w3', 'bare@example.com', 1717200000)",
        )
        .execute(&pool)
        .await
        .expect("insert without optional columns");

        let name: Option<String> =
            sqlx::query_scalar("SELECT name FROM waitlist_signups WHERE id = 'w3'")
                .fetch_one(&pool)
                .await
                .expect("read back row");
        assert!(name.is_none(), "name should default to NULL");
    }

    #[tokio::test]
    async fn down_drops_the_table() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("pool");
        super::super::run_up_to(&pool, 88).await.expect("up to v88");
        super::super::roll_back_to(&pool, 87).await.expect("roll back to v87");

        let exists: Option<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'table' \
             AND name = 'waitlist_signups'",
        )
        .fetch_optional(&pool)
        .await
        .expect("probe sqlite_master");
        assert!(exists.is_none(), "table should be dropped by v88 down");
    }
}
