//! Schema migration v86 — the first-class `api_key` table backing the
//! `/settings/api-keys` SPA surface (Frontier-B depth, SPEC 2).
//!
//! `GET /api/v1/api-keys` was a hard-coded `200 []` stub because no
//! Rust home existed for the SPA's named, scoped key model — only
//! `service_token_binding` (a token *hash* + principal, no name /
//! scopes / cohorts). This migration adds the table all three verbs
//! (list / create / revoke) persist into:
//!
//! ```text
//! api_key(
//!   id              TEXT PRIMARY KEY,
//!   tenant_id       TEXT NOT NULL,
//!   name            TEXT NOT NULL,
//!   token_hash      TEXT NOT NULL UNIQUE,  -- SHA-256 hex, the ONLY secret material
//!   prefix          TEXT NOT NULL,         -- 12-char masked preview, stored at create
//!   scopes_json     TEXT NOT NULL,
//!   cohort_ids_json TEXT NOT NULL,
//!   all_cohorts     INTEGER NOT NULL,
//!   created_at      TEXT NOT NULL,
//!   last_used_at    TEXT,
//!   expires_at      TEXT,
//!   revoked         INTEGER NOT NULL DEFAULT 0
//! )
//! ```
//!
//! `token_hash` is the SHA-256 hex digest of the raw bearer token; the
//! raw token never lands in the database — it is returned exactly once
//! in the create response and forgotten. `prefix` is the masked preview
//! computed at create time (`sk_live_a4b8...c2e1`), stored as plain
//! metadata so a list/read never re-derives anything from the secret.
//! The composite `(tenant_id, revoked)` index gives the live-key list a
//! tenant-scoped, revoked-filtered read path.
//!
//! Additive + idempotent: `CREATE TABLE IF NOT EXISTS` /
//! `CREATE INDEX IF NOT EXISTS` only, no existing column is touched, so
//! the migration is safe to re-run and safe to roll forward over any
//! prior schema version. Inline SQL mirrors the v85 pattern (no
//! `migrations/NNN__*.sql` slot file, so no filesystem slot to collide
//! with — the `duplicate_slot_validator` that runs at boot stays happy).

use argo_types::ArgoError;
use sqlx::Sqlite;

const V86_UP_TABLE_SQL: &str = "CREATE TABLE IF NOT EXISTS api_key (\
    id              TEXT PRIMARY KEY,\
    tenant_id       TEXT NOT NULL,\
    name            TEXT NOT NULL,\
    token_hash      TEXT NOT NULL UNIQUE,\
    prefix          TEXT NOT NULL,\
    scopes_json     TEXT NOT NULL,\
    cohort_ids_json TEXT NOT NULL,\
    all_cohorts     INTEGER NOT NULL,\
    created_at      TEXT NOT NULL,\
    last_used_at    TEXT,\
    expires_at      TEXT,\
    revoked         INTEGER NOT NULL DEFAULT 0\
)";

const V86_UP_INDEX_SQL: &str = "CREATE INDEX IF NOT EXISTS idx_api_key_tenant_revoked \
    ON api_key(tenant_id, revoked)";

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    sqlx::query(V86_UP_TABLE_SQL)
        .execute(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("v86 up table: {e}")))?;
    sqlx::query(V86_UP_INDEX_SQL)
        .execute(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("v86 up index: {e}")))?;
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    sqlx::query("DROP TABLE IF EXISTS api_key")
        .execute(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("v86 down: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn up_creates_api_key_table_on_memory() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("pool");

        // Run the full versioned runner up to and including v86 so the
        // migration is exercised exactly as the runtime applies it.
        super::super::run_up_to(&pool, 86).await.expect("migrations");

        // The table exists and accepts the documented shape.
        sqlx::query(
            "INSERT INTO api_key \
             (id, tenant_id, name, token_hash, prefix, scopes_json, cohort_ids_json, \
              all_cohorts, created_at, revoked) \
             VALUES ('k1', 'tenant-a', 'CI', 'deadbeef', 'sk_live_a4b8...c2e1', \
                     '[\"read:workers\"]', '[]', 1, '2026-06-01T00:00:00Z', 0)",
        )
        .execute(&pool)
        .await
        .expect("insert into api_key");

        let name: String =
            sqlx::query_scalar("SELECT name FROM api_key WHERE id = 'k1'")
                .fetch_one(&pool)
                .await
                .expect("read back row");
        assert_eq!(name, "CI");
    }

    #[tokio::test]
    async fn up_enforces_unique_token_hash() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("pool");
        super::super::run_up_to(&pool, 86).await.expect("migrations");

        sqlx::query(
            "INSERT INTO api_key \
             (id, tenant_id, name, token_hash, prefix, scopes_json, cohort_ids_json, \
              all_cohorts, created_at, revoked) \
             VALUES ('k1', 't', 'A', 'samehash', 'p', '[]', '[]', 1, 'now', 0)",
        )
        .execute(&pool)
        .await
        .expect("first insert");

        // Same token_hash — must violate the UNIQUE constraint.
        let dup = sqlx::query(
            "INSERT INTO api_key \
             (id, tenant_id, name, token_hash, prefix, scopes_json, cohort_ids_json, \
              all_cohorts, created_at, revoked) \
             VALUES ('k2', 't', 'B', 'samehash', 'p', '[]', '[]', 1, 'now', 0)",
        )
        .execute(&pool)
        .await;
        let err = dup.expect_err("duplicate token_hash must fail");
        assert!(
            err.to_string().contains("UNIQUE"),
            "expected UNIQUE violation, got: {err}"
        );
    }
}
