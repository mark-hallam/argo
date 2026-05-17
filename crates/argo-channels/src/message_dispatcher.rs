//! Production [`MessageDispatcher`] backing the `send_message` tool
//! (`mission-send-message-production-dispatcher`).
//!
//! The default `argo_tool::SendMessage` dispatcher returns
//! `"send_message backend not wired"` at runtime, so the tool ships in
//! the schema but errors at dispatch time. This module replaces that
//! default with a routing dispatcher that:
//!
//! 1. Routes the call by channel name to an [`OutboundChannel`] impl
//!    registered at construction time.
//! 2. Short-circuits an in-window replay via the
//!    `outbound_message` table (`body_hash` keyed, 5-minute default
//!    window).
//! 3. Mirrors the dispatch into the audit chain (`ToolCall` on
//!    success, `RbacDeny` on a missing-channel rejection) so the
//!    operator's audit-feed sees one row per outbound message.
//!
//! ## Sensible deviations from the brief
//!
//! - **`ChannelRegistry` does not exist.** The brief assumed an
//!   existing primitive in `argo-channels`; the crate exposes
//!   `NotifyChannel`, which delivers approval `Notification`s (not
//!   raw text). This module introduces [`OutboundChannel`] — a small
//!   sibling trait scoped to raw-text outbound sends — and stores
//!   registered impls in a `HashMap<String, Arc<dyn OutboundChannel>>`.
//! - **`AuditKind::MessageSent` / `AuditKind::CapabilityDenied` do
//!   not exist.** The brief named two new variants; adding them
//!   would mutate `argo-audit` (outside this brief's may-touch).
//!   Successful sends emit `AuditKind::ToolCall` with
//!   `event_type = "message_sent"`; refusals emit
//!   `AuditKind::RbacDeny` with `event_type = "message_send_denied"`.
//! - **Capability gate.** The cap check
//!   (`Capability::MessageSend { channel, recipient_scope }`) runs
//!   upstream inside `ToolRegistry::dispatch` before the dispatcher
//!   is reached — the dispatcher's `RbacDeny` audit row covers the
//!   *transport*-level "no such channel registered" rejection
//!   instead, which the upstream cap check cannot anticipate.

use crate::ChannelId;
use argo_audit::{AuditKind, AuditSink};
use argo_store::outbound_message_query;
use argo_store::SessionStore;
use argo_tool::{MessageDispatcher, SendOutcome};
use argo_types::TenantId;
use async_trait::async_trait;
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

/// Default window in which a duplicate `(tenant, body_hash)` send
/// short-circuits to the cached `SendOutcome`.
const DEFAULT_IDEMPOTENCY_WINDOW_SECS: i64 = 300;

/// Raw-text outbound delivery trait, scoped to the production
/// [`ProductionMessageDispatcher`].
///
/// `NotifyChannel::send` takes a structured approval
/// [`crate::Notification`] (the approval-flow primitive); production
/// `send_message` calls do not have approval URLs, expiry, or a risk
/// level to attach. Implementations of this trait wrap the
/// per-transport "post arbitrary text" path (e.g. Slack
/// `chat.postMessage`, Postmark `messages`, BlueBubbles
/// `chat/sendMessage`) without going through the approval shape.
#[async_trait]
pub trait OutboundChannel: Send + Sync + std::fmt::Debug {
    /// Stable channel id (matches the `channel` argument the
    /// `send_message` tool routes by — `slack` / `email` / `sms` /
    /// `imessage` today).
    fn id(&self) -> ChannelId;

    /// Deliver `body` (plus optional `attachments`) to `recipient`.
    /// On success, return the channel-assigned message id.
    async fn send_text(
        &self,
        recipient: &str,
        body: &str,
        attachments: &[PathBuf],
    ) -> Result<String, String>;
}

/// Routing dispatcher backed by [`OutboundChannel`] implementations,
/// an [`AuditSink`] for the audit-chain mirror, and a
/// [`SessionStore`] for `outbound_message` idempotency.
pub struct ProductionMessageDispatcher {
    channels: HashMap<String, Arc<dyn OutboundChannel>>,
    audit: Arc<AuditSink>,
    store: Arc<SessionStore>,
    tenant_id: TenantId,
    idempotency_window_secs: i64,
}

impl std::fmt::Debug for ProductionMessageDispatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProductionMessageDispatcher")
            .field("channels", &self.channels.keys().collect::<Vec<_>>())
            .field("tenant_id", &self.tenant_id)
            .field("idempotency_window_secs", &self.idempotency_window_secs)
            .finish()
    }
}

impl ProductionMessageDispatcher {
    /// Build the dispatcher with no channels registered. Callers
    /// chain [`Self::with_channel`] to add transports before handing
    /// the dispatcher to
    /// [`argo_tool::tools::SendMessage::with_dispatcher`].
    #[must_use]
    pub fn new(audit: Arc<AuditSink>, store: Arc<SessionStore>, tenant_id: TenantId) -> Self {
        Self {
            channels: HashMap::new(),
            audit,
            store,
            tenant_id,
            idempotency_window_secs: DEFAULT_IDEMPOTENCY_WINDOW_SECS,
        }
    }

