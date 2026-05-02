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

    /// Enqueue a forge run for the v1.1 daemon to pick up
    /// later. Inserts a `forge_run` row in `'queued'` status
    /// (no `started_at` activity yet) plus a parallel
    /// `forge_queue` row carrying the JSON payload the daemon
    /// needs to actually run the work. Returns the new run id.
    pub async fn enqueue_forge_run(
        &self,
        skill_name: &str,
        base_version: &str,
        examples_count: u32,
        payload_json: &str,
    ) -> StoreResult<Uuid> {
        enqueue_forge_run(
            self.pool(),
            skill_name,
            base_version,
            examples_count,
            payload_json,
        )
        .await
        .map_err(|e| ArgoError::Provider(format!("enqueue_forge_run: {e}")))
    }

    /// Atomically claim the oldest `'queued'` forge_run row,
    /// flip its status to `'running'`, stamp `started_at`, and
    /// return `(run_id, payload_json)`. Returns `None` when no
    /// queued row exists. Race-safe by checking
    /// `rows_affected()` on the UPDATE: if another claimer won,
    /// the SELECT-then-UPDATE retries on the next call.
    pub async fn claim_queued_forge_run(
        &self,
    ) -> StoreResult<Option<(Uuid, String)>> {
        claim_queued_forge_run(self.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("claim_queued_forge_run: {e}")))
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

async fn enqueue_forge_run(
    pool: &Pool<Sqlite>,
    skill_name: &str,
    base_version: &str,
    examples_count: u32,
    payload_json: &str,
) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::now_v7();
    let now = Utc::now().timestamp_micros();
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO forge_run (id, skill_name, started_at, ended_at, base_version, \
         candidate_version, examples_count, iterations, spend_usd_micros, final_score, status) \
         VALUES (?, ?, ?, NULL, ?, NULL, ?, 0, 0, NULL, 'queued')",
    )
    .bind(id.as_bytes().as_slice())
    .bind(skill_name)
    .bind(now)
    .bind(base_version)
    .bind(i64::from(examples_count))
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO forge_queue (forge_run_id, payload_json, enqueued_at) VALUES (?, ?, ?)",
    )
    .bind(id.as_bytes().as_slice())
    .bind(payload_json)
    .bind(now)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(id)
}

async fn claim_queued_forge_run(
    pool: &Pool<Sqlite>,
) -> Result<Option<(Uuid, String)>, sqlx::Error> {
    // Pick the oldest queued row. The forge_queue.enqueued_at
    // is the FIFO key; we don't trust forge_run.started_at for
    // this since enqueue stamps it for "row created", not
    // "execution started".
    let row: Option<(Vec<u8>, String)> = sqlx::query_as(
        "SELECT forge_queue.forge_run_id, forge_queue.payload_json \
         FROM forge_queue \
         JOIN forge_run ON forge_run.id = forge_queue.forge_run_id \
         WHERE forge_run.status = 'queued' \
         ORDER BY forge_queue.enqueued_at ASC LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;
    let Some((id_bytes, payload)) = row else {
        return Ok(None);
    };
    let id = match Uuid::from_slice(&id_bytes) {
        Ok(u) => u,
        Err(_) => return Ok(None),
    };
    let res = sqlx::query(
        "UPDATE forge_run SET status = 'running', started_at = ? \
         WHERE id = ? AND status = 'queued'",
    )
    .bind(Utc::now().timestamp_micros())
    .bind(id.as_bytes().as_slice())
    .execute(pool)
    .await?;
    if res.rows_affected() == 0 {
        // Another worker claimed this row first. Caller can
        // retry on the next tick.
        return Ok(None);
    }
    Ok(Some((id, payload)))
}
