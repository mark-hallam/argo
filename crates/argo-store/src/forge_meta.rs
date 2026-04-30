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

    /// Insert a fresh `forge_run` row in `running` state and
    /// return its id. Used by `argo forge bench` (and the eventual
    /// `forge optimize`) to track a live run.
    pub async fn start_forge_run(
        &self,
        skill_name: &str,
        base_version: &str,
        examples_count: u32,
    ) -> StoreResult<Uuid> {
        start_forge_run(self.pool(), skill_name, base_version, examples_count).await
            .map_err(|e| ArgoError::Provider(format!("start_forge_run: {e}")))
    }

    /// Update a `forge_run` row to a terminal status (succeeded /
    /// partial / failed) with its final aggregate score.
    pub async fn finish_forge_run(
        &self,
        id: Uuid,
        status: &str,
        final_score: Option<f64>,
        iterations: i64,
    ) -> StoreResult<u64> {
        finish_forge_run(self.pool(), id, status, final_score, iterations).await
            .map_err(|e| ArgoError::Provider(format!("finish_forge_run: {e}")))
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

async fn start_forge_run(
    pool: &Pool<Sqlite>,
    skill_name: &str,
    base_version: &str,
    examples_count: u32,
) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::now_v7();
    let now = Utc::now().timestamp_micros();
    sqlx::query(
        "INSERT INTO forge_run (id, skill_name, started_at, ended_at, base_version, \
         candidate_version, examples_count, iterations, spend_usd_micros, final_score, status) \
         VALUES (?, ?, ?, NULL, ?, NULL, ?, 0, 0, NULL, 'running')",
    )
    .bind(id.as_bytes().as_slice())
    .bind(skill_name)
    .bind(now)
    .bind(base_version)
    .bind(i64::from(examples_count))
    .execute(pool)
    .await?;
    Ok(id)
}

async fn finish_forge_run(
    pool: &Pool<Sqlite>,
    id: Uuid,
    status: &str,
    final_score: Option<f64>,
    iterations: i64,
) -> Result<u64, sqlx::Error> {
    let res = sqlx::query(
        "UPDATE forge_run SET status = ?, ended_at = ?, final_score = ?, iterations = ? \
         WHERE id = ?",
    )
    .bind(status)
    .bind(Utc::now().timestamp_micros())
    .bind(final_score)
    .bind(iterations)
    .bind(id.as_bytes().as_slice())
    .execute(pool)
    .await?;
    Ok(res.rows_affected())
}
