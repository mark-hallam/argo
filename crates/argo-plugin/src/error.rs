//! Plugin-host error variants.
use crate::protocol::JsonRpcError;

/// Lifecycle / transport error from the plugin subprocess.
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    /// Requested plugin id is not present in registry config.
    #[error("unknown plugin id: {0}")]
    UnknownPlugin(String),
    /// Failed to spawn the plugin subprocess.
    #[error("plugin spawn failed: {0}")]
    Spawn(String),
    /// Handshake failed (no response, malformed body, version mismatch).
    #[error("plugin handshake failed: {0}")]
    Handshake(String),
    /// I/O error reading from / writing to the plugin's stdin or
    /// stdout.
    #[error("plugin i/o: {0}")]
    Io(String),
    /// The plugin exited or closed its stdout while a call was in
    /// flight.
    #[error("plugin crash: {0}")]
    Crash(String),
    /// The plugin returned a structured error response.
    #[error("plugin error {code}: {message}")]
    Rpc {
        /// JSON-RPC error code.
        code: i64,
        /// Plugin-supplied human-readable message.
        message: String,
        /// Optional structured payload.
        data: Option<serde_json::Value>,
    },
    /// The plugin's reply was malformed JSON.
    #[error("plugin response decode: {0}")]
    Decode(String),
    /// The host called a method after the plugin was shut down.
    #[error("plugin shutdown — call rejected")]
    Shutdown,
    /// Per-call wall-clock deadline exceeded.
    #[error("plugin call timed out after {0} ms")]
    Timeout(u64),
}

impl From<JsonRpcError> for PluginError {
    fn from(e: JsonRpcError) -> Self {
        Self::Rpc {
            code: e.code,
            message: e.message,
            data: e.data,
        }
    }
}

/// Result alias.
pub type PluginResult<T> = Result<T, PluginError>;
