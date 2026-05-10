//! `terminal` — run a shell command in the workspace via `argo-sandbox`.
//!
//! Argo runs commands through `bash -c` for portability; full sandbox
//! isolation is §15 / Phase 5. Phase 3 enforces:
//!
//! - `Capability::Spawn { kind: Subprocess }` (or `Container` for the
//!   sandbox backend) — checked at dispatch.
//! - Per-call wall-clock timeout, defaulted to 60s.
//! - Output capped at 64 KB per stream so a runaway command can't OOM
//!   the agent loop.
//!
//! ## Backends
//!
//! `backend = "local"` (default) routes through
//! [`argo_sandbox::LocalSandbox`] — `bash -c` in the current OS
//! namespace. Suitable for trusted workspaces. Requires
//! `Capability::Spawn { kind: Subprocess }`.
//!
//! `backend = "sandbox"` opt-in container-grade isolation. Today the
//! dispatch still routes through `LocalSandbox` (the structural seam
//! is in place; the real Docker / E2B microVM backends land in a
//! follow-up brief). The dispatch logs a `terminal_sandbox_requested`
//! event so audit can confirm intent and operators see when a vertical
//! requests sandboxed exec while the isolation backend is still
//! pending. Requires `Capability::Spawn { kind: Container }`.
//!
//! ## Streaming output
//!
//! When the caller attaches a [`ToolProgressEvent`] sender via
//! [`ToolContext::with_progress_tx`], the local backend bypasses the
//! `Sandbox::run` seam and executes `bash -c` directly with piped
//! stdio so per-line output reaches the agent loop / TUI / gateway
//! before the child exits. The accumulated batch return value is
//! preserved unchanged. The sandbox backend keeps the buffered path —
//! streaming through containerized backends is a follow-up.
//!
//! ```
//! use argo_tool::{Terminal, Tool};
//! let t = Terminal::default();
//! assert_eq!(t.name(), "terminal");
//! // {"command": "echo hi"}                       — Spawn::Subprocess
//! // {"command": "npm install", "backend": "sandbox"} — Spawn::Container
//! ```
use crate::tool::{
    error_text, success_text, Concurrency, Tool, ToolContext, ToolProgressEvent,
    STREAM_CHUNK_MAX_BYTES,
};
use argo_sandbox::{LocalSandbox, RunRequest, RunResult, Sandbox};
use argo_types::{ArgoError, ArgoResult, Capability, CapabilitySet, SpawnKind, ToolCallResult};
use async_trait::async_trait;
use serde::Deserialize;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;

const DEFAULT_TIMEOUT_MS: u64 = 60_000;
/// Cap per stream — generous enough for normal output, hard enough to
/// stop a `yes` loop from filling memory. Mirrors
/// `argo_sandbox::sandbox::MAX_OUTPUT_BYTES` so the streaming bypass
/// produces the same truncation marker as the sandbox seam.
const MAX_OUTPUT_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Backend {
    Local,
    Sandbox,
}

impl Backend {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "local" => Ok(Self::Local),
            "sandbox" => Ok(Self::Sandbox),
            other => Err(format!(
                "unknown backend '{other}': supported = local | sandbox"
            )),
        }
    }

    fn from_args(args: &serde_json::Value) -> Result<Self, String> {
        match args.get("backend") {
            None | Some(serde_json::Value::Null) => Ok(Self::Local),
            Some(serde_json::Value::String(s)) => Self::parse(s),
            Some(other) => Err(format!(
                "invalid backend type: expected string, got {other}"
            )),
        }
    }

    fn capability(self) -> Capability {
        match self {
            Self::Local => Capability::Spawn {
                spawn_kind: SpawnKind::Subprocess,
            },
            Self::Sandbox => Capability::Spawn {
                spawn_kind: SpawnKind::Container,
            },
        }
    }
}

/// Run a shell command via `bash -c` through the
/// [`argo_sandbox::Sandbox`] seam.
///
/// Output is capped at 64 KB per stream and the wall-clock defaults
/// to 60s. The `backend` argument (`"local"` default | `"sandbox"`)
/// picks between the local-process and (today still local-backed)
/// container seam, with capability requirements differentiated
/// accordingly.
#[derive(Debug, Clone)]
pub struct Terminal {
    local: Arc<dyn Sandbox>,
    sandboxed: Arc<dyn Sandbox>,
}

impl Default for Terminal {
    fn default() -> Self {
        // Today both backends share `LocalSandbox`; the seam is in
        // place so a future Docker / E2B / Firecracker backend slots
        // into `sandboxed` without touching the tool surface.
        let local: Arc<dyn Sandbox> = Arc::new(LocalSandbox::new());
        Self {
            local: local.clone(),
            sandboxed: local,
        }
    }
}

impl Terminal {
    /// Inject custom sandbox instances. Used by tests (and by future
    /// runtime wiring once the Docker / E2B backend lands) to swap the
    /// `"sandbox"` backend without touching the tool surface.
    #[must_use]
    pub fn with_sandboxes(local: Arc<dyn Sandbox>, sandboxed: Arc<dyn Sandbox>) -> Self {
        Self { local, sandboxed }
    }

