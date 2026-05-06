//! Unit tests for `argo trajectory export`.
//!
//! Coverage matrix (matches the brief's "8 unit tests" acceptance):
//!   * Resolved::from_args — happy paths + invalid format/filter
//!   * passes_filters — state filter + date filter
//!   * load_trajectory — round-trip a synthetic bundle
//!   * collect_trajectories — walks + sorts
//!   * render — anthropic / openai / jsonl shapes
//!   * redact_emails_in_str — basic replacement
//!
//! The `tests/trajectory_export_smoke.rs` integration test exercises
//! the full CLI through `argo trajectory export --json`.

use super::*;
use super::super::trajectory_export_render::{redact_emails_in_str, redact_value};
use serde_json::json;
use std::path::PathBuf;

fn args() -> ExportArgs {
    ExportArgs {
        since: None,
        until: None,
        format: "anthropic".into(),
        filter: "passed".into(),
        include_screenshots: false,
        gdpr_redact_emails: false,
        output: None,
        json: false,
    }
}

fn write_bundle(
    root: &std::path::Path,
    job_id: &str,
    action_id: &str,
    state: &str,
    created_at: &str,
) -> PathBuf {
    let bundle = root
        .join(job_id)
        .join("actions")
        .join(action_id);
    std::fs::create_dir_all(&bundle).unwrap();
    let action = json!({
        "schema_key": "worker_action",
        "action_id": action_id,
        "job_id": job_id,
        "action_type": "verify_price",
        "risk_level": "low",
        "requires_approval": false,
        "state": state,
        "payload": {"item_id": "skate-1"},
        "created_at": created_at,
    });
    std::fs::write(
        bundle.join("action.json"),
        serde_json::to_string_pretty(&action).unwrap(),
    )
    .unwrap();
    bundle
}

// ───────────────── Resolved::from_args ─────────────────

#[test]
fn from_args_accepts_default_anthropic_passed() {
    let r = Resolved::from_args(args()).expect("ok");
    assert!(matches!(r.format, Format::Anthropic));
    assert!(matches!(r.filter, Filter::Passed));
}

#[test]
fn from_args_rejects_invalid_format() {
    let mut a = args();
    a.format = "yaml".into();
    let err = Resolved::from_args(a).expect_err("must reject");
    assert!(err.to_string().contains("yaml"), "got: {err}");
    assert!(err.to_string().contains("anthropic"));
}

#[test]
fn from_args_rejects_invalid_filter() {
    let mut a = args();
    a.filter = "failed".into();
    let err = Resolved::from_args(a).expect_err("must reject");
    assert!(err.to_string().contains("failed"));
}

#[test]
fn from_args_parses_iso_and_ymd_dates() {
    let mut a = args();
    a.since = Some("2025-01-01".into());
    a.until = Some("2025-12-31T23:59:59Z".into());
    let r = Resolved::from_args(a).expect("ok");
    assert!(r.since_micros.is_some());
    assert!(r.until_micros.is_some());
}

// ───────────────── passes_filters ─────────────────

#[test]
fn passes_filters_drops_non_completed_when_filter_passed() {
    let r = Resolved::from_args(args()).unwrap();
    let t = Trajectory {
        action_id: "a".into(),
        action_type: "x".into(),
        job_id: "j".into(),
        state: "failed".into(),
        created_at: "2025-06-01T00:00:00Z".into(),
        steps: vec![],
    };
    assert!(!passes_filters(&t, &r));
}

#[test]
fn passes_filters_admits_failed_when_filter_all() {
    let mut a = args();
    a.filter = "all".into();
    let r = Resolved::from_args(a).unwrap();
    let t = Trajectory {
        action_id: "a".into(),
        action_type: "x".into(),
        job_id: "j".into(),
        state: "failed".into(),
        created_at: "2025-06-01T00:00:00Z".into(),
        steps: vec![],
    };
    assert!(passes_filters(&t, &r));
}

#[test]
fn passes_filters_drops_outside_date_window() {
    let mut a = args();
    a.since = Some("2025-06-01".into());
    a.until = Some("2025-06-30".into());
    let r = Resolved::from_args(a).unwrap();
    let t = Trajectory {
        action_id: "a".into(),
        action_type: "x".into(),
        job_id: "j".into(),
        state: "completed".into(),
        created_at: "2025-05-01T00:00:00Z".into(),
        steps: vec![],
    };
    assert!(!passes_filters(&t, &r));
}

// ───────────────── load + collect ─────────────────

