//! Agent Client Protocol — JSON-RPC 2.0 wire envelopes.
//!
//! ACP runs JSON-RPC 2.0 over stdio. The envelopes here mirror the
//! shape we already use for plugin (§11) and MCP (§12), kept in a
//! separate crate so the protocols can evolve independently.
use serde::{Deserialize, Serialize};

/// JSON-RPC version string.
pub const JSONRPC_VERSION: &str = "2.0";

/// Inbound ACP request from a harness.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AcpRequest {
    /// Always `"2.0"`.
    pub jsonrpc: String,
    /// Correlation id (number or string).
    pub id: serde_json::Value,
    /// Method name (e.g. `"session/send_message"`).
    pub method: String,
    /// Method-specific params.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl AcpRequest {
    /// Build a request with the standard `"2.0"` jsonrpc field.
    pub fn new<S: Into<String>>(
        id: serde_json::Value,
        method: S,
        params: Option<serde_json::Value>,
    ) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            method: method.into(),
            params,
        }
    }
}

/// Outbound ACP response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AcpResponse {
    /// Always `"2.0"`.
    pub jsonrpc: String,
    /// Correlation id from the matching request.
    pub id: serde_json::Value,
    /// `Some(result)` on success, `None` on error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// `Some(err)` on error, `None` on success.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<AcpError>,
}

impl AcpResponse {
    /// Build a successful response.
    #[must_use]
    pub fn ok(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }
    /// Build an error response.
    #[must_use]
    pub fn err(id: serde_json::Value, error: AcpError) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

/// JSON-RPC error envelope. Codes follow JSON-RPC 2.0 conventions
/// (-32700 parse, -32600 invalid request, -32601 method not found,
/// -32602 invalid params, -32603 internal); ACP-specific errors use
/// the application-defined range.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AcpError {
    /// Error code.
    pub code: i32,
    /// Short message.
    pub message: String,
    /// Optional structured data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl AcpError {
    /// Build a method-not-found (-32601) error.
    #[must_use]
    pub fn method_not_found(method: &str) -> Self {
        Self {
            code: -32601,
            message: format!("method not found: {method}"),
            data: None,
        }
    }
    /// Build an invalid-params (-32602) error.
    #[must_use]
    pub fn invalid_params<S: Into<String>>(detail: S) -> Self {
        Self {
            code: -32602,
            message: detail.into(),
            data: None,
        }
    }
    /// Build an unauthenticated (1001) error.
    #[must_use]
    pub fn unauthenticated<S: Into<String>>(detail: S) -> Self {
        Self {
            code: 1001,
            message: detail.into(),
            data: None,
        }
    }
    /// Build a permission-denied (1003) error.
    #[must_use]
    pub fn permission_denied<S: Into<String>>(detail: S) -> Self {
        Self {
            code: 1003,
            message: detail.into(),
            data: None,
        }
    }
}

/// Outbound ACP notification (server → harness, no `id`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AcpNotification {
    /// Always `"2.0"`.
    pub jsonrpc: String,
    /// Method name (e.g. `"session/stream_chunk"`).
    pub method: String,
    /// Method-specific params.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl AcpNotification {
    /// Build a notification.
    pub fn new<S: Into<String>>(method: S, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.into(),
            params,
        }
    }
}
