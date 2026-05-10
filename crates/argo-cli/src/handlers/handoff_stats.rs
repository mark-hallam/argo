//! `argo handoff-stats` — aggregate metrics over the orchestrator's
//! `handoff` table (Stream `codex-handoff-stats-cli`).
//!
//! Surfaced as the top-level `argo handoff-stats` for the same reason
//! as `argo handoff-rules` and friends: the `HandoffCmd` enum lives
//! in `cli_session_cmds.rs` outside this brief's may-touch list.

use crate::cli::cli_handoff_cmds::HandoffStatsArgs;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_orchestrator::OrchestratorStore;
use argo_store::handoff_stats_query::{
    breakdown_by, fetch_window, overview, top_n_from, top_n_to, volume_per_bucket, BreakdownAxis,
    BreakdownRow, HandoffWindowRow, OverviewStats, TopRow, VolumeBucket,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

const TOP_N: usize = 5;

/// Dispatch `argo handoff-stats`.
pub async fn run(args: HandoffStatsArgs) -> ArgoResult<()> {
    let now = Utc::now().timestamp_micros();
    let window_secs = if args.histogram { 86_400 } else { 7 * 86_400 };
    let since_us = parse_window_start(args.since.as_deref(), now, window_secs)?;
    let tenant = parse_tenant(args.tenant.as_deref())?;
    let store = open_store().await?;
    let rows = fetch_window(&store, tenant, since_us, now).await?;
    if args.histogram {
        emit_histogram(&rows, &tenant, since_us, now, args.bucket_mins, args.json)?;
    } else {
        let by = parse_axis(args.by.as_deref())?;
        emit_report(&rows, &tenant, since_us, now, by, args.json)?;
    }
    Ok(())
}

#[derive(Debug, Serialize)]
struct StatsReport {
    tenant_id: String,
    since: String,
    until: String,
    by: &'static str,
    overview: OverviewStats,
    top_from: Vec<TopRow>,
    top_to: Vec<TopRow>,
    breakdown: Vec<BreakdownRow>,
}

#[derive(Debug, Serialize)]
struct HistogramReport {
    tenant_id: String,
    since: String,
    until: String,
    bucket_mins: u32,
    buckets: Vec<VolumeBucket>,
}

fn emit_report(
    rows: &[HandoffWindowRow],
    tenant: &TenantId,
    since_us: i64,
    until_us: i64,
    axis: BreakdownAxis,
    json: bool,
) -> ArgoResult<()> {
    let report = StatsReport {
        tenant_id: tenant.0.to_string(),
        since: rfc3339(since_us),
        until: rfc3339(until_us),
        by: axis_label(axis),
        overview: overview(rows),
        top_from: top_n_from(rows, TOP_N),
        top_to: top_n_to(rows, TOP_N),
        breakdown: breakdown_by(rows, axis),
    };
    if json {
        println!(
            "{}",
            serde_json::to_string(&report)
                .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?
        );
    } else {
        print!("{}", render_markdown(&report));
    }
    Ok(())
}

fn emit_histogram(
    rows: &[HandoffWindowRow],
    tenant: &TenantId,
    since_us: i64,
    until_us: i64,
    bucket_mins: u32,
    json: bool,
) -> ArgoResult<()> {
    let bucket_secs = i64::from(bucket_mins.max(1)) * 60;
    let buckets = volume_per_bucket(rows, since_us, until_us, bucket_secs);
    let report = HistogramReport {
        tenant_id: tenant.0.to_string(),
        since: rfc3339(since_us),
        until: rfc3339(until_us),
        bucket_mins,
        buckets: buckets.clone(),
    };
    if json {
        println!(
            "{}",
            serde_json::to_string(&report)
                .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?
        );
    } else {
        print!("{}", render_histogram(&report, &buckets));
    }
    Ok(())
}

fn render_markdown(r: &StatsReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "# Handoff stats — {} to {}\n\n",
        r.since, r.until
    ));
    out.push_str("## Overview\n\n");
    out.push_str("| metric | value |\n");
    out.push_str("|--------|-------|\n");
    out.push_str(&format!("| total | {} |\n", r.overview.total));
    out.push_str(&format!(
        "| success_rate | {} |\n",
        fmt_pct(r.overview.success_rate)
    ));
    out.push_str(&format!(
        "| escalation_rate | {} |\n",
        fmt_pct(r.overview.escalation_rate)
    ));
    out.push_str(&format!(
        "| median_latency | {} |\n",
        fmt_duration_micros(r.overview.median_latency_micros)
    ));
    out.push_str("\n## By worker (handed-off-FROM)\n\n");
    out.push_str(&render_top("worker", &r.top_from));
    out.push_str("\n## By worker (handed-off-TO)\n\n");
    out.push_str(&render_top("destination", &r.top_to));
    out.push_str(&format!("\n## Breakdown by {}\n\n", r.by));
    out.push_str(&render_breakdown(&r.breakdown));
    out.push_str("\n## By outcome\n\n");
    out.push_str("| outcome | count |\n");
    out.push_str("|---------|------:|\n");
    out.push_str(&format!("| resolved | {} |\n", r.overview.resolved));
    out.push_str(&format!("| escalated | {} |\n", r.overview.escalated));
    out.push_str(&format!("| abandoned | {} |\n", r.overview.abandoned));
    out.push_str(&format!("| pending | {} |\n", r.overview.pending));
    out
}

