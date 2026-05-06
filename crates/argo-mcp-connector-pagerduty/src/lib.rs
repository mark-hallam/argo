//! PagerDuty MCP connector.
//!
//! ```no_run
//! # async fn doc() -> argo_mcp::McpResult<()> {
//! use argo_mcp::{CallToolParams, McpServerService};
//! use argo_mcp_connector_pagerduty::PagerDutyService;
//!
//! let svc = PagerDutyService;
//! assert_eq!(svc.list_tools().len(), 7);
//! let _ = svc.call_tool(CallToolParams {
//!     name: "pagerduty.list_incidents".into(),
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

pub use client::PagerDutyClient;
pub use service::PagerDutyService;
