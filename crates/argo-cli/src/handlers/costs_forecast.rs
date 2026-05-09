//! `argo costs forecast --horizon eod|eom|eoq` — linear extrapolation
//! of end-of-period spend from a recent rate.
//!
//! Projects today / month / quarter total spend from current
//! trajectory: `rate = sum(spend in lookback) / lookback_seconds`
//! and `projected_additional = rate × time-remaining`. A flat ±20%
//! band is a deliberate v1 heuristic; future PRs may replace it
//! with stddev over windows.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// Forecast horizon: end-of-day, end-of-month, end-of-quarter.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Horizon {
    /// End of today (UTC).
    Eod,
    /// End of current month (UTC).
    Eom,
    /// End of current quarter (UTC).
    Eoq,
}

impl Horizon {
    fn label(self) -> &'static str {
        match self {
            Self::Eod => "end of day",
            Self::Eom => "end of month",
            Self::Eoq => "end of quarter",
        }
    }
}

/// Output format for `argo costs forecast`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum ForecastFormat {
    /// Markdown table (default).
    Markdown,
    /// Single-line JSON object.
    Json,
}

/// One spend-ledger row reduced to the fields the forecaster needs.
#[derive(Copy, Clone, Debug)]
struct SpendRow {
    charged_at_micros: i64,
    amount_usd_micros: u64,
}

/// Result of [`compute_forecast`]. Kept private to this handler
/// per the brief; the JSON shape is the public contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct Forecast {
    horizon: Horizon,
    period_start_micros: i64,
    period_end_micros: i64,
    actual_so_far_usd_micros: u64,
    projected_additional_usd_micros: u64,
    projected_total_usd_micros: u64,
    band_lower_usd_micros: u64,
    band_upper_usd_micros: u64,
    lookback_seconds: i64,
    rows_in_lookback: usize,
    no_data: bool,
}

const BAND_PCT: f64 = 0.20;

/// CLI entry point. Loads the spend ledger, computes the forecast,
/// and prints in the requested format.
pub async fn run(
    horizon: Horizon,
    lookback_raw: &str,
    format: ForecastFormat,
) -> ArgoResult<()> {
    let lookback = parse_lookback(lookback_raw)?;
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let now = Utc::now();
    let period_start = period_start(now, horizon);
    let lookback_start = now - lookback;
    let cutoff = period_start
        .timestamp_micros()
        .min(lookback_start.timestamp_micros());
    let rows: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT charged_at_micros, amount_usd_micros FROM spend_ledger \
         WHERE charged_at_micros >= ? ORDER BY charged_at_micros ASC",
    )
    .bind(cutoff)
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("forecast query: {e}")))?;
    let spend: Vec<SpendRow> = rows
        .into_iter()
        .map(|(ts, amt)| SpendRow {
            charged_at_micros: ts,
            amount_usd_micros: u64::try_from(amt).unwrap_or(0),
        })
        .collect();
    let forecast = compute_forecast(&spend, now, horizon, lookback);
    match format {
        ForecastFormat::Json => print_json(&forecast),
        ForecastFormat::Markdown => print_markdown(&forecast, lookback_raw),
    }
    Ok(())
}

/// Pure forecast math. `spend` may include rows outside the period
/// (used for rate estimation only). The caller passes `now` so tests
/// are deterministic.
///
/// Math:
///   `rate = sum(spend in [now - lookback, now]) / effective_lookback_seconds`
///   `projected_additional = rate × (period_end - now)`
///   `projected_total = actual_so_far + projected_additional`
///   `band = projected_total × (1 ± 0.20)`
///
/// `effective_lookback_seconds` clamps to the span of available data
/// when the requested lookback exceeds the oldest row in `spend`,
/// so a 24h lookback against 30 minutes of data uses the 30-minute
/// span, not 24h. The ±20% band is a flat v1 heuristic — replace
/// with stddev over windows in a follow-up.
fn compute_forecast(
    spend: &[SpendRow],
    now: DateTime<Utc>,
    horizon: Horizon,
    lookback: Duration,
) -> Forecast {
    let period_start = period_start(now, horizon);
    let period_end = period_end(now, horizon);
    let now_us = now.timestamp_micros();
    let period_start_us = period_start.timestamp_micros();
    let period_end_us = period_end.timestamp_micros();

    let actual_so_far_usd_micros: u64 = spend
        .iter()
        .filter(|r| r.charged_at_micros >= period_start_us && r.charged_at_micros <= now_us)
        .map(|r| r.amount_usd_micros)
        .sum();

    let lookback_start_us = (now - lookback).timestamp_micros();
    let earliest = spend.iter().map(|r| r.charged_at_micros).min();
    let effective_start_us = match earliest {
        Some(e) if e > lookback_start_us => e,
        _ => lookback_start_us,
    };
    let effective_lookback_seconds =
        ((now_us - effective_start_us).max(1_000_000)) / 1_000_000;

    let lookback_rows: Vec<&SpendRow> = spend
        .iter()
        .filter(|r| r.charged_at_micros >= effective_start_us && r.charged_at_micros <= now_us)
        .collect();
    let lookback_sum_micros: u64 = lookback_rows.iter().map(|r| r.amount_usd_micros).sum();
    let rate_per_sec = lookback_sum_micros as f64 / effective_lookback_seconds as f64;

    let time_remaining_seconds = ((period_end_us - now_us).max(0)) / 1_000_000;
    let projected_additional = (rate_per_sec * time_remaining_seconds as f64).round();
    let projected_additional_usd_micros = if projected_additional.is_finite() {
        projected_additional.max(0.0) as u64
    } else {
        0
    };
    let projected_total_usd_micros =
        actual_so_far_usd_micros.saturating_add(projected_additional_usd_micros);
    let band_lower = (projected_total_usd_micros as f64 * (1.0 - BAND_PCT)).round() as u64;
    let band_upper = (projected_total_usd_micros as f64 * (1.0 + BAND_PCT)).round() as u64;

    Forecast {
        horizon,
        period_start_micros: period_start_us,
        period_end_micros: period_end_us,
        actual_so_far_usd_micros,
        projected_additional_usd_micros,
        projected_total_usd_micros,
        band_lower_usd_micros: band_lower,
        band_upper_usd_micros: band_upper,
        lookback_seconds: effective_lookback_seconds,
        rows_in_lookback: lookback_rows.len(),
        no_data: spend.is_empty(),
    }
}

