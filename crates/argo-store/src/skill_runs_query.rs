//! Read-side helpers for chronological skill invocation history.

use crate::skill_runs_query_decode::{filter_runs, newer_or_equal, parse_run};
use crate::{SessionStore, StoreResult};
use argo_types::ArgoError;
use serde::Serialize;
use sqlx::Row;
use std::collections::HashMap;

/// One latest worker-action snapshot representing a skill invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SkillRunRow {
    /// Skill name recorded on the worker action payload.
    pub skill_name: String,
    /// Invocation timestamp in UTC, formatted as RFC3339.
    pub timestamp: String,
    /// Invocation timestamp in UTC epoch microseconds.
    pub timestamp_micros: i64,
    /// Worker that invoked the skill.
    pub worker_id: String,
    /// Tenant resolved from the action payload or worker table.
    pub tenant_id: Option<String>,
    /// Normalized decision: `success`, `fail`, or `skipped`.
    pub decision: String,
    /// Action latency when recorded on the action payload/result.
    pub latency_ms: Option<i64>,
    /// Worker action identifier.
    pub action_id: String,
    /// Spend joined from `worker_budget`, in USD micros.
    pub spend_usd_micros: u64,
}

/// Return newest-first runs for a single skill.
///
/// # Examples
///
/// ```no_run
/// # async fn demo(store: &argo_store::SessionStore) -> argo_store::StoreResult<()> {
/// let rows = argo_store::runs_for_skill(store, "my-skill", None, None, 10, false).await?;
/// assert!(rows.len() <= 10);
/// # Ok(()) }
/// ```
pub async fn runs_for_skill(
    store: &SessionStore,
    skill_name: &str,
    worker_id: Option<&str>,
    since_micros: Option<i64>,
    limit: usize,
    include_failed: bool,
) -> StoreResult<Vec<SkillRunRow>> {
    let rows = collect_latest_runs(store).await?;
    Ok(filter_runs(
        rows,
        Some(skill_name),
        None,
        worker_id,
        since_micros,
        limit,
        include_failed,
    ))
}

/// Return newest-first skill runs for every skill under one tenant.
///
/// # Examples
///
/// ```no_run
/// # async fn demo(store: &argo_store::SessionStore) -> argo_store::StoreResult<()> {
/// let rows = argo_store::runs_for_tenant(store, "tenant-a", None, 50, true).await?;
/// assert!(rows.iter().all(|row| row.tenant_id.as_deref() == Some("tenant-a")));
/// # Ok(()) }
/// ```
pub async fn runs_for_tenant(
    store: &SessionStore,
    tenant_id: &str,
    since_micros: Option<i64>,
    limit: usize,
    include_failed: bool,
) -> StoreResult<Vec<SkillRunRow>> {
    let rows = collect_latest_runs(store).await?;
    Ok(filter_runs(
        rows,
        None,
        Some(tenant_id),
        None,
        since_micros,
        limit,
        include_failed,
    ))
}

async fn collect_latest_runs(store: &SessionStore) -> StoreResult<Vec<SkillRunRow>> {
    let budgets = budget_by_action(store).await?;
    let tenants = tenant_by_worker(store).await?;
    let rows = sqlx::query(
        "SELECT m.body, m.learned_at \
         FROM memory_entry m \
         WHERE m.file = 'MEMORY.md' \
           AND m.superseded_by IS NULL \
           AND CASE WHEN json_valid(m.body) \
                THEN json_extract(m.body, '$.schema_key') ELSE NULL END = 'worker_action'",
    )
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("skill runs query: {e}")))?;

    let mut latest = HashMap::<String, SkillRunRow>::new();
    for row in rows {
        let body: String = row.get("body");
        let learned_at: i64 = row.get("learned_at");
        let Some(mut run) = parse_run(&body, learned_at)? else {
            continue;
        };
        if run.tenant_id.is_none() {
            run.tenant_id = tenants.get(&run.worker_id).cloned();
        }
        run.spend_usd_micros = budgets.get(&run.action_id).copied().unwrap_or(0);
        match latest.get(&run.action_id) {
            Some(prev) if newer_or_equal(prev, &run) => {}
            _ => {
                latest.insert(run.action_id.clone(), run);
            }
        }
    }
    Ok(latest.into_values().collect())
}

async fn budget_by_action(store: &SessionStore) -> StoreResult<HashMap<String, u64>> {
    if !table_exists(store, "worker_budget").await? {
        return Ok(HashMap::new());
    }
    let rows = sqlx::query(
        "SELECT action_id, COALESCE(SUM(usd_micros), 0) AS spend_usd_micros \
         FROM worker_budget GROUP BY action_id",
    )
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("skill runs budget query: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let action_id: String = row.get("action_id");
            let spend = u64::try_from(row.get::<i64, _>("spend_usd_micros")).unwrap_or(0);
            (action_id, spend)
        })
        .collect())
}

async fn tenant_by_worker(store: &SessionStore) -> StoreResult<HashMap<String, String>> {
    if !table_exists(store, "worker").await? {
        return Ok(HashMap::new());
    }
    let rows = sqlx::query("SELECT id, tenant_id FROM worker")
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("skill runs worker query: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|row| (row.get("id"), row.get("tenant_id")))
        .collect())
}

