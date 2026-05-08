//! Figma MCP connector.
//!
//! ```no_run
//! # async fn doc() -> argo_mcp::McpResult<()> {
//! use argo_mcp::{CallToolParams, McpServerService};
//! use argo_mcp_connector_figma::FigmaService;
//!
//! let svc = FigmaService;
//! assert_eq!(svc.list_tools().len(), 5);
//! let _ = svc.call_tool(CallToolParams {
//!     name: "figma.get_file".into(),
//!     arguments: serde_json::json!({"file_key": "abc123"}),
//! }).await?;
//! # Ok(()) }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod auth;
pub mod client;
mod schemas;
mod service;

pub use client::FigmaClient;
pub use service::FigmaService;
