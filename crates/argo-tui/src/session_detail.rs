//! 4-tab drill-in pane for one session — Messages, Audit, Cost,
//! Skills. Each tab is a thin reader over a primitive that already
//! ships in `argo-store` / `argo-skill`, so the pane introduces no
//! new wire load.
//!
//! Lifecycle:
//! 1. [`SessionDetailPane::new`] builds an empty pane scoped to a
//!    `SessionId`.
//! 2. [`SessionDetailPane::refresh`] populates all four tab data
//!    sources from a `SessionStore` handle.
//! 3. The host event loop calls [`SessionDetailPane::render`] each
//!    draw and routes keystrokes through [`SessionDetailPane::handle_key`].
//!
//! The Enter-to-open binding lives in `input.rs`. That file is at
//! the 500-LOC ceiling today — wiring the binding requires a split
//! and is tracked separately. The pane itself is fully testable
//! against a `TestBackend` in the meantime.
//!
//! Per-tab `draw_*` methods live in the sibling module
//! [`crate::session_detail_render`] to keep this file under the
//! 500-LOC ceiling.
use argo_skill::Skill;
use argo_store::{AuditRow, MessageRow, SessionStore};
use argo_types::{ArgoResult, SessionId};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::Clear,
    Frame,
};
use std::sync::Arc;

/// Active tab inside the [`SessionDetailPane`]. `Tab`/`Shift-Tab`
/// rotates through the four variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    /// Persisted message rows for the session.
    #[default]
    Messages,
    /// Per-session audit-chain rows.
    Audit,
    /// Aggregated cost derived from the message stream.
    Cost,
    /// Active skills installed in the workspace.
    Skills,
}

impl Tab {
    /// Order used by the tab bar and `next`/`prev` cycling.
    pub const ALL: [Tab; 4] = [Tab::Messages, Tab::Audit, Tab::Cost, Tab::Skills];

    /// Human-readable label rendered in the tab bar.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Tab::Messages => "Messages",
            Tab::Audit => "Audit",
            Tab::Cost => "Cost",
            Tab::Skills => "Skills",
        }
    }

    /// Cycle to the next tab (wraps `Skills` → `Messages`).
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Tab::Messages => Tab::Audit,
            Tab::Audit => Tab::Cost,
            Tab::Cost => Tab::Skills,
            Tab::Skills => Tab::Messages,
        }
    }

    /// Cycle to the previous tab (wraps `Messages` → `Skills`).
    #[must_use]
    pub fn prev(self) -> Self {
        match self {
            Tab::Messages => Tab::Skills,
            Tab::Audit => Tab::Messages,
            Tab::Cost => Tab::Audit,
            Tab::Skills => Tab::Cost,
        }
    }

    /// Zero-based index used for tab-bar highlighting.
    #[must_use]
    pub fn index(self) -> usize {
        match self {
            Tab::Messages => 0,
            Tab::Audit => 1,
            Tab::Cost => 2,
            Tab::Skills => 3,
        }
    }
}

/// Outcome of a single keystroke. The host event loop drops the
/// pane on [`DetailOutcome::Close`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailOutcome {
    /// Keep the pane open.
    Stay,
    /// Close the pane and return to the sessions list.
    Close,
}

/// Aggregated cost summary for one session — derived from the
/// loaded message rows so the pane keeps its zero-new-wire promise.
/// A precise spend rollup belongs in the global `$` overlay.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CostSummary {
    /// Total persisted messages.
    pub message_count: usize,
    /// Total characters across `content_json` bodies.
    pub total_chars: usize,
    /// Spend in micros of USD parsed out of `provider_meta`. Zero
    /// when no provider populates the field.
    pub spend_usd_micros: u64,
}

impl CostSummary {
    /// Aggregate a [`CostSummary`] from a list of message rows.
    /// Reads `provider_meta.spend_usd_micros` best-effort —
    /// missing / malformed entries contribute zero.
    #[must_use]
    pub fn from_messages(rows: &[MessageRow]) -> Self {
        let mut total_chars = 0usize;
        let mut spend = 0u64;
        for r in rows {
            total_chars = total_chars.saturating_add(r.content_json.chars().count());
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&r.provider_meta) {
                if let Some(s) = v.get("spend_usd_micros").and_then(|x| x.as_u64()) {
                    spend = spend.saturating_add(s);
                }
            }
        }
        Self {
            message_count: rows.len(),
            total_chars,
            spend_usd_micros: spend,
        }
    }

    /// Spend rendered in dollars (micros / 1_000_000).
    #[must_use]
    pub fn spend_usd(&self) -> f64 {
        self.spend_usd_micros as f64 / 1_000_000.0
    }
}

