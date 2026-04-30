//! ElevenLabs TTS backend for the `synthesize` tool.
//!
//! Wraps `POST https://api.elevenlabs.io/v1/text-to-speech/<voice_id>`.
//! Reads the API key from `ELEVENLABS_API_KEY`; missing → friendly
//! error returned to the caller as an MCP `is_error` content block.
//!
//! ElevenLabs voice IDs are opaque hex strings (e.g.
//! `21m00Tcm4TlvDq8ikWAM` for "Rachel"). We default to "Rachel" if
//! none provided — that's the same default the ElevenLabs Hermes
//! integration uses.
use argo_mcp::{CallToolResult, McpContent};
use serde::Serialize;
use std::path::PathBuf;

const DEFAULT_VOICE_ID: &str = "21m00Tcm4TlvDq8ikWAM";

/// Request body for ElevenLabs' `/v1/text-to-speech/<id>` endpoint.
#[derive(Serialize, Debug, Clone)]
pub struct ElevenLabsRequest<'a> {
    /// Text to synthesise.
    pub text: &'a str,
    /// Engine identifier — "eleven_turbo_v2_5" is the latest at
    /// the time of writing; defaults are sensible for most cases.
    pub model_id: &'a str,
}

/// Build the request body. Pure — no I/O.
#[must_use]
pub fn build_request_body<'a>(text: &'a str) -> ElevenLabsRequest<'a> {
    ElevenLabsRequest {
        text,
        model_id: "eleven_turbo_v2_5",
    }
}

/// POST the request and write the MP3 bytes to a tempfile,
/// returning a CallToolResult whose Text content is the absolute
/// file path.
pub async fn synthesize(text: &str, voice: Option<&str>) -> CallToolResult {
    let api_key = match std::env::var("ELEVENLABS_API_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => {
            return CallToolResult {
                content: vec![McpContent::Text {
                    text: "ELEVENLABS_API_KEY not set".into(),
                }],
                is_error: true,
            };
        }
    };
    let voice_id = voice.unwrap_or(DEFAULT_VOICE_ID);
    let body = build_request_body(text);
    let url = format!("https://api.elevenlabs.io/v1/text-to-speech/{voice_id}");
    let client = reqwest::Client::new();
    let resp = match client
        .post(&url)
        .header("xi-api-key", &api_key)
        .header("Accept", "audio/mpeg")
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return CallToolResult {
                content: vec![McpContent::Text {
                    text: format!("ElevenLabs TTS request failed: {e}"),
                }],
                is_error: true,
            };
        }
    };
    if !resp.status().is_success() {
        let status = resp.status();
        let detail = resp.text().await.unwrap_or_default();
        return CallToolResult {
            content: vec![McpContent::Text {
                text: format!("ElevenLabs TTS error {status}: {detail}"),
            }],
            is_error: true,
        };
    }
    let bytes = match resp.bytes().await {
        Ok(b) => b,
        Err(e) => {
            return CallToolResult {
                content: vec![McpContent::Text {
                    text: format!("read audio body: {e}"),
                }],
                is_error: true,
            };
        }
    };
    let path = tempfile_path("argo-tts-eleven", "mp3");
    if let Err(e) = std::fs::write(&path, &bytes) {
        return CallToolResult {
            content: vec![McpContent::Text {
                text: format!("write {}: {e}", path.display()),
            }],
            is_error: true,
        };
    }
    CallToolResult {
        content: vec![McpContent::Text {
            text: path.to_string_lossy().into_owned(),
        }],
        is_error: false,
    }
}

fn tempfile_path(prefix: &str, ext: &str) -> PathBuf {
    let id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("{prefix}-{id}.{ext}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_body_uses_turbo_v2_5() {
        let body = build_request_body("hello");
        assert_eq!(body.text, "hello");
        assert_eq!(body.model_id, "eleven_turbo_v2_5");
    }

    #[test]
    fn request_body_serialises_to_expected_json() {
        let body = build_request_body("hi");
        let s = serde_json::to_string(&body).unwrap();
        assert!(s.contains("\"text\":\"hi\""));
        assert!(s.contains("\"model_id\":\"eleven_turbo_v2_5\""));
    }

    #[tokio::test]
    async fn synthesize_with_no_key_returns_clean_error() {
        // Save + restore env so we don't leak across tests.
        let prev = std::env::var("ELEVENLABS_API_KEY").ok();
        std::env::remove_var("ELEVENLABS_API_KEY");
        let r = synthesize("hi", None).await;
        if let Some(p) = prev {
            std::env::set_var("ELEVENLABS_API_KEY", p);
        }
        assert!(r.is_error);
        match &r.content[0] {
            McpContent::Text { text } => assert!(text.contains("ELEVENLABS_API_KEY")),
            _ => panic!("expected text"),
        }
    }
}
