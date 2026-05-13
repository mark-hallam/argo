//! Executive-friendly `argo audit report`.
mod render;

use argo_audit::{export_ndjson, verify_all, ExportedRow};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, NaiveDate, Utc};
use render::{json_envelope, render_html, render_markdown};

pub(super) use super::audit_report_aggregate::{
    channel_breakdown, pending_approvals, summarize_rows, top_spenders, ChainVerdict,
    ChannelBreakdown, PendingApproval, SpenderRow, SummaryStats,
};

pub struct ReportArgs {
    pub format: String,
    pub output: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub worker_id: Option<String>,
    pub tenant: Option<String>,
    pub json: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
enum ReportFormat {
    Markdown,
    Html,
}

#[derive(Debug)]
struct NormalizedReport {
    format: ReportFormat,
    output: Option<String>,
    since: Option<String>,
    until: Option<String>,
    since_micros: Option<i64>,
    until_micros: Option<i64>,
    worker_id: Option<String>,
    tenant: Option<String>,
    json: bool,
}

#[derive(Debug)]
struct ReportData {
    generated_at: String,
    rows: Vec<ExportedRow>,
    summary: SummaryStats,
    channels: Vec<ChannelBreakdown>,
    spenders: Vec<SpenderRow>,
    pending: Vec<PendingApproval>,
    chain: ChainVerdict,
    flakes: Vec<&'static str>,
}

pub async fn run(store: &SessionStore, args: ReportArgs) -> ArgoResult<()> {
    let query = NormalizedReport::new(args)?;
    let rows = collect_rows(store, &query).await?;
    let report = build_report(store, rows).await?;
    let artifact = match query.format {
        ReportFormat::Markdown => render_markdown(&query, &report),
        ReportFormat::Html => render_html(&query, &report),
    };
    if let Some(path) = query.output.as_deref() {
        std::fs::write(path, artifact.as_bytes())
            .map_err(|e| ArgoError::Provider(format!("write {path}: {e}")))?;
    } else if !query.json {
        println!("{artifact}");
    }
    if query.json {
        let line = serde_json::to_string(&json_envelope(&query, &report, artifact.len()))
            .map_err(|e| ArgoError::Provider(format!("serialize report summary: {e}")))?;
        println!("{line}");
    }
    Ok(())
}

impl NormalizedReport {
    fn new(args: ReportArgs) -> ArgoResult<Self> {
        let since_micros = args
            .since
            .as_deref()
            .map(|raw| parse_bound(raw, false, "--since"))
            .transpose()?;
        let until_micros = args
            .until
            .as_deref()
            .map(|raw| parse_bound(raw, true, "--until"))
            .transpose()?;
        if let (Some(since), Some(until)) = (since_micros, until_micros) {
            if since > until {
                return Err(ArgoError::Provider(
                    "invalid audit report time range: --since is after --until".into(),
                ));
            }
        }
        Ok(Self {
            format: parse_format(&args.format)?,
            output: args.output,
            since: args.since,
            until: args.until,
            since_micros,
            until_micros,
            worker_id: args.worker_id,
            tenant: args.tenant,
            json: args.json,
        })
    }

    fn matches(&self, row: &ExportedRow) -> bool {
        if self
            .since_micros
            .is_some_and(|since| row.created_at_micros < since)
        {
            return false;
        }
        if self
            .until_micros
            .is_some_and(|until| row.created_at_micros > until)
        {
            return false;
        }
        if self
            .worker_id
            .as_deref()
            .is_some_and(|worker| !payload_matches(&row.payload, worker, WORKER_POINTERS))
        {
            return false;
        }
        if self
            .tenant
            .as_deref()
            .is_some_and(|tenant| !payload_matches(&row.payload, tenant, TENANT_POINTERS))
        {
            return false;
        }
        true
    }
}

async fn collect_rows(
    store: &SessionStore,
    query: &NormalizedReport,
) -> ArgoResult<Vec<ExportedRow>> {
    let mut rows = Vec::new();
    export_ndjson(store, |row| {
        if query.matches(row) {
            rows.push(row.clone());
        }
        Ok(())
    })
    .await?;
    Ok(rows)
}

async fn build_report(store: &SessionStore, rows: Vec<ExportedRow>) -> ArgoResult<ReportData> {
    let verify = verify_all(store).await?;
    Ok(ReportData {
        generated_at: Utc::now().to_rfc3339(),
        summary: summarize_rows(&rows),
        channels: channel_breakdown(&rows),
        spenders: top_spenders(store).await?,
        pending: pending_approvals(store).await?,
        chain: ChainVerdict {
            ok: verify.ok,
            rows: verify.rows,
            breaks: verify.breaks.len(),
        },
        flakes: known_flakes(),
        rows,
    })
}

const WORKER_POINTERS: &[&str] = &["/worker_id", "/from_worker_id", "/to/worker_id"];
const TENANT_POINTERS: &[&str] = &["/tenant_id"];

fn payload_matches(payload: &serde_json::Value, expected: &str, pointers: &[&str]) -> bool {
    pointers
        .iter()
        .any(|ptr| payload.pointer(ptr).and_then(|v| v.as_str()) == Some(expected))
}

fn parse_format(raw: &str) -> ArgoResult<ReportFormat> {
    match raw {
        "markdown" | "md" => Ok(ReportFormat::Markdown),
        "html" => Ok(ReportFormat::Html),
        other => Err(ArgoError::Provider(format!(
            "unsupported audit report format '{other}' (expected markdown or html)"
        ))),
    }
}

fn parse_bound(raw: &str, until: bool, flag: &str) -> ArgoResult<i64> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(raw) {
        return Ok(dt.timestamp_micros());
    }
    let date = NaiveDate::parse_from_str(raw, "%Y-%m-%d").map_err(|e| {
        ArgoError::Provider(format!(
            "invalid {flag} date '{raw}': {e} (expected YYYY-MM-DD or RFC3339)"
        ))
    })?;
    let time = if until {
        date.and_hms_micro_opt(23, 59, 59, 999_999)
    } else {
        date.and_hms_micro_opt(0, 0, 0, 0)
    }
    .ok_or_else(|| ArgoError::Schema(format!("invalid {flag} date '{raw}'")))?;
    Ok(DateTime::<Utc>::from_naive_utc_and_offset(time, Utc).timestamp_micros())
}

