//! Unit tests for [`crate::session_detail`]. Covers tab cycling,
//! key dispatch, and per-tab rendering against a `TestBackend` so
//! we exercise the renderer end-to-end without a live terminal.
use super::*;
use argo_store::{AuditRow, MessageRow};
use argo_types::SessionId;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn make_message(seq: i64, role: &str, text: &str) -> MessageRow {
    let content = serde_json::json!([{"type": "text", "text": text}]);
    MessageRow {
        id: vec![0; 16],
        session_id: vec![0; 16],
        role: role.into(),
        content_json: content.to_string(),
        tool_calls_json: "[]".into(),
        tool_call_id: None,
        reasoning: None,
        provider_meta: "{}".into(),
        created_at: 0,
        seq,
    }
}

fn make_audit(seq: i64, kind: &str, payload: &str) -> AuditRow {
    AuditRow {
        seq,
        kind: kind.into(),
        payload_json: payload.into(),
        created_at: 0,
    }
}

fn render_pane(pane: &SessionDetailPane) -> String {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| pane.render(f, f.area()))
        .unwrap();
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
fn renders_messages_tab_with_five_messages() {
    let mut pane = SessionDetailPane::new(SessionId::new());
    pane.messages = vec![
        make_message(1, "user", "hello world"),
        make_message(2, "assistant", "how can I help"),
        make_message(3, "user", "tell me about argo"),
        make_message(4, "assistant", "argo is an agent platform"),
        make_message(5, "user", "thanks"),
    ];
    pane.cost = CostSummary::from_messages(&pane.messages);
    let frame = render_pane(&pane);
    assert!(frame.contains("Messages"), "tab bar missing:\n{frame}");
    assert!(frame.contains("Messages (5)"), "title missing:\n{frame}");
    assert!(frame.contains("hello world"), "row 1 missing:\n{frame}");
    assert!(frame.contains("argo is an agent"), "row 4 missing:\n{frame}");
}

#[test]
fn renders_audit_tab_with_three_events() {
    let mut pane = SessionDetailPane::new(SessionId::new());
    pane.audit = vec![
        make_audit(1, "turn_start", "{}"),
        make_audit(2, "tool_call", "{\"tool\":\"web\"}"),
        make_audit(3, "turn_end", "{\"ok\":true}"),
    ];
    pane.tab = Tab::Audit;
    let frame = render_pane(&pane);
    assert!(frame.contains("Audit (3)"), "title missing:\n{frame}");
    assert!(frame.contains("turn_start"), "kind missing:\n{frame}");
    assert!(frame.contains("tool_call"), "kind missing:\n{frame}");
    assert!(frame.contains("turn_end"), "kind missing:\n{frame}");
}

#[test]
fn renders_cost_tab_with_non_zero_spend() {
    let mut pane = SessionDetailPane::new(SessionId::new());
    let mut m = make_message(1, "assistant", "reply");
    m.provider_meta = serde_json::json!({"spend_usd_micros": 12_345_678u64}).to_string();
    pane.messages = vec![m];
    pane.cost = CostSummary::from_messages(&pane.messages);
    pane.tab = Tab::Cost;
    let frame = render_pane(&pane);
    assert!(frame.contains("Cost"), "tab bar missing:\n{frame}");
    assert!(frame.contains("$"), "spend marker missing:\n{frame}");
    // 12_345_678 micros = $12.345678 — formatted to 4dp.
    assert!(frame.contains("12.3457"), "spend value missing:\n{frame}");
    assert!(frame.contains("messages"), "label missing:\n{frame}");
    assert_eq!(pane.cost.message_count, 1);
}

#[test]
fn renders_skills_tab_with_two_active_skills() {
    use argo_skill::{LifecycleState, Provenance, Skill, SkillFrontmatter};
    use argo_types::{CapabilitySet, Lock, SkillId};
    let mut pane = SessionDetailPane::new(SessionId::new());
    let make_skill = |name: &str, version: &str| Skill {
        id: SkillId::new(),
        frontmatter: SkillFrontmatter {
            name: name.into(),
            version: version.into(),
            description: "d".into(),
            capabilities: CapabilitySet::default(),
            provenance: Provenance {
                source: "builtin".into(),
                hash: "sha256-x".into(),
            },
            lock: Lock::None,
            evals: Vec::new(),
        },
        body: String::new(),
        hash: [0u8; 32],
        learned_in: None,
        superseded_by: None,
        created_at: chrono::Utc::now(),
        vertical_id: None,
        lifecycle_state: LifecycleState::Active,
    };
    pane.skills = vec![make_skill("alpha", "0.1"), make_skill("beta", "0.2")];
    pane.tab = Tab::Skills;
    let frame = render_pane(&pane);
    assert!(frame.contains("Skills (2)"), "title missing:\n{frame}");
    assert!(frame.contains("alpha"), "skill 1 missing:\n{frame}");
    assert!(frame.contains("beta"), "skill 2 missing:\n{frame}");
}

#[test]
fn tab_and_shift_tab_cycle_correctly() {
    let mut pane = SessionDetailPane::new(SessionId::new());
    assert_eq!(pane.tab, Tab::Messages);
    assert_eq!(pane.handle_key(key(KeyCode::Tab)), DetailOutcome::Stay);
    assert_eq!(pane.tab, Tab::Audit);
    pane.handle_key(key(KeyCode::Tab));
    assert_eq!(pane.tab, Tab::Cost);
    pane.handle_key(key(KeyCode::Tab));
    assert_eq!(pane.tab, Tab::Skills);
    pane.handle_key(key(KeyCode::Tab));
    assert_eq!(pane.tab, Tab::Messages, "wraps to Messages");

    // Shift-Tab goes the other way.
    pane.handle_key(key(KeyCode::BackTab));
    assert_eq!(pane.tab, Tab::Skills, "BackTab wraps backward");
    pane.handle_key(key(KeyCode::BackTab));
    assert_eq!(pane.tab, Tab::Cost);

    // Switching tabs resets scroll so the new body starts at the top.
    pane.scroll = 7;
    pane.handle_key(key(KeyCode::Tab));
    assert_eq!(pane.scroll, 0);
}

#[test]
fn q_returns_detail_outcome_close() {
    let mut pane = SessionDetailPane::new(SessionId::new());
    assert_eq!(pane.handle_key(key(KeyCode::Char('q'))), DetailOutcome::Close);
    assert_eq!(pane.handle_key(key(KeyCode::Esc)), DetailOutcome::Close);
    // Other keys keep the pane open.
    assert_eq!(pane.handle_key(key(KeyCode::Char('j'))), DetailOutcome::Stay);
}
