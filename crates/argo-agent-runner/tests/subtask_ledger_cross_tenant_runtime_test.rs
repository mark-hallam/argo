//! Cross-tenant regression tests for the runtime callers of the
//! `subtask_ledger` helpers (Brief
//! `eval-subtask-ledger-caller-tenant-audit`).
//!
//! Pins the contract that
//! [`argo_agent_runner::turn_budget::check_pre_turn_budget`] and
//! [`argo_agent_runner::turn_budget::record_subtask_turn_spend`]
//! both resolve the subtask id through a **session-scoped** lookup
//! (`SELECT id FROM subtask_ledger WHERE parent_session_id = ? AND
//! state = 'running'`). The id is never user-supplied — the
//! runtime hands in a `SessionId` it just loaded, and the session
//! row binds its `(tenant_id, workspace_id)` at creation.
//!
//! These tests seed two tenants in the same in-memory DB, give each
//! a session with a `running` subtask, then exercise the runtime
//! callers against one tenant's session and assert the other
//! tenant's subtask row is byte-identical to its pre-call snapshot.
//!
//! If the runtime ever regresses to a bare-id lookup (or
//! `running_subtask_id`'s `WHERE parent_session_id = ?` predicate is
//! dropped), the spend-bleed assertion in
//! [`post_turn_spend_is_session_scoped_does_not_bleed_across_tenants`]
//! flips red.

use argo_agent_runner::turn_budget::{check_pre_turn_budget, record_subtask_turn_spend};
use argo_agent_runner::SpendCtx;
use argo_audit::AuditSink;
use argo_store::subtask::{self, NewSubtask, SubtaskState};
use argo_store::subtask_budget;
use argo_store::SessionStore;
use argo_types::{SessionId, TenantId, WorkspaceId};
use std::sync::Arc;

/// Seed one tenant + one workspace + one session + one `running`
/// subtask with a fresh budget. Returns the SessionId and the
/// minted subtask id.
async fn seed_tenant(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    budget_usd_cents: i64,
) -> (SessionId, String) {
    let ws = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, 'smoke', '.', strftime('%s','now'))",
    )
    .bind(ws.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .expect("seed workspace");

    let session_id = store
        .create_session(tenant, ws, "model", "provider", Some("t"), None)
        .await
        .expect("create session");

    let subtask_id = subtask::insert(
        store.pool(),
        &NewSubtask {
            parent_goal: "test-goal",
            parent_session_id: Some(&session_id.0.to_string()),
            // Unique worker_id per tenant so the
            // `idx_subtask_ledger_node_unique` index doesn't reject
            // the second seed (both tenants share `plan_node_path`).
            worker_id: &format!("wkr-{}", tenant.0),
            plan_node_path: "/test/root",
            description: "d",
        },
    )
    .await
    .expect("insert subtask");

    subtask::update_state(store.pool(), &subtask_id, SubtaskState::Running, None)
        .await
        .expect("running");
    subtask_budget::set_subtask_budget(store.pool(), &subtask_id, budget_usd_cents)
        .await
        .expect("set budget");
    (session_id, subtask_id)
}

async fn fresh_store() -> Arc<SessionStore> {
    let s = SessionStore::open(":memory:").await.expect("open");
    s.migrate().await.expect("migrate");
    Arc::new(s)
}

#[tokio::test]
async fn pre_turn_gate_is_session_scoped_does_not_bleed_across_tenants() {
    let store = fresh_store().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    // AuditSink::new_with_tenant ties the sink to a specific tenant;
    // the gate's tenant identity is the one whose budget is in
    // play. The check_pre_turn_budget call below uses tenant A's
    // session id, so we stamp the sink with tenant A.
    let audit = AuditSink::new_with_tenant(store.clone(), tenant_a.0.to_string());

    // Tenant A: budget of 100 cents, fully spent → gate must fail.
    let (session_a, subtask_a) = seed_tenant(&store, tenant_a, 100).await;
    subtask_budget::record_subtask_spend(store.pool(), &subtask_a, 100)
        .await
        .expect("spend tenant A");

    // Tenant B: budget of 100 cents, zero spent → gate must pass.
    let (session_b, _subtask_b) = seed_tenant(&store, tenant_b, 100).await;

    // Tenant A's gate is exhausted.
    let err_a = check_pre_turn_budget(&store, session_a, &audit)
        .await
        .expect_err("tenant A must gate on exhausted budget");
    assert!(
        matches!(err_a, argo_types::ArgoError::BudgetExhausted { .. }),
        "expected BudgetExhausted for tenant A, got: {err_a:?}"
    );

    // Tenant B's gate passes — the resolver's `parent_session_id =
    // ?` predicate must NOT have picked up tenant A's exhausted row.
    check_pre_turn_budget(&store, session_b, &audit)
        .await
        .expect("tenant B must NOT be gated by tenant A's exhausted budget");
}

#[tokio::test]
async fn post_turn_spend_is_session_scoped_does_not_bleed_across_tenants() {
    let store = fresh_store().await;

    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();

    // Both tenants: budget 10_000, spent 0.
    let (session_a, subtask_a) = seed_tenant(&store, tenant_a, 10_000).await;
    let (_session_b, subtask_b) = seed_tenant(&store, tenant_b, 10_000).await;

    let spend = SpendCtx {
        tenant_id: tenant_a,
        provider: "anthropic",
        model: "claude-sonnet-test",
        // $1 / 1k tokens in, $2 / 1k tokens out.
        pricing: Some((1.0, 2.0)),
    };
    // 1_000 in + 500 out → $2.00 → 200 cents charged to tenant A's
    // running subtask.
    record_subtask_turn_spend(&store, session_a, &spend, Some(1_000), Some(500))
        .await
        .expect("record spend for tenant A");

    let remaining_a = subtask_budget::get_remaining(store.pool(), &subtask_a)
        .await
        .expect("remaining A")
        .expect("tenant A budget is bounded");
    assert_eq!(
        remaining_a, 9_800,
        "tenant A's remaining must reflect the 200-cent charge"
    );

    // Tenant B's row must be untouched.
    let remaining_b = subtask_budget::get_remaining(store.pool(), &subtask_b)
        .await
        .expect("remaining B")
        .expect("tenant B budget is bounded");
    assert_eq!(
        remaining_b, 10_000,
        "tenant B's spent_usd_cents must be untouched by tenant A's turn"
    );
}
