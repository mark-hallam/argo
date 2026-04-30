//! Render layer — pure draw functions; no I/O. Tests can exercise
//! the rendering against a `TestBackend` without touching a real
//! terminal.
use crate::app::ViewerState;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

/// Top-level draw — splits the frame into a left list pane and a
/// right column. The right column stacks the session detail above
/// a scrolling messages pane. Footer carries a one-line keymap.
pub fn draw(frame: &mut Frame<'_>, state: &ViewerState) {
    let area = frame.area();
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(outer[0]);
    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(12), Constraint::Min(1)])
        .split(panes[1]);
    draw_session_list(frame, panes[0], state);
    draw_session_detail(frame, right[0], state);
    draw_messages_pane(frame, right[1], state);
    draw_footer(frame, outer[1], state);
}

fn draw_messages_pane(frame: &mut Frame<'_>, area: Rect, state: &ViewerState) {
    let items: Vec<ListItem> = state
        .selected_messages
        .iter()
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
    let title = match state.selected_session() {
        Some(_) if !state.selected_messages.is_empty() => {
            format!(" messages ({}) ", state.selected_messages.len())
        }
        Some(_) => " messages (none) ".to_string(),
        None => " messages ".to_string(),
    };
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(list, area);
}

fn first_line_of_content(content_json: &str) -> String {
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

fn draw_session_list(frame: &mut Frame<'_>, area: Rect, state: &ViewerState) {
    let indices = state.filtered_indices();
    let items: Vec<ListItem> = indices
        .iter()
        .filter_map(|i| state.sessions.get(*i))
        .map(|s| {
            let title = s.title.as_deref().unwrap_or("(untitled)");
            ListItem::new(Line::from(vec![
                Span::raw(format!("{} ", short_id(&s.id.0.to_string()))),
                Span::styled(
                    title.to_string(),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ]))
        })
        .collect();
    let title = if state.filter.is_empty() {
        format!(" sessions ({}) ", state.sessions.len())
    } else {
        format!(
            " sessions ({}/{} match '{}') ",
            indices.len(),
            state.sessions.len(),
            state.filter
        )
    };
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("▸ ");
    let mut list_state = ListState::default();
    if !indices.is_empty() {
        let clamped = state.selected.min(indices.len() - 1);
        list_state.select(Some(clamped));
    }
    frame.render_stateful_widget(list, area, &mut list_state);
}

fn draw_session_detail(frame: &mut Frame<'_>, area: Rect, state: &ViewerState) {
    let body = match state.selected_session() {
        None => "(no sessions in this workspace)".to_string(),
        Some(s) => {
            let mut lines = Vec::new();
            lines.push(format!("id:           {}", s.id.0));
            lines.push(format!("tenant:       {}", s.tenant_id.0));
            lines.push(format!("workspace:    {}", s.workspace_id.0));
            lines.push(format!("provider:     {}", s.provider));
            lines.push(format!("model:        {}", s.model));
            lines.push(format!(
                "title:        {}",
                s.title.as_deref().unwrap_or("(none)")
            ));
            lines.push(format!("started_at:   {}", s.started_at));
            lines.push(format!(
                "ended_at:     {}",
                s.ended_at
                    .map(|d| d.to_string())
                    .unwrap_or_else(|| "(open)".into())
            ));
            lines.push(format!(
                "messages:     {}",
                state
                    .selected_message_count
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "…".into())
            ));
            lines.join("\n")
        }
    };
    let p = Paragraph::new(body)
        .wrap(Wrap { trim: false })
        .block(Block::default().borders(Borders::ALL).title(" detail "));
    frame.render_widget(p, area);
}

fn draw_footer(frame: &mut Frame<'_>, area: Rect, state: &ViewerState) {
    let body = if state.filter_mode {
        format!(
            "/{}  (Enter applies, Esc clears, Backspace edits)",
            state.filter
        )
    } else if state.filter.is_empty() {
        "↑/k  ↓/j  PgUp/PgDn  g/G  /  filter    q/esc  quit".to_string()
    } else {
        format!(
            "filter='{}' ({} match)    /  edit    Esc  clear    q  quit",
            state.filter,
            state.filtered_indices().len()
        )
    };
    let p = Paragraph::new(body);
    frame.render_widget(p, area);
}

fn short_id(s: &str) -> String {
    s.chars().take(8).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_store::SessionHeader;
    use argo_types::{TenantId, WorkspaceId};
    use chrono::Utc;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn header(title: &str) -> SessionHeader {
        SessionHeader {
            id: argo_types::SessionId::new(),
            tenant_id: TenantId::new(),
            workspace_id: WorkspaceId::new(),
            model: "mock-model".into(),
            provider: "mock-provider".into(),
            title: Some(title.into()),
            started_at: Utc::now(),
            ended_at: None,
        }
    }

    fn render_to_strings(state: &ViewerState) -> String {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, state)).unwrap();
        let buffer = terminal.backend().buffer().clone();
        let mut out = String::new();
        for y in 0..24 {
            for x in 0..80 {
                out.push_str(buffer.cell((x, y)).map(|c| c.symbol()).unwrap_or(""));
            }
            out.push('\n');
        }
        out
    }

    #[test]
    fn empty_state_renders_no_sessions_message() {
        let state = ViewerState::new(Vec::new());
        let frame = render_to_strings(&state);
        assert!(frame.contains("(no sessions"));
        // Footer always shows the filter / quit keymap.
        assert!(frame.contains("quit"));
    }

    #[test]
    fn renders_session_titles_in_list() {
        let state = ViewerState::new(vec![header("morning"), header("afternoon")]);
        let frame = render_to_strings(&state);
        assert!(frame.contains("morning"));
        assert!(frame.contains("afternoon"));
    }

    #[test]
    fn renders_selected_detail_pane() {
        let state = ViewerState::new(vec![header("focus")]);
        let frame = render_to_strings(&state);
        assert!(frame.contains("provider:"));
        assert!(frame.contains("mock-provider"));
    }

    #[test]
    fn first_line_summarizes_tool_use_and_result() {
        let json = serde_json::json!([
            {"type":"tool_use","id":"01","name":"file_read","arguments":{"path":"/x"}}
        ])
        .to_string();
        assert!(first_line_of_content(&json).starts_with("→ file_read"));

        let res = serde_json::json!([
            {"type":"tool_result","id":"abc","is_error":true,
             "content":[{"type":"text","text":"boom"}]}
        ])
        .to_string();
        let line = first_line_of_content(&res);
        assert!(line.contains("← abc"));
        assert!(line.contains("[error]"));
        assert!(line.contains("boom"));
    }
}
