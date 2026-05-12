//! File-backed web inbox approval notification channel.
//!
//! In addition to the JSONL queue used by the HTML inbox, the module
//! exposes a process-global event hook ([`set_event_hook`]) so the
//! gateway's `/inbox/ws` route can fan out structured
//! `approval.{created,decided,undone,expired}` frames in lock-step
//! with state-machine transitions (J3). The hook is `Option<…>` and
//! defaults to a no-op, so callers that don't care (CLI sends,
//! background sweepers) pay nothing.

use crate::quiet_hours::QuietHours;
use crate::{
    quiet_hours_deferred_receipt, ChannelError, ChannelId, ChannelResult, DeliveryReceipt,
    DeliveryStatus, InboundEvent, Notification, NotifyChannel, RiskLevel,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;
use tokio::io::AsyncWriteExt;

/// Configuration for [`WebInboxChannel`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebInboxChannelConfig {
    /// Channel id.
    pub channel_id: ChannelId,
    /// JSONL queue read by the gateway's `/inbox` route.
    pub inbox_path: PathBuf,
    /// Gateway base URL operators open in a browser.
    pub gateway_base_url: String,
    /// Operator quiet hours. `None` means quiet hours are not configured
    /// for this channel — today's behavior preserved.
    #[serde(default)]
    pub quiet_hours: Option<QuietHours>,
}

impl WebInboxChannelConfig {
    /// Local test/development config.
    #[must_use]
    pub fn local(inbox_path: impl Into<PathBuf>) -> Self {
        Self {
            channel_id: ChannelId::new("webinbox"),
            inbox_path: inbox_path.into(),
            gateway_base_url: "http://127.0.0.1:8734".into(),
            quiet_hours: None,
        }
    }
}

/// Pending approval row stored in the web inbox queue.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WebInboxRow {
    /// Operator that may see and decide this approval.
    pub operator_id: String,
    /// Worker action/approval id.
    pub approval_id: String,
    /// Worker action id shown to the operator.
    pub action_id: String,
    /// Risk stamped by the manifest.
    pub risk_level: RiskLevel,
    /// Human-readable approval prompt.
    pub prompt_body: String,
    /// Signed approve URL.
    pub approve_url: String,
    /// Signed reject URL.
    pub reject_url: String,
    /// Approval expiry.
    pub expiry: DateTime<Utc>,
    /// Queue insertion time.
    pub enqueued_at: DateTime<Utc>,
}

/// Lifecycle event published when an approval row's state changes
/// (J3). Decoupled from the gateway's WS layer so the channels crate
/// has no upward dependency.
#[derive(Debug, Clone)]
pub enum WebInboxEvent {
    /// New approval row was appended to the inbox.
    ApprovalCreated {
        /// Per-tenant broadcast key. v1 uses the operator id.
        tenant_id: String,
        /// Created row.
        row: WebInboxRow,
    },
    /// Operator decided the approval (`approve` or `refuse`).
    ApprovalDecided {
        /// Per-tenant broadcast key.
        tenant_id: String,
        /// Updated row.
        row: WebInboxRow,
        /// Locked wire value: `"approve"` or `"refuse"`.
        decision: String,
    },
    /// Operator undid a recent decision (within the 30s window).
    ApprovalUndone {
        /// Per-tenant broadcast key.
        tenant_id: String,
        /// Reverted row.
        row: WebInboxRow,
    },
    /// Approval timed out without a decision.
    ApprovalExpired {
        /// Per-tenant broadcast key.
        tenant_id: String,
        /// Expired row.
        row: WebInboxRow,
    },
}

/// Process-global event hook. The gateway sets it on first WS
/// subscribe to fan events out to per-tenant broadcast channels.
type EventHook = Box<dyn Fn(WebInboxEvent) + Send + Sync + 'static>;

static EVENT_HOOK: OnceLock<EventHook> = OnceLock::new();

/// Register the global event hook. Idempotent — only the first
/// caller wins, subsequent calls are silently ignored so tests
/// re-using process state don't double-register.
pub fn set_event_hook(hook: EventHook) {
    let _ = EVENT_HOOK.set(hook);
}

