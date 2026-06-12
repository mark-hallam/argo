//! Regression test for `fix-routing-decision-persist-wire`.
//!
//! Drives one routed turn through
//! `argo_agent_runner::turn_routing::maybe_route_turn_with_transport`
//! and asserts the `routing_decision` TABLE (not just the audit log)
//! receives exactly one row, tagged with the correct `tenant_id` and a
//! non-Fixed (`"tiered"`) decision policy.
//!
//! Before the persist wire-up the only production caller built
//! `TieredRouter::new` bare and never wrapped it in `PersistingRouter`,
//! so the `routing_decision` table was permanently empty (§24.4 was met
//! by audit-log only). This pins the live SQL row.

use argo_audit::AuditSink;
use argo_provider::{
    ProviderRequest, ProviderResponse, ProviderResponseDelta, StopReason, Transport,
};
use argo_store::SessionStore;
use argo_types::{
    ArgoResult, CapabilitySet, ProviderModel, RoutingPolicy, SessionId, TenantId, TieredRouting,
    WorkspaceId,
};
use async_trait::async_trait;
use futures::stream::{self, BoxStream};
use std::sync::Arc;

/// Stub classifier transport: returns `"complex"` for every request so
/// the tier router picks the `complex` tier deterministically.
struct AlwaysComplex {
    caps: CapabilitySet,
}

#[async_trait]
impl Transport for AlwaysComplex {
    fn id(&self) -> &str {
        "stub-classifier"
    }

    fn capabilities(&self) -> &CapabilitySet {
        &self.caps
    }

    async fn complete(&self, _req: &ProviderRequest) -> ArgoResult<ProviderResponse> {
        Ok(ProviderResponse {
            text: Some("complex".into()),
            reasoning: None,
            tool_calls: Vec::new(),
            stop_reason: StopReason::EndTurn,
            tokens_in: None,
            tokens_out: None,
            provider_meta: serde_json::json!({}),
        })
    }

    async fn complete_stream(
        &self,
        _req: &ProviderRequest,
    ) -> ArgoResult<BoxStream<'static, ArgoResult<ProviderResponseDelta>>> {
        Ok(Box::pin(stream::empty()))
    }
}

fn complex_policy() -> RoutingPolicy {
    let cheap = ProviderModel {
        provider: "stub".into(),
        model: "gpt-cheap".into(),
    };
    let smart = ProviderModel {
        provider: "stub".into(),
        model: "claude-smart".into(),
    };
    RoutingPolicy::Tiered(TieredRouting {
        classifier_model: cheap.clone(),
        simple: cheap.clone(),
        medium: ProviderModel {
            provider: "stub".into(),
            model: "mid-tier".into(),
        },
        complex: smart,
        classify_threshold_ms: 1_000,
        fallback_to_default_on_error: true,
    })
}

async fn fixture() -> (Arc<SessionStore>, SessionId, TenantId, WorkspaceId) {
    let store = SessionStore::open(":memory:").await.expect("open");
    store.migrate().await.expect("migrate");
    let (tenant, workspace) = store
        .ensure_workspace("t", "/tmp/x")
        .await
        .expect("workspace");
    let sid = store
        .create_session(tenant, workspace, "session-default-model", "stub", None, None)
        .await
        .expect("create");
    (Arc::new(store), sid, tenant, workspace)
}

#[tokio::test]
async fn routed_turn_persists_exactly_one_routing_decision_row() {
    let (store, sid, tenant, _ws) = fixture().await;
    store
        .set_routing_policy(sid, Some(&complex_policy()))
        .await
        .expect("set policy");
    let session = store
        .get_session(sid)
        .await
        .expect("get session")
        .expect("session row");

    // Pre-condition: table starts empty for this tenant.
    let before = store
        .list_routing_decisions_for_tenant(tenant)
        .await
        .expect("list before");
    assert!(before.is_empty(), "table must start empty; got {before:?}");

    let audit = AuditSink::new_with_tenant(Arc::clone(&store), tenant.to_string());
    let classifier: Arc<dyn Transport> = Arc::new(AlwaysComplex {
        caps: CapabilitySet(Vec::new()),
    });
    let decision = argo_agent_runner::turn_routing::maybe_route_turn_with_transport(
        &store,
        &session,
        classifier,
        Some("please untangle the recursive dependency in turbine-v3"),
        &audit,
    )
    .await
    .expect("route")
    .expect("policy present so decision is Some");

    assert_eq!(decision.tier, "complex", "stub classifier picks complex");

    // The wire-up under test: the row must actually land in the table.
    let rows = store
        .list_routing_decisions_for_tenant(tenant)
        .await
        .expect("list after");
    assert_eq!(
        rows.len(),
        1,
        "exactly one routing_decision row must persist; got {}",
        rows.len()
    );
    let row = &rows[0];
    assert_eq!(row.tenant_id, tenant, "row must carry the routing tenant");
    assert_eq!(row.session_id, sid, "row must carry the routing session");
    assert_ne!(row.policy, "fixed", "Fixed decisions are not persisted");
    assert_eq!(row.policy, "tiered", "tier-routed decision policy");
    assert_eq!(row.tier, "complex", "persisted tier matches the decision");
}

#[tokio::test]
async fn unrouted_session_persists_no_routing_decision_row() {
    // No routing policy → maybe_route_turn returns None and nothing
    // should ever reach the routing_decision table.
    let (store, sid, tenant, _ws) = fixture().await;
    let session = store.get_session(sid).await.expect("get").expect("row");
    assert!(session.routing_policy.is_none());

    let audit = AuditSink::new_with_tenant(Arc::clone(&store), tenant.to_string());
    let classifier: Arc<dyn Transport> = Arc::new(AlwaysComplex {
        caps: CapabilitySet(Vec::new()),
    });
    let decision = argo_agent_runner::turn_routing::maybe_route_turn_with_transport(
        &store, &session, classifier, Some("hi"), &audit,
    )
    .await
    .expect("route");
    assert!(decision.is_none(), "no policy → no decision");

    let rows = store
        .list_routing_decisions_for_tenant(tenant)
        .await
        .expect("list");
    assert!(rows.is_empty(), "unrouted session writes no row; got {rows:?}");
}
