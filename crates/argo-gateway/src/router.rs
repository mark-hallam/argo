//! Multi-tenant + multi-agent gateway routing (§21.3).
//!
//! For each inbound message, the router resolves:
//!
//! 1. **Tenant** — via the `[gateway.tenant_routing.<adapter>]`
//!    table (channel_id → TenantId). Falls back to the default
//!    tenant when no row matches.
//! 2. **Agent name** — via `[gateway.agent_routing.<adapter>]`
//!    (channel_id → AgentName). Falls back to the configured
//!    default agent.
//! 3. **Principal** — `Principal::Gateway { adapter, identity = sender_id }`
//!    so RBAC can pin per-sender capabilities.
//! 4. **Session id** — looked up by `(tenant_id, agent_name, sender_id)`
//!    so the same human gets the same conversation across
//!    Slack ↔ Discord ↔ Telegram (§21.3 cross-platform continuity).
use crate::adapter::InboundMessage;
use argo_types::{AgentName, ArgoError, ArgoResult, Principal, SessionId, TenantId};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Gateway routing config — typically loaded from `argo.toml`'s
/// `[gateway]` section.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// Per-adapter tenant routing tables: `channel_id → TenantId`.
    #[serde(default)]
    pub tenant_routing: HashMap<String, HashMap<String, TenantId>>,
    /// Per-adapter agent routing tables: `channel_id → AgentName`.
    #[serde(default)]
    pub agent_routing: HashMap<String, HashMap<String, AgentName>>,
    /// Default tenant when no `tenant_routing` row matches.
    #[serde(default)]
    pub default_tenant: Option<TenantId>,
    /// Default agent name when no `agent_routing` row matches.
    #[serde(default)]
    pub default_agent: Option<AgentName>,
}

/// Resolved routing for one inbound message.
#[derive(Debug, Clone)]
pub struct Resolved {
    /// Owning tenant.
    pub tenant_id: TenantId,
    /// Named agent within the tenant.
    pub agent_name: AgentName,
    /// Acting principal — `Gateway { adapter, identity = sender_id }`.
    pub principal: Principal,
    /// Persistent session id for `(tenant_id, agent_name, sender_id)`.
    pub session_id: SessionId,
}

/// In-memory session cache keyed by `(tenant_id, agent_name, sender_id)`.
/// The real implementation will persist this in `argo-store`; the
/// in-memory version exercises the §21.3 "same sender → same session
/// across adapters" invariant in tests.
#[derive(Debug, Default)]
pub struct SessionCache {
    inner: RwLock<HashMap<SessionKey, SessionId>>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct SessionKey {
    tenant_id: TenantId,
    agent_name: AgentName,
    sender_id: String,
}

impl SessionCache {
    /// New empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get-or-create a session for the given key.
    pub fn resolve(
        &self,
        tenant_id: TenantId,
        agent_name: &AgentName,
        sender_id: &str,
    ) -> SessionId {
        let key = SessionKey {
            tenant_id,
            agent_name: agent_name.clone(),
            sender_id: sender_id.to_string(),
        };
        if let Some(id) = self.inner.read().get(&key) {
            return *id;
        }
        let mut g = self.inner.write();
        if let Some(id) = g.get(&key) {
            return *id;
        }
        let id = SessionId::new();
        g.insert(key, id);
        id
    }
}

/// The router itself.
#[derive(Debug)]
pub struct GatewayRouter {
    config: GatewayConfig,
    sessions: SessionCache,
}

impl GatewayRouter {
    /// Build a router with the given config and a fresh in-memory
    /// session cache.
    #[must_use]
    pub fn new(config: GatewayConfig) -> Self {
        Self {
            config,
            sessions: SessionCache::new(),
        }
    }

    /// Borrow the config (for tests + telemetry).
    #[must_use]
    pub fn config(&self) -> &GatewayConfig {
        &self.config
    }

