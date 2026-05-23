//! Claim-5 LIVE regression — MCP half, pool-routing assertions.
//!
//! Mission claim 5 (verbatim from MISSION.md): *"Any task a human
//! can do on a PC. TWO execution backends: MCP (well-known APIs)
//! AND computer-use (Anthropic-style desktop control)."*
//!
//! This file covers the **pool-routing** half of the MCP-side
//! claim-5 LIVE assertion. Cap-check + audit-row assertions live in
//! `claim_5_live_mcp_cap_audit_test.rs`. The cu (computer-use) half
//! lives in `crates/argo-executor-computer-use-macos/tests/`.
//!
//! Both files + the cu file green = claim 5 LIVE.
//!
//! ## What this asserts
//!
//! 1. `mcp_fake_server_call_is_keyed_by_tenant_and_server` — pool
//!    routing semantics: the dispatcher receives `(tenant_id,
//!    server, tool)` verbatim from the runner.
//! 2. `mcp_fake_server_two_tenants_route_independently` — two
//!    tenants invoking the same `(server, tool)` produce two
//!    distinct dispatcher calls keyed by their tenant_ids (the
//!    pool-isolation contract — what the production
//!    `argo_mcp::McpClientPool` inherits structurally).

mod common;

use argo_types::{Principal, TenantId};
use argo_workflow::{ExecutionRequest, Executor, TaskVerdict, WorkerSessionRunner};

use crate::common::{
    one_step_workflow, CapturingAudit, FakeMcpServerPool, TableCapabilities,
};

/// Assertion 1: pool routing — `(tenant_id, server, tool)` flows
/// verbatim from runner to dispatcher.
#[tokio::test]
async fn mcp_fake_server_call_is_keyed_by_tenant_and_server() {
    let tenant_id = TenantId::new();
    let principal = Principal::Local {
        os_user: "claim5-live-pool-routing".into(),
    };
    let pool = FakeMcpServerPool::new();
    pool.route_ok(
        tenant_id,
        "slack-mcp",
        "post_message",
        serde_json::json!({"ts": "1234567890.000100"}),
    );
    let audit = CapturingAudit::new();
    let caps = TableCapabilities::granting([(principal.clone(), "post_message".into())]);
    let runner = WorkerSessionRunner::new(
        pool.clone(),
        audit.clone(),
        caps,
        principal,
        tenant_id,
    );
    let exec = Executor::new(runner);
    let report = exec
        .execute(ExecutionRequest::new(one_step_workflow(
            "slack-mcp",
            "post_message",
            serde_json::json!({"channel": "C01", "text": "hi"}),
        )))
        .await
        .expect("execute");

    assert!(report.ok, "fake MCP success should produce ok report");
    let calls = pool.calls();
    assert_eq!(calls.len(), 1, "exactly one pool dispatch expected");
    assert_eq!(calls[0].key.tenant_id, tenant_id);
    assert_eq!(calls[0].key.server, "slack-mcp");
    assert_eq!(calls[0].key.tool, "post_message");
    assert_eq!(
        calls[0].args,
        serde_json::json!({"channel": "C01", "text": "hi"})
    );
}

/// Assertion 2: two tenants invoking the same `(server, tool)` go
/// to distinct routing keys — the per-tenant pool isolation contract.
#[tokio::test]
async fn mcp_fake_server_two_tenants_route_independently() {
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    assert_ne!(tenant_a, tenant_b);

    let principal_a = Principal::Local {
        os_user: "claim5-live-tenant-a".into(),
    };
    let principal_b = Principal::Local {
        os_user: "claim5-live-tenant-b".into(),
    };

    let pool = FakeMcpServerPool::new();
    pool.route_ok(
        tenant_a,
        "shared-manifest",
        "ping",
        serde_json::json!({"pong": "from-tenant-a"}),
    );
    pool.route_ok(
        tenant_b,
        "shared-manifest",
        "ping",
        serde_json::json!({"pong": "from-tenant-b"}),
    );

    let audit_a = CapturingAudit::new();
    let audit_b = CapturingAudit::new();
    let caps_a = TableCapabilities::granting([(principal_a.clone(), "ping".into())]);
    let caps_b = TableCapabilities::granting([(principal_b.clone(), "ping".into())]);

    let runner_a = WorkerSessionRunner::new(
        pool.clone(),
        audit_a.clone(),
        caps_a,
        principal_a,
        tenant_a,
    );
    let runner_b = WorkerSessionRunner::new(
        pool.clone(),
        audit_b.clone(),
        caps_b,
        principal_b,
        tenant_b,
    );

    let report_a = Executor::new(runner_a)
        .execute(ExecutionRequest::new(one_step_workflow(
            "shared-manifest",
            "ping",
            serde_json::json!({}),
        )))
        .await
        .expect("execute tenant_a");
    let report_b = Executor::new(runner_b)
        .execute(ExecutionRequest::new(one_step_workflow(
            "shared-manifest",
            "ping",
            serde_json::json!({}),
        )))
        .await
        .expect("execute tenant_b");

    assert!(report_a.ok);
    assert!(report_b.ok);

    // The pool saw two calls, one per tenant, against the same
    // (server, tool) — the load-bearing per-tenant isolation
    // contract.
    let calls = pool.calls();
    assert_eq!(calls.len(), 2, "two tenants ⇒ two distinct dispatches");
    let tenants: std::collections::HashSet<TenantId> =
        calls.iter().map(|c| c.key.tenant_id).collect();
    assert!(tenants.contains(&tenant_a));
    assert!(tenants.contains(&tenant_b));
    assert_eq!(tenants.len(), 2, "pool key must split by tenant");

    // Each tenant got back ONLY its own per-tenant response.
    match report_a
        .task_verdicts
        .get("call")
        .expect("tenant_a verdict")
    {
        TaskVerdict::Success { outputs } => assert_eq!(
            outputs.get("response"),
            Some(&serde_json::json!({"pong": "from-tenant-a"})),
        ),
        other => panic!("expected Success, got {other:?}"),
    }
    match report_b
        .task_verdicts
        .get("call")
        .expect("tenant_b verdict")
    {
        TaskVerdict::Success { outputs } => assert_eq!(
            outputs.get("response"),
            Some(&serde_json::json!({"pong": "from-tenant-b"})),
        ),
        other => panic!("expected Success, got {other:?}"),
    }

    // Audit rows stayed in their tenant lanes.
    let events_a = audit_a.events();
    let events_b = audit_b.events();
    assert_eq!(events_a.len(), 1);
    assert_eq!(events_b.len(), 1);
    assert_eq!(events_a[0].tenant_id, tenant_a);
    assert_eq!(events_b[0].tenant_id, tenant_b);
}
