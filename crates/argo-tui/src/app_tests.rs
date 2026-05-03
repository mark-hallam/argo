//! Unit tests for `app::ViewerState`. Lives in its own file so
//! that `app.rs` stays under the 500-LOC budget as new state
//! fields + methods get added.
use super::*;
use argo_types::{TenantId, WorkspaceId};
use chrono::Utc;

fn fake_session(title: &str) -> SessionHeader {
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

fn fake_msg_row(seq: i64) -> argo_store::MessageRow {
    argo_store::MessageRow {
        id: vec![0u8; 16],
        session_id: vec![0u8; 16],
        role: "user".into(),
        content_json: "[]".into(),
        tool_calls_json: String::new(),
        tool_call_id: None,
        reasoning: None,
        provider_meta: "null".into(),
        created_at: 0,
        seq,
    }
}

#[test]
fn next_clamps_at_end() {
    let mut s = ViewerState::new(vec![fake_session("a"), fake_session("b")]);
    s.next();
    assert_eq!(s.selected, 1);
    s.next();
    assert_eq!(s.selected, 1, "must not pass the last index");
}

#[test]
fn prev_clamps_at_zero() {
    let mut s = ViewerState::new(vec![fake_session("a")]);
    s.prev();
    assert_eq!(s.selected, 0);
}

#[test]
fn empty_list_does_not_panic_on_navigation() {
    let mut s = ViewerState::new(Vec::new());
    s.next();
    s.prev();
    assert!(s.selected_session().is_none());
}

#[test]
fn navigation_invalidates_cached_message_count() {
    let mut s = ViewerState::new(vec![fake_session("a"), fake_session("b")]);
    s.selected_message_count = Some(7);
    s.next();
    assert_eq!(s.selected_message_count, None);
}

#[test]
fn next_n_clamps_at_end() {
    let mut s = ViewerState::new((0..5).map(|i| fake_session(&i.to_string())).collect());
    s.next_n(3);
    assert_eq!(s.selected, 3);
    s.next_n(99);
    assert_eq!(s.selected, 4);
}

#[test]
fn prev_n_clamps_at_zero() {
    let mut s = ViewerState::new((0..5).map(|i| fake_session(&i.to_string())).collect());
    s.selected = 4;
    s.prev_n(3);
    assert_eq!(s.selected, 1);
    s.prev_n(99);
    assert_eq!(s.selected, 0);
}

#[test]
fn first_and_last_jump() {
    let mut s = ViewerState::new((0..5).map(|i| fake_session(&i.to_string())).collect());
    s.last();
    assert_eq!(s.selected, 4);
    s.first();
    assert_eq!(s.selected, 0);
}

#[test]
fn first_and_last_no_op_on_empty() {
    let mut s = ViewerState::new(Vec::new());
    s.first();
    s.last();
    assert!(s.selected_session().is_none());
}

#[test]
fn empty_filter_shows_all() {
    let s = ViewerState::new(vec![fake_session("morning"), fake_session("afternoon")]);
    assert_eq!(s.filtered_indices(), vec![0, 1]);
}

#[test]
fn filter_substring_match() {
    let mut s = ViewerState::new(vec![
        fake_session("morning summary"),
        fake_session("afternoon stand-up"),
        fake_session("evening review"),
    ]);
    s.filter_push('m');
    s.filter_push('o');
    s.filter_push('r');
    assert_eq!(s.filtered_indices(), vec![0]);
}

#[test]
fn filter_is_case_insensitive() {
    let mut s = ViewerState::new(vec![fake_session("Morning")]);
    s.filter_push('m');
    assert_eq!(s.filtered_indices(), vec![0]);
}

#[test]
fn changing_filter_resets_selection() {
    let mut s = ViewerState::new(vec![
        fake_session("alpha"),
        fake_session("beta"),
        fake_session("gamma"),
    ]);
    s.selected = 2;
    s.filter_push('a');
    assert_eq!(s.selected, 0);
}

#[test]
fn selected_session_honors_filter() {
    let mut s = ViewerState::new(vec![
        fake_session("a-thing"),
        fake_session("b-thing"),
        fake_session("c-thing"),
    ]);
    s.filter_push('b');
    let title = s.selected_session().unwrap().title.as_deref().unwrap();
    assert_eq!(title, "b-thing");
}

#[test]
fn filter_clear_returns_to_full_list() {
    let mut s = ViewerState::new(vec![fake_session("alpha"), fake_session("beta")]);
    s.filter_push('a');
    s.filter_clear();
    assert_eq!(s.filter, "");
    assert!(!s.filter_mode);
    assert_eq!(s.filtered_indices(), vec![0, 1]);
}

#[test]
fn fresh_state_has_no_agent_in_flight() {
    let s = ViewerState::new(Vec::new());
    assert!(!s.agent_running());
    assert!(s.last_agent_error.is_none());
    assert!(s.live_events.is_empty());
}

#[test]
fn push_live_event_caps_at_max() {
    let mut s = ViewerState::new(Vec::new());
    for _ in 0..(MAX_LIVE_EVENTS + 5) {
        s.push_live_event(&serde_json::json!({
            "event": "TurnEnd",
            "payload": {}
        }));
    }
    assert_eq!(s.live_events.len(), MAX_LIVE_EVENTS);
}

#[test]
fn clear_live_events_resets_ring() {
    let mut s = ViewerState::new(Vec::new());
    s.push_live_event(&serde_json::json!({"event":"TurnStart","payload":{}}));
    assert!(!s.live_events.is_empty());
    s.clear_live_events();
    assert!(s.live_events.is_empty());
}

#[test]
fn toggle_focus_cycles_between_panes() {
    let mut s = ViewerState::new(Vec::new());
    assert_eq!(s.focus, PaneFocus::Sessions);
    s.toggle_focus();
    assert_eq!(s.focus, PaneFocus::Messages);
    s.toggle_focus();
    assert_eq!(s.focus, PaneFocus::Sessions);
}

#[test]
fn scroll_messages_clamps_at_zero_and_top() {
    let mut s = ViewerState::new(Vec::new());
    s.selected_messages = (0..3).map(fake_msg_row).collect();
    s.scroll_messages_up();
    assert_eq!(s.messages_scroll, 0);
    s.scroll_messages_down();
    s.scroll_messages_down();
    assert_eq!(s.messages_scroll, 2);
    s.scroll_messages_down();
    assert_eq!(s.messages_scroll, 2, "clamps at last row");
}

#[test]
fn scroll_messages_page_jumps_clamp_and_snap() {
    let mut s = ViewerState::new(Vec::new());
    s.selected_messages = (0..30).map(fake_msg_row).collect();

    s.scroll_messages_down_n(10);
    assert_eq!(s.messages_scroll, 10);
    s.scroll_messages_down_n(99);
    assert_eq!(s.messages_scroll, 29, "clamps at last row");
    s.scroll_messages_up_n(15);
    assert_eq!(s.messages_scroll, 14);
    s.scroll_messages_first();
    assert_eq!(s.messages_scroll, 0);
    s.scroll_messages_last();
    assert_eq!(s.messages_scroll, 29);
}
