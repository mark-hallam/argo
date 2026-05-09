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
