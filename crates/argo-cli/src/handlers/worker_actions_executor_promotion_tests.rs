//! Coverage for the dormant per-action execution-graduation resolve
//! seam (UF Phase-B · B3, DEC-037).
//!
//! Acceptance mapping:
//! - "a non-promoted action still produces a human-handoff bundle" →
//!   [`human_resolved_executor_produces_handoff_bundle`] +
//!   [`enabled_without_promotion_resolves_human`].
//! - "a promoted MCP action performs a real side-effect against a MOCK
//!   MCP server" → [`promoted_action_performs_real_mcp_side_effect`]
//!   (drives the Python stdio echo fixture).
//! - dormancy (default-OFF) → [`disabled_resolves_human_even_with_promotion`].
//! - tenant isolation → [`promotion_is_tenant_scoped_at_resolve`].

use super::*;
use crate::handlers::worker_actions::ActionRecord;
use crate::handlers::worker_actions_executor::{ActionOutcome, ExecutorKind};
use argo_identity::WorkerId;
use argo_store::executor_promotion::{promote, PromotionGrant};
use argo_store::SessionStore;
use argo_types::TenantId;
use serde_json::json;
use std::path::PathBuf;

fn python3() -> Option<String> {
    let out = std::process::Command::new("which")
        .arg("python3")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!s.is_empty()).then_some(s)
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("mcp_echo_server.py")
}

/// Manifest declaring `send_email` as `executor = "human"` (draft-only
/// at birth) plus an `echo` stdio MCP server, so a promotion can later
/// graduate the action onto the echo tool.
fn echo_manifest(python: &str) -> argo_vertical::VerticalManifest {
    let script = fixture_path();
    let toml = format!(
        r#"schema_version = "1"

[vertical]
id = "grad-echo"
name = "graduation echo"
version = "0.1.0"

[mcp_servers]
echo = {{ transport = "stdio", command = "{python}", args = ["{script}"] }}

[actions.send_email]
risk = "low"
requires_approval = false
description = "Send the drafted email"
executor = "human"
"#,
        python = python,
        script = script.to_string_lossy(),
    );
    argo_vertical::parse(&toml).expect("echo manifest parses")
}

fn mcp_grant(tenant: &TenantId, worker_id: &str) -> PromotionGrant {
    PromotionGrant {
        tenant_id: tenant.to_string(),
        worker_id: worker_id.into(),
        action_type: "send_email".into(),
        executor_kind: "mcp".into(),
        mcp_server: Some("echo".into()),
        mcp_tool: Some("echo".into()),
        promoted_by: "principal:owner".into(),
    }
}

