//! Integration tests for the §12 MCP stdio client.
//!
//! Drives the `argo-test-mcp-server-echo` reference binary via the
//! public `McpClient` surface. Covers the §12.3 acceptance gates that
//! don't need a real third-party MCP server.
use argo_mcp::{McpClient, McpContent, McpError, MCP_PROTOCOL_VERSION};
use std::time::Duration;

fn server_path() -> &'static str {
    env!("CARGO_BIN_EXE_argo-test-mcp-server-echo")
}

#[tokio::test]
async fn handshake_and_server_info() {
    let client = McpClient::connect_stdio(server_path(), &[]).await.unwrap();
    assert_eq!(client.server_info().name, "echo-server");
    assert_eq!(client.server_info().version, "0.1.0");
    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn list_tools_returns_three_entries() {
    let client = McpClient::connect_stdio(server_path(), &[]).await.unwrap();
    let tools = client.list_tools().await.unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"echo"));
    assert!(names.contains(&"crash"));
    assert!(names.contains(&"slow"));
    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn call_tool_echo_round_trips_text() {
    let client = McpClient::connect_stdio(server_path(), &[]).await.unwrap();
    let r = client
        .call_tool("echo", serde_json::json!({"text": "hello"}))
        .await
        .unwrap();
    assert!(!r.is_error);
    match &r.content[0] {
        McpContent::Text { text } => assert_eq!(text, "hello"),
        other => panic!("expected text, got {other:?}"),
    }
    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn unknown_tool_yields_rpc_error() {
    let client = McpClient::connect_stdio(server_path(), &[]).await.unwrap();
    let r = client.call_tool("nope", serde_json::json!({})).await;
    match r {
        Err(McpError::Rpc { code, .. }) => assert_eq!(code, -32602),
        other => panic!("expected Rpc(-32602), got {other:?}"),
    }
    client.shutdown().await.unwrap();
}

#[tokio::test]
async fn server_crash_surfaces_as_mcp_error_crash() {
    let client = McpClient::connect_stdio(server_path(), &[]).await.unwrap();
    // First call succeeds (response is flushed before the server
    // exits).
    let _first = client
        .call_tool("crash", serde_json::json!({}))
        .await
        .unwrap();
    // Second call must observe the dead server and not hang.
    // Whether it surfaces as Crash (reader saw EOF first, set
    // crash_reason or failed pending) or Io (write_line hit EPIPE
    // first) depends on which side of the pipe the OS marks broken
    // first — both indicate the server is gone.
    let after = client
        .call_tool("echo", serde_json::json!({"text": "anyone home?"}))
        .await;
    assert!(
        matches!(after, Err(McpError::Crash(_)) | Err(McpError::Io(_))),
        "expected Crash or Io after server crash, got {after:?}"
    );
}

#[tokio::test]
async fn per_call_timeout_fires() {
    let client =
        McpClient::connect_stdio_with_timeout(server_path(), &[], Duration::from_millis(80))
            .await
            .unwrap();
    let r = client
        .call_tool("slow", serde_json::json!({"ms": 500}))
        .await;
    match r {
        Err(McpError::Timeout(ms)) => assert_eq!(ms, 80),
        other => panic!("expected Timeout(80), got {other:?}"),
    }
    let _ = client.shutdown().await;
}

#[tokio::test]
async fn shutdown_rejects_subsequent_calls() {
    let client = McpClient::connect_stdio(server_path(), &[]).await.unwrap();
    client.shutdown().await.unwrap();
    let r = client.list_tools().await;
    assert!(matches!(r, Err(McpError::Shutdown)));
}

#[tokio::test]
async fn protocol_version_constant_is_pinned() {
    assert_eq!(MCP_PROTOCOL_VERSION, "2024-11-05");
}
