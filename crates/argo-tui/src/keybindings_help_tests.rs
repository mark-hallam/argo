use super::*;
use crate::app::{PaneFocus, ViewerState};
use argo_store::SessionHeader;
use argo_types::{TenantId, WorkspaceId};
use chrono::Utc;
use ratatui::{backend::TestBackend, Terminal};

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
        routing_policy: None,
    }
}

fn render_to_string(state: &ViewerState) -> String {
    let backend = TestBackend::new(90, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render(frame, frame.area(), state))
        .unwrap();
    let buffer = terminal.backend().buffer().clone();
    let mut out = String::new();
    for y in 0..24 {
        for x in 0..90 {
            out.push_str(buffer.cell((x, y)).map(|c| c.symbol()).unwrap_or(""));
        }
        out.push('\n');
    }
    out
}

#[test]
fn global_only_renders_global_section() {
    let sections = sections_for_surface(ParentSurface::GlobalOnly);

    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].title, "Global");
    assert!(sections[0]
        .bindings
        .iter()
        .any(|b| b.keys.contains("Ctrl-K")));
}

#[test]
fn sessions_list_renders_global_and_local_sections() {
    let sections = sections_for_surface(ParentSurface::SessionsList);

    assert_eq!(sections.len(), 2);
    assert_eq!(sections[0].title, "Global");
    assert_eq!(sections[1].title, "Sessions list");
    assert!(sections[1].bindings.iter().any(|b| b.keys == "j / k"));
}

#[test]
fn render_handles_empty_binding_section() {
    let lines = lines_for_sections(&[HelpSection {
        title: "Empty",
        bindings: Vec::new(),
    }]);

    assert!(lines
        .iter()
        .any(|line| line.to_string().contains("No keybindings")));
}

#[test]
fn q_and_question_mark_close_overlay() {
    assert_eq!(handle_key(KeyCode::Char('q')), HelpOutcome::Close);
    assert_eq!(handle_key(KeyCode::Esc), HelpOutcome::Close);
    assert_eq!(handle_key(KeyCode::Char('?')), HelpOutcome::Close);
    assert_eq!(handle_key(KeyCode::Char('j')), HelpOutcome::StayOpen);
}

#[test]
fn state_surface_tracks_current_focus_and_overlay() {
    let mut state = ViewerState::new(vec![header("focus")]);
    assert_eq!(surface_for_state(&state), ParentSurface::SessionsList);

    state.focus = PaneFocus::Messages;
    assert_eq!(surface_for_state(&state), ParentSurface::MessagesPane);

    state.cost_overlay = Some(argo_store::GlobalSpend::default());
    assert_eq!(
        surface_for_state(&state),
        ParentSurface::ListOverlay("Cost rollup")
    );
}

#[test]
fn render_shows_current_surface_bindings() {
    let mut state = ViewerState::new(vec![header("help-target")]);
    state.focus = PaneFocus::Messages;
    let frame = render_to_string(&state);

    assert!(frame.contains("keybindings"));
    assert!(frame.contains("Global"));
    assert!(frame.contains("Messages pane"));
    assert!(frame.contains("yank latest message"));
}