/// Drill-in pane state. See module-level docs for lifecycle.
pub struct SessionDetailPane {
    /// Session this pane is rendering.
    pub session_id: SessionId,
    /// Currently-active tab.
    pub tab: Tab,
    /// Cached message rows (populated by [`SessionDetailPane::refresh`]).
    pub messages: Vec<MessageRow>,
    /// Cached audit rows.
    pub audit: Vec<AuditRow>,
    /// Aggregated cost summary.
    pub cost: CostSummary,
    /// Cached active skills.
    pub skills: Vec<Skill>,
    /// Per-tab vertical scroll offset; reset to zero on tab switch.
    pub scroll: usize,
}

impl SessionDetailPane {
    /// Hard cap on rows the messages tab keeps in memory. Mirrors
    /// the existing TUI `MAX_VIEWER_MESSAGES` budget.
    pub const MAX_MESSAGES: usize = 500;

    /// Build an empty pane scoped to `session_id`. Call
    /// [`SessionDetailPane::refresh`] before [`SessionDetailPane::render`]
    /// to populate the bodies.
    ///
    /// ```
    /// use argo_tui::session_detail::SessionDetailPane;
    /// use argo_types::SessionId;
    /// let pane = SessionDetailPane::new(SessionId::new());
    /// assert!(pane.messages.is_empty());
    /// ```
    #[must_use]
    pub fn new(session_id: SessionId) -> Self {
        Self {
            session_id,
            tab: Tab::default(),
            messages: Vec::new(),
            audit: Vec::new(),
            cost: CostSummary::default(),
            skills: Vec::new(),
            scroll: 0,
        }
    }

    /// Re-pull all four tab data sources. Errors from the optional
    /// readers (audit, skills) are absorbed — the corresponding
    /// tab simply renders empty — so a missing audit / skill row
    /// never tears down the pane.
    ///
    /// # Errors
    /// Returns an error only when `list_messages` fails (the
    /// primary reader); the cost summary is derived from whatever
    /// messages came back.
    pub async fn refresh(&mut self, store: &Arc<SessionStore>) -> ArgoResult<()> {
        let mut messages = store.list_messages(self.session_id).await?;
        if messages.len() > Self::MAX_MESSAGES {
            let drop = messages.len() - Self::MAX_MESSAGES;
            messages.drain(..drop);
        }
        self.cost = CostSummary::from_messages(&messages);
        self.messages = messages;
        self.audit = store
            .list_audit_for_session(self.session_id)
            .await
            .unwrap_or_default();
        let skill_store = argo_skill::SkillStore::new(Arc::clone(store));
        self.skills = skill_store.list_active().await.unwrap_or_default();
        self.clamp_scroll();
        Ok(())
    }

    /// Handle one keystroke. Returns [`DetailOutcome::Close`] when
    /// the user pressed `q` / `Esc`, otherwise [`DetailOutcome::Stay`].
    pub fn handle_key(&mut self, key: KeyEvent) -> DetailOutcome {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return DetailOutcome::Close,
            KeyCode::Tab => {
                self.tab = self.tab.next();
                self.scroll = 0;
            }
            KeyCode::BackTab => {
                self.tab = self.tab.prev();
                self.scroll = 0;
            }
            KeyCode::Char('j') | KeyCode::Down => self.scroll_down(),
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll = self.scroll.saturating_sub(1);
            }
            _ => {}
        }
        DetailOutcome::Stay
    }

    fn scroll_down(&mut self) {
        let max = self.body_len().saturating_sub(1);
        if self.scroll < max {
            self.scroll += 1;
        }
    }

    fn clamp_scroll(&mut self) {
        let max = self.body_len().saturating_sub(1);
        if self.scroll > max {
            self.scroll = max;
        }
    }

    fn body_len(&self) -> usize {
        match self.tab {
            Tab::Messages => self.messages.len(),
            Tab::Audit => self.audit.len(),
            Tab::Cost => 1,
            Tab::Skills => self.skills.len(),
        }
    }

    /// Render the pane to `frame`, covering `area`. Splits into a
    /// 3-row tab bar on top and a body block below. The per-tab
    /// `draw_*` methods live in [`crate::session_detail_render`].
    pub fn render(&self, frame: &mut Frame<'_>, area: Rect) {
        frame.render_widget(Clear, area);
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .split(area);
        self.draw_tab_bar(frame, rows[0]);
        match self.tab {
            Tab::Messages => self.draw_messages(frame, rows[1]),
            Tab::Audit => self.draw_audit(frame, rows[1]),
            Tab::Cost => self.draw_cost(frame, rows[1]),
            Tab::Skills => self.draw_skills(frame, rows[1]),
        }
    }
}

#[cfg(test)]
#[path = "session_detail_tests.rs"]
mod tests;
