//! "Local" voice backend — shells out to user-provided binaries
//! (whisper.cpp, piper, espeak, anything else that reads stdin and
//! writes stdout). Avoids pulling whisper-rs / piper-rs and their
//! C++ build-time deps into the workspace; the trade-off is that
//! the user has to install + configure the underlying tool.
//!
//! Configuration:
//!   - `ARGO_TRANSCRIBE_CMD` — shell command line for STT. The
//!     audio file path is appended as the last argument; stdout is
//!     captured as the transcript.
//!   - `ARGO_SYNTHESIZE_CMD` — shell command line for TTS. The
//!     text is piped on stdin; stdout is captured as raw audio
//!     bytes (mp3 / wav / ogg — caller's choice). Output is
//!     written to a tempfile to match the existing OpenAI TTS
//!     contract; the file path comes back as a Text content block.
//!
//! Example: `ARGO_TRANSCRIBE_CMD="whisper-cli -m ggml-base.en.bin -nt"`
//! then `transcribe { audio: "...", backend: "local" }` runs
//! `whisper-cli -m ggml-base.en.bin -nt <audio_path>` and returns
//! its stdout verbatim as the transcript.
use argo_mcp::{CallToolResult, McpContent};
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

const TRANSCRIBE_ENV: &str = "ARGO_TRANSCRIBE_CMD";
const SYNTHESIZE_ENV: &str = "ARGO_SYNTHESIZE_CMD";

fn err(text: String) -> CallToolResult {
    CallToolResult {
        content: vec![McpContent::Text { text }],
        is_error: true,
    }
}

/// Run `$ARGO_TRANSCRIBE_CMD <audio_path>` and return stdout as
/// the transcript.
pub async fn transcribe(audio: &str) -> CallToolResult {
    let raw = match std::env::var(TRANSCRIBE_ENV) {
        Ok(c) if !c.trim().is_empty() => c,
        _ => {
            return err(format!(
                "{TRANSCRIBE_ENV} is not set; export it to a CLI like \
                 `whisper-cli -m <model> -nt` to use backend=local"
            ));
        }
    };
    let mut parts = raw.split_whitespace();
    let Some(program) = parts.next() else {
        return err(format!("{TRANSCRIBE_ENV} is empty after splitting"));
    };
    let mut cmd = tokio::process::Command::new(program);
    cmd.args(parts);
    cmd.arg(audio);
    let out = match cmd.output().await {
        Ok(o) => o,
        Err(e) => return err(format!("spawn {program}: {e}")),
    };
    if !out.status.success() {
        return err(format!(
            "{program} exited {} — stderr: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    CallToolResult {
        content: vec![McpContent::Text { text }],
        is_error: false,
    }
}

/// Run `$ARGO_SYNTHESIZE_CMD` with `text` piped to stdin, write
/// the resulting audio bytes to a tempfile, and return the path
/// as a Text content block (matches the OpenAI TTS contract).
pub async fn synthesize(text: &str, format: &str) -> CallToolResult {
    let raw = match std::env::var(SYNTHESIZE_ENV) {
        Ok(c) if !c.trim().is_empty() => c,
        _ => {
            return err(format!(
                "{SYNTHESIZE_ENV} is not set; export it to a CLI like \
                 `piper --model <m> --output_file -` to use backend=local"
            ));
        }
    };
    let mut parts = raw.split_whitespace();
    let Some(program) = parts.next() else {
        return err(format!("{SYNTHESIZE_ENV} is empty after splitting"));
    };
    let mut cmd = tokio::process::Command::new(program);
    cmd.args(parts);
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return err(format!("spawn {program}: {e}")),
    };
    if let Some(mut stdin) = child.stdin.take() {
        if let Err(e) = stdin.write_all(text.as_bytes()).await {
            return err(format!("stdin write: {e}"));
        }
        drop(stdin);
    }
    let out = match child.wait_with_output().await {
        Ok(o) => o,
        Err(e) => return err(format!("wait {program}: {e}")),
    };
    if !out.status.success() {
        return err(format!(
            "{program} exited {} — stderr: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let ext = match format {
        "ogg-opus" => "ogg",
        "wav" => "wav",
        _ => "mp3",
    };
    let path = tempfile_path("argo-tts-local", ext);
    if let Err(e) = std::fs::write(&path, &out.stdout) {
        return err(format!("write {}: {e}", path.display()));
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
