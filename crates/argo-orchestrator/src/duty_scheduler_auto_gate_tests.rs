//! Unit tests for [`super::AutoApprovalDutyGate`] (UF Phase-B · B2).
//!
//! Proves the brief's acceptance matrix at the gate layer:
//! generous policy → real auto-approve; `$0` budget → held + an
//! approval item appears; cross-tenant worker → fail-safe deny.
//! The flag-OFF (deny-all preserved) half is a boot-path concern
//! pinned in `argo-cli`'s `boot_spawns` flag-parser test.
use super::*;
use argo_audit::AuditSink;
use argo_identity::{IdentityStore, Persona, Role, Worker};
use argo_types::TenantId;
use sqlx::Row;

async fn open_store() -> Arc<SessionStore> {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    store
}

async fn make_worker(store: &SessionStore, policy: AutoApprovalPolicy) -> Worker {
    let identities = IdentityStore::new(store);
    let mut worker = Worker::draft(
        TenantId::new(),
        "gtm",
        Persona::named("Maya", "op"),
        Role::new("Sell."),
    );
    worker.auto_approval = policy;
    identities.create_worker(&worker).await.unwrap();
    worker
}

fn duty_for(worker: &Worker) -> DutyRecord {
    DutyRecord::new(
        worker.tenant_id,
        worker.id,
        "0 * * * * *",
        "ping",
        "scheduled ping",
    )
}

fn generous() -> AutoApprovalPolicy {
    AutoApprovalPolicy {
        enabled: true,
        ..Default::default()
    }
}

/// Test resolver that reports a fixed `(risk, requires_approval)` for any
/// duty, so a test can drive the DEC-041 risk tier deterministically.
fn resolver(risk: &'static str, requires_approval: bool) -> DutyRiskResolver {
    Arc::new(move |_vid: &str, _action: &str| Some((risk.to_string(), requires_approval)))
}

async fn count_approval_asked(store: &SessionStore, tenant: TenantId) -> i64 {
    sqlx::query("SELECT COUNT(*) AS n FROM audit_record WHERE kind = ? AND tenant_id = ?")
        .bind("approval.asked")
        .bind(tenant.0.to_string())
        .fetch_one(store.pool())
        .await
        .unwrap()
        .try_get::<i64, _>("n")
        .unwrap()
}

#[tokio::test]
async fn safe_tier_low_risk_auto_approves_under_generous_policy() {
    // DEC-041 safe tier: low risk + manifest requires_approval=false →
    // auto-fire under an enabled (generous) policy.
    let store = open_store().await;
    let worker = make_worker(&store, generous()).await;
    let gate =
        AutoApprovalDutyGate::new(Arc::clone(&store)).with_risk_resolver(resolver("low", false));
    let decision = gate.evaluate(&duty_for(&worker)).await.unwrap();
    assert_eq!(decision, FireDecision::AutoApprove);
}

#[tokio::test]
async fn no_resolver_fails_closed_to_held() {
    // DEC-041 fail-closed: without a resolver the duty's risk is unknown →
    // treated as high + approval-required → held, even under a generous policy.
    let store = open_store().await;
    let worker = make_worker(&store, generous()).await;
    let gate = AutoApprovalDutyGate::new(Arc::clone(&store));
    let decision = gate.evaluate(&duty_for(&worker)).await.unwrap();
    assert_eq!(decision, FireDecision::DeniedGate);
}

#[tokio::test]
async fn high_risk_is_held_even_under_generous_policy() {
    let store = open_store().await;
    let worker = make_worker(&store, generous()).await;
    let gate =
        AutoApprovalDutyGate::new(Arc::clone(&store)).with_risk_resolver(resolver("high", false));
    let decision = gate.evaluate(&duty_for(&worker)).await.unwrap();
    assert_eq!(decision, FireDecision::DeniedGate);
}

#[tokio::test]
async fn medium_risk_is_held_even_under_generous_policy() {
    // The DEC-041 nuance: `medium` ("public surface but reversible") is NOT
    // in the act-by-default safe tier (DEC-028 no-autonomous-public-posting),
    // so it stays held even though the shared evaluator would otherwise
    // auto-approve a medium action within caps.
    let store = open_store().await;
    let worker = make_worker(&store, generous()).await;
    let gate = AutoApprovalDutyGate::new(Arc::clone(&store))
        .with_risk_resolver(resolver("medium", false));
    let decision = gate.evaluate(&duty_for(&worker)).await.unwrap();
    assert_eq!(decision, FireDecision::DeniedGate);
}

