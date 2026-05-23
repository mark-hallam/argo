//! Claim-5 LIVE regression — MCP half, capability + audit assertions.
//!
//! Mission claim 5 (verbatim from MISSION.md): *"Any task a human
//! can do on a PC. TWO execution backends: MCP (well-known APIs)
//! AND computer-use (Anthropic-style desktop control)."*
//!
//! This file covers the **capability + audit + failure-surfacing**
//! half of the MCP-side claim-5 LIVE assertion. Pool-routing
//! assertions live in `claim_5_live_mcp_pool_routing_test.rs`. The
//! cu (computer-use) half lives in
//! `crates/argo-executor-computer-use-macos/tests/`.
//!
//! ## What this asserts
//!
//! 3. `mcp_fake_server_cap_check_fires_before_dispatch` — a
//!    foreign principal is refused at the capability gate, the
//!    fake server is never invoked, audit row is step_failed.
//! 4. `mcp_fake_server_audit_row_carries_tenant_and_principal` —
//!    a successful dispatch produces exactly one
//!    `workflow.step.executed` row with `tenant_id` + principal
//!    label intact.
//! 5. `mcp_fake_server_dispatch_error_is_surfaced_not_swallowed` —
//!    a fake MCP `isError=true` response (modeled as
//!    `ToolDispatchError::ToolError`) produces a Failed verdict
//!    AND a `workflow.step.failed` audit row.
//! 6. `mcp_fake_server_transport_500_takes_same_failure_path` —
//!    transport-flavoured failures take the same forensic path
//!    as tool errors (negative-path parity).

mod common;

use argo_types::{Principal, TenantId};
use argo_workflow::{ExecutionRequest, Executor, TaskVerdict, WorkerSessionRunner};

use crate::common::{
    one_step_workflow, CapturingAudit, FakeMcpServerPool, TableCapabilities,
};

/// Assertion 3: cap-check fires BEFORE dispatch. The fake MCP
/// server is the load-bearing witness — if the runner skipped the
/// capability gate, the fake server would record a call.
#[tokio::test]
async fn mcp_fake_server_cap_check_fires_before_dispatch() {
    let tenant_id = TenantId::new();
    let principal_with_grant = Principal::Local {
        os_user: "alice-has-grant".into(),
    };
    let principal_without_grant = Principal::Local {
        os_user: "bob-no-grant".into(),
    };

    let pool = FakeMcpServerPool::new();
    pool.route_ok(
        tenant_id,
        "github-mcp",
        "create_issue",
        serde_json::json!({"number": 9999}),
    );

    // Cap table grants `create_issue` to alice only.
    let caps = TableCapabilities::granting([(
        principal_with_grant.clone(),
        "create_issue".into(),
    )]);
    let audit = CapturingAudit::new();

    let runner = WorkerSessionRunner::new(
        pool.clone(),
        audit.clone(),
        caps,
        principal_without_grant.clone(),
        tenant_id,
    );
    let exec = Executor::new(runner);
    let report = exec
        .execute(ExecutionRequest::new(one_step_workflow(
            "github-mcp",
            "create_issue",
            serde_json::json!({"title": "fake bug"}),
        )))
        .await
        .expect("execute");

    // Load-bearing: fake server MUST NOT have been touched.
    assert!(
        pool.calls().is_empty(),
        "cap-check must refuse before MCP dispatch; got calls={:?}",
        pool.calls(),
    );

    match report
        .task_verdicts
        .get("call")
        .expect("verdict")
    {
        TaskVerdict::Failed { reason } => assert!(
            reason.starts_with("capability denied"),
            "expected capability-denied reason, got {reason:?}"
        ),
        other => panic!("expected Failed, got {other:?}"),
    }
    assert!(!report.ok);

    // Audit row: exactly one step_failed carrying the principal.
    let events = audit.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, "workflow.step.failed");
    assert_eq!(events[0].tool, "create_issue");
    assert_eq!(events[0].tenant_id, tenant_id);
    assert_eq!(events[0].principal_label, "local:bob-no-grant");
}

