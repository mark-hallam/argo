//! `forge_run` mutators + read helpers for `argo forge runs
//! show / abort`. Lives in its own file so `lib.rs` stays
//! under the 500-LOC ceiling.
use crate::models::ForgeRunRow;
use crate::SessionStore;
use crate::StoreResult;
use argo_types::ArgoError;
use chrono::Utc;
use sqlx::{Pool, Sqlite};
use uuid::Uuid;

impl SessionStore {
    /// Fetch one forge_run row by id. Returns `None` if no row
    /// matches. Used by `argo forge runs show <id>`.
    pub async fn get_forge_run(&self, id: Uuid) -> StoreResult<Option<ForgeRunRow>> {
        get_forge_run(self.pool(), id).await
            .map_err(|e| ArgoError::Provider(format!("get_forge_run: {e}")))
    }

    /// Mark a forge_run as `aborted`. Returns rows affected (0
    /// when no row matches). Backs `argo forge runs abort <id>`.
    pub async fn abort_forge_run(&self, id: Uuid) -> StoreResult<u64> {
        abort_forge_run(self.pool(), id).await
            .map_err(|e| ArgoError::Provider(format!("abort_forge_run: {e}")))
    }
}

async fn get_forge_run(
    pool: &Pool<Sqlite>,
    id: Uuid,
) -> Result<Option<ForgeRunRow>, sqlx::Error> {
    sqlx::query_as::<_, ForgeRunRow>(
        "SELECT id, skill_name, started_at, ended_at, base_version, \
         candidate_version, iterations, spend_usd_micros, \
         final_score, status FROM forge_run WHERE id = ?",
    )
    .bind(id.as_bytes().as_slice())
    .fetch_optional(pool)
    .await
}

async fn abort_forge_run(pool: &Pool<Sqlite>, id: Uuid) -> Result<u64, sqlx::Error> {
    let res = sqlx::query(
        "UPDATE forge_run SET status = 'aborted', ended_at = ? WHERE id = ?",
    )
    .bind(Utc::now().timestamp_micros())
    .bind(id.as_bytes().as_slice())
    .execute(pool)
    .await?;
    Ok(res.rows_affected())
}
