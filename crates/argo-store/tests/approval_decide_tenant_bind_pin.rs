//! FINDINGS-ONLY pin test for `eval-argo-store-tenant-bind-sweep` §4
//! finding 1: `approvals::decide_request` SQL has no `tenant_id`
//! filter.
//!
//! This test pins the **current** behaviour: today, `decide_request`
//! takes no `tenant_id` argument and the underlying SQL filters only
//! on `(id, status)`. A caller in tenant_b's context can therefore
//! flip tenant_a's pending approval to Approved if tenant_b knows or
//! can guess the approval id.
//!
//! The test is `#[ignore]`'d so it does not block CI today. It exists
//! to:
//!
//! 1. Make the gap explicit and reviewable — anyone reading
//!    `crates/argo-store/tests/` sees a named, runnable demonstration
//!    that the contract is loose.
//! 2. Be the regression gate when the fix-brief
//!    `mission-approval-decide-request-tenant-pinned` lands. That
//!    brief will:
//!    - add `tenant_id: TenantId` to `decide_request`'s signature,
//!    - change the WHERE clause to bind it,
//!    - un-ignore this test and invert its assertion: the
//!      cross-tenant decide call must return `false` (no row
//!      affected) and the row must remain Pending.
//!
//! The test does NOT validate that the gap is currently a *shipped*
//! leak — production callers in `crates/` are all `*_tests.rs` files
//! today; the `/api/v1/approvals` HTTP route goes through
//! `worker_actions_transitions::approve()`, not `decide_request`. The
//! leak surface materialises the moment a route handler wires
//! `decide_request`. This test pins the SQL-level contract that the
//! future fix must close.
//!
//! Cite: `docs/skycot/evals/scheduled/2026-05-23/eval-argo-store-tenant-bind-sweep.md` §4 finding 1.

use argo_store::approvals::{
    create_request, decide_request, get_request, list_pending, ApprovalDecision, DefaultDecision,
    NewApprovalRequest,
};
use argo_store::SessionStore;
use argo_types::TenantId;

/// Build a minimally-valid `NewApprovalRequest` for the audit.
fn approval_for(tenant: TenantId, requested_by: &'static str) -> NewApprovalRequest<'static> {
    NewApprovalRequest {
        action_id: vec![0u8; 16],
        tenant_id: tenant,
        requested_by,
        deadline_at: 1_700_000_000,
        default_decision: DefaultDecision::Deny,
        draft_content: Some("FINDINGS-ONLY pin test draft body"),
    }
}

/// Pins the §4-finding-1 gap: `decide_request` will successfully
/// approve a row even when the caller's "tenant context" mismatches
/// the row's `tenant_id`. The fix-brief
/// `mission-approval-decide-request-tenant-pinned` un-ignores this
/// test and inverts the assertion.
#[tokio::test]
#[ignore = "FINDINGS-ONLY pin: un-ignore + invert assertion when mission-approval-decide-request-tenant-pinned lands"]
async fn findings_decide_request_succeeds_across_tenant_boundary() {
    let store = SessionStore::open(":memory:").await.expect("open");
    store.migrate().await.expect("migrate");
    let pool = store.pool().clone();
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    assert_ne!(tenant_a, tenant_b, "test plumbing: tenant ids must differ");

    // tenant_a posts a pending approval.
    let id = create_request(&pool, &approval_for(tenant_a, "slack:worker-tenant-a"))
        .await
        .expect("create a-row");

    // Get the row to confirm the row's stored tenant_id is tenant_a.
    let row_before = get_request(&pool, &id)
        .await
        .expect("get a-row")
        .expect("a-row exists");
    assert_eq!(
        row_before.tenant_id, tenant_a,
        "test plumbing: stored row must belong to tenant_a"
    );

    // ⚠️ THE GAP: `decide_request` takes no tenant_id, so a
    // hypothetical tenant_b caller (or any code path that didn't
    // validate ownership) can flip the row.
    //
    // The "decided_by" string here is just a free-form audit
    // attribution — there is no check that the decider is in
    // tenant_a. This succeeds today.
    let decided = decide_request(
        &pool,
        &id,
        "slack:operator-tenant-b", // ← caller in tenant_b's context
        ApprovalDecision::Approve,
    )
    .await
    .expect("decide call returned an error (unexpected)");

    // Today: decided == true (cross-tenant decide succeeded).
    // After fix: this assertion will be inverted — decided must be
    // false, and the row must remain pending.
    assert!(
        decided,
        "FINDINGS-ONLY pin: today the SQL `WHERE id = ? AND status = 'pending'` has no tenant gate; a cross-tenant decide succeeds. If you see this assertion FAIL on `false`, the fix-brief has landed — un-ignore and invert."
    );

    // Confirm via the read path: tenant_a's pending list no longer
    // contains the row (because it was decided by tenant_b).
    let pending_a = list_pending(&pool, tenant_a, 100)
        .await
        .expect("list_pending tenant_a");
    assert!(
        pending_a.iter().all(|r| r.id != id),
        "FINDINGS-ONLY pin: the cross-tenant decide should have dropped the row from tenant_a's Pending list (today). Once the fix lands, this row should STILL be present in tenant_a's Pending list because the cross-tenant decide should have failed."
    );
}
