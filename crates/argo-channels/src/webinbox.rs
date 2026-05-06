//! File-backed web inbox approval notification channel.

use crate::{
    ChannelError, ChannelId, ChannelResult, DeliveryReceipt, DeliveryStatus, InboundEvent,
    Notification, NotifyChannel, RiskLevel,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
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
}

impl WebInboxChannelConfig {
    /// Local test/development config.
    #[must_use]
    pub fn local(inbox_path: impl Into<PathBuf>) -> Self {
        Self {
            channel_id: ChannelId::new("webinbox"),
            inbox_path: inbox_path.into(),
            gateway_base_url: "http://127.0.0.1:8734".into(),
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