async fn table_exists(store: &SessionStore, table: &str) -> StoreResult<bool> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?")
            .bind(table)
            .fetch_one(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("table lookup {table}: {e}")))?;
    Ok(count > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Duration, TimeZone, Utc};
    use serde_json::json;
    use uuid::Uuid;

    #[tokio::test]
    async fn skill_runs_limit_newest_first() {
        let (store, _workspace) = setup().await;
        let now = fixed_now();
        for i in 0..12 {
            insert_action(&store, "my-skill", "tenant-a", "w-1", "completed", i, now).await;
        }
        insert_action(
            &store,
            "other-skill",
            "tenant-a",
            "w-1",
            "completed",
            0,
            now,
        )
        .await;

        let rows = runs_for_skill(&store, "my-skill", None, None, 10, false)
            .await
            .unwrap();

        assert_eq!(rows.len(), 10);
        assert_eq!(rows[0].action_id, "my-skill-00");
        assert_eq!(rows[9].action_id, "my-skill-09");
    }

    #[tokio::test]
    async fn since_filters_by_timestamp() {
        let (store, _workspace) = setup().await;
        let now = fixed_now();
        insert_action(&store, "my-skill", "tenant-a", "w-1", "completed", 30, now).await;
        insert_action(&store, "my-skill", "tenant-a", "w-1", "completed", 120, now).await;

        let cutoff = (now - Duration::hours(1)).timestamp_micros();
        let rows = runs_for_skill(&store, "my-skill", None, Some(cutoff), 10, false)
            .await
            .unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].action_id, "my-skill-30");
    }

    #[tokio::test]
    async fn include_failed_controls_fail_and_skipped_rows() {
        let (store, _workspace) = setup().await;
        let now = fixed_now();
        insert_action(&store, "my-skill", "tenant-a", "w-1", "completed", 0, now).await;
        insert_action(&store, "my-skill", "tenant-a", "w-1", "failed", 1, now).await;
        insert_action(&store, "my-skill", "tenant-a", "w-1", "blocked", 2, now).await;

        let success = runs_for_skill(&store, "my-skill", None, None, 10, false)
            .await
            .unwrap();
        let all = runs_for_skill(&store, "my-skill", None, None, 10, true)
            .await
            .unwrap();

        assert_eq!(success.len(), 1);
        assert_eq!(all.len(), 3);
        assert!(all.iter().any(|row| row.decision == "fail"));
        assert!(all.iter().any(|row| row.decision == "skipped"));
    }

    #[tokio::test]
    async fn worker_filter_matches_single_worker() {
        let (store, _workspace) = setup().await;
        let now = fixed_now();
        insert_action(&store, "my-skill", "tenant-a", "w-1", "completed", 0, now).await;
        insert_action(&store, "my-skill", "tenant-a", "w-2", "completed", 1, now).await;

        let rows = runs_for_skill(&store, "my-skill", Some("w-2"), None, 10, false)
            .await
            .unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].worker_id, "w-2");
    }

    #[tokio::test]
    async fn tenant_query_returns_cross_skill_rows() {
        let (store, _workspace) = setup().await;
        let now = fixed_now();
        insert_action(&store, "alpha", "tenant-a", "w-1", "completed", 0, now).await;
        insert_action(&store, "beta", "tenant-a", "w-2", "completed", 1, now).await;
        insert_action(&store, "gamma", "tenant-b", "w-3", "completed", 2, now).await;

        let rows = runs_for_tenant(&store, "tenant-a", None, 50, false)
            .await
            .unwrap();

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].skill_name, "alpha");
        assert_eq!(rows[1].skill_name, "beta");
    }

    async fn setup() -> (SessionStore, argo_types::WorkspaceId) {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        let (_tenant, workspace) = store.ensure_workspace("test", "/tmp/test").await.unwrap();
        (store, workspace)
    }

    async fn insert_action(
        store: &SessionStore,
        skill: &str,
        tenant: &str,
        worker: &str,
        state: &str,
        minutes_ago: i64,
        now: DateTime<Utc>,
    ) {
        let (_tenant, workspace) = store.ensure_workspace("test", "/tmp/test").await.unwrap();
        let created_at = now - Duration::minutes(minutes_ago);
        let body = json!({
            "schema_key": "worker_action",
            "action_id": format!("{skill}-{minutes_ago:02}"),
            "worker_id": worker,
            "state": state,
            "payload": {
                "skill_used": skill,
                "worker_id": worker,
                "tenant_id": tenant,
                "latency_ms": 125
            },
            "created_at": created_at.to_rfc3339(),
        })
        .to_string();
        let hash = [7_u8; 32];
        sqlx::query(
            "INSERT INTO memory_entry (id, workspace_id, file, body, learned_in, learned_at, hash) \
             VALUES (?, ?, 'MEMORY.md', ?, NULL, ?, ?)",
        )
        .bind(Uuid::now_v7().as_bytes().as_slice())
        .bind(workspace.0.as_bytes().as_slice())
        .bind(body)
        .bind(created_at.timestamp_micros())
        .bind(hash.as_slice())
        .execute(store.pool())
        .await
        .unwrap();
    }

    fn fixed_now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 7, 12, 0, 0).unwrap()
    }
}
