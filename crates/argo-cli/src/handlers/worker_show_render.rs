//! Markdown rendering for `argo worker show` (split out of
//! `worker_show.rs` to keep both files comfortably under the 500-LOC
//! file invariant - see brief `claude-worker-introspection-cli`).

use super::worker_show::{AuditRow, DutyRow, SessionRow, WorkerShowReport};
use argo_identity::{NotifyChannelRef, Persona, Role};

pub(crate) fn render_markdown(r: &WorkerShowReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "# Worker {} - `{}`\n\n",
        r.worker.persona.name, r.worker.id
    ));
    out.push_str(&render_persona(&r.worker.persona));
    out.push_str(&render_role(&r.worker.role));
    out.push_str(&render_charter(r.worker.charter.as_deref()));
    out.push_str(&render_channels(&r.worker.channels));
    out.push_str(&render_sessions(&r.recent_sessions));
    out.push_str(&render_audit(&r.recent_audit));
    out.push_str(&render_duties(&r.duties));
    out.push_str(&render_spend(r.spend_ytd_usd_micros));
    out.push_str(&format!("_Generated at {}_\n", r.generated_at));
    out
}

fn render_persona(p: &Persona) -> String {
    let mut out = String::from("## Persona\n\n");
    out.push_str(&format!("- Name: `{}`\n", p.name));
    out.push_str(&format!("- Builder: `{}`\n", p.builder_attribution));
    if let Some(bio) = &p.bio {
        out.push_str(&format!("- Bio: {bio}\n"));
    }
    out.push_str(&format!("- Voice preamble: {}\n\n", p.voice_profile.preamble));
    out
}

fn render_role(role: &Role) -> String {
    let mut out = String::from("## Role\n\n");
    out.push_str(&format!("- Charter: {}\n", role.charter));
    if let Some(esc) = &role.escalation {
        out.push_str(&format!("- Escalation: {esc}\n"));
    }
    if !role.kpis.is_empty() {
        out.push_str("- KPIs:\n");
        for kpi in &role.kpis {
            out.push_str(&format!("  - {kpi}\n"));
        }
    }
    out.push('\n');
    out
}

fn render_charter(charter: Option<&str>) -> String {
    let body = match charter {
        Some(s) if !s.is_empty() => format!("```markdown\n{}\n```\n\n", s.trim_end()),
        _ => "(none)\n\n".to_string(),
    };
    format!("## Charter\n\n{body}")
}

fn render_channels(channels: &[NotifyChannelRef]) -> String {
    let mut out = String::from("## Channels\n\n");
    if channels.is_empty() {
        out.push_str("(none)\n\n");
        return out;
    }
    for c in channels {
        let risk = c
            .risk_level
            .as_deref()
            .map(|r| format!(" - risk={r}"))
            .unwrap_or_default();
        out.push_str(&format!("- `{}` -> `{}`{}\n", c.channel, c.target, risk));
    }
    out.push('\n');
    out
}

fn render_sessions(rows: &[SessionRow]) -> String {
    let mut out = String::from("## Recent Sessions\n\n");
    if rows.is_empty() {
        out.push_str("(none)\n\n");
        return out;
    }
    for r in rows {
        let v = r.vertical_id.as_deref().unwrap_or("-");
        let title = r.title.as_deref().unwrap_or("");
        out.push_str(&format!("- `{}` ({}) {} {}\n", r.id, v, r.started_at, title));
    }
    out.push('\n');
    out
}

fn render_audit(rows: &[AuditRow]) -> String {
    let mut out = String::from("## Recent Audit\n\n");
    if rows.is_empty() {
        out.push_str("(none)\n\n");
        return out;
    }
    for r in rows {
        out.push_str(&format!(
            "- seq={} kind=`{}` at {}\n",
            r.seq, r.kind, r.created_at
        ));
    }
    out.push('\n');
    out
}

fn render_duties(rows: &[DutyRow]) -> String {
    let mut out = String::from("## Duties\n\n");
    if rows.is_empty() {
        out.push_str("(none)\n\n");
        return out;
    }
    for r in rows {
        out.push_str(&format!(
            "- `{}` `{}` -> `{}` ({}){}\n",
            r.id,
            r.cron,
            r.entry_action,
            r.description,
            if r.enabled { "" } else { " [paused]" }
        ));
    }
    out.push('\n');
    out
}

fn render_spend(spend_micros: u64) -> String {
    let usd = spend_micros as f64 / 1_000_000.0;
    format!("## Spend YTD\n\n- `${usd:.2}` ({spend_micros} usd-micros)\n\n")
}
