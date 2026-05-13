//! Unit tests for the action-level MCP executor module.
//!
//! Covers the brief's six required cases: success / tool-error /
//! timeout (driven against a Python stdio fixture in
//! `tests/fixtures/mcp_echo_server.py`) plus missing-server /
//! missing-tool / args-substitution at the helper-function level.
//!
//! The Python fixture has stdlib-only deps (json, sys, time) so any
//! dev/CI machine with python3 in PATH can run the full set. When
//! python3 is absent the three subprocess-driven tests skip with a
//! WARN line rather than fail — argo's CI matrix has python3, but
//! we keep the door open for minimal Linux containers.

use super::*;
use crate::handlers::worker_actions::ActionRecord;
use argo_identity::WorkerId;
use argo_mcp::{McpClientPool, PoolConfig};
use argo_vertical::{ActionDeclaration, ExecutorDeclaration, ExecutorTagged, McpServerDeclaration};
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// Per-test [`McpClientPool`] so subprocesses spawned in one
/// `#[tokio::test]` runtime don't leak into another's.
/// `mission-mcp-pool-wire-dispatch` injects the pool through
/// `dispatch_action`; this helper keeps test isolation intact.
fn test_client_pool() -> Arc<McpClientPool> {
    McpClientPool::new(PoolConfig::default())
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("mcp_echo_server.py")
}

fn python3() -> Option<String> {
    // Honour PATH so workspace setups with venvs / homebrew etc. work
    // unchanged. Skip the test if no python3 is reachable.
    let output = std::process::Command::new("which").arg("python3").output();
    match output {
        Ok(o) if o.status.success() => {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        }
        _ => None,
    }
}

fn echo_server_decl() -> McpServerDeclaration {
    let py = python3().unwrap_or_default();
    McpServerDeclaration {
        transport: "stdio".into(),
        command: py,
        args: vec![fixture_path().to_string_lossy().into_owned()],
    }
}

fn sample_action() -> ActionRecord {
    ActionRecord {
        schema_key: ActionRecord::SCHEMA_KEY.to_string(),
        action_id: "act-1".into(),
        job_id: "job-1".into(),
        worker_id: WorkerId::nil(),
        item_id: Some("item-7".into()),
        action_type: "send_email".into(),
        risk_level: "low".into(),
        requires_approval: false,
        state: "executing".into(),
        payload: json!({
            "to": "{{ payload.recipient }}",
            "recipient": "buyer@example.com",
            "subject": "Hello {{ item_id }}",
            "body": json!({
                "greeting": "Dear customer",
                "trail": ["{{ action_id }}", 42, true],
            }),
        }),
        approved_by: None,
        approved_at: None,
        executed_at: None,
        platform: Some("gmail".into()),
        result: None,
        evidence: vec![],
        blocked_reason: None,
        created_at: "2026-05-07T00:00:00Z".into(),
    }
}

fn tagged_decl(server: Option<&str>, tool: Option<&str>) -> ActionDeclaration {
    let mut t = ExecutorTagged {
        kind: "mcp".into(),
        ..Default::default()
    };
    t.server = server.map(str::to_string);
    t.tool = tool.map(str::to_string);
    ActionDeclaration {
        executor: ExecutorDeclaration::Tagged(t),
        ..Default::default()
    }
}

fn echo_action(tool: &str, payload: serde_json::Value) -> ActionRecord {
    let mut a = sample_action();
    a.payload = payload;
    a.action_type = format!("call_{tool}");
    a
}

fn echo_servers() -> HashMap<String, McpServerDeclaration> {
    let mut servers = HashMap::new();
    servers.insert("echo".into(), echo_server_decl());
    servers
}

#[test]
fn args_substitution_walks_strings_objects_arrays() {
    let action = sample_action();
    let out = substitute_args(&action.payload, &action);
    let map = out.as_object().expect("object");
    assert_eq!(map.get("to").and_then(|v| v.as_str()), Some("buyer@example.com"));
    assert_eq!(
        map.get("subject").and_then(|v| v.as_str()),
        Some("Hello item-7")
    );
    let body = map.get("body").and_then(|v| v.as_object()).expect("body");
    let trail = body.get("trail").and_then(|v| v.as_array()).expect("trail");
    assert_eq!(trail[0].as_str(), Some("act-1"));
    assert_eq!(trail[1].as_i64(), Some(42));
    assert_eq!(trail[2].as_bool(), Some(true));
}

