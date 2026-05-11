//! Telegram MCP connector.
//!
//! Thin Telegram Bot-API client: send a chat message, send a reply-keyboard
//! prompt, answer an inline callback, and verify the secret token Telegram
//! attaches to inbound webhook deliveries. The connector is scoped to the
//! primitives required by Skycot's multi-channel approvals path (Claim 6 of
//! the mission backlog); webhook-dispatch wiring into the gateway is a
//! follow-up brief.
//!
//! Auth is a single bot token of shape `<bot_id>:<auth_hash>` (treat as
//! secret — never log raw). HTTPS only; Telegram API base is
//! `https://api.telegram.org`. The base URL is configurable via
//! [`TelegramConnector::with_base_url`] so tests can point the client at a
//! wiremock server.
//!
//! ```no_run
//! # async fn doc() -> argo_types::ArgoResult<()> {
//! use argo_mcp_connector_telegram::TelegramConnector;
//! let bot = TelegramConnector::new("123:ABC".to_string());
//! let _msg = bot.send_message(42, "hello").await?;
//! # Ok(()) }
//! ```
#![deny(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod client;
#[cfg(test)]
mod client_tests;
mod types;
mod webhook;

pub use client::TelegramConnector;
pub use types::{TelegramCallbackQuery, TelegramMessage, TelegramWebhookUpdate};
pub use webhook::{verify_webhook_secret, TELEGRAM_WEBHOOK_SECRET_HEADER};
