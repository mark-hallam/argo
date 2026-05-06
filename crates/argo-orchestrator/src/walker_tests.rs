//! Walker tests — exercise the four scenarios from the brief:
//! worker-step ahead → re-handoff, human-step ahead → notify,
//! chain exhausted → audit + session.Expired, non-overdue → no-op.
use super::*;
use crate::handoff::{Handoff, HandoffStatus};
use crate::session::Session;
use argo_audit::AuditSink;
use argo_identity::WorkerId;
use argo_store::SessionStore;
use argo_types::TenantId;
use chrono::Duration;
use std::sync::Mutex as StdMutex;

/// Mock notifier that records every `notify_human` call so tests
/// can assert (a) it was called for `Human` steps and (b) the
/// resolved address scheme.
struct RecordingNotifier {
    calls: StdMutex<Vec<(String, String)>>,
    fail: bool,
}

impl RecordingNotifier {
    fn new() -> Self {
        Self {
            calls: StdMutex::new(Vec::new()),
            fail: false,
        }
    }

    fn failing() -> Self {
        Self {
            calls: StdMutex::new(Vec::new()),
            fail: true,
        }
    }

    fn calls(&self) -> Vec<(String, String)> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl HandoffNotifier for RecordingNotifier {
    async fn notify_human(&self, addr: &str, summary: &str) -> ArgoResult<String> {
        if self.fail {
            return Err(ArgoError::Provider("simulated channel failure".into()));
        }
        self.calls
            .lock()
            .unwrap()
            .push((addr.to_string(), summary.to_string()));
        Ok(format!("delivery-{}", addr))
    }
}

async fn fixture() -> (
    Arc<OrchestratorStore>,
    Arc<AuditSink>,
    SqlitePool,
    TenantId,
) {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let pool = store.pool().clone();
    let repo = Arc::new(OrchestratorStore::new(Arc::clone(&store)));
    repo.ensure_schema().await.unwrap();
    let audit = Arc::new(AuditSink::new(Arc::clone(&store)));
    let tenant = TenantId::new();
    (repo, audit, pool, tenant)
}

async fn insert_session(repo: &OrchestratorStore, tenant: TenantId, title: &str) -> Session {
    let s = Session::new(tenant, title);
    repo.insert_session(&s).await.unwrap();
    s
}

fn make_handoff(
    session: &Session,
    from: WorkerId,
    to: EscalationStep,
    chain: Vec<String>,
    chain_index: usize,
    deadline: DateTime<Utc>,
) -> Handoff {
    let mut h = Handoff::new(
        session.id.clone(),
        from,
        to,
        "verify and post the listing",
        serde_json::json!({
            "escalation_chain": chain,
            "escalation_index": chain_index,
            "user_payload": {"sku": "skate-A"},
        }),
    );
    h.deadline = Some(deadline);
    h
}

#[tokio::test]
async fn overdue_handoff_with_worker_step_ahead_rehands_off() {
    let (repo, _audit, pool, tenant) = fixture().await;
    let session = insert_session(&repo, tenant, "skate").await;
    let from = WorkerId::new();
    let next_worker = WorkerId::new();
    let chain = vec![
        format!("worker:{from}"),
        format!("worker:{next_worker}"),
        "human:ops@acmeco.com".into(),
    ];
    let now = Utc::now();
    let h = make_handoff(
        &session,
        from,
        EscalationStep::Worker(from),
        chain,
        0,
        now - Duration::seconds(60),
    );
    let h_id = h.id.clone();
    repo.insert_handoff(tenant, &h).await.unwrap();

    let walker = HandoffWalker::new(
        Arc::clone(&repo),
        pool.clone(),
        Arc::new(RecordingNotifier::new()),
    );
    let report = walker.sweep_once(now).await.unwrap();
    assert_eq!(report.expired, 1);
    assert_eq!(report.re_handed_off, 1);
    assert_eq!(report.chain_exhausted, 0);
    assert_eq!(report.notifications, 0);

    // Original is now Expired.
    let original = repo.find_handoff(tenant, &h_id).await.unwrap().unwrap();
    assert_eq!(original.status, HandoffStatus::Expired);

    // A NEW pending handoff to the next worker exists with index = 1.
    let session_handoffs = repo
        .list_handoffs_for_session(tenant, &session.id)
        .await
        .unwrap();
    let new_pending: Vec<&Handoff> = session_handoffs
        .iter()
        .filter(|h| matches!(h.status, HandoffStatus::Pending))
        .collect();
    assert_eq!(new_pending.len(), 1);
    let next_idx = new_pending[0]
        .payload
        .get("escalation_index")
        .and_then(|v| v.as_u64())
        .unwrap();
    assert_eq!(next_idx, 1);
}

#[tokio::test]
async fn overdue_handoff_with_human_step_ahead_calls_notifier() {
    let (repo, _audit, pool, tenant) = fixture().await;
    let session = insert_session(&repo, tenant, "skate").await;
    let from = WorkerId::new();
    let chain = vec![format!("worker:{from}"), "human:mark@acmeco.com".into()];
    let now = Utc::now();
    let h = make_handoff(
        &session,
        from,
        EscalationStep::Worker(from),
        chain,
        0,
        now - Duration::seconds(60),
    );
    repo.insert_handoff(tenant, &h).await.unwrap();

    let notifier = Arc::new(RecordingNotifier::new());
    let walker = HandoffWalker::new(Arc::clone(&repo), pool.clone(), Arc::clone(&notifier) as _);
    let report = walker.sweep_once(now).await.unwrap();
    assert_eq!(report.expired, 1);
    assert_eq!(report.re_handed_off, 0);
    assert_eq!(report.notifications, 1);
    let calls = notifier.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "mark@acmeco.com");
    assert!(calls[0].1.contains("Handoff escalated"));
}

