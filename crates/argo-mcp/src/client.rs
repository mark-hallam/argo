//! MCP client — stdio transport.
//!
//! Spawns an MCP server as a subprocess, completes the
//! `initialize` → `notifications/initialized` handshake, and exposes a
//! typed surface for `tools/list` and `tools/call`.
//!
//! Crash handling: `McpError::Crash` if the server exits or closes
//! stdout mid-call. The agent loop catches this and may retry once
//! with backoff (§12.3 acceptance). Subsequent calls reject cleanly
//! rather than hang.
use crate::error::{McpError, McpResult};
use crate::protocol::{
    CallToolParams, CallToolResult, ClientInfo, InitializeParams, InitializeResult,
    JsonRpcRequest, JsonRpcResponse, ListToolsResult, McpToolSchema, ARGO_CLIENT_NAME,
    ARGO_CLIENT_VERSION, MCP_PROTOCOL_VERSION,
};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{oneshot, Mutex};
use tokio::task::JoinHandle;

/// Default per-call timeout. MCP servers are normally fast; 30 s is
/// generous enough for browser navigations.
pub const DEFAULT_CALL_TIMEOUT: Duration = Duration::from_secs(30);

/// `initialize` carries its own deadline so a tight per-call timeout
/// for tests / hot paths doesn't kill startup.
const INIT_TIMEOUT: Duration = Duration::from_secs(5);

/// Pending-call table: id → response channel.
type PendingMap = Arc<Mutex<HashMap<u64, oneshot::Sender<McpResult<JsonRpcResponse>>>>>;

/// Stdio-transport MCP client.
pub struct McpClient {
    server_info: crate::protocol::ServerInfo,
    next_id: AtomicU64,
    pending: PendingMap,
    stdin: Arc<Mutex<ChildStdin>>,
    shutdown_flag: Arc<AtomicBool>,
    crash_reason: Arc<Mutex<Option<String>>>,
    reader_task: Mutex<Option<JoinHandle<()>>>,
    child: Mutex<Option<Child>>,
    call_timeout: Duration,
}

impl std::fmt::Debug for McpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpClient")
            .field("server_name", &self.server_info.name)
            .field("server_version", &self.server_info.version)
            .field("call_timeout_ms", &self.call_timeout.as_millis())
            .finish()
    }
}

impl McpClient {
    /// Spawn an MCP server over stdio and complete `initialize`.
    pub async fn connect_stdio(
        program: impl AsRef<std::ffi::OsStr>,
        args: &[String],
    ) -> McpResult<Self> {
        Self::connect_stdio_with_timeout(program, args, DEFAULT_CALL_TIMEOUT).await
    }

