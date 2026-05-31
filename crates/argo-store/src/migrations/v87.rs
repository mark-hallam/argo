//! Schema migration v87 — the `worker_conversation_thread` table backing
//! the "Active conversations" panel on the worker Memory tab (Frontier-B
//! depth, worker-memory surfaces).
//!
//! `GET /api/v1/workers/<id>/memory` hard-coded `active_conversations` as
//! an empty `Vec::new()` because no Rust home existed for a worker's open
//! threads. The companion `recent_decisions` panel is *derived* from the
//! existing `audit_record` rows (the approval log IS the decision record),
//! so it needs no table; only "active conversations" needs a durable home:
//!
//! ```text
//! worker_conversation_thread(
//!   id                  TEXT PRIMARY KEY,
//!   tenant_id           TEXT NOT NULL,
//!   worker_id           BLOB NOT NULL,        -- 16-byte WorkerId, matches memory_entry.worker_id
//!   counterparty_handle TEXT NOT NULL,        -- PII-redacted at render
//!   topic               TEXT NOT NULL,
//!   last_message_at     INTEGER NOT NULL,     -- unix micros, newest-first sort key
//!   audit_href          TEXT                  -- optional drill-in target
//! )
//! ```
//!
//! The composite `(tenant_id, worker_id, last_message_at)` index gives the
//! per-worker thread list a tenant-scoped, recency-ordered read path
//! mirroring how `fetch_active_facts` reads `memory_entry`.
//!
//! Additive + idempotent: `CREATE TABLE IF NOT EXISTS` /
//! `CREATE INDEX IF NOT EXISTS` only, no existing column touched, so the
//! migration is safe to re-run and safe to roll forward over any prior
//! schema version. Inline SQL mirrors the v85/v86 pattern (no
//! `migrations/NNN__*.sql` slot file, so no filesystem slot to collide
//! with — the `duplicate_slot_validator` that runs at boot stays happy).

use argo_types::ArgoError;
use sqlx::Sqlite;

const V87_UP_TABLE_SQL: &str = "CREATE TABLE IF NOT EXISTS worker_conversation_thread (\
    id                  TEXT PRIMARY KEY,\
    tenant_id           TEXT NOT NULL,\
    worker_id           BLOB NOT NULL,\
    counterparty_handle TEXT NOT NULL,\
    topic               TEXT NOT NULL,\
    last_message_at     INTEGER NOT NULL,\
    audit_href          TEXT\
)";

const V87_UP_INDEX_SQL: &str = "CREATE INDEX IF NOT EXISTS \
    idx_worker_conversation_thread_tenant_worker_recency \
    ON worker_conversation_thread(tenant_id, worker_id, last_message_at)";

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    sqlx::query(V87_UP_TABLE_SQL)
        .execute(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("v87 up table: {e}")))?;
    sqlx::query(V87_UP_INDEX_SQL)
        .execute(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("v87 up index: {e}")))?;
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    sqlx::query("DROP TABLE IF EXISTS worker_conversation_thread")
        .execute(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("v87 down: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn up_creates_conversation_thread_table_on_memory() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("pool");

        // Run the full versioned runner up to and including v87 so the
        // migration is exercised exactly as the runtime applies it.
        super::super::run_up_to(&pool, 87).await.expect("migrations");

        // The table exists and accepts the documented shape (worker_id as
        // a 16-byte blob, like memory_entry.worker_id).
        let worker_blob: &[u8] = &[1u8; 16];
        sqlx::query(
            "INSERT INTO worker_conversation_thread \
             (id, tenant_id, worker_id, counterparty_handle, topic, last_message_at, audit_href) \
             VALUES ('t1', 'tenant-a', ?, 'Diego Alvarez', 'Interview scheduling', \
                     1717200000000000, '/audit/action/t1')",
        )
        .bind(worker_blob)
        .execute(&pool)
        .await
        .expect("insert into worker_conversation_thread");

        let topic: String = sqlx::query_scalar(
            "SELECT topic FROM worker_conversation_thread WHERE id = 't1'",
        )
        .fetch_one(&pool)
        .await
        .expect("read back row");
        assert_eq!(topic, "Interview scheduling");
    }

    #[tokio::test]
    async fn audit_href_is_nullable() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("pool");
        super::super::run_up_to(&pool, 87).await.expect("migrations");

        let worker_blob: &[u8] = &[2u8; 16];
        // audit_href omitted — must be accepted (column is nullable).
        sqlx::query(
            "INSERT INTO worker_conversation_thread \
             (id, tenant_id, worker_id, counterparty_handle, topic, last_message_at) \
             VALUES ('t2', 't', ?, 'handle', 'topic', 1717200000000000)",
        )
        .bind(worker_blob)
        .execute(&pool)
        .await
        .expect("insert without audit_href");

        let href: Option<String> = sqlx::query_scalar(
            "SELECT audit_href FROM worker_conversation_thread WHERE id = 't2'",
        )
        .fetch_one(&pool)
        .await
        .expect("read back row");
        assert!(href.is_none(), "audit_href should default to NULL");
    }

    #[tokio::test]
    async fn down_drops_the_table() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("pool");
        super::super::run_up_to(&pool, 87).await.expect("up to v87");

        // Roll back below v87 — the table must be gone.
        super::super::roll_back_to(&pool, 86).await.expect("roll back to v86");

        let exists: Option<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'table' \
             AND name = 'worker_conversation_thread'",
        )
        .fetch_optional(&pool)
        .await
        .expect("probe sqlite_master");
        assert!(exists.is_none(), "table should be dropped by v87 down");
    }
}
