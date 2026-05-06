//! Jira Cloud MCP connector.
//!
//! ```no_run
//! # async fn doc() -> argo_mcp::McpResult<()> {
//! use argo_mcp::{CallToolParams, McpServerService};
//! use argo_mcp_connector_jira::JiraService;
//!
//! let svc = JiraService::default();
//! assert_eq!(svc.list_tools().len(), 7);
//! let _ = svc.call_tool(CallToolParams {
//!     name: "jira.list_projects".into(),
//!     arguments: serde_json::json!({"limit": 10}),
//! }).await?;
//! # Ok(()) }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod auth;
mod client;
mod schemas;
mod service;

pub use auth::JiraConfig;
pub use client::JiraClient;
pub use service::JiraService;
