//! Voice MCP service — schema + dispatcher. The three tools shipped
//! match §28.Z spec text:
//!
//! - `transcribe` — STT (whisper-rs binding lands later).
//! - `synthesize` — TTS (piper-rs binding lands later). Exposes the
//!   OpenAI-TTS `instructions` field per the spec acceptance.
//! - `voice_chat` — half-duplex voice round-trip helper.
//!
//! Each tool returns a "backend not wired" CallToolResult with
//! `is_error = true` so callers can introspect the schema and
//! handle missing audio rigs gracefully.
use argo_mcp::{
    CallToolParams, CallToolResult, McpContent, McpResult, McpServerService, McpToolSchema,
    ServerInfo,
};
use async_trait::async_trait;

const NOT_WIRED: &str = "voice MCP backend not yet wired — whisper-rs (STT) + piper-rs (TTS) \
    + opus (OGG/Opus) bindings land per §12.2.2; configure cloud TTS in server.toml as a fallback.";

/// Voice MCP service.
pub struct VoiceService;

#[async_trait]
impl McpServerService for VoiceService {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "argo-mcp-voice".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn list_tools(&self) -> Vec<McpToolSchema> {
        vec![
            transcribe_schema(),
            synthesize_schema(),
            voice_chat_schema(),
        ]
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        match params.name.as_str() {
            "synthesize" => Ok(synthesize_dispatch(&params.arguments).await),
            "transcribe" => Ok(transcribe_dispatch(&params.arguments).await),
            "voice_chat" => Ok(CallToolResult {
                content: vec![McpContent::Text {
                    text: NOT_WIRED.into(),
                }],
                is_error: true,
            }),
            other => Ok(CallToolResult {
                content: vec![McpContent::Text {
                    text: format!("unknown tool: {other}"),
                }],
                is_error: true,
            }),
        }
    }
}

async fn transcribe_dispatch(args: &serde_json::Value) -> CallToolResult {
    let backend = args
        .get("backend")
        .and_then(|v| v.as_str())
        .unwrap_or("whisper");
    let audio = match args.get("audio").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => {
            return CallToolResult {
                content: vec![McpContent::Text {
                    text: "missing required field 'audio'".into(),
                }],
                is_error: true,
            };
        }
    };
    match backend {
        "openai" => {
            let language = args.get("language").and_then(|v| v.as_str());
            crate::openai::transcribe(audio, language).await
        }
        "local" => crate::local::transcribe(audio).await,
        "whisper" => {
            let language = args.get("language").and_then(|v| v.as_str());
            crate::whisper_native::transcribe(audio, language).await
        }
        _ => CallToolResult {
            content: vec![McpContent::Text {
                text: NOT_WIRED.into(),
            }],
            is_error: true,
        },
    }
}

async fn synthesize_dispatch(args: &serde_json::Value) -> CallToolResult {
    let backend = args
        .get("backend")
        .and_then(|v| v.as_str())
        .unwrap_or("piper");
    let text = match args.get("text").and_then(|v| v.as_str()) {
        Some(t) => t,
        None => {
            return CallToolResult {
                content: vec![McpContent::Text {
                    text: "missing required field 'text'".into(),
                }],
                is_error: true,
            };
        }
    };
    let voice = args.get("voice").and_then(|v| v.as_str());
    let instructions = args.get("instructions").and_then(|v| v.as_str());
    let format = args.get("format").and_then(|v| v.as_str()).unwrap_or("mp3");
    match backend {
        "openai" => crate::openai::synthesize(text, voice, instructions, format).await,
        "elevenlabs" => crate::elevenlabs::synthesize(text, voice).await,
        "minimax" => crate::minimax::synthesize(text, voice).await,
        "local" => crate::local::synthesize(text, format).await,
        "piper" => crate::piper_native::synthesize(text, voice, format).await,
        _ => CallToolResult {
            content: vec![McpContent::Text {
                text: NOT_WIRED.into(),
            }],
            is_error: true,
        },
    }
}

