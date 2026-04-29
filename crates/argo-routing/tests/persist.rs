//! Integration test: §24.4 acceptance "all non-Fixed decisions persisted".
use argo_routing::{FixedRouter, PersistingRouter, Router, RoutingContext};
use argo_store::SessionStore;
use argo_types::{
    ArgoResult, ProviderModel, RoutingDecision, RoutingDecisionId, RoutingPolicy, SessionId,
    TenantId, WorkspaceId,
};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::{Duration, Instant};

struct StaticTieredRouter {
    tier: &'static str,
    chosen: ProviderModel,
}

#[async_trait]
impl Router for StaticTieredRouter {
    async fn route(&self, ctx: &RoutingContext<'_>) -> ArgoResult<RoutingDecision> {
        Ok(RoutingDecision {
            id: RoutingDecisionId::new(),
            session_id: ctx.session_id,
            turn: ctx.turn,
            policy: "tiered".into(),
            tier: self.tier.into(),
            classified_at: chrono::Utc::now(),
            classifier_latency_ms: 42,
            chosen_provider_model: self.chosen.clone(),
            reason: "static test router".into(),
        })
    }
}

async fn fresh_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    Arc::new(store)
}

async fn seed_session(store: &SessionStore) -> SessionId {
    let tenant = TenantId::new();
    let ws = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, 'test', '.', strftime('%s','now'))",
    )
    .bind(ws.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .unwrap();
    store
        .create_session(tenant, ws, "model", "provider", Some("test"))
        .await
        .unwrap()
}

fn ctx<'a>(session_id: SessionId, msg: &'a str, deadline: Instant) -> RoutingContext<'a> {
    RoutingContext {
        session_id,
        turn: 1,
        messages: &[],
        user_message: msg,
        tools_in_scope: &[],
        default_model: ProviderModel {
            provider: "openai".into(),
            model: "gpt-4o-mini".into(),
        },
        policy: RoutingPolicy::Fixed,
        deadline,
    }
}

#[tokio::test]
async fn persists_tiered_decision_to_routing_decision_table() {
    let store = fresh_store().await;
    let session_id = seed_session(&store).await;
    let inner = StaticTieredRouter {
        tier: "complex",
        chosen: ProviderModel {
            provider: "anthropic".into(),
            model: "claude-opus-4-6".into(),
        },
    };
    let router = PersistingRouter::new(inner, store.clone());
    let _ = router
        .route(&ctx(
            session_id,
            "do the thing",
            Instant::now() + Duration::from_secs(60),
        ))
        .await
        .unwrap();
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM routing_decision WHERE session_id = ?")
        .bind(session_id.0.as_bytes().as_slice())
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(n, 1);
}

#[tokio::test]
async fn fixed_decisions_are_not_persisted() {
    let store = fresh_store().await;
    let session_id = seed_session(&store).await;
    let router = PersistingRouter::new(FixedRouter::new(), store.clone());
    let _ = router
        .route(&ctx(
            session_id,
            "anything",
            Instant::now() + Duration::from_secs(60),
        ))
        .await
        .unwrap();
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM routing_decision WHERE session_id = ?")
        .bind(session_id.0.as_bytes().as_slice())
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(n, 0);
}

#[tokio::test]
async fn persisted_row_carries_full_decision_fields() {
    let store = fresh_store().await;
    let session_id = seed_session(&store).await;
    let inner = StaticTieredRouter {
        tier: "medium",
        chosen: ProviderModel {
            provider: "openai".into(),
            model: "gpt-4o".into(),
        },
    };
    let router = PersistingRouter::new(inner, store.clone());
    let _ = router
        .route(&ctx(
            session_id,
            "review",
            Instant::now() + Duration::from_secs(60),
        ))
        .await
        .unwrap();
    let (policy, tier, prov, model, latency): (String, String, String, String, i64) =
        sqlx::query_as(
            "SELECT policy, tier, chosen_provider, chosen_model, classifier_latency_ms \
             FROM routing_decision WHERE session_id = ?",
        )
        .bind(session_id.0.as_bytes().as_slice())
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(policy, "tiered");
    assert_eq!(tier, "medium");
    assert_eq!(prov, "openai");
    assert_eq!(model, "gpt-4o");
    assert_eq!(latency, 42);
}
