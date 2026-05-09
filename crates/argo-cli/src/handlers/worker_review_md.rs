//! Markdown renderer for `argo worker review`. Pure given a
//! `ReviewReport` — split out of the handler so the core file stays
//! comfortably under the 500-LOC ceiling.

use super::worker_review::{
    AnomalySection, FeedbackSample, HandoffSample, KpiDelta, ReviewReport, SuccessSection,
    SummarySection,
};

/// Render the canonical operator-facing markdown packet.
pub fn render_markdown(report: &ReviewReport) -> String {
    let mut out = String::new();
    let title = report.worker_name.as_deref().unwrap_or("(unknown worker)");
    out.push_str(&format!("# Worker {title} — {} review\n", report.period));
    out.push_str(&format!(
        "_id: `{}` · period start: {} · generated: {}_\n\n",
        report.worker_id, report.period_start_rfc3339, report.generated_at_rfc3339,
    ));
    push_summary(&mut out, &report.summary);
    push_kpis(&mut out, &report.kpis);
    push_success(&mut out, &report.success);
    push_feedback(&mut out, &report.feedback);
    push_handoffs(&mut out, &report.handoffs_sent, &report.handoffs_received);
    push_anomalies(&mut out, report.anomalies.as_ref());
    out
}

fn push_summary(out: &mut String, s: &SummarySection) {
    out.push_str("## Summary\n\n");
    let last = s.last_session_rfc3339.as_deref().unwrap_or("(none)");
    out.push_str(&format!("- Sessions: **{}**\n", s.session_count));
    out.push_str(&format!("- Last session: {last}\n\n"));
}

fn push_kpis(out: &mut String, kpis: &[KpiDelta]) {
    out.push_str("## KPIs\n\n");
    if kpis.is_empty() {
        out.push_str("_No KPI observations._\n\n");
        return;
    }
    out.push_str("| KPI | baseline | latest | delta | observed |\n");
    out.push_str("|---|---:|---:|---:|---|\n");
    for k in kpis {
        out.push_str(&format!(
            "| {} | {} | {:.4} | {} | {} |\n",
            k.kpi,
            k.baseline.map(|v| format!("{v:.4}")).unwrap_or_else(|| "n/a".into()),
            k.latest,
            k.delta.map(|v| format!("{v:+.4}")).unwrap_or_else(|| "n/a".into()),
            k.latest_at_rfc3339,
        ));
    }
    out.push('\n');
}

fn push_success(out: &mut String, s: &SuccessSection) {
    out.push_str("## Success rate\n\n");
    out.push_str(&format!(
        "- Decisions in window: **{}** (granted: {}, revoked: {}, consumed: {})\n",
        s.total, s.granted, s.revoked, s.consumed,
    ));
    out.push_str(&format!(
        "- Approval rate: **{}**\n\n",
        format_rate(s.approval_rate),
    ));
}

fn push_feedback(out: &mut String, samples: &[FeedbackSample]) {
    out.push_str("## Operator feedback\n\n");
    if samples.is_empty() {
        out.push_str("_No operator feedback._\n\n");
        return;
    }
    for f in samples {
        let comment = if f.comment.is_empty() {
            "(no comment)".to_string()
        } else {
            f.comment.clone()
        };
        out.push_str(&format!(
            "- `{}` on `{}` at {} — {}\n",
            f.vote, f.action_id, f.given_at_rfc3339, comment,
        ));
    }
    out.push('\n');
}

fn push_handoffs(out: &mut String, sent: &[HandoffSample], received: &[HandoffSample]) {
    out.push_str("## Handoffs\n\n");
    out.push_str("### Sent\n\n");
    if sent.is_empty() {
        out.push_str("_None._\n\n");
    } else {
        for h in sent {
            push_handoff_line(out, h, "to");
        }
        out.push('\n');
    }
    out.push_str("### Received\n\n");
    if received.is_empty() {
        out.push_str("_None._\n\n");
    } else {
        for h in received {
            push_handoff_line(out, h, "from");
        }
        out.push('\n');
    }
}

fn push_handoff_line(out: &mut String, h: &HandoffSample, direction: &str) {
    out.push_str(&format!(
        "- `{}` {direction} `{}` [{}] at {} — {}\n",
        h.handoff_id, h.counterpart, h.status, h.created_at_rfc3339, h.expected_outcome,
    ));
}

fn push_anomalies(out: &mut String, section: Option<&AnomalySection>) {
    out.push_str("## Anomalies\n\n");
    match section {
        Some(a) => out.push_str(&format!("- Events in window: **{}**\n", a.count)),
        None => out.push_str("_Anomaly tracking not enabled (worker_anomaly_event missing)._\n"),
    }
}

/// Format an approval rate (`0.0..=1.0`) as a percentage; `n/a` for
/// `None`. Matches the worker_metrics convention.
pub fn format_rate(rate: Option<f64>) -> String {
    match rate {
        Some(r) => format!("{:.2}%", r * 100.0),
        None => "n/a".to_string(),
    }
}
