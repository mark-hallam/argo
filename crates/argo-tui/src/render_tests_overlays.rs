//! Snapshot tests for the data overlays (forge runs, skills,
//! memory, cost, audit, providers, cron) — extracted from
//! `render_tests.rs` so that file stays under the 500-LOC ceiling.
//! Helpers (`header`, `render_to_strings`) and `super::*` symbols
//! are inherited from the parent `tests` module.
use super::*;
use chrono::Utc;

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
            evals: Vec::new(),
        },
        body: String::new(),
        hash: [0xab; 32],
        learned_in: None,
        superseded_by: None,
        created_at: Utc::now(),
        vertical_id: None,
        lifecycle_state: argo_skill::LifecycleState::Active,
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
        vertical_id: None,
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
        today: 1_500_000,
        this_month: 12_000_000,
        all_time: 145_750_000,
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
fn cron_overlay_renders_names() {
    let mut state = ViewerState::new(vec![header("k-target")]);
    state.cron_overlay = Some(vec!["nightly-summary".into(), "weekly-roll".into()]);
    let frame = render_to_strings(&state);
    assert!(frame.contains("cron jobs (2)"), "title: {frame}");
    assert!(frame.contains("nightly-summary"));
    assert!(frame.contains("weekly-roll"));
}

#[test]
fn cron_overlay_handles_empty() {
    let mut state = ViewerState::new(vec![header("k-target")]);
    state.cron_overlay = Some(Vec::new());
    let frame = render_to_strings(&state);
    assert!(frame.contains("cron jobs (0)"));
    assert!(frame.contains("(no cron"));
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
