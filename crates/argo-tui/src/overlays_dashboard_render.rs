//! Rendering primitives for the `D` real-time dashboard overlay.
//! Split out from `overlays_dashboard.rs` to keep the parent module
//! under the 500-LOC ceiling — the parent now owns state + formatter
//! helpers, this sibling owns the ratatui pane drawers.
//!
//! Public surface re-export: [`crate::overlays_dashboard`] re-exports
//! [`draw_dashboard`] via `pub(crate) use`, so external callers (and
//! the sibling tests file) keep their existing
//! `crate::overlays_dashboard::draw_dashboard` import path.
use crate::overlays_dashboard::{
    format_pending_line, format_worker_line, AuditTailEntry, DashboardState,
};
use crate::overlays_h_stack::PanelState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

/// Render the dashboard. The overlay covers the WHOLE frame (no
/// modal centering) — it's meant to BE the operator's screen, not
/// a pop-up. The 3 horizontal stripes are sized
/// 30 % / 30 % / 40 % so the audit tail (which is the busiest
/// pane) gets the most rows.
pub(crate) fn draw_dashboard(frame: &mut Frame<'_>, area: Rect, state: &DashboardState) {
    frame.render_widget(Clear, area);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(30),
            Constraint::Percentage(40),
        ])
        .split(area);
    draw_workers_pane(frame, rows[0], &state.workers);
    draw_pending_pane(frame, rows[1], &state.pending);
    draw_audit_pane(
        frame,
        rows[2],
        &state.audit_tail,
        state.audit_scroll,
        state.audit_loading,
    );
}

fn draw_workers_pane(frame: &mut Frame<'_>, area: Rect, panel: &PanelState) {
    let title = " workers — D/Esc/q closes ".to_string();
    match panel {
        PanelState::Loading => {
            let p = Paragraph::new(Line::from("  loading workers…")).block(panel_block(&title));
            frame.render_widget(p, area);
        }
        PanelState::Unreachable(msg) => {
            let p = Paragraph::new(Line::from(format!("  {msg}"))).block(panel_block(&title));
            frame.render_widget(p, area);
        }
        PanelState::OkOne(_) => {
            let p = Paragraph::new(Line::from(
                "  (unexpected single-object payload — expected JSON array)",
            ))
            .block(panel_block(&title));
            frame.render_widget(p, area);
        }
        PanelState::Ok(rows) => {
            let items: Vec<ListItem> = if rows.is_empty() {
                vec![ListItem::new(Line::from("  (no active workers)"))]
            } else {
                rows.iter()
                    .map(|v| ListItem::new(Line::from(format!("  {}", format_worker_line(v)))))
                    .collect()
            };
            let list = List::new(items).block(panel_block(&title));
            frame.render_widget(list, area);
        }
    }
}

fn draw_pending_pane(frame: &mut Frame<'_>, area: Rect, panel: &PanelState) {
    let title = " pending approvals (newest first) ".to_string();
    let now_micros = chrono::Utc::now().timestamp_micros();
    match panel {
        PanelState::Loading => {
            let p = Paragraph::new(Line::from("  loading pending approvals…"))
                .block(panel_block(&title));
            frame.render_widget(p, area);
        }
        PanelState::Unreachable(msg) => {
            let p = Paragraph::new(Line::from(format!("  {msg}"))).block(panel_block(&title));
            frame.render_widget(p, area);
        }
        PanelState::OkOne(_) => {
            let p = Paragraph::new(Line::from(
                "  (unexpected single-object payload — expected JSON array)",
            ))
            .block(panel_block(&title));
            frame.render_widget(p, area);
        }
        PanelState::Ok(rows) => {
            let items: Vec<ListItem> = if rows.is_empty() {
                vec![ListItem::new(Line::from("  (no pending approvals)"))]
            } else {
                rows.iter()
                    .map(|v| {
                        ListItem::new(Line::from(format!(
                            "  {}",
                            format_pending_line(v, now_micros)
                        )))
                    })
                    .collect()
            };
            let list = List::new(items).block(panel_block(&title));
            frame.render_widget(list, area);
        }
    }
}

fn draw_audit_pane(
    frame: &mut Frame<'_>,
    area: Rect,
    entries: &[AuditTailEntry],
    scroll: usize,
    loading: bool,
) {
    let title = format!(" audit tail (j/k arrows scroll) — {} rows ", entries.len());
    if loading && entries.is_empty() {
        let p =
            Paragraph::new(Line::from("  loading audit tail…")).block(panel_block(&title));
        frame.render_widget(p, area);
        return;
    }
    if entries.is_empty() {
        let p = Paragraph::new(Line::from("  (no audit rows yet)")).block(panel_block(&title));
        frame.render_widget(p, area);
        return;
    }
    // Render newest-first (entries are stored newest last from
    // `set_audit_tail`). Apply the scroll offset by skipping rows
    // from the head of the newest-first iteration.
    let mut items: Vec<ListItem> = entries
        .iter()
        .rev()
        .skip(scroll)
        .map(|e| {
            let session = e.session_short.as_deref().unwrap_or("(no-session)");
            let cursor = if e.kind.contains("error") {
                Span::styled("  ! ", Style::default().add_modifier(Modifier::BOLD))
            } else {
                Span::raw("    ")
            };
            ListItem::new(Line::from(vec![
                cursor,
                Span::raw(format!(
                    "{kind:<14} {age:<10} {session:<10}  {headline}",
                    kind = e.kind,
                    age = e.age,
                    session = session,
                    headline = e.headline,
                )),
            ]))
        })
        .collect();
    if items.is_empty() {
        items.push(ListItem::new(Line::from("  (scrolled past last row)")));
    }
    let list = List::new(items).block(panel_block(&title));
    frame.render_widget(list, area);
}

fn panel_block(title: &str) -> Block<'_> {
    Block::default().borders(Borders::ALL).title(title)
}
