//! 6 unit tests for [`ImageGenMcpServer`]. All use [`StubBackend`]
//! plus an in-memory [`AuditSink`], except `openai_compat_*` which
//! stands up a one-shot TCP listener mimicking an openai endpoint.

use super::*;
use crate::backend::{OpenAiCompatibleBackend, StubBackend};
use argo_mcp::{CallToolParams, McpServerService};
use std::io::{Read, Write};
use std::net::TcpListener;

fn build(require_confirm: bool) -> ImageGenMcpServer {
    ImageGenMcpServer::new(
        Arc::new(StubBackend),
        "dall-e-3",
        "1024x1024",
        require_confirm,
        AuditSink::in_memory(),
    )
}

#[tokio::test]
async fn list_tools_advertises_generate_and_edit() {
    let svc = build(false);
    let tools = svc.list_tools();
    let names: Vec<_> = tools.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(tools.len(), 2);
    assert!(names.contains(&"image.generate"));
    assert!(names.contains(&"image.edit"));
}

#[tokio::test]
async fn generate_writes_cost_audit_row_before_returning() {
    let svc = build(false);
    let result = svc
        .call_tool(CallToolParams {
            name: "image.generate".into(),
            arguments: serde_json::json!({"prompt": "a flying galleon"}),
        })
        .await
        .unwrap();
    assert!(!result.is_error, "{:?}", result.content);
    let rows = svc.audit().lock().await.buffer();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["kind"], "image_gen.cost_estimate");
    assert_eq!(rows[0]["tool"], "image.generate");
    assert_eq!(rows[0]["model"], "dall-e-3");
    assert_eq!(rows[0]["estimated_usd_cents"], 4);
}

#[tokio::test]
async fn edit_decodes_b64_and_returns_result() {
    let svc = build(false);
    // base64 for the bytes b"\x89PNG".
    let result = svc
        .call_tool(CallToolParams {
            name: "image.edit".into(),
            arguments: serde_json::json!({
                "prompt": "make it sepia",
                "image_b64": "iVBORw==",
                "model": "dall-e-2"
            }),
        })
        .await
        .unwrap();
    assert!(!result.is_error, "{:?}", result.content);
    let rows = svc.audit().lock().await.buffer();
    assert_eq!(rows[0]["model"], "dall-e-2");
    assert_eq!(rows[0]["estimated_usd_cents"], 2);
}

#[tokio::test]
async fn require_confirm_blocks_calls_above_threshold() {
    let svc = build(true);
    // Below the 20¢ gate: dall-e-3 at 4¢ passes.
    let ok = svc
        .call_tool(CallToolParams {
            name: "image.generate".into(),
            arguments: serde_json::json!({"prompt": "x"}),
        })
        .await
        .unwrap();
    assert!(!ok.is_error, "4¢ should pass the 20¢ gate");

    // Above the gate: gpt-image-1-hd at 25¢ short-circuits.
    let blocked = svc
        .call_tool(CallToolParams {
            name: "image.generate".into(),
            arguments: serde_json::json!({"prompt": "x", "model": "gpt-image-1-hd"}),
        })
        .await
        .unwrap();
    assert!(blocked.is_error, "25¢ should trip the 20¢ gate");
    match &blocked.content[0] {
        argo_mcp::McpContent::Text { text } => assert!(text.contains("ApprovalRequired"), "{text}"),
        _ => panic!("expected text"),
    }
    // Audit row was written BEFORE the gate fired — both calls present.
    let rows = svc.audit().lock().await.buffer();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1]["estimated_usd_cents"], 25);
}

#[tokio::test]
async fn unknown_tool_returns_is_error_with_message() {
    let svc = build(false);
    let result = svc
        .call_tool(CallToolParams {
            name: "image.deepfry".into(),
            arguments: serde_json::json!({"prompt": "x"}),
        })
        .await
        .unwrap();
    assert!(result.is_error);
    match &result.content[0] {
        argo_mcp::McpContent::Text { text } => assert!(text.contains("unknown tool")),
        _ => panic!("expected text"),
    }
}

#[tokio::test]
async fn openai_compat_backend_against_mocked_http_endpoint() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = vec![0u8; 65536];
        let n = stream.read(&mut buf).unwrap();
        let req = String::from_utf8_lossy(&buf[..n]);
        assert!(req.contains("POST /v1/images/generations"));
        assert!(req.to_ascii_lowercase().contains("authorization: bearer "));
        let body = r#"{"data":[{"b64_json":"aGVsbG8="}]}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).unwrap();
    });
    let backend = OpenAiCompatibleBackend::new(format!("http://{addr}/v1"), "sk-test").unwrap();
    let result = backend
        .generate("a sloop", "1024x1024", "dall-e-3")
        .await
        .unwrap();
    assert_eq!(result.b64_data.as_deref(), Some("aGVsbG8="));
    assert_eq!(result.model, "dall-e-3");
    assert_eq!(result.prompt_hash.len(), 16);
}
