//! Plugin subprocess host.
//!
//! Spawns a plugin binary, performs the JSON-RPC handshake, then
//! exposes a typed [`PluginHost::call`] that maps each Argo → plugin
//! method to a request/response round-trip over the plugin's
//! stdin/stdout.
//!
//! Crash handling: if the plugin exits or closes stdout while we have
//! an in-flight call, that call resolves to [`PluginError::Crash`] and
//! every subsequent call rejects with the same. The agent loop catches
//! this and continues — `PluginCrash` does NOT halt the loop
//! (§11.5 acceptance).
use crate::error::{PluginError, PluginResult};
use crate::protocol::{
    HandshakeRequest, HandshakeResponse, JsonRpcRequest, JsonRpcResponse, PROTOCOL_VERSION,
};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::task::JoinHandle;

/// Default per-call timeout — plugins that block longer surface as
/// `PluginError::Timeout` so the agent loop never stalls.
pub const DEFAULT_CALL_TIMEOUT: Duration = Duration::from_secs(30);

/// Handshake-specific timeout. Decoupled from `call_timeout` so a tight
/// per-call budget for tests / hot paths doesn't kill the one-time
/// startup handshake on slower systems / debug builds.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

/// In-flight call table — keyed by request id, drained by the reader
/// task when the response arrives or the plugin crashes.
type PendingMap = Arc<Mutex<HashMap<u64, oneshot::Sender<PluginResult<JsonRpcResponse>>>>>;

/// One running plugin.
pub struct PluginHost {
    name: String,
    next_id: AtomicU64,
    pending: PendingMap,
    stdin: Arc<Mutex<ChildStdin>>,
    shutdown_flag: Arc<AtomicBool>,
    crash_reason: Arc<Mutex<Option<String>>>,
    reader_task: Mutex<Option<JoinHandle<()>>>,
    child: Mutex<Option<Child>>,
    call_timeout: Duration,
}

impl std::fmt::Debug for PluginHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginHost")
            .field("name", &self.name)
            .field("call_timeout_ms", &self.call_timeout.as_millis())
            .finish()
    }
}

impl PluginHost {
    /// Spawn the plugin binary and complete the handshake.
    pub async fn start(
        program: impl AsRef<std::ffi::OsStr>,
        args: &[String],
    ) -> PluginResult<Self> {
        Self::start_with_timeout(program, args, DEFAULT_CALL_TIMEOUT).await
    }

    /// Same as [`Self::start`] but with a custom per-call timeout.
    pub async fn start_with_timeout(
        program: impl AsRef<std::ffi::OsStr>,
        args: &[String],
        call_timeout: Duration,
    ) -> PluginResult<Self> {
        let mut child = Command::new(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| PluginError::Spawn(e.to_string()))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| PluginError::Spawn("plugin stdin missing".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| PluginError::Spawn("plugin stdout missing".into()))?;

        let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));
        let crash_reason = Arc::new(Mutex::new(None));
        let shutdown_flag = Arc::new(AtomicBool::new(false));

        let reader_pending = pending.clone();
        let reader_crash = crash_reason.clone();
        let reader_task = tokio::spawn(read_loop(stdout, reader_pending, reader_crash));

        let host = Self {
            name: String::new(),
            next_id: AtomicU64::new(1),
            pending,
            stdin: Arc::new(Mutex::new(stdin)),
            shutdown_flag,
            crash_reason,
            reader_task: Mutex::new(Some(reader_task)),
            child: Mutex::new(Some(child)),
            call_timeout,
        };
        let handshake = handshake(&host).await?;
        Ok(Self {
            name: handshake.name,
            ..host
        })
    }

    /// Plugin name reported during handshake.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Send `method` with `params` and await the response. Times out
    /// after the configured `call_timeout`.
    pub async fn call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> PluginResult<serde_json::Value> {
        self.call_with_deadline(method, params, self.call_timeout)
            .await
    }

    /// Send `method` with a custom timeout (used internally by
    /// `handshake` so a tight per-call budget doesn't kill startup).
    async fn call_with_deadline(
        &self,
        method: &str,
        params: serde_json::Value,
        timeout: Duration,
    ) -> PluginResult<serde_json::Value> {
        if self.shutdown_flag.load(Ordering::SeqCst) {
            return Err(PluginError::Shutdown);
        }
        if let Some(reason) = self.crash_reason.lock().await.clone() {
            return Err(PluginError::Crash(reason));
        }
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let req = JsonRpcRequest::new(id, method, params);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);
        if let Err(e) = write_one(&self.stdin, &req).await {
            self.pending.lock().await.remove(&id);
            return Err(e);
        }
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(Ok(resp))) => match (resp.result, resp.error) {
                (Some(v), None) => Ok(v),
                (None, Some(e)) => Err(PluginError::from(e)),
                _ => Err(PluginError::Decode(
                    "response has neither result nor error".into(),
                )),
            },
            Ok(Ok(Err(e))) => Err(e),
            Ok(Err(_)) => Err(PluginError::Crash(
                "response channel dropped (reader died)".into(),
            )),
            Err(_) => {
                self.pending.lock().await.remove(&id);
                Err(PluginError::Timeout(timeout.as_millis() as u64))
            }
        }
    }

    /// Send the `shutdown` notification, then wait briefly for the
    /// child to exit. Subsequent calls reject with
    /// [`PluginError::Shutdown`].
    pub async fn shutdown(&self) -> PluginResult<()> {
        if self.shutdown_flag.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        let req = JsonRpcRequest::new(0, "shutdown", serde_json::json!({}));
        // Best-effort: ignore a write error here — the plugin may have
        // already exited (which is the normal shutdown completion path).
        let _ = write_one(&self.stdin, &req).await;
        if let Some(mut child) = self.child.lock().await.take() {
            let _ = tokio::time::timeout(Duration::from_millis(500), child.wait()).await;
            let _ = child.kill().await;
        }
        if let Some(handle) = self.reader_task.lock().await.take() {
            handle.abort();
        }
        Ok(())
    }
}

