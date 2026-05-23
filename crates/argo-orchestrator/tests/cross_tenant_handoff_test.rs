//! Cross-tenant negative tests for every public orchestrator surface
//! that takes a `TenantId` (Stream
//! `eval-orchestrator-cross-tenant-handoff-negative-test`).
//!
//! Round-12 SHARED.md evaluator handoff (2026-05-23T12:00:00Z) found
//! the argo-orchestrator SQL was tenant-scoped at every WHERE clause
//! but the crate had ZERO cross-tenant negative tests. Same shape as
//! the round-3 audit-export pattern that later shipped a bug
//! (#2046 → #2051): SQL was right, no test pinned it, a refactor
//! silently dropped the predicate. This file locks the contract
//! before that regression class can land.
//!
//! Five public surfaces are tested, one [`#[tokio::test]`] per
//! verb. For each, the assertion is **byte-identical response shape**
//! between (a) the cross-tenant attempt and (b) a genuine
//! nonexistent-id call — per the existence-leak pattern from #2068
//! ("just returns 404" is too weak; cross-tenant must be
//! indistinguishable from genuine-not-found).
//!
//! ## FINDINGS — not blockers, but follow-up briefs are warranted
//!
//! [`argo_orchestrator::DutyStore::get`],
//! [`argo_orchestrator::DutyStore::mark_fired`],
//! [`argo_orchestrator::DutyStore::set_paused`],
//! [`argo_orchestrator::DutyStore::link_fire`], and
//! [`argo_orchestrator::DutyStore::sessions_for_duty`] do NOT take a
//! `TenantId` argument. They are id-scoped lookups; tenant scoping
//! depends entirely on the caller pre-filtering via
//! `list_for_tenant`. Production today calls them only from
//! `DutyScheduler::tick`, which seeds them from `list_active`, so the
//! seam holds — but the surface is not self-defending. Recommended
//! fix-brief: `mission-orchestrator-duty-store-tenant-args` (add
//! `TenantId` to every `DutyStore` mutator + lookup, mirror the
//! `OrchestratorStore` shape).

use argo_identity::WorkerId;
use argo_orchestrator::{
    BreachAction, DutyRecord, DutyStore, EscalationPolicy, EscalationStep, Handoff, HandoffId,
    HandoffStatus, OrchestratorStore, RetryPolicyStore, Session, SkillRetryPolicy, Sla, SlaStore,
    SlaTarget,
};
use argo_store::SessionStore;
use argo_types::TenantId;
use std::sync::Arc;

async fn fresh_store() -> Arc<SessionStore> {
    let s = Arc::new(SessionStore::open(":memory:").await.unwrap());
    s.migrate().await.unwrap();
    s
}

/// Two tenants seeded with one session-worker pair each — the canonical
/// fixture for every cross-tenant negative below.
struct TwoTenants {
    repo: OrchestratorStore,
    tenant_a: TenantId,
    tenant_b: TenantId,
    worker_a: WorkerId,
    session_a: Session,
}

async fn two_tenants() -> TwoTenants {
    let store = fresh_store().await;
    let repo = OrchestratorStore::new(store);
    repo.ensure_schema().await.unwrap();
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let worker_a = WorkerId::new();
    let session_a = Session::new(tenant_a, "tenant_a session");
    repo.insert_session(&session_a).await.unwrap();
    // tenant_b also gets a session so list_* calls have a real
    // tenant_b principal — otherwise an empty result is unfalsifiable.
    let session_b = Session::new(tenant_b, "tenant_b session");
    repo.insert_session(&session_b).await.unwrap();
    TwoTenants {
        repo,
        tenant_a,
        tenant_b,
        worker_a,
        session_a,
    }
}

fn handoff_for(session: &Session, from: WorkerId, to: WorkerId) -> Handoff {
    Handoff::new(
        session.id.clone(),
        from,
        EscalationStep::Worker(to),
        "verify",
        serde_json::json!({"sku": "USD-AEON-60"}),
    )
}

// ─── Surface 1: Handoff::find / insert / list_pending / list_for_session ───

