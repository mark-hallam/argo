//! `H` orchestrator-handoffs overlay (Stream
//! `claude-tui-h-stack-panels`). Reads from
//! `GET /orchestrator/handoffs?status=pending` (lands with
//! H-orchestrator-routes). Renders one row per pending handoff
//! with from/to/expected_outcome/deadline so the operator knows
//! who's waiting on what.
use crate::overlays_h_stack::PanelState;
use ratatui::{
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use serde_json::Value;

/// Pending orchestrator handoffs. Triggered by `H`. Esc/H/q close.
/// Each row renders `<id> <from> <to> <status> <deadline>`.
pub(crate) fn draw_handoffs(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &PanelState,
    scroll: usize,
) {
    let w = area.width.saturating_mul(9) / 10;
    let h = area.height.saturating_mul(8) / 10;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let panel = Rect::new(x, y, w.max(40), h.max(8));

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(format!(
        "  {:<12} {:<16} {:<16} {:<10} deadline",
        "id", "from", "to", "status",
    )));
    let title = match state {
        PanelState::Loading => {
            lines.push(Line::from("  loading…"));
            " orchestrator handoffs — Esc/H/q closes ".to_string()
        }
        PanelState::Unreachable(msg) => {
            lines.push(Line::from(format!("  {msg}")));
            " orchestrator handoffs (unreachable) — Esc/H/q closes ".to_string()
        }
        PanelState::OkOne(_) => {
            lines.push(Line::from(
                "  (unexpected single object — expected array)",
            ));
            " orchestrator handoffs — Esc/H/q closes ".to_string()
        }
        PanelState::Ok(rows) if rows.is_empty() => {
            lines.push(Line::from("  (no pending handoffs)"));
            " orchestrator handoffs (0) — Esc/H/q closes ".to_string()
        }
        PanelState::Ok(rows) => {
            let clamped = scroll.min(rows.len().saturating_sub(1));
            for row in &rows[clamped..] {
                lines.push(Line::from(format!("  {}", format_handoff_row(row))));
            }
            format!(" orchestrator handoffs ({}) — Esc/H/q closes ", rows.len())
        }
    };
    let para = Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(Clear, panel);
    frame.render_widget(para, panel);
}

fn format_handoff_row(row: &Value) -> String {
    let id = trim(row.get("id").and_then(Value::as_str).unwrap_or("?"), 12);
    let from = trim(row.get("from").and_then(Value::as_str).unwrap_or("?"), 16);
    let to = trim(format_to(row.get("to")).as_str(), 16);
    let status = row
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("?");
    let deadline = row
        .get("deadline")
        .and_then(Value::as_str)
        .unwrap_or("(none)");
    format!("{id:<12} {from:<16} {to:<16} {status:<10} {deadline}")
}

fn format_to(v: Option<&Value>) -> String {
    let v = match v {
        Some(v) => v,
        None => return "?".to_string(),
    };
    if let Some(s) = v.as_str() {
        return s.to_string();
    }
    if let Some(o) = v.as_object() {
        if let Some(w) = o.get("Worker").and_then(Value::as_str) {
            return format!("worker:{w}");
        }
        if let Some(h) = o.get("Human").and_then(Value::as_str) {
            return format!("human:{h}");
        }
        if let Some(c) = o.get("Channel").and_then(Value::as_str) {
            return format!("channel:{c}");
        }
    }
    v.to_string()
}

fn trim(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}…", &s[..max.saturating_sub(1)])
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn format_handoff_row_renders_string_to_field() {
        let row = json!({
            "id": "01HHH",
            "from": "worker-a",
            "to": "worker-b",
            "status": "Pending",
            "deadline": "2026-05-07T00:00:00Z",
        });
        let s = format_handoff_row(&row);
        assert!(s.contains("worker-a"));
        assert!(s.contains("worker-b"));
        assert!(s.contains("Pending"));
        assert!(s.contains("2026-05-07"));
    }

    #[test]
    fn format_to_handles_escalation_step_variants() {
        assert_eq!(format_to(Some(&json!({"Worker": "w-1"}))), "worker:w-1");
        assert_eq!(format_to(Some(&json!({"Human": "alice"}))), "human:alice");
        assert_eq!(
            format_to(Some(&json!({"Channel": "slack"}))),
            "channel:slack"
        );
        assert_eq!(format_to(Some(&json!("plain-string"))), "plain-string");
        assert_eq!(format_to(None), "?");
    }

    #[test]
    fn trim_truncates_long_strings() {
        assert_eq!(trim("0123456789ABCDEF", 8), "0123456…");
        assert_eq!(trim("short", 8), "short");
    }
}
