//! `backend="whisper"` STT via the whisper-rs (whisper.cpp)
//! Rust bindings — v1.1 architectural item. **Scaffolding
//! only**: this PR commits the dispatch path + feature flag
//! without pulling whisper-rs as a dep (it's a heavy
//! C++/CMake build). With `--features whisper-native`, the
//! actual binding will be wired in a follow-up PR; without
//! it, the path returns a clear "feature-gated, recompile"
//! error so users know the rebuild incantation.
//!
//! See `local.rs` for a no-deps escape hatch (shell out to
//! `whisper-cli`).
use argo_mcp::{CallToolResult, McpContent};

#[cfg(feature = "whisper-native")]
pub async fn transcribe(_audio_path: &str, _language: Option<&str>) -> CallToolResult {
    // The wire-up of whisper-rs lands in a follow-up PR.
    // Currently the feature flag exists but the dep is not
    // listed in Cargo.toml — flipping it on without that dep
    // will fail to compile, which is a deliberate signal.
    CallToolResult {
        content: vec![McpContent::Text {
            text: "backend=whisper: whisper-native feature enabled but whisper-rs dep not yet \
                   listed; complete the binding in a follow-up PR"
                .into(),
        }],
        is_error: true,
    }
}

#[cfg(not(feature = "whisper-native"))]
pub async fn transcribe(_audio_path: &str, _language: Option<&str>) -> CallToolResult {
    CallToolResult {
        content: vec![McpContent::Text {
            text: "backend=whisper requires the `whisper-native` feature. Rebuild with \
                   `cargo build -p argo-mcp-voice --features whisper-native` (heavy native \
                   build — needs cmake + a C++ compiler). For a no-deps alternative, set \
                   ARGO_TRANSCRIBE_CMD and use `backend=local`."
                .into(),
        }],
        is_error: true,
    }
}
