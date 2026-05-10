//! `argo worker review <id>` — quarterly / monthly / weekly review
//! packet (Stream `claude-worker-review-cycle-cli`, mission claim 2).
//! Read-only aggregator over existing tables — no new schema. SQL
//! loaders live in `worker_review_query`; markdown rendering lives in
//! `worker_review_md`.

use crate::handlers::worker_review_md::render_markdown;
use crate::handlers::worker_review_query::{
    load_anomaly_count, load_feedback, load_handoffs_received, load_handoffs_sent, load_kpi_deltas,
    load_success, load_summary, lookup_worker_name, open_pool,
};
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Utc};
use serde::Serialize;
use sqlx::SqlitePool;
use std::collections::BTreeMap;

/// Operator-facing `--period` choices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Period {
    Quarterly,
    Monthly,
    Weekly,
}

impl Period {
    pub fn parse(raw: &str) -> ArgoResult<Self> {
        match raw {
            "quarterly" | "quarter" | "q" => Ok(Self::Quarterly),
            "monthly" | "month" | "m" => Ok(Self::Monthly),
            "weekly" | "week" | "w" => Ok(Self::Weekly),
            other => Err(ArgoError::Provider(format!(
                "invalid --period '{other}' (expected quarterly|monthly|weekly)"
            ))),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Quarterly => "quarterly",
            Self::Monthly => "monthly",
            Self::Weekly => "weekly",
        }
    }

    /// Weeks are a rolling 7-day window; months/quarters are
    /// calendar-aligned at midnight UTC.
    pub fn start_for(self, now: DateTime<Utc>) -> DateTime<Utc> {
        match self {
            Self::Weekly => now - Duration::days(7),
            Self::Monthly => first_of_month(now),
            Self::Quarterly => first_of_quarter(now),
        }
    }
}

/// Output format. `Markdown` is the default operator surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Markdown,
    Json,
}