#[tokio::test]
async fn find_handoff_cross_tenant_returns_none_identical_to_nonexistent_id() {
    let f = two_tenants().await;
    let h = handoff_for(&f.session_a, f.worker_a, WorkerId::new());
    f.repo.insert_handoff(f.tenant_a, &h).await.unwrap();

    let cross_tenant = f.repo.find_handoff(f.tenant_b, &h.id).await;
    let nonexistent = f.repo.find_handoff(f.tenant_b, &HandoffId::new()).await;

    // Both are Ok(None); the Debug repr must be byte-identical so a
    // future caller cannot distinguish "wrong tenant" from "no such id".
    assert_eq!(
        format!("{cross_tenant:?}"),
        format!("{nonexistent:?}"),
        "cross-tenant find must be indistinguishable from nonexistent"
    );
    assert!(matches!(cross_tenant, Ok(None)));

    // Sanity: tenant_a sees its own row.
    assert!(f.repo.find_handoff(f.tenant_a, &h.id).await.unwrap().is_some());
}

#[tokio::test]
async fn insert_handoff_into_other_tenants_session_is_rejected_identical_to_nonexistent_session() {
    let f = two_tenants().await;
    let h_cross = handoff_for(&f.session_a, f.worker_a, WorkerId::new());

    // tenant_b tries to attach a handoff to tenant_a's session.
    let cross = f.repo.insert_handoff(f.tenant_b, &h_cross).await;

    // Compare to the genuine "no such session" path: a fabricated
    // session id (never inserted) under tenant_b's own principal.
    let mut h_nonexistent = handoff_for(&f.session_a, f.worker_a, WorkerId::new());
    let ghost = Session::new(f.tenant_b, "ghost");
    h_nonexistent.session_id = ghost.id;
    let nonexistent = f.repo.insert_handoff(f.tenant_b, &h_nonexistent).await;

    assert_eq!(
        format!("{cross:?}"),
        format!("{nonexistent:?}"),
        "cross-tenant insert must be indistinguishable from nonexistent-session insert"
    );
    assert!(cross.is_err());
}

#[tokio::test]
async fn list_pending_handoffs_for_worker_cross_tenant_returns_empty() {
    let f = two_tenants().await;
    let h = handoff_for(&f.session_a, f.worker_a, f.worker_a);
    f.repo.insert_handoff(f.tenant_a, &h).await.unwrap();

    let cross = f
        .repo
        .list_pending_handoffs_for_worker(f.tenant_b, f.worker_a)
        .await;
    let unknown_worker = f
        .repo
        .list_pending_handoffs_for_worker(f.tenant_b, WorkerId::new())
        .await;

    assert_eq!(format!("{cross:?}"), format!("{unknown_worker:?}"));
    assert_eq!(cross.unwrap().len(), 0);

    // Sanity: tenant_a sees its own pending handoff.
    let owned = f
        .repo
        .list_pending_handoffs_for_worker(f.tenant_a, f.worker_a)
        .await
        .unwrap();
    assert_eq!(owned.len(), 1);
}

#[tokio::test]
async fn list_handoffs_for_session_cross_tenant_returns_empty() {
    let f = two_tenants().await;
    let h = handoff_for(&f.session_a, f.worker_a, WorkerId::new());
    f.repo.insert_handoff(f.tenant_a, &h).await.unwrap();

    let cross = f
        .repo
        .list_handoffs_for_session(f.tenant_b, &f.session_a.id)
        .await;
    // tenant_b asking after one of its OWN sessions but with no
    // handoffs attached — the genuine empty path.
    let session_b = Session::new(f.tenant_b, "extra tenant_b session");
    f.repo.insert_session(&session_b).await.unwrap();
    let nonexistent = f
        .repo
        .list_handoffs_for_session(f.tenant_b, &session_b.id)
        .await;

    assert_eq!(format!("{cross:?}"), format!("{nonexistent:?}"));
    assert_eq!(cross.unwrap().len(), 0);
}

// ─── Surface 2: update_handoff_status / cancel_handoff ───

