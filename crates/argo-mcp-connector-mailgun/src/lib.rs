//! Mailgun MCP connector.
//!
//! Outbound email via Mailgun's REST API plus inbound webhook signature
//! verification. Mirrors [`argo_mcp_connector_sendgrid`]'s shape but uses
//! Mailgun's HTTP-Basic auth (user `api`, pass `<api_key>`),
//! `application/x-www-form-urlencoded` request body, and
//! `HMAC-SHA256(timestamp + token, api_key)` webhook signing.
//!
//! Region routing is operator-driven (data residency): [`MailgunRegion::Us`]
//! targets `api.mailgun.net`; [`MailgunRegion::Eu`] targets
//! `api.eu.mailgun.net`. Same auth, same payload shape, different host.
//!
//! ```no_run
//! use argo_mcp_connector_mailgun::{MailgunConnector, MailgunMessage, MailgunRegion};
//!
//! # async fn doc() -> argo_types::ArgoResult<()> {
//! let mg = MailgunConnector::new(
//!     "key-test".into(),
//!     "mg.example.com".into(),
//!     MailgunRegion::Us,
//! );
//! let resp = mg
//!     .send_email(MailgunMessage {
//!         from: "Argo <noreply@example.com>".into(),
//!         to: vec!["recipient@example.com".into()],
//!         cc: vec![],
//!         bcc: vec![],
//!         subject: "Hello".into(),
//!         text: Some("Body".into()),
//!         html: None,
//!         reply_to: None,
//!         tags: vec!["onboarding".into()],
//!     })
//!     .await?;
//! assert!(!resp.message_id().is_empty());
//! # Ok(()) }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod client;
mod types;
mod webhook;

#[cfg(test)]
mod client_tests;

pub use client::MailgunConnector;
pub use types::{
    MailgunEventKind, MailgunMessage, MailgunRegion, MailgunSendResponse, MailgunWebhookEvent,
};
pub use webhook::{verify_webhook, verify_webhook_with_skew, DEFAULT_WEBHOOK_SKEW_SECONDS};
