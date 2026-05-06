//! Fireflies.ai MCP connector.
//!
//! ```no_run
//! # async fn doc() -> argo_mcp::McpResult<()> {
//! use argo_mcp::{CallToolParams, McpServerService};
//! use argo_mcp_connector_fireflies::FirefliesService;
//!
//! let svc = FirefliesService;
//! assert_eq!(svc.list_tools().len(), 5);
//! let _ = svc.call_tool(CallToolParams {
//!     name: "fireflies.list_meetings".into(),
//!     arguments: serde_json::json!({"limit": 10}),
//! }).await?;
//! # Ok(()) }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod auth;
pub mod client;
mod schemas;
mod service;

pub use client::FirefliesClient;
pub use service::FirefliesService;
