//! Close-side input handlers for the H-stack overlays (Stream
//! `claude-tui-h-stack-panels`). Lives in a sibling file so
//! `input.rs` stays under the 500-LOC budget.
//!
//! Each function returns `true` if the key was consumed by an
//! H-stack overlay (the caller in `input.rs` then `continue`s its
//! input loop), `false` if no H-stack overlay was open and the
//! key should fall through to the regular dispatcher.
use crate::app::ViewerState;
use crate::input_helpers::handle_scroll_overlay;
use crate::overlays_h_stack::{fetch_panel, gateway_url};
use crossterm::event::KeyCode;

/// Handle a single keypress against the H-stack overlay close
/// rules: each panel responds to its own toggle key + `Esc` + `q`,
/// and (for the scrolling panels) `j`/`k`/`PgUp`/`PgDn`/`g`/`G`.
///
/// Returns `true` if the key was consumed.
pub(crate) fn try_close_h_stack_overlay(state: &mut ViewerState, key_code: KeyCode) -> bool {
    // Orchestrator-sessions overlay: Esc/O/q close.
    if let Some(panel) = state.h_sessions_overlay.as_ref() {
        let n = panel.row_count();
        if handle_scroll_overlay(key_code, &['O'], &mut state.h_sessions_overlay_scroll, n) {
            state.h_sessions_overlay = None;
            state.h_sessions_overlay_scroll = 0;
        }
        return true;
    }
    // Orchestrator-handoffs overlay: Esc/H/q close.
    if let Some(panel) = state.h_handoffs_overlay.as_ref() {
        let n = panel.row_count();
        if handle_scroll_overlay(key_code, &['H'], &mut state.h_handoffs_overlay_scroll, n) {
            state.h_handoffs_overlay = None;
            state.h_handoffs_overlay_scroll = 0;
        }
        return true;
    }
    // Budget overlay: Esc/B/q close (single-screen, no scroll).
    if state.h_budget_overlay.is_some()
        && matches!(
            key_code,
            KeyCode::Esc | KeyCode::Char('B') | KeyCode::Char('q')
        )
    {
        state.h_budget_overlay = None;
        return true;
    }
    false
}

/// Open handler for the H-stack hotkeys (`O`/`H`/`B`). Returns
/// `true` if the key was one of the H-stack opens (and the
/// corresponding panel state was populated), `false` if not so the
/// caller can continue its match.
pub(crate) async fn try_open_h_stack_overlay(
    state: &mut ViewerState,
    key_code: KeyCode,
) -> bool {
    let base = gateway_url();
    match key_code {
        KeyCode::Char('O') => {
            state.h_sessions_overlay =
                Some(fetch_panel(&base, "/orchestrator/sessions", true).await);
            true
        }
        KeyCode::Char('H') => {
            state.h_handoffs_overlay = Some(
                fetch_panel(&base, "/orchestrator/handoffs?status=pending", true).await,
            );
            true
        }
        KeyCode::Char('B') => {
            state.h_budget_overlay = Some(fetch_panel(&base, "/budget/summary", false).await);
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overlays_h_stack::PanelState;
    use argo_store::SessionHeader;
    use argo_types::{TenantId, WorkspaceId};
    use chrono::Utc;

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
            vertical_id: None,
        }
    }

    #[test]
    fn esc_closes_sessions_overlay() {
        let mut state = ViewerState::new(vec![header("h")]);
        state.h_sessions_overlay = Some(PanelState::Loading);
        let consumed = try_close_h_stack_overlay(&mut state, KeyCode::Esc);
        assert!(consumed);
        assert!(state.h_sessions_overlay.is_none());
    }

    #[test]
    fn b_closes_budget_overlay() {
        let mut state = ViewerState::new(vec![header("h")]);
        state.h_budget_overlay = Some(PanelState::Loading);
        let consumed = try_close_h_stack_overlay(&mut state, KeyCode::Char('B'));
        assert!(consumed);
        assert!(state.h_budget_overlay.is_none());
    }

    #[test]
    fn h_closes_handoffs_overlay() {
        let mut state = ViewerState::new(vec![header("h")]);
        state.h_handoffs_overlay = Some(PanelState::Loading);
        let consumed = try_close_h_stack_overlay(&mut state, KeyCode::Char('H'));
        assert!(consumed);
        assert!(state.h_handoffs_overlay.is_none());
    }

    #[test]
    fn q_closes_any_open_h_stack_overlay() {
        let mut state = ViewerState::new(vec![header("h")]);
        state.h_handoffs_overlay = Some(PanelState::Loading);
        let consumed = try_close_h_stack_overlay(&mut state, KeyCode::Char('q'));
        assert!(consumed);
        assert!(state.h_handoffs_overlay.is_none());
    }

    #[test]
    fn returns_false_when_no_h_stack_overlay_open() {
        let mut state = ViewerState::new(vec![header("h")]);
        let consumed = try_close_h_stack_overlay(&mut state, KeyCode::Char('q'));
        assert!(!consumed);
    }
}
