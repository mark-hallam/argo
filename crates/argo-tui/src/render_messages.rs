//! Messages-pane render — split from `render.rs` to keep that file
//! under the 500-LOC ceiling. Pure draw functions (no I/O) plus the
//! best-effort content-block summarisers used to render each row.
use crate::app::{PaneFocus, ViewerState};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::layout::Rect;
use ratatui::Frame;

pub(crate) fn draw_messages_pane(frame: &mut Frame<'_>, area: Rect, state: &ViewerState) {
    let scroll = state.messages_scroll.min(state.selected_messages.len());
    let mut items: Vec<ListItem> = state
        .selected_messages
        .iter()
        .skip(scroll)
        .map(|m| {
            let body = first_line_of_content(&m.content_json);
            ListItem::new(Line::from(vec![
                Span::raw(format!("[{:>3}] ", m.seq)),
                Span::styled(
                    format!("{:<10}", m.role),
                    Style::default().add_modifier(Modifier::DIM),
                ),
                Span::raw(body),
            ]))
        })
        .collect();
    // Append the live-event trail (if any) at the bottom so the
    // user sees streaming progress without leaving the TUI. The
    // trail is auto-cleared when a new turn spawns.
    for line in &state.live_events {
        items.push(ListItem::new(Line::from(vec![Span::styled(
            line.clone(),
            Style::default().add_modifier(Modifier::DIM),
        )])));
    }
    let title = match state.selected_session() {
        Some(_) if !state.selected_messages.is_empty() => {
            let live = if state.live_events.is_empty() {
                String::new()
            } else {
                format!(" + {} live", state.live_events.len())
            };
            let scroll_tag = if scroll > 0 {
                format!(" @{scroll}")
            } else {
                String::new()
            };
            format!(
                " messages ({}{live}{scroll_tag}) ",
                state.selected_messages.len()
            )
        }
        Some(_) => " messages (none) ".to_string(),
        None => " messages ".to_string(),
    };
    let mut block = Block::default().borders(Borders::ALL).title(title);
    if matches!(state.focus, PaneFocus::Messages) {
        block = block.border_style(Style::default().add_modifier(Modifier::BOLD));
    }
    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

pub(crate) fn first_line_of_content(content_json: &str) -> String {
    // Best-effort summary: prefer the first text block; otherwise
    // surface the first tool_use / tool_result so tool-dispatch
    // turns don't render as walls of escaped JSON. Fall back to
    // the raw content_json truncated to 80 chars on parse failure.
    let parsed: serde_json::Value =
        serde_json::from_str(content_json).unwrap_or(serde_json::Value::Null);
    if let Some(arr) = parsed.as_array() {
        if let Some(line) = arr.iter().find_map(content_block_summary) {
            return line.chars().take(80).collect();
        }
    }
    content_json.chars().take(80).collect()
}

/// Single-line label for a content block, prioritising text > tool_use > tool_result.
fn content_block_summary(b: &serde_json::Value) -> Option<String> {
    let kind = b.get("type").and_then(|v| v.as_str())?;
    match kind {
        "text" => b
            .get("text")
            .and_then(|v| v.as_str())
            .and_then(|t| t.lines().next())
            .map(str::to_string),
        "tool_use" => {
            let name = b.get("name").and_then(|v| v.as_str()).unwrap_or("?");
            Some(format!("→ {name}"))
        }
        "tool_result" => {
            let id = b.get("id").and_then(|v| v.as_str()).unwrap_or("?");
            let is_error = b.get("is_error").and_then(|v| v.as_bool()).unwrap_or(false);
            let body = b
                .get("content")
                .and_then(|c| c.as_array())
                .and_then(|a| a.iter().find_map(|cb| cb.get("text").and_then(|t| t.as_str())))
                .unwrap_or("");
            let tag = if is_error { " [error]" } else { "" };
            Some(format!("← {id}{tag}: {body}"))
        }
        _ => None,
    }
}