    /// Same as [`Self::connect_stdio`] with a custom per-call timeout.
    pub async fn connect_stdio_with_timeout(
        program: impl AsRef<std::ffi::OsStr>,
        args: &[String],
        call_timeout: Duration,
    ) -> McpResult<Self> {
        let mut child = Command::new(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| McpError::Spawn(e.to_string()))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| McpError::Spawn("server stdin missing".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| McpError::Spawn("server stdout missing".into()))?;
        let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));
        let crash_reason = Arc::new(Mutex::new(None));
        let reader_pending = pending.clone();
        let reader_crash = crash_reason.clone();
        let reader_task = tokio::spawn(read_loop(stdout, reader_pending, reader_crash));
        let mut client = Self {
            server_info: crate::protocol::ServerInfo {
                name: String::new(),
                version: String::new(),
            },
            next_id: AtomicU64::new(1),
            pending,
            stdin: Arc::new(Mutex::new(stdin)),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            crash_reason,
            reader_task: Mutex::new(Some(reader_task)),
            child: Mutex::new(Some(child)),
            call_timeout,
        };
        let init = client.initialize().await?;
        client.server_info = init.server_info;
        client.send_initialized_notification().await?;
        Ok(client)
    }

    /// Server identity reported during `initialize`.
    #[must_use]
    pub fn server_info(&self) -> &crate::protocol::ServerInfo {
        &self.server_info
    }

    /// `tools/list` — every tool the server exposes.
    pub async fn list_tools(&self) -> McpResult<Vec<McpToolSchema>> {
        let v = self
            .call_method("tools/list", None, self.call_timeout)
            .await?;
        let parsed: ListToolsResult = serde_json::from_value(v)
            .map_err(|e| McpError::Decode(format!("tools/list result: {e}")))?;
        Ok(parsed.tools)
    }

    /// `tools/call` — invoke `name` with `arguments`.
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> McpResult<CallToolResult> {
        let params = CallToolParams {
            name: name.into(),
            arguments,
        };
        let v = self
            .call_method(
                "tools/call",
                Some(
                    serde_json::to_value(params)
                        .map_err(|e| McpError::Decode(format!("tools/call params: {e}")))?,
                ),
                self.call_timeout,
            )
            .await?;
        serde_json::from_value(v).map_err(|e| McpError::Decode(format!("tools/call result: {e}")))
    }

    /// Best-effort shutdown: cancel reader task, kill subprocess.
    pub async fn shutdown(&self) -> McpResult<()> {
        if self.shutdown_flag.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        if let Some(mut child) = self.child.lock().await.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        if let Some(handle) = self.reader_task.lock().await.take() {
            handle.abort();
        }
        Ok(())
    }

    async fn initialize(&self) -> McpResult<InitializeResult> {
        let params = InitializeParams {
            protocol_version: MCP_PROTOCOL_VERSION.into(),
            capabilities: serde_json::json!({}),
            client_info: ClientInfo {
                name: ARGO_CLIENT_NAME.into(),
                version: ARGO_CLIENT_VERSION.into(),
            },
        };
        let v = self
            .call_method(
                "initialize",
                Some(
                    serde_json::to_value(params)
                        .map_err(|e| McpError::Initialize(format!("encode: {e}")))?,
                ),
                INIT_TIMEOUT,
            )
            .await?;
        let parsed: InitializeResult =
            serde_json::from_value(v).map_err(|e| McpError::Initialize(format!("decode: {e}")))?;
        if parsed.protocol_version != MCP_PROTOCOL_VERSION {
            return Err(McpError::Initialize(format!(
                "protocol version mismatch: argo speaks {} but server reported {}",
                MCP_PROTOCOL_VERSION, parsed.protocol_version
            )));
        }
        Ok(parsed)
    }

    async fn send_initialized_notification(&self) -> McpResult<()> {
        // `notifications/initialized` is a notification (no id), per
        // MCP spec. We use id=0 so the server treats it the same way
        // most JSON-RPC libs do; servers MUST NOT respond.
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        });
        let line = serde_json::to_string(&body)
            .map_err(|e| McpError::Decode(format!("notify encode: {e}")))?;
        write_line(&self.stdin, &line).await
    }

    async fn call_method(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
        timeout: Duration,
    ) -> McpResult<serde_json::Value> {
        if self.shutdown_flag.load(Ordering::SeqCst) {
            return Err(McpError::Shutdown);
        }
        if let Some(reason) = self.crash_reason.lock().await.clone() {
            return Err(McpError::Crash(reason));
        }
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let req = JsonRpcRequest::new(id, method, params);
        let body = serde_json::to_string(&req)
            .map_err(|e| McpError::Decode(format!("request encode: {e}")))?;
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);
        if let Err(e) = write_line(&self.stdin, &body).await {
            self.pending.lock().await.remove(&id);
            return Err(e);
        }
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(Ok(resp))) => match (resp.result, resp.error) {
                (Some(v), None) => Ok(v),
                (None, Some(e)) => Err(McpError::from(e)),
                _ => Err(McpError::Decode(
                    "MCP response has neither result nor error".into(),
                )),
            },
            Ok(Ok(Err(e))) => Err(e),
            Ok(Err(_)) => Err(McpError::Crash(
                "response channel dropped (reader died)".into(),
            )),
            Err(_) => {
                self.pending.lock().await.remove(&id);
                Err(McpError::Timeout(timeout.as_millis() as u64))
            }
        }
    }
}

async fn write_line(stdin: &Arc<Mutex<ChildStdin>>, line: &str) -> McpResult<()> {
    let mut g = stdin.lock().await;
    g.write_all(line.as_bytes())
        .await
        .map_err(|e| McpError::Io(format!("stdin write: {e}")))?;
    g.write_all(b"\n")
        .await
        .map_err(|e| McpError::Io(format!("stdin newline: {e}")))?;
    g.flush()
        .await
        .map_err(|e| McpError::Io(format!("stdin flush: {e}")))?;
    Ok(())
}

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
                        // Could be a server-→-client request /
                        // notification (servers shouldn't send these
                        // before we register handlers, but they're
                        // legal). Phase 1 just logs.
                        tracing::debug!(line, error = %e, "MCP non-response line");
                    }
                }
            }
            Ok(None) | Err(_) => {
                let reason = "MCP server closed stdout".to_string();
                *crash_reason.lock().await = Some(reason.clone());
                fail_pending(&pending, reason).await;
                return;
            }
        }
    }
}

async fn deliver(pending: &PendingMap, resp: JsonRpcResponse) {
    let Some(id) = resp.id.as_u64() else {
        tracing::warn!(?resp.id, "MCP response with non-numeric id");
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
        let _ = tx.send(Err(McpError::Crash(reason.clone())));
    }
}
