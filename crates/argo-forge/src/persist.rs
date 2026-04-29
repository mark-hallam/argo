//! `forge_run` table persistence helpers.
use crate::run::{ForgeRun, ForgeStatus};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

/// Insert (or upsert) a `ForgeRun` row.
pub async fn persist_forge_run(store: &SessionStore, run: &ForgeRun) -> ArgoResult<()> {
    sqlx::query(
        "INSERT OR REPLACE INTO forge_run \
         (id, skill_name, started_at, ended_at, base_version, candidate_version, \
          examples_count, iterations, spend_usd_micros, final_score, status) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(run.id.0.as_bytes().as_slice())
    .bind(&run.skill_name)
    .bind(run.started_at.timestamp_micros())
    .bind(run.ended_at.map(|t| t.timestamp_micros()))
    .bind(&run.base_version)
    .bind(run.candidate_version.as_deref())
    .bind(i64::from(run.examples_count))
    .bind(i64::from(run.iterations))
    .bind(i64::try_from(run.spend_usd_micros).unwrap_or(i64::MAX))
    .bind(run.final_score)
    .bind(run.status.label())
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("persist_forge_run: {e}")))?;
    Ok(())
}

/// Test/diagnostic helper: count rows.
pub async fn count_forge_runs(store: &SessionStore) -> ArgoResult<i64> {
    sqlx::query_scalar("SELECT COUNT(*) FROM forge_run")
        .fetch_one(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("count_forge_runs: {e}")))
}

/// Look up a run's status by id-prefix (handy for the CLI).
pub async fn load_forge_status(
    store: &SessionStore,
    run_id: argo_types::ForgeRunId,
) -> ArgoResult<Option<ForgeStatus>> {
    let row: Option<String> = sqlx::query_scalar("SELECT status FROM forge_run WHERE id = ?")
        .bind(run_id.0.as_bytes().as_slice())
        .fetch_optional(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("load_forge_status: {e}")))?;
    Ok(row.and_then(|label| ForgeStatus::from_label(&label)))
}