    /// Register an [`OutboundChannel`] under its
    /// [`OutboundChannel::id`].
    #[must_use]
    pub fn with_channel(mut self, channel: Arc<dyn OutboundChannel>) -> Self {
        self.channels.insert(channel.id().to_string(), channel);
        self
    }

    /// Override the idempotency window (defaults to 5 minutes). The
    /// dispatcher returns a cached `SendOutcome` when a `(tenant,
    /// body_hash)` row appears within `now - window`.
    #[must_use]
    pub fn with_idempotency_window_secs(mut self, secs: i64) -> Self {
        self.idempotency_window_secs = secs;
        self
    }
}

#[async_trait]
impl MessageDispatcher for ProductionMessageDispatcher {
    async fn dispatch(
        &self,
        channel: &str,
        recipient: &str,
        body: &str,
        attachments: &[PathBuf],
    ) -> Result<SendOutcome, String> {
        let body_hash = hash_body(channel, recipient, body, attachments);
        let now_secs = Utc::now().timestamp();
        let since = now_secs.saturating_sub(self.idempotency_window_secs);
        if let Some(prior) =
            outbound_message_query::lookup_recent_by_body_hash(
                &self.store,
                self.tenant_id,
                &body_hash,
                since,
            )
            .await
            .map_err(|e| format!("idempotency lookup failed: {e}"))?
        {
            if let Ok(outcome) = serde_json::from_str::<CachedOutcome>(&prior.outcome_json) {
                return Ok(SendOutcome {
                    message_id: prior.message_id,
                    sent_at: outcome.sent_at,
                });
            }
        }
        let Some(transport) = self.channels.get(channel) else {
            let payload = serde_json::json!({
                "event_type": "message_send_denied",
                "reason": "no_outbound_channel_registered",
                "channel": channel,
                "recipient": recipient,
                "registered": self.channels.keys().collect::<Vec<_>>(),
            });
            let _ = self
                .audit
                .write(None, AuditKind::RbacDeny, payload, None)
                .await;
            return Err(format!(
                "no outbound channel registered for '{channel}' \
                 (registered: {:?})",
                self.channels.keys().collect::<Vec<_>>()
            ));
        };
        let provider_id = match transport.send_text(recipient, body, attachments).await {
            Ok(id) => id,
            Err(e) => {
                let payload = serde_json::json!({
                    "event_type": "message_send_failed",
                    "channel": channel,
                    "recipient": recipient,
                    "error": e,
                });
                let _ = self
                    .audit
                    .write(None, AuditKind::ToolCall, payload, None)
                    .await;
                return Err(e);
            }
        };
        let message_id = if provider_id.is_empty() {
            Uuid::now_v7().to_string()
        } else {
            provider_id
        };
        let sent_at = Utc::now();
        let outcome = SendOutcome {
            message_id: message_id.clone(),
            sent_at,
        };
        let cached = CachedOutcome { sent_at };
        let outcome_json = serde_json::to_string(&cached).unwrap_or_else(|_| "{}".to_string());
        if let Err(e) = outbound_message_query::insert_outbound_message(
            &self.store,
            &message_id,
            self.tenant_id,
            channel,
            recipient,
            &body_hash,
            sent_at.timestamp(),
            &outcome_json,
        )
        .await
        {
            // Idempotency-table write failure is best-effort —
            // surfacing it would lose the already-sent message;
            // log via audit instead and let the caller proceed.
            tracing::warn!(
                target: "channels::message_dispatcher",
                error = %e,
                channel,
                "outbound_message insert failed; idempotency check disabled for this send",
            );
        }
        let payload = serde_json::json!({
            "event_type": "message_sent",
            "channel": channel,
            "recipient": recipient,
            "message_id": message_id,
            "sent_at": sent_at.to_rfc3339(),
        });
        let _ = self
            .audit
            .write(None, AuditKind::ToolCall, payload, None)
            .await;
        Ok(outcome)
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct CachedOutcome {
    sent_at: chrono::DateTime<Utc>,
}

fn hash_body(channel: &str, recipient: &str, body: &str, attachments: &[PathBuf]) -> String {
    let mut h = Sha256::new();
    h.update(channel.as_bytes());
    h.update(b"\0");
    h.update(recipient.as_bytes());
    h.update(b"\0");
    h.update(body.as_bytes());
    h.update(b"\0");
    for a in attachments {
        h.update(a.as_os_str().as_encoded_bytes());
        h.update(b"\0");
    }
    hex_encode(&h.finalize())
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
#[path = "message_dispatcher_tests.rs"]
mod tests;
