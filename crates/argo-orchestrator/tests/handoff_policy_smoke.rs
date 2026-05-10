//! Integration smoke for `claude-worker-handoff-policy`: prove the
//! `HandoffPolicyHook` re-routes the next worker, and that no hook
//! attached preserves the existing chain advancement (graceful
//! absence guarantee).

use argo_identity::WorkerId;
use argo_orchestrator::{
    EscalationStep, Handoff, HandoffNotifier, HandoffPolicyHook, HandoffStatus, HandoffWalker,
    OrchestratorStore, Session,
};
use argo_store::SessionStore;
use argo_types::{ArgoResult, TenantId};
use async_trait::async_trait;
use chrono::{Duration, Utc};
use std::sync::Arc;

struct NoopNotifier;

#[async_trait]
impl HandoffNotifier for NoopNotifier {
    async fn notify_human(&self, addr: &str, _summary: &str) -> ArgoResult<String> {
        Ok(format!("noop-{addr}"))
    }
}

/// Hook that always re-routes to a configured worker.
struct StaticHook {
    target: WorkerId,
}

impl HandoffPolicyHook for StaticHook {
    fn override_target(&self, _expected_outcome: &str) -> Option<WorkerId> {
        Some(self.target)
    }
}

async fn fixture() -> (Arc<OrchestratorStore>, sqlx::SqlitePool, TenantId, Session) {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let pool = store.pool().clone();
    let repo = Arc::new(OrchestratorStore::new(Arc::clone(&store)));
    repo.ensure_schema().await.unwrap();
    let tenant = TenantId::new();
    let session = Session::new(tenant, "policy-smoke");
    repo.insert_session(&session).await.unwrap();
    (repo, pool, tenant, session)
}

fn overdue_handoff(session: &Session, from: WorkerId, chain: Vec<String>) -> Handoff {
    let now = Utc::now();
    let mut h = Handoff::new(
        session.id.clone(),
        from,
        EscalationStep::Worker(from),
        "approval_request",
        serde_json::json!({
            "escalation_chain": chain,
            "escalation_index": 0,
        }),
    );
    h.deadline = Some(now - Duration::seconds(60));
    h
}

#[tokio::test]
async fn policy_hook_overrides_next_worker_target() {
    let (repo, pool, tenant, session) = fixture().await;
    let from = WorkerId::new();
    let chain_next = WorkerId::new();
    let policy_target = WorkerId::new();
    let chain = vec![format!("worker:{from}"), format!("worker:{chain_next}")];
    let h = overdue_handoff(&session, from, chain);
    repo.insert_handoff(tenant, &h).await.unwrap();

    let walker = HandoffWalker::new(Arc::clone(&repo), pool.clone(), Arc::new(NoopNotifier))
        .with_policy_hook(Arc::new(StaticHook {
            target: policy_target,
        }));
    let report = walker.sweep_once(Utc::now()).await.unwrap();
    assert_eq!(report.expired, 1);
    assert_eq!(report.re_handed_off, 1);

    // The new pending handoff should target `policy_target`, NOT
    // `chain_next` — proof the policy hook ran ahead of the chain.
    let pending: Vec<Handoff> = repo
        .list_handoffs_for_session(tenant, &session.id)
        .await
        .unwrap()
        .into_iter()
        .filter(|h| matches!(h.status, HandoffStatus::Pending))
        .collect();
    assert_eq!(pending.len(), 1);
    match &pending[0].to {
        EscalationStep::Worker(w) => assert_eq!(*w, policy_target),
        EscalationStep::Human(addr) => panic!("expected worker target, got human:{addr}"),
    }
}

