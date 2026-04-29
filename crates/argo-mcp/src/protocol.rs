//! MCP protocol message types — JSON-RPC 2.0 framing + the
//! method-specific bodies Phase 1 needs (`initialize`, `tools/list`,
//! `tools/call`).
//!
//! Reference: <https://modelcontextprotocol.io/specification>.
//! Argo speaks MCP version `2024-11-05` for Phase 1.
use serde::{Deserialize, Serialize};

/// MCP protocol version pinned for Phase 1.
pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

/// Argo's name + version in the `initialize` handshake.
pub const ARGO_CLIENT_NAME: &str = "argo";
/// Argo client version reported during `initialize`.
pub const ARGO_CLIENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// JSON-RPC 2.0 request (Argo → server, sometimes the reverse).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// Always `"2.0"`.
    pub jsonrpc: String,
    /// Correlation id (numeric or string).
    pub id: serde_json::Value,
    /// Method name.
    pub method: String,
    /// Method-specific params (object).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl JsonRpcRequest {
    /// Build a request with id `id` and the given method/params.
    #[must_use]
    pub fn new(id: u64, method: &str, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id: serde_json::Value::Number(id.into()),
            method: method.into(),
            params,
        }
    }
}

/// JSON-RPC 2.0 response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// Always `"2.0"`.
    pub jsonrpc: String,
    /// Echo of the request's id.
    pub id: serde_json::Value,
    /// Success body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// Structured error in a [`JsonRpcResponse`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// JSON-RPC error code.
    pub code: i64,
    /// Human-readable message.
    pub message: String,
    /// Optional structured payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// `initialize` params (Argo → server).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    /// MCP protocol version Argo speaks.
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    /// Client capabilities object — empty for Phase 1.
    pub capabilities: serde_json::Value,
    /// Client info.
    #[serde(rename = "clientInfo")]
    pub client_info: ClientInfo,
}

/// Server-reported result of `initialize`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
    /// Protocol version the server agrees to.
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    /// Server capabilities (free-form).
    pub capabilities: serde_json::Value,
    /// Server info.
    #[serde(rename = "serverInfo")]
    pub server_info: ServerInfo,
}

/// Argo's identity sent during `initialize`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    /// Client name.
    pub name: String,
    /// Client version.
    pub version: String,
}

/// MCP server identity reported during `initialize`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    /// Server name (matches manifest).
    pub name: String,
    /// Server version.
    pub version: String,
}

/// Body of a `tools/list` response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListToolsResult {
    /// Tool schemas the server exposes.
    pub tools: Vec<McpToolSchema>,
    /// Cursor for pagination (we ignore it in Phase 1).
    #[serde(
        rename = "nextCursor",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub next_cursor: Option<String>,
}

/// One tool the server exposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolSchema {
    /// Tool name.
    pub name: String,
    /// Description (visible to the model).
    #[serde(default)]
    pub description: String,
    /// JSON Schema for the `arguments` object.
    #[serde(rename = "inputSchema", default = "default_input_schema")]
    pub input_schema: serde_json::Value,
}

fn default_input_schema() -> serde_json::Value {
    serde_json::json!({"type": "object"})
}

/// `tools/call` params.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolParams {
    /// Tool name.
    pub name: String,
    /// Arguments per `inputSchema`.
    pub arguments: serde_json::Value,
}

/// `tools/call` result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolResult {
    /// Content blocks (Argo only consumes text + image for Phase 1).
    pub content: Vec<McpContent>,
    /// Whether the tool reported an error (the JSON-RPC layer succeeded;
    /// `is_error = true` is an *application-level* error).
    #[serde(rename = "isError", default)]
    pub is_error: bool,
}

/// Subset of MCP content types Argo handles in Phase 1.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpContent {
    /// Plain text.
    Text {
        /// Text body.
        text: String,
    },
    /// Image (base64-encoded data URL).
    Image {
        /// Data URL or remote URL.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        data: Option<String>,
        /// MIME type.
        #[serde(rename = "mimeType", default, skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
    },
    /// Anything else — preserved as-is so the agent can fall back to JSON.
    #[serde(other)]
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_round_trips() {
        let r = JsonRpcRequest::new(1, "tools/list", None);
        let s = serde_json::to_string(&r).unwrap();
        let back: JsonRpcRequest = serde_json::from_str(&s).unwrap();
        assert_eq!(back.method, "tools/list");
    }

    #[test]
    fn list_tools_result_decodes() {
        let raw = r#"{
            "tools": [
                {"name": "navigate", "description": "go to URL",
                 "inputSchema": {"type": "object", "properties": {"url": {"type": "string"}}}}
            ]
        }"#;
        let parsed: ListToolsResult = serde_json::from_str(raw).unwrap();
        assert_eq!(parsed.tools.len(), 1);
        assert_eq!(parsed.tools[0].name, "navigate");
    }

    #[test]
    fn call_tool_result_handles_is_error() {
        let raw = r#"{"content": [{"type":"text","text":"oops"}], "isError": true}"#;
        let parsed: CallToolResult = serde_json::from_str(raw).unwrap();
        assert!(parsed.is_error);
        assert_eq!(parsed.content.len(), 1);
    }

    #[test]
    fn unknown_content_type_does_not_fail_decode() {
        let raw = r#"{"content": [{"type": "audio", "data": "..."}], "isError": false}"#;
        let parsed: CallToolResult = serde_json::from_str(raw).unwrap();
        assert!(matches!(parsed.content[0], McpContent::Unknown));
    }
}
