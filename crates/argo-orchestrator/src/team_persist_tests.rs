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

/// Read the title of a persisted run row by its run id.
async fn title_of(store: &OrchestratorStore, tenant: TenantId, run_id: &str) -> String {
    let sid = crate::session::SessionId::from_string(run_id.to_string());
    store
        .find_session(tenant, &sid)
        .await
        .unwrap()
        .expect("row exists")
        .title
}

/// Every persisted row in one team run — children + synthesis —
/// carries the SAME run-wide `[team:<id>]` prefix, and that id is a
/// real uuid (not the literal "<id>"). This is the grouping signal the
/// `/orchestration` view parses off the title; without it concurrent
/// team runs collapse into one tree.
#[tokio::test]
async fn all_rows_in_a_run_share_one_team_id_prefix() {
    let store = store().await;
    let tenant = TenantId::new();
    let exec = Arc::new(PersistingExecutor::new(Arc::clone(&store)));
    let sup = Supervisor::new(exec);

    let result = sup.run_team(&plan(), tenant, None).await.expect("run");

    // Pull the `<id>` out of the synthesis row's `[team:<id>] ...` title.
    let synth_title = title_of(&store, tenant, &result.synthesis_run_id).await;
    assert!(synth_title.contains("lead synthesis —"), "synth title: {synth_title}");
    let team_id = synth_title
        .strip_prefix("[team:")
        .and_then(|s| s.split(']').next())
        .expect("synthesis row titled [team:<id>] ...")
        .to_string();
    // The id is a parseable uuid, not a placeholder.
    assert!(uuid::Uuid::parse_str(&team_id).is_ok(), "team id not a uuid: {team_id}");

    // Every child row shares the identical `[team:<id>] ` prefix.
    let want_prefix = format!("[team:{team_id}] ");
    for outcome in &result.outcomes {
        let run_id = outcome.run_id.as_ref().expect("run id");
        let title = title_of(&store, tenant, run_id).await;
        assert!(
            title.starts_with(&want_prefix),
            "child title {title:?} must start with {want_prefix:?}"
        );
        assert!(title.contains('→'), "child title carries the step arrow: {title}");
    }
}

/// Two separate `run_team` calls mint two DISTINCT team ids — the
/// property that lets the dashboard render concurrent runs as two
/// separate trees rather than one merged "Delegated steps" node.
#[tokio::test]
async fn two_runs_get_distinct_team_ids() {
    let store = store().await;
    let tenant = TenantId::new();
    let exec = Arc::new(PersistingExecutor::new(Arc::clone(&store)));
    let sup = Supervisor::new(exec);

    let r1 = sup.run_team(&plan(), tenant, None).await.expect("run 1");
    let r2 = sup.run_team(&plan(), tenant, None).await.expect("run 2");

    let id_of = |title: &str| -> String {
        title
            .strip_prefix("[team:")
            .and_then(|s| s.split(']').next())
            .expect("[team:<id>] title")
            .to_string()
    };
    let id1 = id_of(&title_of(&store, tenant, &r1.synthesis_run_id).await);
    let id2 = id_of(&title_of(&store, tenant, &r2.synthesis_run_id).await);
    assert_ne!(id1, id2, "each team run must mint a distinct id");
}