#[tokio::test]
async fn no_policy_hook_preserves_chain_target() {
    let (repo, pool, tenant, session) = fixture().await;
    let from = WorkerId::new();
    let chain_next = WorkerId::new();
    let chain = vec![format!("worker:{from}"), format!("worker:{chain_next}")];
    let h = overdue_handoff(&session, from, chain);
    repo.insert_handoff(tenant, &h).await.unwrap();

    let walker = HandoffWalker::new(Arc::clone(&repo), pool.clone(), Arc::new(NoopNotifier));
    let report = walker.sweep_once(Utc::now()).await.unwrap();
    assert_eq!(report.expired, 1);
    assert_eq!(report.re_handed_off, 1);

    let pending: Vec<Handoff> = repo
        .list_handoffs_for_session(tenant, &session.id)
        .await
        .unwrap()
        .into_iter()
        .filter(|h| matches!(h.status, HandoffStatus::Pending))
        .collect();
    assert_eq!(pending.len(), 1);
    match &pending[0].to {
        EscalationStep::Worker(w) => assert_eq!(*w, chain_next),
        EscalationStep::Human(addr) => panic!("expected worker target, got human:{addr}"),
    }
}

/// End-to-end exercise of the org-chart manager-chain fallback
/// wired in `mission-handoffwalker-manager-chain`: seed a 3-level
/// chain, exhaust the explicit chain with the first sweep, then
/// observe the walker advance through `manager_b` → `manager_c`
/// before writing `escalation_exhausted` once the topmost is past.
#[tokio::test]
async fn org_chart_fallback_walks_three_levels_then_exhausts() {
    let (repo, pool, tenant, session) = fixture().await;
    let worker_a = WorkerId::new();
    let worker_b = WorkerId::new();
    let worker_c = WorkerId::new();
    // Insert org_member rows: a → b → c (root).
    seed_member(&pool, tenant, "member-c", Some(&worker_c), None).await;
    seed_member(&pool, tenant, "member-b", Some(&worker_b), Some("member-c")).await;
    seed_member(&pool, tenant, "member-a", Some(&worker_a), Some("member-b")).await;

    let chain = vec![format!("worker:{worker_a}")];
    let h = overdue_handoff(&session, worker_a, chain);
    repo.insert_handoff(tenant, &h).await.unwrap();

    let walker = HandoffWalker::new(Arc::clone(&repo), pool.clone(), Arc::new(NoopNotifier))
        .with_default_deadline(-60); // emit with already-past deadline so each sweep re-fires

    // Sweep 1 — explicit chain exhausted, org-chart hop 0 → worker_b.
    let r1 = walker.sweep_once(Utc::now()).await.unwrap();
    assert_eq!(r1.re_handed_off, 1);
    let to_after_1 = latest_pending_target(&repo, tenant, &session).await;
    assert_eq!(to_after_1, worker_b);

    // Sweep 2 — org-chart hop 1 → worker_c.
    let r2 = walker.sweep_once(Utc::now()).await.unwrap();
    assert_eq!(r2.re_handed_off, 1);
    let to_after_2 = latest_pending_target(&repo, tenant, &session).await;
    assert_eq!(to_after_2, worker_c);

    // Sweep 3 — past the topmost manager → exhausted audit.
    let r3 = walker.sweep_once(Utc::now()).await.unwrap();
    assert_eq!(r3.chain_exhausted, 1);
    assert_eq!(r3.re_handed_off, 0);
    let session_uuid = uuid::Uuid::parse_str(session.id.as_str()).unwrap();
    let exhausted: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record \
         WHERE session_id = ? AND kind = 'escalation_exhausted'",
    )
    .bind(session_uuid.as_bytes().as_slice())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(exhausted, 1);
}

async fn seed_member(
    pool: &sqlx::SqlitePool,
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

async fn latest_pending_target(
    repo: &OrchestratorStore,
    tenant: TenantId,
    session: &Session,
) -> WorkerId {
    let pending: Vec<Handoff> = repo
        .list_handoffs_for_session(tenant, &session.id)
        .await
        .unwrap()
        .into_iter()
        .filter(|h| matches!(h.status, HandoffStatus::Pending))
        .collect();
    assert_eq!(
        pending.len(),
        1,
        "expected exactly one Pending after sweep, got {}",
        pending.len()
    );
    match &pending[0].to {
        EscalationStep::Worker(w) => *w,
        EscalationStep::Human(addr) => panic!("expected worker target, got human:{addr}"),
    }
}
