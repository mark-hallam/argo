//! Asana MCP connector.
//!
//! ```no_run
//! # async fn doc() -> argo_mcp::McpResult<()> {
//! use argo_mcp::{CallToolParams, McpServerService};
//! use argo_mcp_connector_asana::AsanaService;
//!
//! let svc = AsanaService;
//! assert_eq!(svc.list_tools().len(), 8);
//! let _ = svc.call_tool(CallToolParams {
//!     name: "asana.list_workspaces".into(),
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

pub use client::AsanaClient;
pub use service::AsanaService;