/// Fire a lifecycle event through the registered hook (no-op when no
/// hook is set).
pub fn dispatch_event(event: WebInboxEvent) {
    if let Some(hook) = EVENT_HOOK.get() {
        hook(event);
    }
}

impl From<Notification> for WebInboxRow {
    fn from(n: Notification) -> Self {
        Self {
            operator_id: n.target.operator_id,
            approval_id: n.approval_id,
            action_id: n.action_id,
            risk_level: n.risk_level,
            prompt_body: n.prompt_body,
            approve_url: n.approve_url,
            reject_url: n.reject_url,
            expiry: n.expiry,
            enqueued_at: Utc::now(),
        }
    }
}

/// Pull-only approval channel read by the gateway web inbox.
pub struct WebInboxChannel {
    config: WebInboxChannelConfig,
}

impl WebInboxChannel {
    /// Create a web inbox channel from config.
    #[must_use]
    pub fn new(config: WebInboxChannelConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl NotifyChannel for WebInboxChannel {
    fn id(&self) -> ChannelId {
        self.config.channel_id.clone()
    }

    async fn send(&self, notification: Notification) -> ChannelResult<DeliveryReceipt> {
        if let Some(quiet) = self.config.quiet_hours.as_ref() {
            if quiet.contains(Utc::now()) {
                tracing::info!(
                    channel = %self.config.channel_id,
                    approval_id = %notification.approval_id,
                    "deferred-send: quiet_hours"
                );
                return Ok(quiet_hours_deferred_receipt(
                    self.id(),
                    notification.approval_id,
                ));
            }
        }
        if let Some(parent) = self.config.inbox_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| ChannelError::Provider(format!("create inbox dir: {e}")))?;
        }
        let row = WebInboxRow::from(notification);
        let mut line = serde_json::to_string(&row)
            .map_err(|e| ChannelError::Provider(format!("serialize inbox row: {e}")))?;
        line.push('\n');
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.inbox_path)
            .await
            .map_err(|e| ChannelError::Provider(format!("open inbox: {e}")))?;
        file.write_all(line.as_bytes())
            .await
            .map_err(|e| ChannelError::Provider(format!("write inbox: {e}")))?;
        file.flush()
            .await
            .map_err(|e| ChannelError::Provider(format!("flush inbox: {e}")))?;
        // v1 tenant key = operator id (single-tenant-per-operator
        // model). v1.5 replaces with a proper tenant_id once the
        // channels layer carries one end-to-end.
        dispatch_event(WebInboxEvent::ApprovalCreated {
            tenant_id: row.operator_id.clone(),
            row: row.clone(),
        });
        Ok(DeliveryReceipt {
            channel_id: self.id(),
            notification_id: row.approval_id,
            provider_message_id: None,
            status: DeliveryStatus::Sent,
            deferred_reason: None,
            delivered_at: Utc::now(),
        })
    }

    async fn poll(&self) -> ChannelResult<Vec<InboundEvent>> {
        Ok(Vec::new())
    }

    fn quiet_hours(&self) -> Option<&QuietHours> {
        self.config.quiet_hours.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OperatorTarget, RiskLevel};

    #[tokio::test]
    async fn send_appends_operator_scoped_row_without_provider_call() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("webinbox.jsonl");
        let channel = WebInboxChannel::new(WebInboxChannelConfig::local(&path));
        let expiry = Utc::now() + chrono::Duration::minutes(5);
        let receipt = channel
            .send(Notification {
                target: OperatorTarget {
                    operator_id: "op-1".into(),
                    email: "op@example.com".into(),
                },
                action_id: "act-1".into(),
                approval_id: "act-1".into(),
                risk_level: RiskLevel::High,
                prompt_body: "Approve listing?".into(),
                approve_url: "http://gw/approve?token=t".into(),
                reject_url: "http://gw/reject?token=t".into(),
                expiry,
            })
            .await
            .expect("send");
        assert_eq!(receipt.status, DeliveryStatus::Sent);
        let raw = std::fs::read_to_string(path).expect("read inbox");
        let row: WebInboxRow = serde_json::from_str(raw.trim()).expect("row");
        assert_eq!(row.operator_id, "op-1");
        assert_eq!(row.approval_id, "act-1");
    }
}
