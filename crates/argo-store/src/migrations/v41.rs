//! Schema migration v41 — additive `skill.shared` and
//! `skill.shared_with_cluster` columns for cross-tenant skill sharing
//! PoC (Stream `claude-skill-share-cross-tenant-poc`, mission claim 4).
//!
//! v40 is the cascade tombstone (#1054); v41 is the next free
//! versioned slot. Reconciles raw SQL `40__skill_shared.sql`. UP adds
//! the two columns + a partial index keyed on shared rows. DOWN drops
//! them.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V41_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v41 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V41_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v41 down: {e}")))?;
    }
    Ok(())
}

const V41_UP: &[&str] = &[
    "ALTER TABLE skill ADD COLUMN shared INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE skill ADD COLUMN shared_with_cluster TEXT",
    "CREATE INDEX IF NOT EXISTS idx_skill_shared_cluster \
     ON skill(shared_with_cluster) WHERE shared = 1",
];

const V41_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_skill_shared_cluster",
    "ALTER TABLE skill DROP COLUMN shared_with_cluster",
    "ALTER TABLE skill DROP COLUMN shared",
];