fn known_flakes() -> Vec<&'static str> {
    vec!["setup_steps::vertical_step_skips_empty_registry (ARGO_VERTICALS_DIR env leak)"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_renders_all_six_sections() {
        let report = sample_report();
        let query = sample_query(ReportFormat::Markdown);
        let out = render_markdown(&query, &report);
        for heading in [
            "## Summary stats",
            "## Approval-decision breakdown by NotifyChannel",
            "## Top 10 spending workers",
            "## Pending-approval queue snapshot",
            "## Hash-chain integrity check verdict",
            "## Pre-existing flakes",
        ] {
            assert!(out.contains(heading), "missing {heading}");
        }
    }

    #[test]
    fn html_is_standalone_and_escaped() {
        let mut report = sample_report();
        report.pending[0].action_type = "notify <exec>".into();
        let out = render_html(&sample_query(ReportFormat::Html), &report);
        assert!(out.starts_with("<!doctype html><html"));
        assert!(out.contains("&lt;exec&gt;"));
        assert!(out.contains("</html>"));
    }

    #[test]
    fn summary_counts_expected_audit_kinds() {
        let rows = vec![
            row("tool_call", serde_json::json!({})),
            row("approval.granted", serde_json::json!({"channel":"slack"})),
            row("escalation.required", serde_json::json!({})),
            row(
                "capability.gap",
                serde_json::json!({"kind":"spend_anomaly"}),
            ),
        ];
        let s = summarize_rows(&rows);
        assert_eq!(s.actions_executed, 1);
        assert_eq!(s.approvals_granted, 1);
        assert_eq!(s.escalations_triggered, 1);
        assert_eq!(s.anomalies_detected, 1);
    }

    #[test]
    fn json_envelope_lists_sections() {
        let report = sample_report();
        let value = json_envelope(&sample_query(ReportFormat::Markdown), &report, 42);
        assert_eq!(value["report"]["bytes"], 42);
        assert_eq!(value["report"]["sections"].as_array().unwrap().len(), 6);
    }

    fn sample_query(format: ReportFormat) -> NormalizedReport {
        NormalizedReport {
            format,
            output: None,
            since: Some("2026-05-06".into()),
            until: None,
            since_micros: None,
            until_micros: None,
            worker_id: None,
            tenant: None,
            json: false,
        }
    }

    fn sample_report() -> ReportData {
        ReportData {
            generated_at: "2026-05-06T00:00:00Z".into(),
            rows: vec![row("tool_call", serde_json::json!({}))],
            summary: SummaryStats {
                actions_executed: 1,
                approvals_granted: 1,
                escalations_triggered: 0,
                anomalies_detected: 0,
            },
            channels: vec![ChannelBreakdown {
                channel: "slack".into(),
                granted: 1,
                revoked: 0,
                consumed: 0,
                total: 1,
            }],
            spenders: vec![SpenderRow {
                worker_id: "worker-a".into(),
                charges: 1,
                usd_micros: 1_250_000,
            }],
            pending: vec![PendingApproval {
                action_id: "act-1".into(),
                worker_id: "worker-a".into(),
                action_type: "approve_listing".into(),
                risk_level: "high".into(),
                created_at: "2026-05-06T00:00:00Z".into(),
            }],
            chain: ChainVerdict {
                ok: true,
                rows: 1,
                breaks: 0,
            },
            flakes: known_flakes(),
        }
    }

    fn row(kind: &str, payload: serde_json::Value) -> ExportedRow {
        ExportedRow {
            seq: 1,
            id: "row-1".into(),
            session_id: None,
            vertical_id: None,
            kind: kind.into(),
            payload,
            parent_hash: "00".repeat(32),
            self_hash: "11".repeat(32),
            created_at_micros: 1_778_025_600_000_000,
        }
    }
}
