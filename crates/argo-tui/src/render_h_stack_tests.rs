//! Snapshot tests for the H-stack overlays (Stream
//! `claude-tui-h-stack-panels`). Mirrors the pattern in
//! `render_tests.rs`: build a `ViewerState`, drive `render::draw`
//! against a `TestBackend`, then assert the rendered grid contains
//! the strings the operator should see.
use super::*;
use crate::overlays_h_stack::PanelState;
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
        vertical_id: None,
    }
}

fn render_to_strings(state: &ViewerState) -> String {
    let backend = TestBackend::new(120, 28);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| super::draw(f, state)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let mut out = String::new();
    for y in 0..28 {
        for x in 0..120 {
            out.push_str(buffer.cell((x, y)).map(|c| c.symbol()).unwrap_or(""));
        }
        out.push('\n');
    }
    out
}

#[test]
fn sessions_overlay_renders_rows() {
    let mut state = ViewerState::new(vec![header("h-stack")]);
    state.h_sessions_overlay = Some(PanelState::Ok(vec![serde_json::json!({
        "id": "01HXSESS01",
        "title": "skate-followup",
        "status": "Active",
        "participants": ["w-1", "w-2"],
    })]));
    let frame = render_to_strings(&state);
    assert!(frame.contains("orchestrator sessions (1)"));
    assert!(frame.contains("skate-followup"));
    assert!(frame.contains("Active"));
}

#[test]
fn sessions_overlay_renders_unreachable_state() {
    let mut state = ViewerState::new(vec![header("h-stack")]);
    state.h_sessions_overlay = Some(PanelState::Unreachable(
        "Gateway unreachable: route missing".into(),
    ));
    let frame = render_to_strings(&state);
    assert!(frame.contains("unreachable"));
    assert!(frame.contains("Gateway unreachable"));
}

#[test]
fn sessions_overlay_renders_empty_list() {
    let mut state = ViewerState::new(vec![header("h-stack")]);
    state.h_sessions_overlay = Some(PanelState::Ok(Vec::new()));
    let frame = render_to_strings(&state);
    assert!(frame.contains("(no active sessions)"));
}

#[test]
fn handoffs_overlay_renders_pending_rows() {
    let mut state = ViewerState::new(vec![header("h-stack")]);
    state.h_handoffs_overlay = Some(PanelState::Ok(vec![serde_json::json!({
        "id": "01HXH01",
        "from": "worker-a",
        "to": "worker-b",
        "status": "Pending",
        "deadline": "2026-05-07",
    })]));
    let frame = render_to_strings(&state);
    assert!(frame.contains("orchestrator handoffs (1)"));
    assert!(frame.contains("worker-a"));
    assert!(frame.contains("Pending"));
}

#[test]
fn handoffs_overlay_renders_unreachable_state() {
    let mut state = ViewerState::new(vec![header("h-stack")]);
    state.h_handoffs_overlay = Some(PanelState::Unreachable(
        "Gateway 404 (waiting on H-orchestrator-routes)".into(),
    ));
    let frame = render_to_strings(&state);
    assert!(frame.contains("unreachable"));
    assert!(frame.contains("404"));
}

#[test]
fn handoffs_overlay_renders_empty_list() {
    let mut state = ViewerState::new(vec![header("h-stack")]);
    state.h_handoffs_overlay = Some(PanelState::Ok(Vec::new()));
    let frame = render_to_strings(&state);
    assert!(frame.contains("(no pending handoffs)"));
}

#[test]
fn budget_overlay_renders_summary() {
    let mut state = ViewerState::new(vec![header("h-stack")]);
    state.h_budget_overlay = Some(PanelState::OkOne(serde_json::json!({
        "worker_id": "w-42",
        "today_usd": 1.25,
        "this_month_usd": 17.5,
        "daily_cap_usd_micros": 5_000_000_u64,
        "remaining_today_usd": 3.75,
        "last_charges": [],
    })));
    let frame = render_to_strings(&state);
    assert!(frame.contains("worker:"));
    assert!(frame.contains("w-42"));
    assert!(frame.contains("$1.2500"));
    assert!(frame.contains("$17.5000"));
    assert!(frame.contains("$3.7500"));
}

#[test]
fn budget_overlay_renders_unreachable_state() {
    let mut state = ViewerState::new(vec![header("h-stack")]);
    state.h_budget_overlay = Some(PanelState::Unreachable(
        "Gateway unreachable: connect refused".into(),
    ));
    let frame = render_to_strings(&state);
    assert!(frame.contains("unreachable"));
    assert!(frame.contains("Gateway unreachable"));
}

#[test]
fn budget_overlay_renders_loading_state() {
    let mut state = ViewerState::new(vec![header("h-stack")]);
    state.h_budget_overlay = Some(PanelState::Loading);
    let frame = render_to_strings(&state);
    assert!(frame.contains("loading budget"));
}
