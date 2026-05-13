use crate::dispatch::{dispatch_one, McpConnectionPool};
use argo_capability::CapabilityEngine;
use argo_tool::ToolRegistry;
use argo_types::{ContentBlock, Principal, ToolCallId, ToolCallRequest};
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

#[tokio::test]
async fn dispatch_calls_registered_mcp_tool_and_reuses_server() {
    let tmp = tempfile::tempdir().expect("tmpdir");
    let script = tmp.path().join("fake-mcp.sh");
    let marker = tmp.path().join("marker.txt");
    std::fs::write(
        &script,
        r#"#!/bin/sh
echo start >> "$1"
while IFS= read -r line; do
  case "$line" in *notifications/initialized*) continue ;; esac
  id=$(printf '%s' "$line" | sed -n 's/.*"id":\([0-9][0-9]*\).*/\1/p')
  case "$line" in
    *initialize*)
      printf '{"jsonrpc":"2.0","id":%s,"result":{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"fake","version":"0.1.0"}}}\n' "$id"
      ;;
    *tools/list*)
      printf '{"jsonrpc":"2.0","id":%s,"result":{"tools":[{"name":"hello","description":"say hello","inputSchema":{"type":"object","properties":{"name":{"type":"string"}}}}]}}\n' "$id"
      ;;
    *tools/call*)
      echo call >> "$1"
      printf '{"jsonrpc":"2.0","id":%s,"result":{"content":[{"type":"text","text":"hello from mcp"}],"isError":false}}\n' "$id"
      ;;
  esac
done
"#,
    )
    .expect("write script");
    let mut perms = std::fs::metadata(&script).expect("metadata").permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&script, perms).expect("chmod");

    let mut registry = ToolRegistry::new();
    registry.set_mcp_pool(Arc::new(McpConnectionPool::default()));
    registry.register_mcp_server(
        "fake",
        serde_json::json!({
            "transport": "stdio",
            "command": script,
            "args": [marker],
        }),
    );
    let engine = CapabilityEngine::with_defaults();
    let principal = Principal::Local {
        os_user: "tester".into(),
    };
    engine
        .bind(principal.clone(), "owner".into())
        .expect("bind");

    let call = ToolCallRequest {
        id: ToolCallId::new(),
        provider_call_id: None,
        name: "hello".into(),
        arguments: serde_json::json!({"name": "Ada"}),
    };
    let session_id = argo_orchestrator::SessionId::new();
    let first =
        dispatch_one(&registry, &engine, &principal, tmp.path(), &session_id, &call).await;
    let second =
        dispatch_one(&registry, &engine, &principal, tmp.path(), &session_id, &call).await;

    assert!(!first.is_error, "first result: {first:?}");
    assert!(!second.is_error, "second result: {second:?}");
    assert!(matches!(
        &first.content[0],
        ContentBlock::Text { text } if text == "hello from mcp"
    ));
    assert!(registry.schemas().iter().any(|s| s.name == "hello"));
    let marker = std::fs::read_to_string(marker).expect("marker");
    assert_eq!(marker.matches("start").count(), 1);
    assert_eq!(marker.matches("call").count(), 2);
}
