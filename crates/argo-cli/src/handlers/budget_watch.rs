//! `argo budget watch` — emit push alerts when worker spend crosses
//! 80% / 100% of the daily budget cap.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_channels::push::{PushApprovalChannel, PushApprovalChannelConfig};
use argo_channels::token::{sign_approval_token, ApprovalDecision};
use argo_channels::{Notification, NotifyChannel, OperatorTarget, RiskLevel};
use argo_identity::{worker_budget, AutoApprovalPolicy};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Datelike, Duration as ChronoDuration, Local, TimeZone, Utc};
use serde::Serialize;
use sqlx::Row;
use std::time::Duration;

const ALERT_SCHEMA: &str =
    include_str!("../../../argo-store/migrations/26__worker_budget_alert.sql");
const WORKER_BUDGET_SCHEMA: &str =
    include_str!("../../../argo-store/migrations/14__worker_budget.sql");
const THRESHOLDS: [u8; 2] = [80, 100];

#[derive(Debug, Clone, Serialize)]
pub(crate) struct BudgetAlertRow {
    pub id: String,
    pub worker_id: String,
    pub threshold: u8,
    pub fired_on: String,
    pub fired_at: String,
    pub fired_at_epoch: i64,
    pub spend_amount_usd_micros: u64,
    pub spend_amount_usd: f64,
    pub headroom_amount_usd_micros: u64,
    pub headroom_amount_usd: f64,
    pub notified_via: String,
    pub synthetic: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TickSummary {
    checked_workers: usize,
    fired: Vec<BudgetAlertRow>,
}

#[derive(Debug, Clone)]
pub(crate) struct AlertCandidate {
    pub worker_id: String,
    pub threshold: u8,
    pub spend_amount: u64,
    pub month_spend_amount: u64,
    pub cap_amount: u64,
    pub headroom_amount: u64,
    pub synthetic: bool,
}

pub async fn run(worker: Option<&str>, once: bool, json: bool) -> ArgoResult<()> {
    let store = open_store().await?;
    loop {
        let summary = run_tick(&store, worker, Utc::now()).await?;
        render_tick(&summary, json)?;
        if once {
            return Ok(());
        }
        tokio::time::sleep(watch_interval()).await;
    }
}

pub(crate) async fn open_store() -> ArgoResult<SessionStore> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    ensure_alert_schema(&store).await?;
    ensure_worker_budget_schema(&store).await?;
    Ok(store)
}

pub(crate) async fn run_tick(
    store: &SessionStore,
    worker: Option<&str>,
    now: DateTime<Utc>,
) -> ArgoResult<TickSummary> {
    ensure_alert_schema(store).await?;
    ensure_worker_budget_schema(store).await?;
    let workers = worker_ids(store, worker).await?;
    let day = alert_day(now);
    let mut fired = Vec::new();
    for worker_id in &workers {
        let Some(cap) = daily_cap_for_worker(store, worker_id).await? else {
            continue;
        };
        if cap == 0 {
            continue;
        }
        let spend = worker_budget::spend_today_for_worker(store.pool(), worker_id, now).await?;
        let month = spend_this_month_for_worker(store, worker_id, now).await?;
        for threshold in THRESHOLDS {
            if !crossed_threshold(spend, cap, threshold) {
                continue;
            }
            if has_fired_on_day(store, worker_id, threshold, &day).await? {
                continue;
            }
            let candidate = AlertCandidate {
                worker_id: worker_id.clone(),
                threshold,
                spend_amount: spend,
                month_spend_amount: month,
                cap_amount: cap,
                headroom_amount: cap.saturating_sub(spend),
                synthetic: false,
            };
            fired.push(fire_alert(store, candidate, now).await?);
        }
    }
    Ok(TickSummary {
        checked_workers: workers.len(),
        fired,
    })
}

pub(crate) async fn fire_alert(
    store: &SessionStore,
    candidate: AlertCandidate,
    now: DateTime<Utc>,
) -> ArgoResult<BudgetAlertRow> {
    ensure_alert_schema(store).await?;
    let notified_via = send_push_alert(&candidate, now).await;
    insert_alert(store, &candidate, now, &notified_via).await
}

