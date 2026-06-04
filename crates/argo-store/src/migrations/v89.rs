//! Schema migration v89 — the `enterprise_inquiry` table backing the public
//! enterprise Expression-of-Interest capture (`POST /api/v1/enterprise-inquiry`,
//! WS6-2).
//!
//! The `/pricing`, `/contact` (and future enterprise) surfaces replace their
//! `mailto:` "Talk to sales" CTA with a real inquiry form. The handler posts a
//! structured lead (company + contact name + work email, plus optional seats +
//! message) so the EOI→MOU→contract motion has a persisted, auditable front
//! door instead of an un-tracked email. Like the waitlist (v88), this is a
//! **platform-level** surface — it sits in the same un-gated, pre-tenant lane
//! as magic-link signup, so the row carries NO `tenant_id`:
//!
//! ```text
//! enterprise_inquiry(
//!   id         TEXT PRIMARY KEY,   -- uuid v7, minted by the handler
//!   company    TEXT NOT NULL,      -- required
//!   name       TEXT NOT NULL,      -- required contact name
//!   email      TEXT NOT NULL,      -- required (lower-cased + trimmed)
//!   seats      TEXT,               -- optional free-text seat estimate
//!   message    TEXT,               -- optional context
//!   created_at INTEGER NOT NULL    -- unix seconds
//! )
//! ```
//!
//! **Append-only (no dedupe).** Unlike the waitlist there is NO `UNIQUE(email)`
//! constraint: a legitimate enterprise lead may inquire more than once (a
//! second contact, a follow-up with new context), and every inquiry is a
//! distinct sales-relevant record we don't want to silently swallow. The
//! handler therefore plain-`INSERT`s and returns `202` on success — there is no
//! conflict path to handle.
//!
//! Additive + idempotent: `CREATE TABLE IF NOT EXISTS` only, no existing
//! column touched, so the migration is safe to re-run and safe to roll forward
//! over any prior schema version. Inline SQL mirrors the v88 pattern (no
//! `migrations/NNN__*.sql` slot file, so no filesystem slot to collide with —
//! the `duplicate_slot_validator` that runs at boot stays happy).

use argo_types::ArgoError;
use sqlx::Sqlite;

const V89_UP_TABLE_SQL: &str = "CREATE TABLE IF NOT EXISTS enterprise_inquiry (\
    id         TEXT PRIMARY KEY,\
    company    TEXT NOT NULL,\
    name       TEXT NOT NULL,\
    email      TEXT NOT NULL,\
    seats      TEXT,\
    message    TEXT,\
    created_at INTEGER NOT NULL\
)";

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    sqlx::query(V89_UP_TABLE_SQL)
        .execute(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("v89 up table: {e}")))?;
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    sqlx::query("DROP TABLE IF EXISTS enterprise_inquiry")
        .execute(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("v89 down: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn up_creates_enterprise_inquiry_table() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("pool");
        super::super::run_up_to(&pool, 89).await.expect("migrations");

        sqlx::query(
            "INSERT INTO enterprise_inquiry (id, company, name, email, seats, message, created_at) \
             VALUES ('e1', 'Acme', 'Alice', 'alice@acme.com', '250', 'rollout in Q3', 1717200000)",
        )
        .execute(&pool)
        .await
        .expect("insert into enterprise_inquiry");

        let company: String =
            sqlx::query_scalar("SELECT company FROM enterprise_inquiry WHERE id = 'e1'")
                .fetch_one(&pool)
                .await
                .expect("read back row");
        assert_eq!(company, "Acme");
    }

    #[tokio::test]
    async fn append_only_keeps_repeat_inquiries() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("pool");
        super::super::run_up_to(&pool, 89).await.expect("migrations");

        // Same company+email twice — append-only, so BOTH rows persist (there
        // is no UNIQUE constraint to swallow the second lead).
        for id in ["e1", "e2"] {
            sqlx::query(
                "INSERT INTO enterprise_inquiry (id, company, name, email, created_at) \
                 VALUES (?, 'Acme', 'Alice', 'alice@acme.com', 1717200000)",
            )
            .bind(id)
            .execute(&pool)
            .await
            .expect("insert");
        }

        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM enterprise_inquiry WHERE email = 'alice@acme.com'",
        )
        .fetch_one(&pool)
        .await
        .expect("count rows");
        assert_eq!(count, 2, "append-only must keep both inquiries");
    }

    #[tokio::test]
    async fn optional_columns_are_nullable() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("pool");
        super::super::run_up_to(&pool, 89).await.expect("migrations");

        // seats / message omitted — must be accepted (columns nullable).
        sqlx::query(
            "INSERT INTO enterprise_inquiry (id, company, name, email, created_at) \
             VALUES ('e3', 'BareCo', 'Bob', 'bob@bareco.com', 1717200000)",
        )
        .execute(&pool)
        .await
        .expect("insert without optional columns");

        let seats: Option<String> =
            sqlx::query_scalar("SELECT seats FROM enterprise_inquiry WHERE id = 'e3'")
                .fetch_one(&pool)
                .await
                .expect("read back row");
        assert!(seats.is_none(), "seats should default to NULL");
    }

    #[tokio::test]
    async fn down_drops_the_table() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("pool");
        super::super::run_up_to(&pool, 89).await.expect("up to v89");
        super::super::roll_back_to(&pool, 88).await.expect("roll back to v88");

        let exists: Option<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'table' \
             AND name = 'enterprise_inquiry'",
        )
        .fetch_optional(&pool)
        .await
        .expect("probe sqlite_master");
        assert!(exists.is_none(), "table should be dropped by v89 down");
    }
}