fn transcribe_schema() -> McpToolSchema {
    McpToolSchema {
        name: "transcribe".into(),
        description: "Transcribe audio to text. \
            Default backend is whisper.cpp (not yet wired); \
            pass `backend = \"openai\"` to use the OpenAI Whisper API."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "audio":    { "type": "string", "description": "File path of the audio clip." },
                "language": { "type": "string", "description": "ISO-639 hint (optional)." },
                "backend":  { "type": "string", "enum": ["whisper", "openai", "local"], "default": "whisper" }
            },
            "required": ["audio"]
        }),
    }
}

fn synthesize_schema() -> McpToolSchema {
    McpToolSchema {
        name: "synthesize".into(),
        description:
            "Render text to speech. Returns OGG/Opus by default; MP3 fallback for legacy clients."
                .into(),
        // §28.Z gate: "OpenAI TTS instructions field exposed in tool schema".
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "text":         { "type": "string" },
                "voice":        { "type": "string", "description": "Backend-specific voice id." },
                "format":       { "type": "string", "enum": ["ogg-opus", "mp3"], "default": "ogg-opus" },
                "backend":      { "type": "string", "enum": ["piper", "openai", "elevenlabs", "minimax", "local"], "default": "piper" },
                "instructions": { "type": "string", "description": "OpenAI-TTS style/instructions field (Hermes parity)." }
            },
            "required": ["text"]
        }),
    }
}

fn voice_chat_schema() -> McpToolSchema {
    McpToolSchema {
        name: "voice_chat".into(),
        description:
            "Half-duplex voice round-trip: transcribe input → run agent → synthesize reply.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "audio":   { "type": "string", "description": "User audio clip path or data URL." },
                "session": { "type": "string", "description": "Argo session id to thread the call through." }
            },
            "required": ["audio"]
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_mcp::CallToolParams;

    #[tokio::test]
    async fn list_advertises_three_tools_in_spec_order() {
        let s = VoiceService;
        let tools = s.list_tools();
        assert_eq!(tools.len(), 3);
        assert_eq!(tools[0].name, "transcribe");
        assert_eq!(tools[1].name, "synthesize");
        assert_eq!(tools[2].name, "voice_chat");
    }

    #[tokio::test]
    async fn synthesize_schema_exposes_instructions_field() {
        // §28.Z gate: "OpenAI TTS instructions field exposed in tool schema".
        let s = VoiceService;
        let tools = s.list_tools();
        let synth = &tools[1];
        let props = &synth.input_schema["properties"];
        assert!(props.get("instructions").is_some());
    }

    #[tokio::test]
    async fn synthesize_schema_lists_ogg_opus_default_with_mp3_fallback() {
        let s = VoiceService;
        let tools = s.list_tools();
        let synth = &tools[1];
        let format = &synth.input_schema["properties"]["format"];
        assert_eq!(format["default"], "ogg-opus");
        let formats: Vec<String> = format["enum"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        assert!(formats.contains(&"ogg-opus".to_string()));
        assert!(formats.contains(&"mp3".to_string()));
    }

    #[tokio::test]
    async fn call_tool_transcribe_default_backend_signals_setup_path() {
        // Default backend is `whisper`, which routes to
        // whisper_native. Without `--features whisper-native`
        // the path returns a "rebuild with feature" error.
        // With the feature enabled, it returns an
        // ARGO_WHISPER_MODEL setup error (or a real
        // transcript if both flag and env are wired). Either
        // shape is a useful signal — the test asserts the
        // tool surface still answers the call rather than
        // pinning a specific message.
        let s = VoiceService;
        std::env::remove_var("ARGO_WHISPER_MODEL");
        let result = s
            .call_tool(CallToolParams {
                name: "transcribe".into(),
                arguments: serde_json::json!({"audio": "/tmp/x.ogg"}),
            })
            .await
            .unwrap();
        assert!(result.is_error);
        match &result.content[0] {
            McpContent::Text { text } => assert!(
                text.contains("whisper-native")
                    || text.contains("ARGO_WHISPER_MODEL")
                    || text.contains("not yet wired"),
                "expected feature-gated, model-missing, or not-wired message, got: {text}"
            ),
            _ => panic!("expected text"),
        }
    }

    #[tokio::test]
    async fn call_tool_unknown_returns_error() {
        let s = VoiceService;
        let result = s
            .call_tool(CallToolParams {
                name: "no_such".into(),
                arguments: serde_json::json!({}),
            })
            .await
            .unwrap();
        assert!(result.is_error);
    }
}
