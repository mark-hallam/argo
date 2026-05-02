//! `backend="piper"` TTS — placeholder. The intent was to wire
//! `piper-rs` behind the `piper-native` feature flag, mirroring
//! whisper-rs (#636). As of #636, `piper-rs` 0.1.9 (latest on
//! crates.io) fails to compile against the current `ort` (ONNX
//! runtime) — `Shape::view()` was removed, breaking 21 sites in
//! piper-rs. Until upstream stabilises, the `backend="piper"`
//! path returns the helpful error below pointing callers at the
//! `backend="local"` shell-out, which works with the standard
//! `piper` CLI binary today.
//!
//! When piper-rs catches up, this file gains the same shape as
//! `whisper_native.rs`: a `cfg(feature = "piper-native")`
//! synthesize path that loads the voice model + emits WAV bytes.
use argo_mcp::{CallToolResult, McpContent};

pub async fn synthesize(_text: &str, _voice: Option<&str>, _format: &str) -> CallToolResult {
    CallToolResult {
        content: vec![McpContent::Text {
            text: "backend=piper: piper-rs upstream is currently broken against ort 2.x \
                   (Shape::view() was removed). Use `backend=local` instead — set \
                   ARGO_SYNTHESIZE_CMD to your `piper` binary, e.g. \
                   `piper --model en_US-lessac-medium.onnx --output_file -`. \
                   Native Rust binding will land once piper-rs upstream is fixed."
                .into(),
        }],
        is_error: true,
    }
}
