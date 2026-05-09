//! Show-command pipeline for `argo budget show`: data assembly, supporting
//! queries, and the human-readable text renderers. Carries the shared
//! formatting helpers (`micros_to_usd`, `format_usd`) and the sibling
//! `render_recent_text` for `argo budget recent` so all human rendering and
//! data-shaping helpers live in one place. The parent dispatcher
//! (`budget.rs`) stays focused on routing.
use super::budget::ensure_budget_schema;
use argo_identity::{worker_budget, AutoApprovalPolicy};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Datelike, TimeZone, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct BudgetSummary {
    pub(crate) worker_id: String,
    pub(crate) today_usd_micros: u64,
    pub(crate) this_month_usd_micros: u64,
    pub(crate) today_usd: f64,
    pub(crate) this_month_usd: f64,
    pub(crate) daily_cap_usd_micros: Option<u64>,
    pub(crate) monthly_cap_usd_micros: Option<u64>,
    pub(crate) remaining_today_usd: Option<f64>,
    pub(crate) remaining_month_usd: Option<f64>,
    pub(crate) last_charges: Vec<ChargeRow>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ChargeRow {
    pub(crate) worker_id: String,
    pub(crate) action_id: String,
    pub(crate) usd_micros: u64,
    pub(crate) usd: f64,
    pub(crate) charged_at: String,
    pub(crate) charged_at_epoch: i64,
}

pub(crate) async fn budget_summary(
    store: &SessionStore,
    worker_id: &str,
    now: DateTime<Utc>,
) -> ArgoResult<BudgetSummary> {
    let pool = store.pool();
    let today = worker_budget::spend_today_for_worker(pool, worker_id, now).await?;
    let month = spend_this_month_for_worker(store, worker_id, now).await?;
    let policy = worker_policy(store, worker_id).await?;
    let daily_cap = policy.as_ref().and_then(|p| p.daily_budget_usd_micros);
    let last_charges = super::budget::recent_charges(store, 5, Some(worker_id), None).await?;
    Ok(BudgetSummary {
        worker_id: worker_id.to_string(),
        today_usd_micros: today,
        this_month_usd_micros: month,
        today_usd: micros_to_usd(today),
        this_month_usd: micros_to_usd(month),
        daily_cap_usd_micros: daily_cap,
        monthly_cap_usd_micros: None,
        remaining_today_usd: daily_cap.map(|cap| micros_to_usd(cap.saturating_sub(today))),
        remaining_month_usd: None,
        last_charges,
    })
}

async fn spend_this_month_for_worker(
    store: &SessionStore,
    worker_id: &str,
    now: DateTime<Utc>,
) -> ArgoResult<u64> {
    ensure_budget_schema(store).await?;
    let start = month_start_utc(now).timestamp();
    let total: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(usd_micros), 0) FROM worker_budget \
         WHERE worker_id = ? AND charged_at >= ?",
    )
    .bind(worker_id)
    .bind(start)
    .fetch_one(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("worker budget spend this month: {e}")))?;
    Ok(u64::try_from(total).unwrap_or(0))
}

async fn worker_policy(
    store: &SessionStore,
    worker_id: &str,
) -> ArgoResult<Option<AutoApprovalPolicy>> {
    if !table_exists(store, "worker").await? {
        return Ok(None);
    }
    let row: Option<(String,)> =
        sqlx::query_as("SELECT auto_approval_json FROM worker WHERE id = ? LIMIT 1")
            .bind(worker_id)
            .fetch_optional(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("worker policy lookup: {e}")))?;
    row.map(|(body,)| {
        serde_json::from_str(&body)
            .map_err(|e| ArgoError::Provider(format!("worker policy JSON: {e}")))
    })
    .transpose()
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

fn month_start_utc(now: DateTime<Utc>) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
        .single()
        .unwrap_or(now)
}

pub(crate) fn micros_to_usd(micros: u64) -> f64 {
    micros as f64 / 1_000_000.0
}

fn format_usd(micros: u64) -> String {
    let usd = micros_to_usd(micros);
    if micros > 0 && usd < 0.01 {
        format!("{usd:.4}")
    } else {
        format!("{usd:.2}")
    }
}

pub(crate) fn render_recent_text(
    rows: &[ChargeRow],
    worker: Option<&str>,
    since: Option<DateTime<Utc>>,
) -> String {
    if rows.is_empty() {
        return "(no worker budget charges)\n".to_string();
    }
    let mut out = format!("budget recent ({} ledger rows", rows.len());
    if let Some(worker) = worker {
        out.push_str(&format!(", worker={worker}"));
    }
    if let Some(since) = since {
        out.push_str(&format!(", since={}", since.to_rfc3339()));
    }
    out.push_str("):\n");
    for row in rows {
        out.push_str(&format!(
            "  {}  ${:>10}  worker={}  action={}\n",
            row.charged_at,
            format_usd(row.usd_micros),
            row.worker_id,
            row.action_id
        ));
    }
    out
}

pub(crate) fn render_summary_text(summary: &BudgetSummary) -> String {
    let mut out = format!("budget show worker={}\n", summary.worker_id);
    out.push_str(&format!(
        "  today:      ${:>10}\n",
        format_usd(summary.today_usd_micros)
    ));
    out.push_str(&format!(
        "  this month: ${:>10}\n",
        format_usd(summary.this_month_usd_micros)
    ));
    match summary.daily_cap_usd_micros {
        Some(cap) => out.push_str(&format!(
            "  remaining today: ${:>10}\n",
            format_usd(cap.saturating_sub(summary.today_usd_micros))
        )),
        None => out.push_str("  remaining today: (unlimited)\n"),
    }
    out.push_str("  remaining month: (unlimited)\n");
    out.push_str("last charges:\n");
    if summary.last_charges.is_empty() {
        out.push_str("  (none)\n");
    } else {
        for row in &summary.last_charges {
            out.push_str(&format!(
                "  {}  ${:>10}  action={}\n",
                row.charged_at,
                format_usd(row.usd_micros),
                row.action_id
            ));
        }
    }
    out
}