fn render_top(label: &str, rows: &[TopRow]) -> String {
    if rows.is_empty() {
        return "(no rows)\n".into();
    }
    let mut out = format!("| {label} | count |\n|--------|------:|\n");
    for r in rows {
        out.push_str(&format!("| {} | {} |\n", r.key, r.count));
    }
    out
}

fn render_breakdown(rows: &[BreakdownRow]) -> String {
    if rows.is_empty() {
        return "(no rows)\n".into();
    }
    let mut out = String::from(
        "| key | total | resolved | escalated | abandoned | pending | success_rate |\n\
         |-----|------:|---------:|----------:|----------:|--------:|-------------:|\n",
    );
    for r in rows {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} |\n",
            r.key,
            r.count,
            r.resolved,
            r.escalated,
            r.abandoned,
            r.pending,
            fmt_pct(r.success_rate),
        ));
    }
    out
}

fn render_histogram(report: &HistogramReport, buckets: &[VolumeBucket]) -> String {
    if buckets.is_empty() {
        return "(empty window)\n".into();
    }
    let max = buckets.iter().map(|b| b.count).max().unwrap_or(0);
    let mut out = format!(
        "# Handoff volume — {} to {} (bucket {} mins)\n\n",
        report.since, report.until, report.bucket_mins
    );
    for b in buckets {
        let bar_len = if max == 0 {
            0
        } else {
            ((b.count as f64 / max as f64) * 40.0).round() as usize
        };
        let bar: String = "█".repeat(bar_len);
        out.push_str(&format!(
            "{} | {:>4} | {}\n",
            rfc3339(b.bucket_start_micros),
            b.count,
            bar,
        ));
    }
    out
}

fn fmt_pct(value: Option<f64>) -> String {
    match value {
        Some(v) => format!("{:.1}%", v * 100.0),
        None => "—".into(),
    }
}

fn fmt_duration_micros(value: Option<i64>) -> String {
    let Some(us) = value else {
        return "—".into();
    };
    let secs = us as f64 / 1_000_000.0;
    if secs < 60.0 {
        format!("{secs:.2}s")
    } else if secs < 3600.0 {
        format!("{:.2}m", secs / 60.0)
    } else {
        format!("{:.2}h", secs / 3600.0)
    }
}

fn rfc3339(micros: i64) -> String {
    DateTime::<Utc>::from_timestamp_micros(micros)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| micros.to_string())
}

fn parse_axis(raw: Option<&str>) -> ArgoResult<BreakdownAxis> {
    Ok(match raw.unwrap_or("worker") {
        "worker" => BreakdownAxis::Worker,
        "skill" => BreakdownAxis::Skill,
        "destination" => BreakdownAxis::Destination,
        other => {
            return Err(ArgoError::Provider(format!(
                "--by '{other}' — expected one of worker|skill|destination"
            )));
        }
    })
}

