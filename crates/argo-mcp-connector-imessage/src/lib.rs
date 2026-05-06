//! iMessage MCP connector backed by a Blue Bubbles server.
//!
//! ```no_run
//! # async fn demo() -> argo_mcp::McpResult<()> {
//! use argo_mcp::McpServerService;
//! use argo_mcp_connector_imessage::IMessageService;
//!
//! let service = IMessageService;
//! assert_eq!(service.list_tools().len(), 5);
//! # Ok(()) }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod auth;
pub mod client;
mod schemas;
mod service;

pub use client::IMessageMcpClient;
pub use service::IMessageService;
