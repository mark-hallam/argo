//! Telegram adapter — Phase-6 stub.
use crate::adapter::{Adapter, InboundMessage, OutboundReply};
use argo_types::{ArgoError, ArgoResult};
use async_trait::async_trait;

/// Telegram adapter — Phase-6 stub. Real Bot-API long-poll /
/// webhook integration lands with the §21.4 follow-up.
#[derive(Debug, Clone)]
pub struct TelegramAdapter {
    /// Bot token (e.g. `123456:ABC-DEF…`).
    pub bot_token: String,
}

impl TelegramAdapter {
    /// New stub bound to a Telegram bot token.
    #[must_use]
    pub fn new(bot_token: impl Into<String>) -> Self {
        Self {
            bot_token: bot_token.into(),
        }
    }
}

#[async_trait]
impl Adapter for TelegramAdapter {
    fn id(&self) -> &str {
        "telegram"
    }
    async fn take_inbound(&self) -> ArgoResult<Option<InboundMessage>> {
        Err(ArgoError::Provider(
            "telegram adapter not yet wired — needs Bot-API long-poll / webhook (§21.4 follow-up)"
                .into(),
        ))
    }
    async fn reply(&self, _reply: OutboundReply) -> ArgoResult<()> {
        Err(ArgoError::Provider(
            "telegram adapter not yet wired — `sendMessage` lands with §21.4 follow-up".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_id_is_telegram() {
        assert_eq!(TelegramAdapter::new("test").id(), "telegram");
    }

    #[tokio::test]
    async fn stubs_return_clear_provider_errors() {
        let a = TelegramAdapter::new("test");
        assert!(matches!(
            a.take_inbound().await,
            Err(ArgoError::Provider(_))
        ));
    }
}
