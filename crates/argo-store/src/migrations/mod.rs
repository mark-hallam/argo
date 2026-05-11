//! Versioned, reversible schema migrations.
mod v1;
mod v2;
mod v3;
mod v4;
mod v5;
mod v10;
mod v11;
mod v12;
mod v13;
mod v14;
mod v15;
mod v16;
mod v17;
mod v18;
mod v19;
mod v20;
mod v21_skill_ab_tests;
mod v22;
mod v23;
mod v24;
mod v25;
mod v26;
mod v27;
mod v28;
mod v29;
mod v30;
mod v31;
mod v32;
mod v33;
mod v34;
mod v35;
mod v36;
mod v37;
mod v38;
mod v39;
mod v40;
mod v41;
mod v42;
mod v43;
mod v44;
mod v45;
mod v46;
mod v47;
mod v48;
mod v49;
mod v50;
mod v51;
mod v52;
mod v53;
mod v54;
mod v55;
mod v56;
mod v57;
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
        Migration {
            version: 12,
            up: |tx| Box::pin(v12::up(tx)),
            down: |tx| Box::pin(v12::down(tx)),
        },
        Migration {
            version: 13,
            up: |tx| Box::pin(v13::up(tx)),
            down: |tx| Box::pin(v13::down(tx)),
        },
        Migration {
            version: 14,
            up: |tx| Box::pin(v14::up(tx)),
            down: |tx| Box::pin(v14::down(tx)),
        },
        Migration {
            version: 15,
            up: |tx| Box::pin(v15::up(tx)),
            down: |tx| Box::pin(v15::down(tx)),
        },
        Migration {
            version: 16,
            up: |tx| Box::pin(v16::up(tx)),
            down: |tx| Box::pin(v16::down(tx)),
        },
        Migration {
            version: 17,
            up: |tx| Box::pin(v17::up(tx)),
            down: |tx| Box::pin(v17::down(tx)),
        },
        Migration {
            version: 18,
            up: |tx| Box::pin(v18::up(tx)),
            down: |tx| Box::pin(v18::down(tx)),
        },
        Migration {
            version: 19,
            up: |tx| Box::pin(v19::up(tx)),
            down: |tx| Box::pin(v19::down(tx)),
        },
        Migration {
            version: 20,
            up: |tx| Box::pin(v20::up(tx)),
            down: |tx| Box::pin(v20::down(tx)),
        },
        Migration {
            version: 21,
            up: |tx| Box::pin(v21_skill_ab_tests::up(tx)),
            down: |tx| Box::pin(v21_skill_ab_tests::down(tx)),
        },
        Migration {
            version: 22,
            up: |tx| Box::pin(v22::up(tx)),
            down: |tx| Box::pin(v22::down(tx)),
        },
        Migration {
            version: 23,
            up: |tx| Box::pin(v23::up(tx)),
            down: |tx| Box::pin(v23::down(tx)),
        },
        Migration {
            version: 24,
            up: |tx| Box::pin(v24::up(tx)),
            down: |tx| Box::pin(v24::down(tx)),
        },
        Migration {
            version: 25,
            up: |tx| Box::pin(v25::up(tx)),
            down: |tx| Box::pin(v25::down(tx)),
        },
        Migration {
            version: 26,
            up: |tx| Box::pin(v26::up(tx)),
            down: |tx| Box::pin(v26::down(tx)),
        },
        Migration {
            version: 27,
            up: |tx| Box::pin(v27::up(tx)),
            down: |tx| Box::pin(v27::down(tx)),
        },
        Migration {
            version: 28,
            up: |tx| Box::pin(v28::up(tx)),
            down: |tx| Box::pin(v28::down(tx)),
        },
        Migration {
            version: 29,
            up: |tx| Box::pin(v29::up(tx)),
            down: |tx| Box::pin(v29::down(tx)),
        },
        Migration {
            version: 30,
            up: |tx| Box::pin(v30::up(tx)),
            down: |tx| Box::pin(v30::down(tx)),
        },
        Migration {
            version: 31,
            up: |tx| Box::pin(v31::up(tx)),
            down: |tx| Box::pin(v31::down(tx)),
        },
        Migration {
            version: 32,
            up: |tx| Box::pin(v32::up(tx)),
            down: |tx| Box::pin(v32::down(tx)),
        },
        Migration {
            version: 33,
            up: |tx| Box::pin(v33::up(tx)),
            down: |tx| Box::pin(v33::down(tx)),
        },
        Migration {
            version: 34,
            up: |tx| Box::pin(v34::up(tx)),
            down: |tx| Box::pin(v34::down(tx)),
        },
        Migration {
            version: 35,
            up: |tx| Box::pin(v35::up(tx)),
            down: |tx| Box::pin(v35::down(tx)),
        },
        Migration {
            version: 36,
            up: |tx| Box::pin(v36::up(tx)),
            down: |tx| Box::pin(v36::down(tx)),
        },
        Migration {
            version: 37,
            up: |tx| Box::pin(v37::up(tx)),
            down: |tx| Box::pin(v37::down(tx)),
        },
        Migration {
            version: 38,
            up: |tx| Box::pin(v38::up(tx)),
            down: |tx| Box::pin(v38::down(tx)),
        },
        Migration {
            version: 39,
            up: |tx| Box::pin(v39::up(tx)),
            down: |tx| Box::pin(v39::down(tx)),
        },
        Migration {
            version: 40,
            up: |tx| Box::pin(v40::up(tx)),
            down: |tx| Box::pin(v40::down(tx)),
        },
        Migration {
            version: 41,
            up: |tx| Box::pin(v41::up(tx)),
            down: |tx| Box::pin(v41::down(tx)),
        },
        Migration {
            version: 42,
            up: |tx| Box::pin(v42::up(tx)),
            down: |tx| Box::pin(v42::down(tx)),
        },
        Migration {
            version: 43,
            up: |tx| Box::pin(v43::up(tx)),
            down: |tx| Box::pin(v43::down(tx)),
        },
        Migration {
            version: 44,
            up: |tx| Box::pin(v44::up(tx)),
            down: |tx| Box::pin(v44::down(tx)),
        },
        Migration {
            version: 45,
            up: |tx| Box::pin(v45::up(tx)),
            down: |tx| Box::pin(v45::down(tx)),
        },
        Migration {
            version: 46,
            up: |tx| Box::pin(v46::up(tx)),
            down: |tx| Box::pin(v46::down(tx)),
        },
        Migration {
            version: 47,
            up: |tx| Box::pin(v47::up(tx)),
            down: |tx| Box::pin(v47::down(tx)),
        },
        Migration {
            version: 48,
            up: |tx| Box::pin(v48::up(tx)),
            down: |tx| Box::pin(v48::down(tx)),
        },
        Migration {
            version: 49,
            up: |tx| Box::pin(v49::up(tx)),
            down: |tx| Box::pin(v49::down(tx)),
        },
        Migration {
            version: 50,
            up: |tx| Box::pin(v50::up(tx)),
            down: |tx| Box::pin(v50::down(tx)),
        },
        Migration {
            version: 51,
            up: |tx| Box::pin(v51::up(tx)),
            down: |tx| Box::pin(v51::down(tx)),
        },
        Migration {
            version: 52,
            up: |tx| Box::pin(v52::up(tx)),
            down: |tx| Box::pin(v52::down(tx)),
        },
        Migration {
            version: 53,
            up: |tx| Box::pin(v53::up(tx)),
            down: |tx| Box::pin(v53::down(tx)),
        },
        Migration {
            version: 54,
            up: |tx| Box::pin(v54::up(tx)),
            down: |tx| Box::pin(v54::down(tx)),
        },
        Migration {
            version: 55,
            up: |tx| Box::pin(v55::up(tx)),
            down: |tx| Box::pin(v55::down(tx)),
        },
        Migration {
            version: 56,
            up: |tx| Box::pin(v56::up(tx)),
            down: |tx| Box::pin(v56::down(tx)),
        },
        Migration {
            version: 57,
            up: |tx| Box::pin(v57::up(tx)),
            down: |tx| Box::pin(v57::down(tx)),
        },
    ]
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
