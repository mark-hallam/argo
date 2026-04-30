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
/// right detail pane. Footer carries a one-line keymap.
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
    draw_session_list(frame, panes[0], state);
    draw_session_detail(frame, panes[1], state);
    draw_footer(frame, outer[1]);
}

fn draw_session_list(frame: &mut Frame<'_>, area: Rect, state: &ViewerState) {
    let items: Vec<ListItem> = state
        .sessions
        .iter()
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
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" sessions "))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("▸ ");
    let mut list_state = ListState::default();
    if !state.sessions.is_empty() {
        list_state.select(Some(state.selected));
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

fn draw_footer(frame: &mut Frame<'_>, area: Rect) {
    let p = Paragraph::new("↑/k  ↓/j  navigate    q/esc  quit");
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
        assert!(frame.contains("navigate"));
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
}
