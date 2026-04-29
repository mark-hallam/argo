//! Reference MCP server for the `argo-mcp` integration tests.
//!
//! Implements the Phase-1 surface Argo cares about:
//!
//! - `initialize` — returns the pinned protocol version + a fixed
//!   serverInfo block.
//! - `notifications/initialized` — accepted (no response per MCP spec).
//! - `tools/list` — returns two tools: `echo` and `crash`.
//! - `tools/call` for `echo` → returns the input as a text content
//!   block.
//! - `tools/call` for `crash` → exits the process with code 7
//!   immediately (used by the crash-handling test).
//! - `tools/call` for `slow` → sleeps then returns (timeout test).
//! - `tools/call` for an unknown tool → JSON-RPC error code -32602.
use std::io::{BufRead, Write};
use std::process::ExitCode;

const PROTOCOL_VERSION: &str = "2024-11-05";

fn main() -> ExitCode {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => return ExitCode::from(1),
        };
        if line.trim().is_empty() {
            continue;
        }
        let req: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let id = req.get("id").cloned();
        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = req.get("params").cloned().unwrap_or(serde_json::json!({}));
        // notifications have no id and expect no response.
        if id.is_none() {
            // notifications/initialized — accept silently.
            continue;
        }
        let id = id.unwrap_or(serde_json::Value::Null);
        let response = match method {
            "initialize" => initialize_response(&id),
            "tools/list" => tools_list_response(&id),
            "tools/call" => tools_call(&id, &params),
            _ => error(&id, -32601, &format!("method not found: {method}")),
        };
        let _ = writeln!(out, "{response}");
        let _ = out.flush();
        // `crash` tool exits the process *after* the response is
        // flushed, so the in-flight call resolves first and the next
        // call observes the broken pipe.
        if method == "tools/call" && params.get("name").and_then(|v| v.as_str()) == Some("crash") {
            return ExitCode::from(7);
        }
    }
    ExitCode::SUCCESS
}

fn initialize_response(id: &serde_json::Value) -> String {
    success(
        id,
        serde_json::json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "echo-server", "version": "0.1.0"},
        }),
    )
}

fn tools_list_response(id: &serde_json::Value) -> String {
    success(
        id,
        serde_json::json!({
            "tools": [
                {"name": "echo", "description": "echo back input",
                 "inputSchema": {"type": "object",
                                  "properties": {"text": {"type": "string"}}}},
                {"name": "crash", "description": "exit the server",
                 "inputSchema": {"type": "object"}},
                {"name": "slow", "description": "sleep then echo",
                 "inputSchema": {"type": "object",
                                  "properties": {"ms": {"type": "integer"}}}}
            ]
        }),
    )
}

fn tools_call(id: &serde_json::Value, params: &serde_json::Value) -> String {
    let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or(serde_json::json!({}));
    match name {
        "echo" => {
            let text = args
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            success(
                id,
                serde_json::json!({
                    "content": [{"type":"text","text": text}],
                    "isError": false,
                }),
            )
        }
        "crash" => success(
            id,
            serde_json::json!({"content": [{"type":"text","text":"about to crash"}], "isError": false}),
        ),
        "slow" => {
            let ms = args.get("ms").and_then(|v| v.as_u64()).unwrap_or(0);
            std::thread::sleep(std::time::Duration::from_millis(ms));
            success(
                id,
                serde_json::json!({
                    "content": [{"type":"text","text": format!("slept {ms}")}],
                    "isError": false,
                }),
            )
        }
        other => error(id, -32602, &format!("unknown tool: {other}")),
    }
}

fn success(id: &serde_json::Value, result: serde_json::Value) -> String {
    serde_json::to_string(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    }))
    .unwrap_or_else(|_| String::from("{\"jsonrpc\":\"2.0\",\"id\":null,\"error\":{\"code\":-32603,\"message\":\"internal\"}}"))
}

fn error(id: &serde_json::Value, code: i64, message: &str) -> String {
    serde_json::to_string(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": code, "message": message},
    }))
    .unwrap_or_else(|_| String::from("{\"jsonrpc\":\"2.0\",\"id\":null,\"error\":{\"code\":-32603,\"message\":\"internal\"}}"))
}
