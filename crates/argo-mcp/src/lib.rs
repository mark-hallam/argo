//! `argo-mcp` — MCP client (stdio transport, §12).
//!
//! Argo speaks the [Model Context Protocol](https://modelcontextprotocol.io/)
//! version `2024-11-05` against any compliant MCP server. Phase 1
//! ships **stdio** transport — Argo spawns the server as a subprocess
//! and exchanges JSON-RPC 2.0 messages over its stdin/stdout. SSE +
//! HTTP transports land later (§12.1).
//!
//! ```no_run
//! # async fn doc() -> argo_mcp::McpResult<()> {
//! use argo_mcp::McpClient;
//! let client = McpClient::connect_stdio("my-mcp-server", &[]).await?;
//! let tools = client.list_tools().await?;
//! for t in &tools {
//!     println!("{}: {}", t.name, t.description);
//! }
//! let result = client
//!     .call_tool("navigate", serde_json::json!({"url": "https://example.com"}))
//!     .await?;
//! assert!(!result.is_error);
//! client.shutdown().await?;
//! # Ok(()) }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod client;
mod error;
/// Connection pool for warm MCP subprocesses
/// (`mission-mcp-connection-pooling`). See module docs.
pub mod pool;
#[cfg(test)]
mod pool_tests;
mod protocol;
mod server;

pub use client::{McpClient, DEFAULT_CALL_TIMEOUT};
pub use error::{McpError, McpResult};
pub use pool::{
    McpClientPool, PoolConfig, PoolGuard, PoolKey, PoolStats, DEFAULT_POOL_IDLE_TTL_SECS,
    DEFAULT_POOL_MAX_CLIENTS,
};
pub use protocol::{
    CallToolParams, CallToolResult, ClientInfo, InitializeParams, InitializeResult, JsonRpcError,
    JsonRpcRequest, JsonRpcResponse, ListToolsResult, McpContent, McpToolSchema, ServerInfo,
    ARGO_CLIENT_NAME, ARGO_CLIENT_VERSION, MCP_PROTOCOL_VERSION,
};
pub use server::{handle_request, run_loop, McpServerService};
