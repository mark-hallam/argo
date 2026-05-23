//! # Mission Claim 6 MVP — cross-tenant approval queue isolation
//!
//! Pins the operator-locked MVP row (round 19):
//!
//! > Cross-tenant isolation — tenant_b can't see/decide tenant_a's
//! > pending Slack-approval queue.
//!
//! ## What this test proves
//!
//! 1. **List isolation.** `list_pending(tenant_a)` MUST return ONLY
//!    tenant_a's Pending rows and `list_pending(tenant_b)` MUST return
//!    ONLY tenant_b's Pending rows. Zero cross-tenant rows in either
//!    direction.
//! 2. **Status filter is independent of tenant filter.** Decided rows
//!    drop out of each tenant's Pending list without disturbing the
//!    other tenant's count.
//! 3. **SQL belt-and-braces.** Even at the raw-SQL layer, the
//!    `tenant_id` column on every approval row matches the owning
//!    tenant — catches a hypothetical insert-time bug that would
//!    silently clobber tenant_id.
//! 4. **Get returns tenant_id for callers to enforce decide scoping.**
//!    `get_request(tenant_a_row_id)` returns the row with
//!    `tenant_id == tenant_a`, so any HTTP route handler can refuse to
//!    let tenant_b's session act on it. This pins the contract the
//!    gateway depends on for `decide_request` scoping.
//! 5. **Limit isolation.** A high limit on tenant_a never causes
//!    tenant_b rows to leak in (defensive — catches an off-by-one in
//!    the LIMIT clause that returns extra rows from across tenants).
//!
//! ## Why this is the MVP row 5 test
//!
//! Slack outbound (row 1) + Slack inbound (row 2) prove the message
//! path; the audit chain (row 6) proves tamper detection. Row 5 is the
//! piece that prevents a tenant_b operator from seeing — or worse,
//! approving — tenant_a's pending Slack-routed approval. Without this
//! invariant a cross-tenant approval becomes a cross-tenant action,
//! which would breach the "multi-tenant by construction" core invariant.
//!
//! ## Authored
//!
//! Evaluator round 24, 2026-05-23.

use argo_store::approvals::{
    create_request, decide_request, get_request, list_pending, ApprovalDecision,
    DefaultDecision, NewApprovalRequest,
};
use argo_store::SessionStore;
use argo_types::TenantId;

/// Build a `NewApprovalRequest` carrying a Slack-shaped `requested_by`
/// + draft body so the row is unambiguously a Slack-routed approval.
fn slack_request<'a>(
    tenant: TenantId,
    deadline_at: i64,
    requested_by: &'a str,
    draft: &'a str,
) -> NewApprovalRequest<'a> {
    NewApprovalRequest {
        action_id: vec![0u8; 16],
        tenant_id: tenant,
        requested_by,
        deadline_at,
        default_decision: DefaultDecision::Deny,
        draft_content: Some(draft),
    }
}

