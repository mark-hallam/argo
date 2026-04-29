//! `Adapter` trait — single seam between an external chat platform
//! (Slack / Discord / Telegram / …) and Argo's gateway router.
//!
//! Phase-6 ship: trait + the typed message + reply shapes. Concrete
//! adapter implementations stub their actual API calls until the
//! soak-test PR lands.
use argo_types::{ArgoError, ArgoResult};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// One inbound message from an adapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundMessage {
    /// Adapter id (e.g. `"slack"`, `"discord"`, `"telegram"`).
    pub adapter: String,
    /// Platform-specific channel/room id.
    pub channel_id: String,
    /// Platform-specific sender id (Slack user, Discord user, Telegram user).
    pub sender_id: String,
    /// Optional thread / reply context.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
    /// Message text.
    pub text: String,
    /// Wall-clock arrival time.
    pub timestamp: DateTime<Utc>,
}

impl InboundMessage {
    /// Convenience constructor with `timestamp = now`.
    #[must_use]
    pub fn now(
        adapter: impl Into<String>,
        channel_id: impl Into<String>,
        sender_id: impl Into<String>,
        text: impl Into<String>,
    ) -> Self {
        Self {
            adapter: adapter.into(),
            channel_id: channel_id.into(),
            sender_id: sender_id.into(),
            thread_id: None,
            text: text.into(),
            timestamp: Utc::now(),
        }
    }
}

/// One outbound reply.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundReply {
    /// Channel to reply on.
    pub channel_id: String,
    /// Thread to reply within (some adapters thread, some don't).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
    /// Reply text.
    pub text: String,
}

/// The adapter seam.
///
/// Each adapter polls / subscribes to its platform and surfaces
/// messages via `take_inbound`. The router pulls one message at a
/// time, routes it through Argo, and writes the reply via `reply`.
///
/// ```
/// # use argo_gateway::Adapter;
/// fn assert_object_safe(_a: &dyn Adapter) {}
/// ```
#[async_trait]
pub trait Adapter: Send + Sync + std::fmt::Debug {
    /// Stable id (`"slack"` / `"discord"` / `"telegram"`).
    fn id(&self) -> &str;

    /// Pull the next inbound message. `None` when the adapter is
    /// idle (no messages right now); the router treats `None` as
    /// "try again later". `Err` is fatal for that adapter.
    async fn take_inbound(&self) -> ArgoResult<Option<InboundMessage>>;

    /// Send a reply. Errors are not fatal — the router logs and
    /// continues.
    async fn reply(&self, reply: OutboundReply) -> ArgoResult<()>;
}

/// Format a polite RBAC denial for end-users (Slack / Discord look
/// the same; Telegram strips Markdown). Used by the router when
/// `Agent::run_turn` returns `ArgoError::RbacDenied`.
#[must_use]
pub fn format_rbac_denial(role: &str, capability_summary: &str) -> String {
    format!(
        ":lock: That action needs a higher role.\n\
         Your role: `{role}`\n\
         Required capability: `{capability_summary}`\n\
         Ask an admin to grant the right role, then retry."
    )
}

/// Tiny helper — turn an `ArgoError::RbacDenied` into a polite
/// reply text for the user. Returns `None` for non-RBAC errors so
/// the caller can render those differently.
#[must_use]
pub fn rbac_denial_text(err: &ArgoError) -> Option<String> {
    if let ArgoError::RbacDenied { role, cap } = err {
        Some(format_rbac_denial(role, &format!("{cap:?}")))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_types::Capability;

    #[test]
    fn rbac_denial_formatter_includes_role_and_cap() {
        let s = format_rbac_denial("guest", "ToolDispatch{terminal}");
        assert!(s.contains("guest"));
        assert!(s.contains("terminal"));
        assert!(s.contains(":lock:"));
    }

    #[test]
    fn rbac_denial_text_returns_none_for_non_rbac_errors() {
        let e = ArgoError::Provider("network".into());
        assert!(rbac_denial_text(&e).is_none());
    }

    #[test]
    fn rbac_denial_text_returns_some_for_rbac_error() {
        let e = ArgoError::RbacDenied {
            role: "guest".into(),
            cap: Capability::ToolDispatch {
                tool_name_glob: "terminal".into(),
            },
        };
        let s = rbac_denial_text(&e).unwrap();
        assert!(s.contains("guest"));
    }

    #[test]
    fn inbound_message_now_populates_timestamp() {
        let m = InboundMessage::now("slack", "#general", "U123", "hi");
        assert_eq!(m.adapter, "slack");
        assert!(m.timestamp <= Utc::now());
    }
}
