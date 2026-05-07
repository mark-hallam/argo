//! GitHub Issues MCP connector.
//!
//! ```no_run
//! # async fn doc() -> argo_mcp::McpResult<()> {
//! use argo_mcp::{CallToolParams, McpServerService};
//! use argo_mcp_connector_github_issues::GithubIssuesService;
//!
//! let svc = GithubIssuesService;
//! assert_eq!(svc.list_tools().len(), 7);
//! let _ = svc.call_tool(CallToolParams {
//!     name: "github_issues.list_issues".into(),
//!     arguments: serde_json::json!({"owner": "octo", "repo": "repo"}),
//! }).await?;
//! # Ok(()) }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod auth;
pub mod client;
mod schemas;
mod service;

pub use client::GithubIssuesClient;
pub use service::GithubIssuesService;