#[tokio::test]
async fn claim_6_mvp_cross_tenant_slack_approval_queue_isolation() {
    let store = SessionStore::open(":memory:").await.expect("open");
    store.migrate().await.expect("migrate");
    let pool = store.pool().clone();
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    assert_ne!(
        tenant_a, tenant_b,
        "test plumbing: tenant ids must differ"
    );

    // ── Phase 1: tenant_a posts 3 Slack-routed approvals; tenant_b posts 2.
    let a_listing_below_floor = create_request(
        &pool,
        &slack_request(
            tenant_a,
            1_700_000_000,
            "slack:worker-mariah-marketplace",
            "Mariah is about to post a $1,200 hockey stick listing 18% below floor — confirm?",
        ),
    )
    .await
    .expect("create a1");
    let a_inventory_sync = create_request(
        &pool,
        &slack_request(
            tenant_a,
            1_700_000_001,
            "slack:worker-mariah-marketplace",
            "Sync 437 SKU inventory updates from CSV — confirm?",
        ),
    )
    .await
    .expect("create a2");
    let a_refund = create_request(
        &pool,
        &slack_request(
            tenant_a,
            1_700_000_002,
            "slack:worker-jamal-support",
            "Issue $89.99 refund to customer #12345 — confirm?",
        ),
    )
    .await
    .expect("create a3");
    let b_recruit_screen = create_request(
        &pool,
        &slack_request(
            tenant_b,
            1_700_000_003,
            "slack:worker-priya-recruiter",
            "Pass candidate #88210 to second-round interview — confirm?",
        ),
    )
    .await
    .expect("create b1");
    let b_data_entry = create_request(
        &pool,
        &slack_request(
            tenant_b,
            1_700_000_004,
            "slack:worker-eva-dataentry",
            "Apply OCR results to 12 invoices flagged below 0.85 confidence — confirm?",
        ),
    )
    .await
    .expect("create b2");

    // Decide one of tenant_a's so it leaves the Pending list — confirms
    // the status filter is orthogonal to the tenant filter.
    let decided = decide_request(
        &pool,
        &a_refund,
        tenant_a,
        "human:operator-tenant-a",
        ApprovalDecision::Approve,
    )
    .await
    .expect("decide a3");
    assert!(decided, "test plumbing: first decide must affect 1 row");

    // ── Phase 2: list_pending(tenant_a) returns ONLY tenant_a's 2 remaining
    // Pending rows; list_pending(tenant_b) returns ONLY tenant_b's 2.
    let pending_a = list_pending(&pool, tenant_a, 100)
        .await
        .expect("list_pending tenant_a");
    assert_eq!(
        pending_a.len(),
        2,
        "claim-6 MVP row 5: tenant_a must see EXACTLY 2 Pending rows (3 created, 1 decided); got {}",
        pending_a.len()
    );
    for row in &pending_a {
        assert_eq!(
            row.tenant_id, tenant_a,
            "claim-6 MVP row 5: tenant_a list_pending row {} has wrong tenant_id {:?}",
            row.id, row.tenant_id
        );
    }
    let pending_a_ids: Vec<&str> = pending_a.iter().map(|r| r.id.as_str()).collect();
    assert!(
        pending_a_ids.contains(&a_listing_below_floor.as_str()),
        "claim-6 MVP row 5: tenant_a missing its own row {}",
        a_listing_below_floor
    );
    assert!(
        pending_a_ids.contains(&a_inventory_sync.as_str()),
        "claim-6 MVP row 5: tenant_a missing its own row {}",
        a_inventory_sync
    );
    assert!(
        !pending_a_ids.contains(&a_refund.as_str()),
        "claim-6 MVP row 5: decided row {} must drop out of Pending list",
        a_refund
    );
    assert!(
        !pending_a_ids.contains(&b_recruit_screen.as_str())
            && !pending_a_ids.contains(&b_data_entry.as_str()),
        "claim-6 MVP row 5: tenant_a list_pending LEAKED tenant_b rows — got {pending_a_ids:?}"
    );

    let pending_b = list_pending(&pool, tenant_b, 100)
        .await
        .expect("list_pending tenant_b");
    assert_eq!(
        pending_b.len(),
        2,
        "claim-6 MVP row 5: tenant_b must see EXACTLY 2 Pending rows; got {}",
        pending_b.len()
    );
    for row in &pending_b {
        assert_eq!(
            row.tenant_id, tenant_b,
            "claim-6 MVP row 5: tenant_b list_pending row {} has wrong tenant_id {:?}",
            row.id, row.tenant_id
        );
    }
    let pending_b_ids: Vec<&str> = pending_b.iter().map(|r| r.id.as_str()).collect();
    assert!(
        pending_b_ids.contains(&b_recruit_screen.as_str())
            && pending_b_ids.contains(&b_data_entry.as_str()),
        "claim-6 MVP row 5: tenant_b missing its own rows; got {pending_b_ids:?}"
    );
    assert!(
        !pending_b_ids.contains(&a_listing_below_floor.as_str())
            && !pending_b_ids.contains(&a_inventory_sync.as_str())
            && !pending_b_ids.contains(&a_refund.as_str()),
        "claim-6 MVP row 5: tenant_b list_pending LEAKED tenant_a rows — got {pending_b_ids:?}"
    );

    // ── Phase 3: SQL belt-and-braces. Every approval_request row in the
    // table MUST have its tenant_id matching the tenant_id of the
    // creating side. Reads outside list_pending must agree.
    let raw_counts: Vec<(Vec<u8>, i64)> =
        sqlx::query_as("SELECT tenant_id, COUNT(*) FROM approval_request GROUP BY tenant_id")
            .fetch_all(&pool)
            .await
            .expect("raw groupby");
    let tenant_a_bytes = tenant_a.0.as_bytes().to_vec();
    let tenant_b_bytes = tenant_b.0.as_bytes().to_vec();
    let mut saw_a = 0;
    let mut saw_b = 0;
    for (bytes, count) in &raw_counts {
        if bytes == &tenant_a_bytes {
            saw_a = *count;
        } else if bytes == &tenant_b_bytes {
            saw_b = *count;
        } else {
            panic!(
                "claim-6 MVP row 5: unexpected tenant_id bytes in approval_request: {:?}",
                bytes
            );
        }
    }
    assert_eq!(
        saw_a, 3,
        "claim-6 MVP row 5: tenant_a should own 3 raw rows (1 decided + 2 pending); got {saw_a}"
    );
    assert_eq!(
        saw_b, 2,
        "claim-6 MVP row 5: tenant_b should own 2 raw rows; got {saw_b}"
    );

    // ── Phase 4: get_request returns the row's tenant_id so the gateway
    // can refuse cross-tenant decides. This pins the contract: any
    // gateway route handler MUST check `row.tenant_id == session.tenant_id`
    // before calling decide_request.
    let row_a = get_request(&pool, &a_listing_below_floor)
        .await
        .expect("get_request a1")
        .expect("row exists");
    assert_eq!(
        row_a.tenant_id, tenant_a,
        "claim-6 MVP row 5: get_request must return tenant_id; got {:?}",
        row_a.tenant_id
    );
    let row_b = get_request(&pool, &b_recruit_screen)
        .await
        .expect("get_request b1")
        .expect("row exists");
    assert_eq!(
        row_b.tenant_id, tenant_b,
        "claim-6 MVP row 5: get_request must return tenant_id; got {:?}",
        row_b.tenant_id
    );
    assert_ne!(
        row_a.tenant_id, row_b.tenant_id,
        "claim-6 MVP row 5: per-row tenant_id must differ across tenants"
    );

    // ── Phase 5: Limit isolation. A high limit on tenant_a never lets
    // tenant_b rows leak in.
    let pending_a_high_limit = list_pending(&pool, tenant_a, 1_000_000)
        .await
        .expect("list_pending high limit tenant_a");
    for row in &pending_a_high_limit {
        assert_eq!(
            row.tenant_id, tenant_a,
            "claim-6 MVP row 5: high-limit list_pending(tenant_a) leaked row {} from tenant {:?}",
            row.id, row.tenant_id
        );
    }
    assert_eq!(
        pending_a_high_limit.len(),
        2,
        "claim-6 MVP row 5: high-limit list_pending(tenant_a) row count must still equal Pending count"
    );
}