pub(crate) async fn list_alerts(
    store: &SessionStore,
    worker: Option<&str>,
    since: Option<DateTime<Utc>>,
    limit: u32,
) -> ArgoResult<Vec<BudgetAlertRow>> {
    ensure_alert_schema(store).await?;
    let mut sql = String::from(
        "SELECT id, worker_id, threshold, fired_on, fired_at, spend_amount, \
         headroom_amount, notified_via, synthetic FROM worker_budget_alert",
    );
    let mut clauses = Vec::new();
    if worker.is_some() {
        clauses.push("worker_id = ?");
    }
    if since.is_some() {
        clauses.push("fired_at >= ?");
    }
    if !clauses.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&clauses.join(" AND "));
    }
    sql.push_str(" ORDER BY fired_at DESC LIMIT ?");
    let mut query = sqlx::query(&sql);
    if let Some(worker) = worker {
        query = query.bind(worker);
    }
    if let Some(since) = since {
        query = query.bind(since.timestamp());
    }
    let rows = query
        .bind(i64::from(limit.clamp(1, 500)))
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("budget alert list: {e}")))?;
    rows.into_iter().map(row_from_sql).collect()
}

pub(crate) fn parse_since(raw: Option<&str>) -> ArgoResult<Option<DateTime<Utc>>> {
    let Some(s) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(Some(dt.with_timezone(&Utc)));
    }
    if let Some(lookback) = parse_lookback(s) {
        return Ok(Some(Utc::now() - lookback));
    }
    Err(ArgoError::Provider(format!(
        "invalid --since '{s}' (expected duration like 30m/1h/7d or RFC3339)"
    )))
}

fn parse_lookback(raw: &str) -> Option<ChronoDuration> {
    let (value, unit) = raw.split_at(raw.len().checked_sub(1)?);
    let value = value.parse::<i64>().ok().filter(|v| *v > 0)?;
    match unit {
        "s" => Some(ChronoDuration::seconds(value)),
        "m" => Some(ChronoDuration::minutes(value)),
        "h" => Some(ChronoDuration::hours(value)),
        "d" => Some(ChronoDuration::days(value)),
        _ => None,
    }
}

fn render_tick(summary: &TickSummary, json: bool) -> ArgoResult<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string(summary)
                .map_err(|e| ArgoError::Provider(format!("serialize tick: {e}")))?
        );
    } else if summary.fired.is_empty() {
        println!(
            "budget watch: checked {} worker(s); no threshold crossings",
            summary.checked_workers
        );
    } else {
        for row in &summary.fired {
            println!(
                "budget alert: worker={} threshold={} spend=${:.2} headroom=${:.2} via={}",
                row.worker_id,
                row.threshold,
                row.spend_amount_usd,
                row.headroom_amount_usd,
                row.notified_via
            );
        }
    }
    Ok(())
}

async fn send_push_alert(candidate: &AlertCandidate, now: DateTime<Utc>) -> String {
    let notification = match notification_for(candidate, now) {
        Ok(n) => n,
        Err(e) => return format!("push_failed:{}", compact(&e.to_string())),
    };
    let channel = PushApprovalChannel::new(PushApprovalChannelConfig::from_env());
    match channel.send(notification).await {
        Ok(receipt) => receipt
            .provider_message_id
            .map(|id| format!("push:{id}"))
            .unwrap_or_else(|| "push".to_string()),
        Err(e) => format!("push_failed:{}", compact(&e.to_string())),
    }
}

