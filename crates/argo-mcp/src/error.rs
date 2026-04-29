//! `argo-mcp` error variants.
use crate::protocol::JsonRpcError;

/// Lifecycle / transport / protocol error from an MCP server.
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    /// Failed to spawn the MCP server subprocess.
    #[error("MCP spawn failed: {0}")]
    Spawn(String),
    /// `initialize` failed (no response, malformed body, version mismatch).
    #[error("MCP initialize failed: {0}")]
    Initialize(String),
    /// I/O error reading from / writing to the server's stdio.
    #[error("MCP i/o: {0}")]
    Io(String),
    /// The server exited or closed its stdout while a call was in flight.
    /// §12.3 calls this `McpServerCrash` — the loop catches it and may
    /// retry once with backoff.
    #[error("MCP server crash: {0}")]
    Crash(String),
    /// The server returned a structured JSON-RPC error.
    #[error("MCP rpc error {code}: {message}")]
    Rpc {
        /// JSON-RPC error code.
        code: i64,
        /// Server-supplied human-readable message.
        message: String,
        /// Optional structured payload.
        data: Option<serde_json::Value>,
    },
    /// The server's reply was malformed JSON or wrong shape.
    #[error("MCP decode: {0}")]
    Decode(String),
    /// Per-call wall-clock deadline exceeded.
    #[error("MCP call timed out after {0} ms")]
    Timeout(u64),
    /// Caller used the client after `shutdown()`.
    #[error("MCP client shut down")]
    Shutdown,
}

impl From<JsonRpcError> for McpError {
    fn from(e: JsonRpcError) -> Self {
        Self::Rpc {
            code: e.code,
            message: e.message,
            data: e.data,
        }
    }
}

/// Result alias for argo-mcp.
pub type McpResult<T> = Result<T, McpError>;