    fn pick(&self, backend: Backend) -> &Arc<dyn Sandbox> {
        match backend {
            Backend::Local => &self.local,
            Backend::Sandbox => &self.sandboxed,
        }
    }
}

// `backend` is read directly off the raw JSON in
// [`Backend::from_args`] (and again at capability time) so it does
// not appear in this struct, but the JSON schema still advertises it
// to the model.
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
         stream; default timeout 60s. `backend` selects `local` \
         (default; Spawn::Subprocess) or `sandbox` (Spawn::Container — \
         routes through the argo-sandbox seam for isolation-grade exec)."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["command"],
            "properties": {
                "command": { "type": "string" },
                "working_dir": { "type": "string", "description": "Workspace-relative." },
                "timeout_ms": { "type": "integer", "minimum": 100 },
                "backend": {
                    "type": "string",
                    "enum": ["local", "sandbox"],
                    "default": "local",
                    "description": "`local` runs directly under bash (Spawn::Subprocess). `sandbox` routes through the argo-sandbox seam (Spawn::Container)."
                },
            },
            "additionalProperties": false,
        })
    }
    fn concurrency(&self) -> Concurrency {
        Concurrency::Serial
    }

    fn required_capabilities(&self, args: &serde_json::Value) -> ArgoResult<CapabilitySet> {
        let backend = Backend::from_args(args).map_err(|msg| ArgoError::Tool {
            tool: "terminal".into(),
            msg,
        })?;
        Ok(CapabilitySet(vec![backend.capability()]))
    }

    async fn dispatch(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> ArgoResult<ToolCallResult> {
        let started = Instant::now();
        let backend = match Backend::from_args(&args) {
            Ok(b) => b,
            Err(msg) => return Ok(err(ctx, started, msg)),
        };
        let parsed: Args = match serde_json::from_value(args) {
            Ok(a) => a,
            Err(e) => return Ok(err(ctx, started, format!("invalid arguments: {e}"))),
        };
        let cwd = parsed
            .working_dir
            .map(|d| ctx.workspace_root.join(d))
            .unwrap_or_else(|| ctx.workspace_root.clone());
        let timeout = Duration::from_millis(parsed.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS));

        if backend == Backend::Sandbox {
            // Audit hook: explicit log so operators see when a vertical
            // requests sandboxed exec while the Docker / E2B backend is
            // still pending. The `resolved_backend_id` field reveals
            // whether the seam has been swapped (e.g. `"docker"`) or
            // is still falling back to local execution.
            tracing::info!(
                target: "argo.tool.terminal",
                event = "terminal_sandbox_requested",
                tool = "terminal",
                backend = "sandbox",
                resolved_backend_id = self.pick(backend).id(),
                command_preview = truncate(&parsed.command, 200).as_str(),
                "terminal sandbox-mode requested"
            );
        }

        // Streaming bypass: the local backend executes `bash -c`
        // directly with piped stdio so per-line output reaches the
        // agent loop before the child exits. The sandbox backend
        // keeps the buffered seam — streaming through container
        // backends is a follow-up. The non-streaming path stays
        // byte-for-byte identical to the previous implementation.
        if let (Some(tx), Backend::Local) = (ctx.progress_tx.as_ref(), backend) {
            return Ok(match run_streaming(&parsed.command, &cwd, timeout, tx.clone()).await {
                Ok(result) => {
                    into_call_result(ctx, started, &parsed.command, timeout, &result)
                }
                Err(e) => err(ctx, started, format!("terminal exec failed: {e}")),
            });
        }

        let mut req = RunRequest::new(parsed.command.clone(), cwd);
        req.timeout = Some(timeout);
        match self.pick(backend).run(&req).await {
            Ok(result) => Ok(into_call_result(ctx, started, &parsed.command, timeout, &result)),
            Err(e) => Ok(err(ctx, started, format!("terminal exec failed: {e}"))),
        }
    }
}