#[tokio::test]
async fn chain_exhaustion_writes_audit_and_expires_session() {
    let (repo, _audit, pool, tenant) = fixture().await;
    let session = insert_session(&repo, tenant, "skate").await;
    let from = WorkerId::new();
    // Single-step chain — advancing past 0 yields no more steps.
    let chain = vec![format!("worker:{from}")];
    let now = Utc::now();
    let h = make_handoff(
        &session,
        from,
        EscalationStep::Worker(from),
        chain,
        0,
        now - Duration::seconds(60),
    );
    repo.insert_handoff(tenant, &h).await.unwrap();

    let walker = HandoffWalker::new(
        Arc::clone(&repo),
        pool.clone(),
        Arc::new(RecordingNotifier::new()),
    );
    let report = walker.sweep_once(now).await.unwrap();
    assert_eq!(report.expired, 1);
    assert_eq!(report.chain_exhausted, 1);

    // Session flipped to Expired.
    let s = repo.find_session(tenant, &session.id).await.unwrap().unwrap();
    assert_eq!(s.status, crate::SessionStatus::Expired);

    // Audit row labelled escalation_exhausted exists.
    let session_uuid = uuid::Uuid::parse_str(session.id.as_str()).unwrap();
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE session_id = ? AND kind = 'escalation_exhausted'",
    )
    .bind(session_uuid.as_bytes().as_slice())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit_count, 1);
}

#[tokio::test]
async fn non_overdue_handoff_is_left_alone() {
    let (repo, _audit, pool, tenant) = fixture().await;
    let session = insert_session(&repo, tenant, "skate").await;
    let from = WorkerId::new();
    let chain = vec![
        format!("worker:{from}"),
        format!("worker:{}", WorkerId::new()),
    ];
    let now = Utc::now();
    let h = make_handoff(
        &session,
        from,
        EscalationStep::Worker(from),
        chain,
        0,
        now + Duration::hours(1), // future deadline
    );
    let h_id = h.id.clone();
    repo.insert_handoff(tenant, &h).await.unwrap();

    let walker = HandoffWalker::new(
        Arc::clone(&repo),
        pool.clone(),
        Arc::new(RecordingNotifier::new()),
    );
    let report = walker.sweep_once(now).await.unwrap();
    assert_eq!(report.expired, 0);
    assert_eq!(report.re_handed_off, 0);
    assert_eq!(report.chain_exhausted, 0);

    // Handoff still Pending.
    let still = repo.find_handoff(tenant, &h_id).await.unwrap().unwrap();
    assert_eq!(still.status, HandoffStatus::Pending);
}

#[tokio::test]
async fn notify_failure_treats_as_chain_exhausted() {
    let (repo, _audit, pool, tenant) = fixture().await;
    let session = insert_session(&repo, tenant, "skate").await;
    let from = WorkerId::new();
    let chain = vec![format!("worker:{from}"), "human:ops@acmeco.com".into()];
    let now = Utc::now();
    let h = make_handoff(
        &session,
        from,
        EscalationStep::Worker(from),
        chain,
        0,
        now - Duration::seconds(60),
    );
    repo.insert_handoff(tenant, &h).await.unwrap();

    let walker = HandoffWalker::new(
        Arc::clone(&repo),
        pool.clone(),
        Arc::new(RecordingNotifier::failing()),
    );
    let report = walker.sweep_once(now).await.unwrap();
    assert_eq!(report.expired, 1);
    assert_eq!(report.notifications, 0);
    assert_eq!(report.chain_exhausted, 1);
    let s = repo.find_session(tenant, &session.id).await.unwrap().unwrap();
    assert_eq!(s.status, crate::SessionStatus::Expired);
}

#[tokio::test]
async fn second_sweep_is_idempotent() {
    let (repo, _audit, pool, tenant) = fixture().await;
    let session = insert_session(&repo, tenant, "skate").await;
    let from = WorkerId::new();
    let chain = vec![format!("worker:{from}")];
    let now = Utc::now();
    let h = make_handoff(
        &session,
        from,
        EscalationStep::Worker(from),
        chain,
        0,
        now - Duration::seconds(60),
    );
    repo.insert_handoff(tenant, &h).await.unwrap();

    let walker = HandoffWalker::new(
        Arc::clone(&repo),
        pool.clone(),
        Arc::new(RecordingNotifier::new()),
    );
    let r1 = walker.sweep_once(now).await.unwrap();
    let r2 = walker.sweep_once(now + Duration::seconds(30)).await.unwrap();
    assert_eq!(r1.expired, 1);
    assert_eq!(r2.expired, 0);
    assert_eq!(r2.chain_exhausted, 0);
}
