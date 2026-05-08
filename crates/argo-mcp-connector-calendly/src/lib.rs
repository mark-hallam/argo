//! Calendly MCP connector.
//!
//! ```no_run
//! # async fn doc() -> argo_mcp::McpResult<()> {
//! use argo_mcp::{CallToolParams, McpServerService};
//! use argo_mcp_connector_calendly::CalendlyService;
//!
//! let svc = CalendlyService;
//! assert_eq!(svc.list_tools().len(), 5);
//! let _ = svc.call_tool(CallToolParams {
//!     name: "calendly.list_event_types".into(),
//!     arguments: serde_json::json!({"user_uri": "https://api.calendly.com/users/u"}),
//! }).await?;
//! # Ok(()) }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod auth;
pub mod client;
mod schemas;
mod service;

pub use client::CalendlyClient;
pub use service::CalendlyService;