async fn run_streaming(
    command: &str,
    cwd: &std::path::Path,
    timeout: Duration,
    tx: mpsc::Sender<ToolProgressEvent>,
) -> std::io::Result<RunResult> {
    let started = Instant::now();
    let mut child = tokio::process::Command::new("bash")
        .arg("-c")
        .arg(command)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| std::io::Error::other("failed to capture stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| std::io::Error::other("failed to capture stderr"))?;
    let stdout_handle =
        tokio::spawn(pump_stream(stdout, tx.clone(), StreamKind::Stdout));
    let stderr_handle =
        tokio::spawn(pump_stream(stderr, tx.clone(), StreamKind::Stderr));

    let (timed_out, status) = match tokio::time::timeout(timeout, child.wait()).await {
        Ok(Ok(s)) => (false, Some(s)),
        Ok(Err(e)) => return Err(e),
        Err(_) => {
            // Timeout: kill child and let the pump tasks drain.
            let _ = child.start_kill();
            let _ = child.wait().await;
            (true, None)
        }
    };

    let (stdout_head, stdout_total, stdout_dropped) = stdout_handle
        .await
        .map_err(|e| std::io::Error::other(format!("stdout pump panicked: {e}")))?;
    let (stderr_head, stderr_total, stderr_dropped) = stderr_handle
        .await
        .map_err(|e| std::io::Error::other(format!("stderr pump panicked: {e}")))?;

    let exit_code = match (timed_out, status) {
        (false, Some(s)) => s.code().unwrap_or(-1),
        _ => -1,
    };

    if stdout_dropped > 0 || stderr_dropped > 0 {
        tracing::warn!(
            target: "argo.tool.terminal",
            event = "terminal_stream_backpressure",
            stdout_dropped_bytes = stdout_dropped,
            stderr_dropped_bytes = stderr_dropped,
            "terminal progress channel full; dropped chunks"
        );
    }

    let _ = tx.try_send(ToolProgressEvent::Exited { exit_code });

    Ok(RunResult {
        stdout: render_capped(&stdout_head, stdout_total),
        stderr: if timed_out {
            format!(
                "local-sandbox: command exceeded {} ms timeout",
                timeout.as_millis()
            )
        } else {
            render_capped(&stderr_head, stderr_total)
        },
        exit_code,
        timed_out,
        duration: started.elapsed(),
    })
}

#[derive(Copy, Clone)]
enum StreamKind {
    Stdout,
    Stderr,
}

/// Read `reader` line-by-line, append every line (with a trailing `\n`
/// to mirror the buffered path's raw stdout) into a capped head
/// buffer, and `try_send` each line as a progress event. On a full
/// channel the chunk is dropped and the dropped-bytes counter is
/// bumped — the child must never block on a slow consumer.
///
/// Returns `(head_bytes_up_to_cap, total_bytes_seen, dropped_bytes)`.
async fn pump_stream<R>(
    reader: R,
    tx: mpsc::Sender<ToolProgressEvent>,
    kind: StreamKind,
) -> (Vec<u8>, usize, usize)
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut head: Vec<u8> = Vec::new();
    let mut total: usize = 0;
    let mut dropped: usize = 0;
    let mut lines = BufReader::new(reader).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let line_with_nl = line.len().saturating_add(1);
        total = total.saturating_add(line_with_nl);
        if head.len() < MAX_OUTPUT_BYTES {
            let remaining = MAX_OUTPUT_BYTES - head.len();
            let take = remaining.min(line.len());
            head.extend_from_slice(&line.as_bytes()[..take]);
            if take == line.len() && head.len() < MAX_OUTPUT_BYTES {
                head.push(b'\n');
            }
        }
        for chunk in split_chunks(&line, STREAM_CHUNK_MAX_BYTES) {
            let event = match kind {
                StreamKind::Stdout => ToolProgressEvent::Stdout(chunk.to_string()),
                StreamKind::Stderr => ToolProgressEvent::Stderr(chunk.to_string()),
            };
            if tx.try_send(event).is_err() {
                dropped = dropped.saturating_add(chunk.len());
            }
        }
    }
    (head, total, dropped)
}

/// Split `s` into substrings whose UTF-8 length is at most `cap`
/// bytes, honoring char boundaries.
fn split_chunks(s: &str, cap: usize) -> Vec<&str> {
    if s.len() <= cap {
        return vec![s];
    }
    let mut out = Vec::new();
    let mut start = 0;
    while start < s.len() {
        let mut end = (start + cap).min(s.len());
        while end > start && !s.is_char_boundary(end) {
            end -= 1;
        }
        if end == start {
            // Pathological multi-byte char wider than cap; fall back
            // to a single-codepoint slice to make forward progress.
            end = (start + 1..=s.len())
                .find(|&i| s.is_char_boundary(i))
                .unwrap_or(s.len());
        }
        out.push(&s[start..end]);
        start = end;
    }
    out
}

/// Render a capped output buffer to a String, attaching the same
/// truncation marker the buffered path produces. `total` is the full
/// byte-length seen on the stream; `head` is the first
/// `MAX_OUTPUT_BYTES` of it.
fn render_capped(head: &[u8], total: usize) -> String {
    if total <= head.len() {
        return String::from_utf8_lossy(head).into_owned();
    }
    let s = String::from_utf8_lossy(head).into_owned();
    format!(
        "{s}\n... [truncated {} bytes from this stream]",
        total - head.len()
    )
}

fn err(ctx: &ToolContext, started: Instant, msg: String) -> ToolCallResult {
    error_text(ctx.call_id, msg, started.elapsed().as_millis() as u64)
}

fn into_call_result(
    ctx: &ToolContext,
    started: Instant,
    command: &str,
    timeout: Duration,
    result: &RunResult,
) -> ToolCallResult {
    if result.timed_out {
        return error_text(
            ctx.call_id,
            format!(
                "terminal exec exceeded {} ms timeout (`{}`)",
                timeout.as_millis(),
                truncate(command, 200)
            ),
            started.elapsed().as_millis() as u64,
        );
    }
    success_text(
        ctx.call_id,
        format_output(&result.stdout, &result.stderr, result.exit_code),
        started.elapsed().as_millis() as u64,
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
#[path = "terminal_tests.rs"]
mod tests;
