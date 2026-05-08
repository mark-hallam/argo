//! SendGrid MCP connector.
//!
//! ```no_run
//! # async fn doc() -> argo_mcp::McpResult<()> {
//! use argo_mcp::{CallToolParams, McpServerService};
//! use argo_mcp_connector_sendgrid::SendgridService;
//!
//! let svc = SendgridService;
//! assert_eq!(svc.list_tools().len(), 5);
//! let _ = svc.call_tool(CallToolParams {
//!     name: "sendgrid.list_templates".into(),
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

pub use client::SendgridClient;
pub use service::SendgridService;
