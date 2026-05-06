//! Input-routing tests for the dashboard overlay (Stream
//! `claude-tui-realtime-dashboard`). Sibling of
//! `input_dashboard.rs`; lives here so the host file stays under
//! the 500-LOC ceiling.
use super::{open_dashboard_overlay, try_handle_dashboard_overlay};
use crate::app::ViewerState;
use crate::overlays_dashboard::AuditTailEntry;
use argo_store::SessionHeader;
use argo_types::{TenantId, WorkspaceId};
use chrono::Utc;
use crossterm::event::KeyCode;

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
    }
}

#[test]
fn esc_closes_dashboard() {
    let mut state = ViewerState::new(vec![header("t")]);
    open_dashboard_overlay(&mut state);
    assert!(state.dashboard_overlay.is_some());
    let consumed = try_handle_dashboard_overlay(&mut state, KeyCode::Esc);
    assert!(consumed);
    assert!(state.dashboard_overlay.is_none());
}

#[test]
fn d_key_also_closes_dashboard() {
    let mut state = ViewerState::new(vec![header("t")]);
    open_dashboard_overlay(&mut state);
    let consumed = try_handle_dashboard_overlay(&mut state, KeyCode::Char('D'));
    assert!(consumed);
    assert!(state.dashboard_overlay.is_none());
}

#[test]
fn q_key_closes_dashboard() {
    let mut state = ViewerState::new(vec![header("t")]);
    open_dashboard_overlay(&mut state);
    let consumed = try_handle_dashboard_overlay(&mut state, KeyCode::Char('q'));
    assert!(consumed);
    assert!(state.dashboard_overlay.is_none());
}

#[test]
fn arrow_keys_scroll_audit_tail() {
    let mut state = ViewerState::new(vec![header("t")]);
    open_dashboard_overlay(&mut state);
    {
        let dash = state.dashboard_overlay.as_mut().unwrap();
        dash.set_audit_tail(vec![
            AuditTailEntry {
                kind: "a".into(),
                age: "now".into(),
                headline: "x".into(),
                session_short: None,
            },
            AuditTailEntry {
                kind: "b".into(),
                age: "now".into(),
                headline: "y".into(),
                session_short: None,
            },
            AuditTailEntry {
                kind: "c".into(),
                age: "now".into(),
                headline: "z".into(),
                session_short: None,
            },
        ]);
    }
    assert_eq!(
        state.dashboard_overlay.as_ref().unwrap().audit_scroll,
        0
    );
    let consumed = try_handle_dashboard_overlay(&mut state, KeyCode::Down);
    assert!(consumed);
    assert_eq!(
        state.dashboard_overlay.as_ref().unwrap().audit_scroll,
        1
    );
    try_handle_dashboard_overlay(&mut state, KeyCode::Char('j'));
    assert_eq!(
        state.dashboard_overlay.as_ref().unwrap().audit_scroll,
        2
    );
    // Clamp at last row.
    try_handle_dashboard_overlay(&mut state, KeyCode::Char('j'));
    assert_eq!(
        state.dashboard_overlay.as_ref().unwrap().audit_scroll,
        2
    );
    try_handle_dashboard_overlay(&mut state, KeyCode::Up);
    assert_eq!(
        state.dashboard_overlay.as_ref().unwrap().audit_scroll,
        1
    );
}

#[test]
fn home_and_end_jump_to_first_and_last_audit_row() {
    let mut state = ViewerState::new(vec![header("t")]);
    open_dashboard_overlay(&mut state);
    {
        let dash = state.dashboard_overlay.as_mut().unwrap();
        dash.set_audit_tail(
            (0..5)
                .map(|i| AuditTailEntry {
                    kind: format!("k{i}"),
                    age: "now".into(),
                    headline: "x".into(),
                    session_short: None,
                })
                .collect(),
        );
    }
    try_handle_dashboard_overlay(&mut state, KeyCode::Char('G'));
    assert_eq!(
        state.dashboard_overlay.as_ref().unwrap().audit_scroll,
        4
    );
    try_handle_dashboard_overlay(&mut state, KeyCode::Char('g'));
    assert_eq!(
        state.dashboard_overlay.as_ref().unwrap().audit_scroll,
        0
    );
}

#[test]
fn returns_false_when_no_overlay_open() {
    let mut state = ViewerState::new(vec![header("t")]);
    assert!(!try_handle_dashboard_overlay(
        &mut state,
        KeyCode::Esc
    ));
}

#[test]
fn unknown_keys_are_consumed_to_block_global_dispatch() {
    // The dashboard is the operator's "always-on" screen — random
    // keystrokes shouldn't open another overlay on top.
    let mut state = ViewerState::new(vec![header("t")]);
    open_dashboard_overlay(&mut state);
    let consumed =
        try_handle_dashboard_overlay(&mut state, KeyCode::Char('M'));
    assert!(consumed);
    // M didn't close the dashboard.
    assert!(state.dashboard_overlay.is_some());
}
