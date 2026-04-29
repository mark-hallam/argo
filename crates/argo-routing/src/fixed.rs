//! Zero-overhead router: always returns the configured default model
//! (§24.2).
use crate::router::{Router, RoutingContext};
use argo_types::{ArgoResult, RoutingDecision, RoutingDecisionId};
use async_trait::async_trait;
use chrono::Utc;

/// Default model wins; no classifier call, no persistence.
///
/// ```
/// # use argo_routing::FixedRouter;
/// let _r = FixedRouter::new();
/// ```
#[derive(Debug, Default, Clone, Copy)]
pub struct FixedRouter;

impl FixedRouter {
    /// New zero-config router.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Router for FixedRouter {
    async fn route(&self, ctx: &RoutingContext<'_>) -> ArgoResult<RoutingDecision> {
        Ok(RoutingDecision {
            id: RoutingDecisionId::new(),
            session_id: ctx.session_id,
            turn: ctx.turn,
            policy: "fixed".into(),
            tier: "unrouted".into(),
            classified_at: Utc::now(),
            classifier_latency_ms: 0,
            chosen_provider_model: ctx.default_model.clone(),
            reason: "fixed router — default model".into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_types::{ProviderModel, RoutingPolicy, SessionId};

    #[test]
    fn returns_default_model_with_unrouted_tier() {
        let router = FixedRouter::new();
        let default_model = ProviderModel {
            provider: "openai".into(),
            model: "gpt-4o".into(),
        };
        let ctx = RoutingContext {
            session_id: SessionId::new(),
            turn: 1,
            messages: &[],
            user_message: "hi",
            tools_in_scope: &[],
            default_model: default_model.clone(),
            policy: RoutingPolicy::Fixed,
            deadline: std::time::Instant::now() + std::time::Duration::from_secs(60),
        };
        let d = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(router.route(&ctx))
            .unwrap();
        assert_eq!(d.policy, "fixed");
        assert_eq!(d.tier, "unrouted");
        assert_eq!(d.chosen_provider_model, default_model);
        assert_eq!(d.classifier_latency_ms, 0);
    }
}
