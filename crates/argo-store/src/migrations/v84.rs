//! Schema migration v84 — relax the `skill` table's uniqueness
//! invariant from `UNIQUE(name, version)` to
//! `UNIQUE(name, version, tenant_id)` so per-tenant catalog seeding
//! works. Stream
//! `fix-skill-table-unique-name-version-globally-blocks-per-tenant-seed`.
//! Source: evaluator round 97 (2026-05-27) — `seeds_independently_per_tenant`
//! still fails post-#2437 with `UNIQUE constraint failed: skill.name,
//! skill.version` because v80 added the `tenant_id` column but did not
//! amend the uniqueness invariant.
//!
//! SQLite cannot drop a UNIQUE constraint with `ALTER TABLE`. The body
//! lives in `migrations/135__skill_unique_per_tenant.sql` and follows
//! the canonical table-swap (create `skill_new` with the tightened
//! UNIQUE, copy rows, drop old, rename, recreate the four auxiliary
//! indexes from v9/v11/v41/v80).
//!
//! FK enforcement is OFF in the live runtime today (no
//! `PRAGMA foreign_keys = ON` anywhere in `crates/argo-store/`), so
//! `DROP TABLE skill` does not currently cascade-delete `session_skill`
//! rows. To forward-protect the migration if FKs ever flip on, the SQL
//! materialises `session_skill` into a temp table, empties the live
//! table before the drop, swaps, and re-inserts. `PRAGMA
//! defer_foreign_keys = ON` is set alongside the save/restore — it
//! defers FK *violation checks* to commit, but it does NOT defer
//! `ON DELETE CASCADE` actions (CASCADE fires immediately during the
//! triggering DELETE/DROP), which is why the save/restore is the
//! load-bearing guard.
use argo_types::ArgoError;
use sqlx::Sqlite;

use super::split_sql_statements;

const V84_UP_SQL: &str = include_str!("../../migrations/135__skill_unique_per_tenant.sql");

/// Down reverses the swap: rebuild `skill_old` with the v83-shape
/// uniqueness invariant `UNIQUE(name, version)`, copy every row
/// (including the `__migration_placeholder__` sentinels left by v80),
/// drop, rename, then restore the same four auxiliary indexes. Uses
/// the same `session_skill` save-and-restore guard as up() so the
/// rollback is forward-safe under FK enforcement. NOTE: rolling back
/// from a state where multiple tenants seeded the same skill will fail
/// at `INSERT INTO skill_old` with `UNIQUE constraint failed:
/// skill_old.name, skill_old.version` — that is the inherent
/// operational limit of restoring a globally-unique invariant; no
/// migration code can paper over the data divergence.
const V84_DOWN_SQL: &str = "
PRAGMA defer_foreign_keys = ON;

CREATE TABLE IF NOT EXISTS _session_skill_v84_save (
  session_id  BLOB NOT NULL,
  skill_id    BLOB NOT NULL,
  attached_at INTEGER NOT NULL
);
DELETE FROM _session_skill_v84_save;
INSERT INTO _session_skill_v84_save SELECT session_id, skill_id, attached_at FROM session_skill;
DELETE FROM session_skill;

CREATE TABLE skill_old (
  id              BLOB PRIMARY KEY,
  name            TEXT NOT NULL,
  version         TEXT NOT NULL,
  body            TEXT NOT NULL,
  frontmatter     TEXT NOT NULL,
  source          TEXT NOT NULL,
  hash            BLOB NOT NULL,
  learned_in      BLOB,
  superseded_by   BLOB REFERENCES skill(id),
  created_at      INTEGER NOT NULL,
  vertical_id     TEXT,
  lifecycle_state TEXT NOT NULL DEFAULT 'active',
  shared          INTEGER NOT NULL DEFAULT 0,
  shared_with_cluster TEXT,
  min_trust_grade INTEGER,
  tenant_id       TEXT NOT NULL DEFAULT '__migration_placeholder__',
  UNIQUE(name, version)
);

INSERT INTO skill_old (
  id, name, version, body, frontmatter, source, hash,
  learned_in, superseded_by, created_at, vertical_id, lifecycle_state,
  shared, shared_with_cluster, min_trust_grade, tenant_id
)
SELECT id, name, version, body, frontmatter, source, hash,
  learned_in, superseded_by, created_at, vertical_id, lifecycle_state,
  shared, shared_with_cluster, min_trust_grade, tenant_id
FROM skill;

DROP TABLE skill;

ALTER TABLE skill_old RENAME TO skill;

CREATE INDEX idx_skill_vertical_id ON skill(vertical_id);
CREATE INDEX idx_skill_lifecycle ON skill(lifecycle_state);
CREATE INDEX idx_skill_shared_cluster ON skill(shared_with_cluster) WHERE shared = 1;
CREATE INDEX IF NOT EXISTS idx_skill_tenant_id ON skill(tenant_id);

INSERT INTO session_skill (session_id, skill_id, attached_at)
  SELECT session_id, skill_id, attached_at FROM _session_skill_v84_save;
DROP TABLE _session_skill_v84_save;

