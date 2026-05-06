//! Gmail MCP connector.
//!
//! ```no_run
//! # async fn doc() -> argo_mcp::McpResult<()> {
//! use argo_mcp::{CallToolParams, McpServerService};
//! use argo_mcp_connector_gmail::GmailService;
//!
//! let svc = GmailService::default();
//! let tools = svc.list_tools();
//! assert_eq!(tools.len(), 5);
//! let _ = svc.call_tool(CallToolParams {
//!     name: "gmail.search".into(),
//!     arguments: serde_json::json!({"query": "newer_than:1d"}),
//! }).await?;
//! # Ok(()) }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod auth;
mod client;
mod mime;
mod schemas;
mod service;

pub use client::GmailClient;
pub use service::GmailService;
