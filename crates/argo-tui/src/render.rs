//! Render layer — pure draw functions; no I/O. Tests can exercise
//! the rendering against a `TestBackend` without touching a real
//! terminal.
use crate::app::{PaneFocus, ViewerState};
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
    // Outer rows: main · sticky composer · footer.
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
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
    draw_composer(frame, outer[1], state);
    draw_footer(frame, outer[2], state);
    if state.help_overlay {
        crate::overlays::draw_help(frame, area);
    }
    if let Some(idx) = state.message_overlay {
        crate::overlays::draw_message(frame, area, state, idx);
    }
    if let Some(rows) = state.forge_runs.as_ref() {
        crate::overlays::draw_forge_runs(frame, area, rows, state.forge_runs_scroll);
    }
    if let Some(rows) = state.skills_overlay.as_ref() {
        crate::overlays::draw_skills(frame, area, rows, state.skills_overlay_scroll);
    }
    if let Some(rows) = state.memory_overlay.as_ref() {
        crate::overlays::draw_memory(frame, area, rows, state.memory_overlay_scroll);
    }
    if let Some(snap) = state.cost_overlay.as_ref() {
        crate::overlays::draw_cost(frame, area, snap);
    }
    if let Some(rows) = state.audit_overlay.as_ref() {
        crate::overlays::draw_audit(frame, area, rows, state.audit_overlay_scroll);
    }
    if let Some(rows) = state.providers_overlay.as_ref() {
        crate::overlays::draw_providers(frame, area, rows, state.providers_overlay_scroll);
    }
    if let Some(rows) = state.cron_overlay.as_ref() {
        crate::overlays::draw_cron(frame, area, rows, state.cron_overlay_scroll);
    }
    if let Some(rows) = state.tools_overlay.as_ref() {
        crate::overlays::draw_tools(frame, area, rows, state.tools_overlay_scroll);
    }
}

fn draw_composer(frame: &mut Frame<'_>, area: Rect, state: &ViewerState) {
    let title = if state.composer_mode {
        " composer (Enter=send · Esc=cancel) ".to_string()
    } else {
        " composer (i=edit) ".to_string()
    };
    let style = if state.composer_mode {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default().add_modifier(Modifier::DIM)
    };
    let body = if state.composer.is_empty() && !state.composer_mode {
        match &state.last_sent {
            Some(s) => format!("(last sent: {}…)", s.chars().take(60).collect::<String>()),
            None => String::new(),
        }
    } else {
        state.composer.clone()
    };
    let para = Paragraph::new(body).style(style).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title),
    );
    frame.render_widget(para, area);
}

fn draw_messages_pane(frame: &mut Frame<'_>, area: Rect, state: &ViewerState) {
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
            // ● = still open (ended_at IS NULL), ○ = ended.
            let marker = if s.ended_at.is_none() { "●" } else { "○" };
            ListItem::new(Line::from(vec![
                Span::raw(format!("{marker} {} ", short_id(&s.id.0.to_string()))),
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
    let mut block = Block::default().borders(Borders::ALL).title(title);
    if matches!(state.focus, PaneFocus::Sessions) {
        block = block.border_style(Style::default().add_modifier(Modifier::BOLD));
    }
    let list = List::new(items)
        .block(block)
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
            // Server-side elapsed: end-or-now minus started_at.
            let now = chrono::Utc::now();
            let end = s.ended_at.unwrap_or(now);
            let secs = (end - s.started_at).num_seconds().max(0);
            lines.push(format!("elapsed:      {}", fmt_duration(secs)));
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
    // Agent-status line takes priority over keymap so the user
    // sees an in-flight turn even when filter/composer are idle.
    if state.agent_running() {
        let n = state.live_events.len();
        let elapsed = state
            .agent_started_at
            .map(|t| fmt_duration(t.elapsed().as_secs() as i64))
            .unwrap_or_else(|| "?".into());
        let body = if n == 0 {
            format!("● agent running [{elapsed}]… (assistant reply will appear above)")
        } else {
            format!("● agent running [{elapsed}]… ({n} live event{plural} so far)",
                plural = if n == 1 { "" } else { "s" })
        };
        let p = Paragraph::new(body);
        frame.render_widget(p, area);
        return;
    }
    if let Some(err) = state.last_agent_error.as_deref() {
        let body = format!("✗ agent error: {} (press i to retry)", first_line(err));
        let p = Paragraph::new(body);
        frame.render_widget(p, area);
        return;
    }
    if state.composer_mode {
        let p = Paragraph::new("composer: type, Enter=send, Esc=cancel");
        frame.render_widget(p, area);
        return;
    }
    if let Some(buf) = state.rename_buffer.as_deref() {
        let p = Paragraph::new(format!(
            "rename: '{buf}'  (Enter saves, Esc cancels, Backspace edits)"
        ));
        frame.render_widget(p, area);
        return;
    }
    if let Some(buf) = state.steer_buffer.as_deref() {
        let p = Paragraph::new(format!(
            "steer: '{buf}'  (Enter persists [steer] msg, Esc cancels)"
        ));
        frame.render_widget(p, area);
        return;
    }
    if state.pending_delete.is_some() {
        let title = state
            .selected_session()
            .and_then(|s| s.title.as_deref())
            .unwrap_or("(untitled)");
        let p = Paragraph::new(format!(
            "delete '{title}'?  press y to confirm, any other key to cancel"
        ));
        frame.render_widget(p, area);
        return;
    }
    let body = if state.filter_mode {
        format!(
            "/{}  (Enter applies, Esc clears, Backspace edits)",
            state.filter
        )
    } else if state.filter.is_empty() {
        "?=help  j/k  Tab focus  / filter  i comp  s steer  n new  r ren  d del  q quit"
            .to_string()
    } else {
        format!(
            "filter='{}' ({} match)    /  edit    Esc  clear    q  quit",
            state.filter,
            state.filtered_indices().len()
        )
    };
    // Prepend the workspace tag if we know it; lets the user
    // tell at a glance which workspace they're viewing.
    let body = match state.workspace_label.as_deref() {
        Some(ws) => format!("[ws={ws}]  {body}"),
        None => body,
    };
    let p = Paragraph::new(body);
    frame.render_widget(p, area);
}

fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or("").chars().take(80).collect()
}

/// Humanize a duration in seconds — `42s`, `7m12s`, `1h3m`,
/// `2d4h`. Used by the session-detail pane to render
/// `elapsed_seconds` without wall-clock arithmetic mid-render.
fn fmt_duration(seconds: i64) -> String {
    if seconds < 60 {
        return format!("{seconds}s");
    }
    if seconds < 3600 {
        return format!("{m}m{s}s", m = seconds / 60, s = seconds % 60);
    }
    if seconds < 86400 {
        return format!("{h}h{m}m", h = seconds / 3600, m = (seconds % 3600) / 60);
    }
    format!("{d}d{h}h", d = seconds / 86400, h = (seconds % 86400) / 3600)
}

fn short_id(s: &str) -> String {
    s.chars().take(8).collect()
}


#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