PRAGMA defer_foreign_keys = OFF;
";

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for stmt in split_sql_statements(V84_UP_SQL) {
        sqlx::query(&stmt)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v84 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for stmt in split_sql_statements(V84_DOWN_SQL) {
        sqlx::query(&stmt)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v84 down: {e}")))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::SqlitePoolOptions;
    use sqlx::{Pool, Sqlite};

    /// Build an in-memory pool with `PRAGMA foreign_keys = ON` armed on
    /// every connection. The runtime keeps FKs off, but these tests
    /// deliberately turn them on so the v84 `defer_foreign_keys` guard
    /// is exercised and any future FK-on regression would surface here
    /// rather than silently in production.
    async fn fk_on_pool() -> Pool<Sqlite> {
        SqlitePoolOptions::new()
            .max_connections(1)
            .after_connect(|conn, _meta| {
                Box::pin(async move {
                    sqlx::query("PRAGMA foreign_keys = ON")
                        .execute(&mut *conn)
                        .await?;
                    Ok(())
                })
            })
            .connect("sqlite::memory:")
            .await
            .expect("pool")
    }

    fn skill_blob_id(byte: u8) -> Vec<u8> {
        vec![byte; 16]
    }

    async fn insert_skill_row(
        pool: &Pool<Sqlite>,
        id_byte: u8,
        name: &str,
        version: &str,
        tenant: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO skill \
             (id, name, version, body, frontmatter, source, hash, created_at, tenant_id) \
             VALUES (?, ?, ?, '', '', '', X'00', 0, ?)",
        )
        .bind(skill_blob_id(id_byte))
        .bind(name)
        .bind(version)
        .bind(tenant)
        .execute(pool)
        .await
        .map(|_| ())
    }

    #[tokio::test]
    async fn up_allows_same_name_version_across_tenants() {
        let pool = fk_on_pool().await;
        super::super::run_up_to(&pool, 84).await.expect("migrations");

        insert_skill_row(&pool, 0xA1, "skill-a", "1.0.0", "tenant_a")
            .await
            .expect("tenant_a insert");
        // Same name + version, DIFFERENT tenant — must succeed under
        // the relaxed `UNIQUE(name, version, tenant_id)` invariant.
        insert_skill_row(&pool, 0xB2, "skill-a", "1.0.0", "tenant_b")
            .await
            .expect("tenant_b insert (post-fix invariant)");
        // Same name + version + SAME tenant — must still fail
        // (per-tenant uniqueness still bites).
        let dup = insert_skill_row(&pool, 0xC3, "skill-a", "1.0.0", "tenant_a").await;
        let err = dup.expect_err("per-tenant duplicate must still fail");
        let msg = err.to_string();
        assert!(
            msg.contains("UNIQUE") && msg.contains("skill"),
            "expected per-tenant UNIQUE violation, got: {msg}"
        );
    }

    #[tokio::test]
    async fn up_preserves_session_skill_fk_after_table_swap() {
        let pool = fk_on_pool().await;

        // Stop at v83 so we can seed pre-fix rows that the v84
        // table-swap must NOT cascade-delete.
        super::super::run_up_to(&pool, 83).await.expect("v83");

        let workspace_id = vec![0x10; 16];
        let session_id = vec![0x20; 16];
        let skill_id = skill_blob_id(0xD4);

        sqlx::query(
            "INSERT INTO workspace (id, tenant_id, name, root_path, created_at) \
             VALUES (?, X'00', 'w', '/tmp', 0)",
        )
        .bind(&workspace_id)
        .execute(&pool)
        .await
        .expect("workspace");

        sqlx::query(
            "INSERT INTO session \
             (id, tenant_id, workspace_id, started_at, model, provider) \
             VALUES (?, X'00', ?, 0, 'm', 'p')",
        )
        .bind(&session_id)
        .bind(&workspace_id)
        .execute(&pool)
        .await
        .expect("session");

        sqlx::query(
            "INSERT INTO skill \
             (id, name, version, body, frontmatter, source, hash, created_at, tenant_id) \
             VALUES (?, 'fk-skill', '1.0.0', '', '', '', X'00', 0, 'tenant_a')",
        )
        .bind(&skill_id)
        .execute(&pool)
        .await
        .expect("skill pre-v84");

        sqlx::query(
            "INSERT INTO session_skill (session_id, skill_id, attached_at) \
             VALUES (?, ?, 0)",
        )
        .bind(&session_id)
        .bind(&skill_id)
        .execute(&pool)
        .await
        .expect("session_skill pre-v84");

        // Apply v84 — the DROP TABLE skill step would cascade-delete
        // session_skill rows if defer_foreign_keys were omitted and
        // foreign_keys were on (as they are on this test pool).
        super::super::run_up_to(&pool, 84).await.expect("v84");

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM session_skill")
            .fetch_one(&pool)
            .await
            .expect("count");
        assert_eq!(
            count, 1,
            "session_skill row was cascade-deleted; v84 missed defer_foreign_keys"
        );
    }

    #[tokio::test]
    async fn up_preserves_all_skill_indexes_after_table_swap() {
        let pool = fk_on_pool().await;
        super::super::run_up_to(&pool, 84).await.expect("migrations");

        let names: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master \
             WHERE type = 'index' AND tbl_name = 'skill'",
        )
        .fetch_all(&pool)
        .await
        .expect("indexes");

        for required in [
            "idx_skill_vertical_id",
            "idx_skill_lifecycle",
            "idx_skill_shared_cluster",
            "idx_skill_tenant_id",
        ] {
            assert!(
                names.iter().any(|n| n == required),
                "missing index {required}; have: {names:?}"
            );
        }
    }
}
