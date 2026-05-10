//! Image-generation MCP service.
//!
//! Bridges MCP `call_tool` requests to [`crate::client::ImageGenClient`]
//! and writes the rendered PNG to
//! `<workspace_root>/.argo/generated/<sha256>.png`.

use crate::client::ImageGenClient;
use argo_mcp::{
    CallToolParams, CallToolResult, McpContent, McpResult, McpServerService, McpToolSchema,
    ServerInfo,
};
use async_trait::async_trait;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Env var that overrides the workspace root used for output paths.
pub const ARGO_WORKSPACE_ROOT_ENV: &str = "ARGO_WORKSPACE_ROOT";

/// Image-generation MCP service.
#[derive(Default)]
pub struct ImageGenService;

#[async_trait]
impl McpServerService for ImageGenService {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "argo-mcp-connector-image-gen".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn list_tools(&self) -> Vec<McpToolSchema> {
        crate::schemas::all()
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        if params.name != "image-gen.generate_image" {
            return Ok(error_result(format!("unknown tool: {}", params.name)));
        }
        match dispatch_generate(&params.arguments).await {
            Ok(value) => Ok(CallToolResult {
                content: vec![McpContent::Text {
                    text: value.to_string(),
                }],
                is_error: false,
            }),
            Err(text) => Ok(error_result(text)),
        }
    }
}

async fn dispatch_generate(args: &serde_json::Value) -> Result<serde_json::Value, String> {
    let prompt = required_str(args, "prompt")?;
    let style = optional_str(args, "style");
    let size = optional_str(args, "size");
    let backend = optional_str(args, "backend");
    let client = ImageGenClient::from_env(backend.as_deref())?;
    let img = client
        .generate(&prompt, style.as_deref(), size.as_deref())
        .await?;
    let root = workspace_root();
    let dir = root.join(".argo").join("generated");
    let path = write_png(&dir, &img.bytes)?;
    Ok(serde_json::json!({
        "url": format!("file://{}", path.display()),
        "backend": img.backend.name(),
        "bytes": img.bytes.len(),
    }))
}

/// Write `bytes` to `<dir>/<sha256>.png`. Returns the full path.
pub fn write_png(dir: &Path, bytes: &[u8]) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let name: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    let path = dir.join(format!("{name}.png"));
    std::fs::write(&path, bytes).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(path)
}

fn workspace_root() -> PathBuf {
    if let Ok(value) = std::env::var(ARGO_WORKSPACE_ROOT_ENV) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn error_result(text: String) -> CallToolResult {
    CallToolResult {
        content: vec![McpContent::Text { text }],
        is_error: true,
    }
}

fn required_str(args: &serde_json::Value, field: &str) -> Result<String, String> {
    args.get(field)
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("missing string field '{field}'"))
}

fn optional_str(args: &serde_json::Value, field: &str) -> Option<String> {
    args.get(field)
        .and_then(|v| v.as_str())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn list_tools_advertises_one_tool() {
        let svc = ImageGenService;
        let tools = svc.list_tools();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "image-gen.generate_image");
    }

    #[tokio::test]
    async fn unknown_tool_returns_tool_error() {
        let result = ImageGenService
            .call_tool(CallToolParams {
                name: "image-gen.zoom".into(),
                arguments: serde_json::json!({}),
            })
            .await
            .unwrap();
        assert!(result.is_error);
    }

    #[test]
    fn dispatch_rejects_missing_prompt_before_touching_backend() {
        // Synchronous helper guard: no env access, no client construction.
        let err = required_str(&serde_json::json!({}), "prompt").unwrap_err();
        assert!(err.contains("missing string field 'prompt'"));
    }

    #[test]
    fn write_png_uses_sha256_filename_and_creates_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("nested/.argo/generated");
        let path = write_png(&dir, b"hello").unwrap();
        // sha256("hello") -> 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
        assert!(path.ends_with(
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824.png"
        ));
        assert_eq!(std::fs::read(&path).unwrap(), b"hello");
    }
}
