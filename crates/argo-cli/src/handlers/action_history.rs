//! `argo action history` plus shared action triage helpers.

use super::worker_actions::ActionRecord;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use sqlx::Row;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ActionView {
    pub(crate) action_id: String,
    pub(crate) worker_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tenant_id: Option<String>,
    pub(crate) created_at: String,
    pub(crate) action_type: String,
    pub(crate) state: String,
    pub(crate) decision: String,
    pub(crate) spend_usd_micros: u64,
    pub(crate) spend_usd: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) skill_used: Option<String>,
}

#[derive(Debug, Serialize)]
struct HistoryOut {
    worker_id: String,
    since: Option<String>,
    limit: usize,
    rows: Vec<ActionView>,
}

pub(crate) async fn run(
    worker_id: &str,
    since: Option<&str>,
    limit: usize,
    include_skill: bool,
    json: bool,
) -> ArgoResult<()> {
    let since_duration = since.map(parse_duration).transpose()?;
    let rows = history_rows(worker_id, since_duration, limit, include_skill, Utc::now()).await?;
    if json {
        let out = HistoryOut {
            worker_id: worker_id.to_string(),
            since: since.map(str::to_string),
            limit,
            rows,
        };
        print_json("action_history", out)
    } else {
        render_history_text(&rows, include_skill);
        Ok(())
    }
}

pub(crate) async fn history_rows(
    worker_id: &str,
    since: Option<Duration>,
    limit: usize,
    include_skill: bool,
    now: DateTime<Utc>,
) -> ArgoResult<Vec<ActionView>> {
    let actions = collect_action_views(include_skill).await?;
    Ok(filter_history(actions, worker_id, since, limit, now))
}

pub(crate) async fn collect_action_views(include_skill: bool) -> ArgoResult<Vec<ActionView>> {
    let store = open_store().await?;
    let spend = spend_by_action(&store).await?;
    let tenants = tenant_by_worker(&store).await?;
    let actions = super::worker_actions_query::collect_latest_actions(None).await?;
    Ok(actions
        .into_iter()
        .map(|action| view_for_action(action, &spend, &tenants, include_skill))
        .collect())
}

pub(crate) fn filter_history(
    mut rows: Vec<ActionView>,
    worker_id: &str,
    since: Option<Duration>,
    limit: usize,
    now: DateTime<Utc>,
) -> Vec<ActionView> {
    let cutoff = since.map(|duration| now - duration);
    rows.retain(|row| row.worker_id == worker_id && passes_since(row, cutoff));
    sort_newest_first(&mut rows);
    rows.truncate(limit);
    rows
}

pub(crate) fn filter_recent(
    mut rows: Vec<ActionView>,
    tenant: Option<&str>,
    worker: Option<&str>,
    limit: usize,
) -> Vec<ActionView> {
    rows.retain(|row| {
        tenant.is_none_or(|want| row.tenant_id.as_deref() == Some(want))
            && worker.is_none_or(|want| row.worker_id == want)
    });
    sort_newest_first(&mut rows);
    rows.truncate(limit);
    rows
}

pub(crate) fn parse_duration(raw: &str) -> ArgoResult<Duration> {
    let raw = raw.trim();
    if raw.len() < 2 {
        return Err(ArgoError::Schema(format!("invalid duration '{raw}'")));
    }
    let (digits, unit) = raw.split_at(raw.len() - 1);
    let value = digits
        .parse::<i64>()
        .map_err(|e| ArgoError::Schema(format!("invalid duration '{raw}': {e}")))?;
    if value <= 0 {
        return Err(ArgoError::Provider(format!(
            "duration must be positive: {raw}"
        )));
    }
    match unit {
        "s" => Ok(Duration::seconds(value)),
        "m" => Ok(Duration::minutes(value)),
        "h" => Ok(Duration::hours(value)),
        "d" => Ok(Duration::days(value)),
        _ => Err(ArgoError::Provider(format!(
            "unsupported duration suffix '{unit}' in '{raw}'"
        ))),
    }
}

pub(crate) fn action_micros(row: &ActionView) -> i64 {
    parse_rfc3339_micros(&row.created_at).unwrap_or_default()
}

pub(crate) fn print_json<T: Serialize>(key: &str, value: T) -> ArgoResult<()> {
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({"ok": true, key: value}))
            .map_err(|e| ArgoError::Provider(format!("{key} JSON: {e}")))?
    );
    Ok(())
}

fn view_for_action(
    action: ActionRecord,
    spend: &HashMap<String, u64>,
    tenants: &HashMap<String, String>,
    include_skill: bool,
) -> ActionView {
    let worker_id = worker_id_for_action(&action);
    let tenant_id = tenant_id_for_action(&action).or_else(|| tenants.get(&worker_id).cloned());
    let spend_usd_micros = spend
        .get(&action.action_id)
        .copied()
        .or_else(|| action.payload.get("spend_usd_micros").and_then(|v| v.as_u64()))
        .unwrap_or(0);
    let decision = action
        .payload
        .get("decision")
        .and_then(|v| v.as_str())
        .unwrap_or(&action.state)
        .to_string();
    let skill_used = include_skill.then(|| skill_for_action(&action)).flatten();
    ActionView {
        action_id: action.action_id,
        worker_id,
        tenant_id,
        created_at: action.created_at,
        action_type: action.action_type,
        state: action.state.clone(),
        decision,
        spend_usd_micros,
        spend_usd: micros_to_usd(spend_usd_micros),
        skill_used,
    }
}

fn worker_id_for_action(action: &ActionRecord) -> String {
    action
        .payload
        .get("worker_id")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| action.worker_id.to_string())
}

