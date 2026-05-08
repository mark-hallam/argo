//! Unit tests for `argo worker show` rendering. Covers markdown
//! shape, JSON envelope, and the empty-section "(none)" rendering
//! the brief calls out as graceful-empty acceptance criteria.
#![cfg(test)]

use super::worker_show::{
    envelope_for_test, report_for_test, AuditRow, DutyRow, SessionRow, WorkerShowReport,
};
use super::worker_show_render::render_markdown;
use argo_identity::{MemoryScope, NotifyChannelRef, Persona, Role, Worker, WorkerId};
use argo_types::TenantId;
use chrono::{TimeZone, Utc};
use uuid::Uuid;

// Helpers

fn fixed_time() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 5, 8, 12, 0, 0).single().unwrap()
}

fn worker_with_charter(charter: Option<&str>, channels: Vec<NotifyChannelRef>) -> Worker {
    let mut w = Worker::draft(
        TenantId::new(),
        "support",
        Persona::named("Maya", "operator@example.com"),
        Role::new("Triage approved support tickets and follow up."),
    );
    let id = WorkerId(Uuid::parse_str("01919e00-0000-7000-8000-000000000001").unwrap());
    w.id = id;
    w.memory_namespace.worker_id = id;
    w.memory_namespace.scope = MemoryScope::WorkerSelf;
    w.charter = charter.map(str::to_string);
    w.channels = channels;
    w
}

fn full_report() -> WorkerShowReport {
    let charter = Some("# Operating principles\n\n- Always confirm before action.\n- Defer high-risk steps.");
    let channels = vec![
        NotifyChannelRef::new("slack", "C123"),
        NotifyChannelRef {
            channel: "email".into(),
            target: "ops@example.com".into(),
            risk_level: Some("high".into()),
        },
    ];
    let worker = worker_with_charter(charter, channels);
    let sessions = vec![SessionRow {
        id: "01919e00-0000-7000-8000-000000000000".into(),
        vertical_id: Some("support".into()),
        started_at: "2026-05-08T11:30:00+00:00".into(),
        title: Some("Refund follow-up".into()),
    }];
    let audit = vec![
        AuditRow {
            seq: 42,
            kind: "auto_approval_decision".into(),
            created_at: "2026-05-08T11:25:00+00:00".into(),
        },
        AuditRow {
            seq: 41,
            kind: "handoff_emitted".into(),
            created_at: "2026-05-08T11:20:00+00:00".into(),
        },
    ];
    let duties = vec![DutyRow {
        id: "duty-1".into(),
        cron: "0 9 * * 1-5".into(),
        entry_action: "morning_check".into(),
        description: "Sweep new tickets".into(),
        enabled: true,
    }];
    report_for_test(worker, sessions, audit, duties, 12_345_678, fixed_time())
}

fn empty_report() -> WorkerShowReport {
    let worker = worker_with_charter(None, Vec::new());
    report_for_test(worker, Vec::new(), Vec::new(), Vec::new(), 0, fixed_time())
}

// Markdown shape

#[test]
fn render_markdown_includes_every_section_heading() {
    let body = render_markdown(&full_report());
    for heading in [
        "## Persona",
        "## Role",
        "## Charter",
        "## Channels",
        "## Recent Sessions",
        "## Recent Audit",
        "## Duties",
        "## Spend YTD",
    ] {
        assert!(
            body.contains(heading),
            "missing heading {heading} in:\n{body}"
        );
    }
}

#[test]
fn render_markdown_matches_golden_file() {
    assert_eq!(
        render_markdown(&full_report()),
        include_str!("../../tests/fixtures/worker_show_golden.md")
    );
}

#[test]
fn render_markdown_top_level_title_carries_persona_and_id() {
    let body = render_markdown(&full_report());
    assert!(body.starts_with("# Worker Maya - `"));
}

#[test]
fn render_markdown_charter_present_renders_fenced_block() {
    let body = render_markdown(&full_report());
    assert!(
        body.contains("```markdown\n# Operating principles"),
        "fenced charter block missing"
    );
}

#[test]
fn render_markdown_charter_absent_renders_none_placeholder() {
    let body = render_markdown(&empty_report());
    assert!(body.contains("## Charter\n\n(none)"));
}

