#![allow(deprecated)]

//! Genesis P5 (a) — the conservative default truth-boundary policy bound at
//! birth, exercised through the REAL `genesis_default_truth_boundary_ctx`
//! builder + the REAL `apply_truth_boundary_gate` enforcement seam (DEC-025
//! #6).
//!
//! These prove the *enforcement half* of P5 (a): that a worker carrying the
//! bound policy actually escalates a declared high-risk claim, never gates a
//! cross-tenant session, and honestly allows an undeclared claim. The
//! *data-plane half* (the marker survives the hire chokepoint → the runtime
//! read reports `Bound`) is pinned in `turn_vertical_fence_probation_tests`.

use super::turn_helpers_run::RoundOutcome;
use super::turn_helpers_truth_boundary::{
    apply_truth_boundary_gate, default_claim_provenance, genesis_default_truth_boundary_ctx,
};
use argo_audit::{rows_for_session, AuditSink};
use argo_store::approvals::list_pending;
use argo_store::SessionStore;
use argo_types::{SessionId, TenantId};
use std::sync::Arc;
use uuid::Uuid;

fn outcome(text: &str, declared_classes: Vec<&str>) -> RoundOutcome {
    RoundOutcome {
        assistant_id: Uuid::now_v7(),
        text: text.to_string(),
        declared_claim_classes: declared_classes.into_iter().map(str::to_string).collect(),
        claim_provenance: default_claim_provenance(),
        tokens_in: None,
        tokens_out: None,
        tool_calls: Vec::new(),
    }
}

async fn fresh_session() -> (Arc<SessionStore>, AuditSink, SessionId, TenantId) {
    let store = Arc::new(SessionStore::open(":memory:").await.expect("open store"));
    store.migrate().await.expect("migrate store");
    let (tenant, workspace) = store
        .ensure_workspace("genesis-truth-boundary", "/tmp/genesis-truth-boundary")
        .await
        .expect("workspace");
    let session_id = store
        .create_session(tenant, workspace, "mock-model", "mock-provider", Some("t"), None)
        .await
        .expect("session");
    let audit = AuditSink::new(Arc::clone(&store));
    (store, audit, session_id, tenant)
}

async fn audit_kinds(store: &Arc<SessionStore>, audit: &AuditSink, session_id: SessionId) -> Vec<String> {
    audit.flush_pending().await.expect("flush audit");
    rows_for_session(store, session_id)
        .await
        .expect("audit rows")
        .into_iter()
        .map(|row| row.kind)
        .collect()
}

#[tokio::test]
async fn genesis_default_policy_escalates_a_declared_financial_claim() {
    // A generated teammate that declares a `financial` claim must NOT ship it
    // to the customer — it escalates (queues an approval, holds the reply).
    let (store, audit, session_id, tenant) = fresh_session().await;
    let ctx = genesis_default_truth_boundary_ctx(tenant);
    let result = apply_truth_boundary_gate(
        &ctx,
        &store,
        &audit,
        None,
        session_id,
        tenant,
        outcome("Your refund of $4,200 is approved.", vec!["financial"]),
    )
    .await
    .expect("gate");

    let pending = list_pending(store.pool(), tenant, 10)
        .await
        .expect("pending approvals");
    assert_eq!(
        pending.len(),
        1,
        "a declared financial claim must enqueue an approval"
    );
    assert!(result.text.contains("Approval request"));
    assert_eq!(
        audit_kinds(&store, &audit, session_id).await,
        vec!["claim.escalated"]
    );
}

#[tokio::test]
async fn genesis_default_policy_escalates_legal_medical_and_factual() {
    // `legal`, `medical`, and `factual` declared claims all escalate (the
    // conservative posture). Proven through the gate, not just the policy
    // constructor.
    for class in ["legal", "medical", "factual"] {
        let (store, audit, session_id, tenant) = fresh_session().await;
        let ctx = genesis_default_truth_boundary_ctx(tenant);
        let result = apply_truth_boundary_gate(
            &ctx,
            &store,
            &audit,
            None,
            session_id,
            tenant,
            outcome("Here is my definitive answer.", vec![class]),
        )
        .await
        .expect("gate");
        assert!(
            result.text.contains("Approval request"),
            "class '{class}' must escalate"
        );
    }
}

#[tokio::test]
async fn genesis_default_policy_does_not_gate_a_cross_tenant_session() {
    // The bound policy is scoped to ITS tenant. A session belonging to a
    // DIFFERENT tenant must never be gated by it (cross-tenant claim-side
    // safety, enforced by `active_policy_for`).
    let (store, audit, session_id, session_tenant) = fresh_session().await;
    let other_tenant = TenantId::new();
    // Policy bound to `other_tenant`, but the session is `session_tenant`.
    let ctx = genesis_default_truth_boundary_ctx(other_tenant);
    let result = apply_truth_boundary_gate(
        &ctx,
        &store,
        &audit,
        None,
        session_id,
        session_tenant,
        outcome("Your refund of $4,200 is approved.", vec!["financial"]),
    )
    .await
    .expect("gate");

    assert_eq!(
        result.text, "Your refund of $4,200 is approved.",
        "another tenant's genesis policy must not gate this session"
    );
    assert!(
        audit_kinds(&store, &audit, session_id).await.is_empty(),
        "no claim audit row when the policy tenant does not match the session"
    );
}

#[tokio::test]
async fn genesis_default_policy_allows_an_undeclared_claim_the_honest_ceiling() {
    // The documented ceiling: the evaluator matches only producer-DECLARED
    // classes. An undeclared over-claim is Allow-by-default — a real
    // improvement over a policy-less worker, not a fabrication guarantee.
    let (store, audit, session_id, tenant) = fresh_session().await;
    let ctx = genesis_default_truth_boundary_ctx(tenant);
    let result = apply_truth_boundary_gate(
        &ctx,
        &store,
        &audit,
        None,
        session_id,
        tenant,
        outcome("Your refund of $4,200 is approved.", vec![]),
    )
    .await
    .expect("gate");

    assert_eq!(result.text, "Your refund of $4,200 is approved.");
    // An Allow still audits (claim.allowed) — the verdict is recorded.
    assert_eq!(
        audit_kinds(&store, &audit, session_id).await,
        vec!["claim.allowed"]
    );
}