// `child` was built with `kill_on_drop(true)` so the OS reaps the
// subprocess when the host is dropped — no explicit Drop impl needed.

async fn read_loop(
    stdout: tokio::process::ChildStdout,
    pending: PendingMap,
    crash_reason: Arc<Mutex<Option<String>>>,
) {
    let mut reader = BufReader::new(stdout).lines();
    loop {
        match reader.next_line().await {
            Ok(Some(line)) => {
                if line.trim().is_empty() {
                    continue;
                }
                match serde_json::from_str::<JsonRpcResponse>(&line) {
                    Ok(resp) => deliver(&pending, resp).await,
                    Err(e) => {
                        tracing::warn!(error = %e, line, "plugin emitted non-response line");
                    }
                }
            }
            Ok(None) | Err(_) => {
                let reason = "plugin closed stdout".to_string();
                *crash_reason.lock().await = Some(reason.clone());
                fail_pending(&pending, reason).await;
                return;
            }
        }
    }
}

async fn deliver(pending: &PendingMap, resp: JsonRpcResponse) {
    let Some(id) = resp.id.as_u64() else {
        tracing::warn!(?resp.id, "plugin response with non-numeric id");
        return;
    };
    if let Some(tx) = pending.lock().await.remove(&id) {
        let _ = tx.send(Ok(resp));
    }
}

async fn fail_pending(pending: &PendingMap, reason: String) {
    let mut g = pending.lock().await;
    let drained: Vec<_> = g.drain().collect();
    drop(g);
    for (_, tx) in drained {
        let _ = tx.send(Err(PluginError::Crash(reason.clone())));
    }
}

async fn write_one(stdin: &Arc<Mutex<ChildStdin>>, req: &JsonRpcRequest) -> PluginResult<()> {
    let body = serde_json::to_string(req)
        .map_err(|e| PluginError::Decode(format!("request encode: {e}")))?;
    let mut g = stdin.lock().await;
    g.write_all(body.as_bytes())
        .await
        .map_err(|e| PluginError::Io(format!("stdin write: {e}")))?;
    g.write_all(b"\n")
        .await
        .map_err(|e| PluginError::Io(format!("stdin newline: {e}")))?;
    g.flush()
        .await
        .map_err(|e| PluginError::Io(format!("stdin flush: {e}")))?;
    Ok(())
}

async fn handshake(host: &PluginHost) -> PluginResult<HandshakeResponse> {
    let resp = host
        .call_with_deadline(
            "handshake",
            serde_json::to_value(HandshakeRequest {
                protocol_version: PROTOCOL_VERSION.into(),
            })
            .map_err(|e| PluginError::Handshake(format!("encode: {e}")))?,
            HANDSHAKE_TIMEOUT,
        )
        .await?;
    let body: HandshakeResponse =
        serde_json::from_value(resp).map_err(|e| PluginError::Handshake(format!("decode: {e}")))?;
    if body.protocol_version != PROTOCOL_VERSION {
        return Err(PluginError::Handshake(format!(
            "protocol version mismatch: argo speaks {} but plugin reported {}",
            PROTOCOL_VERSION, body.protocol_version
        )));
    }
    Ok(body)
}

// Allow channel-typed mpsc unused warnings — kept for the upcoming
// plugin-→-Argo notification path (Phase 4 follow-up).
#[allow(dead_code)]
type NotificationTx = mpsc::Sender<JsonRpcRequest>;

mod _tests_use {
    #[allow(unused_imports)]
    use super::*;
}
