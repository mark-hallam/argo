//! Walker tests for the org-chart manager-chain fallback wired in
//! `mission-handoffwalker-manager-chain`. Split into its own file
//! (sibling to `walker_tests.rs`) to keep both under the 500-LOC
//! hard ceiling. The four scenarios mirror the brief's acceptance
//! criteria:
//!
//! 1. Fallback emits to the direct manager when the explicit chain
//!    is exhausted but the originator has an `org_member` row.
//! 2. Fallback walks past the direct manager to the grand-manager
//!    when `org_chain_index` has already been bumped once.
//! 3. Topmost manager exhausts cleanly — `escalation_exhausted`
//!    audit fires + session flips to `Expired`.
//! 4. Worker without an `org_member` row falls straight through to
//!    the exhaust path without probing the chain.
use super::*;
use crate::handoff::{Handoff, HandoffStatus};
use crate::session::Session;
use argo_identity::WorkerId;
use argo_store::SessionStore;
use argo_types::{ArgoResult, TenantId};
use async_trait::async_trait;
use chrono::Duration;

struct NoopNotifier;

#[async_trait]
impl HandoffNotifier for NoopNotifier {
    async fn notify_human(&self, addr: &str, _summary: &str) -> ArgoResult<String> {
        Ok(format!("noop-{addr}"))
    }
}

async fn fixture() -> (Arc<OrchestratorStore>, SqlitePool, TenantId) {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let pool = store.pool().clone();
    let repo = Arc::new(OrchestratorStore::new(Arc::clone(&store)));
    repo.ensure_schema().await.unwrap();
    let tenant = TenantId::new();
    (repo, pool, tenant)
}

async fn insert_session(repo: &OrchestratorStore, tenant: TenantId, title: &str) -> Session {
    let s = Session::new(tenant, title);
    repo.insert_session(&s).await.unwrap();
    s
}

/// Seed one `org_member` row. Kind is `worker`; `manager_id` builds
/// the chain; `worker_id = None` simulates a human-only slot.
async fn insert_org_member(
    pool: &SqlitePool,
    tenant: TenantId,
    id: &str,
    worker_id: Option<&WorkerId>,
    manager_id: Option<&str>,
) {
    sqlx::query(
        "INSERT INTO org_member (id, tenant_id, kind, canonical, unit_id, \
         manager_id, worker_id, created_at) \
         VALUES (?, ?, 'worker', ?, NULL, ?, ?, 0)",
    )
    .bind(id)
    .bind(tenant.to_string())
    .bind(id)
    .bind(manager_id)
    .bind(worker_id.map(WorkerId::to_string))
    .execute(pool)
    .await
    .unwrap();
}

async fn exhausted_audit_count(pool: &SqlitePool, session: &Session) -> i64 {
    let session_uuid = uuid::Uuid::parse_str(session.id.as_str()).unwrap();
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM audit_record \
         WHERE session_id = ? AND kind = 'escalation_exhausted'",
    )
    .bind(session_uuid.as_bytes().as_slice())
    .fetch_one(pool)
    .await
    .unwrap()
}

fn make_handoff(
    session: &Session,
    from: WorkerId,
    chain: Vec<String>,
    chain_index: usize,
    org_chain_index: Option<usize>,
    deadline: DateTime<Utc>,
) -> Handoff {
    let mut payload = serde_json::json!({
        "escalation_chain": chain,
        "escalation_index": chain_index,
        "user_payload": {"sku": "skate-A"},
    });
    if let Some(idx) = org_chain_index {
        payload["org_chain_index"] = serde_json::json!(idx);
    }
    let mut h = Handoff::new(
        session.id.clone(),
        from,
        EscalationStep::Worker(from),
        "verify and post the listing",
        payload,
    );
    h.deadline = Some(deadline);
    h
}

fn pending_for(handoffs: Vec<Handoff>) -> Vec<Handoff> {
    handoffs
        .into_iter()
        .filter(|h| matches!(h.status, HandoffStatus::Pending))
        .collect()
}

#[tokio::test]
async fn org_chart_fallback_emits_to_direct_manager() {
    let (repo, pool, tenant) = fixture().await;
    let session = insert_session(&repo, tenant, "org-fallback").await;
    let worker_a = WorkerId::new();
    let worker_b = WorkerId::new();
    insert_org_member(&pool, tenant, "member-b", Some(&worker_b), None).await;
    insert_org_member(&pool, tenant, "member-a", Some(&worker_a), Some("member-b")).await;

    // Single-step chain → chain.advance(0) returns None → fallback fires.
    let chain = vec![format!("worker:{worker_a}")];
    let now = Utc::now();
    let h = make_handoff(&session, worker_a, chain, 0, None, now - Duration::seconds(60));
    repo.insert_handoff(tenant, &h).await.unwrap();

    let walker = HandoffWalker::new(Arc::clone(&repo), pool.clone(), Arc::new(NoopNotifier));
    let report = walker.sweep_once(now).await.unwrap();
    assert_eq!(report.expired, 1);
    assert_eq!(report.re_handed_off, 1);
    assert_eq!(report.chain_exhausted, 0);

    let pending = pending_for(
        repo.list_handoffs_for_session(tenant, &session.id)
            .await
            .unwrap(),
    );
    assert_eq!(pending.len(), 1);
    let to_worker = match &pending[0].to {
        EscalationStep::Worker(w) => *w,
        EscalationStep::Human(addr) => panic!("expected worker, got human:{addr}"),
    };
    assert_eq!(to_worker, worker_b);
    let next_idx = pending[0]
        .payload
        .get("escalation_index")
        .and_then(|v| v.as_u64())
        .unwrap();
    let next_org = pending[0]
        .payload
        .get("org_chain_index")
        .and_then(|v| v.as_u64())
        .unwrap();
    assert_eq!(next_idx, 1);
    assert_eq!(next_org, 1);

    // Fallback path must NOT write escalation_exhausted.
    assert_eq!(exhausted_audit_count(&pool, &session).await, 0);
}

