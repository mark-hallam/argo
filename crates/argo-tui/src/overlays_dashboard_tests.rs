//! Unit + snapshot tests for the `D` Dashboard overlay (Stream
//! `claude-tui-realtime-dashboard`). Lives in a sibling file so
//! `overlays_dashboard.rs` stays under the 500-LOC budget.
use super::{
    draw_dashboard, format_pending_line, format_worker_line, human_age, payload_headline,
    AuditTailEntry, DashboardState, AUDIT_TAIL_CAPACITY,
};
use crate::overlays_h_stack::PanelState;
use ratatui::{backend::TestBackend, Terminal};
use serde_json::json;

#[test]
fn dashboard_state_new_starts_loading_and_empty() {
    let s = DashboardState::new();
    assert!(matches!(s.workers, PanelState::Loading));
    assert!(matches!(s.pending, PanelState::Loading));
    assert!(s.audit_tail.is_empty());
    assert_eq!(s.audit_scroll, 0);
    assert!(s.is_empty());
}

#[test]
fn set_audit_tail_caps_to_capacity_and_clamps_scroll() {
    let mut s = DashboardState::new();
    s.audit_scroll = 100; // pretend operator scrolled before refresh
    let entries = (0..(AUDIT_TAIL_CAPACITY + 5))
        .map(|i| AuditTailEntry {
            kind: "k".into(),
            age: "0s ago".into(),
            headline: format!("h{i}"),
            session_short: None,
        })
        .collect();
    s.set_audit_tail(entries);
    assert_eq!(s.audit_tail.len(), AUDIT_TAIL_CAPACITY);
    assert_eq!(s.audit_scroll, AUDIT_TAIL_CAPACITY - 1);
    assert!(!s.audit_loading);
}

#[test]
fn scroll_audit_down_clamps_at_last_row() {
    let mut s = DashboardState::new();
    s.set_audit_tail(vec![
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
    ]);
    s.scroll_audit_down();
    assert_eq!(s.audit_scroll, 1);
    s.scroll_audit_down();
    assert_eq!(s.audit_scroll, 1, "clamp at last row");
}

#[test]
fn scroll_audit_up_clamps_at_zero() {
    let mut s = DashboardState::new();
    s.scroll_audit_up();
    assert_eq!(s.audit_scroll, 0);
}

#[test]
fn human_age_handles_seconds_minutes_hours_days_and_zero() {
    let now = 10_000_000_000_i64;
    assert_eq!(human_age(now, now), "now");
    assert_eq!(human_age(now, now - 500_000), "now"); // sub-second
    assert_eq!(human_age(now, now - 5_000_000), "5s ago");
    assert_eq!(human_age(now, now - 90_000_000), "1m ago");
    assert_eq!(human_age(now, now - 3_700_000_000), "1h ago");
    assert_eq!(human_age(now, now - 90_000_000_000), "1d ago");
}

#[test]
fn payload_headline_prefers_tool_then_action_then_model_then_text() {
    assert_eq!(
        payload_headline("tool_call", &json!({"tool": "gmail_search"})),
        "tool_call · gmail_search"
    );
    assert_eq!(
        payload_headline("action_emit", &json!({"action_kind": "stripe_charge"})),
        "action_emit · stripe_charge"
    );
    assert_eq!(
        payload_headline("turn_start", &json!({"model": "claude-sonnet"})),
        "turn_start · claude-sonnet"
    );
    let with_text = payload_headline(
        "assistant_message",
        &json!({"text": "this is a long enough message that we should truncate"}),
    );
    assert!(with_text.starts_with("assistant_message · "));
    assert!(with_text.len() <= "assistant_message · ".len() + 40);
    assert_eq!(payload_headline("opaque", &json!({})), "opaque");
}

#[test]
fn format_worker_line_prints_short_id_and_spend() {
    let v = json!({
        "worker_id": "12345678-aaaa-bbbb-cccc-deadbeefcafe",
        "role": "support",
        "last_action": "gmail_send",
        "today_usd": 0.42,
        "paused": false,
    });
    let line = format_worker_line(&v);
    assert!(line.starts_with("12345678"), "short id at head: {line}");
    assert!(line.contains("active"));
    assert!(line.contains("role=support"));
    assert!(line.contains("$0.4200"));

    let paused = json!({"worker_id": "abc", "paused": true});
    assert!(format_worker_line(&paused).contains("PAUSED"));
}

#[test]
fn format_pending_line_renders_age_with_now_clock() {
    let now = 1_700_000_000_000_000_i64;
    let v = json!({
        "action_id": "deadbeef-0000",
        "worker_id": "feedface-0000",
        "action_kind": "stripe_charge",
        "requested_at_micros": now - 120_000_000_i64,
    });
    let line = format_pending_line(&v, now);
    assert!(line.contains("deadbeef"));
    assert!(line.contains("worker=feedface"));
    assert!(line.contains("stripe_charge"));
    assert!(line.contains("2m ago"));
}

#[test]
fn dashboard_renders_loading_panes_to_test_backend_snapshot() {
    // 1-snapshot acceptance: render the freshly-opened dashboard
    // (all 3 panes Loading) into a TestBackend and assert the
    // expected hints + the close-key hint appear.
    let backend = TestBackend::new(120, 30);
    let mut term = Terminal::new(backend).expect("test terminal");
    let s = DashboardState::new();
    term.draw(|f| draw_dashboard(f, f.area(), &s)).unwrap();
    let buf = term.backend().buffer().clone();
    let mut text = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            text.push_str(buf[(x, y)].symbol());
        }
        text.push('\n');
    }
    assert!(
        text.contains("workers"),
        "workers pane title in snapshot:\n{text}"
    );
    assert!(
        text.contains("pending approvals"),
        "pending pane title in snapshot:\n{text}"
    );
    assert!(
        text.contains("audit tail"),
        "audit pane title in snapshot:\n{text}"
    );
    assert!(
        text.contains("loading workers"),
        "workers loading hint:\n{text}"
    );
    assert!(
        text.contains("loading pending approvals"),
        "pending loading hint:\n{text}"
    );
    assert!(
        text.contains("loading audit tail"),
        "audit loading hint:\n{text}"
    );
    assert!(text.contains("D/Esc/q"), "close hint visible:\n{text}");
}
