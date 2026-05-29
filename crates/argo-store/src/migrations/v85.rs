//! Schema migration v85 — the generic per-tenant `workspace_settings`
//! key/value table backing the `/settings` SPA surface (theatre item
//! `T1`).
//!
//! `/settings` had three read-only panels (workspace, providers,
//! notifications) that the F13 audit flagged as hardcoded mocks. This
//! migration adds the one table all three persist into:
//!
//! ```text
//! workspace_settings(
//!   tenant_id  TEXT NOT NULL,
//!   group_key  TEXT NOT NULL,   -- "workspace" | "providers" | "notifications"
//!   value_json TEXT NOT NULL,   -- the whole panel object, serialized
//!   updated_at INTEGER NOT NULL,
//!   PRIMARY KEY (tenant_id, group_key)
//! )
//! ```
//!
//! One row per `(tenant, group)`. The gateway handler upserts the full
//! merged panel object into `value_json`; absent rows fall back to the
//! bundled defaults in `crates/argo-cli/.../routes_workspace_settings`
//! so a GET always returns a complete shape. The composite primary key
//! gives the per-tenant scoping for free — a query keyed by
//! `(tenant_id, group_key)` can never read another tenant's row.
//!
//! Additive + idempotent: `CREATE TABLE IF NOT EXISTS` only, no
//! existing column is touched, so the migration is safe to re-run and
//! safe to roll forward over any prior schema version. Inline SQL
//! mirrors the v83 pattern (no `migrations/NNN__*.sql` slot file, so no
//! filesystem slot to collide with).

use argo_types::ArgoError;
use sqlx::Sqlite;

const V85_UP_SQL: &str = "CREATE TABLE IF NOT EXISTS workspace_settings (\
    tenant_id  TEXT NOT NULL,\
    group_key  TEXT NOT NULL,\
    value_json TEXT NOT NULL,\
    updated_at INTEGER NOT NULL,\
    PRIMARY KEY (tenant_id, group_key)\
)";

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    sqlx::query(V85_UP_SQL)
        .execute(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("v85 up: {e}")))?;
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    sqlx::query("DROP TABLE IF EXISTS workspace_settings")
        .execute(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("v85 down: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn up_creates_workspace_settings_table_on_memory() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("pool");

        // Run the full versioned runner up to and including v85 so the
        // migration is exercised exactly as the runtime applies it.
        super::super::run_up_to(&pool, 85).await.expect("migrations");

        // The table exists and accepts the documented shape.
        sqlx::query(
            "INSERT INTO workspace_settings (tenant_id, group_key, value_json, updated_at) \
             VALUES ('tenant-a', 'workspace', '{\"name\":\"x\"}', 1)",
        )
        .execute(&pool)
        .await
        .expect("insert into workspace_settings");

        let value: String = sqlx::query_scalar(
            "SELECT value_json FROM workspace_settings \
             WHERE tenant_id = 'tenant-a' AND group_key = 'workspace'",
        )
        .fetch_one(&pool)
        .await
        .expect("read back row");
        assert_eq!(value, "{\"name\":\"x\"}");
    }

    #[tokio::test]
    async fn up_enforces_composite_primary_key() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("pool");
        super::super::run_up_to(&pool, 85).await.expect("migrations");

        sqlx::query(
            "INSERT INTO workspace_settings (tenant_id, group_key, value_json, updated_at) \
             VALUES ('t', 'providers', '{}', 1)",
        )
        .execute(&pool)
        .await
        .expect("first insert");

        // Same (tenant, group) — must violate the PRIMARY KEY.
        let dup = sqlx::query(
            "INSERT INTO workspace_settings (tenant_id, group_key, value_json, updated_at) \
             VALUES ('t', 'providers', '{}', 2)",
        )
        .execute(&pool)
        .await;
        let err = dup.expect_err("duplicate (tenant, group) must fail");
        assert!(
            err.to_string().contains("UNIQUE") || err.to_string().contains("PRIMARY"),
            "expected primary-key violation, got: {err}"
        );

        // Different group under the same tenant is allowed.
        sqlx::query(
            "INSERT INTO workspace_settings (tenant_id, group_key, value_json, updated_at) \
             VALUES ('t', 'notifications', '{}', 3)",
        )
        .execute(&pool)
        .await
        .expect("distinct group under same tenant");
    }
}
