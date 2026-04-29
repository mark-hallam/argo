//! Reference Argo plugin used by `argo-plugin` integration tests.
//!
//! Implements the §11 protocol over line-delimited JSON-RPC. Behaviour:
//!
//! - `handshake` → returns the pinned protocol version + plugin name.
//! - `ping` → `{"pong": true, "params": <echo>}`.
//! - `pre_tool_call` → `{"action": "allow"}` for `tool == "ls"`,
//!   `{"action": "deny", "reason": "echo-policy"}` for any other tool.
//! - `slow` → sleeps for `params.ms` ms then returns `{"slept": ms}`.
//!   Used to exercise the per-call timeout path.
//! - `crash` → exits the process with code 7 immediately.
//! - `bad_version_handshake` → handshake that lies about the protocol
//!   version (used by the version-mismatch test).
//! - `shutdown` → exits cleanly.
//!
//! Anything else returns a JSON-RPC error with code -32601 (method not
//! found).
use std::io::{BufRead, Write};
use std::process::ExitCode;

const PROTOCOL_VERSION: &str = "0.1.0";

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
            Err(e) => {
                let _ = writeln!(
                    out,
                    "{}",
                    error_response(&serde_json::Value::Null, -32700, &format!("parse: {e}"))
                );
                let _ = out.flush();
                continue;
            }
        };
        let id = req.get("id").cloned().unwrap_or(serde_json::Value::Null);
        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = req.get("params").cloned().unwrap_or(serde_json::json!({}));
        let response = match method {
            "handshake" => handshake_response(&id, false),
            "bad_version_handshake" => handshake_response(&id, true),
            "ping" => success_response(&id, serde_json::json!({"pong": true, "params": params})),
            "pre_tool_call" => pre_tool_call(&id, &params),
            "slow" => {
                let ms = params.get("ms").and_then(|v| v.as_u64()).unwrap_or(0);
                std::thread::sleep(std::time::Duration::from_millis(ms));
                success_response(&id, serde_json::json!({"slept": ms}))
            }
            "crash" => return ExitCode::from(7),
            "shutdown" => {
                // Acknowledge shutdown then exit.
                let _ = writeln!(out, "{}", success_response(&id, serde_json::json!({})));
                let _ = out.flush();
                return ExitCode::SUCCESS;
            }
            _ => error_response(&id, -32601, &format!("method not found: {method}")),
        };
        let _ = writeln!(out, "{response}");
        let _ = out.flush();
    }
    ExitCode::SUCCESS
}

fn handshake_response(id: &serde_json::Value, lie_about_version: bool) -> String {
    let version = if lie_about_version {
        "9.9.9-mismatch"
    } else {
        PROTOCOL_VERSION
    };
    success_response(
        id,
        serde_json::json!({
            "protocol_version": version,
            "name": "echo",
            "version": "0.1.0",
            "capabilities": [],
        }),
    )
}

fn pre_tool_call(id: &serde_json::Value, params: &serde_json::Value) -> String {
    let tool = params.get("tool").and_then(|v| v.as_str()).unwrap_or("");
    let body = if tool == "ls" {
        serde_json::json!({"action": "allow"})
    } else {
        serde_json::json!({"action": "deny", "reason": "echo-policy"})
    };
    success_response(id, body)
}

fn success_response(id: &serde_json::Value, result: serde_json::Value) -> String {
    serde_json::to_string(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    }))
    .unwrap_or_else(|_| String::from("{\"jsonrpc\":\"2.0\",\"id\":null,\"error\":{\"code\":-32603,\"message\":\"internal\"}}"))
}

fn error_response(id: &serde_json::Value, code: i64, message: &str) -> String {
    serde_json::to_string(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": code, "message": message},
    }))
    .unwrap_or_else(|_| String::from("{\"jsonrpc\":\"2.0\",\"id\":null,\"error\":{\"code\":-32603,\"message\":\"internal\"}}"))
}
