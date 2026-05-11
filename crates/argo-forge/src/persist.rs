//! `forge_run` table persistence helpers.
use crate::cost::ForgeCost;
use crate::run::{ForgeRun, ForgeStatus};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;

/// Insert (or upsert) a `ForgeRun` row.
pub async fn persist_forge_run(store: &SessionStore, run: &ForgeRun) -> ArgoResult<()> {
    sqlx::query(
        "INSERT OR REPLACE INTO forge_run \
         (id, skill_name, started_at, ended_at, base_version, candidate_version, \
          examples_count, iterations, spend_usd_micros, final_score, status, \
          llm_input_tokens, llm_output_tokens, llm_cache_read_tokens, \
          llm_cost_usd_micros, e2b_seconds, e2b_cost_usd_micros, total_cost_usd_micros) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
    .bind(i64::try_from(run.cost.llm_input_tokens).unwrap_or(i64::MAX))
    .bind(i64::try_from(run.cost.llm_output_tokens).unwrap_or(i64::MAX))
    .bind(i64::try_from(run.cost.llm_cache_read_tokens).unwrap_or(i64::MAX))
    .bind(i64::try_from(run.cost.llm_cost_usd_micros).unwrap_or(i64::MAX))
    .bind(i64::try_from(run.cost.e2b_seconds).unwrap_or(i64::MAX))
    .bind(i64::try_from(run.cost.e2b_cost_usd_micros).unwrap_or(i64::MAX))
    .bind(i64::try_from(run.cost.total_cost_usd_micros).unwrap_or(i64::MAX))
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

/// Per-skill cost rollup for the `argo forge cost-report` CLI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillCostRow {
    /// Skill name (the `skill_name` column on `forge_run`).
    pub skill_name: String,
    /// Number of runs that contributed to this rollup row.
    pub run_count: i64,
    /// Sum of `total_cost_usd_micros` across matching runs.
    pub total_cost_usd_micros: i64,
    /// Sum of `llm_cost_usd_micros` across matching runs.
    pub llm_cost_usd_micros: i64,
    /// Sum of `e2b_cost_usd_micros` across matching runs.
    pub e2b_cost_usd_micros: i64,
    /// `total_cost_usd_micros / run_count` (integer division).
    pub mean_cost_usd_micros: i64,
    /// Sum of `llm_input_tokens` across matching runs.
    pub llm_input_tokens: i64,
    /// Sum of `llm_output_tokens` across matching runs.
    pub llm_output_tokens: i64,
    /// Sum of `llm_cache_read_tokens` across matching runs.
    pub llm_cache_read_tokens: i64,
}

/// Filter args for [`query_forge_costs`]. All filters AND together;
/// `None` skips that filter. Aggregation is always `GROUP BY skill_name`
/// ordered by `total_cost_usd_micros DESC`.
#[derive(Debug, Clone, Default)]
pub struct CostQueryFilter {
    /// Exact-match skill_name filter.
    pub skill: Option<String>,
    /// Inclusive lower bound on `started_at`.
    pub since: Option<DateTime<Utc>>,
}

/// Aggregate per-skill cost rollups across the `forge_run` table.
/// Returns one row per distinct `skill_name`, sorted by
/// `total_cost_usd_micros DESC`.
pub async fn query_forge_costs(
    store: &SessionStore,
    filter: &CostQueryFilter,
) -> ArgoResult<Vec<SkillCostRow>> {
    // Build the WHERE clause dynamically; sqlx doesn't have a native
    // optional-bind so we conditionally append "AND <pred> = ?" and
    // mirror the binds.
    let mut sql = String::from(
        "SELECT skill_name, \
                COUNT(*) AS n, \
                COALESCE(SUM(total_cost_usd_micros), 0) AS total_cost, \
                COALESCE(SUM(llm_cost_usd_micros), 0) AS llm_cost, \
                COALESCE(SUM(e2b_cost_usd_micros), 0) AS e2b_cost, \
                COALESCE(SUM(llm_input_tokens), 0) AS in_toks, \
                COALESCE(SUM(llm_output_tokens), 0) AS out_toks, \
                COALESCE(SUM(llm_cache_read_tokens), 0) AS cache_toks \
         FROM forge_run \
         WHERE 1 = 1",
    );
    if filter.skill.is_some() {
        sql.push_str(" AND skill_name = ?");
    }
    if filter.since.is_some() {
        sql.push_str(" AND started_at >= ?");
    }
    sql.push_str(" GROUP BY skill_name ORDER BY total_cost DESC");

    let mut q = sqlx::query(&sql);
    if let Some(ref s) = filter.skill {
        q = q.bind(s);
    }
    if let Some(ts) = filter.since {
        q = q.bind(ts.timestamp_micros());
    }
    let rows = q
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("query_forge_costs: {e}")))?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let n: i64 = row.get("n");
        let total: i64 = row.get("total_cost");
        let mean = if n > 0 { total / n } else { 0 };
        out.push(SkillCostRow {
            skill_name: row.get("skill_name"),
            run_count: n,
            total_cost_usd_micros: total,
            llm_cost_usd_micros: row.get("llm_cost"),
            e2b_cost_usd_micros: row.get("e2b_cost"),
            mean_cost_usd_micros: mean,
            llm_input_tokens: row.get("in_toks"),
            llm_output_tokens: row.get("out_toks"),
            llm_cache_read_tokens: row.get("cache_toks"),
        });
    }
    Ok(out)
}

/// Read back one run's cost attribution (handy for testing the
/// round-trip path).
pub async fn load_forge_cost(
    store: &SessionStore,
    run_id: argo_types::ForgeRunId,
) -> ArgoResult<Option<ForgeCost>> {
    let row = sqlx::query(
        "SELECT llm_input_tokens, llm_output_tokens, llm_cache_read_tokens, \
                llm_cost_usd_micros, e2b_seconds, e2b_cost_usd_micros, total_cost_usd_micros \
         FROM forge_run WHERE id = ?",
    )
    .bind(run_id.0.as_bytes().as_slice())
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("load_forge_cost: {e}")))?;
    Ok(row.map(|r| {
        let to_u64 = |v: i64| u64::try_from(v).unwrap_or(0);
        ForgeCost {
            llm_input_tokens: to_u64(r.get::<i64, _>("llm_input_tokens")),
            llm_output_tokens: to_u64(r.get::<i64, _>("llm_output_tokens")),
            llm_cache_read_tokens: to_u64(r.get::<i64, _>("llm_cache_read_tokens")),
            llm_cost_usd_micros: to_u64(r.get::<i64, _>("llm_cost_usd_micros")),
            e2b_seconds: to_u64(r.get::<i64, _>("e2b_seconds")),
            e2b_cost_usd_micros: to_u64(r.get::<i64, _>("e2b_cost_usd_micros")),
            total_cost_usd_micros: to_u64(r.get::<i64, _>("total_cost_usd_micros")),
        }
    }))
}