impl Format {
    pub fn parse(raw: &str) -> ArgoResult<Self> {
        match raw {
            "markdown" | "md" => Ok(Self::Markdown),
            "json" => Ok(Self::Json),
            other => Err(ArgoError::Provider(format!(
                "invalid --format '{other}' (expected markdown|json)"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ReviewReport {
    pub worker_id: String,
    pub worker_name: Option<String>,
    pub period: String,
    pub period_start_rfc3339: String,
    pub generated_at_rfc3339: String,
    pub summary: SummarySection,
    pub kpis: Vec<KpiDelta>,
    pub success: SuccessSection,
    pub feedback: Vec<FeedbackSample>,
    pub handoffs_sent: Vec<HandoffSample>,
    pub handoffs_received: Vec<HandoffSample>,
    /// `None` when `worker_anomaly_event` is missing — soft-link.
    pub anomalies: Option<AnomalySection>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SummarySection {
    pub session_count: i64,
    pub last_session_rfc3339: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KpiDelta {
    pub kpi: String,
    pub baseline: Option<f64>,
    pub latest: f64,
    pub delta: Option<f64>,
    pub latest_at_rfc3339: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SuccessSection {
    pub granted: i64,
    pub revoked: i64,
    pub consumed: i64,
    pub total: i64,
    pub approval_rate: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FeedbackSample {
    pub vote: String,
    pub action_id: String,
    pub comment: String,
    pub given_at_rfc3339: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HandoffSample {
    pub handoff_id: String,
    pub counterpart: String,
    pub status: String,
    pub expected_outcome: String,
    pub created_at_rfc3339: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnomalySection {
    pub count: i64,
}

/// CLI entry: parse, query, render.
pub async fn run(
    worker_id: &str,
    period: &str,
    since: Option<&str>,
    format: &str,
) -> ArgoResult<()> {
    let period = Period::parse(period)?;
    let format = Format::parse(format)?;
    let now = Utc::now();
    let period_start = match since {
        Some(raw) => parse_since(raw)?,
        None => period.start_for(now),
    };
    let pool = open_pool().await?;
    let report = build_report(&pool, worker_id, period, period_start, now).await?;
    match format {
        Format::Markdown => print!("{}", render_markdown(&report)),
        Format::Json => {
            let s = serde_json::to_string_pretty(&report)
                .map_err(|e| ArgoError::Provider(format!("worker review json: {e}")))?;
            println!("{s}");
        }
    }
    Ok(())
}

/// Pure assembly step — exposed for tests.
pub async fn build_report(
    pool: &SqlitePool,
    worker_id: &str,
    period: Period,
    period_start: DateTime<Utc>,
    now: DateTime<Utc>,
) -> ArgoResult<ReviewReport> {
    Ok(ReviewReport {
        worker_id: worker_id.to_string(),
        worker_name: lookup_worker_name(pool, worker_id).await?,
        period: period.label().to_string(),
        period_start_rfc3339: period_start.to_rfc3339(),
        generated_at_rfc3339: now.to_rfc3339(),
        summary: load_summary(pool, worker_id).await?,
        kpis: load_kpi_deltas(pool, worker_id, period_start).await?,
        success: load_success(pool, worker_id, period_start).await?,
        feedback: load_feedback(pool, worker_id).await?,
        handoffs_sent: load_handoffs_sent(pool, worker_id).await?,
        handoffs_received: load_handoffs_received(pool, worker_id).await?,
        anomalies: load_anomaly_count(pool, worker_id, period_start).await?,
    })
}

/// Latest reading per KPI; baseline = last reading BEFORE `period_start`.
pub fn kpi_deltas_from_rows(
    rows: &[(String, i64, f64)],
    period_start: DateTime<Utc>,
) -> Vec<KpiDelta> {
    let cutoff = period_start.timestamp_micros();
    let mut grouped: BTreeMap<String, Vec<(i64, f64)>> = BTreeMap::new();
    for (name, ts, value) in rows {
        grouped.entry(name.clone()).or_default().push((*ts, *value));
    }
    let mut out = Vec::with_capacity(grouped.len());
    for (kpi, mut series) in grouped {
        series.sort_by_key(|(t, _)| *t);
        let baseline = series
            .iter()
            .rev()
            .find(|(t, _)| *t < cutoff)
            .map(|(_, v)| *v);
        let Some((latest_at, latest)) = series.last().copied() else {
            continue;
        };
        out.push(KpiDelta {
            kpi,
            baseline,
            latest,
            delta: baseline.map(|b| latest - b),
            latest_at_rfc3339: micros_to_rfc3339(latest_at).unwrap_or_default(),
        });
    }
    out
}

/// Tally the audit-record approval kinds. Exposed for tests.
pub fn success_from_kinds(kinds: &[String]) -> SuccessSection {
    let mut s = SuccessSection {
        granted: 0,
        revoked: 0,
        consumed: 0,
        total: 0,
        approval_rate: None,
    };
    for kind in kinds {
        s.total += 1;
        match kind.as_str() {
            "approval.granted" => s.granted += 1,
            "approval.revoked" => s.revoked += 1,
            "approval.consumed" => s.consumed += 1,
            _ => {}
        }
    }
    let denom = s.granted + s.revoked;
    if denom > 0 {
        #[allow(clippy::cast_precision_loss)]
        let rate = s.granted as f64 / denom as f64;
        s.approval_rate = Some(rate);
    }
    s
}

/// Convert micros-since-epoch to RFC3339; `None` on overflow.
pub fn micros_to_rfc3339(micros: i64) -> Option<String> {
    DateTime::<Utc>::from_timestamp_micros(micros).map(|dt| dt.to_rfc3339())
}

/// Quarter starts: Jan/Apr/Jul/Oct first-of-month, 00:00:00 UTC.
pub fn first_of_quarter(now: DateTime<Utc>) -> DateTime<Utc> {
    let m = match now.month() { 1..=3 => 1, 4..=6 => 4, 7..=9 => 7, _ => 10 };
    midnight_utc(now.year(), m, 1)
}

/// First-of-current-month, 00:00:00 UTC.
pub fn first_of_month(now: DateTime<Utc>) -> DateTime<Utc> {
    midnight_utc(now.year(), now.month(), 1)
}

fn midnight_utc(year: i32, month: u32, day: u32) -> DateTime<Utc> {
    NaiveDate::from_ymd_opt(year, month, day)
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map_or_else(DateTime::<Utc>::default, |ndt| Utc.from_utc_datetime(&ndt))
}

/// Parse `--since` as RFC3339.
pub fn parse_since(raw: &str) -> ArgoResult<DateTime<Utc>> {
    let dt = DateTime::parse_from_rfc3339(raw).map_err(|e| {
        ArgoError::Provider(format!(
            "invalid --since '{raw}' (expected RFC3339, e.g. 2026-04-01T00:00:00Z): {e}"
        ))
    })?;
    Ok(dt.with_timezone(&Utc))
}

#[cfg(test)]
#[path = "worker_review_tests.rs"]
mod tests;
