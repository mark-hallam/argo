//! WhatsApp Business Cloud MCP connector — outbound v1.
//!
//! Wraps Meta's [WhatsApp Business Cloud API](https://developers.facebook.com/docs/whatsapp/cloud-api)
//! with a typed Rust surface: `send_text`, `send_template`, `send_media`,
//! `mark_read`. Inbound webhook routing is a separate gateway-route brief
//! — this crate is outbound-only.
//!
//! ```no_run
//! use argo_mcp_connector_whatsapp_cloud::{WhatsAppCloudClient, WhatsAppCloudError};
//!
//! # async fn doc() -> Result<(), WhatsAppCloudError> {
//! // Reads the bearer token from $ARGO_WHATSAPP_CLOUD_ACCESS_TOKEN per call.
//! let wa = WhatsAppCloudClient::new("123456789012345".into());
//! let id = wa.send_text("+15557654321", "approval received").await?;
//! println!("sent wamid={id}");
//! # Ok(()) }
//! ```
//!
//! ## 24-hour messaging window
//!
//! WhatsApp restricts session messages (free-form text and media) to a
//! 24-hour window since the customer's last inbound message. Outside that
//! window, only pre-approved **template** messages are deliverable.
//! Violations surface as [`WhatsAppCloudError::OutsideMessagingWindow`];
//! callers SHOULD route to `send_template` when they cannot prove the
//! customer is in-session.
//!
//! ## Auth
//!
//! `Authorization: Bearer <access_token>` with a long-lived (60-day)
//! system-user token. The token is read from the
//! `ARGO_WHATSAPP_CLOUD_ACCESS_TOKEN` env var on every request — never
//! cached inside the connector. Refresh is operator-side via Meta
//! Business Manager.
//!
//! ## Rate-limit handling
//!
//! On HTTP 429 the connector parses `Retry-After` and (when within a
//! configurable ceiling) sleeps with jitter before a single retry.
//! Exceeding the ceiling surfaces [`WhatsAppCloudError::RateLimited`] to
//! the caller with the upstream-requested delay.
//!
//! ## Media
//!
//! v1 accepts an HTTPS URL (Meta's edge fetches the bytes) or a
//! pre-uploaded media id. Raw byte upload via multipart is a v2.

#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod client;
mod types;

pub use client::{WhatsAppCloudClient, DEFAULT_ACCESS_TOKEN_ENV};
pub use types::{
    WhatsAppCloudError, WhatsAppMediaKind, WhatsAppMediaSource, WhatsAppMessageId,
    WhatsAppRateLimitInfo, WhatsAppTemplateVariable,
};
