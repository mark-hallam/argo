//! Resend MCP connector.
//!
//! Transactional email via Resend's REST API (<https://resend.com/docs>).
//! Outbound-only in v1: `send_email`, `send_batch`, `get_email`, and
//! `list_domains`. Inbound webhook routing (delivered / bounced / opened /
//! clicked / complained) is intentionally out-of-scope — a separate
//! gateway-route brief covers the inbound side.
//!
//! Sibling connectors `argo-mcp-connector-sendgrid` and
//! `argo-mcp-connector-mailgun` were the 1:1 templates for this crate's
//! shape (crate layout, auth, region routing, fixture pattern, error
//! surface). Resend's wire format is simpler than either — request bodies
//! are flat JSON, auth is a single Bearer header, and the same payload
//! shape works for `us` and `eu` regions (only the base URL changes).
//!
//! Auth uses an `re_…`-prefixed API key. The connector reads it from the
//! environment variable [`RESEND_API_KEY_ENV`] on each call (no caching);
//! environment reads are cheap and freshness matters for key rotation.
//! Region (`us` / `eu`) is operator-driven via [`RESEND_REGION_ENV`].
//!
//! ```no_run
//! use argo_mcp_connector_resend::{ResendClient, ResendEmail, ResendRegion};
//!
//! # async fn doc() -> Result<(), argo_mcp_connector_resend::ResendError> {
//! let resend = ResendClient::new("re_test_key".into(), ResendRegion::Us);
//! let id = resend
//!     .send_email(ResendEmail {
//!         from: "Argo <noreply@example.com>".into(),
//!         to: vec!["recipient@example.com".into()],
//!         subject: "Hello".into(),
//!         html: Some("<p>Hi</p>".into()),
//!         text: None,
//!         reply_to: None,
//!         cc: vec![],
//!         bcc: vec![],
//!         tags: vec![],
//!     })
//!     .await?;
//! assert!(!id.0.is_empty());
//! # Ok(()) }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod client;
mod types;

#[cfg(test)]
#[path = "client_tests.rs"]
mod client_tests;

pub use client::{ResendClient, RESEND_API_KEY_ENV, RESEND_REGION_ENV};
pub use types::{
    DnsRecord, ResendDomain, ResendEmail, ResendEmailStatus, ResendError, ResendMessageId,
    ResendRegion, ResendTag,
};
