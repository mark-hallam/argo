//! Smoke test for the `mission-claim-3-runtime-wiring-v1x` wire-up.
//!
//! Drives `argo_agent_runner::turn_routing::maybe_route_turn_with_transport`
//! against an in-memory session that opts into `RoutingPolicy::Tiered`
//! and a stub `Transport` whose `complete` always returns the single
//! word `"complex"`. The assertions:
//!
//! 1. The classifier is consulted (tier classifier text "complex"
//!    surfaces in the returned [`RoutingDecision`]).
//! 2. The chosen `(provider, model)` matches the `complex` tier from
//!    the policy — i.e. tier-routing actually overrides
//!    `session.model`.
//! 3. An `AuditKind::RoutingDecision` audit row lands on the session.
//!
//! These three together close the cost-tier half of mission claim 3.
//! Inter-provider fallback is OUT OF SCOPE (sibling brief
//! `mission-provider-fallback-on-error-v1x`).

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

/// Stub classifier transport: returns `"complex"` for every request.
/// Satisfies `argo_routing::TieredRouter::route`'s contract — the
/// router parses `resp.text` for the tier keyword and picks
/// `tiered.complex` when it sees `complex`.
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
async fn tiered_routing_overrides_session_model_with_complex_tier() {
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
    assert!(
        session.routing_policy.is_some(),
        "routing_policy must round-trip"
    );

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
    assert_eq!(
        decision.chosen_provider_model.model, "claude-smart",
        "the complex tier's model must override session.model"
    );
    assert_ne!(
        decision.chosen_provider_model.model, session.model,
        "tier routing must NOT degenerate to session.model"
    );

    let rows = store
        .list_audit_for_session(sid)
        .await
        .expect("audit fetch");
    assert!(
        rows.iter().any(|r| r.kind == "routing.decision"),
        "routing.decision audit row must land; got {:?}",
        rows.iter().map(|r| r.kind.clone()).collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn no_policy_returns_none() {
    let (store, sid, tenant, _ws) = fixture().await;
    let session = store.get_session(sid).await.expect("get").expect("row");
    assert!(session.routing_policy.is_none());

    let audit = AuditSink::new_with_tenant(Arc::clone(&store), tenant.to_string());
    let classifier: Arc<dyn Transport> = Arc::new(AlwaysComplex {
        caps: CapabilitySet(Vec::new()),
    });
    let decision = argo_agent_runner::turn_routing::maybe_route_turn_with_transport(
        &store,
        &session,
        classifier,
        Some("hi"),
        &audit,
    )
    .await
    .expect("route");

    assert!(
        decision.is_none(),
        "sessions with no policy must skip tier routing"
    );
}
