//! `argo-mcp-voice` — voice MCP server (§28.Z).
//!
//! First cut ships the **schema + dispatcher** for the three tools
//! the spec calls out (`transcribe`, `synthesize`, `voice_chat`)
//! plus the OpenAI-TTS `instructions` field gate. The actual
//! audio backends (whisper-rs for STT, piper-rs for TTS, opus for
//! OGG/Opus encoding) are heavy native bindings that bloat the
//! workspace build; they land with their own follow-up PR per
//! §12.2.2 spec text. Today the tool calls return a clear
//! "backend not wired" content block so callers can probe + UI
//! the schema without a real audio rig.
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod elevenlabs;
mod openai;
mod service;

use argo_mcp::run_loop;
use argo_types::ArgoResult;
use std::io::{self, BufReader};

#[tokio::main]
async fn main() {
    let exit = match run().await {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("argo-mcp-voice: {e}");
            1
        }
    };
    std::process::exit(exit);
}

async fn run() -> ArgoResult<()> {
    let svc = service::VoiceService;
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_loop(&svc, BufReader::new(stdin.lock()), stdout.lock())
        .await
        .map_err(|e| argo_types::ArgoError::Provider(format!("voice mcp: {e}")))?;
    Ok(())
}
