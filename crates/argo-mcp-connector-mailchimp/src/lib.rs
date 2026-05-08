//! Mailchimp MCP connector.
//!
//! ```no_run
//! # async fn doc() -> argo_mcp::McpResult<()> {
//! use argo_mcp::{CallToolParams, McpServerService};
//! use argo_mcp_connector_mailchimp::MailchimpService;
//!
//! let svc = MailchimpService;
//! assert_eq!(svc.list_tools().len(), 6);
//! let _ = svc.call_tool(CallToolParams {
//!     name: "mailchimp.list_audiences".into(),
//!     arguments: serde_json::json!({}),
//! }).await?;
//! # Ok(()) }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod auth;
pub mod client;
mod schemas;
mod service;

pub use client::MailchimpClient;
pub use service::MailchimpService;