    /// Resolve one inbound message into a `Resolved` envelope.
    /// Errors when neither `tenant_routing` nor `default_tenant`
    /// matches — that's a configuration bug, not a runtime fault.
    pub fn resolve(&self, msg: &InboundMessage) -> ArgoResult<Resolved> {
        let tenant_id = self
            .config
            .tenant_routing
            .get(&msg.adapter)
            .and_then(|t| t.get(&msg.channel_id))
            .copied()
            .or(self.config.default_tenant)
            .ok_or_else(|| {
                ArgoError::Schema(format!(
                    "gateway: no tenant routing matched (adapter={}, channel_id={}); set default_tenant or add a routing row",
                    msg.adapter, msg.channel_id
                ))
            })?;
        let agent_name = self
            .config
            .agent_routing
            .get(&msg.adapter)
            .and_then(|t| t.get(&msg.channel_id))
            .cloned()
            .or_else(|| self.config.default_agent.clone())
            .ok_or_else(|| {
                ArgoError::Schema(format!(
                    "gateway: no agent routing matched (adapter={}, channel_id={}); set default_agent or add a routing row",
                    msg.adapter, msg.channel_id
                ))
            })?;
        let principal = Principal::Gateway {
            adapter: msg.adapter.clone(),
            identity: msg.sender_id.clone(),
        };
        let session_id = self
            .sessions
            .resolve(tenant_id, &agent_name, &msg.sender_id);
        Ok(Resolved {
            tenant_id,
            agent_name,
            principal,
            session_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_with_defaults() -> GatewayConfig {
        GatewayConfig {
            default_tenant: Some(TenantId::new()),
            default_agent: Some(AgentName("alice".into())),
            ..Default::default()
        }
    }

    #[test]
    fn defaults_apply_when_no_row_matches() {
        let cfg = cfg_with_defaults();
        let want_tenant = cfg.default_tenant.unwrap();
        let router = GatewayRouter::new(cfg);
        let msg = InboundMessage::now("slack", "#general", "U1", "hi");
        let r = router.resolve(&msg).unwrap();
        assert_eq!(r.tenant_id, want_tenant);
        assert_eq!(r.agent_name.0, "alice");
        match r.principal {
            Principal::Gateway { adapter, identity } => {
                assert_eq!(adapter, "slack");
                assert_eq!(identity, "U1");
            }
            _ => panic!("expected Gateway principal"),
        }
    }

    #[test]
    fn explicit_row_beats_default() {
        let other_tenant = TenantId::new();
        let mut cfg = cfg_with_defaults();
        cfg.tenant_routing
            .entry("slack".into())
            .or_default()
            .insert("#secret".into(), other_tenant);
        cfg.agent_routing
            .entry("slack".into())
            .or_default()
            .insert("#secret".into(), AgentName("ops".into()));
        let router = GatewayRouter::new(cfg);
        let msg = InboundMessage::now("slack", "#secret", "U1", "hi");
        let r = router.resolve(&msg).unwrap();
        assert_eq!(r.tenant_id, other_tenant);
        assert_eq!(r.agent_name.0, "ops");
    }

    #[test]
    fn missing_tenant_with_no_default_is_schema_error() {
        let router = GatewayRouter::new(GatewayConfig::default());
        let msg = InboundMessage::now("slack", "#x", "U1", "hi");
        let err = router.resolve(&msg).unwrap_err();
        assert!(matches!(err, ArgoError::Schema(_)));
    }

    #[test]
    fn missing_agent_with_no_default_is_schema_error() {
        let mut cfg = GatewayConfig {
            default_tenant: Some(TenantId::new()),
            ..Default::default()
        };
        cfg.default_agent = None;
        let router = GatewayRouter::new(cfg);
        let msg = InboundMessage::now("slack", "#x", "U1", "hi");
        let err = router.resolve(&msg).unwrap_err();
        assert!(matches!(err, ArgoError::Schema(_)));
    }

    #[test]
    fn same_sender_resumes_session_across_adapters() {
        // §21.3 acceptance — same (tenant, agent, sender) keeps the
        // same SessionId whether the message arrives via Slack or
        // Telegram.
        let cfg = cfg_with_defaults();
        let router = GatewayRouter::new(cfg);
        let m1 = InboundMessage::now("slack", "#general", "U1", "hi");
        let m2 = InboundMessage::now("telegram", "@bob", "U1", "follow-up");
        let r1 = router.resolve(&m1).unwrap();
        let r2 = router.resolve(&m2).unwrap();
        assert_eq!(r1.session_id, r2.session_id);
    }

    #[test]
    fn different_senders_get_different_sessions() {
        let cfg = cfg_with_defaults();
        let router = GatewayRouter::new(cfg);
        let m1 = InboundMessage::now("slack", "#general", "U1", "hi");
        let m2 = InboundMessage::now("slack", "#general", "U2", "hi");
        let r1 = router.resolve(&m1).unwrap();
        let r2 = router.resolve(&m2).unwrap();
        assert_ne!(r1.session_id, r2.session_id);
    }

    #[test]
    fn multi_agent_routing_routes_same_channel_to_different_agents() {
        // §21.4 acceptance: same channel routed to different agents
        // based on `[gateway.agent_routing]`.
        let mut cfg = cfg_with_defaults();
        cfg.agent_routing
            .entry("slack".into())
            .or_default()
            .insert("#dev".into(), AgentName("eng-bot".into()));
        cfg.agent_routing
            .entry("slack".into())
            .or_default()
            .insert("#design".into(), AgentName("design-bot".into()));
        let router = GatewayRouter::new(cfg);
        let m1 = InboundMessage::now("slack", "#dev", "U1", "hi");
        let m2 = InboundMessage::now("slack", "#design", "U1", "hi");
        let r1 = router.resolve(&m1).unwrap();
        let r2 = router.resolve(&m2).unwrap();
        assert_eq!(r1.agent_name.0, "eng-bot");
        assert_eq!(r2.agent_name.0, "design-bot");
        // …and same sender across two agents → DIFFERENT sessions
        // because (tenant, agent_name, sender) is the key.
        assert_ne!(r1.session_id, r2.session_id);
    }
}
