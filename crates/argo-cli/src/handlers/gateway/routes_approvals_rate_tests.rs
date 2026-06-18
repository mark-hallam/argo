//! Tests for the durable tenant-wide approval-rate read-side
//! (`routes_approvals_rate`).
//!
//! The verifier the brief demands (checkable, NO THEATER): seed N
//! `approval.granted` + M `approval.declined` audit rows for a tenant,
//! call the read, assert `total == N+M` and `approved_fraction ==
//! N/(N+M)`; assert a cross-tenant row is NOT counted.

use super::{count_decisions, ApprovalRateSummary};
use argo_audit::{AuditKind, AuditSink};
use serde_json::json;
use std::sync::Arc;

#[test]
fn to_json_is_none_with_no_decisions() {
    let summary = ApprovalRateSummary {
        granted: 0,
        declined: 0,
    };
    assert!(
        summary.to_json().is_none(),
        "no decisions → absent field (divide-by-zero safe)"
    );
}

#[test]
fn to_json_shapes_the_envelope() {
    let summary = ApprovalRateSummary {
        granted: 3,
        declined: 1,
    };
    let v = summary.to_json().expect("decisions present");
    assert_eq!(v["total"], 4);
    assert_eq!(v["approved_fraction"], 0.75);
    assert_eq!(v["refused_fraction"], 0.25);
    assert_eq!(v["expired_fraction"], 0.0);
}

/// Open an in-memory-backed store and write `granted` + `declined`
/// decision rows for `tenant`, flushed so the count sees them.
async fn seed_decisions(
    store: &Arc<argo_store::SessionStore>,
    tenant: &str,
    granted: usize,
    declined: usize,
) {
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    for _ in 0..granted {
        sink.write(None, AuditKind::ApprovalGranted, json!({}), None)
            .await
            .expect("write granted");
    }
    for _ in 0..declined {
        sink.write(None, AuditKind::ApprovalDeclined, json!({}), None)
            .await
            .expect("write declined");
    }
    sink.flush_pending().await.expect("flush decisions");
}

#[tokio::test]
async fn counts_granted_vs_declined_and_excludes_cross_tenant() {
    let store = argo_store::SessionStore::open(":memory:")
        .await
        .expect("in-memory store");
    store.migrate().await.expect("migrate");
    let store = Arc::new(store);
    // Tenant A: N=3 granted, M=1 declined → total 4, approved 0.75.
    seed_decisions(&store, "tenant-a", 3, 1).await;
    // Tenant B: a different volume so a tenant leak would show up.
    seed_decisions(&store, "tenant-b", 5, 5).await;

    let a = count_decisions(store.pool(), "tenant-a")
        .await
        .expect("count a");
    assert_eq!(a.total(), 4, "N+M for tenant A only");
    let json_a = a.to_json().expect("a has decisions");
    assert_eq!(json_a["total"], 4);
    assert_eq!(json_a["approved_fraction"], 0.75);

    // Cross-tenant fail-closed: tenant B's 10 rows are never counted for A,
    // and A's 4 rows are never counted for B.
    let b = count_decisions(store.pool(), "tenant-b")
        .await
        .expect("count b");
    assert_eq!(b.total(), 10, "tenant B sees only its own rows");
    assert_eq!(b.granted, 5);
    assert_eq!(b.declined, 5);
}