#[tokio::test]
async fn dispatch_action_rejects_missing_server() {
    let action = sample_action();
    let decl = tagged_decl(None, Some("echo"));
    let pool = McpPool::default();
    let client_pool = test_client_pool();
    let err = dispatch_action(
        &action,
        Path::new("."),
        &decl,
        &HashMap::new(),
        &pool,
        &client_pool,
        Duration::from_millis(100),
    )
    .await
    .expect_err("missing server");
    assert!(
        err.to_string().contains("missing `executor.server`"),
        "got {err}"
    );
}

#[tokio::test]
async fn dispatch_action_rejects_missing_tool() {
    let action = sample_action();
    let decl = tagged_decl(Some("gmail"), None);
    let pool = McpPool::default();
    let client_pool = test_client_pool();
    let err = dispatch_action(
        &action,
        Path::new("."),
        &decl,
        &HashMap::new(),
        &pool,
        &client_pool,
        Duration::from_millis(100),
    )
    .await
    .expect_err("missing tool");
    assert!(
        err.to_string().contains("missing `executor.tool`"),
        "got {err}"
    );
}

#[tokio::test]
async fn dispatch_action_success_path_returns_completed() {
    let Some(_) = python3() else {
        eprintln!("WARN: skipping success test — python3 not in PATH");
        return;
    };
    let action = echo_action("echo", json!({"text": "hi {{ item_id }}"}));
    let decl = tagged_decl(Some("echo"), Some("echo"));
    let pool = McpPool::default();
    let client_pool = test_client_pool();
    let outcome = dispatch_action(
        &action,
        Path::new("."),
        &decl,
        &echo_servers(),
        &pool,
        &client_pool,
        Duration::from_secs(5),
    )
    .await
    .expect("success");
    match outcome {
        ActionOutcome::Completed { result_json, .. } => {
            assert_eq!(result_json["server"].as_str(), Some("echo"));
            assert_eq!(result_json["tool"].as_str(), Some("echo"));
            let content = result_json["content"]
                .as_array()
                .expect("content array");
            assert_eq!(content.len(), 1);
            // Substitution happened before the args reached the server.
            assert_eq!(content[0]["text"].as_str(), Some("hi item-7"));
        }
        other => panic!("expected Completed, got {other:?}"),
    }
}

#[tokio::test]
async fn dispatch_action_tool_error_returns_provider() {
    let Some(_) = python3() else {
        eprintln!("WARN: skipping tool-error test — python3 not in PATH");
        return;
    };
    let action = echo_action("tool_error", json!({}));
    let decl = tagged_decl(Some("echo"), Some("tool_error"));
    let pool = McpPool::default();
    let client_pool = test_client_pool();
    let err = dispatch_action(
        &action,
        Path::new("."),
        &decl,
        &echo_servers(),
        &pool,
        &client_pool,
        Duration::from_secs(5),
    )
    .await
    .expect_err("tool_error returns is_error=true");
    let msg = err.to_string();
    assert!(msg.contains("returned error"), "got {msg}");
    assert!(msg.contains("tool refused"), "got {msg}");
}

#[tokio::test]
async fn dispatch_action_timeout_returns_provider() {
    let Some(_) = python3() else {
        eprintln!("WARN: skipping timeout test — python3 not in PATH");
        return;
    };
    let action = echo_action("slow", json!({"ms": 1500}));
    let decl = tagged_decl(Some("echo"), Some("slow"));
    let pool = McpPool::default();
    let client_pool = test_client_pool();
    let err = dispatch_action(
        &action,
        Path::new("."),
        &decl,
        &echo_servers(),
        &pool,
        &client_pool,
        // Tight enough that `slow ms=1500` blows past the budget,
        // generous enough to absorb python3 spawn jitter on slow CI.
        Duration::from_millis(300),
    )
    .await
    .expect_err("timeout");
    assert!(
        err.to_string().contains("exceeded wallclock budget"),
        "got {err}"
    );
}
