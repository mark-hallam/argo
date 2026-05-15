//! Pending-approvals tray for the TUI.
use argo_types::{ArgoError, ArgoResult};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;
use serde_json::{json, Value};

/// One pending approval row displayed by the tray.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingApproval {
    id: String,
    worker_id: String,
    action_kind: String,
    reason: String,
    requested_at: String,
}

impl PendingApproval {
    /// Build a row from explicit display fields.
    pub fn new(
        id: impl Into<String>,
        worker_id: impl Into<String>,
        action_kind: impl Into<String>,
        reason: impl Into<String>,
        requested_at: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            worker_id: worker_id.into(),
            action_kind: action_kind.into(),
            reason: reason.into(),
            requested_at: requested_at.into(),
        }
    }
}

/// Result of a tray keypress.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApprovalOutcome {
    /// Key was handled without closing or deciding a row.
    Stay,
    /// Close the tray.
    Close,
    /// Approve the selected approval id.
    Approve(String),
    /// Reject the selected approval id.
    Reject(String),
}

/// Modal tray state.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ApprovalTray {
    pending: Vec<PendingApproval>,
    selected: usize,
    error: Option<String>,
}

impl ApprovalTray {
    /// Construct a tray from rows.
    pub fn new(pending: Vec<PendingApproval>) -> Self {
        Self {
            pending,
            selected: 0,
            error: None,
        }
    }

    fn set_pending(&mut self, pending: Vec<PendingApproval>) {
        self.pending = pending;
        self.selected = self.selected.min(self.pending.len().saturating_sub(1));
    }

    fn set_error(&mut self, error: impl Into<String>) {
        self.error = Some(error.into());
    }

    fn clear_error(&mut self) {
        self.error = None;
    }

    /// Handle one keypress while the tray is open.
    pub fn handle_key(&mut self, key: KeyEvent) -> ApprovalOutcome {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => ApprovalOutcome::Close,
            KeyCode::Down | KeyCode::Char('j') => {
                self.bump(1);
                ApprovalOutcome::Stay
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.bump(self.pending.len().saturating_sub(1));
                ApprovalOutcome::Stay
            }
            KeyCode::Char('y') if !key.modifiers.contains(KeyModifiers::CONTROL) => self
                .selected_row()
                .map(|row| ApprovalOutcome::Approve(row.id.clone()))
                .unwrap_or(ApprovalOutcome::Stay),
            KeyCode::Char('n') if !key.modifiers.contains(KeyModifiers::CONTROL) => self
                .selected_row()
                .map(|row| ApprovalOutcome::Reject(row.id.clone()))
                .unwrap_or(ApprovalOutcome::Stay),
            _ => ApprovalOutcome::Stay,
        }
    }

    /// Render the tray as a centered modal.
    pub fn render(&self, frame: &mut Frame<'_>, area: Rect) {
        let w = area.width.saturating_mul(8) / 10;
        let h = area.height.saturating_mul(7) / 10;
        let x = area.x + (area.width.saturating_sub(w)) / 2;
        let y = area.y + (area.height.saturating_sub(h)) / 2;
        let panel = Rect::new(x, y, w.max(50), h.max(10));

        let mut lines = Vec::with_capacity(self.pending.len() + 4);
        lines.push(Line::from(format!(
            " {:<3} {:<12} {:<18} {:<18} {}",
            "", "worker", "action", "requested", "reason"
        )));
        if self.pending.is_empty() {
            lines.push(Line::from("  No pending approvals"));
        } else {
            for (idx, row) in self.pending.iter().enumerate() {
                let marker = if idx == self.selected { ">" } else { " " };
                lines.push(Line::from(format!(
                    " {marker} {:<12} {:<18} {:<18} {}",
                    clip(&row.worker_id, 12),
                    clip(&row.action_kind, 18),
                    clip(&row.requested_at, 18),
                    clip(&row.reason, 60),
                )));
            }
        }
        if let Some(error) = self.error.as_deref() {
            lines.push(Line::from(""));
            lines.push(Line::from(format!("  error: {}", clip(error, 96))));
        }

        let para = Paragraph::new(lines)
            .style(Style::default().add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL).title(format!(
                " pending approvals ({}) - j/k move - y approve - n reject - q close ",
                self.pending.len()
            )));
        frame.render_widget(Clear, panel);
        frame.render_widget(para, panel);
    }

    fn selected_row(&self) -> Option<&PendingApproval> {
        self.pending.get(self.selected)
    }
    fn bump(&mut self, delta: usize) {
        if !self.pending.is_empty() {
            self.selected = (self.selected + delta) % self.pending.len();
        }
    }
}