#[test]
fn load_trajectory_synthesizes_request_step_for_non_cu_action() {
    let root = tempfile::tempdir().unwrap();
    let bundle = write_bundle(root.path(), "j-1", "a-1", "completed", "2025-06-01T00:00:00Z");
    let t = load_trajectory(&bundle, false).expect("load");
    assert_eq!(t.action_id, "a-1");
    assert_eq!(t.state, "completed");
    assert_eq!(t.steps.len(), 1, "non-CU action should synthesise one step");
    assert_eq!(t.steps[0].action["kind"], "action_request");
    assert_eq!(t.steps[0].result["kind"], "action_result");
}

#[test]
fn collect_trajectories_walks_sorted_and_filtered() {
    let root = tempfile::tempdir().unwrap();
    let outputs = root.path().join("outputs");
    write_bundle(&outputs, "j-1", "a-2", "completed", "2025-06-02T00:00:00Z");
    write_bundle(&outputs, "j-1", "a-1", "completed", "2025-06-01T00:00:00Z");
    write_bundle(&outputs, "j-2", "a-3", "failed", "2025-06-03T00:00:00Z");
    let r = Resolved::from_args(args()).unwrap();
    let collected = collect_trajectories(&outputs, &r).expect("ok");
    assert_eq!(collected.len(), 2, "filter passed drops the failed one");
    assert_eq!(collected[0].action_id, "a-1");
    assert_eq!(collected[1].action_id, "a-2");
}

// ───────────────── render shapes ─────────────────

#[test]
fn render_anthropic_emits_user_then_assistant_messages_per_step() {
    use super::super::trajectory_export_render::render;
    let t = Trajectory {
        action_id: "a".into(),
        action_type: "x".into(),
        job_id: "j".into(),
        state: "completed".into(),
        created_at: "2025-06-01T00:00:00Z".into(),
        steps: vec![TrajectoryStep {
            at: "2025-06-01T00:00:00Z".into(),
            action: json!({"act": "click"}),
            result: json!({"ok": true}),
            screenshot: None,
        }],
    };
    let mut a = args();
    a.format = "anthropic".into();
    let r = Resolved::from_args(a).unwrap();
    let bytes = render(&[t], &r).expect("render");
    let body = String::from_utf8(bytes).unwrap();
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    let messages = v[0]["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0]["role"], "user");
    assert_eq!(messages[1]["role"], "assistant");
}

#[test]
fn render_jsonl_emits_one_object_per_line() {
    use super::super::trajectory_export_render::render;
    let t1 = Trajectory {
        action_id: "a-1".into(),
        action_type: "x".into(),
        job_id: "j".into(),
        state: "completed".into(),
        created_at: "2025-06-01T00:00:00Z".into(),
        steps: vec![],
    };
    let t2 = Trajectory {
        action_id: "a-2".into(),
        ..t1.clone()
    };
    let mut a = args();
    a.format = "jsonl".into();
    let r = Resolved::from_args(a).unwrap();
    let bytes = render(&[t1, t2], &r).expect("render");
    let body = String::from_utf8(bytes).unwrap();
    let lines: Vec<&str> = body.lines().collect();
    assert_eq!(lines.len(), 2);
    for line in lines {
        let _: serde_json::Value = serde_json::from_str(line).expect("valid json line");
    }
}

#[test]
fn render_openai_emits_tool_calls() {
    use super::super::trajectory_export_render::render;
    let t = Trajectory {
        action_id: "a".into(),
        action_type: "x".into(),
        job_id: "j".into(),
        state: "completed".into(),
        created_at: "2025-06-01T00:00:00Z".into(),
        steps: vec![TrajectoryStep {
            at: "2025-06-01T00:00:00Z".into(),
            action: json!({"act": "click"}),
            result: json!({"ok": true}),
            screenshot: None,
        }],
    };
    let mut a = args();
    a.format = "openai".into();
    let r = Resolved::from_args(a).unwrap();
    let bytes = render(&[t], &r).expect("render");
    let body = String::from_utf8(bytes).unwrap();
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    let messages = v[0]["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[1]["role"], "assistant");
    assert!(messages[1]["tool_calls"].is_array());
}

// ───────────────── redaction ─────────────────

#[test]
fn redact_emails_in_str_replaces_addresses_with_marker() {
    let s = redact_emails_in_str("contact alice@example.com or bob@example.org for details");
    assert!(!s.contains("alice@example.com"), "got: {s}");
    assert!(!s.contains("bob@example.org"), "got: {s}");
    assert!(s.contains("<redacted>"), "got: {s}");
}

#[test]
fn redact_value_walks_nested_json() {
    let mut v = json!({
        "outer": {
            "list": ["mark@example.com", "no email here"],
            "deep": {"email": "deep@example.io"},
        },
    });
    redact_value(&mut v);
    let body = serde_json::to_string(&v).unwrap();
    assert!(!body.contains("@example"), "got: {body}");
    assert!(body.contains("<redacted>"));
}
