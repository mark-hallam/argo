//! `B` budget overlay (Stream `claude-tui-h-stack-panels`). Reads
//! from `GET /budget/summary` (lands with H-orchestrator-routes —
//! gateway endpoint counterpart to `argo budget show`). Renders
//! today's spend / monthly rollup / remaining headroom against
//! the workspace's policy caps.
//!
//! Disambiguated from the existing `$` cost-rollup overlay (that
//! one is workspace-wide spend across providers; this one is the
//! H-1 worker-budget surface — auto-approval headroom view).
use crate::overlays_h_stack::PanelState;
use ratatui::{
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use serde_json::Value;

/// Budget summary overlay. Triggered by `B`. Esc/B/q close.
/// Renders today's spend / monthly rollup / remaining-today
/// headroom against the policy caps.
pub(crate) fn draw_budget(frame: &mut Frame<'_>, area: Rect, state: &PanelState) {
    let w = area.width.min(64);
    let h = area.height.min(14);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let panel = Rect::new(x, y, w.max(40), h.max(8));

    let mut lines: Vec<Line> = Vec::new();
    let title = match state {
        PanelState::Loading => {
            lines.push(Line::from("  loading budget summary…"));
            " budget — Esc/B/q closes ".to_string()
        }
        PanelState::Unreachable(msg) => {
            lines.push(Line::from(format!("  {msg}")));
            " budget (unreachable) — Esc/B/q closes ".to_string()
        }
        PanelState::Ok(_) => {
            lines.push(Line::from(
                "  (unexpected JSON array — expected single object)",
            ));
            " budget — Esc/B/q closes ".to_string()
        }
        PanelState::OkOne(v) => {
            for line in format_budget_lines(v) {
                lines.push(Line::from(format!("  {line}")));
            }
            " budget — Esc/B/q closes ".to_string()
        }
    };
    let para = Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(Clear, panel);
    frame.render_widget(para, panel);
}

/// Project a budget JSON object into a small list of human lines.
/// Tolerant of missing keys — any field that's absent renders as
/// "(unset)" so the operator immediately sees what the gateway
/// returned vs. what's missing.
pub(crate) fn format_budget_lines(v: &Value) -> Vec<String> {
    let worker = v
        .get("worker_id")
        .and_then(Value::as_str)
        .unwrap_or("(workspace)");
    let today_usd = v
        .get("today_usd")
        .and_then(Value::as_f64)
        .map(|f| format!("${f:.4}"))
        .unwrap_or_else(|| "(unset)".to_string());
    let month_usd = v
        .get("this_month_usd")
        .and_then(Value::as_f64)
        .map(|f| format!("${f:.4}"))
        .unwrap_or_else(|| "(unset)".to_string());
    let cap = v
        .get("daily_cap_usd_micros")
        .and_then(Value::as_u64)
        .map(|m| format!("${:.4}", m as f64 / 1_000_000.0))
        .unwrap_or_else(|| "(no cap)".to_string());
    let remaining = v
        .get("remaining_today_usd")
        .and_then(Value::as_f64)
        .map(|f| format!("${f:.4}"))
        .unwrap_or_else(|| "(no cap)".to_string());
    let last = v
        .get("last_charges")
        .and_then(Value::as_array)
        .map(|a| a.len())
        .unwrap_or(0);
    vec![
        format!("worker:           {worker}"),
        format!("today:            {today_usd}"),
        format!("this month:       {month_usd}"),
        format!("daily cap:        {cap}"),
        format!("remaining today:  {remaining}"),
        format!("recent charges:   {last}"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn format_budget_lines_renders_full_payload() {
        let v = json!({
            "worker_id": "w-1",
            "today_usd": 1.5,
            "this_month_usd": 42.0,
            "daily_cap_usd_micros": 5_000_000_u64,
            "remaining_today_usd": 3.5,
            "last_charges": [{"a": 1}, {"a": 2}, {"a": 3}],
        });
        let lines = format_budget_lines(&v);
        assert!(lines[0].contains("w-1"));
        assert!(lines[1].contains("$1.5000"));
        assert!(lines[2].contains("$42.0000"));
        assert!(lines[3].contains("$5.0000"));
        assert!(lines[4].contains("$3.5000"));
        assert!(lines[5].contains('3'));
    }

    #[test]
    fn format_budget_lines_handles_empty_payload() {
        let v = json!({});
        let lines = format_budget_lines(&v);
        assert!(lines[0].contains("(workspace)"));
        assert!(lines[1].contains("(unset)"));
        assert!(lines[3].contains("(no cap)"));
        assert!(lines[5].ends_with('0'));
    }
}