/// Open the tray and fetch the current pending list.
pub async fn open_approval_tray(state: &mut crate::app::ViewerState) {
    let mut tray = ApprovalTray::default();
    refresh_tray(&mut tray).await;
    state.approval_tray = Some(tray);
}

/// Refresh the tray if it is currently open.
pub async fn refresh_open_tray(state: &mut crate::app::ViewerState) {
    if let Some(tray) = state.approval_tray.as_mut() {
        refresh_tray(tray).await;
    }
}

/// Handle a key while the approval tray is open.
pub async fn try_handle_approval_tray(
    state: &mut crate::app::ViewerState,
    key: KeyEvent,
) -> ArgoResult<bool> {
    let Some(tray) = state.approval_tray.as_mut() else {
        return Ok(false);
    };
    match tray.handle_key(key) {
        ApprovalOutcome::Stay => {}
        ApprovalOutcome::Close => state.approval_tray = None,
        ApprovalOutcome::Approve(id) => {
            if let Err(err) = decide(&id, "approve").await {
                if let Some(tray) = state.approval_tray.as_mut() {
                    tray.set_error(err.to_string());
                }
            } else {
                refresh_open_tray(state).await;
            }
        }
        ApprovalOutcome::Reject(id) => {
            if let Err(err) = decide(&id, "refuse").await {
                if let Some(tray) = state.approval_tray.as_mut() {
                    tray.set_error(err.to_string());
                }
            } else {
                refresh_open_tray(state).await;
            }
        }
    }
    Ok(true)
}
async fn refresh_tray(tray: &mut ApprovalTray) {
    match fetch_pending().await {
        Ok(rows) => {
            tray.set_pending(rows);
            tray.clear_error();
        }
        Err(err) => tray.set_error(err.to_string()),
    }
}

async fn fetch_pending() -> ArgoResult<Vec<PendingApproval>> {
    let url = format!(
        "{}/api/v1/approvals?limit=100",
        crate::overlays_h_stack::gateway_url().trim_end_matches('/')
    );
    let response = reqwest::get(&url)
        .await
        .map_err(|e| ArgoError::Provider(format!("approval tray fetch: {e}")))?;
    if !response.status().is_success() {
        return Err(ArgoError::Provider(format!(
            "approval tray fetch: HTTP {}",
            response.status()
        )));
    }
    let body: Value = response
        .json()
        .await
        .map_err(|e| ArgoError::Provider(format!("approval tray parse: {e}")))?;
    Ok(parse_pending(&body))
}

async fn decide(id: &str, decision: &str) -> ArgoResult<()> {
    let url = format!(
        "{}/api/v1/approvals/{id}/decide",
        crate::overlays_h_stack::gateway_url().trim_end_matches('/')
    );
    let response = reqwest::Client::new()
        .post(url)
        .json(&json!({ "decision": decision }))
        .send()
        .await
        .map_err(|e| ArgoError::Provider(format!("approval tray decide: {e}")))?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(ArgoError::Provider(format!(
            "approval tray decide: HTTP {}",
            response.status()
        )))
    }
}

fn parse_pending(body: &Value) -> Vec<PendingApproval> {
    body.get("items")
        .and_then(Value::as_array)
        .map(|items| items.iter().map(parse_row).collect())
        .unwrap_or_default()
}

fn parse_row(row: &Value) -> PendingApproval {
    let id = string_at(row, &["id"]).unwrap_or_else(|| "(unknown)".to_string());
    let worker_id = string_at(row, &["worker", "name"])
        .or_else(|| string_at(row, &["worker", "id"]))
        .unwrap_or_else(|| "(worker)".to_string());
    let verb = string_at(row, &["verb"]).unwrap_or_else(|| "approval".to_string());
    let action_kind = verb
        .strip_prefix("wants to ")
        .unwrap_or(&verb)
        .replace('_', " ");
    let reason = string_at(row, &["reason"])
        .or_else(|| string_at(row, &["detail"]))
        .unwrap_or_else(|| "(no reason provided)".to_string());
    let requested_at = string_at(row, &["received_at"]).unwrap_or_else(|| "-".to_string());
    PendingApproval::new(id, worker_id, action_kind, reason, requested_at)
}

fn string_at(row: &Value, path: &[&str]) -> Option<String> {
    let mut current = row;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str().map(str::to_string)
}

fn clip(value: &str, max: usize) -> String {
    let mut out: String = value.chars().take(max).collect();
    if value.chars().count() > max {
        out.push('~');
    }
    out
}

#[cfg(test)]
#[path = "approval_tray_tests.rs"]
mod tests;
