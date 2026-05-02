//! `backend="piper"` TTS via the piper-rs Rust bindings —
//! v1.1 architectural item. **Scaffolding only**: this PR
//! commits the dispatch path + feature flag without pulling
//! piper-rs as a dep (heavy native build). With
//! `--features piper-native`, the actual binding will be
//! wired in a follow-up; without it, the path returns a clear
//! "feature-gated, recompile" error.
//!
//! See `local.rs` for a no-deps escape hatch (shell out to
//! `piper`).
use argo_mcp::{CallToolResult, McpContent};

#[cfg(feature = "piper-native")]
pub async fn synthesize(_text: &str, _voice: Option<&str>, _format: &str) -> CallToolResult {
    CallToolResult {
        content: vec![McpContent::Text {
            text: "backend=piper: piper-native feature enabled but piper-rs dep not yet listed; \
                   complete the binding in a follow-up PR"
                .into(),
        }],
        is_error: true,
    }
}

#[cfg(not(feature = "piper-native"))]
pub async fn synthesize(_text: &str, _voice: Option<&str>, _format: &str) -> CallToolResult {
    CallToolResult {
        content: vec![McpContent::Text {
            text: "backend=piper requires the `piper-native` feature. Rebuild with \
                   `cargo build -p argo-mcp-voice --features piper-native` (heavy native \
                   build). For a no-deps alternative, set ARGO_SYNTHESIZE_CMD and use \
                   `backend=local`."
                .into(),
        }],
        is_error: true,
    }
}
