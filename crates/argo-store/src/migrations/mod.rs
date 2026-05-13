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
mod v58;
mod v59;
mod v60;
mod v61;
mod v62;
mod v63;
mod v6;
mod v7;
mod v8;
mod v9;

mod registry;

use argo_types::ArgoError;
use sqlx::{Pool, Sqlite};

pub(crate) use registry::{current_version, roll_back_to, run};
#[cfg(test)]
pub(crate) use registry::run_up_to;

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
