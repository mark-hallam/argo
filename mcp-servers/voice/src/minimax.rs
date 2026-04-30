//! MiniMax TTS backend for the `synthesize` tool.
//!
//! Wraps `POST https://api.minimax.chat/v1/t2a_v2`. Reads the API
//! key from `MINIMAX_API_KEY` and the group id from
//! `MINIMAX_GROUP_ID` (MiniMax requires both). Missing → friendly
//! error returned to the caller as an MCP `is_error` content
//! block.
use argo_mcp::{CallToolResult, McpContent};
use serde::Serialize;
use std::path::PathBuf;

const DEFAULT_VOICE_ID: &str = "male-qn-qingse";
const DEFAULT_MODEL: &str = "speech-01-turbo";

/// Voice settings — MiniMax wraps speed/volume/pitch in a sub-object.
#[derive(Serialize, Debug, Clone)]
pub struct MiniMaxVoiceSetting<'a> {
    pub voice_id: &'a str,
    pub speed: f32,
    pub vol: f32,
    pub pitch: i32,
}

/// Audio settings — MP3 24kHz mono is the safe universal default.
#[derive(Serialize, Debug, Clone)]
pub struct MiniMaxAudioSetting {
    pub sample_rate: u32,
    pub bitrate: u32,
    pub format: &'static str,
    pub channel: u32,
}

/// Request body for MiniMax T2A v2.
#[derive(Serialize, Debug, Clone)]
pub struct MiniMaxRequest<'a> {
    pub model: &'a str,
    pub text: &'a str,
    pub stream: bool,
    pub voice_setting: MiniMaxVoiceSetting<'a>,
    pub audio_setting: MiniMaxAudioSetting,
}

/// Build the request body. Pure — no I/O.
#[must_use]
pub fn build_request_body<'a>(text: &'a str, voice: &'a str) -> MiniMaxRequest<'a> {
    MiniMaxRequest {
        model: DEFAULT_MODEL,
        text,
        stream: false,
        voice_setting: MiniMaxVoiceSetting {
            voice_id: voice,
            speed: 1.0,
            vol: 1.0,
            pitch: 0,
        },
        audio_setting: MiniMaxAudioSetting {
            sample_rate: 24_000,
            bitrate: 128_000,
            format: "mp3",
            channel: 1,
        },
    }
}

/// POST the request and write the MP3 bytes to a tempfile, returning
/// a CallToolResult whose Text content is the absolute file path.
pub async fn synthesize(text: &str, voice: Option<&str>) -> CallToolResult {
    let api_key = match std::env::var("MINIMAX_API_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => return error("MINIMAX_API_KEY not set".into()),
    };
    let group_id = match std::env::var("MINIMAX_GROUP_ID") {
        Ok(g) if !g.is_empty() => g,
        _ => return error("MINIMAX_GROUP_ID not set".into()),
    };
    let voice_id = voice.unwrap_or(DEFAULT_VOICE_ID);
    let body = build_request_body(text, voice_id);
    let url = format!("https://api.minimax.chat/v1/t2a_v2?GroupId={group_id}");
    let client = reqwest::Client::new();
    let resp = match client
        .post(&url)
        .bearer_auth(&api_key)
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return error(format!("MiniMax TTS request failed: {e}")),
    };
    if !resp.status().is_success() {
        let status = resp.status();
        let detail = resp.text().await.unwrap_or_default();
        return error(format!("MiniMax TTS error {status}: {detail}"));
    }
    // MiniMax T2A v2 returns JSON `{ data: { audio: "<hex>" }, ... }`
    // when `stream = false`. Decode the hex to MP3 bytes.
    let body: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => return error(format!("parse MiniMax response: {e}")),
    };
    let hex_audio = match body["data"]["audio"].as_str() {
        Some(s) if !s.is_empty() => s,
        _ => return error(format!("no audio data in response: {body}")),
    };
    let bytes = match hex::decode(hex_audio) {
        Ok(b) => b,
        Err(e) => return error(format!("decode MiniMax hex audio: {e}")),
    };
    let path = tempfile_path("argo-tts-minimax", "mp3");
    if let Err(e) = std::fs::write(&path, &bytes) {
        return error(format!("write {}: {e}", path.display()));
    }
    CallToolResult {
        content: vec![McpContent::Text {
            text: path.to_string_lossy().into_owned(),
        }],
        is_error: false,
    }
}

fn error(text: String) -> CallToolResult {
    CallToolResult {
        content: vec![McpContent::Text { text }],
        is_error: true,
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
    fn request_body_uses_speech_01_turbo() {
        let body = build_request_body("hi", "male-qn-qingse");
        assert_eq!(body.model, "speech-01-turbo");
        assert_eq!(body.text, "hi");
        assert!(!body.stream);
        assert_eq!(body.voice_setting.voice_id, "male-qn-qingse");
        assert_eq!(body.audio_setting.format, "mp3");
        assert_eq!(body.audio_setting.channel, 1);
    }

    #[test]
    fn request_body_serialises_to_expected_json() {
        let body = build_request_body("hi", "voice-x");
        let s = serde_json::to_string(&body).unwrap();
        assert!(s.contains("\"text\":\"hi\""));
        assert!(s.contains("\"voice_id\":\"voice-x\""));
        assert!(s.contains("\"format\":\"mp3\""));
    }

    #[tokio::test]
    async fn synthesize_with_no_key_returns_clean_error() {
        let prev = std::env::var("MINIMAX_API_KEY").ok();
        std::env::remove_var("MINIMAX_API_KEY");
        let r = synthesize("hi", None).await;
        if let Some(p) = prev {
            std::env::set_var("MINIMAX_API_KEY", p);
        }
        assert!(r.is_error);
        match &r.content[0] {
            McpContent::Text { text } => assert!(text.contains("MINIMAX_API_KEY")),
            _ => panic!("expected text"),
        }
    }

    #[tokio::test]
    async fn synthesize_with_no_group_id_returns_clean_error() {
        let prev_key = std::env::var("MINIMAX_API_KEY").ok();
        let prev_grp = std::env::var("MINIMAX_GROUP_ID").ok();
        std::env::set_var("MINIMAX_API_KEY", "fake-key");
        std::env::remove_var("MINIMAX_GROUP_ID");
        let r = synthesize("hi", None).await;
        if let Some(p) = prev_key {
            std::env::set_var("MINIMAX_API_KEY", p);
        } else {
            std::env::remove_var("MINIMAX_API_KEY");
        }
        if let Some(p) = prev_grp {
            std::env::set_var("MINIMAX_GROUP_ID", p);
        }
        assert!(r.is_error);
        match &r.content[0] {
            McpContent::Text { text } => assert!(text.contains("MINIMAX_GROUP_ID")),
            _ => panic!("expected text"),
        }
    }
}
