//! End-to-end integration test against a stubbed OpenAI HTTP server.
//!
//! Exercises the full path: tool dispatch → backend selection →
//! OpenAI client → b64 decode → file write under the workspace root.
//! Asserts both the on-disk file and the returned `file://` URL match.

use argo_mcp::{CallToolParams, McpContent, McpServerService};
use argo_mcp_connector_image_gen::{
    auth::{OPENAI_API_KEY_ENV, REPLICATE_API_TOKEN_ENV, STABILITY_API_KEY_ENV},
    ImageGenService,
};
use std::io::{Read, Write};
use std::net::TcpListener;

const OPENAI_BASE_URL_ENV: &str = "IMAGE_GEN_OPENAI_BASE_URL";
const ARGO_WORKSPACE_ROOT_ENV: &str = "ARGO_WORKSPACE_ROOT";

const ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn b64_encode(bytes: &[u8]) -> String {
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let mut buf = [0u8; 3];
        for (i, b) in chunk.iter().enumerate() {
            buf[i] = *b;
        }
        let n = chunk.len();
        let triple =
            (u32::from(buf[0]) << 16) | (u32::from(buf[1]) << 8) | u32::from(buf[2]);
        out.push(ALPHABET[((triple >> 18) & 0x3F) as usize] as char);
        out.push(ALPHABET[((triple >> 12) & 0x3F) as usize] as char);
        if n >= 2 {
            out.push(ALPHABET[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if n >= 3 {
            out.push(ALPHABET[(triple & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

#[tokio::test]
async fn end_to_end_openai_writes_file_and_returns_url() {
    // Stand up a one-shot HTTP server that mimics OpenAI's
    // images.generations endpoint.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let png_bytes = b"\x89PNG\x0d\x0a\x1a\x0a-int-test-".to_vec();
    let png_clone = png_bytes.clone();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = vec![0u8; 65536];
        let n = stream.read(&mut buf).unwrap();
        let req = String::from_utf8_lossy(&buf[..n]);
        assert!(req.contains("POST /v1/images/generations"));
        assert!(req.to_ascii_lowercase().contains("authorization: bearer "));
        let body = serde_json::json!({
            "data": [{"b64_json": b64_encode(&png_clone)}]
        })
        .to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).unwrap();
    });

    let workspace = tempfile::tempdir().unwrap();
    // Pin env so the dispatcher routes to our fake openai.
    std::env::remove_var("IMAGE_GEN_BACKEND");
    std::env::remove_var(STABILITY_API_KEY_ENV);
    std::env::remove_var(REPLICATE_API_TOKEN_ENV);
    std::env::set_var(OPENAI_API_KEY_ENV, "sk-int-test");
    std::env::set_var(OPENAI_BASE_URL_ENV, format!("http://{addr}"));
    std::env::set_var(ARGO_WORKSPACE_ROOT_ENV, workspace.path());

    let result = ImageGenService
        .call_tool(CallToolParams {
            name: "image-gen.generate_image".into(),
            arguments: serde_json::json!({
                "prompt": "an argonaut at sunrise",
                "style": "vivid",
                "size": "1024x1024",
                "backend": "openai"
            }),
        })
        .await
        .unwrap();

    std::env::remove_var(OPENAI_API_KEY_ENV);
    std::env::remove_var(OPENAI_BASE_URL_ENV);
    std::env::remove_var(ARGO_WORKSPACE_ROOT_ENV);

    assert!(!result.is_error, "tool returned error: {:?}", result.content);
    let text = match &result.content[0] {
        McpContent::Text { text } => text.as_str(),
        other => panic!("expected text content, got {other:?}"),
    };
    let json: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(json["backend"], "openai");
    assert_eq!(json["bytes"], png_bytes.len());

    let expected_path = workspace
        .path()
        .join(".argo")
        .join("generated")
        .join(format!("{}.png", sha256_hex(&png_bytes)));
    let expected_url = format!("file://{}", expected_path.display());
    assert_eq!(json["url"], expected_url);
    assert_eq!(std::fs::read(&expected_path).unwrap(), png_bytes);
}
