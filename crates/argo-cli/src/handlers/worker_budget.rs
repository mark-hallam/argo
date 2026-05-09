//! `argo worker budget set/show` — operator-facing CLI for per-worker
//! spend caps (Stream `claude-worker-budget-cli`).
//!
//! Caps are persisted onto the worker's `auto_approval_json` policy
//! column via [`argo_store::worker_budget_write`]; period-to-date
//! spend rolls up the existing `worker_budget` ledger (the table the
//! dispatch path already populates on every charge).

use crate::cli::cli_worker_budget_cmds::WorkerBudgetCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::worker_budget_write::{self, WorkerBudgetSummary};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::Utc;

/// Dispatch `argo worker budget …` invocations.
pub async fn run(cmd: WorkerBudgetCmd) -> ArgoResult<()> {
    match cmd {
        WorkerBudgetCmd::Set {
            worker_id,
            daily,
            monthly,
            clear,
            json,
        } => set(&worker_id, daily.as_deref(), monthly.as_deref(), clear, json).await,
        WorkerBudgetCmd::Show { worker_id, json } => show(&worker_id, json).await,
    }
}

async fn set(
    worker_id: &str,
    daily: Option<&str>,
    monthly: Option<&str>,
    clear: bool,
    json: bool,
) -> ArgoResult<()> {
    if clear && (daily.is_some() || monthly.is_some()) {
        return Err(ArgoError::Provider(
            "--clear cannot be combined with --daily or --monthly".into(),
        ));
    }
    if !clear && daily.is_none() && monthly.is_none() {
        return Err(ArgoError::Provider(
            "provide --daily, --monthly, or --clear".into(),
        ));
    }
    let store = open_store().await?;
    if clear {
        worker_budget_write::clear_cap(&store, worker_id).await?;
    } else {
        let daily_micros = daily.map(parse_usd).transpose()?;
        let monthly_micros = monthly.map(parse_usd).transpose()?;
        worker_budget_write::set_cap(&store, worker_id, daily_micros, monthly_micros).await?;
    }
    let summary = worker_budget_write::summary(&store, worker_id, Utc::now()).await?;
    print_summary(&summary, true, json);
    Ok(())
}

async fn show(worker_id: &str, json: bool) -> ArgoResult<()> {
    let store = open_store().await?;
    let summary = worker_budget_write::summary(&store, worker_id, Utc::now()).await?;
    print_summary(&summary, false, json);
    Ok(())
}

async fn open_store() -> ArgoResult<SessionStore> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}

fn print_summary(summary: &WorkerBudgetSummary, just_set: bool, json: bool) {
    if json {
        let payload = serde_json::json!({
            "ok": true,
            "worker_id": summary.worker_id,
            "caps": summary.caps,
            "today_micros": summary.today_micros,
            "this_month_micros": summary.this_month_micros,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
        return;
    }
    let header = if just_set { "updated" } else { "current" };
    println!("worker {} budget ({header}):", summary.worker_id);
    println!("  daily   cap: {}", render_cap(summary.caps.daily_micros));
    println!("  monthly cap: {}", render_cap(summary.caps.monthly_micros));
    println!(
        "  today  spent: {}",
        render_micros(summary.today_micros)
    );
    println!(
        "  month  spent: {}",
        render_micros(summary.this_month_micros)
    );
}

fn render_cap(value: Option<u64>) -> String {
    match value {
        None => "(unset — tenant default applies)".into(),
        Some(0) => "$0.00 (deny all)".into(),
        Some(v) => render_micros(v),
    }
}

fn render_micros(micros: u64) -> String {
    let dollars = micros / 1_000_000;
    let cents = (micros % 1_000_000) / 10_000;
    format!("${dollars}.{cents:02}")
}

/// Parse a positive USD amount with up to six fractional digits into
/// integer USD micros. Rejects negatives, scientific notation, leading
/// `+`, or anything ambiguous. `0` is allowed (means "deny all").
pub(crate) fn parse_usd(raw: &str) -> ArgoResult<u64> {
    let s = raw.trim();
    if s.is_empty() {
        return Err(invalid_usd(raw, "empty"));
    }
    if s.starts_with('-') {
        return Err(invalid_usd(raw, "negative values are not allowed"));
    }
    if s.starts_with('+') {
        return Err(invalid_usd(raw, "leading '+' is not allowed"));
    }
    if s.contains(['e', 'E']) {
        return Err(invalid_usd(raw, "scientific notation is not allowed"));
    }
    let (whole, frac) = match s.split_once('.') {
        Some((w, f)) => (w, f),
        None => (s, ""),
    };
    if whole.is_empty() && frac.is_empty() {
        return Err(invalid_usd(raw, "no digits"));
    }
    if !whole.chars().all(|c| c.is_ascii_digit()) {
        return Err(invalid_usd(raw, "non-digit before decimal"));
    }
    if !frac.chars().all(|c| c.is_ascii_digit()) {
        return Err(invalid_usd(raw, "non-digit after decimal"));
    }
    if frac.len() > 6 {
        return Err(invalid_usd(raw, "more than 6 fractional digits"));
    }
    let whole_part: u64 = if whole.is_empty() {
        0
    } else {
        whole
            .parse::<u64>()
            .map_err(|_| invalid_usd(raw, "whole part out of u64 range"))?
    };
    let micros_per_dollar: u64 = 1_000_000;
    let mut padded = String::from(frac);
    while padded.len() < 6 {
        padded.push('0');
    }
    let frac_part: u64 = if padded.is_empty() {
        0
    } else {
        padded
            .parse::<u64>()
            .map_err(|_| invalid_usd(raw, "fractional part out of range"))?
    };
    whole_part
        .checked_mul(micros_per_dollar)
        .and_then(|w| w.checked_add(frac_part))
        .ok_or_else(|| invalid_usd(raw, "total micros out of u64 range"))
}

fn invalid_usd(raw: &str, why: &str) -> ArgoError {
    ArgoError::Provider(format!("'{raw}' is not a valid USD amount: {why}"))
}

#[cfg(test)]
#[path = "worker_budget_tests.rs"]
mod tests;