#[tokio::test]
async fn update_handoff_status_cross_tenant_returns_not_found_identical_to_nonexistent_id() {
    let f = two_tenants().await;
    let h = handoff_for(&f.session_a, f.worker_a, WorkerId::new());
    f.repo.insert_handoff(f.tenant_a, &h).await.unwrap();

    let cross = f
        .repo
        .update_handoff_status(f.tenant_b, &h.id, HandoffStatus::Acknowledged)
        .await;
    let nonexistent = f
        .repo
        .update_handoff_status(f.tenant_b, &HandoffId::new(), HandoffStatus::Acknowledged)
        .await;

    assert_eq!(
        format!("{cross:?}"),
        format!("{nonexistent:?}"),
        "cross-tenant ack must be indistinguishable from ack-of-nonexistent"
    );
    assert!(cross.is_err());

    // Sanity: tenant_a's row was not silently mutated.
    let after = f.repo.find_handoff(f.tenant_a, &h.id).await.unwrap().unwrap();
    assert_eq!(after.status, HandoffStatus::Pending);
}

#[tokio::test]
async fn cancel_handoff_cross_tenant_returns_not_found_identical_to_nonexistent_id() {
    let f = two_tenants().await;
    let h = handoff_for(&f.session_a, f.worker_a, WorkerId::new());
    f.repo.insert_handoff(f.tenant_a, &h).await.unwrap();

    let cross = f.repo.cancel_handoff(f.tenant_b, &h.id, Some("nope")).await;
    let nonexistent = f
        .repo
        .cancel_handoff(f.tenant_b, &HandoffId::new(), Some("nope"))
        .await;

    assert_eq!(
        format!("{cross:?}"),
        format!("{nonexistent:?}"),
        "cross-tenant cancel must be indistinguishable from cancel-of-nonexistent"
    );
    assert!(cross.is_err());

    // Sanity: tenant_a's row was not silently cancelled.
    let after = f.repo.find_handoff(f.tenant_a, &h.id).await.unwrap().unwrap();
    assert_eq!(after.status, HandoffStatus::Pending);
}

// ─── Surface 3: RetryPolicyStore::get / list_for_skill ───

#[tokio::test]
async fn retry_policy_get_cross_tenant_returns_none_identical_to_nonexistent_id() {
    let store = fresh_store().await;
    let retries = RetryPolicyStore::new(store);
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let policy = SkillRetryPolicy::new(tenant_a, "invoice.extract");
    retries.upsert(&policy).await.unwrap();

    let cross = retries.get(tenant_b, &policy.id).await;
    let nonexistent = retries.get(tenant_b, "nonexistent-id").await;

    assert_eq!(format!("{cross:?}"), format!("{nonexistent:?}"));
    assert!(matches!(cross, Ok(None)));

    // Sanity: tenant_a sees its own policy.
    assert!(retries.get(tenant_a, &policy.id).await.unwrap().is_some());
}

#[tokio::test]
async fn retry_policy_list_for_skill_cross_tenant_returns_empty() {
    let store = fresh_store().await;
    let retries = RetryPolicyStore::new(store);
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    retries
        .upsert(&SkillRetryPolicy::new(tenant_a, "invoice.extract"))
        .await
        .unwrap();

    let cross = retries.list_for_skill(tenant_b, "invoice.extract").await;
    let unknown_skill = retries.list_for_skill(tenant_b, "no.such.skill").await;

    assert_eq!(format!("{cross:?}"), format!("{unknown_skill:?}"));
    assert_eq!(cross.unwrap().len(), 0);
}

// ─── Surface 4: DutyStore::list_for_tenant ───
//
// `list_active` is intentionally cross-tenant (scheduler input). The
// tenant-scoped public surface is `list_for_tenant`. See FINDINGS in
// the module header for the id-scoped methods (`get`, `mark_fired`,
// `set_paused`, `link_fire`, `sessions_for_duty`) that take no
// `TenantId` arg and are tracked under
// `mission-orchestrator-duty-store-tenant-args`.

