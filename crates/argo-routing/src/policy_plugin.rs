//! Plugin-delegated routing (§24.2). The real implementation rides the
//! plugin protocol (Phase 4 — Agents I/J); this stub keeps the type slot
//! reserved so config and store layers compile against the final shape.
use crate::router::{Router, RoutingContext};
use argo_types::{ArgoError, ArgoResult, RoutingDecision};
use async_trait::async_trait;

/// Phase-2 stub. Returns `ArgoError::RoutingFailed` so the agent loop can
/// detect mis-configuration and fall back to [`crate::FixedRouter`] until
/// the plugin host (§11) lands.
#[derive(Debug, Clone)]
pub struct PolicyPluginRouter {
    /// Plugin name the host should dispatch to.
    pub plugin: String,
}

impl PolicyPluginRouter {
    /// New stub bound to `plugin`.
    #[must_use]
    pub fn new(plugin: impl Into<String>) -> Self {
        Self {
            plugin: plugin.into(),
        }
    }
}

#[async_trait]
impl Router for PolicyPluginRouter {
    async fn route(&self, _ctx: &RoutingContext<'_>) -> ArgoResult<RoutingDecision> {
        Err(ArgoError::RoutingFailed {
            reason: format!(
                "PolicyPluginRouter({}) requires the Phase 4 plugin host (Agents I/J)",
                self.plugin
            ),
        })
    }
}
