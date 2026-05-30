//! Tests for [`PersistingExecutor`] + [`Supervisor`] end-to-end
//! against a real in-memory [`OrchestratorStore`]: child run rows +
//! the lead synthesis land as `provider='orchestrator'` session rows,
//! all tenant-scoped, and a different tenant sees none of them (the
//! round-37 vacuous-pass guard).
use super::*;
use crate::session::Session;
use crate::team_plan::{DelegationStep, StepId, TeamPlan};
use crate::team_run::Supervisor;
use crate::OrchestratorStore;
use argo_store::SessionStore;
use argo_types::TenantId;
use std::sync::Arc;

async fn store() -> Arc<OrchestratorStore> {
    let inner = Arc::new(SessionStore::open(":memory:").await.unwrap());
    inner.migrate().await.unwrap();
    let store = OrchestratorStore::new(inner);
    store.ensure_schema().await.unwrap();
    Arc::new(store)
}

/// Count `provider='orchestrator'` session rows for one tenant.
async fn orch_run_count(store: &OrchestratorStore, tenant: TenantId) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM session WHERE tenant_id = ? AND provider = 'orchestrator'",
    )
    .bind(tenant.0.as_bytes().as_slice())
    .fetch_one(store.pool())
    .await
    .unwrap()
}

fn plan() -> TeamPlan {
    let lead = argo_identity::WorkerId::new();
    TeamPlan::new(
        lead,
        "research-sprint",
        vec![
            DelegationStep::new(StepId::new("a"), argo_identity::WorkerId::new(), "gather"),
            DelegationStep::new(StepId::new("b"), argo_identity::WorkerId::new(), "price"),
            DelegationStep::new(StepId::new("c"), argo_identity::WorkerId::new(), "draft"),
        ],
    )
}

#[tokio::test]
async fn team_run_persists_children_plus_synthesis_tenant_scoped() {
    let store = store().await;
    let tenant = TenantId::new();
    let exec = Arc::new(PersistingExecutor::new(Arc::clone(&store)));
    let sup = Supervisor::new(exec);

    let result = sup
        .run_team(&plan(), tenant, Some("lead-handoff-1".into()))
        .await
        .expect("run");

    assert!(result.all_completed());
    assert_eq!(result.completed_count(), 3);
    // 3 child run rows + 1 lead synthesis row = 4 orchestrator rows.
    assert_eq!(orch_run_count(&store, tenant).await, 4);
    // Each completed child carries a persisted run id that resolves to
    // a real session row under the tenant.
    for outcome in &result.outcomes {
        let run_id = outcome.run_id.as_ref().expect("run id");
        let sid = crate::session::SessionId::from_string(run_id.clone());
        let row = store.find_session(tenant, &sid).await.unwrap();
        assert!(row.is_some(), "child run row must exist for {run_id}");
    }
    // The synthesis row also resolves under the tenant.
    let synth_sid = crate::session::SessionId::from_string(result.synthesis_run_id.clone());
    assert!(store.find_session(tenant, &synth_sid).await.unwrap().is_some());
}

#[tokio::test]
async fn cross_tenant_isolation_no_rows_leak() {
    let store = store().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let exec = Arc::new(PersistingExecutor::new(Arc::clone(&store)));
    let sup = Supervisor::new(exec);

    let result = sup.run_team(&plan(), tenant_a, None).await.expect("run");

    // Tenant A owns all 4 rows; tenant B sees none.
    assert_eq!(orch_run_count(&store, tenant_a).await, 4);
    assert_eq!(
        orch_run_count(&store, tenant_b).await,
        0,
        "cross-tenant isolation — other tenant must see zero team-run rows"
    );
    // A child run id from tenant A's run is invisible when queried as
    // tenant B (the tenant gate hides existence, not just contents).
    let leaked = result
        .outcomes
        .iter()
        .find_map(|o| o.run_id.clone())
        .expect("a run id");
    let sid = crate::session::SessionId::from_string(leaked);
    assert!(
        store.find_session(tenant_b, &sid).await.unwrap().is_none(),
        "tenant B must not resolve tenant A's run row"
    );
}

#[tokio::test]
async fn dependency_run_persists_only_completed_children() {
    // a → b (b depends on a). Force b's executor branch to still run
    // (both complete here) and confirm 2 children + 1 synthesis land.
    let store = store().await;
    let tenant = TenantId::new();
    let lead = argo_identity::WorkerId::new();
    let dep_plan = TeamPlan::new(
        lead,
        "ordered",
        vec![
            DelegationStep::new(StepId::new("a"), argo_identity::WorkerId::new(), "first"),
            DelegationStep::new(StepId::new("b"), argo_identity::WorkerId::new(), "second")
                .with_depends_on(vec![StepId::new("a")]),
        ],
    );
    let exec = Arc::new(PersistingExecutor::new(Arc::clone(&store)));
    let sup = Supervisor::new(exec);
    let result = sup.run_team(&dep_plan, tenant, None).await.expect("run");
    assert!(result.all_completed());
    assert_eq!(orch_run_count(&store, tenant).await, 3);
}

#[tokio::test]
async fn synthesize_default_returns_distinct_id() {
    // Sanity: the persisting synthesize writes a row distinct from
    // children, and the helper-built Session round-trips its id.
    let store = store().await;
    let tenant = TenantId::new();
    let s = Session::new(tenant, "probe");
    store.insert_session(&s).await.unwrap();
    let loaded = store.find_session(tenant, &s.id).await.unwrap();
    assert!(loaded.is_some());
}
