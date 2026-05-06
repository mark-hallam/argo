//! `D` real-time dashboard overlay (Stream
//! `claude-tui-realtime-dashboard`). One screen with three stacked
//! panes the operator can leave open all day:
//!
//! 1. **Workers** — recently-seen worker rows: id, status, last
//!    action kind, today's spend (USD).
//! 2. **Pending approvals** — actions waiting on a human, newest
//!    first; row format: `worker · action_kind · age`.
//! 3. **Audit tail** — last N audit rows (default 10), scrollable
//!    via the arrow keys / `j` / `k`.
//!
//! Workers + pending fetch over HTTP through [`crate::overlays_h_stack`]
//! `fetch_panel`, mirroring the H-stack pattern (the routes
//! `/orchestrator/workers` and `/orchestrator/approvals/pending`
//! are queued for follow-up — until they ship, the panes render
//! "Gateway unreachable" cleanly via [`PanelState::Unreachable`]
//! exactly like H-stack did before
//! [H-orchestrator-routes (#827)] landed).
//!
//! The audit tail reads in-process via
//! [`argo_audit::tail_rows_filtered`] every 1 s tick from the
//! existing event-loop refresh hook in [`crate::input`]. A future
//! v1.x WebSocket upgrade can swap this for
//! [`argo_audit::AuditSink::subscribe`] on the gateway side and an
//! SSE/WS client here without any panel rewrite.
use crate::overlays_h_stack::PanelState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};
use serde_json::Value;

/// Size cap for the audit-tail rolling buffer. The brief specifies
/// "last 10 audit records" — we keep a small headroom so the
/// scroll keys (`j`/`k`/arrows) have somewhere to move when the
/// operator wants to inspect a row that just scrolled off.
pub const AUDIT_TAIL_CAPACITY: usize = 50;

/// Default render limit for the audit-tail pane. The pane shows the
/// most recent N rows (with the rest accessible via scroll). 10 is
/// the brief target.
pub const AUDIT_TAIL_DEFAULT_VISIBLE: usize = 10;

/// One audit-tail entry — projection of an
/// [`argo_audit::ExportedRow`] into the cheap-to-render shape this
/// overlay needs. Stored in-memory in [`DashboardState::audit_tail`]
/// so the render path doesn't re-parse JSON on every frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditTailEntry {
    /// Audit kind label (e.g. `tool_call`, `turn_start`).
    pub kind: String,
    /// `created_at` formatted as a human age string (e.g. `2m ago`).
    pub age: String,
    /// Short headline derived from the row's payload — one line for
    /// the operator to triage at a glance.
    pub headline: String,
    /// Optional session id (short form, first 8 hex chars).
    pub session_short: Option<String>,
}

/// State for the dashboard overlay. Built when the operator presses
/// `D`; cleared (set to `None` on [`crate::app::ViewerState`]) on
/// `Esc`/`D`/`q`. Each refresh tick (1 s, driven by
/// [`crate::input`]) re-populates the panes by hitting the gateway
/// (`workers` + `pending`) and the local SQLite (`audit_tail`).
#[derive(Debug, Clone)]
pub struct DashboardState {
    /// Workers pane state. Loading at open; `Ok` / `Unreachable`
    /// after the first refresh tick.
    pub workers: PanelState,
    /// Pending-approvals pane state. Same lifecycle.
    pub pending: PanelState,
    /// Audit tail (capped at [`AUDIT_TAIL_CAPACITY`]; newest last).
    pub audit_tail: Vec<AuditTailEntry>,
    /// Vertical scroll offset (in rows) into the audit tail; bumped
    /// by the j/k/arrow handlers in [`crate::input_dashboard`].
    pub audit_scroll: usize,
    /// `true` between an open and the first refresh-tick callback.
    /// Used to render a one-shot "loading audit tail…" hint.
    pub audit_loading: bool,
}

impl Default for DashboardState {
    fn default() -> Self {
        Self::new()
    }
}

impl DashboardState {
    /// Construct a fresh dashboard at open time. All panes start
    /// in `Loading` so the first frame renders cleanly while the
    /// initial refresh tick is in flight.
    #[must_use]
    pub fn new() -> Self {
        Self {
            workers: PanelState::Loading,
            pending: PanelState::Loading,
            audit_tail: Vec::new(),
            audit_scroll: 0,
            audit_loading: true,
        }
    }

    /// Replace the audit tail with the supplied newest-first
    /// entries. Trims to [`AUDIT_TAIL_CAPACITY`] and clamps the
    /// scroll offset so a fresh refresh never points past the new
    /// last row.
    pub fn set_audit_tail(&mut self, mut entries: Vec<AuditTailEntry>) {
        if entries.len() > AUDIT_TAIL_CAPACITY {
            let overflow = entries.len() - AUDIT_TAIL_CAPACITY;
            entries.drain(..overflow);
        }
        self.audit_tail = entries;
        self.audit_loading = false;
        let max = self.audit_tail.len().saturating_sub(1);
        if self.audit_scroll > max {
            self.audit_scroll = max;
        }
    }

