//! MCP **server** half — Argo as the MCP host so external clients
//! (Claude Desktop, ides, the inspector tool) can browse Argo's
//! sessions / messages / etc. (§12.3).
//!
//! Phase scope: a small dispatcher that reads JSON-RPC requests
//! from a `BufRead`, calls into an [`McpServerService`] trait, and
//! writes responses to a `Write`. The shipped service trait is
//! transport-agnostic; the daemon binding wires it to stdin/stdout
//! when invoked as `argo mcp serve`.
//!
//! Three methods are handled:
//!
//! - `initialize` — the MCP handshake; returns server name/version
//!   + the protocol version string.
//! - `tools/list` — returns the schemas of the tools the service
//!   exposes.
//! - `tools/call` — dispatches to [`McpServerService::call_tool`].
//!
//! Other methods get a clean -32601 method-not-found error.
use crate::error::{McpError, McpResult};
use crate::protocol::{
    CallToolParams, CallToolResult, InitializeResult, JsonRpcError, JsonRpcRequest,
    JsonRpcResponse, ListToolsResult, McpToolSchema, ServerInfo, MCP_PROTOCOL_VERSION,
};
use async_trait::async_trait;
use std::io::{BufRead, Write};

/// User-supplied service that backs the MCP server's tool methods.
#[async_trait]
pub trait McpServerService: Send + Sync {
    /// Server identity reported in `initialize`.
    fn server_info(&self) -> ServerInfo;
    /// Tools to advertise via `tools/list`.
    fn list_tools(&self) -> Vec<McpToolSchema>;
    /// Execute a tool call.
    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult>;
}

/// Dispatch one parsed JSON-RPC request through `service` and
/// return the response to write back.
pub async fn handle_request<S: McpServerService>(
    service: &S,
    req: JsonRpcRequest,
) -> JsonRpcResponse {
    match req.method.as_str() {
        "initialize" => handle_initialize(service, req),
        "tools/list" => handle_tools_list(service, req),
        "tools/call" => handle_tools_call(service, req).await,
        // Notifications are pinged in MCP — we ack but do nothing.
        m if m.starts_with("notifications/") => JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: req.id,
            result: Some(serde_json::json!({})),
            error: None,
        },
        other => method_not_found(req.id, other),
    }
}

fn handle_initialize<S: McpServerService>(service: &S, req: JsonRpcRequest) -> JsonRpcResponse {
    let result = InitializeResult {
        protocol_version: MCP_PROTOCOL_VERSION.into(),
        server_info: service.server_info(),
        capabilities: serde_json::json!({"tools": {}}),
    };
    let value = serde_json::to_value(&result).unwrap_or_else(|_| serde_json::json!({}));
    JsonRpcResponse {
        jsonrpc: "2.0".into(),
        id: req.id,
        result: Some(value),
        error: None,
    }
}

fn handle_tools_list<S: McpServerService>(service: &S, req: JsonRpcRequest) -> JsonRpcResponse {
    let body = ListToolsResult {
        tools: service.list_tools(),
        next_cursor: None,
    };
    let value = serde_json::to_value(&body).unwrap_or_else(|_| serde_json::json!({}));
    JsonRpcResponse {
        jsonrpc: "2.0".into(),
        id: req.id,
        result: Some(value),
        error: None,
    }
}

async fn handle_tools_call<S: McpServerService>(
    service: &S,
    req: JsonRpcRequest,
) -> JsonRpcResponse {
    let id = req.id.clone();
    let parsed: Result<CallToolParams, _> = req
        .params
        .ok_or_else(|| serde::de::Error::custom("missing params"))
        .and_then(serde_json::from_value);
    let params = match parsed {
        Ok(p) => p,
        Err(e) => return invalid_params(id, e.to_string()),
    };
    match service.call_tool(params).await {
        Ok(result) => {
            let value = serde_json::to_value(&result).unwrap_or_else(|_| serde_json::json!({}));
            JsonRpcResponse {
                jsonrpc: "2.0".into(),
                id,
                result: Some(value),
                error: None,
            }
        }
        Err(e) => internal_error(id, e.to_string()),
    }
}

fn method_not_found(id: serde_json::Value, method: &str) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".into(),
        id,
        result: None,
        error: Some(JsonRpcError {
            code: -32601,
            message: format!("method not found: {method}"),
            data: None,
        }),
    }
}

fn invalid_params(id: serde_json::Value, detail: String) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".into(),
        id,
        result: None,
        error: Some(JsonRpcError {
            code: -32602,
            message: detail,
            data: None,
        }),
    }
}

fn internal_error(id: serde_json::Value, detail: String) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".into(),
        id,
        result: None,
        error: Some(JsonRpcError {
            code: -32603,
            message: detail,
            data: None,
        }),
    }
}

