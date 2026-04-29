//! Public surface: the [`Router`] trait + [`RoutingContext`] passed to
//! every routing decision.
//!
//! Spec: `docs/plan/argo-build-plan.md` §24.1.
use argo_types::{ArgoResult, Message, ProviderModel, RoutingDecision, RoutingPolicy, SessionId};
use async_trait::async_trait;
use std::time::Instant;

/// Per-turn input to a routing decision.
#[derive(Debug, Clone)]
pub struct RoutingContext<'a> {
    /// Owning session.
    pub session_id: SessionId,
    /// Turn number within the session.
    pub turn: u32,
    /// Conversation prefix at decision time.
    pub messages: &'a [Message],
    /// The user message that triggered this turn (used as the cache key
    /// component for retries).
    pub user_message: &'a str,
    /// Tools the agent loop is about to expose to the model.
    pub tools_in_scope: &'a [String],
    /// Model the agent loop would use absent any routing decision.
    pub default_model: ProviderModel,
    /// Policy in force for this session.
    pub policy: RoutingPolicy,
    /// Wall-clock deadline. If a router cannot return by this instant it
    /// MUST fall through to `default_model` and never block past the
    /// deadline.
    pub deadline: Instant,
}

/// The cost-tier routing seam (Amendment 1, §24.1).
///
/// ```
/// # use argo_routing::Router;
/// fn assert_object_safe(_r: &dyn Router) {}
/// ```
#[async_trait]
pub trait Router: Send + Sync {
    /// One routing decision. The returned [`RoutingDecision`] is also the
    /// row the agent loop persists when `policy ≠ Fixed` (§24.4).
    async fn route(&self, ctx: &RoutingContext<'_>) -> ArgoResult<RoutingDecision>;
}
