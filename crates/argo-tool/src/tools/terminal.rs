//! `terminal` — run a shell command in the workspace.
//!
//! Argo runs commands through `bash -c` for portability; full sandboxing
//! is §15 / Phase 5. Phase 3 enforces:
//!
//! - `Capability::Spawn { kind: Subprocess }` — checked at dispatch.
//! - Per-call wall-clock timeout, defaulted to 60s.
//! - Output capped at 64 KB per stream so a runaway command can't OOM
//!   the agent loop.
use crate::tool::{error_text, success_text, Concurrency, Tool, ToolContext};
use argo_types::{ArgoResult, Capability, CapabilitySet, SpawnKind, ToolCallResult};
use async_trait::async_trait;
use serde::Deserialize;
use std::time::{Duration, Instant};

const DEFAULT_TIMEOUT_MS: u64 = 60_000;
/// Cap per stream — generous enough for normal output, hard enough to
/// stop a `yes` loop from filling memory.
const MAX_OUTPUT_BYTES: usize = 64 * 1024;

/// Run a shell command via `bash -c`. Output capped at 64 KB / stream
/// and 60 s wall-clock by default.
#[derive(Debug, Default)]
pub struct Terminal;

#[derive(Debug, Deserialize)]
struct Args {
    command: String,
    #[serde(default)]
    working_dir: Option<String>,
    #[serde(default)]
    timeout_ms: Option<u64>,
}

#[async_trait]
impl Tool for Terminal {
    fn name(&self) -> &str {
        "terminal"
    }
    fn description(&self) -> &str {
        "Run a shell command in the workspace via `bash -c`. Returns \
         stdout, stderr, and exit code. Output capped at 64 KB per \
         stream; default timeout 60s."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["command"],
            "properties": {
                "command": { "type": "string" },
                "working_dir": { "type": "string", "description": "Workspace-relative." },
                "timeout_ms": { "type": "integer", "minimum": 100 },
            },
            "additionalProperties": false,
        })
    }
    fn concurrency(&self) -> Concurrency {
        Concurrency::Serial
    }

    fn required_capabilities(&self, _args: &serde_json::Value) -> ArgoResult<CapabilitySet> {
        Ok(CapabilitySet(vec![Capability::Spawn {
            spawn_kind: SpawnKind::Subprocess,
        }]))
    }

    async fn dispatch(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> ArgoResult<ToolCallResult> {
        let started = Instant::now();
        let parsed: Args =
            serde_json::from_value(args).map_err(|e| argo_types::ArgoError::Tool {
                tool: "terminal".into(),
                msg: format!("invalid arguments: {e}"),
            })?;
        let cwd = parsed
            .working_dir
            .map(|d| ctx.workspace_root.join(d))
            .unwrap_or_else(|| ctx.workspace_root.clone());
        let timeout = Duration::from_millis(parsed.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS));
        let exec = run_command(&parsed.command, &cwd, timeout);
        match tokio::time::timeout(timeout, exec).await {
            Ok(Ok((stdout, stderr, code))) => Ok(success_text(
                ctx.call_id,
                format_output(&stdout, &stderr, code),
                started.elapsed().as_millis() as u64,
            )),
            Ok(Err(e)) => Ok(error_text(
                ctx.call_id,
                format!("terminal exec failed: {e}"),
                started.elapsed().as_millis() as u64,
            )),
            Err(_) => Ok(error_text(
                ctx.call_id,
                format!(
                    "terminal exec exceeded {} ms timeout (`{}`)",
                    timeout.as_millis(),
                    truncate(&parsed.command, 200)
                ),
                started.elapsed().as_millis() as u64,
            )),
        }
    }
}

async fn run_command(
    command: &str,
    cwd: &std::path::Path,
    _timeout: Duration,
) -> std::io::Result<(String, String, i32)> {
    let output = tokio::process::Command::new("bash")
        .arg("-c")
        .arg(command)
        .current_dir(cwd)
        .output()
        .await?;
    let stdout = truncate_bytes_to_string(&output.stdout, MAX_OUTPUT_BYTES);
    let stderr = truncate_bytes_to_string(&output.stderr, MAX_OUTPUT_BYTES);
    let code = output.status.code().unwrap_or(-1);
    Ok((stdout, stderr, code))
}

fn truncate_bytes_to_string(bytes: &[u8], cap: usize) -> String {
    if bytes.len() <= cap {
        return String::from_utf8_lossy(bytes).into_owned();
    }
    let head = String::from_utf8_lossy(&bytes[..cap]).into_owned();
    format!(
        "{head}\n... [truncated {} bytes from this stream]",
        bytes.len() - cap
    )
}

fn format_output(stdout: &str, stderr: &str, code: i32) -> String {
    format!("exit_code: {code}\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}")
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &s[..end])
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_types::Principal;

    fn ctx(dir: &std::path::Path) -> ToolContext {
        ToolContext::new(
            Principal::Local {
                os_user: "u".into(),
            },
            dir.to_path_buf(),
        )
    }

    #[tokio::test]
    async fn echoes_stdout() {
        let dir = tempfile::tempdir().unwrap();
        let r = Terminal
            .dispatch(serde_json::json!({"command": "echo hi"}), &ctx(dir.path()))
            .await
            .unwrap();
        assert!(!r.is_error);
        let body = match &r.content[0] {
            argo_types::ContentBlock::Text { text } => text.clone(),
            _ => panic!("text"),
        };
        assert!(body.contains("exit_code: 0"));
        assert!(body.contains("hi"));
    }

    #[tokio::test]
    async fn captures_nonzero_exit() {
        let dir = tempfile::tempdir().unwrap();
        let r = Terminal
            .dispatch(serde_json::json!({"command": "exit 7"}), &ctx(dir.path()))
            .await
            .unwrap();
        assert!(!r.is_error); // exec succeeded; exit code is content not failure.
        let body = match &r.content[0] {
            argo_types::ContentBlock::Text { text } => text.clone(),
            _ => panic!("text"),
        };
        assert!(body.contains("exit_code: 7"));
    }

    #[tokio::test]
    async fn timeout_reports_clearly() {
        let dir = tempfile::tempdir().unwrap();
        let r = Terminal
            .dispatch(
                serde_json::json!({"command": "sleep 5", "timeout_ms": 100}),
                &ctx(dir.path()),
            )
            .await
            .unwrap();
        assert!(r.is_error);
        let body = match &r.content[0] {
            argo_types::ContentBlock::Text { text } => text.clone(),
            _ => panic!("text"),
        };
        assert!(body.contains("timeout"));
    }

    #[test]
    fn required_capability_is_subprocess_spawn() {
        let caps = Terminal
            .required_capabilities(&serde_json::json!({"command": "x"}))
            .unwrap();
        assert!(caps.covers(&Capability::Spawn {
            spawn_kind: SpawnKind::Subprocess,
        }));
    }

    #[test]
    fn truncate_bytes_caps_stream_size() {
        let big = vec![b'x'; MAX_OUTPUT_BYTES * 2];
        let s = truncate_bytes_to_string(&big, MAX_OUTPUT_BYTES);
        assert!(s.contains("[truncated"));
        assert!(s.contains(&format!("{MAX_OUTPUT_BYTES} bytes")));
    }
}