fn notification_for(candidate: &AlertCandidate, now: DateTime<Utc>) -> ArgoResult<Notification> {
    let secret = std::env::var("ARGO_CHANNEL_HMAC_SECRET")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "budget-alert-local-secret".to_string());
    let action_id = format!(
        "budget-alert-{}-{}",
        candidate.worker_id, candidate.threshold
    );
    let exp = now.timestamp() + 24 * 60 * 60;
    let approve = sign_approval_token(&action_id, ApprovalDecision::Approve, exp, &secret)
        .map_err(|e| ArgoError::Provider(format!("sign push alert token: {e}")))?;
    let reject = sign_approval_token(&action_id, ApprovalDecision::Reject, exp, &secret)
        .map_err(|e| ArgoError::Provider(format!("sign push alert token: {e}")))?;
    let operator = std::env::var("ARGO_BUDGET_ALERT_OPERATOR_ID")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "operator".to_string());
    Ok(Notification {
        target: OperatorTarget {
            operator_id: operator,
            email: "budget-alert@skycot.local".into(),
        },
        action_id: action_id.clone(),
        approval_id: action_id,
        risk_level: RiskLevel::High,
        prompt_body: format!(
            "Worker {} crossed {}% of daily budget: today ${:.2}, cap ${:.2}, month ${:.2}, headroom ${:.2}.",
            candidate.worker_id,
            candidate.threshold,
            micros_to_usd(candidate.spend_amount),
            micros_to_usd(candidate.cap_amount),
            micros_to_usd(candidate.month_spend_amount),
            micros_to_usd(candidate.headroom_amount)
        ),
        approve_url: format!("https://skycot.local/budget/alerts/ack?token={approve}"),
        reject_url: format!("https://skycot.local/budget/alerts/ack?token={reject}"),
        expiry: Utc.timestamp_opt(exp, 0).single().unwrap_or(now),
    })
}

async fn insert_alert(
    store: &SessionStore,
    candidate: &AlertCandidate,
    now: DateTime<Utc>,
    notified_via: &str,
) -> ArgoResult<BudgetAlertRow> {
    let id = uuid::Uuid::now_v7().to_string();
    let day = alert_day(now);
    sqlx::query(
        "INSERT INTO worker_budget_alert \
         (id, worker_id, threshold, fired_on, fired_at, spend_amount, headroom_amount, notified_via, synthetic) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&candidate.worker_id)
    .bind(i64::from(candidate.threshold))
    .bind(&day)
    .bind(now.timestamp())
    .bind(i64::try_from(candidate.spend_amount).unwrap_or(i64::MAX))
    .bind(i64::try_from(candidate.headroom_amount).unwrap_or(i64::MAX))
    .bind(notified_via)
    .bind(if candidate.synthetic { 1_i64 } else { 0_i64 })
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("insert budget alert: {e}")))?;
    Ok(BudgetAlertRow {
        id,
        worker_id: candidate.worker_id.clone(),
        threshold: candidate.threshold,
        fired_on: day,
        fired_at: now.to_rfc3339(),
        fired_at_epoch: now.timestamp(),
        spend_amount_usd_micros: candidate.spend_amount,
        spend_amount_usd: micros_to_usd(candidate.spend_amount),
        headroom_amount_usd_micros: candidate.headroom_amount,
        headroom_amount_usd: micros_to_usd(candidate.headroom_amount),
        notified_via: notified_via.to_string(),
        synthetic: candidate.synthetic,
    })
}

async fn worker_ids(store: &SessionStore, only: Option<&str>) -> ArgoResult<Vec<String>> {
    if let Some(worker) = only {
        return Ok(vec![worker.to_string()]);
    }
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT DISTINCT worker_id FROM worker_budget ORDER BY worker_id")
            .fetch_all(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("budget watch workers: {e}")))?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

async fn daily_cap_for_worker(store: &SessionStore, worker_id: &str) -> ArgoResult<Option<u64>> {
    if !table_exists(store, "worker").await? {
        return Ok(None);
    }
    let row: Option<(String,)> =
        sqlx::query_as("SELECT auto_approval_json FROM worker WHERE id = ? LIMIT 1")
            .bind(worker_id)
            .fetch_optional(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("worker policy lookup: {e}")))?;
    let Some((body,)) = row else {
        return Ok(None);
    };
    let policy: AutoApprovalPolicy = serde_json::from_str(&body)
        .map_err(|e| ArgoError::Provider(format!("worker policy JSON: {e}")))?;
    Ok(policy.daily_budget_usd_micros)
}

