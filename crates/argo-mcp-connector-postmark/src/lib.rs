//! Postmark MCP connector.
//!
//! ```no_run
//! # async fn doc() -> argo_mcp::McpResult<()> {
//! use argo_mcp::{CallToolParams, McpServerService};
//! use argo_mcp_connector_postmark::PostmarkService;
//!
//! let svc = PostmarkService;
//! assert_eq!(svc.list_tools().len(), 5);
//! let _ = svc.call_tool(CallToolParams {
//!     name: "postmark.list_templates".into(),
//!     arguments: serde_json::json!({"count": 10}),
//! }).await?;
//! # Ok(()) }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod auth;
pub mod client;
mod schemas;
mod service;

pub use client::PostmarkClient;
pub use service::PostmarkService;
