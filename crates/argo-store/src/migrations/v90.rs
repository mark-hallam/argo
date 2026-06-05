//! Schema migration v90 — add `worker_id` to `spend_ledger` for
//! per-teammate cost attribution (UF Phase-A · A6, DEC-038).
//!
//! `spend_ledger` (created in v2) records one row per charged turn but
//! carries no teammate identity, so cost can only be rolled up per
//! `(tenant, session)` — never split per AI teammate ("worker"). This
//! migration threads the **already-existing** `session.worker_id`
//! (added long ago; consumed here, not re-introduced) onto the spend
//! row so dashboards can answer "what did Peta cost this month?".
//!
//! ```text
//! ALTER TABLE spend_ledger ADD COLUMN worker_id BLOB   -- nullable 16-byte UUID
//! ```
//!
//! **Additive + null-safe.** The column is nullable with no default:
//! existing v89 rows keep `worker_id = NULL`, system/non-teammate
//! sessions write `NULL`, and every existing rollup (`load_spend_counters`,
//! `load_global_spend`, budget sums) ignores the column entirely, so a
//! v89 database upgrades to v90 with zero behaviour change to current
//! reporting. No drop, no rename, no backfill — see
//! `docs/skycot/operator/migrations.md`. Inline `ALTER` (no
//! `migrations/NNN__*.sql` slot file) so the `duplicate_slot_validator`
//! that runs at boot stays happy.

use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    sqlx::query("ALTER TABLE spend_ledger ADD COLUMN worker_id BLOB")
        .execute(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("v90 up add worker_id: {e}")))?;
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    sqlx::query("ALTER TABLE spend_ledger DROP COLUMN worker_id")
        .execute(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("v90 down drop worker_id: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::SqlitePoolOptions;
    use sqlx::Row;

    /// A fresh DB migrated up to v90 carries a nullable `worker_id`
    /// column on `spend_ledger`, and a v89-era row (NULL worker_id)
    /// coexists with a v90 row that carries one.
    #[tokio::test]
    async fn up_adds_nullable_worker_id_to_spend_ledger() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("pool");

        // Migrate only up to v89 first, insert a legacy (no worker_id) row.
        super::super::run_up_to(&pool, 89).await.expect("v89");
        let tenant = vec![1u8; 16];
        let session = vec![2u8; 16];
        sqlx::query(
            "INSERT INTO spend_ledger (tenant_id, session_id, charged_at_micros, \
             amount_usd_micros, provider, model) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(tenant.as_slice())
        .bind(session.as_slice())
        .bind(1_000i64)
        .bind(500i64)
        .bind("p")
        .bind("m")
        .execute(&pool)
        .await
        .expect("legacy insert");

        // Now apply v90 — the legacy row survives with NULL worker_id.
        super::super::run_up_to(&pool, 90).await.expect("v90");
        let worker_id: Option<Vec<u8>> =
            sqlx::query("SELECT worker_id FROM spend_ledger WHERE provider = 'p'")
                .fetch_one(&pool)
                .await
                .expect("legacy row")
                .get("worker_id");
        assert!(worker_id.is_none(), "legacy row must keep NULL worker_id");

        // A new row can carry a worker_id, and a second can stay NULL.
        let worker = vec![9u8; 16];
        sqlx::query(
            "INSERT INTO spend_ledger (tenant_id, session_id, charged_at_micros, \
             amount_usd_micros, provider, model, worker_id) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(tenant.as_slice())
        .bind(session.as_slice())
        .bind(2_000i64)
        .bind(700i64)
        .bind("p")
        .bind("m")
        .bind(worker.as_slice())
        .execute(&pool)
        .await
        .expect("worker insert");

        let with_worker: Option<Vec<u8>> =
            sqlx::query("SELECT worker_id FROM spend_ledger WHERE charged_at_micros = 2000")
                .fetch_one(&pool)
                .await
                .expect("worker row")
                .get("worker_id");
        assert_eq!(with_worker.as_deref(), Some(worker.as_slice()));

        // Existing rollups are unaffected: two rows, summed amount intact.
        let total: i64 =
            sqlx::query_scalar("SELECT COALESCE(SUM(amount_usd_micros), 0) FROM spend_ledger")
                .fetch_one(&pool)
                .await
                .expect("sum");
        assert_eq!(total, 1_200);
    }
}
