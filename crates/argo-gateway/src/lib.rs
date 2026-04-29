//! `argo-gateway` — chat-platform adapters + multi-tenant routing
//! (§21).
//!
//! Three components:
//!
//! - [`Adapter`] trait — single seam between an external chat
//!   platform and Argo. Phase 6 ships [`SlackAdapter`],
//!   [`DiscordAdapter`], [`TelegramAdapter`] as stubs that declare
//!   the API surface and return clear `ArgoError::Provider` errors
//!   until the §21.4 24-hour-soak follow-up lands.
//! - [`GatewayRouter`] — resolves `(adapter, channel_id, sender_id)`
//!   into `(tenant_id, agent_name, principal, session_id)` per
//!   §21.3. Cross-platform conversation continuity: same
//!   `(tenant_id, agent_name, sender_id)` resumes the same session
//!   across adapters.
//! - [`format_rbac_denial`] / [`rbac_denial_text`] — turn an
//!   `ArgoError::RbacDenied` into a polite user-facing reply
//!   (§21.4 acceptance).
//!
//! ```
//! use argo_gateway::{GatewayConfig, GatewayRouter, InboundMessage};
//! use argo_types::{AgentName, TenantId};
//! let mut cfg = GatewayConfig::default();
//! cfg.default_tenant = Some(TenantId::new());
//! cfg.default_agent = Some(AgentName("alice".into()));
//! let router = GatewayRouter::new(cfg);
//! let m = InboundMessage::now("slack", "#general", "U1", "hi");
//! let r = router.resolve(&m).unwrap();
//! assert_eq!(r.agent_name.0, "alice");
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod adapter;
mod adapters;
mod router;

pub use adapter::{format_rbac_denial, rbac_denial_text, Adapter, InboundMessage, OutboundReply};
pub use adapters::{DiscordAdapter, SlackAdapter, TelegramAdapter};
pub use router::{GatewayConfig, GatewayRouter, Resolved, SessionCache};
