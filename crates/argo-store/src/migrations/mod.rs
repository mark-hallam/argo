//! Versioned, reversible schema migrations.
mod v1;
mod v2;
mod v3;
mod v4;
mod v5;
mod v10;
mod v11;
mod v6;
mod v7;
mod v8;
mod v9;

use argo_types::ArgoError;
use sqlx::{Pool, Sqlite};

pub(super) async fn run(pool: &Pool<Sqlite>) -> Result<(), ArgoError> {
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

pub(super) async fn roll_back_to(pool: &Pool<Sqlite>, target: u32) -> Result<(), ArgoError> {
    ensure_schema_table(pool).await?;
    let mut current = current_version(pool).await?;
    let migs = all_migrations();
    while current > target {
        let m = migs
            .iter()
            .find(|m| m.version == current)
            .ok_or(ArgoError::Invariant("missing migration"))?;
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

pub(super) async fn current_version(pool: &Pool<Sqlite>) -> Result<u32, ArgoError> {
    ensure_schema_table(pool).await?;
    let v: Option<i64> = sqlx::query_scalar("SELECT version FROM schema_version WHERE id = 1")
        .fetch_optional(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("schema_version read: {e}")))?;
    Ok(u32::try_from(v.unwrap_or(0)).unwrap_or(0))
}

async fn ensure_schema_table(pool: &Pool<Sqlite>) -> Result<(), ArgoError> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS schema_version (
            id          INTEGER PRIMARY KEY CHECK (id = 1),
            version     INTEGER NOT NULL,
            applied_at  INTEGER NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("ensure schema_version: {e}")))?;
    Ok(())
}

type MigFn = for<'a, 'b> fn(
    &'a mut sqlx::Transaction<'b, Sqlite>,
) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<(), ArgoError>> + Send + 'a>,
>;

pub(super) struct Migration {
    pub version: u32,
    pub up: MigFn,
    pub down: MigFn,
}

fn all_migrations() -> Vec<Migration> {
    vec![
        Migration {
            version: 1,
            up: |tx| Box::pin(v1::up(tx)),
            down: |tx| Box::pin(v1::down(tx)),
        },
        Migration {
            version: 2,
            up: |tx| Box::pin(v2::up(tx)),
            down: |tx| Box::pin(v2::down(tx)),
        },
        Migration {
            version: 3,
            up: |tx| Box::pin(v3::up(tx)),
            down: |tx| Box::pin(v3::down(tx)),
        },
        Migration {
            version: 4,
            up: |tx| Box::pin(v4::up(tx)),
            down: |tx| Box::pin(v4::down(tx)),
        },
        Migration {
            version: 5,
            up: |tx| Box::pin(v5::up(tx)),
            down: |tx| Box::pin(v5::down(tx)),
        },
        Migration {
            version: 6,
            up: |tx| Box::pin(v6::up(tx)),
            down: |tx| Box::pin(v6::down(tx)),
        },
        Migration {
            version: 7,
            up: |tx| Box::pin(v7::up(tx)),
            down: |tx| Box::pin(v7::down(tx)),
        },
        Migration {
            version: 8,
            up: |tx| Box::pin(v8::up(tx)),
            down: |tx| Box::pin(v8::down(tx)),
        },
        Migration {
            version: 9,
            up: |tx| Box::pin(v9::up(tx)),
            down: |tx| Box::pin(v9::down(tx)),
        },
        Migration {
            version: 10,
            up: |tx| Box::pin(v10::up(tx)),
            down: |tx| Box::pin(v10::down(tx)),
        },
        Migration {
            version: 11,
            up: |tx| Box::pin(v11::up(tx)),
            down: |tx| Box::pin(v11::down(tx)),
        },
    ]
}
