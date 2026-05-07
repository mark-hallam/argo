//! Read-side helpers for chronological skill invocation history.

use crate::{SessionStore, StoreResult};
use argo_types::ArgoError;
use chrono::{DateTime, TimeZone, Utc};
use serde::Serialize;
use serde_json::Value;
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

fn parse_run(body: &str, learned_at: i64) -> StoreResult<Option<SkillRunRow>> {
    let value: Value = serde_json::from_str(body)
        .map_err(|e| ArgoError::Provider(format!("worker action JSON: {e}")))?;
    let Some(action_id) = string_at(&value, &["action_id"]) else {
        return Ok(None);
    };
    let Some(skill_name) = skill_name(&value) else {
        return Ok(None);
    };
    let worker_id = worker_id(&value);
    let tenant_id = string_at(&value, &["payload", "tenant_id"]);
    let created_at = string_at(&value, &["created_at"]);
    let (timestamp_micros, timestamp) = timestamp_pair(created_at.as_deref(), learned_at);
    let state = string_at(&value, &["state"]).unwrap_or_else(|| "skipped".to_string());
    let decision = decision_for_state(&state).to_string();
    let latency_ms = explicit_latency(&value).or_else(|| derived_latency(&value));
    Ok(Some(SkillRunRow {
        skill_name,
        timestamp,
        timestamp_micros,
        worker_id,
        tenant_id,
        decision,
        latency_ms,
        action_id,
        spend_usd_micros: 0,
    }))
}

fn filter_runs(
    mut rows: Vec<SkillRunRow>,
    skill_name: Option<&str>,
    tenant_id: Option<&str>,
    worker_id: Option<&str>,
    since_micros: Option<i64>,
    limit: usize,
    include_failed: bool,
) -> Vec<SkillRunRow> {
    rows.retain(|row| {
        matches_opt(Some(row.skill_name.as_str()), skill_name)
            && matches_opt(row.tenant_id.as_deref(), tenant_id)
            && matches_opt(Some(row.worker_id.as_str()), worker_id)
            && match since_micros {
                Some(cutoff) => row.timestamp_micros >= cutoff,
                None => true,
            }
            && (include_failed || row.decision == "success")
    });
    rows.sort_by(|a, b| {
        b.timestamp_micros
            .cmp(&a.timestamp_micros)
            .then_with(|| a.action_id.cmp(&b.action_id))
    });
    rows.truncate(limit);
    rows
}

fn matches_opt(actual: Option<&str>, want: Option<&str>) -> bool {
    match want {
        Some(want) => actual == Some(want),
        None => true,
    }
}

fn newer_or_equal(prev: &SkillRunRow, next: &SkillRunRow) -> bool {
    prev.timestamp_micros > next.timestamp_micros
        || (prev.timestamp_micros == next.timestamp_micros && prev.action_id <= next.action_id)
}

fn skill_name(value: &Value) -> Option<String> {
    string_at(value, &["payload", "skill_used"])
        .or_else(|| string_at(value, &["payload", "skill"]))
        .or_else(|| string_at(value, &["payload", "skill_name"]))
}

fn worker_id(value: &Value) -> String {
    string_at(value, &["payload", "worker_id"])
        .or_else(|| string_at(value, &["worker_id"]))
        .unwrap_or_else(|| "-".to_string())
}

fn decision_for_state(state: &str) -> &'static str {
    match state {
        "completed" => "success",
        "failed" => "fail",
        _ => "skipped",
    }
}

fn explicit_latency(value: &Value) -> Option<i64> {
    int_at(value, &["payload", "latency_ms"])
        .or_else(|| int_at(value, &["payload", "duration_ms"]))
        .or_else(|| int_at(value, &["result", "latency_ms"]))
        .or_else(|| int_at(value, &["result", "duration_ms"]))
}

fn derived_latency(value: &Value) -> Option<i64> {
    let created = string_at(value, &["created_at"]).and_then(|s| parse_time(&s));
    let executed = string_at(value, &["executed_at"]).and_then(|s| parse_time(&s));
    match (created, executed) {
        (Some(start), Some(end)) => Some((end - start).num_milliseconds().max(0)),
        _ => None,
    }
}

fn timestamp_pair(raw: Option<&str>, learned_at: i64) -> (i64, String) {
    if let Some(parsed) = raw.and_then(parse_time) {
        return (parsed.timestamp_micros(), parsed.to_rfc3339());
    }
    match Utc.timestamp_micros(learned_at).single() {
        Some(ts) => (learned_at, ts.to_rfc3339()),
        None => (learned_at, learned_at.to_string()),
    }
}

fn parse_time(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn string_at(value: &Value, path: &[&str]) -> Option<String> {
    let mut cur = value;
    for key in path {
        cur = cur.get(*key)?;
    }
    match cur {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn int_at(value: &Value, path: &[&str]) -> Option<i64> {
    let mut cur = value;
    for key in path {
        cur = cur.get(*key)?;
    }
    match cur {
        Value::Number(n) => n
            .as_i64()
            .or_else(|| n.as_u64().and_then(|v| i64::try_from(v).ok())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};
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
