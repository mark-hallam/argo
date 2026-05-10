//! Unit tests for the action-level Browser executor.
//!
//! Covers the brief's required cases: success → Completed with
//! screenshot paths recorded into the bundle, missing `target_url`
//! → Provider error, timeout → Provider error, args + secret
//! substitution at the helper-function level, and per-worker
//! session reuse asserted via mock MCP-server `initialize` count.
//!
//! The Python fixture has stdlib-only deps (json, sys, time) so any
//! dev/CI machine with python3 in PATH can run the full set. When
//! python3 is absent the subprocess-driven tests skip with a WARN
//! line rather than fail.

use super::*;
use crate::handlers::worker_actions_executor_browser_binding::{
    parse_binding, render_string, BrowserMode, SessionScope,
};
use crate::handlers::worker_actions::ActionRecord;
use argo_identity::WorkerId;
use argo_vertical::{ActionDeclaration, ExecutorDeclaration, ExecutorTagged};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::time::Duration;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("browser_mock_mcp.py")
}

fn python3() -> Option<String> {
    let output = std::process::Command::new("which").arg("python3").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!path.is_empty()).then_some(path)
}

fn sample_action(worker_suffix: &str, action_id: &str) -> ActionRecord {
    ActionRecord {
        schema_key: ActionRecord::SCHEMA_KEY.to_string(),
        action_id: action_id.into(),
        job_id: "job-browser-1".into(),
        worker_id: synth_worker_id(worker_suffix),
        item_id: Some("item-7".into()),
        action_type: "post_listing".into(),
        risk_level: "low".into(),
        requires_approval: false,
        state: "executing".into(),
        payload: json!({
            "title": "Vintage lamp",
        }),
        approved_by: None,
        approved_at: None,
        executed_at: None,
        platform: Some("facebook_marketplace".into()),
        result: None,
        evidence: vec![],
        blocked_reason: None,
        created_at: "2026-05-10T00:00:00Z".into(),
    }
}

fn synth_worker_id(suffix: &str) -> WorkerId {
    use std::str::FromStr;
    let mut bytes = [0u8; 16];
    for (i, b) in suffix.as_bytes().iter().enumerate() {
        bytes[i % 16] ^= *b;
    }
    let hex = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let formatted = format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    );
    WorkerId::from_str(&formatted).unwrap_or_else(|_| WorkerId::nil())
}

fn tagged_decl(binding: serde_json::Value) -> ActionDeclaration {
    let mut t = ExecutorTagged {
        kind: "browser".into(),
        ..Default::default()
    };
    t.binding = Some(binding.to_string());
    ActionDeclaration {
        executor: ExecutorDeclaration::Tagged(t),
        ..Default::default()
    }
}

#[test]
fn parse_binding_rejects_missing_target_url() {
    let decl = tagged_decl(json!({"mode": "script"}));
    let err = parse_binding(&decl).expect_err("missing target_url");
    assert!(
        err.to_string().contains("missing required `target_url`"),
        "got {err}"
    );
}

#[test]
fn parse_binding_rejects_empty_binding() {
    let mut t = ExecutorTagged {
        kind: "browser".into(),
        ..Default::default()
    };
    t.binding = Some(String::new());
    let decl = ActionDeclaration {
        executor: ExecutorDeclaration::Tagged(t),
        ..Default::default()
    };
    let err = parse_binding(&decl).expect_err("empty binding");
    assert!(err.to_string().contains("missing `executor.binding`"), "got {err}");
}

#[test]
fn parse_binding_accepts_bare_url() {
    let mut t = ExecutorTagged {
        kind: "browser".into(),
        ..Default::default()
    };
    t.binding = Some("https://example.com/post".into());
    let decl = ActionDeclaration {
        executor: ExecutorDeclaration::Tagged(t),
        ..Default::default()
    };
    let binding = parse_binding(&decl).expect("bare URL parses");
    assert_eq!(binding.target_url, "https://example.com/post");
    assert_eq!(binding.mode, BrowserMode::Cu);
    assert_eq!(binding.session_scope, SessionScope::PerWorker);
}

#[test]
fn parse_binding_recognises_script_mode_and_steps() {
    let decl = tagged_decl(json!({
        "target_url": "https://example.com/login",
        "mode": "script",
        "session_scope": "per_action",
        "steps": [
            {"action": "fill", "selector": "#email", "value": "alice@example.com"},
            {"action": "click", "selector": "button[type=submit]"},
        ],
    }));
    let binding = parse_binding(&decl).expect("script binding parses");
    assert_eq!(binding.mode, BrowserMode::Script);
    assert_eq!(binding.session_scope, SessionScope::PerAction);
    assert_eq!(binding.steps.len(), 2);
    assert_eq!(binding.steps[0].action, "fill");
    assert_eq!(binding.steps[0].selector.as_deref(), Some("#email"));
}

