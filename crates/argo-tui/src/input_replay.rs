//! Close-side + nav input handlers for the replay overlay
//! (Stream `claude-trajectory-replay-ui`). Sibling of
//! `input_h_stack.rs`.
//!
//! Returns `true` when the key was consumed by the replay
//! overlay so the caller in `input.rs` can `continue`.
use crate::app::ViewerState;
use crate::overlays_replay::ReplayState;
use crossterm::event::KeyCode;

/// Try to handle a keypress against the replay overlay. Returns
/// `true` if the key was consumed.
pub(crate) fn try_handle_replay_overlay(state: &mut ViewerState, key_code: KeyCode) -> bool {
    let Some(replay) = state.replay_overlay.as_mut() else {
        return false;
    };
    match key_code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('R') => {
            state.replay_overlay = None;
        }
        KeyCode::Right | KeyCode::Char('l') => replay.step_next(),
        KeyCode::Left | KeyCode::Char('h') => replay.step_prev(),
        KeyCode::Down | KeyCode::Char('j') => replay.step_next(),
        KeyCode::Up | KeyCode::Char('k') => replay.step_prev(),
        _ => {}
    }
    true
}

/// Open the replay overlay with a stub trajectory. The cli will
/// hand a real trajectory in via `populate_replay_overlay` once
/// the action-replay data path is wired into the TUI; for now
/// the overlay surfaces the contract + renders cleanly with
/// placeholder content so the operator can verify keyboard
/// navigation.
pub(crate) fn open_replay_overlay(state: &mut ViewerState) {
    state.replay_overlay = Some(ReplayState::new(stub_trajectory()));
}

fn stub_trajectory() -> Vec<crate::overlays_replay::ReplayStep> {
    use crate::overlays_replay::ReplayStep;
    vec![
        ReplayStep {
            idx: 1,
            kind: "human".into(),
            headline: "operator: list pending actions".into(),
            body: "(stub trajectory — wire to argo action replay data path)".into(),
            screenshot_path: None,
            payload: None,
        },
        ReplayStep {
            idx: 2,
            kind: "mcp".into(),
            headline: "tool: search_threads(query=\"pending\")".into(),
            body: "Returned 3 threads matching the query.".into(),
            screenshot_path: None,
            payload: Some(serde_json::json!({"matches": 3})),
        },
        ReplayStep {
            idx: 3,
            kind: "computer_use".into(),
            headline: "click button #signup".into(),
            body: "Browser action: locate button by id, click.".into(),
            screenshot_path: Some("/tmp/argo/replay/step3.png".into()),
            payload: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_store::SessionHeader;
    use argo_types::{TenantId, WorkspaceId};
    use chrono::Utc;

    fn header(title: &str) -> SessionHeader {
        SessionHeader {
            id: argo_types::SessionId::new(),
            tenant_id: TenantId::new(),
            workspace_id: WorkspaceId::new(),
            model: "mock".into(),
            provider: "mock".into(),
            title: Some(title.into()),
            started_at: Utc::now(),
            ended_at: None,
            vertical_id: None,
            worker_id: None,
        }
    }

    #[test]
    fn esc_closes_replay() {
        let mut state = ViewerState::new(vec![header("t")]);
        open_replay_overlay(&mut state);
        assert!(state.replay_overlay.is_some());
        let consumed = try_handle_replay_overlay(&mut state, KeyCode::Esc);
        assert!(consumed);
        assert!(state.replay_overlay.is_none());
    }

    #[test]
    fn arrow_steps_advance_selection() {
        let mut state = ViewerState::new(vec![header("t")]);
        open_replay_overlay(&mut state);
        assert_eq!(state.replay_overlay.as_ref().unwrap().selected, 0);
        try_handle_replay_overlay(&mut state, KeyCode::Right);
        assert_eq!(state.replay_overlay.as_ref().unwrap().selected, 1);
        try_handle_replay_overlay(&mut state, KeyCode::Right);
        assert_eq!(state.replay_overlay.as_ref().unwrap().selected, 2);
        // Clamped at end.
        try_handle_replay_overlay(&mut state, KeyCode::Right);
        assert_eq!(state.replay_overlay.as_ref().unwrap().selected, 2);
        try_handle_replay_overlay(&mut state, KeyCode::Left);
        assert_eq!(state.replay_overlay.as_ref().unwrap().selected, 1);
    }

    #[test]
    fn returns_false_when_no_overlay_open() {
        let mut state = ViewerState::new(vec![header("t")]);
        assert!(!try_handle_replay_overlay(&mut state, KeyCode::Esc));
    }
}