/// Run the line-oriented JSON-RPC loop against `reader` / `writer`.
/// Each input line is parsed as one request; each response is
/// written as one line. The loop terminates on EOF; malformed
/// input lines emit a parse-error response without aborting.
///
/// # Errors
/// Returns the underlying I/O error on read/write failure.
pub async fn run_loop<S, R, W>(service: &S, mut reader: R, mut writer: W) -> McpResult<()>
where
    S: McpServerService,
    R: BufRead,
    W: Write,
{
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader
            .read_line(&mut line)
            .map_err(|e| McpError::Io(format!("stdin read: {e}")))?;
        if n == 0 {
            return Ok(());
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<JsonRpcRequest>(trimmed) {
            Ok(req) => handle_request(service, req).await,
            Err(e) => JsonRpcResponse {
                jsonrpc: "2.0".into(),
                id: serde_json::Value::Null,
                result: None,
                error: Some(JsonRpcError {
                    code: -32700,
                    message: format!("parse error: {e}"),
                    data: None,
                }),
            },
        };
        let serialized = serde_json::to_string(&response)
            .map_err(|e| McpError::Io(format!("response serialize: {e}")))?;
        writer
            .write_all(serialized.as_bytes())
            .and_then(|()| writer.write_all(b"\n"))
            .map_err(|e| McpError::Io(format!("stdout write: {e}")))?;
        writer
            .flush()
            .map_err(|e| McpError::Io(format!("stdout flush: {e}")))?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::McpContent;

    struct StubService;

    #[async_trait]
    impl McpServerService for StubService {
        fn server_info(&self) -> ServerInfo {
            ServerInfo {
                name: "argo-test".into(),
                version: "0.0.1".into(),
            }
        }
        fn list_tools(&self) -> Vec<McpToolSchema> {
            vec![McpToolSchema {
                name: "echo".into(),
                description: "Echo back the input.".into(),
                input_schema: serde_json::json!({"type": "object", "properties": {"text": {"type": "string"}}}),
            }]
        }
        async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
            if params.name != "echo" {
                return Err(McpError::Io(format!("unknown tool {}", params.name)));
            }
            let text = params
                .arguments
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("(empty)");
            Ok(CallToolResult {
                content: vec![McpContent::Text {
                    text: text.to_string(),
                }],
                is_error: false,
            })
        }
    }

    #[tokio::test]
    async fn initialize_returns_server_info() {
        let req = JsonRpcRequest::new(1, "initialize", Some(serde_json::json!({})));
        let resp = handle_request(&StubService, req).await;
        let result = resp.result.unwrap();
        assert_eq!(result["serverInfo"]["name"], "argo-test");
        assert_eq!(result["protocolVersion"], MCP_PROTOCOL_VERSION);
    }

    #[tokio::test]
    async fn tools_list_advertises_echo() {
        let req = JsonRpcRequest::new(2, "tools/list", None);
        let resp = handle_request(&StubService, req).await;
        let body: ListToolsResult = serde_json::from_value(resp.result.unwrap()).unwrap();
        assert_eq!(body.tools.len(), 1);
        assert_eq!(body.tools[0].name, "echo");
    }

    #[tokio::test]
    async fn tools_call_round_trips_echo() {
        let req = JsonRpcRequest::new(
            3,
            "tools/call",
            Some(serde_json::json!({"name": "echo", "arguments": {"text": "hi"}})),
        );
        let resp = handle_request(&StubService, req).await;
        let body: CallToolResult = serde_json::from_value(resp.result.unwrap()).unwrap();
        assert!(!body.is_error);
        match &body.content[0] {
            McpContent::Text { text } => assert_eq!(text, "hi"),
            _ => panic!("expected text"),
        }
    }

    #[tokio::test]
    async fn unknown_method_returns_method_not_found() {
        let req = JsonRpcRequest::new(4, "no/such", None);
        let resp = handle_request(&StubService, req).await;
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32601);
    }

    #[tokio::test]
    async fn run_loop_handles_three_requests_then_eof() {
        let input = format!(
            "{}\n{}\n{}\n",
            serde_json::to_string(&JsonRpcRequest::new(
                1,
                "initialize",
                Some(serde_json::json!({}))
            ))
            .unwrap(),
            serde_json::to_string(&JsonRpcRequest::new(2, "tools/list", None)).unwrap(),
            serde_json::to_string(&JsonRpcRequest::new(
                3,
                "tools/call",
                Some(serde_json::json!({"name": "echo", "arguments": {"text": "ok"}}))
            ))
            .unwrap(),
        );
        let mut output = Vec::new();
        run_loop(&StubService, input.as_bytes(), &mut output)
            .await
            .unwrap();
        let lines: Vec<&str> = std::str::from_utf8(&output)
            .unwrap()
            .split_terminator('\n')
            .collect();
        assert_eq!(lines.len(), 3);
    }

    #[tokio::test]
    async fn malformed_line_emits_parse_error_without_aborting() {
        let input = "not-json\n";
        let mut output = Vec::new();
        run_loop(&StubService, input.as_bytes(), &mut output)
            .await
            .unwrap();
        let resp: JsonRpcResponse = serde_json::from_slice(&output).unwrap();
        assert_eq!(resp.error.as_ref().unwrap().code, -32700);
    }
}
