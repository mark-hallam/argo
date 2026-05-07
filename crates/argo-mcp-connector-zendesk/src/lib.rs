//! Zendesk Support MCP connector.
//!
//! ```no_run
//! # async fn doc() -> argo_mcp::McpResult<()> {
//! use argo_mcp::{CallToolParams, McpServerService};
//! use argo_mcp_connector_zendesk::ZendeskService;
//!
//! let svc = ZendeskService;
//! assert_eq!(svc.list_tools().len(), 7);
//! let _ = svc.call_tool(CallToolParams {
//!     name: "zendesk.list_tickets".into(),
//!     arguments: serde_json::json!({"query": "status:open"}),
//! }).await?;
//! # Ok(()) }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod auth;
pub mod client;
mod schemas;
mod service;

pub use client::ZendeskClient;
pub use service::ZendeskService;
