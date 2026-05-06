//! HubSpot MCP connector.
//!
//! ```no_run
//! # async fn doc() -> argo_mcp::McpResult<()> {
//! use argo_mcp::{CallToolParams, McpServerService};
//! use argo_mcp_connector_hubspot::HubspotService;
//!
//! let svc = HubspotService;
//! let tools = svc.list_tools();
//! assert_eq!(tools.len(), 5);
//! let _ = svc.call_tool(CallToolParams {
//!     name: "hubspot.get_pipeline".into(),
//!     arguments: serde_json::json!({"object_type": "deals"}),
//! }).await?;
//! # Ok(()) }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod client;
mod schemas;
mod service;

pub use client::HubspotClient;
pub use service::HubspotService;
