//! Slack adapter — Phase-6 stub.
use crate::adapter::{Adapter, InboundMessage, OutboundReply};
use argo_types::{ArgoError, ArgoResult};
use async_trait::async_trait;

/// Slack adapter — Phase-6 stub. Real OAuth + Events-API integration
/// lands with the §21.4 24-hour-soak PR.
#[derive(Debug, Clone)]
pub struct SlackAdapter {
    /// Bot token (xoxb-…).
    pub bot_token: String,
}

impl SlackAdapter {
    /// New stub bound to a Slack bot token.
    #[must_use]
    pub fn new(bot_token: impl Into<String>) -> Self {
        Self {
            bot_token: bot_token.into(),
        }
    }
}

#[async_trait]
impl Adapter for SlackAdapter {
    fn id(&self) -> &str {
        "slack"
    }
    async fn take_inbound(&self) -> ArgoResult<Option<InboundMessage>> {
        Err(ArgoError::Provider(
            "slack adapter not yet wired — needs OAuth + Events API integration (§21.4 follow-up)"
                .into(),
        ))
    }
    async fn reply(&self, _reply: OutboundReply) -> ArgoResult<()> {
        Err(ArgoError::Provider(
            "slack adapter not yet wired — `chat.postMessage` lands with §21.4 follow-up".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_id_is_slack() {
        assert_eq!(SlackAdapter::new("xoxb-test").id(), "slack");
    }

    #[tokio::test]
    async fn stubs_return_clear_provider_errors() {
        let a = SlackAdapter::new("xoxb-test");
        assert!(matches!(
            a.take_inbound().await,
            Err(ArgoError::Provider(_))
        ));
        let reply = OutboundReply {
            channel_id: "#general".into(),
            thread_id: None,
            text: "hi".into(),
        };
        assert!(matches!(a.reply(reply).await, Err(ArgoError::Provider(_))));
    }
}