fn echo_action() -> ActionRecord {
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
        payload: json!({ "text": "hi {{ item_id }}" }),
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

async fn store() -> SessionStore {
    let s = SessionStore::open(":memory:").await.expect("open store");
    s.migrate().await.expect("migrate");
    s
}

#[test]
fn graduation_flag_parses_truthy_and_falsy() {
    assert!(graduation_flag(Some("1")));
    assert!(graduation_flag(Some("true")));
    assert!(graduation_flag(Some("TRUE")));
    assert!(graduation_flag(Some("yes")));
    // Aligned with parse_autofire_flag: case-insensitive, trimmed, +on.
    assert!(graduation_flag(Some("on")));
    assert!(graduation_flag(Some("ON")));
    assert!(graduation_flag(Some("On")));
    assert!(graduation_flag(Some("YES")));
    assert!(graduation_flag(Some(" true ")), "trimmed");
    assert!(!graduation_flag(Some("0")));
    assert!(!graduation_flag(Some("off")));
    assert!(!graduation_flag(Some("")));
    assert!(!graduation_flag(Some("enabled")), "only the canonical truthy set");
    assert!(!graduation_flag(None), "unset flag is dormant");
}

#[tokio::test]
async fn disabled_resolves_human_even_with_promotion() {
    // A promotion exists, but graduation is OFF → the manifest default
    // (human) is returned and the table is never consulted. This is the
    // shipped default behaviour.
    let s = store().await;
    let tenant = TenantId::new();
    let worker = WorkerId::new().to_string();
    promote(s.pool(), &mcp_grant(&tenant, &worker))
        .await
        .expect("promote");
    let manifest = echo_manifest("python3");
    let exec = resolve_with_graduation(&s, tenant, &worker, &manifest, "send_email", false)
        .await
        .expect("resolve");
    assert_eq!(
        exec.kind(),
        ExecutorKind::Human,
        "OFF must stay draft-only even with a grant present"
    );
}

#[tokio::test]
async fn enabled_without_promotion_resolves_human() {
    // Graduation ON but no grant for this action → still human.
    let s = store().await;
    let tenant = TenantId::new();
    let worker = WorkerId::new().to_string();
    let manifest = echo_manifest("python3");
    let exec = resolve_with_graduation(&s, tenant, &worker, &manifest, "send_email", true)
        .await
        .expect("resolve");
    assert_eq!(exec.kind(), ExecutorKind::Human);
}

#[tokio::test]
async fn enabled_with_promotion_resolves_mcp() {
    let s = store().await;
    let tenant = TenantId::new();
    let worker = WorkerId::new().to_string();
    promote(s.pool(), &mcp_grant(&tenant, &worker))
        .await
        .expect("promote");
    let manifest = echo_manifest("python3");
    let exec = resolve_with_graduation(&s, tenant, &worker, &manifest, "send_email", true)
        .await
        .expect("resolve");
    assert_eq!(
        exec.kind(),
        ExecutorKind::Mcp,
        "ON + grant graduates the action onto the MCP executor"
    );
}

#[tokio::test]
async fn promotion_is_tenant_scoped_at_resolve() {
    let s = store().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let worker = WorkerId::new().to_string();
    promote(s.pool(), &mcp_grant(&tenant_a, &worker))
        .await
        .expect("promote in tenant-a");
    let manifest = echo_manifest("python3");
    // Resolving the SAME worker/action under a different tenant must
    // not see tenant-a's grant.
    let exec = resolve_with_graduation(&s, tenant_b, &worker, &manifest, "send_email", true)
        .await
        .expect("resolve");
    assert_eq!(
        exec.kind(),
        ExecutorKind::Human,
        "a promotion must never resolve across a tenant boundary"
    );
}

#[tokio::test]
async fn human_resolved_executor_produces_handoff_bundle() {
    let s = store().await;
    let tenant = TenantId::new();
    let worker = WorkerId::new().to_string();
    let manifest = echo_manifest("python3");
    let exec = resolve_with_graduation(&s, tenant, &worker, &manifest, "send_email", true)
        .await
        .expect("resolve");
    let cwd = tempfile::tempdir().expect("tempdir");
    let outcome = exec.dispatch(&echo_action(), cwd.path()).expect("dispatch");
    assert!(
        matches!(outcome, ActionOutcome::HumanHandoff { .. }),
        "a non-promoted action hands off to a human, got {outcome:?}"
    );
}

/// The acceptance test: a promoted MCP action carries out a **real**
/// tool call against the mock (Python stdio echo) MCP server and
/// completes. Multi-threaded runtime because the executor's sync
/// `dispatch` shim uses `block_in_place` to bridge to the async MCP
/// call. Skips with a WARN when python3 is absent (minimal containers).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn promoted_action_performs_real_mcp_side_effect() {
    let Some(py) = python3() else {
        eprintln!("WARN: skipping promoted MCP side-effect test — python3 not in PATH");
        return;
    };
    let manifest = echo_manifest(&py);
    let tenant = TenantId::new();
    let worker = WorkerId::new().to_string();
    let grant = mcp_grant(&tenant, &worker);
    // Build the executor exactly as the resolve seam would for an
    // ON + promoted action, then dispatch it for real.
    let promo = argo_store::executor_promotion::PromotionRecord {
        tenant_id: grant.tenant_id.clone(),
        worker_id: grant.worker_id.clone(),
        action_type: grant.action_type.clone(),
        executor_kind: grant.executor_kind.clone(),
        mcp_server: grant.mcp_server.clone(),
        mcp_tool: grant.mcp_tool.clone(),
        promoted_by: grant.promoted_by.clone(),
        promoted_at: "2026-06-06T00:00:00+00:00".into(),
    };
    let base = lookup_action_decl(&manifest, "send_email").expect("base decl");
    let servers = manifest_servers(&manifest);
    let exec = promoted_mcp_executor(&promo, base, servers);
    assert_eq!(exec.kind(), ExecutorKind::Mcp);

    let cwd = tempfile::tempdir().expect("tempdir");
    let outcome = exec
        .dispatch(&echo_action(), cwd.path())
        .expect("real MCP dispatch against echo fixture");
    match outcome {
        ActionOutcome::Completed { result_json, .. } => {
            assert_eq!(result_json["executor"].as_str(), Some("mcp"));
            assert_eq!(result_json["server"].as_str(), Some("echo"));
            assert_eq!(result_json["tool"].as_str(), Some("echo"));
            // Template substitution ran before the args reached the
            // server: `{{ item_id }}` → `item-7`.
            let content = result_json["content"].as_array().expect("content");
            assert_eq!(content[0]["text"].as_str(), Some("hi item-7"));
        }
        other => panic!("expected Completed from the echo server, got {other:?}"),
    }
}