#[tokio::test]
async fn org_chart_fallback_walks_to_grand_manager() {
    let (repo, pool, tenant) = fixture().await;
    let session = insert_session(&repo, tenant, "org-fallback-grand").await;
    let worker_a = WorkerId::new();
    let worker_b = WorkerId::new();
    let worker_c = WorkerId::new();
    insert_org_member(&pool, tenant, "member-c", Some(&worker_c), None).await;
    insert_org_member(&pool, tenant, "member-b", Some(&worker_b), Some("member-c")).await;
    insert_org_member(&pool, tenant, "member-a", Some(&worker_a), Some("member-b")).await;

    // Simulate the second sweep: explicit chain exhausted and the
    // org-chart pointer already past the direct manager.
    let chain = vec![format!("worker:{worker_a}")];
    let now = Utc::now();
    let h = make_handoff(&session, worker_a, chain, 1, Some(1), now - Duration::seconds(60));
    repo.insert_handoff(tenant, &h).await.unwrap();

    let walker = HandoffWalker::new(Arc::clone(&repo), pool.clone(), Arc::new(NoopNotifier));
    let report = walker.sweep_once(now).await.unwrap();
    assert_eq!(report.re_handed_off, 1);
    assert_eq!(report.chain_exhausted, 0);

    let pending = pending_for(
        repo.list_handoffs_for_session(tenant, &session.id)
            .await
            .unwrap(),
    );
    assert_eq!(pending.len(), 1);
    let to_worker = match &pending[0].to {
        EscalationStep::Worker(w) => *w,
        EscalationStep::Human(addr) => panic!("expected worker, got human:{addr}"),
    };
    assert_eq!(to_worker, worker_c);
}

#[tokio::test]
async fn org_chart_topmost_manager_writes_exhausted() {
    let (repo, pool, tenant) = fixture().await;
    let session = insert_session(&repo, tenant, "org-fallback-top").await;
    let worker_a = WorkerId::new();
    let worker_b = WorkerId::new();
    insert_org_member(&pool, tenant, "member-b", Some(&worker_b), None).await;
    insert_org_member(&pool, tenant, "member-a", Some(&worker_a), Some("member-b")).await;

    // org_chain_index already past the topmost (only one manager
    // exists at slot 0; asking for slot 1 falls off the chain).
    let chain = vec![format!("worker:{worker_a}")];
    let now = Utc::now();
    let h = make_handoff(&session, worker_a, chain, 2, Some(1), now - Duration::seconds(60));
    repo.insert_handoff(tenant, &h).await.unwrap();

    let walker = HandoffWalker::new(Arc::clone(&repo), pool.clone(), Arc::new(NoopNotifier));
    let report = walker.sweep_once(now).await.unwrap();
    assert_eq!(report.chain_exhausted, 1);
    assert_eq!(report.re_handed_off, 0);

    let s = repo.find_session(tenant, &session.id).await.unwrap().unwrap();
    assert_eq!(s.status, crate::SessionStatus::Expired);
    assert_eq!(exhausted_audit_count(&pool, &session).await, 1);
}

#[tokio::test]
async fn non_org_member_worker_falls_straight_through_to_exhaust() {
    let (repo, pool, tenant) = fixture().await;
    let session = insert_session(&repo, tenant, "no-org-member").await;
    let from = WorkerId::new();
    // No org_member row for `from`; org-chart probe must early-return.
    let chain = vec![format!("worker:{from}")];
    let now = Utc::now();
    let h = make_handoff(&session, from, chain, 0, None, now - Duration::seconds(60));
    repo.insert_handoff(tenant, &h).await.unwrap();

    let walker = HandoffWalker::new(Arc::clone(&repo), pool.clone(), Arc::new(NoopNotifier));
    let report = walker.sweep_once(now).await.unwrap();
    assert_eq!(report.expired, 1);
    assert_eq!(report.chain_exhausted, 1);
    assert_eq!(report.re_handed_off, 0);
    assert_eq!(exhausted_audit_count(&pool, &session).await, 1);
}
