//! Typeform MCP connector.
//!
//! ```no_run
//! use argo_mcp_connector_typeform::TypeformService;
//! use argo_mcp::{CallToolParams, McpServerService};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let svc = TypeformService::default();
//! let _ = svc.call_tool(CallToolParams {
//!     name: "typeform.list_forms".into(),
//!     arguments: serde_json::json!({"page_size": 10}),
//! }).await?;
//! # Ok(())
//! # }
//! ```

mod auth;
mod client;
mod schemas;
mod service;

pub use service::TypeformService;
