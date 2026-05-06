use super::{
    ChainVerdict, ChannelBreakdown, NormalizedReport, PendingApproval, ReportData, SpenderRow,
    SummaryStats,
};

pub(super) fn render_markdown(query: &NormalizedReport, report: &ReportData) -> String {
    let mut out = String::new();
    out.push_str("# Compliance Audit Report\n\n");
    out.push_str(&format!(
        "- Generated: {}\n- Rows in scope: {}\n- Since: {}\n- Until: {}\n\n",
        report.generated_at,
        report.rows.len(),
        query.since.as_deref().unwrap_or("beginning"),
        query.until.as_deref().unwrap_or("now")
    ));
    push_summary_md(&mut out, &report.summary);
    push_channels_md(&mut out, &report.channels);
    push_spenders_md(&mut out, &report.spenders);
    push_pending_md(&mut out, &report.pending);
    push_chain_md(&mut out, &report.chain);
    push_flakes_md(&mut out, &report.flakes);
    out
}

pub(super) fn render_html(query: &NormalizedReport, report: &ReportData) -> String {
    let md = render_markdown(query, report);
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>Compliance Audit Report</title></head><body><pre>{}</pre></body></html>\n",
        html_escape(&md)
    )
}

pub(super) fn json_envelope(
    query: &NormalizedReport,
    report: &ReportData,
    bytes: usize,
) -> serde_json::Value {
    serde_json::json!({
        "report": {
            "ok": true,
            "format": query.format,
            "destination": query.output.as_deref(),
            "bytes": bytes,
            "rows": report.rows.len(),
            "sections": [
                "summary_stats",
                "approval_decision_breakdown_by_notify_channel",
                "top_10_spending_workers",
                "pending_approval_queue_snapshot",
                "hash_chain_integrity_check_verdict",
                "pre_existing_flakes"
            ],
            "summary": report.summary,
            "hash_chain": report.chain,
        }
    })
}

fn push_summary_md(out: &mut String, s: &SummaryStats) {
    out.push_str("## Summary stats\n\n");
    out.push_str("| metric | count |\n| --- | ---: |\n");
    out.push_str(&format!("| actions executed | {} |\n", s.actions_executed));
    out.push_str(&format!(
        "| approvals granted | {} |\n",
        s.approvals_granted
    ));
    out.push_str(&format!(
        "| escalations triggered | {} |\n",
        s.escalations_triggered
    ));
    out.push_str(&format!(
        "| anomalies detected | {} |\n\n",
        s.anomalies_detected
    ));
}

fn push_channels_md(out: &mut String, rows: &[ChannelBreakdown]) {
    out.push_str("## Approval-decision breakdown by NotifyChannel\n\n");
    out.push_str(
        "| channel | granted | revoked | consumed | total |\n| --- | ---: | ---: | ---: | ---: |\n",
    );
    if rows.is_empty() {
        out.push_str("| (none) | 0 | 0 | 0 | 0 |\n\n");
        return;
    }
    for r in rows {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            md_cell(&r.channel),
            r.granted,
            r.revoked,
            r.consumed,
            r.total
        ));
    }
    out.push('\n');
}

fn push_spenders_md(out: &mut String, rows: &[SpenderRow]) {
    out.push_str("## Top 10 spending workers\n\n");
    out.push_str("| worker | charges | usd |\n| --- | ---: | ---: |\n");
    if rows.is_empty() {
        out.push_str("| (none) | 0 | 0.000000 |\n\n");
        return;
    }
    for r in rows {
        out.push_str(&format!(
            "| {} | {} | {:.6} |\n",
            md_cell(&r.worker_id),
            r.charges,
            r.usd_micros as f64 / 1_000_000.0
        ));
    }
    out.push('\n');
}

fn push_pending_md(out: &mut String, rows: &[PendingApproval]) {
    out.push_str("## Pending-approval queue snapshot\n\n");
    out.push_str(
        "| action | worker | type | risk | created_at |\n| --- | --- | --- | --- | --- |\n",
    );
    if rows.is_empty() {
        out.push_str("| (none) | - | - | - | - |\n\n");
        return;
    }
    for r in rows {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            md_cell(&r.action_id),
            md_cell(&r.worker_id),
            md_cell(&r.action_type),
            md_cell(&r.risk_level),
            md_cell(&r.created_at)
        ));
    }
    out.push('\n');
}

fn push_chain_md(out: &mut String, c: &ChainVerdict) {
    out.push_str("## Hash-chain integrity check verdict\n\n");
    out.push_str(&format!(
        "- Status: {}\n- Rows verified: {}\n- Breaks: {}\n\n",
        if c.ok { "OK" } else { "BROKEN" },
        c.rows,
        c.breaks
    ));
}

fn push_flakes_md(out: &mut String, flakes: &[&str]) {
    out.push_str("## Pre-existing flakes\n\n");
    for flake in flakes {
        out.push_str(&format!("- `{}`\n", flake));
    }
    out.push('\n');
}

fn md_cell(raw: &str) -> String {
    raw.replace('|', "\\|").replace('\n', " ")
}

fn html_escape(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
