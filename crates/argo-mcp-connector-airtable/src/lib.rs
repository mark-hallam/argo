//! Airtable MCP connector.
//!
//! ```no_run
//! # async fn doc() -> argo_mcp::McpResult<()> {
//! use argo_mcp::{CallToolParams, McpServerService};
//! use argo_mcp_connector_airtable::AirtableService;
//!
//! let svc = AirtableService;
//! assert_eq!(svc.list_tools().len(), 7);
//! let _ = svc.call_tool(CallToolParams {
//!     name: "airtable.list_bases".into(),
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

pub use client::AirtableClient;
pub use service::AirtableService;
