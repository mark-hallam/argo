//! `ProcessRegistry` — long-lived background OS processes.
//!
//! When `terminal { background: true }` is dispatched, the child is
//! spawned through this registry instead of being awaited inline. The
//! call returns a stable `process_id` immediately, and the new
//! [`crate::tools::Process`] tool exposes `list | status | tail | kill`
//! against the registry handle held on [`crate::ToolContext`].
//!
//! Mirrors Hermes' `process_registry.py`: ring-buffered stdout/stderr
//! tails (~64 KB each), reap-on-poll status, and a SIGTERM →
//! SIGKILL-after-grace kill path on Unix.
//!
//! ```no_run
//! # use std::sync::Arc;
//! # use argo_tool::process_registry::{ProcessRegistry, SpawnRequest};
//! # async fn run() -> argo_types::ArgoResult<()> {
//! let registry = Arc::new(ProcessRegistry::new());
//! let pid = registry
//!     .spawn(SpawnRequest {
//!         command: "tail -f /var/log/system.log".into(),
//!         working_dir: std::path::PathBuf::from("/tmp"),
//!     })
//!     .await?;
//! let live = registry.list().await;
//! assert_eq!(live.len(), 1);
//! assert_eq!(live[0].id, pid);
//! registry.kill(&pid, std::time::Duration::from_secs(2)).await?;
//! # Ok(()) }
//! ```
use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};

#[path = "process_registry_tail.rs"]
mod tail;
pub use tail::TailBuffer;
use tail::{send_sigterm, spawn_pump};

/// Cap per stream — generous enough for normal tail output, hard
/// enough to stop a `yes` loop from ballooning the registry.
const TAIL_CAPACITY: usize = 64 * 1024;

/// Soft cap on the bytes returned by [`ProcessRegistry::tail`] (sum
/// of stdout + stderr). Mirrors the Hermes default.
const TAIL_RETURN_BYTES: usize = 32 * 1024;

/// Stable identifier for a registered background process.
///
/// We re-use UUIDv7 (already enabled workspace-wide) so listings are
/// time-ordered. The model only sees the string form.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProcessId(pub String);

impl ProcessId {
    /// Generate a fresh id.
    #[must_use]
    pub fn new() -> Self {
        Self(uuid::Uuid::now_v7().to_string())
    }
}

impl Default for ProcessId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ProcessId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Request shape accepted by [`ProcessRegistry::spawn`].
#[derive(Debug, Clone)]
pub struct SpawnRequest {
    /// Shell command-line, run via `bash -c`.
    pub command: String,
    /// Working directory the child inherits.
    pub working_dir: PathBuf,
}

/// Lightweight summary returned by [`ProcessRegistry::list`] and
/// [`ProcessRegistry::status`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessInfo {
    /// Registry id.
    pub id: ProcessId,
    /// OS pid (if still alive at last poll).
    pub pid: Option<u32>,
    /// Original command line.
    pub command: String,
    /// `running`, `exited:<code>`, or `killed`.
    pub state: String,
    /// Wall-clock seconds since spawn.
    pub age_seconds: u64,
}

/// Owned by the registry; not exposed to callers.
struct ProcessHandle {
    info: ProcessInfo,
    started_at: Instant,
    child: Mutex<Option<tokio::process::Child>>,
    stdout: Arc<Mutex<TailBuffer>>,
    stderr: Arc<Mutex<TailBuffer>>,
}

/// Concurrent registry of background processes.
#[derive(Debug, Default)]
pub struct ProcessRegistry {
    inner: RwLock<HashMap<ProcessId, Arc<ProcessHandle>>>,
}

impl std::fmt::Debug for ProcessHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProcessHandle")
            .field("info", &self.info)
            .finish()
    }
}

impl ProcessRegistry {
    /// Empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawn a new background process. The child runs through `bash
    /// -c`, mirroring [`crate::tools::Terminal`]. Returns the new
    /// process id.
    pub async fn spawn(&self, req: SpawnRequest) -> ArgoResult<ProcessId> {
        let mut child = tokio::process::Command::new("bash")
            .arg("-c")
            .arg(&req.command)
            .current_dir(&req.working_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| ArgoError::Tool {
                tool: "process_registry".into(),
                msg: format!("spawn failed: {e}"),
            })?;
        let pid = child.id();
        let stdout = Arc::new(Mutex::new(TailBuffer::new(TAIL_CAPACITY)));
        let stderr = Arc::new(Mutex::new(TailBuffer::new(TAIL_CAPACITY)));
        if let Some(out) = child.stdout.take() {
            spawn_pump(out, stdout.clone());
        }
        if let Some(err) = child.stderr.take() {
            spawn_pump(err, stderr.clone());
        }
        let id = ProcessId::new();
        let handle = Arc::new(ProcessHandle {
            info: ProcessInfo {
                id: id.clone(),
                pid,
                command: req.command,
                state: "running".into(),
                age_seconds: 0,
            },
            started_at: Instant::now(),
            child: Mutex::new(Some(child)),
            stdout,
            stderr,
        });
        self.inner.write().await.insert(id.clone(), handle);
        Ok(id)
    }