fn axis_label(axis: BreakdownAxis) -> &'static str {
    match axis {
        BreakdownAxis::Worker => "worker",
        BreakdownAxis::Skill => "skill",
        BreakdownAxis::Destination => "destination",
    }
}

fn parse_window_start(raw: Option<&str>, now: i64, default_secs: u64) -> ArgoResult<i64> {
    let secs = match raw {
        Some(value) => parse_duration_secs(value)?,
        None => default_secs,
    };
    Ok(now - i64::try_from(secs).unwrap_or(i64::MAX) * 1_000_000)
}

fn parse_duration_secs(raw: &str) -> ArgoResult<u64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ArgoError::Provider("empty --since".into()));
    }
    let split = trimmed
        .find(|c: char| c.is_ascii_alphabetic())
        .unwrap_or(trimmed.len());
    let (num, unit) = trimmed.split_at(split);
    let n: u64 = num
        .parse()
        .map_err(|e| ArgoError::Provider(format!("bad --since '{raw}': {e}")))?;
    let mult = match unit {
        "s" | "" => 1,
        "m" => 60,
        "h" => 3_600,
        "d" => 86_400,
        other => {
            return Err(ArgoError::Provider(format!(
                "unknown --since unit '{other}' — use s|m|h|d"
            )));
        }
    };
    Ok(n.saturating_mul(mult))
}

fn parse_tenant(raw: Option<&str>) -> ArgoResult<TenantId> {
    let raw = raw
        .map(ToOwned::to_owned)
        .or_else(|| std::env::var("ARGO_DEFAULT_TENANT").ok())
        .ok_or_else(|| ArgoError::Provider("missing --tenant (or ARGO_DEFAULT_TENANT)".into()))?;
    Uuid::parse_str(&raw)
        .map(TenantId)
        .map_err(|e| ArgoError::Provider(format!("invalid tenant id '{raw}': {e}")))
}

async fn open_store() -> ArgoResult<SessionStore> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = Arc::new(SessionStore::open(&db_path(&home)).await?);
    store.migrate().await?;
    OrchestratorStore::new(Arc::clone(&store)).ensure_schema().await?;
    Arc::try_unwrap(store)
        .map_err(|_| ArgoError::Provider("handoff-stats open_store: store still shared".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_window_start_accepts_each_unit() {
        let now = 10_000_000_000;
        let default = 7 * 86_400;
        assert_eq!(
            parse_window_start(Some("1s"), now, default).unwrap(),
            now - 1_000_000
        );
        assert_eq!(
            parse_window_start(Some("2m"), now, default).unwrap(),
            now - 2 * 60 * 1_000_000
        );
        assert_eq!(
            parse_window_start(Some("3h"), now, default).unwrap(),
            now - 3 * 3_600 * 1_000_000
        );
        assert_eq!(
            parse_window_start(None, now, default).unwrap(),
            now - 7 * 86_400 * 1_000_000
        );
    }

    #[test]
    fn parse_window_start_rejects_unknown_unit() {
        assert!(parse_window_start(Some("3y"), 0, 0).is_err());
        assert!(parse_window_start(Some(""), 0, 0).is_err());
    }

    #[test]
    fn parse_axis_maps_strings_and_rejects_other() {
        assert_eq!(parse_axis(None).unwrap(), BreakdownAxis::Worker);
        assert_eq!(parse_axis(Some("skill")).unwrap(), BreakdownAxis::Skill);
        assert_eq!(
            parse_axis(Some("destination")).unwrap(),
            BreakdownAxis::Destination
        );
        assert!(parse_axis(Some("bogus")).is_err());
    }

    #[test]
    fn fmt_pct_and_duration_handle_none() {
        assert_eq!(fmt_pct(None), "—");
        assert_eq!(fmt_pct(Some(0.5)), "50.0%");
        assert_eq!(fmt_duration_micros(None), "—");
        assert_eq!(fmt_duration_micros(Some(1_500_000)), "1.50s");
        assert_eq!(fmt_duration_micros(Some(120_000_000)), "2.00m");
    }
}
