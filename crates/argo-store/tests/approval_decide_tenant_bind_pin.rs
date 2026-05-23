//! Regression pin for `eval-argo-store-tenant-bind-sweep` §4
//! finding 1: `approvals::decide_request` must filter on `tenant_id`
//! so a caller in tenant_b's context cannot flip tenant_a's pending
//! approval to Granted even if tenant_b knows or guesses the row id.
//!
//! Historical context: prior to `mission-approval-decide-request-tenant-pinned`
//! the SQL filtered only on `(id, status)` and the function signature
//! took no `tenant_id`. The file shipped originally as a FINDINGS-ONLY
//! `#[ignore]`'d pin asserting the *broken* behaviour, with explicit
//! instructions to un-ignore + invert when the fix-brief landed. This
//! is that inversion: the cross-tenant decide MUST return `false`,
//! and the row MUST stay Pending. A second test
//! (`same_tenant_decide_still_succeeds`) pins the positive direction
//! so a future tightening that accidentally rejects in-tenant decides
//! is caught immediately.
//!
//! Cite: `docs/skycot/evals/scheduled/2026-05-23/eval-argo-store-tenant-bind-sweep.md` §4 finding 1.

use argo_store::approvals::{
    create_request, decide_request, get_request, list_pending, ApprovalDecision, ApprovalStatus,
    DefaultDecision, NewApprovalRequest,
};
use argo_store::SessionStore;
use argo_types::TenantId;

/// Build a minimally-valid `NewApprovalRequest` for the regression pin.
fn approval_for(tenant: TenantId, requested_by: &'static str) -> NewApprovalRequest<'static> {
    NewApprovalRequest {
        action_id: vec![0u8; 16],
        tenant_id: tenant,
        requested_by,
        deadline_at: 1_700_000_000,
        default_decision: DefaultDecision::Deny,
        draft_content: Some("regression pin draft body"),
    }
}

/// Closes the §4-finding-1 gap: a `decide_request` invoked with
/// tenant_b on a row owned by tenant_a must affect zero rows and
/// leave the row Pending.
#[tokio::test]
async fn decide_request_rejects_cross_tenant_decide() {
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

    // Confirm the stored row belongs to tenant_a.
    let row_before = get_request(&pool, &id)
        .await
        .expect("get a-row")
        .expect("a-row exists");
    assert_eq!(
        row_before.tenant_id, tenant_a,
        "test plumbing: stored row must belong to tenant_a"
    );

    // tenant_b attempts to decide tenant_a's row using its own tenant id.
    // The SQL `WHERE id = ? AND tenant_id = ? AND status = 'pending'`
    // must match zero rows.
    let decided = decide_request(
        &pool,
        &id,
        tenant_b,
        "slack:operator-tenant-b",
        ApprovalDecision::Approve,
    )
    .await
    .expect("decide call returned an error (unexpected)");

    assert!(
        !decided,
        "cross-tenant decide must affect zero rows (returned true — the tenant_id bind is missing or broken)"
    );

    // Row must still be Pending — no status flip, no decided_by stamp.
    let row_after = get_request(&pool, &id)
        .await
        .expect("get a-row after")
        .expect("a-row still exists");
    assert_eq!(
        row_after.status,
        ApprovalStatus::Pending,
        "row must remain Pending after a rejected cross-tenant decide"
    );
    assert!(
        row_after.decided_by.is_none(),
        "decided_by must remain NULL after a rejected cross-tenant decide; got {:?}",
        row_after.decided_by
    );
    assert!(
        row_after.decision.is_none(),
        "decision must remain NULL after a rejected cross-tenant decide; got {:?}",
        row_after.decision
    );

    // And the row must STILL appear in tenant_a's pending list.
    let pending_a = list_pending(&pool, tenant_a, 100)
        .await
        .expect("list_pending tenant_a");
    assert!(
        pending_a.iter().any(|r| r.id == id),
        "row must remain in tenant_a's Pending list after a rejected cross-tenant decide; got {:?}",
        pending_a.iter().map(|r| r.id.as_str()).collect::<Vec<_>>()
    );
}

/// Positive direction: the same-tenant decide path keeps working.
/// Catches a future over-tightening (wrong bind, wrong column,
/// `AND tenant_id = NULL`) that would silently break legitimate
/// approvals.
#[tokio::test]
async fn same_tenant_decide_still_succeeds() {
    let store = SessionStore::open(":memory:").await.expect("open");
    store.migrate().await.expect("migrate");
    let pool = store.pool().clone();
    let tenant_a = TenantId::new();

    let id = create_request(&pool, &approval_for(tenant_a, "slack:worker-tenant-a"))
        .await
        .expect("create a-row");

    let decided = decide_request(
        &pool,
        &id,
        tenant_a,
        "human:operator-tenant-a",
        ApprovalDecision::Approve,
    )
    .await
    .expect("decide call returned an error");

    assert!(
        decided,
        "in-tenant decide must succeed; the tenant_id bind regressed"
    );

    let row = get_request(&pool, &id)
        .await
        .expect("get")
        .expect("row exists");
    assert_eq!(row.status, ApprovalStatus::Granted);
    assert_eq!(row.decided_by.as_deref(), Some("human:operator-tenant-a"));
    assert_eq!(row.decision.as_deref(), Some("granted"));
}

/// A cross-tenant decide attempt by a *different* tenant must not
/// stomp the row even after the legitimate owner later decides it.
/// In other words: rejection is real, not just delayed. Tenant_a's
/// legitimate decide should be the one that lands.
#[tokio::test]
async fn cross_tenant_reject_does_not_block_legitimate_decide() {
    let store = SessionStore::open(":memory:").await.expect("open");
    store.migrate().await.expect("migrate");
    let pool = store.pool().clone();
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();

    let id = create_request(&pool, &approval_for(tenant_a, "slack:worker-tenant-a"))
        .await
        .expect("create a-row");

    // tenant_b tries first — no row should change.
    let bad = decide_request(
        &pool,
        &id,
        tenant_b,
        "slack:attacker-tenant-b",
        ApprovalDecision::Reject,
    )
    .await
    .unwrap();
    assert!(!bad, "cross-tenant attempt must be a no-op");

    // tenant_a decides legitimately.
    let good = decide_request(
        &pool,
        &id,
        tenant_a,
        "human:operator-tenant-a",
        ApprovalDecision::Approve,
    )
    .await
    .unwrap();
    assert!(good, "legitimate same-tenant decide must affect 1 row");

    let row = get_request(&pool, &id).await.unwrap().unwrap();
    assert_eq!(row.status, ApprovalStatus::Granted);
    assert_eq!(row.decided_by.as_deref(), Some("human:operator-tenant-a"));
    assert_eq!(row.decision.as_deref(), Some("granted"));
}
