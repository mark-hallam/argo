//! Tests for `render::draw` — extracted from `render.rs` so
//! that file stays under the 500-LOC ceiling as the overlay
//! surface grows. Overlay-pane snapshot tests live in the
//! sibling `render_tests_overlays.rs` (mounted below).
use super::*;
use argo_store::SessionHeader;
use argo_types::{TenantId, WorkspaceId};
use chrono::Utc;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

#[path = "render_tests_overlays.rs"]
mod overlays;

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
        worker_id: None,
    }
}

/// #699: bound-session header builder for the TUI vertical
/// surfacing tests. Same shape as `header()` but tags the
/// session with `vertical_id = Some(vertical)` so the list +
/// detail panes render their badge / line.
fn header_bound(title: &str, vertical: &str) -> SessionHeader {
    let mut h = header(title);
    h.vertical_id = Some(vertical.to_string());
    h
}

fn render_to_strings(state: &ViewerState) -> String {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw(f, state)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let mut out = String::new();
    for y in 0..24 {
        for x in 0..80 {
            out.push_str(buffer.cell((x, y)).map(|c| c.symbol()).unwrap_or(""));
        }
        out.push('\n');
    }
    out
}

#[test]
fn empty_state_renders_no_sessions_message() {
    let state = ViewerState::new(Vec::new());
    let frame = render_to_strings(&state);
    assert!(frame.contains("(no sessions"));
    assert!(frame.contains("quit"));
}

#[test]
fn renders_session_titles_in_list() {
    let state = ViewerState::new(vec![header("morning"), header("afternoon")]);
    let frame = render_to_strings(&state);
    assert!(frame.contains("morning"));
    assert!(frame.contains("afternoon"));
}

#[test]
fn renders_selected_detail_pane() {
    let state = ViewerState::new(vec![header("focus")]);
    let frame = render_to_strings(&state);
    assert!(frame.contains("provider:"));
    assert!(frame.contains("mock-provider"));
}

// ---- #699: vertical_id surfacing in TUI ----

#[test]
fn unbound_session_list_does_not_render_vertical_badge() {
    let state = ViewerState::new(vec![header("legacy")]);
    let frame = render_to_strings(&state);
    let row = frame.lines().find(|l| l.contains("legacy")).expect("row");
    assert!(!row.contains('['), "unbound row got badge: {row:?}");
}

#[test]
fn bound_session_list_renders_vertical_badge() {
    // Short title + short id so the narrow 80x24 list pane
    // doesn't clip the badge. Longer ids still render; the
    // clipping is a layout concern, not correctness.
    let state = ViewerState::new(vec![header_bound("m", "gtm")]);
    let frame = render_to_strings(&state);
    assert!(frame.contains("[gtm]"), "missing badge:\n{frame}");
}

#[test]
fn unbound_session_detail_pane_does_not_render_vertical_line() {
    let state = ViewerState::new(vec![header("legacy-detail")]);
    let frame = render_to_strings(&state);
    assert!(frame.contains("provider:"));
    assert!(!frame.contains("vertical:"), "unbound got vertical line:\n{frame}");
}

#[test]
fn bound_session_detail_pane_renders_vertical_line() {
    let state = ViewerState::new(vec![header_bound("focus", "gtm-engineer")]);
    let frame = render_to_strings(&state);
    assert!(frame.contains("vertical:"), "missing line:\n{frame}");
    assert!(frame.contains("gtm-engineer"), "missing id:\n{frame}");
}

#[test]
fn composer_pane_shows_idle_hint_then_draft_when_typing() {
    let mut state = ViewerState::new(vec![header("compose-target")]);
    let idle = render_to_strings(&state);
    assert!(idle.contains("composer (i=edit)"));

    state.composer_mode = true;
    state.composer_push('h');
    state.composer_push('i');
    let typing = render_to_strings(&state);
    assert!(typing.contains("composer (Enter=send"));
    assert!(typing.contains("hi"));

    state.composer_send();
    let after = render_to_strings(&state);
    assert!(after.contains("last sent: hi"));
    assert!(!after.contains("composer (Enter=send"));
}

#[tokio::test]
async fn footer_shows_agent_running_status() {
    let mut state = ViewerState::new(vec![header("agent-target")]);
    let handle = tokio::spawn(std::future::pending::<argo_types::ArgoResult<()>>());
    state.agent_handle = Some(handle);
    let frame = render_to_strings(&state);
    assert!(frame.contains("agent running"));
}

