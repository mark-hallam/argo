//! `argo worker review <id>` — quarterly / monthly / weekly review
//! packet (Stream `claude-worker-review-cycle-cli`, mission claim 2).
//! Read-only aggregator over existing tables — no new schema. Render
//! lives in the sibling `worker_review_md`.

use crate::handlers::worker_review_md::render_markdown;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Utc};
use serde::Serialize;
use sqlx::{Row, SqlitePool};
use std::collections::BTreeMap;
use std::str::FromStr;
use uuid::Uuid;

const FEEDBACK_SAMPLE_LIMIT: i64 = 3;
const HANDOFF_SAMPLE_LIMIT: i64 = 3;

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

async fn lookup_worker_name(pool: &SqlitePool, worker_id: &str) -> ArgoResult<Option<String>> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT name FROM persona WHERE worker_id = ? LIMIT 1")
            .bind(worker_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("worker review name: {e}")))?;
    Ok(row.map(|(name,)| name))
}

async fn load_summary(pool: &SqlitePool, worker_id: &str) -> ArgoResult<SummarySection> {
    let blob = parse_worker_blob(worker_id)?;
    let row: Option<(i64, Option<i64>)> = sqlx::query_as(
        "SELECT COUNT(*) AS n, MAX(started_at) AS last_at \
         FROM session WHERE worker_id = ?",
    )
    .bind(blob.as_slice())
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker review summary: {e}")))?;
    let (count, last) = row.unwrap_or((0, None));
    Ok(SummarySection {
        session_count: count,
        last_session_rfc3339: last.and_then(micros_to_rfc3339),
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

async fn load_kpi_deltas(
    pool: &SqlitePool,
    worker_id: &str,
    period_start: DateTime<Utc>,
) -> ArgoResult<Vec<KpiDelta>> {
    // worker_kpi_value is created lazily on first write — ensure it
    // before SELECTing or an empty-db review errors.
    argo_store::worker_kpi::ensure_schema(pool).await?;
    let raw = sqlx::query(
        "SELECT kpi_name, value, observed_at \
         FROM worker_kpi_value WHERE worker_id = ? \
         ORDER BY observed_at ASC, id ASC",
    )
    .bind(worker_id)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker review kpi: {e}")))?;
    let rows: Vec<(String, i64, f64)> = raw
        .into_iter()
        .map(|r| (r.get("kpi_name"), r.get("observed_at"), r.get("value")))
        .collect();
    Ok(kpi_deltas_from_rows(&rows, period_start))
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

async fn load_success(
    pool: &SqlitePool,
    worker_id: &str,
    period_start: DateTime<Utc>,
) -> ArgoResult<SuccessSection> {
    let blob = parse_worker_blob(worker_id)?;
    let cutoff = period_start.timestamp_micros();
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT a.kind FROM audit_record a \
         JOIN session s ON s.id = a.session_id \
         WHERE s.worker_id = ? AND a.created_at >= ? \
           AND a.kind IN ('approval.granted','approval.revoked','approval.consumed')",
    )
    .bind(blob.as_slice())
    .bind(cutoff)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker review success: {e}")))?;
    let kinds: Vec<String> = rows.into_iter().map(|(k,)| k).collect();
    Ok(success_from_kinds(&kinds))
}

async fn load_feedback(pool: &SqlitePool, worker_id: &str) -> ArgoResult<Vec<FeedbackSample>> {
    let rows = sqlx::query(
        "SELECT vote, action_id, comment, given_at \
         FROM operator_feedback WHERE worker_id = ? \
         ORDER BY given_at DESC LIMIT ?",
    )
    .bind(worker_id)
    .bind(FEEDBACK_SAMPLE_LIMIT)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker review feedback: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|row| FeedbackSample {
            vote: row.get("vote"),
            action_id: row.get("action_id"),
            comment: row.get::<String, _>("comment"),
            given_at_rfc3339: micros_to_rfc3339(row.get("given_at")).unwrap_or_default(),
        })
        .collect())
}

async fn load_handoffs_sent(pool: &SqlitePool, worker_id: &str) -> ArgoResult<Vec<HandoffSample>> {
    let rows = sqlx::query(
        "SELECT id, \"to\", expected_outcome, status, created_at \
         FROM handoff WHERE from_worker_id = ? \
         ORDER BY created_at DESC LIMIT ?",
    )
    .bind(worker_id)
    .bind(HANDOFF_SAMPLE_LIMIT)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker review handoffs sent: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|row| HandoffSample {
            handoff_id: row.get("id"),
            counterpart: counterpart_from_to(&row.get::<String, _>("to")),
            status: row.get("status"),
            expected_outcome: row.get("expected_outcome"),
            created_at_rfc3339: micros_to_rfc3339(row.get("created_at")).unwrap_or_default(),
        })
        .collect())
}

async fn load_handoffs_received(
    pool: &SqlitePool,
    worker_id: &str,
) -> ArgoResult<Vec<HandoffSample>> {
    let rows = sqlx::query(
        "SELECT id, from_worker_id, expected_outcome, status, created_at \
         FROM handoff WHERE json_extract(\"to\", '$.worker_id') = ? \
         ORDER BY created_at DESC LIMIT ?",
    )
    .bind(worker_id)
    .bind(HANDOFF_SAMPLE_LIMIT)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker review handoffs received: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|row| HandoffSample {
            handoff_id: row.get("id"),
            counterpart: row.get("from_worker_id"),
            status: row.get("status"),
            expected_outcome: row.get("expected_outcome"),
            created_at_rfc3339: micros_to_rfc3339(row.get("created_at")).unwrap_or_default(),
        })
        .collect())
}

async fn load_anomaly_count(
    pool: &SqlitePool,
    worker_id: &str,
    period_start: DateTime<Utc>,
) -> ArgoResult<Option<AnomalySection>> {
    if !table_exists(pool, "worker_anomaly_event").await? {
        return Ok(None);
    }
    let cutoff = period_start.timestamp_micros();
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT COUNT(*) FROM worker_anomaly_event \
         WHERE worker_id = ? AND observed_at >= ?",
    )
    .bind(worker_id)
    .bind(cutoff)
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker review anomalies: {e}")))?;
    Ok(Some(AnomalySection {
        count: row.map(|(n,)| n).unwrap_or(0),
    }))
}

async fn table_exists(pool: &SqlitePool, name: &str) -> ArgoResult<bool> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?",
    )
    .bind(name)
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker review table-exists: {e}")))?;
    Ok(row.is_some())
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

/// Decode the handoff `to` JSON column into a printable counterpart.
fn counterpart_from_to(raw: &str) -> String {
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(raw) else {
        return raw.to_string();
    };
    match parsed.get("kind").and_then(|v| v.as_str()) {
        Some("worker") => parsed
            .get("worker_id")
            .and_then(|v| v.as_str())
            .unwrap_or("(worker)")
            .to_string(),
        Some("human") => parsed
            .get("addr")
            .and_then(|v| v.as_str())
            .unwrap_or("(human)")
            .to_string(),
        _ => raw.to_string(),
    }
}

/// `session.worker_id` is a 16-byte BLOB; round-trip through `Uuid`.
fn parse_worker_blob(worker_id: &str) -> ArgoResult<Vec<u8>> {
    let parsed = Uuid::from_str(worker_id)
        .map_err(|e| ArgoError::Provider(format!("invalid worker_id '{worker_id}': {e}")))?;
    Ok(parsed.as_bytes().to_vec())
}

async fn open_pool() -> ArgoResult<SqlitePool> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(store.pool().clone())
}

#[cfg(test)]
#[path = "worker_review_tests.rs"]
mod tests;
