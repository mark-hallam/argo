//! Tests for `render::draw` — extracted from `render.rs` so
//! that file stays under the 500-LOC ceiling as the overlay
//! surface grows.
use super::*;
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
    }
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
fn forge_runs_overlay_renders_rows_and_header() {
    let mut state = ViewerState::new(vec![header("forge-runs")]);
    state.forge_runs = Some(vec![argo_store::ForgeRunRow {
        id: vec![0xab, 0xcd, 0xef, 0x12, 0x34, 0x56, 0x78, 0x9a, 0, 0, 0, 0, 0, 0, 0, 0],
        skill_name: "alpha".into(),
        started_at: 0,
        ended_at: Some(0),
        base_version: "0.1".into(),
        candidate_version: Some("0.2".into()),
        iterations: 3,
        spend_usd_micros: 0,
        final_score: Some(0.842),
        status: "succeeded".into(),
    }]);
    let frame = render_to_strings(&state);
    assert!(frame.contains("forge runs (1)"));
    assert!(frame.contains("alpha"));
    assert!(frame.contains("succeeded"));
    assert!(frame.contains("0.842"));
    assert!(frame.contains("auto-refresh"));
}

#[test]
fn forge_runs_overlay_marks_running_row_with_dot_prefix() {
    let mut state = ViewerState::new(vec![header("forge-runs")]);
    state.forge_runs = Some(vec![argo_store::ForgeRunRow {
        id: vec![0xfe, 0xed, 0xfa, 0xce, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        skill_name: "beta".into(),
        started_at: 0,
        ended_at: None,
        base_version: "0.3".into(),
        candidate_version: None,
        iterations: 0,
        spend_usd_micros: 0,
        final_score: None,
        status: "running".into(),
    }]);
    let frame = render_to_strings(&state);
    assert!(frame.contains("●"), "missing running marker: {frame}");
    assert!(frame.contains("beta"));
    assert!(frame.contains("running"));
    // Title shows the running count.
    assert!(frame.contains("1 ●"), "title missing running count: {frame}");
}

#[test]
fn skills_overlay_renders_active_skills() {
    use argo_skill::{Provenance, Skill, SkillFrontmatter};
    use argo_types::{CapabilitySet, Lock, SkillId};
    let mut state = ViewerState::new(vec![header("skills-target")]);
    state.skills_overlay = Some(vec![Skill {
        id: SkillId::new(),
        frontmatter: SkillFrontmatter {
            name: "summariser".into(),
            version: "0.4".into(),
            description: "summarise threads".into(),
            capabilities: CapabilitySet::default(),
            provenance: Provenance {
                source: "user".into(),
                hash: "sha256-abc".into(),
            },
            lock: Lock::None,
        },
        body: String::new(),
        hash: [0xab; 32],
        learned_in: None,
        superseded_by: None,
        created_at: Utc::now(),
    }]);
    let frame = render_to_strings(&state);
    assert!(frame.contains("active skills (1)"));
    assert!(frame.contains("summariser"));
    assert!(frame.contains("0.4"));
    assert!(frame.contains("none"));
}

#[test]
fn skills_overlay_handles_empty_list() {
    let mut state = ViewerState::new(vec![header("skills-target")]);
    state.skills_overlay = Some(Vec::new());
    let frame = render_to_strings(&state);
    assert!(frame.contains("active skills (0)"));
    assert!(frame.contains("(no active skills"));
}

#[test]
fn memory_overlay_renders_active_entries() {
    use argo_memory::{MemoryEntry, MemoryFile};
    use argo_types::{Lock, WorkspaceId};
    let mut state = ViewerState::new(vec![header("memory-target")]);
    let ws = WorkspaceId::new();
    state.memory_overlay = Some(vec![MemoryEntry {
        id: WorkspaceId::new().0,
        workspace_id: ws,
        file: MemoryFile::User,
        body: "Mark prefers Vim".into(),
        learned_in: None,
        learned_at: Utc::now(),
        superseded_by: None,
        hash: [0xcd; 32],
        lock: Lock::None,
    }]);
    let frame = render_to_strings(&state);
    assert!(frame.contains("active memory (1)"));
    assert!(frame.contains("user"));
    assert!(frame.contains("none"));
}

#[test]
fn cost_overlay_renders_today_mtd_and_all_time_totals() {
    let mut state = ViewerState::new(vec![header("cost-target")]);
    state.cost_overlay = Some(argo_store::GlobalSpend {
        today: 1_500_000,         // $1.50
        this_month: 12_000_000,   // $12.00
        all_time: 145_750_000,    // $145.75
    });
    let frame = render_to_strings(&state);
    assert!(frame.contains("cost rollup"));
    assert!(frame.contains("today:"));
    assert!(frame.contains("$      1.50"), "missing today: {frame}");
    assert!(frame.contains("$     12.00"), "missing mtd: {frame}");
    assert!(frame.contains("$    145.75"), "missing all-time: {frame}");
}

#[test]
fn audit_overlay_renders_chain_rows() {
    let mut state = ViewerState::new(vec![header("audit-target")]);
    state.audit_overlay = Some(vec![
        argo_store::AuditRow {
            seq: 7,
            kind: "TurnStart".into(),
            payload_json: r#"{"provider":"anthropic","model":"claude-x"}"#.into(),
            created_at: 0,
        },
        argo_store::AuditRow {
            seq: 8,
            kind: "ToolCall".into(),
            payload_json: r#"{"tool":"file_read"}"#.into(),
            created_at: 0,
        },
    ]);
    let frame = render_to_strings(&state);
    assert!(frame.contains("audit chain (2)"));
    assert!(frame.contains("TurnStart"));
    assert!(frame.contains("file_read"));
    assert!(frame.contains("anthropic"));
}

#[test]
fn providers_overlay_renders_ids() {
    let mut state = ViewerState::new(vec![header("p-target")]);
    state.providers_overlay = Some(vec!["anthropic".into(), "openai".into()]);
    let frame = render_to_strings(&state);
    assert!(frame.contains("providers (2)"), "title: {frame}");
    assert!(frame.contains("anthropic"));
    assert!(frame.contains("openai"));
}

#[test]
fn providers_overlay_handles_empty() {
    let mut state = ViewerState::new(vec![header("p-target")]);
    state.providers_overlay = Some(Vec::new());
    let frame = render_to_strings(&state);
    assert!(frame.contains("providers (0)"));
    assert!(frame.contains("(no manifests"));
}

#[test]
fn audit_overlay_handles_empty_chain() {
    let mut state = ViewerState::new(vec![header("audit-target")]);
    state.audit_overlay = Some(Vec::new());
    let frame = render_to_strings(&state);
    assert!(frame.contains("audit chain (0)"));
    assert!(frame.contains("(no audit rows"));
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
fn cost_overlay_renders_zero_state_for_fresh_workspace() {
    let mut state = ViewerState::new(vec![header("cost-target")]);
    state.cost_overlay = Some(argo_store::GlobalSpend::default());
    let frame = render_to_strings(&state);
    assert!(frame.contains("$      0.00"), "missing zero: {frame}");
}

#[test]
fn memory_overlay_handles_empty_list() {
    let mut state = ViewerState::new(vec![header("memory-target")]);
    state.memory_overlay = Some(Vec::new());
    let frame = render_to_strings(&state);
    assert!(frame.contains("active memory (0)"));
    assert!(frame.contains("(no active memory"));
}

#[test]
fn forge_runs_overlay_handles_empty_list() {
    let mut state = ViewerState::new(vec![header("forge-runs")]);
    state.forge_runs = Some(Vec::new());
    let frame = render_to_strings(&state);
    assert!(frame.contains("forge runs (0)"));
    assert!(frame.contains("(no forge runs"));
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