#[tokio::test]
async fn low_risk_but_manifest_requires_approval_is_held() {
    // Even low risk is held when the manifest declares requires_approval=true.
    let store = open_store().await;
    let worker = make_worker(&store, generous()).await;
    let gate =
        AutoApprovalDutyGate::new(Arc::clone(&store)).with_risk_resolver(resolver("low", true));
    let decision = gate.evaluate(&duty_for(&worker)).await.unwrap();
    assert_eq!(decision, FireDecision::DeniedGate);
}

#[tokio::test]
async fn zero_per_action_budget_holds_fire_and_surfaces_approval_item() {
    let store = open_store().await;
    // `$0` allowance per action: any non-zero per-fire estimate must
    // be held for a human → `Ask`.
    let policy = AutoApprovalPolicy {
        enabled: true,
        max_usd_micros_per_action: Some(0),
        ..Default::default()
    };
    let worker = make_worker(&store, policy).await;
    let sink = Arc::new(AuditSink::new_with_tenant(
        Arc::clone(&store),
        argo_audit::SYSTEM_AUDIT_TENANT_ID.to_string(),
    ));
    let gate = AutoApprovalDutyGate::new(Arc::clone(&store)).with_audit_sink(sink.clone());
    let decision = gate.evaluate(&duty_for(&worker)).await.unwrap();
    assert_eq!(decision, FireDecision::DeniedGate);
    sink.flush_pending().await.unwrap();
    assert_eq!(
        count_approval_asked(&store, worker.tenant_id).await,
        1,
        "a held fire must surface exactly one approval.asked item in its tenant"
    );
}

#[tokio::test]
async fn zero_daily_budget_blocks_fire_and_surfaces_approval_item() {
    let store = open_store().await;
    let policy = AutoApprovalPolicy {
        enabled: true,
        daily_budget_usd_micros: Some(0),
        ..Default::default()
    };
    let worker = make_worker(&store, policy).await;
    let sink = Arc::new(AuditSink::new_with_tenant(
        Arc::clone(&store),
        argo_audit::SYSTEM_AUDIT_TENANT_ID.to_string(),
    ));
    let gate = AutoApprovalDutyGate::new(Arc::clone(&store)).with_audit_sink(sink.clone());
    let decision = gate.evaluate(&duty_for(&worker)).await.unwrap();
    assert_eq!(decision, FireDecision::DeniedGate);
    sink.flush_pending().await.unwrap();
    assert_eq!(count_approval_asked(&store, worker.tenant_id).await, 1);
}

#[tokio::test]
async fn cross_tenant_worker_fails_closed() {
    let store = open_store().await;
    let worker = make_worker(&store, generous()).await;
    // Same worker_id, a DIFFERENT tenant on the duty → the
    // tenant-scoped lookup returns None → fail-safe deny.
    let mut duty = duty_for(&worker);
    duty.tenant_id = TenantId::new();
    let gate = AutoApprovalDutyGate::new(Arc::clone(&store));
    let decision = gate.evaluate(&duty).await.unwrap();
    assert_eq!(decision, FireDecision::DeniedGate);
}

#[tokio::test]
async fn unconfigured_worker_holds_fire_by_default() {
    let store = open_store().await;
    // Worker with the default (disabled) policy + disabled workspace
    // default → the H-1 evaluator returns `Ask` (policy:disabled).
    let worker = make_worker(&store, AutoApprovalPolicy::default()).await;
    let gate = AutoApprovalDutyGate::new(Arc::clone(&store));
    let decision = gate.evaluate(&duty_for(&worker)).await.unwrap();
    assert_eq!(decision, FireDecision::DeniedGate);
}

#[tokio::test]
async fn no_audit_sink_still_holds_fire_without_panicking() {
    let store = open_store().await;
    let policy = AutoApprovalPolicy {
        enabled: true,
        max_usd_micros_per_action: Some(0),
        ..Default::default()
    };
    let worker = make_worker(&store, policy).await;
    let gate = AutoApprovalDutyGate::new(Arc::clone(&store));
    let decision = gate.evaluate(&duty_for(&worker)).await.unwrap();
    assert_eq!(decision, FireDecision::DeniedGate);
    assert_eq!(count_approval_asked(&store, worker.tenant_id).await, 0);
}
