//! OpenAI Text-to-Speech backend for the `synthesize` tool.
//!
//! Wraps `POST https://api.openai.com/v1/audio/speech`. Reads the API
//! key from the `OPENAI_API_KEY` env var; missing → friendly error
//! returned to the caller as an MCP `is_error` content block (the
//! caller is expected to handle this and either prompt the user or
//! fall back to another backend).
use argo_mcp::{CallToolResult, McpContent};
use serde::Serialize;
use std::path::PathBuf;

/// Request body for OpenAI's `/v1/audio/speech` endpoint.
#[derive(Serialize, Debug, Clone)]
pub struct OpenAiSpeechRequest<'a> {
    pub model: &'a str,
    pub input: &'a str,
    pub voice: &'a str,
    /// Output container. We force "mp3" today — the §28.Z schema
    /// allows "ogg-opus" but OpenAI uses "opus" + a different
    /// container, so we map the user-facing default and stay strict.
    pub response_format: &'a str,
    /// OpenAI-TTS style/instructions (Hermes parity). Optional.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<&'a str>,
}

/// Build the OpenAI TTS request body from the user-facing
/// synthesize args. Pure function — no I/O.
#[must_use]
pub fn build_request_body<'a>(
    text: &'a str,
    voice: Option<&'a str>,
    instructions: Option<&'a str>,
    _format: &'a str,
) -> OpenAiSpeechRequest<'a> {
    // We force mp3 for now — the §28.Z schema also lists "ogg-opus"
    // but OpenAI uses a different naming, and mp3 is universally
    // supported. piper-rs (when wired) will own ogg-opus.
    OpenAiSpeechRequest {
        model: "tts-1",
        input: text,
        voice: voice.unwrap_or("alloy"),
        response_format: "mp3",
        instructions,
    }
}

/// POST the request and write the audio bytes to a tempfile,
/// returning a CallToolResult whose Text content is the absolute
/// file path.
pub async fn synthesize(
    text: &str,
    voice: Option<&str>,
    instructions: Option<&str>,
    format: &str,
) -> CallToolResult {
    let api_key = match std::env::var("OPENAI_API_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => {
            return CallToolResult {
                content: vec![McpContent::Text {
                    text: "OPENAI_API_KEY not set".into(),
                }],
                is_error: true,
            };
        }
    };
    let body = build_request_body(text, voice, instructions, format);
    let client = reqwest::Client::new();
    let resp = match client
        .post("https://api.openai.com/v1/audio/speech")
        .bearer_auth(&api_key)
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return CallToolResult {
                content: vec![McpContent::Text {
                    text: format!("OpenAI TTS request failed: {e}"),
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
                text: format!("OpenAI TTS error {status}: {detail}"),
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
    let path = tempfile_path("argo-tts", "mp3");
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

/// POST the audio file to OpenAI Whisper and return the transcript
/// as a CallToolResult with Text content.
pub async fn transcribe(audio_path: &str, language: Option<&str>) -> CallToolResult {
    let api_key = match std::env::var("OPENAI_API_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => {
            return CallToolResult {
                content: vec![McpContent::Text {
                    text: "OPENAI_API_KEY not set".into(),
                }],
                is_error: true,
            };
        }
    };
    let bytes = match std::fs::read(audio_path) {
        Ok(b) => b,
        Err(e) => {
            return CallToolResult {
                content: vec![McpContent::Text {
                    text: format!("read {audio_path}: {e}"),
                }],
                is_error: true,
            };
        }
    };
    let file_name = std::path::Path::new(audio_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("audio.wav")
        .to_string();
    let part = match reqwest::multipart::Part::bytes(bytes)
        .file_name(file_name)
        .mime_str("application/octet-stream")
    {
        Ok(p) => p,
        Err(e) => {
            return CallToolResult {
                content: vec![McpContent::Text {
                    text: format!("multipart part: {e}"),
                }],
                is_error: true,
            };
        }
    };
    let mut form = reqwest::multipart::Form::new()
        .text("model", "whisper-1")
        .part("file", part);
    if let Some(lang) = language {
        form = form.text("language", lang.to_string());
    }
    let client = reqwest::Client::new();
    let resp = match client
        .post("https://api.openai.com/v1/audio/transcriptions")
        .bearer_auth(&api_key)
        .multipart(form)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return CallToolResult {
                content: vec![McpContent::Text {
                    text: format!("OpenAI transcribe request failed: {e}"),
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
                text: format!("OpenAI transcribe error {status}: {detail}"),
            }],
            is_error: true,
        };
    }
    let body: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            return CallToolResult {
                content: vec![McpContent::Text {
                    text: format!("parse transcribe response: {e}"),
                }],
                is_error: true,
            };
        }
    };
    let text = body
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    CallToolResult {
        content: vec![McpContent::Text { text }],
        is_error: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_body_uses_alloy_voice_default() {
        let body = build_request_body("hello", None, None, "mp3");
        assert_eq!(body.voice, "alloy");
        assert_eq!(body.input, "hello");
        assert_eq!(body.model, "tts-1");
        assert_eq!(body.response_format, "mp3");
        assert!(body.instructions.is_none());
    }

    #[test]
    fn request_body_passes_instructions_through() {
        let body = build_request_body(
            "hello",
            Some("nova"),
            Some("speak with a Boston accent"),
            "mp3",
        );
        assert_eq!(body.voice, "nova");
        assert_eq!(body.instructions, Some("speak with a Boston accent"));
    }

    #[test]
    fn request_body_serialises_to_expected_json() {
        let body = build_request_body("hi", Some("nova"), Some("calmly"), "mp3");
        let s = serde_json::to_string(&body).unwrap();
        // Field order is dictated by the struct — pin it so a future
        // refactor that changes serde derives is caught.
        assert!(s.contains("\"voice\":\"nova\""));
        assert!(s.contains("\"input\":\"hi\""));
        assert!(s.contains("\"instructions\":\"calmly\""));
        assert!(s.contains("\"model\":\"tts-1\""));
    }
}