    /// Snapshot every live process. Reaps any that exited since the
    /// last call so callers see fresh `state` strings.
    pub async fn list(&self) -> Vec<ProcessInfo> {
        let ids: Vec<ProcessId> = self.inner.read().await.keys().cloned().collect();
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(info) = self.status(&id).await {
                out.push(info);
            }
        }
        out.sort_by(|a, b| a.id.0.cmp(&b.id.0));
        out
    }

    /// Status of a single process; `None` if unknown.
    pub async fn status(&self, id: &ProcessId) -> Option<ProcessInfo> {
        let handle = self.inner.read().await.get(id).cloned()?;
        let mut info = handle.info.clone();
        info.age_seconds = handle.started_at.elapsed().as_secs();
        let mut child_slot = handle.child.lock().await;
        if let Some(child) = child_slot.as_mut() {
            match child.try_wait() {
                Ok(Some(status)) => {
                    info.state = match status.code() {
                        Some(code) => format!("exited:{code}"),
                        None => "killed".into(),
                    };
                    info.pid = None;
                    *child_slot = None;
                }
                Ok(None) => {}
                Err(_) => {
                    info.state = "unknown".into();
                }
            }
        }
        Some(info)
    }

    /// Render the most recent ~32 KB of stdout + stderr for a single
    /// process. Returns a [`TailOutput`] so the caller (the `process`
    /// tool) can format consistently.
    pub async fn tail(&self, id: &ProcessId) -> Option<TailOutput> {
        let handle = self.inner.read().await.get(id).cloned()?;
        let stdout = handle.stdout.lock().await.render(TAIL_RETURN_BYTES);
        let stderr = handle.stderr.lock().await.render(TAIL_RETURN_BYTES);
        Some(TailOutput { stdout, stderr })
    }

    /// Send SIGTERM, wait up to `grace` for the child to exit, then
    /// fall back to SIGKILL via tokio. Removes the process from the
    /// registry on success.
    pub async fn kill(&self, id: &ProcessId, grace: Duration) -> ArgoResult<()> {
        let handle = self
            .inner
            .read()
            .await
            .get(id)
            .cloned()
            .ok_or_else(|| ArgoError::Tool {
                tool: "process_registry".into(),
                msg: format!("no such process: {id}"),
            })?;
        let mut child_slot = handle.child.lock().await;
        let Some(mut child) = child_slot.take() else {
            self.inner.write().await.remove(id);
            return Ok(());
        };
        if let Some(pid) = child.id() {
            send_sigterm(pid);
        }
        let waited = tokio::time::timeout(grace, child.wait()).await;
        if waited.is_err() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        drop(child_slot);
        self.inner.write().await.remove(id);
        Ok(())
    }

    /// Reap any processes that have already exited. Called by the
    /// `process` tool's `list` action so completed work doesn't sit
    /// in the registry forever.
    pub async fn reap_finished(&self) {
        let ids: Vec<ProcessId> = self.inner.read().await.keys().cloned().collect();
        for id in ids {
            let exited = {
                let map = self.inner.read().await;
                let Some(handle) = map.get(&id).cloned() else {
                    continue;
                };
                let mut slot = handle.child.lock().await;
                if let Some(child) = slot.as_mut() {
                    matches!(child.try_wait(), Ok(Some(_)))
                } else {
                    true
                }
            };
            if exited {
                self.inner.write().await.remove(&id);
            }
        }
    }

    /// Number of registry entries (live + not-yet-reaped).
    pub async fn len(&self) -> usize {
        self.inner.read().await.len()
    }

    /// True if the registry holds no entries.
    pub async fn is_empty(&self) -> bool {
        self.inner.read().await.is_empty()
    }
}

/// Output of [`ProcessRegistry::tail`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TailOutput {
    /// Most-recent stdout window (lossy UTF-8).
    pub stdout: String,
    /// Most-recent stderr window (lossy UTF-8).
    pub stderr: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_dir() -> PathBuf {
        std::env::temp_dir()
    }

    #[tokio::test]
    async fn spawn_list_and_kill_round_trip() {
        let reg = ProcessRegistry::new();
        let id = reg
            .spawn(SpawnRequest {
                command: "sleep 30".into(),
                working_dir: ok_dir(),
            })
            .await
            .unwrap();
        let live = reg.list().await;
        assert_eq!(live.len(), 1);
        assert_eq!(live[0].id, id);
        assert_eq!(live[0].state, "running");
        assert!(live[0].pid.is_some());

        reg.kill(&id, Duration::from_secs(2)).await.unwrap();
        assert_eq!(reg.list().await.len(), 0);
    }

    #[tokio::test]
    async fn tail_returns_recent_stdout() {
        let reg = ProcessRegistry::new();
        let id = reg
            .spawn(SpawnRequest {
                command: "for i in 1 2 3; do echo line-$i; done; sleep 0.2".into(),
                working_dir: ok_dir(),
            })
            .await
            .unwrap();
        // Give the pump time to drain.
        tokio::time::sleep(Duration::from_millis(400)).await;
        let tail = reg.tail(&id).await.unwrap();
        assert!(tail.stdout.contains("line-1"));
        assert!(tail.stdout.contains("line-3"));
        // Cleanup (process exits naturally; reap_finished drops it).
        reg.reap_finished().await;
    }

    #[tokio::test]
    async fn reap_finished_drops_exited_entries() {
        let reg = ProcessRegistry::new();
        let _ = reg
            .spawn(SpawnRequest {
                command: "true".into(),
                working_dir: ok_dir(),
            })
            .await
            .unwrap();
        // Wait for `true` to exit + the pumps to close.
        tokio::time::sleep(Duration::from_millis(200)).await;
        reg.reap_finished().await;
        assert!(reg.is_empty().await);
    }

    #[tokio::test]
    async fn unknown_kill_returns_tool_error() {
        let reg = ProcessRegistry::new();
        let err = reg
            .kill(&ProcessId("does-not-exist".into()), Duration::from_secs(1))
            .await
            .unwrap_err();
        assert!(matches!(err, ArgoError::Tool { .. }));
    }

    #[test]
    fn process_id_is_unique() {
        let a = ProcessId::new();
        let b = ProcessId::new();
        assert_ne!(a, b);
    }
}
