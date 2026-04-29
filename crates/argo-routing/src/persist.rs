//! Wrapper that persists every non-Fixed [`RoutingDecision`] to the
//! `routing_decision` table (§24.4 acceptance).
use crate::router::{Router, RoutingContext};
use argo_store::SessionStore;
use argo_types::{ArgoResult, RoutingDecision};
use async_trait::async_trait;
use std::sync::Arc;

/// Decorate any [`Router`] with persistence: the inner router classifies,
/// the wrapper writes the row when `decision.policy != "fixed"`.
///
/// The agent loop (§6, Agent F Phase 1) wires this around its configured
/// `Router` so callers don't have to remember to persist.
pub struct PersistingRouter<R: Router> {
    inner: R,
    store: Arc<SessionStore>,
}

impl<R: Router> PersistingRouter<R> {
    /// Wrap `inner` so its decisions are persisted via `store`.
    pub fn new(inner: R, store: Arc<SessionStore>) -> Self {
        Self { inner, store }
    }
}

#[async_trait]
impl<R: Router> Router for PersistingRouter<R> {
    async fn route(&self, ctx: &RoutingContext<'_>) -> ArgoResult<RoutingDecision> {
        let decision = self.inner.route(ctx).await?;
        if decision.policy != "fixed" {
            self.store.persist_routing_decision(&decision).await?;
        }
        Ok(decision)
    }
}

impl<R: Router + std::fmt::Debug> std::fmt::Debug for PersistingRouter<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PersistingRouter")
            .field("inner", &self.inner)
            .finish()
    }
}