#[tokio::test]
async fn claim_6_mvp_cross_tenant_isolation_holds_under_concurrent_inserts() {
    // Defensive — simulate two tenants posting Slack approvals
    // concurrently. After all writes land, list_pending must still be
    // tenant-scoped. Catches a hypothetical bug where a shared
    // in-memory write buffer cross-bleeds tenant_id under contention.
    let store = SessionStore::open(":memory:").await.expect("open");
    store.migrate().await.expect("migrate");
    let pool = store.pool().clone();
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();

    // Spawn 10 concurrent insert pairs (20 total: 10 for A, 10 for B).
    let mut handles = Vec::new();
    for i in 0..10 {
        let pool_a = pool.clone();
        let pool_b = pool.clone();
        handles.push(tokio::spawn(async move {
            create_request(
                &pool_a,
                &slack_request(
                    tenant_a,
                    1_700_000_000 + i,
                    "slack:worker-concurrent-a",
                    "tenant_a concurrent insert",
                ),
            )
            .await
            .expect("a insert")
        }));
        handles.push(tokio::spawn(async move {
            create_request(
                &pool_b,
                &slack_request(
                    tenant_b,
                    1_700_000_100 + i,
                    "slack:worker-concurrent-b",
                    "tenant_b concurrent insert",
                ),
            )
            .await
            .expect("b insert")
        }));
    }
    for h in handles {
        h.await.expect("join");
    }

    let pending_a = list_pending(&pool, tenant_a, 1000).await.expect("list a");
    let pending_b = list_pending(&pool, tenant_b, 1000).await.expect("list b");

    assert_eq!(
        pending_a.len(),
        10,
        "claim-6 MVP row 5: concurrent inserts must yield exactly 10 tenant_a rows"
    );
    assert_eq!(
        pending_b.len(),
        10,
        "claim-6 MVP row 5: concurrent inserts must yield exactly 10 tenant_b rows"
    );
    for row in &pending_a {
        assert_eq!(
            row.tenant_id, tenant_a,
            "claim-6 MVP row 5: concurrent insert leaked tenant_b row into tenant_a's Pending list"
        );
    }
    for row in &pending_b {
        assert_eq!(
            row.tenant_id, tenant_b,
            "claim-6 MVP row 5: concurrent insert leaked tenant_a row into tenant_b's Pending list"
        );
    }
}
