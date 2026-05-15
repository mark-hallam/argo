use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, Terminal};

fn row(id: &str) -> PendingApproval {
    PendingApproval::new(
        id,
        format!("worker-{id}"),
        "post listing",
        format!("reason-{id}"),
        "2026-05-15T00:00:00Z",
    )
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn render_to_string(tray: &ApprovalTray) -> String {
    let backend = TestBackend::new(100, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| tray.render(frame, frame.area()))
        .unwrap();
    format!("{:?}", terminal.backend().buffer())
}

#[test]
fn render_with_zero_pending_shows_empty_state() {
    let frame = render_to_string(&ApprovalTray::default());

    assert!(frame.contains("No pending approvals"), "{frame}");
}

#[test]
fn render_with_five_pending_shows_worker_action_and_reason() {
    let tray = ApprovalTray::new((0..5).map(|idx| row(&idx.to_string())).collect());
    let frame = render_to_string(&tray);

    assert!(frame.contains("worker-0"), "{frame}");
    assert!(frame.contains("post listing"), "{frame}");
    assert!(frame.contains("reason-4"), "{frame}");
}

#[test]
fn j_and_k_navigate_selection() {
    let mut tray = ApprovalTray::new(vec![row("a"), row("b"), row("c")]);

    assert_eq!(tray.selected, 0);
    assert_eq!(
        tray.handle_key(key(KeyCode::Char('j'))),
        ApprovalOutcome::Stay
    );
    assert_eq!(tray.selected, 1);
    assert_eq!(
        tray.handle_key(key(KeyCode::Char('k'))),
        ApprovalOutcome::Stay
    );
    assert_eq!(tray.selected, 0);
}

#[test]
fn y_triggers_approve_for_selected_row() {
    let mut tray = ApprovalTray::new(vec![row("a"), row("b")]);
    tray.handle_key(key(KeyCode::Char('j')));

    assert_eq!(
        tray.handle_key(key(KeyCode::Char('y'))),
        ApprovalOutcome::Approve("b".to_string())
    );
}

#[test]
fn n_triggers_reject_for_selected_row() {
    let mut tray = ApprovalTray::new(vec![row("a")]);

    assert_eq!(
        tray.handle_key(key(KeyCode::Char('n'))),
        ApprovalOutcome::Reject("a".to_string())
    );
}

#[test]
fn parses_gateway_approval_rows() {
    let body = json!({
        "items": [{
            "id": "act-1",
            "worker": { "id": "worker-1", "name": "Maya" },
            "verb": "wants to draft listing",
            "detail": "Approve draft?",
            "received_at": "now"
        }]
    });

    let rows = parse_pending(&body);

    assert_eq!(
        rows,
        vec![PendingApproval::new(
            "act-1",
            "Maya",
            "draft listing",
            "Approve draft?",
            "now",
        )]
    );
}
