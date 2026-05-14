//! Migration registry — the ordered list of every schema migration plus
//! the runtime helpers (`run`, `roll_back_to`, `current_version`,
//! `run_up_to`) that iterate it.
//!
//! Split out from `mod.rs` to keep the parent module under the 500-LOC
//! ceiling. New migration registrations live in ordered sibling
//! modules, and this file keeps the runner orchestration glue.

use argo_types::ArgoError;
use sqlx::{Pool, Sqlite};

use super::{
    ensure_schema_table, registry_compliance, registry_skills, registry_workers, registry_workflow,
    Migration,
};

pub(crate) async fn run(pool: &Pool<Sqlite>) -> Result<(), ArgoError> {
    ensure_schema_table(pool).await?;
    let mut current = current_version(pool).await?;
    for m in all_migrations() {
        if m.version > current {
            let mut tx = pool
                .begin()
                .await
                .map_err(|e| ArgoError::Provider(format!("migration tx: {e}")))?;
            (m.up)(&mut tx).await?;
            sqlx::query(
                "INSERT OR REPLACE INTO schema_version (id, version, applied_at)
                 VALUES (1, ?, strftime('%s','now'))",
            )
            .bind(i64::from(m.version))
            .execute(&mut *tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("schema_version write: {e}")))?;
            tx.commit()
                .await
                .map_err(|e| ArgoError::Provider(format!("migration commit: {e}")))?;
            current = m.version;
        }
    }
    Ok(())
}

pub(crate) async fn roll_back_to(pool: &Pool<Sqlite>, target: u32) -> Result<(), ArgoError> {
    ensure_schema_table(pool).await?;
    let mut current = current_version(pool).await?;
    let migs = all_migrations();
    while current > target {
        let m = migs
            .iter()
            .find(|m| m.version == current)
            .ok_or_else(|| ArgoError::Invariant("missing migration".into()))?;
        let mut tx = pool
            .begin()
            .await
            .map_err(|e| ArgoError::Provider(format!("rollback tx: {e}")))?;
        (m.down)(&mut tx).await?;
        sqlx::query("DELETE FROM schema_version WHERE id = 1")
            .execute(&mut *tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("schema_version delete: {e}")))?;
        sqlx::query(
            "INSERT INTO schema_version (id, version, applied_at)
             VALUES (1, ?, strftime('%s','now'))",
        )
        .bind(i64::from(m.version - 1))
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("schema_version write: {e}")))?;
        tx.commit()
            .await
            .map_err(|e| ArgoError::Provider(format!("rollback commit: {e}")))?;
        current = m.version - 1;
    }
    Ok(())
}

pub(crate) async fn current_version(pool: &Pool<Sqlite>) -> Result<u32, ArgoError> {
    ensure_schema_table(pool).await?;
    let v: Option<i64> = sqlx::query_scalar("SELECT version FROM schema_version WHERE id = 1")
        .fetch_optional(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("schema_version read: {e}")))?;
    Ok(u32::try_from(v.unwrap_or(0)).unwrap_or(0))
}

#[cfg(test)]
pub(crate) async fn run_up_to(pool: &Pool<Sqlite>, target: u32) -> Result<(), ArgoError> {
    ensure_schema_table(pool).await?;
    let mut current = current_version(pool).await?;
    for m in all_migrations().into_iter().filter(|m| m.version <= target) {
        if m.version > current {
            let mut tx = pool
                .begin()
                .await
                .map_err(|e| ArgoError::Provider(format!("migration tx: {e}")))?;
            (m.up)(&mut tx).await?;
            sqlx::query(
                "INSERT OR REPLACE INTO schema_version (id, version, applied_at)
                 VALUES (1, ?, strftime('%s','now'))",
            )
            .bind(i64::from(m.version))
            .execute(&mut *tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("schema_version write: {e}")))?;
            tx.commit()
                .await
                .map_err(|e| ArgoError::Provider(format!("migration commit: {e}")))?;
            current = m.version;
        }
    }
    Ok(())
}

pub(crate) fn all_migrations() -> Vec<Migration> {
    let mut migrations = Vec::new();
    migrations.extend(registry_workers::migrations());
    migrations.extend(registry_skills::migrations());
    migrations.extend(registry_workflow::migrations());
    migrations.extend(registry_compliance::migrations());
    migrations
}
