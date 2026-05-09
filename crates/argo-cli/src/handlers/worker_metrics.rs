//! `argo worker success <id>` — per-worker auto-approval rollup
//! (Stream `claude-worker-success-metrics-vis`, mission claim 2 KPI
//! seed).
//!
//! Read-only aggregator over `audit_record` rows whose `kind` is one
//! of the typed `AuditKind` approval variants — `ApprovalGranted`,
//! `ApprovalRevoked`, `ApprovalConsumed` — joined to `session` via
//! `audit_record.session_id = session.id` and filtered by
//! `session.worker_id`. No new schema; no writes.
//!
//! The brief's v1 premise referenced an `auto_approval_decision`
//! table that does not exist on main. The conductor's v2 retarget
//! rolls up over the existing audit-record stream instead, so the
//! emitted field names track the actual audit kinds (`granted`,
//! `revoked`, `consumed`) rather than the original `approve / deny /
//! timeout / defer` outcome model — the latter does not have a
//! direct audit-kind mapping today.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_audit::AuditKind;
use argo_identity::WorkerId;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use serde::Serialize;
use sqlx::{Pool, Sqlite};
use std::str::FromStr;

/// Bucket granularity for the time-series rollup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Bucket {
    None,
    Day,
    Week,
    Month,
}

impl Bucket {
    pub(crate) fn parse(s: &str) -> ArgoResult<Self> {
        match s {
            "none" => Ok(Self::None),
            "day" => Ok(Self::Day),
            "week" => Ok(Self::Week),
            "month" => Ok(Self::Month),
            other => Err(ArgoError::Provider(format!(
                "invalid --bucket '{other}' (expected none|day|week|month)"
            ))),
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Day => "day",
            Self::Week => "week",
            Self::Month => "month",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Format {
    Markdown,
    Json,
}

impl Format {
    pub(crate) fn parse(s: &str) -> ArgoResult<Self> {
        match s {
            "markdown" | "md" => Ok(Self::Markdown),
            "json" => Ok(Self::Json),
            other => Err(ArgoError::Provider(format!(
                "invalid --format '{other}' (expected markdown|json)"
            ))),
        }
    }
}

/// Decision row pulled from `audit_record` after the worker-bound
/// session join. Kept narrow on purpose — the rollup only needs the
/// kind + when it landed.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DecisionRow {
    pub(crate) kind: AuditKind,
    pub(crate) decided_at: DateTime<Utc>,
}

/// Per-bucket (or whole-period) tally. Field names mirror the audit
/// kinds, not the brief's stale `approve / deny / timeout / defer`
/// outcome model.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct Summary {
    pub granted: u64,
    pub revoked: u64,
    pub consumed: u64,
    pub total: u64,
    /// `granted / (granted + revoked)`. `None` when no granted +
    /// revoked decisions land in the window — divide-by-zero safe.
    pub approval_rate: Option<f64>,
    /// `consumed / granted`. `None` when no grants are present.
    pub consumed_rate: Option<f64>,
}

impl Summary {
    pub(crate) fn from_rows<'a, I: IntoIterator<Item = &'a DecisionRow>>(rows: I) -> Self {
        let mut s = Self {
            granted: 0,
            revoked: 0,
            consumed: 0,
            total: 0,
            approval_rate: None,
            consumed_rate: None,
        };
        for row in rows {
            s.total += 1;
            match row.kind {
                AuditKind::ApprovalGranted => s.granted += 1,
                AuditKind::ApprovalRevoked => s.revoked += 1,
                AuditKind::ApprovalConsumed => s.consumed += 1,
                _ => {}
            }
        }
        let denom = s.granted + s.revoked;
        if denom > 0 {
            #[allow(clippy::cast_precision_loss)]
            let rate = s.granted as f64 / denom as f64;
            s.approval_rate = Some(rate);
        }
        if s.granted > 0 {
            #[allow(clippy::cast_precision_loss)]
            let rate = s.consumed as f64 / s.granted as f64;
            s.consumed_rate = Some(rate);
        }
        s
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct BucketRow {
    pub bucket: String,
    #[serde(flatten)]
    pub summary: Summary,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Report {
    pub worker_id: String,
    pub since_micros: Option<i64>,
    pub bucket: String,
    pub summary: Summary,
    pub buckets: Vec<BucketRow>,
}

/// Top-level entry: parse args, load rows, render.
pub async fn run(
    worker_id: &str,
    since: Option<&str>,
    bucket: &str,
    format: &str,
) -> ArgoResult<()> {
    let parsed_worker = WorkerId::from_str(worker_id)
        .map_err(|e| ArgoError::Provider(format!("invalid worker_id '{worker_id}': {e}")))?;
    let bucket = Bucket::parse(bucket)?;
    let format = Format::parse(format)?;
    let since_micros = since.map(parse_since).transpose()?;
    let pool = open_pool().await?;
    let rows = fetch_decisions(&pool, parsed_worker, since_micros).await?;
    let report = build_report(worker_id, since_micros, bucket, &rows);
    match format {
        Format::Markdown => print_markdown(&report),
        Format::Json => print_json(&report)?,
    }
    Ok(())
}

/// Fetch rows from `audit_record`, joined on `session.worker_id`,
/// filtered to the three approval kinds.
pub(crate) async fn fetch_decisions(
    pool: &Pool<Sqlite>,
    worker_id: WorkerId,
    since_micros: Option<i64>,
) -> ArgoResult<Vec<DecisionRow>> {
    let worker_blob = worker_id.0.as_bytes().to_vec();
    let raw: Vec<(String, i64)> = if let Some(cutoff) = since_micros {
        sqlx::query_as(
            "SELECT a.kind, a.created_at \
             FROM audit_record a \
             JOIN session s ON s.id = a.session_id \
             WHERE s.worker_id = ? \
               AND a.kind IN ('approval.granted','approval.revoked','approval.consumed') \
               AND a.created_at >= ? \
             ORDER BY a.created_at ASC",
        )
        .bind(worker_blob.as_slice())
        .bind(cutoff)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as(
            "SELECT a.kind, a.created_at \
             FROM audit_record a \
             JOIN session s ON s.id = a.session_id \
             WHERE s.worker_id = ? \
               AND a.kind IN ('approval.granted','approval.revoked','approval.consumed') \
             ORDER BY a.created_at ASC",
        )
        .bind(worker_blob.as_slice())
        .fetch_all(pool)
        .await
    }
    .map_err(|e| ArgoError::Provider(format!("worker success rollup: {e}")))?;
    let mut out = Vec::with_capacity(raw.len());
    for (kind, micros) in raw {
        let Some(parsed) = AuditKind::from_label(&kind) else {
            // Belt-and-braces: the IN clause filters at the DB
            // level. A future kind rename would surface here.
            continue;
        };
        let Some(decided_at) = DateTime::<Utc>::from_timestamp_micros(micros) else {
            continue;
        };
        out.push(DecisionRow {
            kind: parsed,
            decided_at,
        });
    }
    Ok(out)
}

pub(crate) fn build_report(
    worker_id: &str,
    since_micros: Option<i64>,
    bucket: Bucket,
    rows: &[DecisionRow],
) -> Report {
    let summary = Summary::from_rows(rows);
    let buckets = match bucket {
        Bucket::None => Vec::new(),
        b => bucketize(rows, b),
    };
    Report {
        worker_id: worker_id.to_string(),
        since_micros,
        bucket: bucket.label().to_string(),
        summary,
        buckets,
    }
}

pub(crate) fn bucketize(rows: &[DecisionRow], bucket: Bucket) -> Vec<BucketRow> {
    use std::collections::BTreeMap;
    let mut by_bucket: BTreeMap<String, Vec<DecisionRow>> = BTreeMap::new();
    for row in rows {
        let key = bucket_key(row.decided_at, bucket);
        by_bucket.entry(key).or_default().push(*row);
    }
    by_bucket
        .into_iter()
        .map(|(bucket, rows)| BucketRow {
            bucket,
            summary: Summary::from_rows(rows.iter()),
        })
        .collect()
}

/// Produce a stable bucket key for a decision timestamp. ISO-week
/// keys use the `YYYY-Www` convention so they sort lexically and
/// match the brief's `strftime('%Y-%W', ...)` shape.
pub(crate) fn bucket_key(ts: DateTime<Utc>, bucket: Bucket) -> String {
    match bucket {
        Bucket::None => String::new(),
        Bucket::Day => ts.format("%Y-%m-%d").to_string(),
        Bucket::Week => {
            let iso = ts.iso_week();
            format!("{:04}-W{:02}", iso.year(), iso.week())
        }
        Bucket::Month => ts.format("%Y-%m").to_string(),
    }
}

/// Parse `--since` durations of the form `30d` / `12h` / `45m` /
/// `90s`. Mirrors `handlers::action_history::parse_duration`.
pub(crate) fn parse_since(raw: &str) -> ArgoResult<i64> {
    let raw = raw.trim();
    if raw.len() < 2 {
        return Err(ArgoError::Provider(format!(
            "invalid --since '{raw}' (e.g. 30d, 12h, 45m, 90s)"
        )));
    }
    let (digits, unit) = raw.split_at(raw.len() - 1);
    let value: i64 = digits.parse().map_err(|e| {
        ArgoError::Provider(format!("invalid --since '{raw}': {e}"))
    })?;
    if value <= 0 {
        return Err(ArgoError::Provider(format!(
            "--since must be positive: {raw}"
        )));
    }
    let dur = match unit {
        "s" => Duration::seconds(value),
        "m" => Duration::minutes(value),
        "h" => Duration::hours(value),
        "d" => Duration::days(value),
        other => {
            return Err(ArgoError::Provider(format!(
                "unsupported --since suffix '{other}' in '{raw}' (expected s/m/h/d)"
            )));
        }
    };
    Ok((Utc::now() - dur).timestamp_micros())
}

fn print_json(report: &Report) -> ArgoResult<()> {
    let s = serde_json::to_string_pretty(report)
        .map_err(|e| ArgoError::Provider(format!("worker success json: {e}")))?;
    println!("{s}");
    Ok(())
}

pub(crate) fn render_markdown(report: &Report) -> String {
    let mut out = String::new();
    let period = match report.since_micros {
        Some(micros) => Utc
            .timestamp_micros(micros)
            .single()
            .map(|dt| format!(" since {}", dt.to_rfc3339()))
            .unwrap_or_default(),
        None => String::new(),
    };
    out.push_str(&format!(
        "# Worker {} success rate{period}\n\n",
        report.worker_id
    ));
    out.push_str(&format!(
        "**Total decisions**: {} (granted: {}, revoked: {}, consumed: {})\n\n",
        report.summary.total,
        report.summary.granted,
        report.summary.revoked,
        report.summary.consumed,
    ));
    out.push_str(&format!(
        "**Approval rate**: {} | **Consumed rate**: {}\n",
        format_rate(report.summary.approval_rate),
        format_rate(report.summary.consumed_rate),
    ));
    if !report.buckets.is_empty() {
        out.push_str(&format!("\n## Per-{} rollup\n\n", report.bucket));
        out.push_str("| bucket | total | granted | revoked | consumed | approval_rate |\n");
        out.push_str("|---|---:|---:|---:|---:|---:|\n");
        for b in &report.buckets {
            out.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} |\n",
                b.bucket,
                b.summary.total,
                b.summary.granted,
                b.summary.revoked,
                b.summary.consumed,
                format_rate(b.summary.approval_rate),
            ));
        }
    }
    out
}

fn print_markdown(report: &Report) {
    print!("{}", render_markdown(report));
}

pub(crate) fn format_rate(rate: Option<f64>) -> String {
    match rate {
        Some(r) => format!("{:.2}%", r * 100.0),
        None => "n/a".to_string(),
    }
}

async fn open_pool() -> ArgoResult<Pool<Sqlite>> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(store.pool().clone())
}

#[cfg(test)]
#[path = "worker_metrics_tests.rs"]
mod tests;
