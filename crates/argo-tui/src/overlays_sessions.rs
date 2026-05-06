//! `O` orchestrator-sessions overlay (Stream
//! `claude-tui-h-stack-panels`). Reads from
//! `GET /orchestrator/sessions` (lands with H-orchestrator-routes).
//! Until that route lands the panel renders the unreachable hint.
use crate::overlays_h_stack::PanelState;
use ratatui::{
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use serde_json::Value;

/// Active orchestrator sessions. Triggered by `O`. Esc/O/q close.
/// Each row renders `<id> <title> <status> <participants>`.
pub(crate) fn draw_sessions(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &PanelState,
    scroll: usize,
) {
    let w = area.width.saturating_mul(8) / 10;
    let h = area.height.saturating_mul(8) / 10;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let panel = Rect::new(x, y, w.max(40), h.max(8));

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(format!(
        "  {:<12} {:<24} {:<10} participants",
        "id", "title", "status",
    )));
    let title = match state {
        PanelState::Loading => {
            lines.push(Line::from("  loading…"));
            " orchestrator sessions — Esc/O/q closes ".to_string()
        }
        PanelState::Unreachable(msg) => {
            lines.push(Line::from(format!("  {msg}")));
            " orchestrator sessions (unreachable) — Esc/O/q closes ".to_string()
        }
        PanelState::OkOne(_) => {
            lines.push(Line::from(
                "  (unexpected single object — expected array)",
            ));
            " orchestrator sessions — Esc/O/q closes ".to_string()
        }
        PanelState::Ok(rows) if rows.is_empty() => {
            lines.push(Line::from("  (no active sessions)"));
            " orchestrator sessions (0) — Esc/O/q closes ".to_string()
        }
        PanelState::Ok(rows) => {
            let clamped = scroll.min(rows.len().saturating_sub(1));
            for row in &rows[clamped..] {
                lines.push(Line::from(format!("  {}", format_session_row(row))));
            }
            format!(" orchestrator sessions ({}) — Esc/O/q closes ", rows.len())
        }
    };
    let para = Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(Clear, panel);
    frame.render_widget(para, panel);
}

fn format_session_row(row: &Value) -> String {
    let id = short_id(row.get("id"));
    let title = row
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("(untitled)");
    let status = row
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("?");
    let participants = row
        .get("participants")
        .and_then(Value::as_array)
        .map(|a| a.len())
        .unwrap_or(0);
    let title_trim = if title.len() > 24 {
        format!("{}…", &title[..23])
    } else {
        title.to_string()
    };
    format!(
        "{id:<12} {title_trim:<24} {status:<10} {participants}"
    )
}

fn short_id(v: Option<&Value>) -> String {
    let s = v.and_then(Value::as_str).unwrap_or("?");
    if s.len() > 12 {
        s[..12].to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn format_session_row_handles_long_titles() {
        let row = json!({
            "id": "01HXYZ123456789ABC",
            "title": "this title is much longer than twenty four chars",
            "status": "Active",
            "participants": ["w-1", "w-2"],
        });
        let s = format_session_row(&row);
        assert!(s.starts_with("01HXYZ123456"));
        assert!(s.contains("Active"));
        assert!(s.ends_with('2'));
    }

    #[test]
    fn format_session_row_handles_missing_fields() {
        let row = json!({});
        let s = format_session_row(&row);
        assert!(s.contains('?'));
        assert!(s.contains("(untitled)"));
        assert!(s.ends_with('0'));
    }

    #[test]
    fn short_id_truncates_long() {
        assert_eq!(short_id(Some(&json!("0123456789ABCDEF"))), "0123456789AB");
        assert_eq!(short_id(Some(&json!("short"))), "short");
        assert_eq!(short_id(None), "?");
    }
}