async fn spend_this_month_for_worker(
    store: &SessionStore,
    worker_id: &str,
    now: DateTime<Utc>,
) -> ArgoResult<u64> {
    let start = Utc
        .with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
        .single()
        .unwrap_or(now)
        .timestamp();
    let total: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(usd_micros), 0) FROM worker_budget \
         WHERE worker_id = ? AND charged_at >= ?",
    )
    .bind(worker_id)
    .bind(start)
    .fetch_one(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("budget alert month spend: {e}")))?;
    Ok(u64::try_from(total).unwrap_or(0))
}

async fn has_fired_on_day(
    store: &SessionStore,
    worker_id: &str,
    threshold: u8,
    day: &str,
) -> ArgoResult<bool> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM worker_budget_alert \
         WHERE worker_id = ? AND threshold = ? AND fired_on = ? AND synthetic = 0",
    )
    .bind(worker_id)
    .bind(i64::from(threshold))
    .bind(day)
    .fetch_one(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("budget alert dedup: {e}")))?;
    Ok(count > 0)
}

async fn ensure_alert_schema(store: &SessionStore) -> ArgoResult<()> {
    for stmt in ALERT_SCHEMA.split_terminator(';') {
        let sql = stmt.trim();
        if !sql.is_empty() {
            sqlx::query(sql)
                .execute(store.pool())
                .await
                .map_err(|e| ArgoError::Provider(format!("budget alert schema: {e}")))?;
        }
    }
    Ok(())
}

async fn ensure_worker_budget_schema(store: &SessionStore) -> ArgoResult<()> {
    for stmt in WORKER_BUDGET_SCHEMA.split_terminator(';') {
        let sql = stmt.trim();
        if !sql.is_empty() {
            sqlx::query(sql)
                .execute(store.pool())
                .await
                .map_err(|e| ArgoError::Provider(format!("worker budget schema: {e}")))?;
        }
    }
    Ok(())
}

async fn table_exists(store: &SessionStore, table: &str) -> ArgoResult<bool> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?")
            .bind(table)
            .fetch_one(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("table lookup {table}: {e}")))?;
    Ok(count > 0)
}

fn row_from_sql(row: sqlx::sqlite::SqliteRow) -> ArgoResult<BudgetAlertRow> {
    let threshold = u8::try_from(row.get::<i64, _>("threshold")).unwrap_or(100);
    let spend = u64::try_from(row.get::<i64, _>("spend_amount")).unwrap_or(0);
    let headroom = u64::try_from(row.get::<i64, _>("headroom_amount")).unwrap_or(0);
    let fired_at_epoch = row.get::<i64, _>("fired_at");
    Ok(BudgetAlertRow {
        id: row.get("id"),
        worker_id: row.get("worker_id"),
        threshold,
        fired_on: row.get("fired_on"),
        fired_at: Utc
            .timestamp_opt(fired_at_epoch, 0)
            .single()
            .unwrap_or(DateTime::<Utc>::UNIX_EPOCH)
            .to_rfc3339(),
        fired_at_epoch,
        spend_amount_usd_micros: spend,
        spend_amount_usd: micros_to_usd(spend),
        headroom_amount_usd_micros: headroom,
        headroom_amount_usd: micros_to_usd(headroom),
        notified_via: row.get("notified_via"),
        synthetic: row.get::<i64, _>("synthetic") != 0,
    })
}

fn crossed_threshold(spend: u64, cap: u64, threshold: u8) -> bool {
    spend.saturating_mul(100) >= cap.saturating_mul(u64::from(threshold))
}

fn alert_day(now: DateTime<Utc>) -> String {
    now.with_timezone(&Local).date_naive().to_string()
}

fn watch_interval() -> Duration {
    std::env::var("ARGO_BUDGET_WATCH_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_secs(60))
}

fn micros_to_usd(micros: u64) -> f64 {
    micros as f64 / 1_000_000.0
}

fn compact(message: &str) -> String {
    let cleaned = message.replace(['\n', '\r'], " ");
    cleaned.chars().take(120).collect()
}

#[cfg(test)]
#[path = "budget_watch_tests.rs"]
mod tests;
