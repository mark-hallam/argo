//! Streaming bypass for the local `terminal` backend.
//!
//! Split out of `terminal.rs` to keep that file under the 500-LOC
//! invariant once the `background: true` dispatch path landed
//! alongside the existing local/sandbox routing.
//!
//! When `ToolContext::progress_tx` is `Some(tx)`, the local backend
//! executes `bash -c` directly with piped stdio so per-line output
//! reaches the agent loop / TUI / gateway before the child exits, and
//! emits a final `Exited` sentinel. The accumulated batch result is
//! still returned so existing callers see byte-identical behavior.

use crate::tool::{ToolProgressEvent, STREAM_CHUNK_MAX_BYTES};
use argo_sandbox::RunResult;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;

/// Cap per stream — generous enough for normal output, hard enough
/// to stop a `yes` loop from filling memory. Mirrors
/// `argo_sandbox::sandbox::MAX_OUTPUT_BYTES` so the streaming bypass
/// produces the same truncation marker as the sandbox seam.
const MAX_OUTPUT_BYTES: usize = 64 * 1024;

#[derive(Copy, Clone)]
enum StreamKind {
    Stdout,
    Stderr,
}

pub(crate) async fn run_streaming(
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
    let stdout_handle = tokio::spawn(pump_stream(stdout, tx.clone(), StreamKind::Stdout));
    let stderr_handle = tokio::spawn(pump_stream(stderr, tx.clone(), StreamKind::Stderr));

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
pub(crate) fn split_chunks(s: &str, cap: usize) -> Vec<&str> {
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
pub(crate) fn render_capped(head: &[u8], total: usize) -> String {
    if total <= head.len() {
        return String::from_utf8_lossy(head).into_owned();
    }
    let s = String::from_utf8_lossy(head).into_owned();
    format!(
        "{s}\n... [truncated {} bytes from this stream]",
        total - head.len()
    )
}