fn tenant_id_for_action(action: &ActionRecord) -> Option<String> {
    action
        .payload
        .get("tenant_id")
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

fn skill_for_action(action: &ActionRecord) -> Option<String> {
    action
        .payload
        .get("skill_used")
        .or_else(|| action.payload.get("skill"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

fn passes_since(row: &ActionView, cutoff: Option<DateTime<Utc>>) -> bool {
    cutoff
        .and_then(|cutoff| parse_rfc3339_micros(&row.created_at).map(|ts| ts >= cutoff.timestamp_micros()))
        .unwrap_or(true)
}

fn sort_newest_first(rows: &mut [ActionView]) {
    rows.sort_by(|a, b| {
        action_micros(b)
            .cmp(&action_micros(a))
            .then_with(|| a.action_id.cmp(&b.action_id))
    });
}

fn parse_rfc3339_micros(raw: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.with_timezone(&Utc).timestamp_micros())
}

fn render_history_text(rows: &[ActionView], include_skill: bool) {
    if rows.is_empty() {
        println!("(no actions)");
        return;
    }
    if include_skill {
        println!("created_at                 worker        action_id       state       type                    decision    spend     skill");
    } else {
        println!("created_at                 worker        action_id       state       type                    decision    spend");
    }
    for row in rows {
        if include_skill {
            println!(
                "{:<26} {:<13} {:<15} {:<11} {:<23} {:<11} ${:<8.4} {}",
                row.created_at,
                row.worker_id,
                short(&row.action_id),
                row.state,
                row.action_type,
                row.decision,
                row.spend_usd,
                row.skill_used.as_deref().unwrap_or("-")
            );
        } else {
            println!(
                "{:<26} {:<13} {:<15} {:<11} {:<23} {:<11} ${:<8.4}",
                row.created_at,
                row.worker_id,
                short(&row.action_id),
                row.state,
                row.action_type,
                row.decision,
                row.spend_usd
            );
        }
    }
}

fn short(id: &str) -> String {
    if id.len() > 14 {
        format!("{}...", &id[..11])
    } else {
        id.to_string()
    }
}

fn micros_to_usd(value: u64) -> f64 {
    value as f64 / 1_000_000.0
}

async fn open_store() -> ArgoResult<SessionStore> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}

async fn spend_by_action(store: &SessionStore) -> ArgoResult<HashMap<String, u64>> {
    if !table_exists(store, "worker_budget").await? {
        return Ok(HashMap::new());
    }
    let rows = sqlx::query(
        "SELECT action_id, COALESCE(SUM(usd_micros), 0) AS total \
         FROM worker_budget GROUP BY action_id",
    )
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("action spend query: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let total = u64::try_from(row.get::<i64, _>("total")).unwrap_or(0);
            (row.get("action_id"), total)
        })
        .collect())
}

async fn tenant_by_worker(store: &SessionStore) -> ArgoResult<HashMap<String, String>> {
    if !table_exists(store, "worker").await? {
        return Ok(HashMap::new());
    }
    let rows = sqlx::query("SELECT id, tenant_id FROM worker")
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("action tenant query: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|row| (row.get("id"), row.get("tenant_id")))
        .collect())
}

async fn table_exists(store: &SessionStore, table: &str) -> ArgoResult<bool> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?",
    )
    .bind(table)
    .fetch_one(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("table lookup {table}: {e}")))?;
    Ok(count > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn row(id: &str, worker: &str, tenant: &str, minutes_ago: i64) -> ActionView {
        let now = Utc.with_ymd_and_hms(2026, 5, 7, 12, 0, 0).unwrap();
        ActionView {
            action_id: id.to_string(),
            worker_id: worker.to_string(),
            tenant_id: Some(tenant.to_string()),
            created_at: (now - Duration::minutes(minutes_ago)).to_rfc3339(),
            action_type: "post_listing".to_string(),
            state: "completed".to_string(),
            decision: "completed".to_string(),
            spend_usd_micros: 500_000,
            spend_usd: 0.5,
            skill_used: Some("pricing".to_string()),
        }
    }

    #[test]
    fn parse_duration_accepts_common_suffixes() {
        assert_eq!(parse_duration("30m").unwrap(), Duration::minutes(30));
        assert_eq!(parse_duration("2h").unwrap(), Duration::hours(2));
        assert_eq!(parse_duration("1d").unwrap(), Duration::days(1));
    }

    #[test]
    fn history_filters_worker_orders_newest_and_limits() {
        let now = Utc.with_ymd_and_hms(2026, 5, 7, 12, 0, 0).unwrap();
        let rows = vec![
            row("old", "w-1", "tenant-a", 20),
            row("new", "w-1", "tenant-a", 1),
            row("other", "w-2", "tenant-a", 0),
        ];

        let out = filter_history(rows, "w-1", None, 1, now);

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].action_id, "new");
    }

    #[test]
    fn since_filter_drops_rows_before_cutoff() {
        let now = Utc.with_ymd_and_hms(2026, 5, 7, 12, 0, 0).unwrap();
        let rows = vec![row("fresh", "w-1", "tenant-a", 10), row("old", "w-1", "tenant-a", 70)];

        let out = filter_history(rows, "w-1", Some(Duration::hours(1)), 10, now);

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].action_id, "fresh");
    }

    #[test]
    fn recent_filters_tenant_and_worker() {
        let rows = vec![
            row("a", "w-1", "tenant-a", 1),
            row("b", "w-2", "tenant-a", 2),
            row("c", "w-1", "tenant-b", 3),
        ];

        let out = filter_recent(rows, Some("tenant-a"), Some("w-1"), 50);

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].action_id, "a");
    }
}
