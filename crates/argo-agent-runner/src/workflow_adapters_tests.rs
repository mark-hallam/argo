//! Unit tests for the production workflow-runner adapters.
//!
//! These exercise the adapter logic that does NOT require a live MCP
//! subprocess: the capability allow/deny gate, the audit-row kind mapping
//! (against a real in-memory [`argo_store::SessionStore`]-backed
//! [`AuditSink`]), and the dispatcher's pre-connect error paths
//! (unknown server / unsupported transport). The happy-path MCP dispatch
//! is covered end-to-end by
//! `argo-workflow/tests/execution_runs_real_step_test.rs` against stub
//! seams; here we pin the *production* wiring around it.

use super::*;
use argo_store::SessionStore;

fn local_principal(user: &str) -> Principal {
    Principal::Local {
        os_user: user.to_string(),
    }
}

fn ctx<'a>(
    tenant: &'a TenantId,
    task_id: &'a str,
    worker: &'a str,
    tool: &'a str,
    principal: &'a Principal,
) -> StepAuditContext<'a> {
    StepAuditContext {
        tenant_id: tenant,
        task_id,
        worker,
        tool,
        principal,
    }
}

// ---- CapabilityWorkflowChecker -------------------------------------

#[test]
fn capability_checker_allows_bound_owner() {
    let engine = CapabilityEngine::with_defaults();
    let principal = local_principal("alice");
    engine
        .bind(principal.clone(), "owner".into())
        .expect("bind owner");
    let checker = CapabilityWorkflowChecker::new(Arc::new(engine));
    // owner carries `ToolDispatch "*"`, so any tool is granted.
    assert!(checker.check_dispatch(&principal, "send_email").is_ok());
}

#[test]
fn capability_checker_denies_unbound_principal() {
    let engine = CapabilityEngine::with_defaults();
    let checker = CapabilityWorkflowChecker::new(Arc::new(engine));
    // Never bound → no effective capabilities → denied.
    let err = checker
        .check_dispatch(&local_principal("mallory"), "drop_table")
        .expect_err("unbound principal must be denied");
    assert!(!err.is_empty(), "denial must carry a reason");
}

#[test]
fn capability_checker_denies_tool_outside_role_grant() {
    let engine = CapabilityEngine::with_defaults();
    let principal = local_principal("guesty");
    // guest is the most restricted preset; it does not carry a
    // `ToolDispatch "*"` grant, so an arbitrary tool is refused.
    engine
        .bind(principal.clone(), "guest".into())
        .expect("bind guest");
    let checker = CapabilityWorkflowChecker::new(Arc::new(engine));
    assert!(checker.check_dispatch(&principal, "wire_funds").is_err());
}

// ---- AuditWorkflowEmitter ------------------------------------------

async fn in_memory_sink(tenant: &TenantId) -> AuditSink {
    let store = SessionStore::open(":memory:")
        .await
        .expect("open in-memory store");
    // `open(":memory:")` does NOT run migrations — the audit_record table
    // only exists after migrate() (matches the audit crate's own test setup).
    store.migrate().await.expect("migrate in-memory store");
    AuditSink::new_with_tenant(Arc::new(store), tenant.to_string())
}

#[tokio::test]
async fn audit_emitter_writes_executed_row_and_returns_ok() {
    let tenant = TenantId::new();
    let sink = in_memory_sink(&tenant).await;
    let emitter = AuditWorkflowEmitter::new(sink);
    let principal = local_principal("alice");
    let c = ctx(&tenant, "task-1", "notes", "append", &principal);
    let res = emitter
        .step_executed(&c, &json!({"ok": true}))
        .await;
    assert!(res.is_ok(), "executed-row write should succeed: {res:?}");
}

#[tokio::test]
async fn audit_emitter_step_failed_is_best_effort_and_does_not_panic() {
    let tenant = TenantId::new();
    let sink = in_memory_sink(&tenant).await;
    let emitter = AuditWorkflowEmitter::new(sink);
    let principal = local_principal("alice");
    let c = ctx(&tenant, "task-2", "notes", "append", &principal);
    // Best-effort: returns unit, must not panic even if it wrote.
    emitter.step_failed(&c, "capability denied: nope").await;
}

#[test]
fn step_payload_carries_identity_and_keyed_value() {
    let tenant = TenantId::new();
    let principal = local_principal("alice");
    let c = ctx(&tenant, "task-3", "stripe", "refund", &principal);
    let p = step_payload(&c, "reason", Value::String("boom".into()));
    assert_eq!(p["task_id"], "task-3");
    assert_eq!(p["worker"], "stripe");
    assert_eq!(p["tool"], "refund");
    assert_eq!(p["reason"], "boom");
    assert_eq!(p["tenant_id"], tenant.to_string());
}

// ---- McpToolDispatcher (no-subprocess error paths) -----------------

fn dispatcher_with(servers: BTreeMap<String, Value>) -> McpToolDispatcher {
    McpToolDispatcher::new(
        McpClientPool::new(argo_mcp::PoolConfig::default()),
        servers,
    )
}

#[tokio::test]
async fn dispatcher_rejects_unknown_server_before_connecting() {
    let dispatcher = dispatcher_with(BTreeMap::new());
    let err = dispatcher
        .call(&TenantId::new(), "ghost", "echo", json!({}))
        .await
        .expect_err("unknown server must be rejected");
    match err {
        ToolDispatchError::BadRequest(m) => assert!(m.contains("ghost"), "got {m}"),
        other => panic!("expected BadRequest, got {other:?}"),
    }
}

#[tokio::test]
async fn dispatcher_rejects_unsupported_transport() {
    let mut servers = BTreeMap::new();
    servers.insert(
        "weird".to_string(),
        json!({ "transport": "http", "command": "x" }),
    );
    let dispatcher = dispatcher_with(servers);
    let err = dispatcher
        .call(&TenantId::new(), "weird", "echo", json!({}))
        .await
        .expect_err("non-stdio transport must be rejected");
    match err {
        ToolDispatchError::BadRequest(m) => assert!(m.contains("http"), "got {m}"),
        other => panic!("expected BadRequest, got {other:?}"),
    }
}

#[test]
fn parse_stdio_transport_extracts_command_and_args() {
    let t = json!({ "transport": "stdio", "command": "notes-mcp", "args": ["--flag", "v"] });
    let (cmd, args) = parse_stdio_transport(&t).expect("parses");
    assert_eq!(cmd, "notes-mcp");
    assert_eq!(args, vec!["--flag".to_string(), "v".to_string()]);
}

#[test]
fn principal_label_renders_each_variant() {
    assert_eq!(principal_label(&local_principal("bob")), "local:bob");
    assert_eq!(
        principal_label(&Principal::Gateway {
            adapter: "slack".into(),
            identity: "u1".into()
        }),
        "slack:u1"
    );
    assert_eq!(principal_label(&Principal::Forge), "forge");
}
