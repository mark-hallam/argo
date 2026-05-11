//! [`ImageGenMcpServer`] — implements [`argo_mcp::McpServerService`].
//!
//! Two tools: `image.generate` and `image.edit`. Each call writes a
//! `cost_estimate` audit row to the configured NDJSON sink BEFORE
//! the backend call. If [`ImageGenMcpServer::require_confirm`] is
//! set and the estimated cost exceeds [`APPROVAL_THRESHOLD_CENTS`],
//! the call short-circuits with an `ApprovalRequired` error and no
//! upstream request is issued.

use crate::backend::{estimate_cost_cents, ImageBackend, ImageResult};
use argo_mcp::{
    CallToolParams, CallToolResult, McpContent, McpResult, McpServerService, McpToolSchema,
    ServerInfo,
};
use async_trait::async_trait;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// `--require-confirm` short-circuits when estimate >= this (20¢).
pub const APPROVAL_THRESHOLD_CENTS: u32 = 20;

/// MCP server exposing `image.generate` + `image.edit`.
pub struct ImageGenMcpServer {
    backend: Arc<dyn ImageBackend>,
    model_default: String,
    size_default: String,
    require_confirm: bool,
    audit: Arc<Mutex<AuditSink>>,
}

/// Path-based NDJSON audit sink. Each row is one JSON object on its
/// own line. Tests use [`AuditSink::in_memory`] to assert content.
pub enum AuditSink {
    /// Append `{}\n` rows to this path.
    File(PathBuf),
    /// In-memory buffer; tests read [`AuditSink::buffer`].
    Memory(Vec<serde_json::Value>),
}

impl AuditSink {
    /// In-memory sink for tests.
    #[must_use]
    pub fn in_memory() -> Self {
        Self::Memory(Vec::new())
    }
    /// Snapshot of buffered rows (memory sinks only; empty otherwise).
    #[must_use]
    pub fn buffer(&self) -> Vec<serde_json::Value> {
        match self {
            Self::Memory(v) => v.clone(),
            Self::File(_) => Vec::new(),
        }
    }
    fn write(&mut self, row: &serde_json::Value) -> Result<(), String> {
        match self {
            Self::Memory(v) => {
                v.push(row.clone());
                Ok(())
            }
            Self::File(p) => {
                if let Some(parent) = p.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| format!("audit mkdir {}: {e}", parent.display()))?;
                }
                let mut f = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&p)
                    .map_err(|e| format!("audit open {}: {e}", p.display()))?;
                let line = serde_json::to_string(row)
                    .map_err(|e| format!("audit serialize: {e}"))?;
                f.write_all(line.as_bytes())
                    .and_then(|()| f.write_all(b"\n"))
                    .map_err(|e| format!("audit write: {e}"))?;
                Ok(())
            }
        }
    }
}

impl ImageGenMcpServer {
    /// Build with a backend, defaults, and an audit sink.
    pub fn new(
        backend: Arc<dyn ImageBackend>,
        model_default: impl Into<String>,
        size_default: impl Into<String>,
        require_confirm: bool,
        audit: AuditSink,
    ) -> Self {
        Self {
            backend,
            model_default: model_default.into(),
            size_default: size_default.into(),
            require_confirm,
            audit: Arc::new(Mutex::new(audit)),
        }
    }

    /// Borrow the audit sink (for test assertions).
    #[must_use]
    pub fn audit(&self) -> Arc<Mutex<AuditSink>> {
        self.audit.clone()
    }

    fn schemas(&self) -> Vec<McpToolSchema> {
        vec![
            McpToolSchema {
                name: "image.generate".into(),
                description: "Generate one image from a text prompt.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "prompt": {"type": "string"},
                        "size": {"type": "string"},
                        "model": {"type": "string"}
                    },
                    "required": ["prompt"]
                }),
            },
            McpToolSchema {
                name: "image.edit".into(),
                description: "Edit an image (base64 PNG) using a text prompt.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "image_b64": {"type": "string"},
                        "prompt": {"type": "string"},
                        "model": {"type": "string"}
                    },
                    "required": ["image_b64", "prompt"]
                }),
            },
        ]
    }

    async fn run_tool(&self, params: CallToolParams) -> Result<ImageResult, String> {
        let prompt = required_str(&params.arguments, "prompt")?;
        let model = optional_str(&params.arguments, "model").unwrap_or_else(|| self.model_default.clone());
        let cents = estimate_cost_cents(&model);
        let row = serde_json::json!({
            "kind": "image_gen.cost_estimate",
            "tool": params.name,
            "backend": self.backend.name(),
            "model": model,
            "estimated_usd_cents": cents,
            "prompt_hash": ImageResult::hash_prompt(&prompt),
            "ts": chrono::Utc::now().to_rfc3339(),
        });
        self.audit.lock().await.write(&row)?;
        if self.require_confirm && cents >= APPROVAL_THRESHOLD_CENTS {
            return Err(format!(
                "ApprovalRequired: estimated {cents}¢ >= {APPROVAL_THRESHOLD_CENTS}¢ threshold; \
                 re-run without --require-confirm or get operator approval"
            ));
        }
        match params.name.as_str() {
            "image.generate" => {
                let size = optional_str(&params.arguments, "size")
                    .unwrap_or_else(|| self.size_default.clone());
                self.backend.generate(&prompt, &size, &model).await
            }
            "image.edit" => {
                let b64 = required_str(&params.arguments, "image_b64")?;
                let bytes = decode_b64(&b64)?;
                self.backend.edit(&bytes, &prompt, &model).await
            }
            other => Err(format!("unknown tool: {other}")),
        }
    }
}

#[async_trait]
impl McpServerService for ImageGenMcpServer {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "argo-mcp-image-gen".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }
    fn list_tools(&self) -> Vec<McpToolSchema> {
        self.schemas()
    }
    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        let body = self.run_tool(params).await;
        Ok(match body {
            Ok(result) => {
                let value = serde_json::to_value(&result).unwrap_or(serde_json::Value::Null);
                CallToolResult {
                    content: vec![McpContent::Text { text: value.to_string() }],
                    is_error: false,
                }
            }
            Err(msg) => CallToolResult {
                content: vec![McpContent::Text { text: msg }],
                is_error: true,
            },
        })
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

fn decode_b64(s: &str) -> Result<Vec<u8>, String> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if cleaned.is_empty() {
        return Err("image_b64 empty".into());
    }
    STANDARD.decode(cleaned).map_err(|e| format!("image_b64 decode: {e}"))
}

#[cfg(test)]
#[path = "server_tests.rs"]
mod server_tests;
