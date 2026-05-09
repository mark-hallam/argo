//! `argo budget watch` — emit push alerts when worker spend crosses
//! 80% / 100% of the daily budget cap.
//!
//! SQL schema bootstrap, query helpers, and alert persistence live in
//! the sibling [`budget_watch_store`] module and are re-exported here
//! so existing call sites (and the tests file) are unchanged.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_channels::push::{PushApprovalChannel, PushApprovalChannelConfig};
use argo_channels::token::{sign_approval_token, ApprovalDecision};
use argo_channels::{Notification, NotifyChannel, OperatorTarget, RiskLevel};
use argo_identity::worker_budget;
#[cfg(test)]
pub(crate) use argo_identity::AutoApprovalPolicy;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Duration as ChronoDuration, Local, TimeZone, Utc};
use serde::Serialize;
use std::time::Duration;

#[path = "budget_watch_store.rs"]
mod budget_watch_store;
pub(crate) use budget_watch_store::{
    daily_cap_for_worker, ensure_alert_schema, ensure_worker_budget_schema, has_fired_on_day,
    insert_alert, list_alerts, spend_this_month_for_worker, worker_ids,
};

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

fn crossed_threshold(spend: u64, cap: u64, threshold: u8) -> bool {
    spend.saturating_mul(100) >= cap.saturating_mul(u64::from(threshold))
}

pub(crate) fn alert_day(now: DateTime<Utc>) -> String {
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

pub(crate) fn micros_to_usd(micros: u64) -> f64 {
    micros as f64 / 1_000_000.0
}

fn compact(message: &str) -> String {
    let cleaned = message.replace(['\n', '\r'], " ");
    cleaned.chars().take(120).collect()
}

#[cfg(test)]
#[path = "budget_watch_tests.rs"]
mod tests;
