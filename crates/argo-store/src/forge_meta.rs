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

    /// Update a *running* `forge_run` row's mid-loop progress —
    /// iter count, best-so-far score, and the candidate-version
    /// string for the most recent iteration. Status is not
    /// touched; the row stays `'running'` until `finish_forge_run`
    /// flips it. The TUI's `F`-overlay auto-refresh shows these
    /// values move while the loop is in flight.
    pub async fn update_forge_run_progress(
        &self,
        id: Uuid,
        iterations: i64,
        best_score: Option<f64>,
        candidate_version: Option<&str>,
    ) -> StoreResult<u64> {
        update_forge_run_progress(
            self.pool(),
            id,
            iterations,
            best_score,
            candidate_version,
        )
        .await
        .map_err(|e| ArgoError::Provider(format!("update_forge_run_progress: {e}")))
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

    /// Sweep stalled forge runs — flip every `running` row whose
    /// `started_at` is older than `cutoff_micros` to `'failed'`
    /// with a `null` final_score. Used to recover from a gateway
    /// crash mid-optimize: in-memory task state is gone, but
    /// the row is still in `running` status until manually
    /// aborted. Returns rows affected.
    ///
    /// Caller picks the cutoff (e.g. now - 1h). The gateway has
    /// no notion of "what's still alive" once it restarts, so
    /// this is the only safe way to clean up.
    pub async fn sweep_stalled_forge_runs(
        &self,
        cutoff_micros: i64,
    ) -> StoreResult<u64> {
        sweep_stalled_forge_runs(self.pool(), cutoff_micros).await
            .map_err(|e| ArgoError::Provider(format!("sweep_stalled_forge_runs: {e}")))
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

async fn update_forge_run_progress(
    pool: &Pool<Sqlite>,
    id: Uuid,
    iterations: i64,
    best_score: Option<f64>,
    candidate_version: Option<&str>,
) -> Result<u64, sqlx::Error> {
    let res = sqlx::query(
        "UPDATE forge_run SET iterations = ?, final_score = ?, candidate_version = ? \
         WHERE id = ? AND status = 'running'",
    )
    .bind(iterations)
    .bind(best_score)
    .bind(candidate_version)
    .bind(id.as_bytes().as_slice())
    .execute(pool)
    .await?;
    Ok(res.rows_affected())
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

async fn sweep_stalled_forge_runs(
    pool: &Pool<Sqlite>,
    cutoff_micros: i64,
) -> Result<u64, sqlx::Error> {
    let res = sqlx::query(
        "UPDATE forge_run SET status = 'failed', ended_at = ? \
         WHERE status = 'running' AND started_at < ?",
    )
    .bind(Utc::now().timestamp_micros())
    .bind(cutoff_micros)
    .execute(pool)
    .await?;
    Ok(res.rows_affected())
}