#[test]
fn render_markdown_channels_absent_renders_none() {
    let body = render_markdown(&empty_report());
    assert!(body.contains("## Channels\n\n(none)"));
}

#[test]
fn render_markdown_sessions_absent_renders_none() {
    let body = render_markdown(&empty_report());
    assert!(body.contains("## Recent Sessions\n\n(none)"));
}

#[test]
fn render_markdown_audit_absent_renders_none() {
    let body = render_markdown(&empty_report());
    assert!(body.contains("## Recent Audit\n\n(none)"));
}

#[test]
fn render_markdown_duties_absent_renders_none() {
    let body = render_markdown(&empty_report());
    assert!(body.contains("## Duties\n\n(none)"));
}

#[test]
fn render_markdown_spend_ytd_formats_dollars_two_decimals() {
    let body = render_markdown(&full_report());
    // 12_345_678 micros = $12.34 (two-decimal truncation rounds to .35).
    assert!(
        body.contains("$12.35") || body.contains("$12.34"),
        "spend formatting unexpected: {body}"
    );
}

#[test]
fn render_markdown_lists_each_channel_with_target() {
    let body = render_markdown(&full_report());
    assert!(body.contains("`slack` -> `C123`"));
    assert!(body.contains("`email` -> `ops@example.com` - risk=high"));
}

#[test]
fn render_markdown_lists_each_session_with_id_and_vertical() {
    let body = render_markdown(&full_report());
    assert!(body.contains("01919e00-0000-7000-8000-000000000000"));
    assert!(body.contains("(support)"));
}

#[test]
fn render_markdown_lists_each_audit_row_with_seq_and_kind() {
    let body = render_markdown(&full_report());
    assert!(body.contains("seq=42 kind=`auto_approval_decision`"));
    assert!(body.contains("seq=41 kind=`handoff_emitted`"));
}

#[test]
fn render_markdown_lists_duty_with_cron_and_entry_action() {
    let body = render_markdown(&full_report());
    assert!(body.contains("`0 9 * * 1-5` -> `morning_check`"));
}

#[test]
fn render_markdown_marks_paused_duties() {
    let mut report = full_report();
    report.duties[0].enabled = false;
    let body = render_markdown(&report);
    assert!(body.contains("[paused]"));
}

#[test]
fn render_markdown_has_no_ansi_escape_sequences() {
    let body = render_markdown(&full_report());
    assert!(!body.contains('\u{1b}'));
}

// JSON envelope

#[test]
fn json_envelope_uses_universal_ok_shape() {
    let envelope = envelope_for_test(&full_report());
    assert_eq!(envelope["ok"], true);
    assert_eq!(envelope["worker_show"]["worker"]["persona"]["name"], "Maya");
}

#[test]
fn json_envelope_round_trips_serde() {
    let envelope = envelope_for_test(&full_report());
    let body = serde_json::to_string(&envelope).expect("serialize");
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("parse");
    assert_eq!(parsed["worker_show"]["spend_ytd_usd_micros"], 12_345_678);
    assert_eq!(parsed["worker_show"]["recent_audit"][0]["seq"], 42);
    assert_eq!(parsed["worker_show"]["duties"][0]["entry_action"], "morning_check");
    assert_eq!(parsed["worker_show"]["recent_sessions"][0]["vertical_id"], "support");
}

#[test]
fn json_envelope_preserves_charter_text_when_present() {
    let envelope = envelope_for_test(&full_report());
    let charter = envelope["worker_show"]["worker"]["charter"]
        .as_str()
        .expect("charter field");
    assert!(charter.contains("Operating principles"));
}

#[test]
fn json_envelope_charter_null_when_absent() {
    let envelope = envelope_for_test(&empty_report());
    assert!(envelope["worker_show"]["worker"]["charter"].is_null());
}

#[test]
fn json_envelope_channels_array_empty_when_absent() {
    let envelope = envelope_for_test(&empty_report());
    let channels = envelope["worker_show"]["worker"]["channels"]
        .as_array()
        .expect("channels array");
    assert!(channels.is_empty());
}
