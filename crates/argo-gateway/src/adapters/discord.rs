//! Discord adapter — Phase-6 stub.
use crate::adapter::{Adapter, InboundMessage, OutboundReply};
use argo_types::{ArgoError, ArgoResult};
use async_trait::async_trait;

/// Discord adapter — Phase-6 stub. Real Gateway-API integration
/// lands with the §21.4 follow-up.
#[derive(Debug, Clone)]
pub struct DiscordAdapter {
    /// Bot token.
    pub bot_token: String,
}

impl DiscordAdapter {
    /// New stub bound to a Discord bot token.
    #[must_use]
    pub fn new(bot_token: impl Into<String>) -> Self {
        Self {
            bot_token: bot_token.into(),
        }
    }
}

#[async_trait]
impl Adapter for DiscordAdapter {
    fn id(&self) -> &str {
        "discord"
    }
    async fn take_inbound(&self) -> ArgoResult<Option<InboundMessage>> {
        Err(ArgoError::Provider(
            "discord adapter not yet wired — needs Gateway-API integration (§21.4 follow-up)"
                .into(),
        ))
    }
    async fn reply(&self, _reply: OutboundReply) -> ArgoResult<()> {
        Err(ArgoError::Provider(
            "discord adapter not yet wired — message-send lands with §21.4 follow-up".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_id_is_discord() {
        assert_eq!(DiscordAdapter::new("test").id(), "discord");
    }

    #[tokio::test]
    async fn stubs_return_clear_provider_errors() {
        let a = DiscordAdapter::new("test");
        assert!(matches!(
            a.take_inbound().await,
            Err(ArgoError::Provider(_))
        ));
    }
}
