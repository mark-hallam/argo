//! Schema migration v83 — quarantine legacy audit rows under the
//! explicit `__legacy__` tenant column without rewriting
//! `payload_json`.
//!
//! Historical audit rows predate payload tenant stamping. Rewriting
//! their payload would invalidate the audit hash-chain input, so this
//! migration only corrects the denormalized `audit_record.tenant_id`
//! bucket for rows whose payload tenant is missing or non-string.

use argo_types::ArgoError;
use sqlx::Sqlite;

const LEGACY_AUDIT_TENANT_ID: &str = "__legacy__";

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    sqlx::query(
        "UPDATE audit_record \
         SET tenant_id = ? \
         WHERE json_type(payload_json, '$.tenant_id') IS NULL \
            OR json_type(payload_json, '$.tenant_id') != 'text'",
    )
    .bind(LEGACY_AUDIT_TENANT_ID)
    .execute(&mut **tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("v83 up: {e}")))?;
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    _tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    // No reversible old tenant exists for legacy payloads. Leaving
    // the column quarantine intact preserves the audit chain and
    // avoids reintroducing the cross-tenant list leak on rollback.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::up;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn up_quarantines_legacy_rows_without_rewriting_payload() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("pool");
        sqlx::query(
            "CREATE TABLE audit_record (\
                payload_json TEXT NOT NULL,\
                tenant_id TEXT NOT NULL\
             )",
        )
        .execute(&pool)
        .await
        .expect("schema");
        let legacy_payload = r#"{"kind":"old","body":"kept byte-for-byte"}"#;
        sqlx::query("INSERT INTO audit_record (payload_json, tenant_id) VALUES (?, 'tenant-a')")
            .bind(legacy_payload)
            .execute(&pool)
            .await
            .expect("legacy row");
        sqlx::query(
            "INSERT INTO audit_record (payload_json, tenant_id) \
             VALUES ('{\"tenant_id\":\"tenant-b\",\"kind\":\"new\"}', 'tenant-b')",
        )
        .execute(&pool)
        .await
        .expect("tenant row");

        let mut tx = pool.begin().await.expect("begin");
        up(&mut tx).await.expect("up");
        tx.commit().await.expect("commit");

        let rows: Vec<(String, String)> =
            sqlx::query_as("SELECT payload_json, tenant_id FROM audit_record ORDER BY rowid")
                .fetch_all(&pool)
                .await
                .expect("rows");
        assert_eq!(rows[0].0, legacy_payload);
        assert_eq!(rows[0].1, "__legacy__");
        assert_eq!(rows[1].1, "tenant-b");
    }
}
