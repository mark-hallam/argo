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
    crate::render_messages::draw_messages_pane(frame, right[1], state);
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
    if let Some(rows) = state.tool_errors_overlay.as_ref() {
        crate::overlays_tool_errors::draw_tool_errors(
            frame,
            area,
            rows,
            state.tool_errors_overlay_scroll,
        );
    }
    if let Some(panel) = state.h_sessions_overlay.as_ref() {
        crate::overlays_sessions::draw_sessions(
            frame,
            area,
            panel,
            state.h_sessions_overlay_scroll,
        );
    }
    if let Some(panel) = state.h_handoffs_overlay.as_ref() {
        crate::overlays_handoffs::draw_handoffs(
            frame,
            area,
            panel,
            state.h_handoffs_overlay_scroll,
        );
    }
    if let Some(panel) = state.h_budget_overlay.as_ref() {
        crate::overlays_budget::draw_budget(frame, area, panel);
    }
    if let Some(replay) = state.replay_overlay.as_ref() {
        crate::overlays_replay::draw_replay(frame, area, replay);
    }
    if let Some(dash) = state.dashboard_overlay.as_ref() {
        crate::overlays_dashboard::draw_dashboard(frame, area, dash);
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

// Re-exported so `render_tests.rs` (mounted via `super::*`) can keep
// referencing `first_line_of_content` without a path change.
#[cfg(test)]
pub(crate) use crate::render_messages::first_line_of_content;

fn draw_session_list(frame: &mut Frame<'_>, area: Rect, state: &ViewerState) {
    let indices = state.filtered_indices();
    let items: Vec<ListItem> = indices
        .iter()
        .filter_map(|i| state.sessions.get(*i))
        .map(|s| {
            let title = s.title.as_deref().unwrap_or("(untitled)");
            // ● = still open (ended_at IS NULL), ○ = ended.
            let marker = if s.ended_at.is_none() { "●" } else { "○" };
            // #699: vertical badge surfaces the binding when set
            // — matches the CLI list column added in #690 so
            // operators see the same signal across surfaces.
            // Unbound rows render no badge (zero-cost) so legacy
            // workspaces stay tidy.
            let mut spans = vec![
                Span::raw(format!("{marker} {} ", short_id(&s.id.0.to_string()))),
                Span::styled(
                    title.to_string(),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ];
            if let Some(v) = s.vertical_id.as_deref() {
                spans.push(Span::styled(
                    format!(" [{v}]"),
                    Style::default().add_modifier(Modifier::DIM),
                ));
            }
            ListItem::new(Line::from(spans))
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
            // #699: vertical line — present only when the
            // session is bound. Mirrors the CLI `argo session
            // show` text view from #689.
            if let Some(v) = s.vertical_id.as_deref() {
                lines.push(format!("vertical:     {v}"));
            }
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

#[cfg(test)]
#[path = "render_h_stack_tests.rs"]
mod h_stack_tests;