#[tokio::test]
async fn duty_list_for_tenant_cross_tenant_returns_empty() {
    let store = fresh_store().await;
    let duties = DutyStore::new(store);
    duties.ensure_schema().await.unwrap();
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let worker = WorkerId::new();
    let duty = DutyRecord::new(tenant_a, worker, "0 * * * * *", "ping", "tenant_a duty");
    duties.insert(&duty).await.unwrap();

    let cross = duties.list_for_tenant(tenant_b).await;
    let fresh_tenant = duties.list_for_tenant(TenantId::new()).await;

    assert_eq!(format!("{cross:?}"), format!("{fresh_tenant:?}"));
    assert_eq!(cross.unwrap().len(), 0);

    // Sanity: tenant_a sees its own duty.
    assert_eq!(duties.list_for_tenant(tenant_a).await.unwrap().len(), 1);
}

// ─── Surface 5: SlaStore::get / list / update / delete ───

fn worker_sla(tenant: TenantId) -> Sla {
    Sla::new(
        tenant,
        SlaTarget::Worker(WorkerId::new()),
        5_000,
        EscalationPolicy::Notify("slack:#ops".into()),
        BreachAction::Notify,
    )
}

#[tokio::test]
async fn sla_get_cross_tenant_returns_none_identical_to_nonexistent_id() {
    let store = fresh_store().await;
    let slas = SlaStore::new(store);
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let sla = worker_sla(tenant_a);
    slas.create(&sla).await.unwrap();

    let cross = slas.get(tenant_b, &sla.id).await;
    let nonexistent = slas.get(tenant_b, "no-such-id").await;

    assert_eq!(format!("{cross:?}"), format!("{nonexistent:?}"));
    assert!(matches!(cross, Ok(None)));
}

#[tokio::test]
async fn sla_list_cross_tenant_returns_empty() {
    let store = fresh_store().await;
    let slas = SlaStore::new(store);
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    slas.create(&worker_sla(tenant_a)).await.unwrap();

    let cross = slas.list(tenant_b).await;
    let fresh_tenant = slas.list(TenantId::new()).await;

    assert_eq!(format!("{cross:?}"), format!("{fresh_tenant:?}"));
    assert_eq!(cross.unwrap().len(), 0);
}

#[tokio::test]
async fn sla_update_cross_tenant_returns_not_found_identical_to_nonexistent_id() {
    let store = fresh_store().await;
    let slas = SlaStore::new(store);
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let sla_a = worker_sla(tenant_a);

    // The update path embeds the caller-supplied id in its error
    // message. To test existence-leakage rigorously we must hit both
    // paths with the SAME id: once before tenant_a's row exists
    // (genuine nonexistent) and once after (cross-tenant).
    let mut spoofed = sla_a.clone();
    spoofed.tenant_id = tenant_b;
    spoofed.target_latency_ms = 99_999;

    // Path B: id genuinely does not exist anywhere yet.
    let nonexistent = slas.update(&spoofed).await;

    // Now bootstrap tenant_a's row, then re-issue the same forged
    // update under tenant_b — id now exists in tenant_a but is
    // invisible to tenant_b.
    slas.create(&sla_a).await.unwrap();
    let cross = slas.update(&spoofed).await;

    assert_eq!(
        format!("{cross:?}"),
        format!("{nonexistent:?}"),
        "cross-tenant update must be byte-identical to nonexistent — \
         differing strings reveal whether the id exists in another tenant"
    );
    assert!(cross.is_err());

    // Sanity: tenant_a's SLA was not mutated by the cross-tenant call.
    let after = slas.get(tenant_a, &sla_a.id).await.unwrap().unwrap();
    assert_eq!(after.target_latency_ms, sla_a.target_latency_ms);
}

#[tokio::test]
async fn sla_delete_cross_tenant_returns_false_identical_to_nonexistent_id() {
    let store = fresh_store().await;
    let slas = SlaStore::new(store);
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let sla = worker_sla(tenant_a);
    slas.create(&sla).await.unwrap();

    let cross = slas.delete(tenant_b, &sla.id).await;
    let nonexistent = slas.delete(tenant_b, "no-such-id").await;

    assert_eq!(format!("{cross:?}"), format!("{nonexistent:?}"));
    assert!(!cross.unwrap());

    // Sanity: tenant_a's row survives.
    assert!(slas.get(tenant_a, &sla.id).await.unwrap().is_some());
}