#[test]
fn footer_shows_agent_error_when_set() {
    let mut state = ViewerState::new(vec![header("agent-target")]);
    state.last_agent_error = Some("ANTHROPIC_API_KEY not set".to_string());
    let frame = render_to_strings(&state);
    assert!(frame.contains("agent error"));
    assert!(frame.contains("ANTHROPIC_API_KEY"));
}

#[test]
fn help_overlay_renders_keybinding_lines() {
    let mut state = ViewerState::new(vec![header("help-target")]);
    state.help_overlay = true;
    let frame = render_to_strings(&state);
    assert!(frame.contains("keybindings"));
    assert!(frame.contains("compose"));
    assert!(frame.contains("steer"));
}

#[test]
fn footer_includes_workspace_label_when_set() {
    let mut state = ViewerState::new(vec![header("ws-target")]);
    state.workspace_label = Some("argo".to_string());
    let frame = render_to_strings(&state);
    assert!(frame.contains("[ws=argo]"), "ws label missing: {frame}");
}

#[test]
fn footer_omits_workspace_label_when_unset() {
    let state = ViewerState::new(vec![header("ws-target")]);
    let frame = render_to_strings(&state);
    assert!(!frame.contains("[ws="), "unexpected ws label: {frame}");
}

#[test]
fn message_overlay_renders_pretty_json_body() {
    let mut state = ViewerState::new(vec![header("drill-target")]);
    state.selected_messages = vec![argo_store::MessageRow {
        id: vec![0u8; 16],
        session_id: vec![0u8; 16],
        role: "tool".into(),
        content_json:
            r#"[{"type":"tool_result","id":"abc","is_error":false,"content":[{"type":"text","text":"hello world"}]}]"#
                .into(),
        tool_calls_json: String::new(),
        tool_call_id: None,
        reasoning: None,
        provider_meta: "null".into(),
        created_at: 0,
        seq: 5,
    }];
    state.message_overlay = Some(0);
    let frame = render_to_strings(&state);
    assert!(frame.contains("seq=5"));
    assert!(frame.contains("role=tool"));
    assert!(frame.contains("hello world"));
    assert!(frame.contains("json"));
}

#[test]
fn message_overlay_drill_view_strips_json_noise() {
    let mut state = ViewerState::new(vec![header("drill-target")]);
    state.selected_messages = vec![argo_store::MessageRow {
        id: vec![0u8; 16],
        session_id: vec![0u8; 16],
        role: "tool".into(),
        content_json:
            r#"[{"type":"tool_result","id":"xyz","is_error":false,"content":[{"type":"text","text":"DRILLED"}]}]"#
                .into(),
        tool_calls_json: String::new(),
        tool_call_id: None,
        reasoning: None,
        provider_meta: "null".into(),
        created_at: 0,
        seq: 7,
    }];
    state.message_overlay = Some(0);
    state.message_overlay_text_only = true;
    let frame = render_to_strings(&state);
    assert!(frame.contains("drill"), "title mode tag missing: {frame}");
    assert!(frame.contains("← xyz"));
    assert!(frame.contains("DRILLED"));
    // The pretty-JSON braces shouldn't appear when drilling.
    assert!(!frame.contains("\"is_error\""), "json leaked: {frame}");
}

#[test]
fn messages_pane_appends_live_event_trail() {
    let mut state = ViewerState::new(vec![header("live-target")]);
    state.push_live_event(&serde_json::json!({
        "event": "TurnStart",
        "payload": {"provider": "anthropic", "model": "claude-x"}
    }));
    state.push_live_event(&serde_json::json!({
        "event": "ToolCall",
        "payload": {"tool": "file_read"}
    }));
    let frame = render_to_strings(&state);
    assert!(frame.contains("live"));
    assert!(frame.contains("TurnStart"));
    assert!(frame.contains("file_read"));
}

#[test]
fn first_line_summarizes_tool_use_and_result() {
    let json = serde_json::json!([
        {"type":"tool_use","id":"01","name":"file_read","arguments":{"path":"/x"}}
    ])
    .to_string();
    assert!(first_line_of_content(&json).starts_with("→ file_read"));

    let res = serde_json::json!([
        {"type":"tool_result","id":"abc","is_error":true,
         "content":[{"type":"text","text":"boom"}]}
    ])
    .to_string();
    let line = first_line_of_content(&res);
    assert!(line.contains("← abc"));
    assert!(line.contains("[error]"));
    assert!(line.contains("boom"));
}