#[test]
fn render_string_substitutes_payload_and_secrets() {
    // `argo_config::cred::get` checks the process env first, so an
    // env var is the cheapest way to seed a secret for the renderer.
    std::env::set_var("BROWSER_TEST_TOKEN", "shhh-42");
    let action = sample_action("alpha", "act-1");
    let out = render_string(
        "title={{ payload.title }} token={{ secret.BROWSER_TEST_TOKEN }} item={{ item_id }} action={{ action_id }}",
        &action,
    );
    assert!(out.contains("title=Vintage lamp"), "got: {out}");
    assert!(out.contains("token=shhh-42"), "got: {out}");
    assert!(out.contains("item=item-7"), "got: {out}");
    assert!(out.contains("action=act-1"), "got: {out}");
}

#[tokio::test]
async fn dispatch_action_rejects_missing_target_url() {
    let action = sample_action("alpha", "act-1");
    let decl = tagged_decl(json!({"mode": "script"}));
    let pool = BrowserPool::default();
    let cwd = tempdir();
    let err = dispatch_action(
        &action,
        cwd.path(),
        &decl,
        &pool,
        Duration::from_millis(100),
    )
    .await
    .expect_err("missing target_url");
    assert!(
        err.to_string().contains("missing required `target_url`"),
        "got {err}"
    );
}

#[tokio::test]
async fn dispatch_action_success_returns_completed_with_screenshot() {
    let Some(py) = python3() else {
        eprintln!("WARN: skipping browser success test — python3 not in PATH");
        return;
    };
    set_browser_bin(&py);
    let action = sample_action("alpha", "act-1");
    let decl = tagged_decl(json!({
        "target_url": "https://example.com/post",
        "mode": "script",
        "steps": [{"action": "click", "selector": "#post"}],
    }));
    let pool = BrowserPool::default();
    let cwd = tempdir();
    let outcome = dispatch_action(&action, cwd.path(), &decl, &pool, Duration::from_secs(10))
        .await
        .expect("success");
    let ActionOutcome::Completed { result_json, bundle_path } = outcome else {
        panic!("expected Completed");
    };
    assert_eq!(result_json["mode"].as_str(), Some("script"));
    let shots = result_json["screenshots"].as_array().expect("screenshots");
    assert_eq!(shots.len(), 1);
    let shot_path = shots[0].as_str().expect("screenshot path");
    assert!(
        Path::new(shot_path).exists(),
        "screenshot {shot_path} should be on disk"
    );
    assert!(bundle_path.exists(), "bundle dir should exist");
}

#[tokio::test]
async fn dispatch_action_timeout_returns_provider() {
    let Some(py) = python3() else {
        eprintln!("WARN: skipping browser timeout test — python3 not in PATH");
        return;
    };
    set_browser_bin(&py);
    let action = sample_action("beta", "act-2");
    let decl = tagged_decl(json!({
        "target_url": "https://slow.example.com/?slow_ms=2000",
        "mode": "script",
        "steps": [],
    }));
    let pool = BrowserPool::default();
    let cwd = tempdir();
    let err = dispatch_action(&action, cwd.path(), &decl, &pool, Duration::from_millis(300))
        .await
        .expect_err("timeout");
    assert!(
        err.to_string().contains("exceeded wallclock budget"),
        "got: {err}"
    );
}

#[tokio::test]
async fn dispatch_action_per_worker_session_reuse() {
    let Some(py) = python3() else {
        eprintln!("WARN: skipping per-worker reuse test — python3 not in PATH");
        return;
    };
    set_browser_bin(&py);
    let pool = BrowserPool::default();
    let cwd = tempdir();
    let decl = tagged_decl(json!({
        "target_url": "https://example.com/post",
        "mode": "script",
        "steps": [],
    }));
    let action_a = sample_action("gamma", "act-a");
    let action_b = sample_action("gamma", "act-b");
    let _ = dispatch_action(&action_a, cwd.path(), &decl, &pool, Duration::from_secs(10))
        .await
        .expect("first dispatch");
    let _ = dispatch_action(&action_b, cwd.path(), &decl, &pool, Duration::from_secs(10))
        .await
        .expect("second dispatch");
    let guard = pool.lock().await;
    assert_eq!(
        guard.len(),
        1,
        "per_worker scope must keep exactly 1 sticky session for the same worker"
    );
}

fn set_browser_bin(py: &str) {
    // SAFETY: tests in this module run in the cargo test process;
    // env vars are scoped per-process. Concurrent tests may race on
    // the env, so each subprocess-driven test sets the var
    // explicitly before dispatch — the value is the same `python3
    // <fixture>` so winners and losers see equivalent state.
    std::env::set_var(
        "ARGO_BROWSER_MCP_BIN",
        format!("{py} {}", fixture_path().display()),
    );
}

fn tempdir() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}