    /// Bump the audit-tail scroll offset down by one (clamped at
    /// the last row).
    pub fn scroll_audit_down(&mut self) {
        let max = self.audit_tail.len().saturating_sub(1);
        if self.audit_scroll < max {
            self.audit_scroll += 1;
        }
    }

    /// Bump the audit-tail scroll offset up by one (clamped at zero).
    pub fn scroll_audit_up(&mut self) {
        self.audit_scroll = self.audit_scroll.saturating_sub(1);
    }

    /// `true` when no panel has any rows to render. Used by the
    /// renderer to surface a single "gathering signals…" hint
    /// instead of three empty panes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.audit_tail.is_empty()
            && matches!(self.workers, PanelState::Loading)
            && matches!(self.pending, PanelState::Loading)
    }
}

/// Build a one-line "<n>s/m/h ago" string from `now_micros - then_micros`.
/// Used by the audit tail + the pending-approvals pane so an operator
/// scanning the dashboard can immediately see how stale a row is. Returns
/// `"now"` for sub-second deltas.
#[must_use]
pub fn human_age(now_micros: i64, then_micros: i64) -> String {
    let delta = now_micros.saturating_sub(then_micros);
    if delta <= 0 {
        return "now".to_string();
    }
    let secs = delta / 1_000_000;
    if secs < 1 {
        return "now".to_string();
    }
    if secs < 60 {
        return format!("{secs}s ago");
    }
    let mins = secs / 60;
    if mins < 60 {
        return format!("{mins}m ago");
    }
    let hours = mins / 60;
    if hours < 24 {
        return format!("{hours}h ago");
    }
    let days = hours / 24;
    format!("{days}d ago")
}

/// Project a raw `argo_audit::ExportedRow`-shaped payload into a
/// short headline for the audit tail. Pulls `tool` / `result` /
/// `text` (best-effort) so the typical row reads like
/// `tool_call · gmail_search`.
#[must_use]
pub fn payload_headline(kind: &str, payload: &Value) -> String {
    if let Some(tool) = payload.get("tool").and_then(Value::as_str) {
        return format!("{kind} · {tool}");
    }
    if let Some(action_kind) = payload.get("action_kind").and_then(Value::as_str) {
        return format!("{kind} · {action_kind}");
    }
    if let Some(model) = payload.get("model").and_then(Value::as_str) {
        return format!("{kind} · {model}");
    }
    if let Some(text) = payload.get("text").and_then(Value::as_str) {
        let snippet: String = text.chars().take(40).collect();
        return format!("{kind} · {snippet}");
    }
    kind.to_string()
}

/// Format one worker row from the `/orchestrator/workers` JSON shape
/// described in the v1.x route brief: `{worker_id, role, last_action,
/// today_usd, paused}`. Tolerant of missing keys.
#[must_use]
pub fn format_worker_line(v: &Value) -> String {
    let id = v
        .get("worker_id")
        .and_then(Value::as_str)
        .map(short_id)
        .unwrap_or_else(|| "(unknown)".to_string());
    let role = v
        .get("role")
        .and_then(Value::as_str)
        .unwrap_or("(no-role)");
    let last = v
        .get("last_action")
        .and_then(Value::as_str)
        .unwrap_or("(idle)");
    let spend = v
        .get("today_usd")
        .and_then(Value::as_f64)
        .map(|f| format!("${f:.4}"))
        .unwrap_or_else(|| "(unset)".to_string());
    let status = if v.get("paused").and_then(Value::as_bool).unwrap_or(false) {
        "PAUSED"
    } else {
        "active"
    };
    format!("{id}  {status:<6}  role={role:<14}  last={last:<14}  {spend}")
}

/// Format one pending-approval row. Expected shape (from the
/// queued `/orchestrator/approvals/pending` route):
/// `{action_id, worker_id, action_kind, requested_at_micros}`.
#[must_use]
pub fn format_pending_line(v: &Value, now_micros: i64) -> String {
    let action = v
        .get("action_id")
        .and_then(Value::as_str)
        .map(short_id)
        .unwrap_or_else(|| "(no-id)".to_string());
    let worker = v
        .get("worker_id")
        .and_then(Value::as_str)
        .map(short_id)
        .unwrap_or_else(|| "?".to_string());
    let kind = v
        .get("action_kind")
        .and_then(Value::as_str)
        .unwrap_or("(unknown)");
    let age = v
        .get("requested_at_micros")
        .and_then(Value::as_i64)
        .map(|t| human_age(now_micros, t))
        .unwrap_or_else(|| "?".to_string());
    format!("{action}  worker={worker:<8}  {kind:<18}  {age}")
}

fn short_id(s: &str) -> String {
    s.chars().take(8).collect()
}

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

#[cfg(test)]
#[path = "overlays_dashboard_tests.rs"]
mod tests;