fn parse_lookback(raw: &str) -> ArgoResult<Duration> {
    super::action_history::parse_duration(raw)
}

fn period_start(now: DateTime<Utc>, horizon: Horizon) -> DateTime<Utc> {
    match horizon {
        Horizon::Eod => start_of_day(now),
        Horizon::Eom => start_of_month(now),
        Horizon::Eoq => start_of_quarter(now),
    }
}

fn period_end(now: DateTime<Utc>, horizon: Horizon) -> DateTime<Utc> {
    match horizon {
        Horizon::Eod => start_of_day(now) + Duration::days(1),
        Horizon::Eom => next_month_start(now),
        Horizon::Eoq => next_quarter_start(now),
    }
}

fn start_of_day(now: DateTime<Utc>) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0)
        .single()
        .unwrap_or(now)
}

fn start_of_month(now: DateTime<Utc>) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
        .single()
        .unwrap_or(now)
}

fn next_month_start(now: DateTime<Utc>) -> DateTime<Utc> {
    let (y, m) = if now.month() == 12 {
        (now.year() + 1, 1)
    } else {
        (now.year(), now.month() + 1)
    };
    Utc.with_ymd_and_hms(y, m, 1, 0, 0, 0)
        .single()
        .unwrap_or(now)
}

fn start_of_quarter(now: DateTime<Utc>) -> DateTime<Utc> {
    let q_first = ((now.month() - 1) / 3) * 3 + 1;
    Utc.with_ymd_and_hms(now.year(), q_first, 1, 0, 0, 0)
        .single()
        .unwrap_or(now)
}

fn next_quarter_start(now: DateTime<Utc>) -> DateTime<Utc> {
    // Per the brief: quarter last month = ((month - 1) / 3) * 3 + 3.
    // End of quarter = start of the month AFTER that.
    let q_last = ((now.month() - 1) / 3) * 3 + 3;
    if q_last >= 12 {
        Utc.with_ymd_and_hms(now.year() + 1, 1, 1, 0, 0, 0)
            .single()
            .unwrap_or(now)
    } else {
        Utc.with_ymd_and_hms(now.year(), q_last + 1, 1, 0, 0, 0)
            .single()
            .unwrap_or(now)
    }
}

fn micros_to_usd(m: u64) -> f64 {
    m as f64 / 1_000_000.0
}

fn fmt_period(ts_micros: i64) -> String {
    Utc.timestamp_micros(ts_micros)
        .single()
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "?".into())
}

fn print_markdown(f: &Forecast, lookback_raw: &str) {
    println!("# Costs forecast — {}", f.horizon.label());
    println!(
        "Period: {} .. {} (UTC)",
        fmt_period(f.period_start_micros),
        fmt_period(f.period_end_micros)
    );
    println!(
        "Lookback: {lookback_raw} ({} rows, effective window {}s)",
        f.rows_in_lookback, f.lookback_seconds
    );
    if f.no_data {
        println!();
        println!("_(no spend data — projections are zero)_");
    }
    println!();
    println!("| metric               | USD            |");
    println!("| -------------------- | -------------: |");
    println!(
        "| actual so far        | ${:>12.4} |",
        micros_to_usd(f.actual_so_far_usd_micros)
    );
    println!(
        "| projected additional | ${:>12.4} |",
        micros_to_usd(f.projected_additional_usd_micros)
    );
    println!(
        "| projected total      | ${:>12.4} |",
        micros_to_usd(f.projected_total_usd_micros)
    );
    println!(
        "| band (±20%)          | ${:.4} – ${:.4} |",
        micros_to_usd(f.band_lower_usd_micros),
        micros_to_usd(f.band_upper_usd_micros),
    );
    println!();
    println!(
        "_Math: rate = sum(spend in lookback) / lookback_seconds; \
         projected_additional = rate × time-remaining. The ±20% band \
         is a v1 flat heuristic; a follow-up may replace it with \
         stddev over windows._"
    );
}

fn print_json(f: &Forecast) {
    let payload = serde_json::to_string(f).unwrap_or_else(|_| "{}".into());
    println!("{payload}");
}

#[cfg(test)]
#[path = "costs_forecast_tests.rs"]
mod tests;