/// Assertion 4: a successful dispatch lands exactly one
/// `workflow.step.executed` audit row carrying the full context.
#[tokio::test]
async fn mcp_fake_server_audit_row_carries_tenant_and_principal() {
    let tenant_id = TenantId::new();
    let principal = Principal::Gateway {
        adapter: "web".into(),
        identity: "operator-9".into(),
    };

    let pool = FakeMcpServerPool::new();
    pool.route_ok(
        tenant_id,
        "linear-mcp",
        "create_ticket",
        serde_json::json!({"id": "LIN-42", "url": "https://linear.app/lin-42"}),
    );
    let audit = CapturingAudit::new();
    let caps = TableCapabilities::granting([(principal.clone(), "create_ticket".into())]);

    let runner = WorkerSessionRunner::new(
        pool.clone(),
        audit.clone(),
        caps,
        principal.clone(),
        tenant_id,
    );
    let exec = Executor::new(runner);
    let report = exec
        .execute(ExecutionRequest::new(one_step_workflow(
            "linear-mcp",
            "create_ticket",
            serde_json::json!({"title": "Claim-5 LIVE proof"}),
        )))
        .await
        .expect("execute");
    assert!(report.ok);

    let events = audit.events();
    assert_eq!(
        events.len(),
        1,
        "exactly one audit row expected on the happy path"
    );
    let row = &events[0];
    assert_eq!(row.kind, "workflow.step.executed");
    assert_eq!(row.task_id, "call");
    assert_eq!(row.worker, "linear-mcp");
    assert_eq!(row.tool, "create_ticket");
    assert_eq!(row.tenant_id, tenant_id);
    assert_eq!(row.principal_label, "web:operator-9");
    assert_eq!(
        row.reason_or_response,
        serde_json::json!({"id": "LIN-42", "url": "https://linear.app/lin-42"})
    );
}

/// Assertion 5: fake-MCP-server tool error surfaces as a Failed
/// verdict + a `workflow.step.failed` audit row. Failures are
/// never silently swallowed (the forensic invariant).
#[tokio::test]
async fn mcp_fake_server_dispatch_error_is_surfaced_not_swallowed() {
    let tenant_id = TenantId::new();
    let principal = Principal::Local {
        os_user: "claim5-live-failpath".into(),
    };

    let pool = FakeMcpServerPool::new();
    // Tool-level error (mimics upstream returning `isError = true`).
    pool.route_tool_error(
        tenant_id,
        "jira-mcp",
        "transition_issue",
        "JIRA-API: transition rejected: invalid status id",
    );
    let audit = CapturingAudit::new();
    let caps = TableCapabilities::granting([(principal.clone(), "transition_issue".into())]);
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
            "jira-mcp",
            "transition_issue",
            serde_json::json!({"id": "ARGO-1", "to": "Done"}),
        )))
        .await
        .expect("execute");

    assert_eq!(pool.calls().len(), 1, "dispatcher must have been called");
    match report
        .task_verdicts
        .get("call")
        .expect("verdict")
    {
        TaskVerdict::Failed { reason } => assert!(
            reason.contains("JIRA-API: transition rejected"),
            "expected upstream message in reason, got {reason:?}",
        ),
        other => panic!("expected Failed verdict, got {other:?}"),
    }
    assert!(!report.ok, "run ok must be false on MCP tool error");

    let events = audit.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, "workflow.step.failed");
    let reason_str = events[0]
        .reason_or_response
        .as_str()
        .expect("reason string");
    assert!(
        reason_str.contains("JIRA-API: transition rejected"),
        "audit reason must carry the upstream message, got {reason_str:?}",
    );
}

/// Assertion 6: transport-flavoured failures (mock MCP returns 500)
/// take the same forensic path as tool errors. Negative-path
/// parity — neither flavour is silently swallowed.
#[tokio::test]
async fn mcp_fake_server_transport_500_takes_same_failure_path() {
    let tenant_id = TenantId::new();
    let principal = Principal::Local {
        os_user: "claim5-live-transport-fail".into(),
    };
    let pool = FakeMcpServerPool::new();
    pool.route_transport_error(
        tenant_id,
        "stripe-mcp",
        "create_charge",
        "stripe API returned 502",
    );
    let caps = TableCapabilities::granting([(principal.clone(), "create_charge".into())]);
    let audit = CapturingAudit::new();
    let runner = WorkerSessionRunner::new(
        pool.clone(),
        audit.clone(),
        caps,
        principal,
        tenant_id,
    );
    let report = Executor::new(runner)
        .execute(ExecutionRequest::new(one_step_workflow(
            "stripe-mcp",
            "create_charge",
            serde_json::json!({"amount_cents": 100}),
        )))
        .await
        .expect("execute");

    assert!(!report.ok);
    assert_eq!(pool.calls().len(), 1, "transport error fires AFTER dispatch");
    match report
        .task_verdicts
        .get("call")
        .expect("verdict")
    {
        TaskVerdict::Failed { reason } => assert!(
            reason.contains("stripe API returned 502"),
            "expected upstream transport message, got {reason:?}",
        ),
        other => panic!("expected Failed verdict, got {other:?}"),
    }

    let events = audit.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, "workflow.step.failed");
    assert_eq!(events[0].tenant_id, tenant_id);
}
